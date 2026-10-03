//! WORKFLOW_EXECUTION_CONTROL_V1 (CTR-SWEC-002/003/007) — the outbox
//! reconciler: canonical forum thread ensure + forum event projection +
//! execution kicks.
//!
//! Discipline:
//!   - The reconciler NEVER writes business tables; it drains its own outbox
//!     and updates workflow_forum_bindings only.
//!   - Delivery is at-least-once; rows are never deleted or dead-lettered
//!     (no silent loss) — failures back off exponentially (capped 10 min).
//!   - Forum thread creation is idempotent by protocol: context query first,
//!     create on empty, 409 on a lost race ⇒ re-query (the forum side
//!     guarantees one thread per workflow instance: CTR-FWIC-001).
//!   - A forum/agent-core outage delays projection but can never roll a
//!     business transaction back nor lose a queued fact.
//!   - Destinations are independently configured: Forum projection may
//!     remain dormant while Core kicks and owner-assistance wakes drain.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use uuid::Uuid;

use crate::store::postgres::outbox;

const FORUM_HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// Duration helpers for the token cache.
pub struct ForumSyncConfig {
    pub forum_enabled: bool,
    pub auth_base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub forum_origin: String,
    pub audience: String,
    pub poll_interval_ms: u64,
    /// When set, EXECUTION_KICK rows POST here with this bearer token.
    pub kick: Option<ExecutionKickConfig>,
}

pub struct ExecutionKickConfig {
    pub url: String,
    pub token: String,
}

impl ForumSyncConfig {
    /// Start the existing outbox loop when either destination is configured.
    /// Forum credentials gate Forum projection only; Core delivery is independent.
    pub fn from_env() -> Option<Self> {
        let enabled = matches!(
            std::env::var("WORKFLOW_FORUM_SYNC_ENABLED").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE")
        );
        let credentials = match (
            std::env::var("WORKFLOW_FORUM_AUTH_BASE_URL"),
            std::env::var("WORKFLOW_FORUM_CLIENT_ID"),
            std::env::var("WORKFLOW_FORUM_CLIENT_SECRET"),
        ) {
            (Ok(a), Ok(id), Ok(secret))
                if !a.is_empty() && !id.is_empty() && !secret.is_empty() =>
            {
                Some((a, id, secret))
            }
            _ => None,
        };
        let kick = match (
            std::env::var("WORKFLOW_EXECUTION_KICK_URL"),
            std::env::var("WORKFLOW_EXECUTION_KICK_TOKEN"),
        ) {
            (Ok(url), Ok(token)) if !url.is_empty() && !token.is_empty() => {
                Some(ExecutionKickConfig { url, token })
            }
            _ => None,
        };
        let forum_enabled = enabled && credentials.is_some();
        if enabled && credentials.is_none() {
            tracing::warn!("forum sync enabled but WORKFLOW_FORUM_AUTH_BASE_URL/CLIENT_ID/CLIENT_SECRET incomplete — Forum projection stays dormant");
        }
        if !forum_enabled && kick.is_none() {
            return None;
        }
        let (auth_base_url, client_id, client_secret) = credentials.unwrap_or_default();
        Some(Self {
            forum_enabled,
            auth_base_url,
            client_id,
            client_secret,
            forum_origin: std::env::var("WORKFLOW_FORUM_ORIGIN")
                .unwrap_or_else(|_| "http://127.0.0.1:3460".to_string()),
            audience: std::env::var("WORKFLOW_FORUM_AUDIENCE")
                .unwrap_or_else(|_| "svc-forum".to_string()),
            poll_interval_ms: std::env::var("WORKFLOW_OUTBOX_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|v| *v >= 250)
                .unwrap_or(5_000),
            kick,
        })
    }
}

struct CachedToken {
    token: String,
    expires_at: Instant,
}

pub(crate) struct ForumClient {
    http: reqwest::Client,
    config: ForumSyncConfig,
    token: Mutex<Option<CachedToken>>,
}

#[derive(Debug)]
pub(crate) enum ForumError {
    Transport(String),
    Conflict,
    Status(u16, String),
}

impl std::fmt::Display for ForumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(detail) => write!(f, "forum transport failure: {detail}"),
            Self::Conflict => write!(f, "forum conflict (409)"),
            Self::Status(status, detail) => write!(f, "forum http {status}: {detail}"),
        }
    }
}

impl ForumClient {
    pub(crate) fn new(config: ForumSyncConfig) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(FORUM_HTTP_TIMEOUT)
                .build()
                .expect("forum sync http client"),
            config,
            token: Mutex::new(None),
        }
    }

    async fn bearer(&self) -> Result<String, ForumError> {
        {
            let cached = self.token.lock().expect("token mutex");
            if let Some(entry) = cached.as_ref() {
                if entry.expires_at > Instant::now() {
                    return Ok(entry.token.clone());
                }
            }
        }
        let credentials = format!("{}:{}", self.config.client_id, self.config.client_secret);
        use base64::Engine;
        let authorization = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
        );
        let url = format!("{}/oauth/token", self.config.auth_base_url);
        let body = format!(
            "grant_type=client_credentials&resource={}&scope={}",
            self.config.audience, "forum.read forum.write"
        );
        let response = self
            .http
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, authorization)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(body)
            .send()
            .await
            .map_err(|e| ForumError::Transport(e.to_string()))?;
        if !response.status().is_success() {
            return Err(ForumError::Status(
                response.status().as_u16(),
                "token endpoint rejected the client".to_string(),
            ));
        }
        let parsed: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ForumError::Transport(format!("malformed token response: {e}")))?;
        let token = parsed["access_token"]
            .as_str()
            .ok_or_else(|| ForumError::Transport("token response has no access_token".to_string()))?
            .to_string();
        let expires_in = parsed["expires_in"].as_u64().unwrap_or(300);
        {
            let mut cached = self.token.lock().expect("token mutex");
            // Refresh 60s before expiry; cache at most the remaining window.
            *cached = Some(CachedToken {
                token: token.clone(),
                expires_at: Instant::now()
                    + Duration::from_secs(expires_in.saturating_sub(60).max(30)),
            });
        }
        Ok(token)
    }

    /// Resolve the canonical thread id by context; Ok(None) = none yet.
    pub(crate) async fn find_thread(
        &self,
        workflow_instance_id: Uuid,
    ) -> Result<Option<String>, ForumError> {
        let token = self.bearer().await?;
        let url = format!(
            "{}/api/threads?contextType=workflow_instance&contextId={}&limit=1",
            self.config.forum_origin, workflow_instance_id
        );
        let response = self
            .http
            .get(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| ForumError::Transport(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(ForumError::Status(
                status.as_u16(),
                "thread lookup failed".to_string(),
            ));
        }
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ForumError::Transport(format!("malformed thread list: {e}")))?;
        for item in body["items"].as_array().into_iter().flatten() {
            let matches = item["contextType"].as_str() == Some("workflow_instance")
                && item["contextId"].as_str() == Some(&workflow_instance_id.to_string());
            if matches {
                if let Some(id) = item["id"].as_str() {
                    return Ok(Some(id.to_string()));
                }
            }
        }
        Ok(None)
    }

    /// Create the canonical thread; a 409 (lost race) maps to ForumError::Conflict.
    async fn create_thread(
        &self,
        workflow_instance_id: Uuid,
        title: &str,
    ) -> Result<String, ForumError> {
        let token = self.bearer().await?;
        let url = format!("{}/api/threads", self.config.forum_origin);
        let response = self
            .http
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .json(&serde_json::json!({
                "title": title,
                "type": "discussion",
                "contextType": "workflow_instance",
                "contextId": workflow_instance_id,
            }))
            .send()
            .await
            .map_err(|e| ForumError::Transport(e.to_string()))?;
        let status = response.status();
        if status == reqwest::StatusCode::CONFLICT {
            return Err(ForumError::Conflict);
        }
        if !status.is_success() {
            return Err(ForumError::Status(
                status.as_u16(),
                "thread create failed".to_string(),
            ));
        }
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ForumError::Transport(format!("malformed thread create: {e}")))?;
        body["thread"]["id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                ForumError::Transport("thread create response has no thread.id".to_string())
            })
    }

    pub(crate) async fn post_message(
        &self,
        thread_id: &str,
        content: &str,
        metadata: serde_json::Value,
    ) -> Result<(), ForumError> {
        let token = self.bearer().await?;
        let url = format!(
            "{}/api/threads/{thread_id}/messages",
            self.config.forum_origin
        );
        let response = self
            .http
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .json(&serde_json::json!({
                "content": content,
                "kind": "comment",
                "metadata": metadata,
            }))
            .send()
            .await
            .map_err(|e| ForumError::Transport(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(ForumError::Status(
                status.as_u16(),
                "message post failed".to_string(),
            ));
        }
        Ok(())
    }

    pub(crate) fn kick_config(&self) -> Option<&ExecutionKickConfig> {
        self.config.kick.as_ref()
    }
}

/// Human-readable projection text per queued event payload (Goal Scope G).
pub(crate) fn forum_text(payload: &serde_json::Value) -> String {
    let event_type = payload["eventType"].as_str().unwrap_or("workflow_event");
    let instance = payload["workflowInstanceId"]
        .as_str()
        .map(|s| s.get(0..8).unwrap_or(s).to_string())
        .unwrap_or_default();
    match event_type {
        "workflow_created" => format!("🆕 Workflow `{instance}` created"),
        "transition_committed" => {
            let effect = payload["effect"].as_str().unwrap_or("ADVANCE");
            let from = payload["sourceNodeId"]
                .as_str()
                .map(|s| s.get(0..8).unwrap_or(s))
                .unwrap_or("?");
            let to = payload["targetNodeId"]
                .as_str()
                .map(|s| s.get(0..8).unwrap_or(s))
                .unwrap_or("?");
            let returns = payload["returnCountPerEdge"].as_i64();
            match returns {
                Some(n) => format!("✅ Node transition committed: `{effect}` {from} → {to} (RETURN #{n} on this edge)"),
                None => format!("✅ Node transition committed: `{effect}` {from} → {to}"),
            }
        }
        "workflow_completed" => format!("🏁 Workflow `{instance}` completed"),
        "workflow_cancelled" => format!(
            "🛑 Workflow `{instance}` cancelled ({})",
            payload["reason"].as_str().unwrap_or("no reason given")
        ),
        "workflow_archived" => format!("📦 Workflow `{instance}` archived"),
        "assistance_requested" => format!("🙋 Assistance requested on workflow `{instance}`"),
        "assistance_escalated" => {
            format!("🚨 Assistance escalated to HUMAN_REQUIRED on workflow `{instance}`")
        }
        "assistance_resolved" => format!("✔️ Assistance resolved on workflow `{instance}`"),
        "return_policy_reached" => {
            if payload["assistanceStatus"].as_str() == Some("OWNER_PENDING") {
                format!("🧭 RETURN policy limit reached on workflow `{instance}` — Domain Owner attention (OWNER_PENDING)")
            } else {
                // Backward-compatible rendering for historical rows written by
                // the pre-owner-attention build, which auto-escalated to human.
                format!("🚨 RETURN policy limit reached on workflow `{instance}` — HUMAN_REQUIRED")
            }
        }
        "owner_attention_requested" => {
            let n = payload["attemptCount"]
                .as_i64()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".to_string());
            format!("🧭 Execution attempts exhausted ({n}) on workflow `{instance}` — Domain Owner attention (OWNER_PENDING)")
        }
        "attempts_exhausted" => {
            let n = payload["attemptCount"]
                .as_i64()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".to_string());
            format!("🚨 Execution attempts exhausted ({n}) on workflow `{instance}` — HUMAN_REQUIRED requested")
        }
        other => format!("ℹ️ {other} on workflow `{instance}`"),
    }
}

/// Ensure the canonical thread exists for a PENDING binding and bind it.
async fn ensure_thread(
    pool: &PgPool,
    forum: &ForumClient,
    workflow_instance_id: Uuid,
) -> Result<(), String> {
    let binding = outbox::get_binding(pool, workflow_instance_id)
        .await
        .map_err(|e| e.to_string())?;
    match binding {
        Some(row) if row.binding_state == "BOUND" => Ok(()),
        Some(_) => {
            if let Some(thread_id) = forum
                .find_thread(workflow_instance_id)
                .await
                .map_err(|e| e.to_string())?
            {
                outbox::bind_thread(pool, workflow_instance_id, &thread_id)
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }
            match forum
                .create_thread(
                    workflow_instance_id,
                    &format!("Workflow {workflow_instance_id}"),
                )
                .await
            {
                Ok(thread_id) => {
                    outbox::bind_thread(pool, workflow_instance_id, &thread_id)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                }
                Err(ForumError::Conflict) => {
                    // Lost race — the canonical thread now exists: re-resolve.
                    let thread_id = forum
                        .find_thread(workflow_instance_id)
                        .await
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| "409 without a resolvable thread".to_string())?;
                    outbox::bind_thread(pool, workflow_instance_id, &thread_id)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                }
                Err(other) => Err(other.to_string()),
            }
        }
        None => Err("forum event has no binding row".to_string()),
    }
}

/// ONE reconciler pass. Returns the number of delivered rows.
pub(crate) async fn run_once(pool: &PgPool, forum: &ForumClient) -> usize {
    let mut delivered = 0usize;
    // Correctness-critical owner wakes cannot share a bounded batch with kicks.
    drain_batch(pool, forum, "OWNER_ASSISTANCE_WAKE", &mut delivered, None).await;
    drain_batch(pool, forum, "EXECUTION_KICK", &mut delivered, None).await;
    if !forum.config.forum_enabled {
        return delivered;
    }

    // Each event ensures its own binding. Attribute a budget timeout to that
    // row and persist its existing backoff before ending this projection pass.
    let budget = Duration::from_millis(forum.config.poll_interval_ms).min(FORUM_HTTP_TIMEOUT);
    drain_batch(pool, forum, "FORUM_EVENT", &mut delivered, Some(budget)).await;
    delivered
}

async fn drain_batch(
    pool: &PgPool,
    forum: &ForumClient,
    kind: &str,
    delivered: &mut usize,
    budget: Option<Duration>,
) {
    let rows = match outbox::next_pending_batch(pool, 20, kind).await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(error = %error, kind, "outbox batch query failed");
            return;
        }
    };
    let deadline = budget.map(|budget| tokio::time::Instant::now() + budget);
    for row in rows {
        if deadline.is_some_and(|deadline| tokio::time::Instant::now() >= deadline) {
            break;
        }
        let delivery = async {
            match row.outbox_kind.as_str() {
                "FORUM_EVENT" => deliver_forum_event(pool, forum, &row).await,
                "EXECUTION_KICK" => deliver_kick(forum, &row).await,
                "OWNER_ASSISTANCE_WAKE" => deliver_owner_assistance_wake(forum, &row).await,
                other => {
                    tracing::warn!(kind = other, "unsupported outbox kind — retained for retry");
                    Err(format!("unsupported outbox kind: {other}"))
                }
            }
        };
        let result = match deadline {
            Some(deadline) => match tokio::time::timeout_at(deadline, delivery).await {
                Ok(result) => result,
                Err(_) => {
                    tracing::warn!(outbox_id = %row.outbox_id, "Forum work budget exhausted; recording retry backoff");
                    Err("Forum work budget exhausted".to_string())
                }
            },
            None => delivery.await,
        };
        match result {
            Ok(()) => {
                if outbox::mark_delivered(pool, row.outbox_id).await.is_ok() {
                    *delivered += 1;
                }
            }
            Err(error) => {
                let _ = outbox::mark_attempt_failed(pool, row.outbox_id, &error).await;
            }
        }
    }
}

type DeliverResult = Result<(), String>;

async fn deliver_forum_event(
    pool: &PgPool,
    forum: &ForumClient,
    row: &outbox::OutboxRow,
) -> DeliverResult {
    // Ensure the binding within this event's attempt so binding failures and
    // timeouts use the same durable row and retry backoff as message delivery.
    ensure_thread(pool, forum, row.workflow_instance_id).await?;
    let binding = outbox::get_binding(pool, row.workflow_instance_id)
        .await
        .map_err(|e| e.to_string())?;
    let thread_id = match binding {
        Some(b) if b.binding_state == "BOUND" => {
            b.forum_thread_id.ok_or("BOUND binding has no thread id")?
        }
        _ => return Err("binding still pending — forum thread unavailable".to_string()),
    };
    let content = forum_text(&row.payload);
    forum
        .post_message(
            &thread_id,
            &content,
            serde_json::json!({
                "workflowInstanceId": row.workflow_instance_id,
                "eventKey": row.event_key,
                "eventType": row.payload["eventType"],
            }),
        )
        .await
        .map_err(|e| e.to_string())
}

async fn deliver_kick(forum: &ForumClient, row: &outbox::OutboxRow) -> DeliverResult {
    let kick = forum
        .kick_config()
        .ok_or("kick endpoint not configured (WORKFLOW_EXECUTION_KICK_URL/TOKEN)")?;
    let response = forum
        .http
        .post(&kick.url)
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", kick.token),
        )
        .json(&row.payload)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if status.is_success() || status.is_client_error() {
        // 4xx = permanent; the poll loop owns correctness either way.
        Ok(())
    } else {
        Err(format!("kick endpoint 5xx: {}", status.as_u16()))
    }
}

async fn deliver_owner_assistance_wake(
    forum: &ForumClient,
    row: &outbox::OutboxRow,
) -> DeliverResult {
    let kick = forum
        .kick_config()
        .ok_or("Agent Core push endpoint not configured (WORKFLOW_EXECUTION_KICK_URL/TOKEN)")?;
    let mut url = reqwest::Url::parse(&kick.url)
        .map_err(|error| format!("invalid WORKFLOW_EXECUTION_KICK_URL: {error}"))?;
    url.set_path("/workflow-execution/owner-assistance-wakes");
    url.set_query(None);
    url.set_fragment(None);

    let response = forum
        .http
        .post(url)
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", kick.token),
        )
        .json(&row.payload)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        // Unlike latency-only execution kicks, owner assistance wake is part
        // of the exception-handling correctness path. Keep the durable row
        // pending across every non-2xx so configuration/deployment drift can
        // be repaired without silently losing the Domain Owner notification.
        Err(format!(
            "owner assistance wake endpoint returned {}",
            status.as_u16()
        ))
    }
}

/// The existing background loop, spawned when either outbox destination resolves.
pub async fn run_loop(pool: PgPool, config: ForumSyncConfig) {
    let forum = ForumClient::new(config);
    let mut ticker = tokio::time::interval(Duration::from_millis(forum.config.poll_interval_ms));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let delivered = run_once(&pool, &forum).await;
        if delivered > 0 {
            tracing::info!(delivered, "outbox reconciler delivered rows");
        }
    }
}

#[cfg(test)]
mod owner_assistance_wake_tests {
    use super::*;
    use axum::{
        extract::{Path, State},
        http::{HeaderMap, StatusCode},
        routing::post,
        Json, Router,
    };
    use std::sync::{Arc, Mutex as StdMutex};

    static CONFIG_ENV: StdMutex<()> = StdMutex::new(());

    fn config_with_forum_flag(
        flag: Option<&str>,
        core: bool,
        credentials: bool,
    ) -> Option<ForumSyncConfig> {
        let _lock = CONFIG_ENV.lock().unwrap();
        let values = [
            ("WORKFLOW_FORUM_SYNC_ENABLED", flag),
            (
                "WORKFLOW_FORUM_AUTH_BASE_URL",
                credentials.then_some("http://127.0.0.1:9"),
            ),
            (
                "WORKFLOW_FORUM_CLIENT_ID",
                credentials.then_some("forum-client"),
            ),
            (
                "WORKFLOW_FORUM_CLIENT_SECRET",
                credentials.then_some("forum-secret"),
            ),
            (
                "WORKFLOW_EXECUTION_KICK_URL",
                core.then_some("http://127.0.0.1:9/workflow-execution/kicks"),
            ),
            (
                "WORKFLOW_EXECUTION_KICK_TOKEN",
                core.then_some("push-secret"),
            ),
        ];
        let previous: Vec<_> = values
            .iter()
            .map(|(key, _)| (*key, std::env::var_os(key)))
            .collect();
        for (key, value) in values {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        let config = ForumSyncConfig::from_env();
        for (key, value) in previous {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        config
    }

    #[test]
    fn core_push_configuration_starts_outbox_without_forum_configuration() {
        for flag in [None, Some("false"), Some("true")] {
            let config = config_with_forum_flag(flag, true, false)
                .expect("configured Core delivery must not depend on optional Forum configuration");
            assert!(!config.forum_enabled);
            assert_eq!(config.kick.unwrap().token, "push-secret");
        }
    }

    #[test]
    fn unconfigured_destinations_stay_dormant_and_forum_can_run_alone() {
        assert!(config_with_forum_flag(None, false, false).is_none());
        assert!(config_with_forum_flag(Some("true"), false, false).is_none());
        let forum = config_with_forum_flag(Some("true"), false, true).unwrap();
        assert!(forum.forum_enabled);
        assert!(forum.kick.is_none());
    }

    async fn outbox_pool(with_bindings: bool) -> PgPool {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must name an isolated test database");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("CREATE TEMP TABLE workflow_outbox (
            outbox_id UUID PRIMARY KEY, workflow_instance_id UUID NOT NULL,
            outbox_kind TEXT NOT NULL, event_key TEXT NOT NULL, payload JSONB NOT NULL,
            attempt_count INT NOT NULL DEFAULT 0, next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            delivered_at TIMESTAMPTZ, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), last_error TEXT,
            UNIQUE (outbox_kind, event_key)
        )").execute(&pool).await.unwrap();
        if with_bindings {
            sqlx::query(
                "CREATE TEMP TABLE workflow_forum_bindings (
                workflow_instance_id UUID PRIMARY KEY, forum_thread_id TEXT,
                binding_state TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )",
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        pool
    }

    async fn insert_wake(pool: &PgPool) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_outbox
            (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
            VALUES ($1,$2,'OWNER_ASSISTANCE_WAKE',$3,'{}'::jsonb)",
        )
        .bind(id)
        .bind(Uuid::new_v4())
        .bind(format!("owner-assistance:{id}"))
        .execute(pool)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn core_wake_is_delivered_before_a_failing_forum_query() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(capture_handler),
            )
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let pool = outbox_pool(false).await;
        sqlx::query(
            "INSERT INTO workflow_outbox
            (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
            VALUES ($1,$2,'FORUM_EVENT','missing-binding-table','{}'::jsonb)",
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();
        insert_wake(&pool).await;
        let forum = client(
            format!("http://{addr}/workflow-execution/kicks"),
            "push-secret",
        );
        assert_eq!(run_once(&pool, &forum).await, 1);
        assert!(capture.body.lock().unwrap().is_some());
        server.abort();
    }

    #[tokio::test]
    async fn forum_backlog_does_not_fill_the_core_wake_batch() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/kicks",
                post(|| async { StatusCode::SERVICE_UNAVAILABLE }),
            )
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(capture_handler),
            )
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let pool = outbox_pool(true).await;
        for index in 0..21 {
            sqlx::query(
                "INSERT INTO workflow_outbox
                (outbox_id, workflow_instance_id, outbox_kind, event_key, payload, created_at)
                VALUES ($1,$2,'FORUM_EVENT',$3,'{}'::jsonb,now() - interval '1 day')",
            )
            .bind(Uuid::new_v4())
            .bind(Uuid::new_v4())
            .bind(format!("forum:{index}"))
            .execute(&pool)
            .await
            .unwrap();
        }
        for index in 0..21 {
            sqlx::query(
                "INSERT INTO workflow_outbox
                (outbox_id, workflow_instance_id, outbox_kind, event_key, payload, created_at)
                VALUES ($1,$2,'EXECUTION_KICK',$3,'{}'::jsonb,now() - interval '1 day')",
            )
            .bind(Uuid::new_v4())
            .bind(Uuid::new_v4())
            .bind(format!("kick:{index}"))
            .execute(&pool)
            .await
            .unwrap();
        }
        let wake = insert_wake(&pool).await;
        let mut forum = client(
            format!("http://{addr}/workflow-execution/kicks"),
            "push-secret",
        );
        run_once(&pool, &forum).await;
        assert!(
            capture.body.lock().unwrap().is_some(),
            "older Forum backlog must not consume the Core delivery batch"
        );
        let delivered: bool = sqlx::query_scalar(
            "SELECT delivered_at IS NOT NULL FROM workflow_outbox WHERE outbox_id = $1",
        )
        .bind(wake)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(delivered);
        let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_outbox WHERE outbox_kind = 'FORUM_EVENT' AND delivered_at IS NULL")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(pending, 21);
        let attempts: i64 = sqlx::query_scalar("SELECT sum(attempt_count)::bigint FROM workflow_outbox WHERE outbox_kind = 'FORUM_EVENT'")
            .fetch_one(&pool).await.unwrap();
        forum.config.forum_enabled = false;
        insert_wake(&pool).await;
        sqlx::query(
            "INSERT INTO workflow_outbox
            (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
            VALUES ($1,$2,'FUTURE_KIND','unknown-kind','{}'::jsonb)",
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(run_once(&pool, &forum).await, 1);
        let unknown: (bool, i32, Option<String>) = sqlx::query_as("SELECT delivered_at IS NULL, attempt_count, last_error FROM workflow_outbox WHERE event_key = 'unknown-kind'")
            .fetch_one(&pool).await.unwrap();
        assert!(
            unknown.0,
            "unsupported kinds must remain pending across forward deployment/rollback"
        );
        assert_eq!(unknown.1, 1);
        assert!(unknown.2.unwrap().contains("FUTURE_KIND"));
        let attempts_after: i64 = sqlx::query_scalar("SELECT sum(attempt_count)::bigint FROM workflow_outbox WHERE outbox_kind = 'FORUM_EVENT'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(
            attempts_after, attempts,
            "disabled Forum projection must stay queued without another attempt"
        );
        server.abort();
    }

    #[tokio::test]
    async fn unsupported_kind_stays_pending_and_retryable() {
        let pool = outbox_pool(false).await;
        sqlx::query(
            "INSERT INTO workflow_outbox
            (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
            VALUES ($1,$2,'FUTURE_KIND','future-kind','{}'::jsonb)",
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();
        let mut forum = client(
            "http://127.0.0.1:9/workflow-execution/kicks".to_string(),
            "push-secret",
        );
        forum.config.forum_enabled = false;
        assert_eq!(run_once(&pool, &forum).await, 0);
        let unknown: (bool, i32, bool, Option<String>) = sqlx::query_as("SELECT delivered_at IS NULL, attempt_count, next_attempt_at > now(), last_error FROM workflow_outbox WHERE event_key = 'future-kind'")
            .fetch_one(&pool).await.unwrap();
        assert!(unknown.0);
        assert_eq!(unknown.1, 1);
        assert!(unknown.2);
        assert!(unknown.3.unwrap().contains("FUTURE_KIND"));
    }

    #[tokio::test]
    async fn slow_forum_work_is_bounded_and_its_event_remains_pending() {
        async fn slow_forum(
            State(capture): State<Capture>,
            Path(thread_id): Path<String>,
            Json(body): Json<serde_json::Value>,
        ) -> StatusCode {
            *capture.body.lock().unwrap() = Some(body);
            if thread_id == "thread-1" {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            StatusCode::OK
        }
        let capture = Capture::default();
        let app = Router::new()
            .route("/api/threads/{thread_id}/messages", post(slow_forum))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let pool = outbox_pool(true).await;
        let instance = Uuid::new_v4();
        let event = Uuid::new_v4();
        let healthy_instance = Uuid::new_v4();
        let healthy_event = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_forum_bindings
            (workflow_instance_id, forum_thread_id, binding_state) VALUES ($1,'thread-1','BOUND')",
        )
        .bind(instance)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO workflow_outbox
            (outbox_id, workflow_instance_id, outbox_kind, event_key, payload, created_at)
            VALUES ($1,$2,'FORUM_EVENT','slow-forum-event','{}'::jsonb,now() - interval '1 day')",
        )
        .bind(event)
        .bind(instance)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO workflow_forum_bindings
            (workflow_instance_id, forum_thread_id, binding_state) VALUES ($1,'thread-2','BOUND')",
        )
        .bind(healthy_instance)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO workflow_outbox
            (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
            VALUES ($1,$2,'FORUM_EVENT','healthy-forum-event','{}'::jsonb)",
        )
        .bind(healthy_event)
        .bind(healthy_instance)
        .execute(&pool)
        .await
        .unwrap();
        let mut forum = client(
            format!("http://{addr}/workflow-execution/kicks"),
            "push-secret",
        );
        forum.config.forum_origin = format!("http://{addr}");
        forum.config.poll_interval_ms = 20;
        *forum.token.lock().unwrap() = Some(CachedToken {
            token: "cached-forum-token".to_string(),
            expires_at: Instant::now() + Duration::from_secs(60),
        });
        tokio::time::timeout(Duration::from_millis(140), run_once(&pool, &forum))
            .await
            .expect("Forum phase must respect its bounded poll-interval budget");
        let event_after: (bool, String, i32, bool) = sqlx::query_as(
            "SELECT delivered_at IS NULL, event_key, attempt_count, next_attempt_at > now() FROM workflow_outbox WHERE outbox_id = $1",
        )
        .bind(event)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            event_after.0,
            "cancelled outbound delivery must not become delivered"
        );
        assert_eq!(event_after.1, "slow-forum-event");
        assert_eq!(
            event_after.2, 1,
            "timed-out delivery must record its existing retry backoff"
        );
        assert!(event_after.3);
        assert!(
            capture.body.lock().unwrap().is_some(),
            "test must exercise a real in-flight Forum HTTP request"
        );
        assert_eq!(
            capture.body.lock().unwrap().as_ref().unwrap()["metadata"]["eventKey"],
            serde_json::json!("slow-forum-event")
        );
        let delivered = tokio::time::timeout(Duration::from_millis(140), run_once(&pool, &forum))
            .await
            .expect("healthy other-instance event must drain while the slow event backs off");
        assert_eq!(delivered, 1);
        let healthy_delivered: bool = sqlx::query_scalar(
            "SELECT delivered_at IS NOT NULL FROM workflow_outbox WHERE outbox_id = $1",
        )
        .bind(healthy_event)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(healthy_delivered);
        assert_eq!(
            capture.body.lock().unwrap().as_ref().unwrap()["metadata"]["eventKey"],
            serde_json::json!("healthy-forum-event")
        );
        server.abort();
    }

    #[derive(Clone, Default)]
    struct Capture {
        authorization: Arc<StdMutex<Option<String>>>,
        body: Arc<StdMutex<Option<serde_json::Value>>>,
    }

    fn client(url: String, token: &str) -> ForumClient {
        ForumClient::new(ForumSyncConfig {
            forum_enabled: true,
            auth_base_url: "http://127.0.0.1:9".to_string(),
            client_id: "unused".to_string(),
            client_secret: "unused".to_string(),
            forum_origin: "http://127.0.0.1:9".to_string(),
            audience: "svc-forum".to_string(),
            poll_interval_ms: 5_000,
            kick: Some(ExecutionKickConfig {
                url,
                token: token.to_string(),
            }),
        })
    }
    fn row(payload: serde_json::Value) -> outbox::OutboxRow {
        outbox::OutboxRow {
            outbox_id: Uuid::new_v4(),
            workflow_instance_id: Uuid::new_v4(),
            outbox_kind: "OWNER_ASSISTANCE_WAKE".to_string(),
            event_key: "owner-assistance:test".to_string(),
            payload,
            attempt_count: 0,
        }
    }

    async fn capture_handler(
        State(capture): State<Capture>,
        headers: HeaderMap,
        Json(body): Json<serde_json::Value>,
    ) -> StatusCode {
        *capture.authorization.lock().unwrap() = headers
            .get(reqwest::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        *capture.body.lock().unwrap() = Some(body);
        StatusCode::OK
    }

    #[tokio::test]
    async fn owner_wake_derives_endpoint_reuses_token_and_forwards_payload() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(capture_handler),
            )
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let payload = serde_json::json!({
            "workflowInstanceId": Uuid::new_v4(),
            "nodeVisitId": Uuid::new_v4(),
            "assistanceCaseId": Uuid::new_v4(),
            "ownerPrincipalId": Uuid::new_v4(),
            "reason": "RETURN_POLICY_EXHAUSTED",
        });
        let forum = client(
            format!("http://{addr}/workflow-execution/kicks?ignored=yes"),
            "push-secret",
        );
        deliver_owner_assistance_wake(&forum, &row(payload.clone()))
            .await
            .unwrap();

        assert_eq!(
            capture.authorization.lock().unwrap().as_deref(),
            Some("Bearer push-secret")
        );
        assert_eq!(capture.body.lock().unwrap().as_ref(), Some(&payload));
        server.abort();
    }

    #[tokio::test]
    async fn owner_wake_keeps_non_success_retryable() {
        let app = Router::new().route(
            "/workflow-execution/owner-assistance-wakes",
            post(|| async { StatusCode::SERVICE_UNAVAILABLE }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let forum = client(
            format!("http://{addr}/workflow-execution/kicks"),
            "push-secret",
        );
        let error = deliver_owner_assistance_wake(&forum, &row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(error.contains("503"));
        server.abort();
    }
    #[test]
    fn owner_attention_forum_projection_preserves_old_and_new_semantics() {
        let workflow = Uuid::new_v4();
        let new_return = forum_text(&serde_json::json!({
            "eventType": "return_policy_reached",
            "workflowInstanceId": workflow,
            "assistanceStatus": "OWNER_PENDING",
        }));
        assert!(new_return.contains("OWNER_PENDING"));
        assert!(new_return.contains("Domain Owner"));

        let historical = forum_text(&serde_json::json!({
            "eventType": "return_policy_reached",
            "workflowInstanceId": workflow,
        }));
        assert!(historical.contains("HUMAN_REQUIRED"));

        let attempts = forum_text(&serde_json::json!({
            "eventType": "owner_attention_requested",
            "workflowInstanceId": workflow,
            "attemptCount": 3,
        }));
        assert!(attempts.contains("OWNER_PENDING"));
    }
}
