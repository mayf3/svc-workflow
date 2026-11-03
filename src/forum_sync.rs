//! The durable outbox reconciler — three independent delivery channels over
//! one shared polling loop:
//!
//!   - `EXECUTION_KICK` push-first poll trigger for freshly created dispatch
//!     intents (latency-only; the Agent Core due-poll loop stays the
//!     correctness path)
//!   - `FORUM_EVENT` canonical forum thread ensure + event projection
//!   - `OWNER_ASSISTANCE_WAKE` durable Domain Owner assistance wakes
//!
//! Channel independence (the reason this module exists):
//!   - Each channel has its own enable flag, credentials and endpoint; a
//!     missing FORUM credential set can no longer keep EXECUTION_KICK rows
//!     queued forever (the 2026-11 production F1 root cause, where the whole
//!     reconciler was gated on `WORKFLOW_FORUM_SYNC_ENABLED` plus the forum
//!     credential trio).
//!   - Rows are claimed per kind with `FOR UPDATE SKIP LOCKED` inside the
//!     delivery transaction: concurrent workers never double-deliver, and a
//!     crashed worker's row lock dies with its connection (lock-based lease;
//!     recovery is immediate, not timeout-bound).
//!   - A disabled channel never claims, attempts or rewrites its rows — no
//!     attempt_count, backoff or delivered_at drift from a channel that is
//!     off.
//!   - Delivery errors are isolated per row and per channel: one channel's
//!     failure never blocks another enabled channel in the same pass. The
//!     shared pass task is panic-contained (logged, loop continues); the
//!     pool must have headroom for one extra connection while a FORUM_EVENT
//!     delivery holds its claim transaction open across the thread-ensure
//!     step.
//!
//! Delivery semantics:
//!   - Delivery is at-least-once; rows are never deleted. The dedupe identity
//!     is `uq_outbox_kind_event_key` (queuing) plus the per-service
//!     idempotency protocols (draining).
//!   - EXECUTION_KICK failure classes: 2xx delivered; 401/403 (auth or
//!     permission failure — repairable, never treated as a successful
//!     delivery) and 408/409/429/5xx/transport failures retry with
//!     exponential backoff and dead-letter after
//!     `outbox::EXECUTION_KICK_MAX_ATTEMPTS`; every other 4xx (the receiver
//!     deterministically rejected the payload — e.g. 400 invalid_arguments)
//!     is permanent and dead-letters immediately. The kicks contract has no
//!     409 semantics; a 409 is therefore treated as an unexpected retryable
//!     condition, bounded by the dead-letter cap. Blind replay is safe for
//!     kicks by contract: a kick only triggers one coalesced engine poll,
//!     kicks never stack, and the one-attempt fence makes a double trigger
//!     harmless. A stable `Idempotency-Key` header (the outbox event key)
//!     rides every request for auditability.
//!   - FORUM_EVENT and OWNER_ASSISTANCE_WAKE keep their existing
//!     correctness-path discipline: every non-2xx retries with backoff and
//!     rows are never dead-lettered (no silent loss of the forum projection
//!     or the Domain Owner notification).
//!   - Dead-lettered rows keep payload, attempt_count and last_error; the
//!     recovery entry is the documented requeue statement in
//!     `migrations/0032_outbox_dead_letter.sql`.
//!   - The kind-scoped batches never select any other outbox_kind. Current
//!     writers emit only the three kinds above; a row written by a NEWER
//!     binary and read by a downgraded one would sit invisible to the drain
//!     (visible in SQL) instead of being blindly marked delivered.
//!
//! Dormancy: without any channel configuration the loop never starts and
//! startup logs the honest disabled line.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use uuid::Uuid;

use crate::store::postgres::outbox;

/// Forum projection channel: canonical thread ensure + event posting.
pub struct ForumEventConfig {
    pub auth_base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub forum_origin: String,
    pub audience: String,
}

/// Execution kick channel: POSTs to the Agent Core product-api
/// `POST /workflow-execution/kicks` with a static bearer token carrying the
/// `workflow.execute` scope. Latency-only — never a correctness dependency.
pub struct ExecutionKickConfig {
    pub url: String,
    pub token: String,
}

/// Dedicated client-credentials path for durable Owner Assistance wakes.
pub struct OwnerAssistanceWakeConfig {
    pub url: String,
    pub auth_base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub audience: String,
}

/// The reconciler's per-channel configuration. Built once at startup; the
/// loop is spawned only when `any_channel_enabled()` is true.
pub struct OutboxReconcilerConfig {
    pub forum: Option<ForumEventConfig>,
    pub kick: Option<ExecutionKickConfig>,
    /// An error is retained so each outbox attempt fails closed with a
    /// sanitized configuration error rather than disabling the reconciler.
    pub owner_assistance_wake: Result<Option<OwnerAssistanceWakeConfig>, &'static str>,
    pub poll_interval_ms: u64,
}

impl OwnerAssistanceWakeConfig {
    fn from_env() -> Result<Option<Self>, &'static str> {
        Self::from_values([
            std::env::var_os("WORKFLOW_OWNER_ASSISTANCE_WAKE_URL"),
            std::env::var_os("WORKFLOW_OWNER_ASSISTANCE_WAKE_AUTH_BASE_URL"),
            std::env::var_os("WORKFLOW_OWNER_ASSISTANCE_WAKE_CLIENT_ID"),
            std::env::var_os("WORKFLOW_OWNER_ASSISTANCE_WAKE_CLIENT_SECRET"),
            std::env::var_os("WORKFLOW_OWNER_ASSISTANCE_WAKE_AUDIENCE"),
        ])
    }

    fn from_values(values: [Option<std::ffi::OsString>; 5]) -> Result<Option<Self>, &'static str> {
        if values.iter().all(Option::is_none) {
            return Ok(None);
        }
        let [Some(url), Some(auth_base_url), Some(client_id), Some(client_secret), Some(audience)] =
            values
        else {
            return Err("owner assistance wake configuration is incomplete or invalid");
        };
        let (Ok(url), Ok(auth_base_url), Ok(client_id), Ok(client_secret), Ok(audience)) = (
            url.into_string(),
            auth_base_url.into_string(),
            client_id.into_string(),
            client_secret.into_string(),
            audience.into_string(),
        ) else {
            return Err("owner assistance wake configuration is incomplete or invalid");
        };
        if [
            url.as_str(),
            auth_base_url.as_str(),
            client_id.as_str(),
            client_secret.as_str(),
            audience.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err("owner assistance wake configuration is incomplete or invalid");
        }
        validate_wake_url(&url)?;
        validate_auth_base_url(&auth_base_url)?;
        Ok(Some(Self {
            url,
            auth_base_url,
            client_id,
            client_secret,
            audience,
        }))
    }
}

fn validate_wake_url(value: &str) -> Result<(), &'static str> {
    let url = reqwest::Url::parse(value)
        .map_err(|_| "owner assistance wake configuration is incomplete or invalid")?;
    if url.scheme() != "http"
        || url.host_str() != Some("127.0.0.1")
        || url.path() != "/workflow-execution/owner-assistance-wakes"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("owner assistance wake configuration is incomplete or invalid");
    }
    Ok(())
}

fn validate_kick_url(value: &str) -> Result<(), &'static str> {
    // Same boundary discipline as the owner wake endpoint: the product-api
    // binds loopback only (the transport boundary), so a non-loopback or
    // wrong-path kick URL is a misconfiguration, not a deployment option.
    // Loopback-host parity with validate_auth_base_url (localhost and
    // loopback literals in IPv4/IPv6); trailing slashes are tolerated.
    let url = reqwest::Url::parse(value)
        .map_err(|_| "execution kick configuration is incomplete or invalid")?;
    let host = url.host_str().unwrap_or_default();
    let loopback_host = host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::Ipv4Addr>()
            .is_ok_and(|address| address.is_loopback())
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::Ipv6Addr>()
            .is_ok_and(|address| address.is_loopback());
    let path = url.path().trim_end_matches('/');
    if url.scheme() != "http"
        || !loopback_host
        || path != "/workflow-execution/kicks"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("execution kick configuration is incomplete or invalid");
    }
    Ok(())
}

fn validate_auth_base_url(value: &str) -> Result<(), &'static str> {
    let url = reqwest::Url::parse(value)
        .map_err(|_| "owner assistance wake configuration is incomplete or invalid")?;
    let host = url.host_str().unwrap_or_default();
    let loopback_host = host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::Ipv4Addr>()
            .is_ok_and(|address| address.is_loopback())
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::Ipv6Addr>()
            .is_ok_and(|address| address.is_loopback());
    let loopback_http = url.scheme() == "http" && loopback_host;
    if !(url.scheme() == "https" || loopback_http)
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("owner assistance wake configuration is incomplete or invalid");
    }
    Ok(())
}

fn env_flag(name: &str) -> Option<bool> {
    match std::env::var(name).as_deref() {
        Ok("1") | Ok("true") | Ok("TRUE") => Some(true),
        Ok(_) => Some(false),
        Err(_) => None,
    }
}

impl OutboxReconcilerConfig {
    pub fn from_env() -> Self {
        let legacy_forum_enabled = env_flag("WORKFLOW_FORUM_SYNC_ENABLED") == Some(true);
        let forum = if legacy_forum_enabled {
            match (
                std::env::var("WORKFLOW_FORUM_AUTH_BASE_URL"),
                std::env::var("WORKFLOW_FORUM_CLIENT_ID"),
                std::env::var("WORKFLOW_FORUM_CLIENT_SECRET"),
            ) {
                (Ok(a), Ok(id), Ok(secret))
                    if !a.is_empty() && !id.is_empty() && !secret.is_empty() =>
                {
                    Some(ForumEventConfig {
                        auth_base_url: a,
                        client_id: id,
                        client_secret: secret,
                        forum_origin: std::env::var("WORKFLOW_FORUM_ORIGIN")
                            .unwrap_or_else(|_| "http://127.0.0.1:3460".to_string()),
                        audience: std::env::var("WORKFLOW_FORUM_AUDIENCE")
                            .unwrap_or_else(|_| "svc-forum".to_string()),
                    })
                }
                _ => {
                    tracing::warn!("forum sync enabled but WORKFLOW_FORUM_AUTH_BASE_URL/CLIENT_ID/CLIENT_SECRET incomplete — forum channel stays dormant");
                    None
                }
            }
        } else {
            None
        };
        let kick = Self::kick_from_env(legacy_forum_enabled);
        let owner_assistance_wake = Self::wake_from_env(legacy_forum_enabled);
        Self {
            forum,
            kick,
            owner_assistance_wake,
            poll_interval_ms: std::env::var("WORKFLOW_OUTBOX_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|v| *v >= 250)
                .unwrap_or(5_000),
        }
    }

    /// Kick channel switch. Explicit `WORKFLOW_EXECUTION_KICK_ENABLED` wins;
    /// when it is absent the legacy nesting applies (the forum master switch
    /// enabled kicks whenever the kick URL/token were present).
    fn kick_from_env(legacy_forum_enabled: bool) -> Option<ExecutionKickConfig> {
        match env_flag("WORKFLOW_EXECUTION_KICK_ENABLED") {
            Some(false) => return None,
            None if !legacy_forum_enabled => return None,
            _ => {}
        }
        match Self::kick_from_values() {
            Some(config) => Some(config),
            None => {
                tracing::error!(
                    "execution kick channel enabled but WORKFLOW_EXECUTION_KICK_URL/TOKEN missing, blank, or not a loopback /workflow-execution/kicks URL — kick channel stays dormant and kick rows will not be delivered"
                );
                None
            }
        }
    }

    fn kick_from_values() -> Option<ExecutionKickConfig> {
        let url = std::env::var("WORKFLOW_EXECUTION_KICK_URL").ok()?;
        let token = std::env::var("WORKFLOW_EXECUTION_KICK_TOKEN").ok()?;
        if url.trim().is_empty() || token.trim().is_empty() {
            return None;
        }
        validate_kick_url(&url).ok()?;
        Some(ExecutionKickConfig { url, token })
    }

    /// Wake channel switch. Explicit `WORKFLOW_OWNER_ASSISTANCE_WAKE_ENABLED`
    /// wins (including an explicit disable over legacy nesting); when it is
    /// absent the legacy nesting applies. A retained partial-configuration
    /// error keeps the channel active so each attempt fails closed with the
    /// sanitized error instead of the channel going silently dark.
    fn wake_from_env(
        legacy_forum_enabled: bool,
    ) -> Result<Option<OwnerAssistanceWakeConfig>, &'static str> {
        match env_flag("WORKFLOW_OWNER_ASSISTANCE_WAKE_ENABLED") {
            Some(true) => OwnerAssistanceWakeConfig::from_env(),
            Some(false) => Ok(None),
            None if legacy_forum_enabled => OwnerAssistanceWakeConfig::from_env(),
            None => Ok(None),
        }
    }

    /// True when at least one channel will claim its rows.
    pub fn any_channel_enabled(&self) -> bool {
        self.forum.is_some() || self.kick.is_some() || self.wake_channel_active()
    }

    fn wake_channel_active(&self) -> bool {
        matches!(self.owner_assistance_wake, Ok(Some(_)) | Err(_))
    }

    /// The channels that will claim rows, for the startup log.
    pub fn enabled_channel_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.kick.is_some() {
            names.push("EXECUTION_KICK");
        }
        if self.forum.is_some() {
            names.push("FORUM_EVENT");
        }
        if self.wake_channel_active() {
            names.push("OWNER_ASSISTANCE_WAKE");
        }
        names
    }
}

struct CachedToken {
    token: String,
    expires_at: Instant,
}

pub struct ForumClient {
    http: reqwest::Client,
    kick_http: reqwest::Client,
    owner_wake_http: reqwest::Client,
    config: OutboxReconcilerConfig,
    token: Mutex<Option<CachedToken>>,
    owner_wake_tokens: tokio::sync::Mutex<HashMap<OwnerWakeTokenKey, CachedToken>>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct OwnerWakeTokenKey {
    client_id: String,
    audience: String,
    scope: String,
}

/// How a failed delivery settles. `Permanent` = the receiver deterministically
/// rejected the request (no retry value); `Retryable` = back off and retry,
/// bounded by the channel's dead-letter cap where one exists.
#[derive(Debug)]
pub(crate) enum ChannelFailure {
    Permanent(String),
    Retryable(String),
}

impl ChannelFailure {
    fn message(&self) -> &str {
        match self {
            Self::Permanent(message) => message,
            Self::Retryable(message) => message,
        }
    }
}

enum OutboxOutcome {
    Delivered,
    RetryScheduled,
    DeadLettered,
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
    pub fn new(config: OutboxReconcilerConfig) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("forum sync http client"),
            // Redirects are not part of the kicks contract; a 3xx from the
            // configured endpoint is classified, never followed.
            kick_http: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("kick http client"),
            owner_wake_http: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("owner wake http client"),
            config,
            token: Mutex::new(None),
            owner_wake_tokens: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    async fn owner_wake_bearer(
        &self,
        config: &OwnerAssistanceWakeConfig,
    ) -> Result<(OwnerWakeTokenKey, String), String> {
        const SCOPE: &str = "workflow.execute";
        let key = OwnerWakeTokenKey {
            client_id: config.client_id.clone(),
            audience: config.audience.clone(),
            scope: SCOPE.to_string(),
        };
        // Hold the async mutex across the fetch so concurrent misses for this
        // exact key coalesce into one Auth request.
        let mut tokens = self.owner_wake_tokens.lock().await;
        if let Some(cached) = tokens.get(&key) {
            if Instant::now() < cached.expires_at {
                return Ok((key, cached.token.clone()));
            }
        }
        tokens.remove(&key);

        let credentials = format!("{}:{}", config.client_id, config.client_secret);
        use base64::Engine;
        let authorization = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
        );
        let base = reqwest::Url::parse(&config.auth_base_url).map_err(|_| {
            "owner assistance wake configuration is incomplete or invalid".to_string()
        })?;
        let token_path = format!("{}/oauth/token", base.path().trim_end_matches('/'));
        let mut token_url = base;
        token_url.set_path(&token_path);
        let request = self
            .owner_wake_http
            .post(token_url)
            .header(reqwest::header::AUTHORIZATION, authorization)
            .form(&[
                ("grant_type", "client_credentials"),
                ("resource", config.audience.as_str()),
                ("scope", SCOPE),
            ]);
        let request_started = Instant::now();
        let response = request
            .send()
            .await
            .map_err(|_| "owner assistance wake Auth request failed".to_string())?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(format!(
                "owner assistance wake Auth returned {}",
                response.status().as_u16()
            ));
        }
        #[derive(serde::Deserialize)]
        struct TokenResponse {
            access_token: String,
            token_type: String,
            expires_in: u64,
            scope: String,
        }
        let token: TokenResponse = response.json().await.map_err(|_| {
            "owner assistance wake Auth returned an invalid token response".to_string()
        })?;
        if token.token_type != "Bearer"
            || token.access_token.trim().is_empty()
            || token.scope != SCOPE
            || !(1..=600).contains(&token.expires_in)
        {
            return Err(
                "owner assistance wake Auth returned an invalid token response".to_string(),
            );
        }
        let renewal_margin = Duration::from_secs_f64((token.expires_in as f64 * 0.10).min(60.0));
        let renew_at = request_started + Duration::from_secs(token.expires_in) - renewal_margin;
        if Instant::now() >= renew_at {
            return Err(
                "owner assistance wake Auth token arrived after its renewal point".to_string(),
            );
        }
        let bearer = token.access_token;
        tokens.insert(
            key.clone(),
            CachedToken {
                token: bearer.clone(),
                expires_at: renew_at,
            },
        );
        Ok((key, bearer))
    }

    async fn invalidate_owner_wake_bearer(&self, key: &OwnerWakeTokenKey, bearer: &str) {
        let mut tokens = self.owner_wake_tokens.lock().await;
        if tokens.get(key).is_some_and(|cached| cached.token == bearer) {
            tokens.remove(key);
        }
    }

    async fn bearer(&self) -> Result<String, ForumError> {
        let Some(forum) = self.config.forum.as_ref() else {
            return Err(ForumError::Transport(
                "forum channel is not configured".to_string(),
            ));
        };
        {
            let cached = self.token.lock().expect("token mutex");
            if let Some(entry) = cached.as_ref() {
                if entry.expires_at > Instant::now() {
                    return Ok(entry.token.clone());
                }
            }
        }
        let credentials = format!("{}:{}", forum.client_id, forum.client_secret);
        use base64::Engine;
        let authorization = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
        );
        let url = format!("{}/oauth/token", forum.auth_base_url);
        let body = format!(
            "grant_type=client_credentials&resource={}&scope={}",
            forum.audience, "forum.read forum.write"
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
        let forum_origin = self
            .config
            .forum
            .as_ref()
            .map(|forum| forum.forum_origin.clone())
            .unwrap_or_default();
        let url = format!(
            "{forum_origin}/api/threads?contextType=workflow_instance&contextId={}&limit=1",
            workflow_instance_id
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
        let forum_origin = self
            .config
            .forum
            .as_ref()
            .map(|forum| forum.forum_origin.clone())
            .unwrap_or_default();
        let url = format!("{forum_origin}/api/threads");
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
        let forum_origin = self
            .config
            .forum
            .as_ref()
            .map(|forum| forum.forum_origin.clone())
            .unwrap_or_default();
        let url = format!("{forum_origin}/api/threads/{thread_id}/messages");
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
///
/// Channels are drained independently: a channel that is not configured never
/// claims its rows, and one channel's failure never blocks another enabled
/// channel in the same pass. Public so integration tests can drive exactly
/// one pass and assert on durable row state.
pub async fn run_once(pool: &PgPool, forum: &ForumClient) -> usize {
    let mut delivered = 0usize;

    // 1. Ensure pending bindings (batch of 20, oldest first) — only the Forum
    //    channel owns the binding fact.
    if forum.config.forum.is_some() {
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
            }
        }
    }

    // 2. Drain each enabled channel's own kind. Batch queries are kind-scoped
    //    and rows are claimed FOR UPDATE SKIP LOCKED inside the delivery
    //    transaction.
    for kind in ["EXECUTION_KICK", "FORUM_EVENT", "OWNER_ASSISTANCE_WAKE"] {
        let channel_active = match kind {
            "EXECUTION_KICK" => forum.config.kick.is_some(),
            "FORUM_EVENT" => forum.config.forum.is_some(),
            _ => forum.config.wake_channel_active(),
        };
        if channel_active {
            delivered += drain_channel(pool, forum, kind).await;
        }
    }
    delivered
}

/// Claim-and-deliver every due row of one channel's kind.
async fn drain_channel(pool: &PgPool, forum: &ForumClient, kind: &str) -> usize {
    let rows = match outbox::next_pending_batch(pool, kind, 20).await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(channel = kind, error = %error, "outbox batch query failed");
            return 0;
        }
    };
    let mut delivered = 0usize;
    for candidate in rows {
        // Governance review wakes coordinate their own retirement/refresh
        // transaction — which itself settles the outbox row whenever a wake
        // is retired — so it must run WITHOUT the claim lock held, or its
        // outbox UPDATE would block on our own claim. After it approves
        // delivery the row is claimed and sent under the usual short lock;
        // a concurrent worker settling the row first just skips the send.
        if kind == "OWNER_ASSISTANCE_WAKE" && candidate.payload.get("phase").is_some() {
            match crate::store::postgres::workflow_instance_repository::assistance_transaction::governance_child::prepare_delivery(pool, candidate.outbox_id, &candidate.payload).await {
                Ok(true) => {}
                Ok(false) => continue,
                Err(error) => {
                    let message = error.to_string();
                    let settled = async {
                        let mut tx = pool.begin().await?;
                        outbox::mark_attempt_failed(&mut tx, candidate.outbox_id, &message, None).await?;
                        tx.commit().await
                    }
                    .await;
                    if let Err(error) = settled {
                        tracing::warn!(channel = kind, outbox_id = %candidate.outbox_id, error = %error, "wake prepare failure settle failed; row keeps its due state");
                    }
                    continue;
                }
            }
        }
        match deliver_claimed(pool, forum, kind, candidate.outbox_id).await {
            Ok(true) => delivered += 1,
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(channel = kind, outbox_id = %candidate.outbox_id, error = %error, "outbox drain step failed; row keeps its due state");
            }
        }
    }
    delivered
}

/// Claim one row (`FOR UPDATE SKIP LOCKED` inside the delivery transaction),
/// deliver it, and settle the outcome in the same transaction. Ok(true) =
/// delivered by this worker; Ok(false) = another worker holds the claim or
/// the row was settled between the batch query and the claim. The claim lock
/// doubles as the lease: a crashed worker's transaction dies with its
/// connection, releasing the row for the next pass. Governance review wakes
/// are prepared (and approved) before claiming; plain wakes deliver directly.
async fn deliver_claimed(
    pool: &PgPool,
    forum: &ForumClient,
    kind: &str,
    outbox_id: Uuid,
) -> Result<bool, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let Some(row) = outbox::claim_row(&mut tx, outbox_id, kind)
        .await
        .map_err(|e| e.to_string())?
    else {
        tx.commit().await.map_err(|e| e.to_string())?;
        return Ok(false);
    };
    let started = Instant::now();
    let result: Result<(), ChannelFailure> = match kind {
        "FORUM_EVENT" => deliver_forum_event(pool, forum, &row)
            .await
            .map_err(ChannelFailure::Retryable),
        "EXECUTION_KICK" => deliver_kick(forum, &row).await,
        "OWNER_ASSISTANCE_WAKE" => deliver_owner_assistance_wake(forum, &row)
            .await
            .map_err(ChannelFailure::Retryable),
        other => Err(ChannelFailure::Retryable(format!(
            "unknown outbox kind {other}"
        ))),
    };
    let outcome = match &result {
        Ok(()) => {
            outbox::mark_delivered(&mut tx, row.outbox_id)
                .await
                .map_err(|e| e.to_string())?;
            OutboxOutcome::Delivered
        }
        Err(ChannelFailure::Permanent(message)) => {
            outbox::mark_dead_lettered(&mut tx, row.outbox_id, message)
                .await
                .map_err(|e| e.to_string())?;
            OutboxOutcome::DeadLettered
        }
        Err(ChannelFailure::Retryable(message)) => {
            // Only the latency-only kick channel dead-letters at the retry
            // cap; the correctness-path channels keep their never-dead-letter
            // discipline (no silent loss of a forum projection or an owner
            // notification).
            let cap = if kind == "EXECUTION_KICK" {
                Some(outbox::EXECUTION_KICK_MAX_ATTEMPTS)
            } else {
                None
            };
            outbox::mark_attempt_failed(&mut tx, row.outbox_id, message, cap)
                .await
                .map_err(|e| e.to_string())?;
            OutboxOutcome::RetryScheduled
        }
    };
    tx.commit().await.map_err(|e| e.to_string())?;
    let duration_ms = started.elapsed().as_millis() as u64;
    match &outcome {
        OutboxOutcome::Delivered => {
            tracing::info!(channel = kind, outbox_id = %row.outbox_id, event_key = %row.event_key, attempt = row.attempt_count, duration_ms, "outbox row delivered");
        }
        OutboxOutcome::RetryScheduled => {
            tracing::warn!(channel = kind, outbox_id = %row.outbox_id, event_key = %row.event_key, attempt = row.attempt_count, duration_ms, error = result.as_ref().err().map(|failure| failure.message()).unwrap_or_default(), "outbox row delivery failed — retry scheduled with backoff");
        }
        OutboxOutcome::DeadLettered => {
            tracing::error!(channel = kind, outbox_id = %row.outbox_id, event_key = %row.event_key, error = result.as_ref().err().map(|failure| failure.message()).unwrap_or_default(), "outbox row dead-lettered — payload and failure reason retained; recovery via the migrations/0032 requeue statement");
        }
    }
    Ok(matches!(outcome, OutboxOutcome::Delivered))
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

/// Deliver one EXECUTION_KICK row to the Agent Core gateway. The kick is a
/// latency optimization only: 200 (kicked or coalesced) is the only success,
/// 401/403 are auth/permission failures (retryable — the token is repairable
/// and must never be confused with a successful delivery), 408/409/429/5xx
/// and transport failures are retryable, and every other 4xx is a permanent
/// rejection of the payload.
async fn deliver_kick(forum: &ForumClient, row: &outbox::OutboxRow) -> Result<(), ChannelFailure> {
    let kick = forum.kick_config().ok_or_else(|| {
        ChannelFailure::Permanent(
            "kick endpoint not configured (WORKFLOW_EXECUTION_KICK_URL/TOKEN)".to_string(),
        )
    })?;
    let started = Instant::now();
    let response = forum
        .kick_http
        .post(&kick.url)
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", kick.token),
        )
        // Stable request identity for audit/replay correlation (the gateway
        // currently ignores it; it must never contradict the payload).
        .header("idempotency-key", row.event_key.as_str())
        .json(&row.payload)
        .timeout(Duration::from_secs(5))
        .send()
        .await;
    let duration_ms = started.elapsed().as_millis() as u64;
    match response {
        Err(error) if error.is_timeout() => Err(ChannelFailure::Retryable(format!(
            "kick request timed out after {duration_ms}ms — delivery result UNKNOWN; replay is safe by contract (a kick only triggers one coalesced poll)"
        ))),
        Err(error) => Err(ChannelFailure::Retryable(format!(
            "kick transport failure: {error}"
        ))),
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                return Ok(());
            }
            let code = status.as_u16();
            match code {
                401 | 403 => Err(ChannelFailure::Retryable(format!(
                    "kick endpoint {code}: authentication/authorization failure — verify WORKFLOW_EXECUTION_KICK_TOKEN and its workflow.execute scope"
                ))),
                408 | 409 | 429 => Err(ChannelFailure::Retryable(format!(
                    "kick endpoint {code}"
                ))),
                code if (500..600).contains(&code) => Err(ChannelFailure::Retryable(format!(
                    "kick endpoint 5xx: {code}"
                ))),
                _ => Err(ChannelFailure::Permanent(format!(
                    "kick endpoint {code}: permanent rejection — the receiver deterministically rejected the payload"
                ))),
            }
        }
    }
}

async fn deliver_owner_assistance_wake(
    forum: &ForumClient,
    row: &outbox::OutboxRow,
) -> DeliverResult {
    let config = forum
        .config
        .owner_assistance_wake
        .as_ref()
        .map_err(|error| *error)?
        .as_ref()
        .ok_or("owner assistance wake configuration is incomplete or invalid")?;
    let (key, bearer) = forum.owner_wake_bearer(config).await?;
    let response = forum
        .owner_wake_http
        .post(&config.url)
        .bearer_auth(&bearer)
        .json(&row.payload)
        .send()
        .await
        .map_err(|_| "owner assistance wake request failed".to_string())?;
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        if status == reqwest::StatusCode::UNAUTHORIZED {
            forum.invalidate_owner_wake_bearer(&key, &bearer).await;
        }
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

/// The background loop. Spawned by main when any channel is enabled.
pub async fn run_loop(pool: PgPool, config: OutboxReconcilerConfig) {
    tracing::info!(channels = ?config.enabled_channel_names(), interval_ms = config.poll_interval_ms, "outbox reconciler loop started");
    let forum = std::sync::Arc::new(ForumClient::new(config));
    let mut ticker = tokio::time::interval(Duration::from_millis(forum.config.poll_interval_ms));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        // Each pass runs in its own supervised task: an unexpected panic in
        // one pass is contained, logged, and the loop continues with the
        // next tick instead of silently killing every channel until process
        // restart. (Errors are always isolated per row/channel; only a panic
        // would take the whole pass down, so it is contained here.)
        let pass = {
            let pass_pool = pool.clone();
            let pass_forum = forum.clone();
            tokio::spawn(async move { run_once(&pass_pool, &pass_forum).await })
        };
        match pass.await {
            Ok(delivered) => {
                if delivered > 0 {
                    tracing::info!(delivered, "outbox reconciler delivered rows");
                }
            }
            Err(join_error) => {
                tracing::error!(error = %join_error, "outbox reconciler pass panicked — contained; the loop continues with the next tick");
            }
        }
    }
}

#[cfg(test)]
mod owner_assistance_wake_tests {
    use super::*;
    use axum::{
        extract::State,
        http::{HeaderMap, StatusCode},
        routing::post,
        Json, Router,
    };
    use std::sync::{Arc, Mutex as StdMutex};

    #[derive(Clone, Default)]
    struct Capture {
        authorization: Arc<StdMutex<Option<String>>>,
        body: Arc<StdMutex<Option<serde_json::Value>>>,
        auth_authorization: Arc<StdMutex<Option<String>>>,
        auth_body: Arc<StdMutex<Option<String>>>,
        idempotency_key: Arc<StdMutex<Option<String>>>,
        auth_calls: Arc<std::sync::atomic::AtomicUsize>,
        wake_calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    fn client(url: String, token: &str) -> ForumClient {
        let parsed = reqwest::Url::parse(&url).unwrap();
        let auth_base_url = format!(
            "{}://{}:{}",
            parsed.scheme(),
            parsed.host_str().unwrap(),
            parsed.port().unwrap()
        );
        ForumClient::new(OutboxReconcilerConfig {
            forum: Some(ForumEventConfig {
                auth_base_url: auth_base_url.clone(),
                client_id: "unused".to_string(),
                client_secret: "unused".to_string(),
                forum_origin: "http://127.0.0.1:9".to_string(),
                audience: "svc-forum".to_string(),
            }),
            poll_interval_ms: 5_000,
            kick: Some(ExecutionKickConfig {
                url,
                token: token.to_string(),
            }),
            owner_assistance_wake: Ok(Some(OwnerAssistanceWakeConfig {
                url: format!(
                    "{}://{}:{}/workflow-execution/owner-assistance-wakes",
                    parsed.scheme(),
                    parsed.host_str().unwrap(),
                    parsed.port().unwrap()
                ),
                auth_base_url,
                client_id: "owner-wake-client".to_string(),
                client_secret: "owner-wake-secret".to_string(),
                audience: "svc-workflow".to_string(),
            })),
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
        capture
            .wake_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        *capture.authorization.lock().unwrap() = headers
            .get(reqwest::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        *capture.idempotency_key.lock().unwrap() = headers
            .get("idempotency-key")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        *capture.body.lock().unwrap() = Some(body);
        StatusCode::OK
    }

    async fn token_handler(
        State(capture): State<Capture>,
        headers: HeaderMap,
        body: String,
    ) -> Json<serde_json::Value> {
        capture
            .auth_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        *capture.auth_authorization.lock().unwrap() = headers
            .get(reqwest::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        *capture.auth_body.lock().unwrap() = Some(body);
        Json(serde_json::json!({
            "access_token": "fresh-token",
            "token_type": "Bearer",
            "expires_in": 600,
            "scope": "workflow.execute"
        }))
    }

    async fn short_token_handler(State(capture): State<Capture>) -> Json<serde_json::Value> {
        capture
            .auth_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Json(serde_json::json!({
            "access_token": "short-token",
            "token_type": "Bearer",
            "expires_in": 1,
            "scope": "workflow.execute"
        }))
    }

    async fn slow_token_handler(State(capture): State<Capture>) -> Json<serde_json::Value> {
        capture
            .auth_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(950)).await;
        Json(serde_json::json!({
            "access_token": "too-late-token",
            "token_type": "Bearer",
            "expires_in": 1,
            "scope": "workflow.execute"
        }))
    }

    async fn invalid_token_handler() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "access_token": "synthetic-invalid-token",
            "token_type": "Bearer",
            "expires_in": 600,
            "scope": "workflow.read"
        }))
    }

    async fn forum_token_handler(State(capture): State<Capture>) -> Json<serde_json::Value> {
        capture
            .auth_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Json(serde_json::json!({
            "access_token": "forum-token",
            "expires_in": 300
        }))
    }

    async fn unauthorized_then_ok(State(capture): State<Capture>) -> StatusCode {
        let attempt = capture
            .wake_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if attempt == 0 {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::OK
        }
    }

    async fn unavailable_then_ok(State(capture): State<Capture>) -> StatusCode {
        let attempt = capture
            .wake_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if attempt == 0 {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::NO_CONTENT
        }
    }

    async fn redirect_to_wake() -> (StatusCode, [(axum::http::HeaderName, String); 1]) {
        (
            StatusCode::TEMPORARY_REDIRECT,
            [(
                reqwest::header::LOCATION,
                "/workflow-execution/owner-assistance-wakes".to_string(),
            )],
        )
    }

    async fn redirect_to_target() -> axum::response::Redirect {
        axum::response::Redirect::temporary("/redirect-target")
    }

    #[tokio::test]
    async fn owner_wake_uses_dedicated_credentials_and_forwards_payload() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(capture_handler),
            )
            .route("/oauth/token", post(token_handler))
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
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "push-secret",
        );
        deliver_owner_assistance_wake(&forum, &row(payload.clone()))
            .await
            .unwrap();

        assert_eq!(
            capture.authorization.lock().unwrap().as_deref(),
            Some("Bearer fresh-token")
        );
        assert_eq!(capture.body.lock().unwrap().as_ref(), Some(&payload));
        assert_eq!(
            capture.auth_authorization.lock().unwrap().as_deref(),
            Some("Basic b3duZXItd2FrZS1jbGllbnQ6b3duZXItd2FrZS1zZWNyZXQ=")
        );
        assert_eq!(
            capture.auth_body.lock().unwrap().as_deref(),
            Some("grant_type=client_credentials&resource=svc-workflow&scope=workflow.execute")
        );
        server.abort();
    }

    #[tokio::test]
    async fn owner_wake_keeps_non_success_retryable() {
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(|| async { StatusCode::SERVICE_UNAVAILABLE }),
            )
            .route(
                "/oauth/token",
                post(|| async {
                    Json(serde_json::json!({
                        "access_token": "fresh-token", "token_type": "Bearer", "expires_in": 600,
                        "scope": "workflow.execute"
                    }))
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let forum = client(
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "push-secret",
        );
        let error = deliver_owner_assistance_wake(&forum, &row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(error.contains("503"));
        server.abort();
    }

    #[tokio::test]
    async fn owner_wake_cache_is_reused_and_401_only_invalidates_for_later_attempt() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(unauthorized_then_ok),
            )
            .route("/oauth/token", post(token_handler))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let forum = client(
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "push-secret",
        );
        let row = row(serde_json::json!({"workflowInstanceId": Uuid::new_v4()}));

        let error = deliver_owner_assistance_wake(&forum, &row)
            .await
            .unwrap_err();
        assert!(error.contains("401"));
        assert_eq!(
            capture.auth_calls.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert_eq!(
            capture.wake_calls.load(std::sync::atomic::Ordering::SeqCst),
            1
        );

        deliver_owner_assistance_wake(&forum, &row).await.unwrap();
        assert_eq!(
            capture.auth_calls.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        assert_eq!(
            capture.wake_calls.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        server.abort();
    }

    #[tokio::test]
    async fn owner_wake_reacquires_before_short_token_renewal_point() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(capture_handler),
            )
            .route("/oauth/token", post(short_token_handler))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let forum = client(
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "push-secret",
        );
        let row = row(serde_json::json!({"workflowInstanceId": Uuid::new_v4()}));

        deliver_owner_assistance_wake(&forum, &row).await.unwrap();
        tokio::time::sleep(Duration::from_millis(950)).await;
        deliver_owner_assistance_wake(&forum, &row).await.unwrap();
        assert_eq!(
            capture.auth_calls.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        server.abort();
    }

    #[tokio::test]
    async fn owner_wake_does_not_use_token_arriving_after_renewal_point() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(capture_handler),
            )
            .route("/oauth/token", post(slow_token_handler))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let forum = client(
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "push-secret",
        );
        let error = deliver_owner_assistance_wake(&forum, &row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(error.contains("renewal point"));
        assert_eq!(
            capture.wake_calls.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert_eq!(forum.owner_wake_tokens.lock().await.len(), 0);
        server.abort();
    }

    #[tokio::test]
    async fn owner_wake_concurrent_cache_misses_share_one_auth_request() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(capture_handler),
            )
            .route("/oauth/token", post(token_handler))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let forum = client(
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "push-secret",
        );
        let first = row(serde_json::json!({"attempt": 1}));
        let second = row(serde_json::json!({"attempt": 2}));

        let (first_result, second_result) = tokio::join!(
            deliver_owner_assistance_wake(&forum, &first),
            deliver_owner_assistance_wake(&forum, &second),
        );
        first_result.unwrap();
        second_result.unwrap();
        assert_eq!(
            capture.auth_calls.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert_eq!(
            capture.wake_calls.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        server.abort();
    }

    #[tokio::test]
    async fn owner_wake_missing_dedicated_config_fails_without_static_fallback() {
        let mut forum = client(
            "http://127.0.0.1:9/workflow-execution/owner-assistance-wakes".to_string(),
            "legacy-static-token",
        );
        forum.config.owner_assistance_wake =
            Err("owner assistance wake configuration is incomplete or invalid");
        let error = deliver_owner_assistance_wake(&forum, &row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(error.contains("configuration is incomplete or invalid"));
        assert!(!error.contains("legacy-static-token"));
    }

    #[tokio::test]
    async fn owner_wake_rejects_bad_auth_response_without_leaking_token() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(capture_handler),
            )
            .route("/oauth/token", post(invalid_token_handler))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let forum = client(
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "push-secret",
        );
        let error = deliver_owner_assistance_wake(&forum, &row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(error.contains("invalid token response"));
        assert!(!error.contains("synthetic-invalid-token"));
        assert!(!error.contains("owner-wake-secret"));
        assert_eq!(
            capture.wake_calls.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        server.abort();
    }

    #[tokio::test]
    async fn owner_wake_rejects_auth_redirect_without_following_it() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(capture_handler),
            )
            .route("/oauth/token", post(redirect_to_wake))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let forum = client(
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "push-secret",
        );
        let error = deliver_owner_assistance_wake(&forum, &row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(error.contains("302") || error.contains("307"));
        assert_eq!(
            capture.wake_calls.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        server.abort();
    }

    #[tokio::test]
    async fn owner_wake_rejects_product_redirect_without_following_it() {
        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(redirect_to_target),
            )
            .route("/redirect-target", post(capture_handler))
            .route("/oauth/token", post(token_handler))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let forum = client(
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "push-secret",
        );
        let error = deliver_owner_assistance_wake(&forum, &row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(error.contains("307"));
        assert_eq!(
            capture.wake_calls.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        server.abort();
    }

    #[tokio::test]
    async fn legacy_execution_kick_still_uses_its_static_token() {
        let capture = Capture::default();
        let app = Router::new()
            .route("/workflow-execution/kicks", post(capture_handler))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let mut forum = client(
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "legacy-kick-token",
        );
        forum.config.kick.as_mut().unwrap().url = format!("http://{addr}/workflow-execution/kicks");
        let payload = serde_json::json!({"dispatchIntentId": Uuid::new_v4()});
        deliver_kick(&forum, &row(payload.clone())).await.unwrap();
        assert_eq!(
            capture.authorization.lock().unwrap().as_deref(),
            Some("Bearer legacy-kick-token")
        );
        assert_eq!(capture.body.lock().unwrap().as_ref(), Some(&payload));
        server.abort();
    }

    #[tokio::test]
    async fn existing_forum_token_cache_remains_separate_and_reusable() {
        let capture = Capture::default();
        let app = Router::new()
            .route("/oauth/token", post(forum_token_handler))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let forum = client(
            format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
            "legacy-kick-token",
        );

        assert_eq!(forum.bearer().await.unwrap(), "forum-token");
        assert_eq!(forum.bearer().await.unwrap(), "forum-token");
        assert_eq!(
            capture.auth_calls.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        server.abort();
    }

    // ── EXECUTION_KICK failure classification ───────────────────────────────

    fn kick_only_client(url: String) -> ForumClient {
        ForumClient::new(OutboxReconcilerConfig {
            forum: None,
            poll_interval_ms: 5_000,
            kick: Some(ExecutionKickConfig {
                url,
                token: "kick-token".to_string(),
            }),
            owner_assistance_wake: Ok(None),
        })
    }

    fn kick_row(payload: serde_json::Value) -> outbox::OutboxRow {
        outbox::OutboxRow {
            outbox_id: Uuid::new_v4(),
            workflow_instance_id: Uuid::new_v4(),
            outbox_kind: "EXECUTION_KICK".to_string(),
            event_key: format!("kick:{}", Uuid::new_v4()),
            payload,
            attempt_count: 0,
        }
    }

    async fn kick_status_server(status: StatusCode) -> (ForumClient, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/workflow-execution/kicks",
            post(move || async move { status }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (
            kick_only_client(format!("http://{addr}/workflow-execution/kicks")),
            server,
        )
    }

    #[tokio::test]
    async fn kick_401_and_403_are_retryable_auth_failures_not_deliveries() {
        for status in [StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN] {
            let (forum, server) = kick_status_server(status).await;
            let failure = deliver_kick(&forum, &kick_row(serde_json::json!({})))
                .await
                .unwrap_err();
            match failure {
                ChannelFailure::Retryable(message) => {
                    assert!(
                        message.contains("authentication/authorization"),
                        "{message}"
                    );
                }
                other => panic!("{status} must be retryable, got {other:?}"),
            }
            server.abort();
        }
    }

    #[tokio::test]
    async fn kick_409_has_no_contract_semantics_and_is_retryable() {
        let (forum, server) = kick_status_server(StatusCode::CONFLICT).await;
        let failure = deliver_kick(&forum, &kick_row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(matches!(failure, ChannelFailure::Retryable(_)));
        server.abort();
    }

    #[tokio::test]
    async fn kick_5xx_is_retryable() {
        let (forum, server) = kick_status_server(StatusCode::BAD_GATEWAY).await;
        let failure = deliver_kick(&forum, &kick_row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(matches!(failure, ChannelFailure::Retryable(message) if message.contains("5xx")));
        server.abort();
    }

    #[tokio::test]
    async fn kick_400_is_permanent() {
        let (forum, server) = kick_status_server(StatusCode::BAD_REQUEST).await;
        let failure = deliver_kick(&forum, &kick_row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(matches!(failure, ChannelFailure::Permanent(_)));
        server.abort();
    }

    #[tokio::test]
    async fn kick_transport_failure_is_retryable() {
        // Closed port: the delivery result is UNKNOWN from this side.
        let forum = kick_only_client("http://127.0.0.1:1/workflow-execution/kicks".to_string());
        let failure = deliver_kick(&forum, &kick_row(serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(matches!(failure, ChannelFailure::Retryable(_)));
    }

    #[tokio::test]
    async fn kick_success_carries_stable_idempotency_key() {
        let capture = Capture::default();
        let app = Router::new()
            .route("/workflow-execution/kicks", post(capture_handler))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let forum = kick_only_client(format!("http://{addr}/workflow-execution/kicks"));
        let row = kick_row(serde_json::json!({"dispatchIntentId": Uuid::new_v4()}));
        deliver_kick(&forum, &row).await.unwrap();
        assert_eq!(
            capture.authorization.lock().unwrap().as_deref(),
            Some("Bearer kick-token")
        );
        assert_eq!(
            capture.idempotency_key.lock().unwrap().as_deref(),
            Some(row.event_key.as_str())
        );
        server.abort();
    }

    // ── channel configuration matrix ────────────────────────────────────────

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct EnvGuard(Vec<&'static str>);
    impl EnvGuard {
        fn new(vars: &[&'static str]) -> Self {
            for name in vars {
                std::env::remove_var(name);
            }
            Self(vars.to_vec())
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for name in &self.0 {
                std::env::remove_var(name);
            }
        }
    }

    const CONFIG_VARS: &[&str] = &[
        "WORKFLOW_FORUM_SYNC_ENABLED",
        "WORKFLOW_FORUM_AUTH_BASE_URL",
        "WORKFLOW_FORUM_CLIENT_ID",
        "WORKFLOW_FORUM_CLIENT_SECRET",
        "WORKFLOW_EXECUTION_KICK_ENABLED",
        "WORKFLOW_EXECUTION_KICK_URL",
        "WORKFLOW_EXECUTION_KICK_TOKEN",
        "WORKFLOW_OWNER_ASSISTANCE_WAKE_ENABLED",
        "WORKFLOW_OWNER_ASSISTANCE_WAKE_URL",
        "WORKFLOW_OWNER_ASSISTANCE_WAKE_AUTH_BASE_URL",
        "WORKFLOW_OWNER_ASSISTANCE_WAKE_CLIENT_ID",
        "WORKFLOW_OWNER_ASSISTANCE_WAKE_CLIENT_SECRET",
        "WORKFLOW_OWNER_ASSISTANCE_WAKE_AUDIENCE",
    ];

    #[test]
    fn kick_only_env_starts_the_reconciler_without_any_forum_dependency() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::new(CONFIG_VARS);
        std::env::set_var("WORKFLOW_EXECUTION_KICK_ENABLED", "1");
        std::env::set_var(
            "WORKFLOW_EXECUTION_KICK_URL",
            "http://127.0.0.1:8080/workflow-execution/kicks",
        );
        std::env::set_var("WORKFLOW_EXECUTION_KICK_TOKEN", "kick-token");

        let config = OutboxReconcilerConfig::from_env();
        assert!(config.kick.is_some());
        assert!(config.forum.is_none());
        assert!(config.any_channel_enabled());
        assert_eq!(config.enabled_channel_names(), vec!["EXECUTION_KICK"]);
    }

    #[test]
    fn legacy_forum_master_switch_nesting_still_enables_kick_and_wake() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::new(CONFIG_VARS);
        std::env::set_var("WORKFLOW_FORUM_SYNC_ENABLED", "1");
        std::env::set_var("WORKFLOW_FORUM_AUTH_BASE_URL", "http://127.0.0.1:4001");
        std::env::set_var("WORKFLOW_FORUM_CLIENT_ID", "forum-client");
        std::env::set_var("WORKFLOW_FORUM_CLIENT_SECRET", "forum-secret");
        std::env::set_var(
            "WORKFLOW_EXECUTION_KICK_URL",
            "http://127.0.0.1:8080/workflow-execution/kicks",
        );
        std::env::set_var("WORKFLOW_EXECUTION_KICK_TOKEN", "kick-token");
        std::env::set_var(
            "WORKFLOW_OWNER_ASSISTANCE_WAKE_URL",
            "http://127.0.0.1:8080/workflow-execution/owner-assistance-wakes",
        );
        std::env::set_var(
            "WORKFLOW_OWNER_ASSISTANCE_WAKE_AUTH_BASE_URL",
            "http://127.0.0.1:4001",
        );
        std::env::set_var("WORKFLOW_OWNER_ASSISTANCE_WAKE_CLIENT_ID", "wake-client");
        std::env::set_var(
            "WORKFLOW_OWNER_ASSISTANCE_WAKE_CLIENT_SECRET",
            "wake-secret",
        );
        std::env::set_var("WORKFLOW_OWNER_ASSISTANCE_WAKE_AUDIENCE", "svc-workflow");

        let config = OutboxReconcilerConfig::from_env();
        assert!(config.forum.is_some());
        assert!(config.kick.is_some());
        assert!(config.wake_channel_active());
        assert_eq!(
            config.enabled_channel_names(),
            vec!["EXECUTION_KICK", "FORUM_EVENT", "OWNER_ASSISTANCE_WAKE"]
        );
    }

    #[test]
    fn explicit_kick_disable_wins_over_legacy_nesting() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::new(CONFIG_VARS);
        std::env::set_var("WORKFLOW_FORUM_SYNC_ENABLED", "1");
        std::env::set_var("WORKFLOW_FORUM_AUTH_BASE_URL", "http://127.0.0.1:4001");
        std::env::set_var("WORKFLOW_FORUM_CLIENT_ID", "forum-client");
        std::env::set_var("WORKFLOW_FORUM_CLIENT_SECRET", "forum-secret");
        std::env::set_var("WORKFLOW_EXECUTION_KICK_ENABLED", "0");
        std::env::set_var(
            "WORKFLOW_EXECUTION_KICK_URL",
            "http://127.0.0.1:8080/workflow-execution/kicks",
        );
        std::env::set_var("WORKFLOW_EXECUTION_KICK_TOKEN", "kick-token");

        let config = OutboxReconcilerConfig::from_env();
        assert!(config.forum.is_some());
        assert!(config.kick.is_none());
        assert_eq!(config.enabled_channel_names(), vec!["FORUM_EVENT"]);
    }

    #[test]
    fn kick_enabled_without_credentials_stays_dormant() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::new(CONFIG_VARS);
        std::env::set_var("WORKFLOW_EXECUTION_KICK_ENABLED", "1");

        let config = OutboxReconcilerConfig::from_env();
        assert!(config.kick.is_none());
        assert!(!config.any_channel_enabled());
    }

    #[test]
    fn kick_url_must_be_loopback_kicks_path() {
        assert!(validate_kick_url("http://127.0.0.1:8080/workflow-execution/kicks").is_ok());
        assert!(validate_kick_url("http://localhost:8788/workflow-execution/kicks").is_ok());
        assert!(validate_kick_url("http://127.0.0.1:8080/workflow-execution/kicks/").is_ok());
        assert!(validate_kick_url("http://[::1]:8080/workflow-execution/kicks").is_ok());
        assert!(validate_kick_url("http://example.test/workflow-execution/kicks").is_err());
        assert!(validate_kick_url("https://example.test/workflow-execution/kicks").is_err());
        assert!(validate_kick_url(
            "http://127.0.0.1:8080/workflow-execution/owner-assistance-wakes"
        )
        .is_err());
        assert!(validate_kick_url("http://127.0.0.1:8080/workflow-execution/kicks?x=1").is_err());
    }

    #[test]
    fn nothing_configured_means_fully_dormant() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::new(CONFIG_VARS);

        let config = OutboxReconcilerConfig::from_env();
        assert!(!config.any_channel_enabled());
        assert!(config.enabled_channel_names().is_empty());
    }

    #[test]
    fn owner_wake_urls_are_constrained_to_loopback_and_exact_route() {
        assert!(validate_wake_url(
            "http://127.0.0.1:3000/workflow-execution/owner-assistance-wakes"
        )
        .is_ok());
        assert!(validate_wake_url(
            "https://127.0.0.1:3000/workflow-execution/owner-assistance-wakes"
        )
        .is_err());
        assert!(
            validate_wake_url("http://example.test/workflow-execution/owner-assistance-wakes")
                .is_err()
        );
        assert!(validate_wake_url(
            "http://127.0.0.1:3000/workflow-execution/owner-assistance-wakes?x=1"
        )
        .is_err());
        assert!(validate_auth_base_url("https://auth.example.test").is_ok());
        assert!(validate_auth_base_url("http://127.0.0.1:3001").is_ok());
        assert!(validate_auth_base_url("http://[::1]:3001").is_ok());
        assert!(validate_auth_base_url("http://localhost:3001").is_ok());
        assert!(validate_auth_base_url("http://auth.example.test").is_err());
        assert!(validate_auth_base_url("https://user:pass@auth.example.test").is_err());
    }

    #[test]
    fn owner_wake_configuration_is_absent_only_when_all_settings_are_absent() {
        assert!(
            OwnerAssistanceWakeConfig::from_values(std::array::from_fn(|_| None))
                .unwrap()
                .is_none()
        );

        let partial = [
            Some("http://127.0.0.1:3000/workflow-execution/owner-assistance-wakes".into()),
            None,
            None,
            None,
            None,
        ];
        assert!(OwnerAssistanceWakeConfig::from_values(partial).is_err());

        let blank = [
            Some(" ".into()),
            Some("http://127.0.0.1:3001".into()),
            Some("client".into()),
            Some("secret".into()),
            Some("aud".into()),
        ];
        assert!(OwnerAssistanceWakeConfig::from_values(blank).is_err());

        let complete = [
            Some("http://127.0.0.1:3000/workflow-execution/owner-assistance-wakes".into()),
            Some("http://127.0.0.1:3001".into()),
            Some("client".into()),
            Some("secret".into()),
            Some("aud".into()),
        ];
        assert!(OwnerAssistanceWakeConfig::from_values(complete)
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    #[ignore = "requires task-owned TEST_DATABASE_URL on loopback PostgreSQL"]
    async fn owner_wake_outbox_stays_pending_then_recovers_on_later_pass() {
        use sqlx::postgres::PgPoolOptions;
        let database_url = std::env::var("OWNER_WAKE_TEST_DATABASE_URL")
            .expect("OWNER_WAKE_TEST_DATABASE_URL must name the unique task database");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("connect task-owned isolated PostgreSQL database");
        sqlx::query(
            "CREATE TEMP TABLE workflow_forum_bindings (
                workflow_instance_id UUID PRIMARY KEY,
                binding_state TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TEMP TABLE workflow_outbox (
                outbox_id UUID PRIMARY KEY,
                workflow_instance_id UUID NOT NULL,
                outbox_kind TEXT NOT NULL,
                event_key TEXT NOT NULL,
                payload JSONB NOT NULL,
                attempt_count INT NOT NULL DEFAULT 0,
                next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                delivered_at TIMESTAMPTZ,
                last_error TEXT,
                dead_lettered_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let capture = Capture::default();
        let app = Router::new()
            .route(
                "/workflow-execution/owner-assistance-wakes",
                post(unavailable_then_ok),
            )
            .route("/oauth/token", post(short_token_handler))
            .with_state(capture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let instance_id = Uuid::new_v4();
        let row_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_outbox
                (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
             VALUES ($1, $2, 'OWNER_ASSISTANCE_WAKE', 'owner-assistance:isolated-test', $3)",
        )
        .bind(row_id)
        .bind(instance_id)
        .bind(serde_json::json!({"workflowInstanceId": instance_id}))
        .execute(&pool)
        .await
        .unwrap();
        let config = OutboxReconcilerConfig {
            forum: Some(ForumEventConfig {
                auth_base_url: format!("http://{addr}"),
                client_id: "unused-for-forum-auth".to_string(),
                client_secret: "unused-for-forum-auth".to_string(),
                forum_origin: "http://127.0.0.1:9".to_string(),
                audience: "svc-forum".to_string(),
            }),
            poll_interval_ms: 5_000,
            kick: Some(ExecutionKickConfig {
                url: format!("http://{addr}/workflow-execution/kicks"),
                token: "must-not-be-used".to_string(),
            }),
            owner_assistance_wake: Ok(Some(OwnerAssistanceWakeConfig {
                url: format!("http://{addr}/workflow-execution/owner-assistance-wakes"),
                auth_base_url: format!("http://{addr}"),
                client_id: "task-owner-wake-client".to_string(),
                client_secret: "task-owner-wake-secret".to_string(),
                audience: "svc-workflow".to_string(),
            })),
        };
        let forum = ForumClient::new(config);

        assert_eq!(run_once(&pool, &forum).await, 0);
        let failed = sqlx::query(
            "SELECT attempt_count, delivered_at IS NULL AS pending, last_error FROM workflow_outbox WHERE outbox_id = $1",
        )
        .bind(row_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        use sqlx::Row;
        assert_eq!(failed.try_get::<i32, _>("attempt_count").unwrap(), 1);
        assert!(failed.try_get::<bool, _>("pending").unwrap());
        let failure = failed.try_get::<String, _>("last_error").unwrap();
        assert!(failure.contains("503"));
        assert!(!failure.contains("task-owner-wake-secret"));
        assert_eq!(
            capture.auth_calls.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert_eq!(
            capture.wake_calls.load(std::sync::atomic::Ordering::SeqCst),
            1
        );

        // Existing backoff is two seconds after the first attempt. At that
        // point the one-second synthetic token has crossed its renewal point.
        tokio::time::sleep(Duration::from_millis(2_100)).await;
        assert_eq!(run_once(&pool, &forum).await, 1);
        let delivered = sqlx::query(
            "SELECT attempt_count, delivered_at IS NOT NULL AS delivered, last_error FROM workflow_outbox WHERE outbox_id = $1",
        )
        .bind(row_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(delivered.try_get::<i32, _>("attempt_count").unwrap(), 1);
        assert!(delivered.try_get::<bool, _>("delivered").unwrap());
        assert!(delivered
            .try_get::<Option<String>, _>("last_error")
            .unwrap()
            .is_none());
        assert_eq!(
            capture.auth_calls.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        assert_eq!(
            capture.wake_calls.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        server.abort();
        pool.close().await;
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
