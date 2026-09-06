//! Canonical identity admission client for assignment-producing commands.
//!
//! Governing authority: `docs/specs/SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2.md`
//! CTR-CIR-003 (accepted). For each distinct Agent Principal supplied by an
//! assignment-producing command this client performs exactly two directory
//! reads, authenticating as svc-workflow's existing generic backend SERVICE
//! identity:
//!
//! 1. an OAuth2 client-credentials token per exact directory audience
//!    (`identity-directory` with `auth.directory.read`, `agent-directory`
//!    with `agent.directory.read`) — fresh per command, never cached across
//!    commands;
//! 2. the pinned Auth directory route
//!    `GET /api/v1/directory/principals/{principalId}/agent`;
//! 3. the pinned Agent-core directory route `GET /v1/directory/agents/{agentId}`
//!    using the exact returned ID.
//!
//! Admission requires `principalStatus == "active"`, `exists == true` and
//! `enabled == true`; anything else — including timeouts and unavailable
//! validators — is a fail-closed error for the whole command. No automatic
//! retry. The whole orchestration is bounded by a 5-second monotonic window
//! from the caller-provided command start. Plain HTTP is allowed only for
//! exact loopback endpoints; non-loopback configuration requires HTTPS and
//! redirects are forbidden.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use serde::Deserialize;
use tokio::sync::Semaphore;
use uuid::Uuid;

/// Total admission window in milliseconds, measured from the caller-provided
/// monotonic command start through the whole orchestration (CTR-CIR-003).
pub const ADMISSION_DEADLINE_MS: u64 = 5000;

/// Auth identity-directory audience for the Auth directory read.
pub const ADMISSION_AUDIENCE_IDENTITY_DIRECTORY: &str = "identity-directory";
/// Scope required by the Auth directory read.
pub const ADMISSION_SCOPE_AUTH_DIRECTORY_READ: &str = "auth.directory.read";
/// Agent-core directory audience for the Agent directory read.
pub const ADMISSION_AUDIENCE_AGENT_DIRECTORY: &str = "agent-directory";
/// Scope required by the Agent-core directory read.
pub const ADMISSION_SCOPE_AGENT_DIRECTORY_READ: &str = "agent.directory.read";

/// Maximum response body size for admission reads (1 MB).
const MAX_ADMISSION_BODY_BYTES: usize = 1_048_576;
/// Default and upper bound for in-flight admission reads (CTR-CIR-003: at most 8).
const DEFAULT_MAX_IN_FLIGHT: usize = 8;

const ENV_ENABLED: &str = "WORKFLOW_ADMISSION_ENABLED";
const ENV_AUTH_BASE_URL: &str = "WORKFLOW_ADMISSION_AUTH_BASE_URL";
const ENV_CORE_BASE_URL: &str = "WORKFLOW_ADMISSION_CORE_BASE_URL";
const ENV_CLIENT_ID: &str = "WORKFLOW_ADMISSION_CLIENT_ID";
const ENV_CLIENT_SECRET: &str = "WORKFLOW_ADMISSION_CLIENT_SECRET";
const ENV_DEADLINE_MS: &str = "WORKFLOW_ADMISSION_DEADLINE_MS";
const ENV_MAX_IN_FLIGHT: &str = "WORKFLOW_ADMISSION_MAX_IN_FLIGHT";

const DEFAULT_CLIENT_ID: &str = "svc-workflow";

/// A string holding secret material with a redacted `Debug` implementation,
/// so secrets never leak into logs, panics or sanitized error reports.
#[derive(Clone)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Access the secret value. Callers must never log or serialize it.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(***)")
    }
}

/// Which pinned admission endpoint an error or denial refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endpoint {
    /// Token endpoint `POST {AUTH_BASE}/oauth/token`.
    Token,
    /// Auth directory read `GET {AUTH_BASE}/api/v1/directory/principals/{uuid}/agent`.
    AuthRead,
    /// Agent-core directory read `GET {CORE_BASE}/v1/directory/agents/{agentId}`.
    CoreRead,
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Endpoint::Token => "token",
            Endpoint::AuthRead => "auth_read",
            Endpoint::CoreRead => "core_read",
        })
    }
}

/// Why an observed directory record failed the admission acceptance
/// predicates (these are rejections of the principal, not transport errors).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    /// Auth directory observed a principal status other than `active`.
    PrincipalNotActive(String),
    /// Agent-core directory observed `exists == false`.
    AgentMissing,
    /// Agent-core directory observed `enabled == false`.
    AgentDisabled,
}

impl fmt::Display for RejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RejectionReason::PrincipalNotActive(status) => {
                write!(f, "principal status is not active: {status}")
            }
            RejectionReason::AgentMissing => {
                write!(f, "agent missing in agent-core directory")
            }
            RejectionReason::AgentDisabled => {
                write!(f, "agent disabled in agent-core directory")
            }
        }
    }
}

/// Sanitized admission errors: `Display` and `Debug` never include secrets,
/// Authorization headers or response bodies.
#[derive(Debug, Clone)]
pub enum AdmissionError {
    /// Invalid or incomplete admission configuration.
    Config(String),
    /// The 5-second admission window was exhausted (including a start that
    /// was already past the deadline — no network is attempted).
    Timeout,
    /// A directory endpoint could not be reached.
    Unavailable,
    /// A directory endpoint answered with a non-200 status.
    Denied {
        endpoint: Endpoint,
        status: u16,
        code: String,
    },
    /// The principal was observed and failed an acceptance predicate.
    Rejected {
        principal: Uuid,
        reason: RejectionReason,
    },
    /// A response did not match the pinned response contract.
    MalformedResponse { endpoint: Endpoint, reason: String },
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdmissionError::Config(detail) => {
                write!(f, "admission configuration error: {detail}")
            }
            AdmissionError::Timeout => write!(f, "admission deadline exceeded"),
            AdmissionError::Unavailable => write!(f, "admission directory endpoint unavailable"),
            AdmissionError::Denied {
                endpoint,
                status,
                code,
            } => write!(
                f,
                "admission request denied by {endpoint} endpoint: status {status}, code {code}"
            ),
            AdmissionError::Rejected { principal, reason } => {
                write!(f, "principal {principal} rejected by admission: {reason}")
            }
            AdmissionError::MalformedResponse { endpoint, reason } => {
                write!(f, "malformed response from {endpoint} endpoint: {reason}")
            }
        }
    }
}

impl std::error::Error for AdmissionError {}

/// Successful observation of one Agent Principal across both directory reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedAgent {
    pub agent_id: String,
    pub principal_status: String,
    pub exists: bool,
    pub enabled: bool,
}

/// Result of one admission run: only successful admissions reach the report;
/// any rejection or error fails the whole command instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionReport {
    pub observations: BTreeMap<Uuid, AdmittedAgent>,
}

/// Admission configuration, loaded from the environment following the
/// `HttpConfig::from_env` style.
///
/// The client identity is svc-workflow's EXISTING canonical backend SERVICE
/// identity supplied by deployment env; the `svc-workflow` default exists
/// only so disabled-mode construction never fails.
#[derive(Clone, Debug)]
pub struct AdmissionConfig {
    pub enabled: bool,
    pub auth_base_url: String,
    pub core_base_url: String,
    pub client_id: String,
    pub client_secret: SecretString,
    pub deadline_ms: u64,
    pub max_in_flight: usize,
}

impl AdmissionConfig {
    pub fn from_env() -> Result<Self, AdmissionError> {
        let enabled = std::env::var(ENV_ENABLED).ok().as_deref() == Some("1");
        let auth_base_url = std::env::var(ENV_AUTH_BASE_URL).unwrap_or_default();
        let core_base_url = std::env::var(ENV_CORE_BASE_URL).unwrap_or_default();
        let client_id =
            std::env::var(ENV_CLIENT_ID).unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string());
        let client_secret = SecretString::new(std::env::var(ENV_CLIENT_SECRET).unwrap_or_default());

        let deadline_ms = parse_env(ENV_DEADLINE_MS, ADMISSION_DEADLINE_MS)?;
        if deadline_ms > ADMISSION_DEADLINE_MS {
            return Err(AdmissionError::Config(format!(
                "{ENV_DEADLINE_MS} must not exceed {ADMISSION_DEADLINE_MS}"
            )));
        }
        let max_in_flight = parse_env(ENV_MAX_IN_FLIGHT, DEFAULT_MAX_IN_FLIGHT)?;
        if max_in_flight == 0 {
            return Err(AdmissionError::Config(format!(
                "{ENV_MAX_IN_FLIGHT} must be at least 1"
            )));
        }
        if max_in_flight > DEFAULT_MAX_IN_FLIGHT {
            return Err(AdmissionError::Config(format!(
                "{ENV_MAX_IN_FLIGHT} must not exceed {DEFAULT_MAX_IN_FLIGHT}"
            )));
        }

        if enabled {
            if auth_base_url.is_empty() {
                return Err(AdmissionError::Config(format!(
                    "{ENV_AUTH_BASE_URL} is required when admission is enabled"
                )));
            }
            if core_base_url.is_empty() {
                return Err(AdmissionError::Config(format!(
                    "{ENV_CORE_BASE_URL} is required when admission is enabled"
                )));
            }
            if client_id.is_empty() {
                return Err(AdmissionError::Config(format!(
                    "{ENV_CLIENT_ID} must not be empty when admission is enabled"
                )));
            }
            if client_secret.expose().is_empty() {
                return Err(AdmissionError::Config(format!(
                    "{ENV_CLIENT_SECRET} is required when admission is enabled"
                )));
            }
        }

        Ok(Self {
            enabled,
            auth_base_url,
            core_base_url,
            client_id,
            client_secret,
            deadline_ms,
            max_in_flight,
        })
    }
}

/// Fail-closed admission client (CTR-CIR-003).
#[derive(Clone)]
pub struct AdmissionClient {
    config: AdmissionConfig,
    http_client: reqwest::Client,
    semaphore: Arc<Semaphore>,
}

impl AdmissionClient {
    /// Validate the configuration and build the client.
    ///
    /// Enforces the URL policy (plain HTTP only for exact loopback hosts;
    /// non-loopback requires HTTPS), the 5-second window bound and the
    /// 8-request in-flight bound.
    pub fn new(config: AdmissionConfig) -> Result<Self, AdmissionError> {
        let auth_base_url = validate_base_url(ENV_AUTH_BASE_URL, &config.auth_base_url)?;
        let core_base_url = validate_base_url(ENV_CORE_BASE_URL, &config.core_base_url)?;
        if config.deadline_ms == 0 || config.deadline_ms > ADMISSION_DEADLINE_MS {
            return Err(AdmissionError::Config(format!(
                "admission deadline must be between 1 and {ADMISSION_DEADLINE_MS} ms"
            )));
        }
        if config.max_in_flight == 0 || config.max_in_flight > DEFAULT_MAX_IN_FLIGHT {
            return Err(AdmissionError::Config(format!(
                "admission max in-flight must be between 1 and {DEFAULT_MAX_IN_FLIGHT}"
            )));
        }
        let semaphore = Arc::new(Semaphore::new(config.max_in_flight));
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| {
                AdmissionError::Config("failed to build admission HTTP client".to_string())
            })?;
        Ok(Self {
            config: AdmissionConfig {
                auth_base_url,
                core_base_url,
                ..config
            },
            http_client,
            semaphore,
        })
    }

    /// Remaining admission budget in milliseconds for the command that began
    /// at `start` (saturating at zero).
    pub fn remaining_budget_ms(&self, start: Instant) -> u64 {
        let elapsed_ms = start.elapsed().as_millis();
        (self.config.deadline_ms as u128).saturating_sub(elapsed_ms) as u64
    }

    /// Validate every distinct Agent Principal for one assignment-producing
    /// command. The whole orchestration — token acquisition plus both
    /// directory reads per principal — is bounded by the 5-second window
    /// from `start`. Any error rejects the entire command (fail-closed);
    /// only fully admitted principals appear in the report.
    pub async fn admit(
        &self,
        start: Instant,
        principals: &BTreeSet<Uuid>,
    ) -> Result<AdmissionReport, AdmissionError> {
        if principals.is_empty() {
            return Ok(AdmissionReport {
                observations: BTreeMap::new(),
            });
        }
        if !self.config.enabled {
            return Err(AdmissionError::Config("admission is disabled".to_string()));
        }

        // Fresh token per exact read audience per command; never cached
        // across commands (CTR-CIR-003).
        let auth_token = self
            .fetch_token(
                start,
                ADMISSION_AUDIENCE_IDENTITY_DIRECTORY,
                ADMISSION_SCOPE_AUTH_DIRECTORY_READ,
            )
            .await?;
        let core_token = self
            .fetch_token(
                start,
                ADMISSION_AUDIENCE_AGENT_DIRECTORY,
                ADMISSION_SCOPE_AGENT_DIRECTORY_READ,
            )
            .await?;

        let mut tasks = tokio::task::JoinSet::new();
        for principal in principals {
            let client = self.clone();
            let principal = *principal;
            let auth_token = auth_token.clone();
            let core_token = core_token.clone();
            tasks.spawn(async move {
                let result = client
                    .admit_one(start, principal, &auth_token, &core_token)
                    .await;
                (principal, result)
            });
        }

        let mut results: Vec<(Uuid, Result<AdmittedAgent, AdmissionError>)> = Vec::new();
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok(pair) => results.push(pair),
                Err(_) => return Err(AdmissionError::Unavailable),
            }
        }
        results.sort_by_key(|(principal, _)| *principal);

        let mut observations = BTreeMap::new();
        for (principal, result) in results {
            match result {
                Ok(agent) => {
                    observations.insert(principal, agent);
                }
                Err(error) => return Err(error),
            }
        }
        Ok(AdmissionReport { observations })
    }

    /// Validate one principal via the pinned Auth read and then the pinned
    /// Agent-core read, enforcing the acceptance predicates.
    async fn admit_one(
        &self,
        start: Instant,
        principal: Uuid,
        auth_token: &str,
        core_token: &str,
    ) -> Result<AdmittedAgent, AdmissionError> {
        let auth = self
            .read_auth_directory(start, principal, auth_token)
            .await?;
        if auth.principal_status != "active" {
            return Err(AdmissionError::Rejected {
                principal,
                reason: RejectionReason::PrincipalNotActive(auth.principal_status),
            });
        }

        let core = self
            .read_core_directory(start, &auth.agent_id, core_token)
            .await?;
        if !core.exists {
            return Err(AdmissionError::Rejected {
                principal,
                reason: RejectionReason::AgentMissing,
            });
        }
        if !core.enabled {
            return Err(AdmissionError::Rejected {
                principal,
                reason: RejectionReason::AgentDisabled,
            });
        }

        Ok(AdmittedAgent {
            agent_id: core.agent_id,
            principal_status: auth.principal_status,
            exists: true,
            enabled: true,
        })
    }

    /// Acquire a fresh client-credentials token for one exact audience/scope
    /// pair from the pinned Auth token endpoint.
    async fn fetch_token(
        &self,
        start: Instant,
        audience: &str,
        scope: &str,
    ) -> Result<String, AdmissionError> {
        let url = format!("{}/oauth/token", self.config.auth_base_url);
        let credentials = format!(
            "{}:{}",
            self.config.client_id,
            self.config.client_secret.expose()
        );
        let authorization = format!("Basic {}", BASE64_STANDARD.encode(credentials.as_bytes()));
        let body = format!("grant_type=client_credentials&resource={audience}&scope={scope}");
        let builder = self
            .http_client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, authorization)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(body);

        let (status, body) = self.execute(start, Endpoint::Token, builder).await?;
        if status != 200 {
            return Err(denied(Endpoint::Token, status, &body));
        }
        let token: TokenResponse =
            serde_json::from_slice(&body).map_err(|error| AdmissionError::MalformedResponse {
                endpoint: Endpoint::Token,
                reason: format!("token response does not match the pinned contract: {error}"),
            })?;
        if token.token_type != "Bearer" {
            return Err(AdmissionError::MalformedResponse {
                endpoint: Endpoint::Token,
                reason: "token_type must be Bearer".to_string(),
            });
        }
        if token.scope != scope {
            return Err(AdmissionError::MalformedResponse {
                endpoint: Endpoint::Token,
                reason: "token scope does not echo the requested scope".to_string(),
            });
        }
        Ok(token.access_token)
    }

    /// Pinned Auth directory route: exact Principal -> exact Agent relation.
    async fn read_auth_directory(
        &self,
        start: Instant,
        principal: Uuid,
        token: &str,
    ) -> Result<AuthDirectoryAgentResponse, AdmissionError> {
        let url = format!(
            "{}/api/v1/directory/principals/{principal}/agent",
            self.config.auth_base_url
        );
        let builder = self.http_client.get(&url).bearer_auth(token);
        let (status, body) = self.execute(start, Endpoint::AuthRead, builder).await?;
        if status != 200 {
            return Err(denied(Endpoint::AuthRead, status, &body));
        }
        let parsed: AuthDirectoryAgentResponse =
            serde_json::from_slice(&body).map_err(|error| AdmissionError::MalformedResponse {
                endpoint: Endpoint::AuthRead,
                reason: format!(
                    "auth directory response does not match the pinned contract: {error}"
                ),
            })?;
        if parsed.principal_id != principal.to_string() {
            return Err(AdmissionError::MalformedResponse {
                endpoint: Endpoint::AuthRead,
                reason: "response principalId does not bind the requested principal".to_string(),
            });
        }
        Ok(parsed)
    }

    /// Pinned Agent-core directory route: exact Agent observation.
    async fn read_core_directory(
        &self,
        start: Instant,
        agent_id: &str,
        token: &str,
    ) -> Result<CoreAgentDirectoryResponse, AdmissionError> {
        let url = format!(
            "{}/v1/directory/agents/{agent_id}",
            self.config.core_base_url
        );
        let builder = self.http_client.get(&url).bearer_auth(token);
        let (status, body) = self.execute(start, Endpoint::CoreRead, builder).await?;
        if status != 200 {
            return Err(denied(Endpoint::CoreRead, status, &body));
        }
        let parsed: CoreAgentDirectoryResponse =
            serde_json::from_slice(&body).map_err(|error| AdmissionError::MalformedResponse {
                endpoint: Endpoint::CoreRead,
                reason: format!(
                    "agent-core directory response does not match the pinned contract: {error}"
                ),
            })?;
        if parsed.agent_id != agent_id {
            return Err(AdmissionError::MalformedResponse {
                endpoint: Endpoint::CoreRead,
                reason: "response agentId does not bind the requested agent".to_string(),
            });
        }
        Ok(parsed)
    }

    /// Send one admission request under the global in-flight bound, with the
    /// per-request timeout equal to the remaining admission budget. Zero
    /// budget left means `Timeout` without touching the network.
    async fn execute(
        &self,
        start: Instant,
        endpoint: Endpoint,
        builder: reqwest::RequestBuilder,
    ) -> Result<(u16, Vec<u8>), AdmissionError> {
        if self.remaining_budget_ms(start) == 0 {
            return Err(AdmissionError::Timeout);
        }
        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        if self.remaining_budget_ms(start) == 0 {
            return Err(AdmissionError::Timeout);
        }
        let budget = Duration::from_millis(self.remaining_budget_ms(start));
        let response = match builder.timeout(budget).send().await {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(endpoint = %endpoint, error = %error, "admission request failed");
                return Err(if error.is_timeout() {
                    AdmissionError::Timeout
                } else {
                    AdmissionError::Unavailable
                });
            }
        };
        let status = response.status().as_u16();
        let body = match response.bytes().await {
            Ok(body) => body,
            Err(error) => {
                tracing::warn!(endpoint = %endpoint, error = %error, "admission response body read failed");
                return Err(if error.is_timeout() {
                    AdmissionError::Timeout
                } else {
                    AdmissionError::Unavailable
                });
            }
        };
        if body.len() > MAX_ADMISSION_BODY_BYTES {
            return Err(AdmissionError::MalformedResponse {
                endpoint,
                reason: "response body exceeds 1 MiB cap".to_string(),
            });
        }
        Ok((status, body.to_vec()))
    }
}

/// Enforce the URL policy: plain HTTP only for exact loopback hosts
/// (127.0.0.1, ::1, localhost); non-loopback requires HTTPS.
fn validate_base_url(field: &str, url: &str) -> Result<String, AdmissionError> {
    let trimmed = url.trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AdmissionError::Config(format!("{field} must not be empty")));
    }
    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|_| AdmissionError::Config(format!("{field} is not a valid URL")))?;
    match parsed.scheme() {
        "https" => {}
        "http" => {
            // IPv6 hosts serialize bracketed ("[::1]"); trim for the match.
            let host = parsed
                .host_str()
                .unwrap_or("")
                .trim_start_matches('[')
                .trim_end_matches(']');
            let loopback = matches!(host, "localhost" | "127.0.0.1" | "::1");
            if !loopback {
                return Err(AdmissionError::Config(format!(
                    "{field} allows plain HTTP only for loopback hosts (127.0.0.1, ::1, localhost); non-loopback endpoints require HTTPS"
                )));
            }
        }
        other => {
            return Err(AdmissionError::Config(format!(
                "{field} must use http (loopback only) or https, got {other}"
            )));
        }
    }
    Ok(trimmed.to_string())
}

/// Build a `Denied` error, extracting the error code from a JSON body
/// `{"error": "..."}` when present, else `http_<status>`.
fn denied(endpoint: Endpoint, status: u16, body: &[u8]) -> AdmissionError {
    let code = serde_json::from_slice::<ErrorBody>(body)
        .ok()
        .map(|parsed| parsed.error)
        .filter(|code| !code.is_empty())
        .unwrap_or_else(|| format!("http_{status}"));
    tracing::warn!(endpoint = %endpoint, status, "admission directory request denied");
    AdmissionError::Denied {
        endpoint,
        status,
        code,
    }
}

fn parse_env<T>(name: &str, default: T) -> Result<T, AdmissionError>
where
    T: std::str::FromStr + ToString,
{
    std::env::var(name)
        .unwrap_or_else(|_| default.to_string())
        .parse::<T>()
        .map_err(|_| AdmissionError::Config(format!("{name} has an invalid value")))
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    #[allow(dead_code)]
    expires_in: u64,
    scope: String,
}

/// Pinned Auth directory response: EXACTLY these three fields.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthDirectoryAgentResponse {
    #[serde(rename = "principalId")]
    principal_id: String,
    #[serde(rename = "agentId")]
    agent_id: String,
    #[serde(rename = "principalStatus")]
    principal_status: String,
}

/// Pinned Agent-core directory response: EXACTLY these three fields.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoreAgentDirectoryResponse {
    #[serde(rename = "agentId")]
    agent_id: String,
    exists: bool,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    error: String,
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::io::Write;
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use super::*;

    // ---- hand-rolled stub HTTP server ----

    #[derive(Clone, Debug)]
    struct StubRequest {
        method: String,
        path: String,
        authorization: Option<String>,
        content_type: Option<String>,
        body: String,
    }

    #[derive(Clone)]
    struct StubResponse {
        delay: Duration,
        status: u16,
        body: String,
    }

    fn json_response(status: u16, body: String) -> StubResponse {
        StubResponse {
            delay: Duration::ZERO,
            status,
            body,
        }
    }

    struct StubServer {
        addr: SocketAddr,
        requests: Arc<Mutex<Vec<StubRequest>>>,
        connections: Arc<AtomicUsize>,
    }

    fn spawn_stub<F>(handler: F) -> StubServer
    where
        F: Fn(&StubRequest) -> StubResponse + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub listener");
        let addr = listener.local_addr().expect("stub addr");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let connections = Arc::new(AtomicUsize::new(0));
        let handler = Arc::new(handler);
        {
            let requests = requests.clone();
            let connections = connections.clone();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { break };
                    connections.fetch_add(1, Ordering::SeqCst);
                    let handler = handler.clone();
                    let requests = requests.clone();
                    std::thread::spawn(move || {
                        serve_connection(stream, handler, requests);
                    });
                }
            });
        }
        StubServer {
            addr,
            requests,
            connections,
        }
    }

    fn serve_connection(
        stream: TcpStream,
        handler: Arc<dyn Fn(&StubRequest) -> StubResponse + Send + Sync>,
        requests: Arc<Mutex<Vec<StubRequest>>>,
    ) {
        let mut stream = stream;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        if let Some(request) = read_request(&mut stream) {
            requests
                .lock()
                .expect("stub requests lock")
                .push(request.clone());
            let response = handler(&request);
            if response.delay > Duration::ZERO {
                std::thread::sleep(response.delay);
            }
            let head = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response.status,
                reason(response.status),
                response.body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(response.body.as_bytes());
            let _ = stream.flush();
        }
        let _ = stream.shutdown(std::net::Shutdown::Both);
    }

    fn read_request(stream: &mut TcpStream) -> Option<StubRequest> {
        let mut buf: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 2048];
        loop {
            if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&buf[..pos]).to_string();
                let mut lines = head.split("\r\n");
                let request_line = lines.next().unwrap_or("");
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or("").to_string();
                let path = parts.next().unwrap_or("").to_string();
                let mut authorization = None;
                let mut content_type = None;
                let mut content_length = 0usize;
                for line in lines {
                    if let Some((name, value)) = line.split_once(':') {
                        let name = name.trim().to_ascii_lowercase();
                        let value = value.trim().to_string();
                        match name.as_str() {
                            "authorization" => authorization = Some(value),
                            "content-type" => content_type = Some(value),
                            "content-length" => content_length = value.parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }
                let body_start = pos + 4;
                let body_end = body_start + content_length;
                while buf.len() < body_end {
                    let n = stream.read(&mut chunk).ok()?;
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                let end = body_end.min(buf.len());
                let body = String::from_utf8_lossy(&buf[body_start..end]).to_string();
                return Some(StubRequest {
                    method,
                    path,
                    authorization,
                    content_type,
                    body,
                });
            }
            let n = stream.read(&mut chunk).ok()?;
            if n == 0 {
                return None;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
    }

    fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    fn reason(status: u16) -> &'static str {
        match status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            409 => "Conflict",
            422 => "Unprocessable Entity",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            _ => "Status",
        }
    }

    // ---- canned bodies ----

    fn token_body(resource: &str, scope: &str) -> String {
        format!(
            r#"{{"access_token":"tok-{resource}","token_type":"Bearer","expires_in":3600,"scope":"{scope}"}}"#
        )
    }

    fn auth_body(principal: &Uuid, agent_id: &str, principal_status: &str) -> String {
        format!(
            r#"{{"principalId":"{principal}","agentId":"{agent_id}","principalStatus":"{principal_status}"}}"#
        )
    }

    fn core_body(agent_id: &str, exists: bool, enabled: bool) -> String {
        format!(r#"{{"agentId":"{agent_id}","exists":{exists},"enabled":{enabled}}}"#)
    }

    fn form_param(body: &str, key: &str) -> String {
        body.split('&')
            .find_map(|pair| {
                pair.split_once('=')
                    .filter(|(name, _)| *name == key)
                    .map(|(_, value)| value.to_string())
            })
            .unwrap_or_default()
    }

    fn stub_config(auth: &StubServer, core: &StubServer) -> AdmissionConfig {
        AdmissionConfig {
            enabled: true,
            auth_base_url: format!("http://{}", auth.addr),
            core_base_url: format!("http://{}", core.addr),
            client_id: "svc-workflow".to_string(),
            client_secret: SecretString::new("test-secret"),
            deadline_ms: ADMISSION_DEADLINE_MS,
            max_in_flight: DEFAULT_MAX_IN_FLIGHT,
        }
    }

    /// Shared handler script: token endpoint echoing the requested
    /// audience/scope, Auth read answering per-principal (with an optional
    /// override status or raw body), core read answering exists/enabled
    /// (with overrides).
    struct DirectoryScript {
        auth_status_override: Option<String>,
        auth_raw_body: Option<String>,
        auth_http_status: Option<u16>,
        core_raw_body: Option<String>,
        core_exists: bool,
        core_enabled: bool,
        core_agent_id_override: Option<String>,
        core_http_status: Option<u16>,
        token_delay: Duration,
    }

    impl DirectoryScript {
        fn happy() -> Self {
            Self {
                auth_status_override: None,
                auth_raw_body: None,
                auth_http_status: None,
                core_raw_body: None,
                core_exists: true,
                core_enabled: true,
                core_agent_id_override: None,
                core_http_status: None,
                token_delay: Duration::ZERO,
            }
        }

        fn serve(self) -> (StubServer, StubServer) {
            let DirectoryScript {
                auth_status_override,
                auth_raw_body,
                auth_http_status,
                core_raw_body,
                core_exists,
                core_enabled,
                core_agent_id_override,
                core_http_status,
                token_delay,
            } = self;
            let auth_stub = spawn_stub(move |request| {
                if request.path == "/oauth/token" {
                    let resource = form_param(&request.body, "resource");
                    let scope = form_param(&request.body, "scope");
                    return StubResponse {
                        delay: token_delay,
                        ..json_response(200, token_body(&resource, &scope))
                    };
                }
                if let Some(rest) = request.path.strip_prefix("/api/v1/directory/principals/") {
                    let uuid_part = rest.trim_end_matches("/agent");
                    if let Ok(principal) = Uuid::parse_str(uuid_part) {
                        if let Some(status) = auth_http_status {
                            if let Some(raw) = &auth_raw_body {
                                return json_response(status, raw.clone());
                            }
                            return json_response(
                                status,
                                format!(r#"{{"error":"synthetic_{status}"}}"#),
                            );
                        }
                        if let Some(raw) = &auth_raw_body {
                            return json_response(200, raw.clone());
                        }
                        let status = auth_status_override.as_deref().unwrap_or("active");
                        let agent_id = format!("agent-{principal}");
                        return json_response(200, auth_body(&principal, &agent_id, status));
                    }
                }
                json_response(404, r#"{"error":"unexpected_path"}"#.to_string())
            });
            let core_stub = spawn_stub(move |request| {
                if let Some(agent_id) = request.path.strip_prefix("/v1/directory/agents/") {
                    if let Some(status) = core_http_status {
                        if let Some(raw) = &core_raw_body {
                            return json_response(status, raw.clone());
                        }
                        return json_response(
                            status,
                            format!(r#"{{"error":"synthetic_{status}"}}"#),
                        );
                    }
                    if let Some(raw) = &core_raw_body {
                        return json_response(200, raw.clone());
                    }
                    let response_id = core_agent_id_override.as_deref().unwrap_or(agent_id);
                    return json_response(200, core_body(response_id, core_exists, core_enabled));
                }
                json_response(404, r#"{"error":"unexpected_path"}"#.to_string())
            });
            (auth_stub, core_stub)
        }
    }

    // ---- tests ----

    #[tokio::test]
    async fn happy_path_two_principals_fetches_token_once_per_audience() {
        let principal_a = Uuid::new_v4();
        let principal_b = Uuid::new_v4();
        let script = DirectoryScript::happy();
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");

        let principals: BTreeSet<Uuid> = [principal_a, principal_b].into_iter().collect();
        let start = Instant::now();
        let report = client
            .admit(start, &principals)
            .await
            .expect("admission ok");

        assert_eq!(report.observations.len(), 2);
        for principal in [principal_a, principal_b] {
            let observed = &report.observations[&principal];
            assert_eq!(observed.agent_id, format!("agent-{principal}"));
            assert_eq!(observed.principal_status, "active");
            assert!(observed.exists);
            assert!(observed.enabled);
        }

        let requests = auth_stub.requests.lock().expect("requests lock");
        // The Auth stub serves both tokens plus both auth reads; the core
        // stub serves the two core reads. Six requests total, no retries.
        assert_eq!(requests.len(), 4);
        assert_eq!(core_stub.requests.lock().expect("requests lock").len(), 2);

        let token_requests: Vec<&StubRequest> = requests
            .iter()
            .filter(|request| request.path == "/oauth/token")
            .collect();
        assert_eq!(token_requests.len(), 2, "one token per audience");
        for request in &token_requests {
            assert_eq!(request.method, "POST");
            assert_eq!(
                request.content_type.as_deref(),
                Some("application/x-www-form-urlencoded")
            );
            let authorization = request.authorization.as_deref().expect("Basic header");
            let encoded = authorization.strip_prefix("Basic ").expect("Basic scheme");
            let decoded = BASE64_STANDARD
                .decode(encoded.as_bytes())
                .expect("base64 decodes");
            let decoded = String::from_utf8(decoded).expect("utf8 credentials");
            assert_eq!(decoded, "svc-workflow:test-secret");
            assert_eq!(
                form_param(&request.body, "grant_type"),
                "client_credentials"
            );
        }
        let resources: BTreeMap<String, String> = token_requests
            .iter()
            .map(|request| {
                (
                    form_param(&request.body, "resource"),
                    form_param(&request.body, "scope"),
                )
            })
            .collect();
        assert_eq!(
            resources.get("identity-directory").map(String::as_str),
            Some("auth.directory.read")
        );
        assert_eq!(
            resources.get("agent-directory").map(String::as_str),
            Some("agent.directory.read")
        );
        const IDENTITY_TOKEN_BODY: &str =
            "grant_type=client_credentials&resource=identity-directory&scope=auth.directory.read";
        const AGENT_TOKEN_BODY: &str =
            "grant_type=client_credentials&resource=agent-directory&scope=agent.directory.read";
        assert!(token_requests
            .iter()
            .any(|request| request.body == IDENTITY_TOKEN_BODY));
        assert!(token_requests
            .iter()
            .any(|request| request.body == AGENT_TOKEN_BODY));

        let auth_reads: Vec<&StubRequest> = requests
            .iter()
            .filter(|request| request.path.starts_with("/api/v1/directory/principals/"))
            .collect();
        assert_eq!(auth_reads.len(), 2);
        for request in &auth_reads {
            assert_eq!(request.method, "GET");
            assert_eq!(
                request.authorization.as_deref(),
                Some("Bearer tok-identity-directory")
            );
            let uuid_part = request
                .path
                .strip_prefix("/api/v1/directory/principals/")
                .unwrap()
                .trim_end_matches("/agent");
            assert!(principals.contains(&Uuid::parse_str(uuid_part).expect("uuid path")));
        }

        let core_requests = core_stub.requests.lock().expect("requests lock");
        let core_reads: Vec<&StubRequest> = core_requests
            .iter()
            .filter(|request| request.path.starts_with("/v1/directory/agents/"))
            .collect();
        assert_eq!(core_reads.len(), 2);
        for request in &core_reads {
            assert_eq!(request.method, "GET");
            assert_eq!(
                request.authorization.as_deref(),
                Some("Bearer tok-agent-directory")
            );
            let agent_id = request.path.strip_prefix("/v1/directory/agents/").unwrap();
            let principal = Uuid::parse_str(agent_id.trim_start_matches("agent-")).expect("uuid");
            assert!(principals.contains(&principal));
        }
    }

    #[tokio::test]
    async fn auth_principal_id_mismatch_is_malformed_response() {
        let principal = Uuid::new_v4();
        let other = Uuid::new_v4();
        let script = DirectoryScript {
            auth_raw_body: Some(auth_body(&other, "agent-x", "active")),
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");

        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let result = client.admit(Instant::now(), &principals).await;
        match result {
            Err(AdmissionError::MalformedResponse {
                endpoint: Endpoint::AuthRead,
                ..
            }) => {}
            other => panic!("expected MalformedResponse(AuthRead), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn auth_unknown_extra_field_is_malformed_response() {
        let principal = Uuid::new_v4();
        let raw = format!(
            r#"{{"principalId":"{principal}","agentId":"agent-x","principalStatus":"active","displayName":"nope"}}"#
        );
        let script = DirectoryScript {
            auth_raw_body: Some(raw),
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");

        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let result = client.admit(Instant::now(), &principals).await;
        match result {
            Err(AdmissionError::MalformedResponse {
                endpoint: Endpoint::AuthRead,
                reason,
            }) => assert!(reason.contains("unknown field"), "reason: {reason}"),
            other => panic!("expected MalformedResponse(AuthRead), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn principal_not_active_is_rejected() {
        let principal = Uuid::new_v4();
        let script = DirectoryScript {
            auth_status_override: Some("disabled".to_string()),
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");

        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let result = client.admit(Instant::now(), &principals).await;
        match result {
            Err(AdmissionError::Rejected {
                principal: rejected,
                reason: RejectionReason::PrincipalNotActive(status),
            }) => {
                assert_eq!(rejected, principal);
                assert_eq!(status, "disabled");
            }
            other => panic!("expected Rejected(PrincipalNotActive), got {other:?}"),
        }
        // The disabled principal must never reach the agent-core read.
        assert!(
            !core_stub
                .requests
                .lock()
                .expect("requests lock")
                .iter()
                .any(|request| request.path.starts_with("/v1/directory/agents/")),
            "core read must not run after a non-active principal"
        );
    }

    #[tokio::test]
    async fn core_rejections_and_mismatch() {
        // exists == false -> AgentMissing
        let principal = Uuid::new_v4();
        let script = DirectoryScript {
            core_exists: false,
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");
        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let result = client.admit(Instant::now(), &principals).await;
        match result {
            Err(AdmissionError::Rejected {
                principal: rejected,
                reason: RejectionReason::AgentMissing,
            }) => assert_eq!(rejected, principal),
            other => panic!("expected Rejected(AgentMissing), got {other:?}"),
        }

        // enabled == false -> AgentDisabled
        let principal = Uuid::new_v4();
        let script = DirectoryScript {
            core_enabled: false,
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");
        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let result = client.admit(Instant::now(), &principals).await;
        match result {
            Err(AdmissionError::Rejected {
                principal: rejected,
                reason: RejectionReason::AgentDisabled,
            }) => assert_eq!(rejected, principal),
            other => panic!("expected Rejected(AgentDisabled), got {other:?}"),
        }

        // agentId mismatch -> MalformedResponse(CoreRead)
        let principal = Uuid::new_v4();
        let script = DirectoryScript {
            core_agent_id_override: Some("agent-someone-else".to_string()),
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");
        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let result = client.admit(Instant::now(), &principals).await;
        match result {
            Err(AdmissionError::MalformedResponse {
                endpoint: Endpoint::CoreRead,
                ..
            }) => {}
            other => panic!("expected MalformedResponse(CoreRead), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn denied_statuses_carry_extracted_codes() {
        // Auth read 404 with a JSON error body.
        let principal = Uuid::new_v4();
        let script = DirectoryScript {
            auth_http_status: Some(404),
            auth_raw_body: Some(r#"{"error":"PRINCIPAL_NOT_FOUND"}"#.to_string()),
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");
        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let result = client.admit(Instant::now(), &principals).await;
        match result {
            Err(AdmissionError::Denied {
                endpoint: Endpoint::AuthRead,
                status,
                code,
            }) => {
                assert_eq!(status, 404);
                assert_eq!(code, "PRINCIPAL_NOT_FOUND");
            }
            other => panic!("expected Denied(AuthRead), got {other:?}"),
        }

        // Auth read 401 with a JSON error body.
        let principal = Uuid::new_v4();
        let script = DirectoryScript {
            auth_http_status: Some(401),
            auth_raw_body: Some(r#"{"error":"INVALID_TOKEN"}"#.to_string()),
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");
        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let result = client.admit(Instant::now(), &principals).await;
        match result {
            Err(AdmissionError::Denied {
                endpoint: Endpoint::AuthRead,
                status,
                code,
            }) => {
                assert_eq!(status, 401);
                assert_eq!(code, "INVALID_TOKEN");
            }
            other => panic!("expected Denied(AuthRead), got {other:?}"),
        }

        // Core read 503 with a JSON error body.
        let principal = Uuid::new_v4();
        let script = DirectoryScript {
            core_http_status: Some(503),
            core_raw_body: Some(r#"{"error":"AGENT_DEFINITION_QUERY_FAILED"}"#.to_string()),
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");
        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let result = client.admit(Instant::now(), &principals).await;
        match result {
            Err(AdmissionError::Denied {
                endpoint: Endpoint::CoreRead,
                status,
                code,
            }) => {
                assert_eq!(status, 503);
                assert_eq!(code, "AGENT_DEFINITION_QUERY_FAILED");
            }
            other => panic!("expected Denied(CoreRead), got {other:?}"),
        }

        // Non-JSON error body falls back to http_<status>.
        let principal = Uuid::new_v4();
        let script = DirectoryScript {
            auth_http_status: Some(500),
            auth_raw_body: Some("gateway exploded".to_string()),
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");
        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let result = client.admit(Instant::now(), &principals).await;
        match result {
            Err(AdmissionError::Denied {
                endpoint: Endpoint::AuthRead,
                status,
                code,
            }) => {
                assert_eq!(status, 500);
                assert_eq!(code, "http_500");
            }
            other => panic!("expected Denied(AuthRead), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn past_deadline_times_out_with_zero_network() {
        let script = DirectoryScript::happy();
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");

        let principal = Uuid::new_v4();
        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        let start = Instant::now()
            .checked_sub(Duration::from_millis(ADMISSION_DEADLINE_MS + 1_000))
            .expect("past instant");
        let result = client.admit(start, &principals).await;
        assert!(matches!(result, Err(AdmissionError::Timeout)));
        assert_eq!(auth_stub.connections.load(Ordering::SeqCst), 0);
        assert_eq!(core_stub.connections.load(Ordering::SeqCst), 0);
        assert!(auth_stub.requests.lock().expect("requests lock").is_empty());
        assert!(core_stub.requests.lock().expect("requests lock").is_empty());
    }

    #[tokio::test]
    async fn slow_endpoint_times_out_against_remaining_budget() {
        let principal = Uuid::new_v4();
        let script = DirectoryScript {
            token_delay: Duration::from_millis(500),
            ..DirectoryScript::happy()
        };
        let (auth_stub, core_stub) = script.serve();
        let client =
            AdmissionClient::new(stub_config(&auth_stub, &core_stub)).expect("client builds");

        let principals: BTreeSet<Uuid> = [principal].into_iter().collect();
        // Leave ~100ms of budget; the stub answers after 500ms.
        let start = Instant::now()
            .checked_sub(Duration::from_millis(ADMISSION_DEADLINE_MS - 100))
            .expect("near-deadline instant");
        let result = client.admit(start, &principals).await;
        assert!(matches!(result, Err(AdmissionError::Timeout)));
    }

    #[test]
    fn non_loopback_http_is_config_error() {
        let script = DirectoryScript::happy();
        let (auth_stub, core_stub) = script.serve();

        let mut config = stub_config(&auth_stub, &core_stub);
        config.auth_base_url = "http://directory.example.internal".to_string();
        assert!(matches!(
            AdmissionClient::new(config),
            Err(AdmissionError::Config(_))
        ));

        let mut config = stub_config(&auth_stub, &core_stub);
        config.core_base_url = "http://core.example.internal".to_string();
        assert!(matches!(
            AdmissionClient::new(config),
            Err(AdmissionError::Config(_))
        ));

        // Loopback plain HTTP and non-loopback HTTPS are accepted.
        for (auth_url, core_url) in [
            ("http://127.0.0.1:1", "http://localhost:1"),
            ("http://[::1]:1", "http://127.0.0.1:1"),
            (
                "https://directory.example.internal",
                "https://core.example.internal",
            ),
        ] {
            let mut config = stub_config(&auth_stub, &core_stub);
            config.auth_base_url = auth_url.to_string();
            config.core_base_url = core_url.to_string();
            assert!(
                AdmissionClient::new(config).is_ok(),
                "expected {auth_url} / {core_url} to be accepted"
            );
        }
    }

    #[test]
    fn config_debug_redacts_client_secret() {
        let script = DirectoryScript::happy();
        let (auth_stub, core_stub) = script.serve();
        let config = stub_config(&auth_stub, &core_stub);
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("test-secret"));
        assert!(rendered.contains("SecretString(***)"));
    }

    // ---- from_env tests (serialized; they mutate process env) ----

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_admission_env() {
        std::env::remove_var(ENV_ENABLED);
        std::env::remove_var(ENV_AUTH_BASE_URL);
        std::env::remove_var(ENV_CORE_BASE_URL);
        std::env::remove_var(ENV_CLIENT_ID);
        std::env::remove_var(ENV_CLIENT_SECRET);
        std::env::remove_var(ENV_DEADLINE_MS);
        std::env::remove_var(ENV_MAX_IN_FLIGHT);
    }

    #[test]
    fn from_env_disabled_by_default_and_validates_enabled_mode() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // Default: disabled, defaults populated, construction never fails.
        clear_admission_env();
        let config = AdmissionConfig::from_env().expect("disabled mode never fails");
        assert!(!config.enabled);
        assert_eq!(config.client_id, "svc-workflow");
        assert_eq!(config.deadline_ms, ADMISSION_DEADLINE_MS);
        assert_eq!(config.max_in_flight, DEFAULT_MAX_IN_FLIGHT);

        // Enabled without the required secret -> Config error.
        std::env::set_var(ENV_ENABLED, "1");
        std::env::set_var(ENV_AUTH_BASE_URL, "http://127.0.0.1:4001");
        std::env::set_var(ENV_CORE_BASE_URL, "http://127.0.0.1:8080");
        assert!(matches!(
            AdmissionConfig::from_env(),
            Err(AdmissionError::Config(_))
        ));

        // Enabled with everything set -> enabled config.
        std::env::set_var(ENV_CLIENT_SECRET, "s3cret");
        let config = AdmissionConfig::from_env().expect("enabled mode parses");
        assert!(config.enabled);
        assert_eq!(config.auth_base_url, "http://127.0.0.1:4001");
        assert_eq!(config.core_base_url, "http://127.0.0.1:8080");

        // Deadline beyond the 5-second bound -> Config error.
        std::env::set_var(ENV_DEADLINE_MS, "5001");
        assert!(matches!(
            AdmissionConfig::from_env(),
            Err(AdmissionError::Config(_))
        ));
        std::env::remove_var(ENV_DEADLINE_MS);

        // In-flight bound violations -> Config error.
        std::env::set_var(ENV_MAX_IN_FLIGHT, "9");
        assert!(matches!(
            AdmissionConfig::from_env(),
            Err(AdmissionError::Config(_))
        ));
        std::env::set_var(ENV_MAX_IN_FLIGHT, "0");
        assert!(matches!(
            AdmissionConfig::from_env(),
            Err(AdmissionError::Config(_))
        ));
        std::env::remove_var(ENV_MAX_IN_FLIGHT);

        // Enabled without base URLs -> Config error.
        std::env::remove_var(ENV_AUTH_BASE_URL);
        assert!(matches!(
            AdmissionConfig::from_env(),
            Err(AdmissionError::Config(_))
        ));

        clear_admission_env();
    }
}
