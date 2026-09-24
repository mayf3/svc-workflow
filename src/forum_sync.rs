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
//!   - Dormant by configuration: without WORKFLOW_FORUM_SYNC_ENABLED the
//!     loop never starts and startup logs the honest disabled line.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use uuid::Uuid;

use crate::store::postgres::outbox;

/// Duration helpers for the token cache.
pub struct ForumSyncConfig {
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
    /// `Some(config)` only when WORKFLOW_FORUM_SYNC_ENABLED=1|true AND the
    /// auth credentials are present; otherwise None (dormant).
    pub fn from_env() -> Option<Self> {
        let enabled = matches!(
            std::env::var("WORKFLOW_FORUM_SYNC_ENABLED").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE")
        );
        if !enabled {
            return None;
        }
        let (auth_base_url, client_id, client_secret) = match (
            std::env::var("WORKFLOW_FORUM_AUTH_BASE_URL"),
            std::env::var("WORKFLOW_FORUM_CLIENT_ID"),
            std::env::var("WORKFLOW_FORUM_CLIENT_SECRET"),
        ) {
            (Ok(a), Ok(id), Ok(secret))
                if !a.is_empty() && !id.is_empty() && !secret.is_empty() =>
            {
                (a, id, secret)
            }
            _ => {
                tracing::warn!("forum sync enabled but WORKFLOW_FORUM_AUTH_BASE_URL/CLIENT_ID/CLIENT_SECRET incomplete — reconciler stays dormant");
                return None;
            }
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
        Some(Self {
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
                .timeout(Duration::from_secs(10))
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
            format!("🚨 RETURN policy limit reached on workflow `{instance}` — HUMAN_REQUIRED")
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
        None => Ok(()), // no binding row (e.g. NON_BUSINESS_TEST instance)
    }
}

/// ONE reconciler pass. Returns the number of delivered rows.
pub(crate) async fn run_once(pool: &PgPool, forum: &ForumClient) -> usize {
    let mut delivered = 0usize;

    // 1. Ensure pending bindings (batch of 20, oldest first).
    match outbox::pending_bindings(pool, 20).await {
        Ok(instances) => {
            for instance_id in instances {
                if let Err(error) = ensure_thread(pool, forum, instance_id).await {
                    tracing::warn!(instance = %instance_id, error = %error, "forum binding ensure failed; retries with backoff");
                }
            }
        }
        Err(error) => {
            tracing::warn!(error = %error, "pending binding query failed");
            return delivered;
        }
    }

    // 2. Drain the due outbox batch.
    let rows = match outbox::next_pending_batch(pool, 20).await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(error = %error, "outbox batch query failed");
            return delivered;
        }
    };
    for row in rows {
        let result = match row.outbox_kind.as_str() {
            "FORUM_EVENT" => deliver_forum_event(pool, forum, &row).await,
            "EXECUTION_KICK" => deliver_kick(forum, &row).await,
            other => {
                tracing::warn!(
                    kind = other,
                    "unknown outbox kind — marked delivered to avoid an endless loop"
                );
                Ok(())
            }
        };
        match result {
            Ok(()) => {
                if outbox::mark_delivered(pool, row.outbox_id).await.is_ok() {
                    delivered += 1;
                }
            }
            Err(error) => {
                let _ = outbox::mark_attempt_failed(pool, row.outbox_id, &error).await;
            }
        }
    }
    delivered
}

type DeliverResult = Result<(), String>;

async fn deliver_forum_event(
    pool: &PgPool,
    forum: &ForumClient,
    row: &outbox::OutboxRow,
) -> DeliverResult {
    // The binding must be BOUND before any event can post; ensure it now (the
    // row's instance may not have been in this pass's pending batch).
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

/// The background loop. Spawned by main when the forum sync config resolves.
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
