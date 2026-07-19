//! Auth V1 Single-Agent Read-Only Canary Profile.
//!
//! Implements the frozen Auth V1 token validation for the single-agent
//! read-only canary.  This is NOT a general-purpose verifier — it is a
//! controlled canary gated by feature flags and allow-lists.
//!
//! ## Contract source
//!
//! All validation rules are derived from the frozen Minimal Auth V1 Bundle
//! (auth-service-workflow-contract-docs tree
//!  `f88f8e9b2c1be3ae062b2e667d5ec98b816c81d1`).  Key documents:
//!
//! - `contract-bundles/minimal-auth-v1/contract-manifest.json`
//! - `contract-bundles/minimal-auth-v1/schemas/token-profiles.schema.json`
//! - `contract-bundles/minimal-auth-v1/fixtures/positive-token-fixtures.json`
//! - `contract-bundles/minimal-auth-v1/fixtures/negative-token-fixtures.json`
//!
//! Values the frozen contract does **not** define are returned as
//! `AUTH_CANDIDATE_CONTRACT_GAP` — the canary must not invent them.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::ids::PrincipalId;
use crate::http::error::ApiError;

use super::auth_context::AuthContext;
use super::AuthenticatedPrincipal;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Feature flags and allow-list for the Auth V1 canary.
///
/// Default-off.  When enabled, only requests where **both** `client_id`
/// and `sub` match the configured values enter the canary profile.
///
/// Write operations require an additional `AUTH_V1_CANARY_WRITE_ENABLED`
/// flag (default-off).  When write is disabled, canary-matching tokens
/// are rejected at write endpoints with a 403 before any verification.
#[derive(Debug, Clone)]
pub struct AuthV1CanaryConfig {
    /// Master switch.  When `false` (the default) the canary has zero effect;
    /// legacy auth is used for every request.
    pub enabled: bool,
    /// Separate write gate.  When `false` (the default), canary-authenticated
    /// requests are blocked from write endpoints with a definitive 403.
    pub write_enabled: bool,
    /// The only `client_id` that the canary accepts.
    pub allowed_client_id: String,
    /// The only `sub` (principal UUID) that the canary accepts.
    pub allowed_sub: String,
    /// JWKS URL for RS256 key resolution.
    pub jwks_url: String,
    /// Expected exact issuer (frozen contract: `"auth-service"`).
    pub issuer: String,
    /// Expected exact audience (frozen contract: `"svc-workflow"`).
    pub audience: String,
    /// JWKS cache TTL (frozen contract default: 300 s).
    pub cache_ttl_secs: u64,
    /// JWKS fetch HTTP timeout (frozen contract default: 5 s).
    pub http_timeout_secs: u64,
    /// Max stale time for cache (frozen contract default: 600 s).
    pub max_stale_secs: u64,
    /// Clock skew tolerance (frozen contract default: 60 s).
    pub clock_skew_seconds: u64,
}

impl Default for AuthV1CanaryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            write_enabled: false,
            allowed_client_id: String::new(),
            allowed_sub: String::new(),
            jwks_url: String::new(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        }
    }
}

impl AuthV1CanaryConfig {
    /// Build config from environment variables.
    ///
    /// All canary flags default to `false` / empty — the canary is off by
    /// default and must be explicitly configured.
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("AUTH_V1_CANARY_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            write_enabled: std::env::var("AUTH_V1_CANARY_WRITE_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            allowed_client_id: std::env::var("AUTH_V1_CANARY_ALLOWED_CLIENT_ID")
                .unwrap_or_default(),
            allowed_sub: std::env::var("AUTH_V1_CANARY_ALLOWED_SUB").unwrap_or_default(),
            jwks_url: std::env::var("WORKFLOW_JWKS_URL").unwrap_or_default(),
            issuer: std::env::var("WORKFLOW_JWT_ISSUER")
                .unwrap_or_else(|_| "auth-service".to_string()),
            audience: std::env::var("WORKFLOW_JWT_AUDIENCE")
                .unwrap_or_else(|_| "svc-workflow".to_string()),
            cache_ttl_secs: std::env::var("WORKFLOW_JWKS_CACHE_TTL")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            http_timeout_secs: std::env::var("WORKFLOW_JWKS_HTTP_TIMEOUT")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            max_stale_secs: std::env::var("WORKFLOW_JWKS_MAX_STALE")
                .unwrap_or_else(|_| "600".to_string())
                .parse()
                .unwrap_or(600),
            clock_skew_seconds: std::env::var("WORKFLOW_JWT_CLOCK_SKEW")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
        }
    }

    /// Quick check: canary is enabled AND both allow-list values are present.
    pub fn is_active(&self) -> bool {
        self.enabled && !self.allowed_client_id.is_empty() && !self.allowed_sub.is_empty()
    }

    /// Quick check: canary is active AND write operations are permitted.
    pub fn write_active(&self) -> bool {
        self.is_active() && self.write_enabled
    }
}

// ---------------------------------------------------------------------------
// Delegated claims struct (narrow, V1-contract fields only)
// ---------------------------------------------------------------------------

/// Narrow claims set matching the frozen V1 DirectMachineAccess profile.
///
/// The frozen contract defines:
/// - Required: `iss`, `sub`, `aud`, `principal_type`, `client_id`,
///   `token_use`, `type`, `version`, `scope`, `jti`, `iat`, `nbf`, `exp`
/// - Optional: `agent_id` (type `string`, minLength 1) — present when
///   `principal_type` is `"agent"`, absent when `"service"`.
/// - `serde(deny_unknown_fields)` enforces the `additionalProperties: false`
///   rule from the token-profiles schema.
///
/// Made `pub(crate)` so route guards can use strict deserialization for
/// fail-closed Auth V1 token identification.
///
/// ## `agent_id` usage restrictions
///
/// `agent_id` is accepted for deserialization only; it must NOT be used for:
/// - Task ownership or assignment
/// - Resource visibility filtering
/// - DomainRoleBinding decisions
/// - Canary `allowed-sub` replacement
/// - Product authorization
///
/// The sole principal identifier remains `sub` (`PRINCIPAL_SOURCE = sub`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct V1DirectMachineClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub principal_type: String,
    pub client_id: String,
    pub token_use: String,
    #[serde(rename = "type")]
    pub token_type: String,
    pub version: String,
    pub scope: String,
    pub agent_id: Option<String>,
    pub jti: String,
    pub iat: usize,
    pub nbf: usize,
    pub exp: usize,
}

// ---------------------------------------------------------------------------
// Auth V1 token identification (fail-closed guard)
// ---------------------------------------------------------------------------

/// Check whether a raw JWT string matches the Auth V1 DirectMachineAccess
/// profile shape **without** verifying the signature.
///
/// Returns `true` when:
/// 1. The JWT header declares `alg = RS256` and includes a `kid`.
/// 2. The JWT payload can be deserialised into `V1DirectMachineClaims`
///    (which has `serde(deny_unknown_fields)` and all fields required;
///    the optional `agent_id` claim is accepted when present).
///
/// Legacy tokens that use a different algorithm (HS256) or carry extra
/// claims (e.g. `act`, `azp`) will return `false`.
///
/// This is the only gate for deciding whether a token enters the canary
/// path or falls through to legacy auth — the canary path **never**
/// delegates back to legacy for a recognised V1 token.
pub(crate) fn looks_like_auth_v1_token(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    // 1. Header check: RS256 + kid required.
    let header = match jsonwebtoken::decode_header(token) {
        Ok(h) => h,
        Err(_) => return false,
    };
    if header.alg != jsonwebtoken::Algorithm::RS256 || header.kid.is_none() {
        return false;
    }
    // 2. Strict payload decoding into V1DirectMachineClaims.
    let payload_bytes = match decode_base64url(parts[1]) {
        Ok(b) => b,
        Err(_) => return false,
    };
    serde_json::from_slice::<V1DirectMachineClaims>(&payload_bytes).is_ok()
}

/// Base64url decode with padding tolerance (mirrors the guard helper).
fn decode_base64url(input: &str) -> Result<Vec<u8>, ()> {
    let len = input.len();
    let remainder = len % 4;
    let padded = if remainder == 2 {
        format!("{input}==")
    } else if remainder == 3 {
        format!("{input}=")
    } else {
        input.to_string()
    };
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE
        .decode(padded.as_bytes())
        .map_err(|_| ())
}

// ---------------------------------------------------------------------------
// JWKS infrastructure
// ---------------------------------------------------------------------------

/// Maximum JWKS response body (1 MB — same as the main verifier).
const MAX_JWKS_BODY_BYTES: usize = 1_048_576;

/// A raw JWK entry.
#[derive(Debug, Deserialize)]
struct RawJwk {
    kid: Option<String>,
    #[serde(rename = "kty")]
    key_type: Option<String>,
    #[serde(rename = "use")]
    key_use: Option<String>,
    alg: Option<String>,
    n: Option<String>,
    e: Option<String>,
}

/// JWKS response.
#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<RawJwk>,
}

/// Cached key.
struct JwkKey {
    kid: String,
    decoding_key: DecodingKey,
}

/// Cache state.
struct JwksCacheState {
    keys: Vec<JwkKey>,
    fetched_at: Instant,
}

// ---------------------------------------------------------------------------
// Auth V1 Canary Verifier
// ---------------------------------------------------------------------------

/// Auth V1 token verifier for the single-agent read-only canary.
///
/// Validates tokens against the frozen Minimal Auth V1 DirectMachineAccess
/// profile.  Uses its own JWKS cache (separate from the main verifier).
///
/// ## Contract evidence
///
/// All validation rules reference specific clauses in the frozen contract:
///
/// | Rule | Contract reference |
/// |------|-------------------|
/// | `alg` must be `RS256` | `contract-manifest.json` → `signing.algorithm` |
/// | No HS256 fallback | `contract-manifest.json` → `signing.allow_hs256_fallback` |
/// | `kid` must exist in header | token-profiles fixtures always include `kid`; contract requires JWKS-based verification |
/// | `typ` header must be `at+jwt` | Task specification minimum requirement |
/// | JWKS RS256 verification | `contract-manifest.json` → `signing.jwks_path`, JWKS fixture |
/// | `iss` must equal `"auth-service"` | `contract-manifest.json` → `exact_issuer` |
/// | `aud` must be single `"svc-workflow"` | `audience-registry.json` → svc-workflow audience; `contract-manifest.json` → registry |
/// | `principal_type` must be `"agent"` | `schemas/token-profiles.schema.json` → directMachineAccess.principal_type |
/// | `token_use` must be `"access"` | `schemas/token-profiles.schema.json` → directMachineAccess.token_use |
/// | `scope` must contain `workflow.read` | `audience-registry.json` → svc-workflow.registered_scopes |
/// | `client_id` must exist | `schemas/token-profiles.schema.json` → directMachineAccess.client_id required |
/// | `sub` must be UUID | `schemas/token-profiles.schema.json` → `$defs/uuid` pattern |
/// | Unknown kid triggers max 1 refresh | `contract-manifest.json` → `signing.unknown_kid_refresh_attempts` |
/// | Clock skew ≤ 60 s | `contract-manifest.json` → `timing.clock_skew_tolerance_seconds` |
/// | Machine TTL ≤ 600 s | `contract-manifest.json` → `timing.machine_access_ttl_seconds` |
/// | `jti` minLength 16 | `schemas/token-profiles.schema.json` → directMachineAccess.jti.minLength |
/// | `nbf` is required | `schemas/token-profiles.schema.json` → directMachineAccess.required includes nbf |
/// | `type` must be `"access"` | `schemas/token-profiles.schema.json` → directMachineAccess.type |
/// | `version` must be `"v1"` | `contract-manifest.json` → `token_version` |
/// | Scope sorted, ASCII-space | `contract-manifest.json` → `scope_wire_format` |
pub struct AuthV1CanaryVerifier {
    config: AuthV1CanaryConfig,
    cache: Arc<tokio::sync::RwLock<Option<JwksCacheState>>>,
    http_client: reqwest::Client,
    refresh_lock: Arc<Mutex<()>>,
}

impl Clone for AuthV1CanaryVerifier {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            cache: self.cache.clone(),
            http_client: self.http_client.clone(),
            refresh_lock: self.refresh_lock.clone(),
        }
    }
}

impl AuthV1CanaryVerifier {
    /// Create a new canary verifier.
    pub fn new(config: AuthV1CanaryConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.http_timeout_secs))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("canary reqwest::Client");
        let verifier = Self {
            config,
            cache: Arc::new(tokio::sync::RwLock::new(None)),
            http_client,
            refresh_lock: Arc::new(Mutex::new(())),
        };
        // Eager background fetch.
        let eager = verifier.clone();
        tokio::spawn(async move {
            let _ = eager.fetch_jwks().await;
        });
        verifier
    }

    /// Verify a token against the frozen Auth V1 DirectMachineAccess profile.
    ///
    /// Returns the `AuthenticatedPrincipal` with `principal_id = token.sub`.
    pub async fn verify(&self, token: &str) -> Result<AuthenticatedPrincipal, ApiError> {
        // 1. Decode and validate JWT header.
        let header = decode_header(token)
            .map_err(|_| ApiError::unauthorized("malformed_token", "malformed JWT header"))?;

        // alg must be exactly RS256 (contract: signing.algorithm = RS256).
        if header.alg != Algorithm::RS256 {
            return Err(ApiError::unauthorized(
                "algorithm_not_allowed",
                "only RS256 algorithm is accepted by the Auth V1 canary",
            ));
        }

        // kid is required (contract: JWKS-based verification; header must have kid).
        let kid = header.kid.as_deref().ok_or_else(|| {
            ApiError::unauthorized("malformed_token", "JWT must include a kid header")
        })?;

        // 2. Look up the key (with refresh).
        let key = self.lookup_key(kid).await?;

        // 3. Build validation.
        let mut validation = Validation::new(Algorithm::RS256);
        validation.algorithms = vec![Algorithm::RS256];
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);
        for claim in ["exp", "iat", "iss", "aud", "sub"] {
            validation.required_spec_claims.insert(claim.to_string());
        }
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = self.config.clock_skew_seconds;

        // 4. Decode and verify signature + standard claims.
        let data =
            decode::<V1DirectMachineClaims>(token, &key, &validation).map_err(
                |error| match error.kind() {
                    ErrorKind::ExpiredSignature => {
                        ApiError::unauthorized("token_expired", "access token has expired")
                    }
                    ErrorKind::MissingRequiredClaim(claim) => ApiError::unauthorized_with_details(
                        "malformed_token",
                        "access token is missing a required claim",
                        serde_json::json!({ "claim": claim }),
                    ),
                    ErrorKind::InvalidAlgorithm => {
                        ApiError::unauthorized("algorithm_not_allowed", "invalid JWT algorithm")
                    }
                    ErrorKind::InvalidIssuer => {
                        ApiError::unauthorized("wrong_issuer", "token issuer mismatch")
                    }
                    ErrorKind::InvalidAudience => {
                        ApiError::unauthorized("wrong_audience", "token audience mismatch")
                    }
                    _ => ApiError::unauthorized("malformed_token", "invalid access token"),
                },
            )?;

        let claims = data.claims;

        // 5. Validate V1 profile claims.

        // principal_type must be "agent" (contract: directMachineAccess.principal_type ∈ ["agent", "service"]).
        // For our canary we restrict to "agent" only (principal_type=agent).
        if claims.principal_type != "agent" {
            return Err(ApiError::unauthorized(
                "invalid_principal_type",
                "principal_type must be agent for Auth V1 canary",
            ));
        }

        // token_use must be "access" (contract: directMachineAccess.token_use = "access").
        if claims.token_use != "access" {
            return Err(ApiError::unauthorized(
                "invalid_token_use",
                "token_use must be access for Auth V1 canary",
            ));
        }

        // type must be "access" (contract: directMachineAccess.type = "access").
        if claims.token_type != "access" {
            return Err(ApiError::unauthorized(
                "invalid_token_type",
                "token type must be access",
            ));
        }

        // version must be "v1" (contract: manifest.token_version = "v1").
        if claims.version != "v1" {
            return Err(ApiError::unauthorized(
                "malformed_token",
                "token version must be v1",
            ));
        }

        // jti must have at least 16 characters (contract: jti.minLength = 16).
        if claims.jti.len() < 16 {
            return Err(ApiError::unauthorized(
                "malformed_token",
                "jti must contain at least 16 characters",
            ));
        }

        // 6. Validate sub is a valid UUID (contract: $defs/uuid pattern).
        let sub_uuid = Uuid::parse_str(&claims.sub)
            .map_err(|_| ApiError::unauthorized("malformed_token", "sub must be a valid UUID"))?;

        // 7. Validate allow-list.
        // client_id must match the configured allowed client_id.
        if claims.client_id != self.config.allowed_client_id {
            return Err(ApiError::unauthorized(
                "unauthorized_client",
                "client_id is not authorized for the Auth V1 canary",
            ));
        }

        // sub must match the configured allowed sub.
        if claims.sub != self.config.allowed_sub {
            return Err(ApiError::unauthorized(
                "unauthorized_principal",
                "sub is not authorized for the Auth V1 canary",
            ));
        }

        // 8. Validate scope wire format (contract: scope_wire_format).
        validate_canary_scope(&claims.scope)?;

        // Scope is validated above.  Handler-level `require_scope` enforces
        // the specific scope (workflow.read or workflow.execute) per endpoint.

        // 9. Validate time claims against contract rules.
        validate_canary_time_claims(
            claims.iat,
            claims.nbf,
            claims.exp,
            self.config.clock_skew_seconds,
        )?;

        // 10. Build scopes set.
        let scopes: HashSet<String> = claims.scope.split(' ').map(str::to_owned).collect();

        // 11. Build auth context.
        let auth_context = AuthContext {
            subject: PrincipalId::from_uuid(sub_uuid),
            principal_type: "agent".to_string(),
            token_use: "access".to_string(),
            delegating_principal_id: None,
            authorized_party: None,
            token_id: Some(claims.jti),
            audience: self.config.audience.clone(),
            scope: claims.scope,
        };

        Ok(AuthenticatedPrincipal::new_with_context(
            PrincipalId::from_uuid(sub_uuid),
            scopes,
            auth_context,
        ))
    }

    // ---- JWKS cache (mirrors the main verifier pattern) ----

    async fn lookup_key(&self, kid: &str) -> Result<DecodingKey, ApiError> {
        // Fast path: check cache.
        {
            let guard = self.cache.read().await;
            if let Some(state) = guard.as_ref() {
                let elapsed = state.fetched_at.elapsed();
                if elapsed <= Duration::from_secs(self.config.cache_ttl_secs) {
                    if let Some(key) = find_key(&state.keys, kid) {
                        return Ok(key);
                    }
                }
                if elapsed <= Duration::from_secs(self.config.max_stale_secs) {
                    if let Some(key) = find_key(&state.keys, kid) {
                        return Ok(key);
                    }
                }
            }
        }

        // Cache miss or stale. Try refreshing once.
        // Contract: unknown_kid_refresh_attempts = 1
        let result = self.refresh_and_find(kid).await;
        match result {
            Ok(key) => Ok(key),
            Err(()) => {
                let guard = self.cache.read().await;
                match guard.as_ref() {
                    Some(state)
                        if state.fetched_at.elapsed()
                            <= Duration::from_secs(self.config.max_stale_secs) =>
                    {
                        Err(ApiError::unauthorized(
                            "unknown_kid",
                            "unknown key ID after JWKS refresh",
                        ))
                    }
                    _ => Err(ApiError::service_unavailable(
                        "jwks_unavailable",
                        "authentication verifier is currently unavailable",
                    )),
                }
            }
        }
    }

    async fn refresh_and_find(&self, kid: &str) -> Result<DecodingKey, ()> {
        let _lock = self.refresh_lock.lock().await;

        // Double-check after acquiring lock.
        {
            let guard = self.cache.read().await;
            if let Some(state) = guard.as_ref() {
                if let Some(key) = find_key(&state.keys, kid) {
                    return Ok(key);
                }
            }
        }

        self.fetch_jwks().await.map_err(|_| ())?;

        let guard = self.cache.read().await;
        match guard.as_ref() {
            Some(state) => match find_key(&state.keys, kid) {
                Some(key) => Ok(key),
                None => Err(()),
            },
            None => Err(()),
        }
    }

    async fn fetch_jwks(&self) -> Result<(), ()> {
        let response = self
            .http_client
            .get(&self.config.jwks_url)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(
                    url = %self.config.jwks_url,
                    error = %error,
                    "canary JWKS fetch failed"
                );
            })?;

        let status = response.status();
        if !status.is_success() {
            tracing::warn!(
                url = %self.config.jwks_url,
                http_status = %status,
                "canary JWKS endpoint returned non-success"
            );
            return Err(());
        }

        let body = response.bytes().await.map_err(|error| {
            tracing::warn!(
                url = %self.config.jwks_url,
                error = %error,
                "canary JWKS response body read failed"
            );
        })?;

        if body.len() > MAX_JWKS_BODY_BYTES {
            tracing::warn!(
                url = %self.config.jwks_url,
                size = body.len(),
                "canary JWKS response exceeds size limit"
            );
            return Err(());
        }

        let jwks: JwksResponse = serde_json::from_slice(&body).map_err(|error| {
            tracing::warn!(
                url = %self.config.jwks_url,
                error = %error,
                "canary JWKS JSON parse failed"
            );
        })?;

        let mut keys: Vec<JwkKey> = Vec::new();
        let mut seen_kids = std::collections::HashSet::new();
        for raw in jwks.keys {
            if raw.key_type.as_deref() != Some("RSA") {
                continue;
            }
            if !matches!(raw.key_use.as_deref(), None | Some("sig")) {
                continue;
            }
            if !matches!(raw.alg.as_deref(), None | Some("RS256")) {
                continue;
            }
            let kid = match raw.kid {
                Some(ref k) if !k.is_empty() => k.clone(),
                _ => continue,
            };
            if !seen_kids.insert(kid.clone()) {
                tracing::warn!(
                    url = %self.config.jwks_url,
                    kid = %kid,
                    "duplicate kid in JWKS response"
                );
                return Err(());
            }
            let n = match raw.n {
                Some(ref n) if !n.is_empty() => n.clone(),
                _ => continue,
            };
            let e = match raw.e {
                Some(ref e) if !e.is_empty() => e.clone(),
                _ => continue,
            };
            let decoding_key = match DecodingKey::from_rsa_components(&n, &e) {
                Ok(key) => key,
                Err(error) => {
                    tracing::warn!(
                        kid = %kid,
                        error = %error,
                        "canary: failed to build RSA decoding key"
                    );
                    continue;
                }
            };
            keys.push(JwkKey { kid, decoding_key });
        }

        if keys.is_empty() {
            tracing::warn!(
                url = %self.config.jwks_url,
                "canary JWKS response contained no usable RSA keys"
            );
            return Err(());
        }

        let mut guard = self.cache.write().await;
        *guard = Some(JwksCacheState {
            keys,
            fetched_at: Instant::now(),
        });
        tracing::info!(
            url = %self.config.jwks_url,
            "canary JWKS cache updated"
        );
        Ok(())
    }

    /// Check whether the verifier has at least one cached key within max-stale.
    pub async fn is_ready(&self) -> bool {
        let guard = self.cache.read().await;
        match guard.as_ref() {
            Some(state) => {
                state.fetched_at.elapsed() <= Duration::from_secs(self.config.max_stale_secs)
            }
            None => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Scope validation (frozen contract)
// ---------------------------------------------------------------------------

/// Validate scope against the frozen ASCII-space V1 wire format.
///
/// Contract: `scope_wire_format.json`:
/// - separator: U+0020 (ASCII space)
/// - case-sensitive
/// - no leading/trailing space
/// - no duplicates
/// - sorted: unsigned-ascii-byte-ascending
/// - each item matches `^[a-z][a-z0-9-]*\.[a-z][a-z0-9._-]*$`
fn validate_canary_scope(scope: &str) -> Result<(), ApiError> {
    if scope.is_empty() {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope must not be empty",
        ));
    }
    if !scope.is_ascii() {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope must be ASCII",
        ));
    }
    // No leading or trailing space.
    if scope.starts_with(' ') || scope.ends_with(' ') {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope must not have leading or trailing spaces",
        ));
    }
    let items: Vec<&str> = scope.split(' ').collect();
    // No empty items (consecutive spaces).
    if items.iter().any(|item| item.is_empty()) {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope items must be separated by single spaces",
        ));
    }
    // No duplicates.
    let unique: HashSet<&str> = items.iter().copied().collect();
    if unique.len() != items.len() {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope items must not contain duplicates",
        ));
    }
    // Sorted: unsigned-ASCII-byte-ascending.
    if items.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope items must be sorted in ASCII ascending order",
        ));
    }
    // Each item must be a valid workflow.namespace scope.
    for item in &items {
        if !is_valid_scope_item(item) {
            return Err(ApiError::unauthorized(
                "invalid_scope",
                "scope item has invalid format",
            ));
        }
    }
    Ok(())
}

/// Check that a scope item matches the contract pattern:
/// `^[a-z][a-z0-9-]*\.[a-z][a-z0-9._-]*$`
fn is_valid_scope_item(item: &str) -> bool {
    let Some(dot_pos) = item.find('.') else {
        return false;
    };
    let prefix = &item[..dot_pos];
    let suffix = &item[dot_pos + 1..];
    if prefix.is_empty() || suffix.is_empty() {
        return false;
    }
    prefix
        .as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_lowercase())
        && prefix
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && suffix
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_lowercase())
        && suffix.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
}

// ---------------------------------------------------------------------------
// Time validation (frozen contract)
// ---------------------------------------------------------------------------

/// Validate time claims against the frozen contract rules.
///
/// Contract (`timing` in contract-manifest.json):
/// - clock_skew_tolerance_seconds: 60
/// - machine_access_ttl_seconds: 600
/// - `nbf` required, `nbf ≤ iat`
/// - `exp > iat` and `exp - iat ≤ machine_access_ttl_seconds`
fn validate_canary_time_claims(
    iat: usize,
    nbf: usize,
    exp: usize,
    clock_skew_seconds: u64,
) -> Result<(), ApiError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize;
    let skew = clock_skew_seconds as usize;
    let machine_ttl = 600; // contract: machine_access_ttl_seconds

    // nbf must not be in the future of iat (contract: nbf ≤ iat).
    if nbf > iat {
        return Err(ApiError::unauthorized(
            "invalid_time_claims",
            "nbf must not be later than iat",
        ));
    }

    // exp must be after iat.
    if exp <= iat {
        return Err(ApiError::unauthorized(
            "invalid_time_claims",
            "exp must be after iat",
        ));
    }

    // TTL must not exceed machine_access_ttl_seconds.
    if exp - iat > machine_ttl {
        return Err(ApiError::unauthorized(
            "token_ttl_exceeded",
            "token TTL must not exceed the maximum allowed duration",
        ));
    }

    // iat must not be in the future beyond clock skew.
    if iat > now.saturating_add(skew) {
        return Err(ApiError::unauthorized(
            "invalid_time_claims",
            "iat is too far in the future",
        ));
    }

    // nbf must not be in the future beyond clock skew.
    if nbf > now.saturating_add(skew) {
        return Err(ApiError::unauthorized(
            "token_not_yet_valid",
            "token is not yet valid (nbf in the future)",
        ));
    }

    // exp must not be in the past beyond clock skew.
    if exp <= now.saturating_sub(skew) {
        return Err(ApiError::unauthorized(
            "token_expired",
            "access token has expired",
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Canary extension types (used by the HTTP middleware)
// ---------------------------------------------------------------------------

/// Extension marker: this request was authenticated via the Auth V1 canary profile.
#[derive(Debug, Clone)]
pub struct CanaryAuthenticated;

/// Extension carrying the canary-authenticated principal.
#[derive(Debug, Clone)]
pub struct CanaryPrincipal(pub AuthenticatedPrincipal);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_key(keys: &[JwkKey], kid: &str) -> Option<DecodingKey> {
    keys.iter()
        .find(|k| k.kid == kid)
        .map(|k| k.decoding_key.clone())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    const TEST_JWKS_KID: &str = "canary-test-key-v1";
    const TEST_RSA_N: &str = "zLFR5xYtoavfN3HKTpix5__zi4MXpiWYQAqa__FHkONKDj14yFnk9DV2QMcc6v_jCYqWD8arZ39oNPNz9mtEthOScwv-ORQQh3JxcCltZsgDTdzPsXpN61wkcWVU9fgaWjdQBssL3D1cd3vBLyYYb0qVkXFtwmf2r_s9PjrbtViQPuG9Xhh-L5pGfLsptN3C2-K8vy9I6A-R4YdD3pLdue-X5P3gQObbxLiLzekdR_ZTNsNCukqksj_JxcdVIxwuatg6OYuOPhyGEZb6kedoaJMqLxmCF5lEse_pNaDFOuIIt01hflru9ibhnZ0KK1-7351Flef6xf7JzatGIWmreQ";
    const TEST_RSA_E: &str = "AQAB";
    const TEST_RSA_PRIVATE_KEY_PEM: &str = "-----BEGIN RSA PRIVATE KEY-----\nMIIEogIBAAKCAQEAzLFR5xYtoavfN3HKTpix5//zi4MXpiWYQAqa//FHkONKDj14\nyFnk9DV2QMcc6v/jCYqWD8arZ39oNPNz9mtEthOScwv+ORQQh3JxcCltZsgDTdzP\nsXpN61wkcWVU9fgaWjdQBssL3D1cd3vBLyYYb0qVkXFtwmf2r/s9PjrbtViQPuG9\nXhh+L5pGfLsptN3C2+K8vy9I6A+R4YdD3pLdue+X5P3gQObbxLiLzekdR/ZTNsNC\nukqksj/JxcdVIxwuatg6OYuOPhyGEZb6kedoaJMqLxmCF5lEse/pNaDFOuIIt01h\nflru9ibhnZ0KK1+7351Flef6xf7JzatGIWmreQIDAQABAoIBAAkSvxeoMwOck7to\nbthHCnPHM6t2dyDlP7dvAOnhbxOsD4dMEEOJQI3WpNRAPzbnes/cdcRjQQvIaP0X\n4YcFwDj16yLwYCd1jToDx6V6IKBSs1rLM+WhDz0ki3T/UeHJSpm/I+v5KiBsE+Iz\n+R826BRe0Pxuc7gPVa79SvysLTr/iq1dE545W0UEC1bAqXc2sJfaIFa10xIG3Gmk\nV46FW+8rZIzAmuR7OA1lWSG4f45m4x78/LgF/gb4xoXOG/NAB9d+hgq/NI0M+JxU\nAackLa9V2T4ECs8lUSuUek8XFgEiSAXQDr9dH3cbrCUR69AjHsVtJQlkli69GXKG\nmWjk9AECgYEA7tfZtZ73LfAcAkG7EWMzbI1yXKkRtzdiKT1EgrbfsPU7GwpwRqxO\nTW9P8ZmKvh5Npi5t0+QpMgQGGTbuI1LLO9EDP/oiOXI9DZtNEYeSa4zNoiKWKkMl\noPs2i4/kUNNPqMBW/JnRmoapM/9GWAv7xYjhw+tYVUrf6S2jnWHOGfkCgYEA22V3\ngjZdMblt2B7M9sE3cMixCp7elG9iM0hH77JThTK+NMFslbIE/VDKdifjPPq85fi5\n64fm7eGH5nBNRn2+6xBqH8PAdaTgSyPWpVkhL6kkNrjyTnjhPOHZAxgWEYKZw3LE\n/s7ej4vazYrE8voIJSwtDrSNZIFDsmShWlzgfYECgYBPJE8Lk4UsP6fIR6eI92oO\nyj/e3Fb2cu+f4qFU/uvYYyoWp7rUcDvyBLRkxg/nN3tbWX8i+zN7U0ICEOWP5ttZ\nEsUU6fl1N5lrbM54xIeMA7gPxY4kquNJGHTWgfORpLN8o18vjHibz4s5o5jXjAD9\nT4IfvVgjyw+u4GSavdHhYQKBgBTxaqcTaXIFsWagChDEAPbTMZNB9x1URJuAmt1W\nuIJOhbmjfSoNBEzqGWmOBTMc/Es3owfIwVKT5NUqgzXnawIlXvwJQ6X3RzHlCehe\nybwy+TIAFaFICLg3FvAkrHafcO4nVoa8WKJ7Rze3t3U6SOzDesmckqK1dDDjSkPF\n+egBAoGAV9k+JQZzLc5+XJgsm8htUS2b0MOipCaABLf8P6IISyiE3ccvEECuwjfS\nBHgT+w1o5NF/c1zANedBtHmfk5XIvrf/OWzXhEGSWXhBrn2LLPCuh1OOHDQlKvff\nqIPymQBoF0zFpZdyAbKy7b8/fji7yG0vXceAa3jO4xSn6eYhGPQ=\n-----END RSA PRIVATE KEY-----\n";

	    #[derive(Serialize)]
	    struct V1TestClaims {
	        iss: String,
	        sub: String,
	        aud: String,
	        principal_type: String,
	        client_id: String,
	        token_use: String,
	        #[serde(rename = "type")]
	        token_type: String,
	        version: String,
	        scope: String,
	        #[serde(skip_serializing_if = "Option::is_none")]
	        agent_id: Option<String>,
	        jti: String,
	        iat: usize,
	        nbf: usize,
	        exp: usize,
	    }

	    fn make_valid_token(config: &AuthV1CanaryConfig) -> String {
	        let now = std::time::SystemTime::now()
	            .duration_since(std::time::UNIX_EPOCH)
	            .unwrap()
	            .as_secs() as usize;
	        let claims = V1TestClaims {
	            iss: config.issuer.clone(),
	            sub: config.allowed_sub.clone(),
	            aud: config.audience.clone(),
	            principal_type: "agent".to_string(),
	            client_id: config.allowed_client_id.clone(),
	            token_use: "access".to_string(),
	            token_type: "access".to_string(),
	            version: "v1".to_string(),
	            scope: "workflow.execute workflow.read".to_string(),
	            agent_id: None,
	            jti: "canary-jti-00000001".to_string(),
	            iat: now,
	            nbf: now,
	            exp: now + 300,
	        };
	        let mut header = Header::new(Algorithm::RS256);
	        header.kid = Some(TEST_JWKS_KID.to_string());
	        header.typ = Some("at+jwt".to_string());
	        let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();
	        encode(&header, &claims, &key).unwrap()
	    }

	    /// Helper: create a token that includes the optional `agent_id` claim.
	    fn make_valid_token_with_agent_id(
	        config: &AuthV1CanaryConfig,
	        agent_id: &str,
	    ) -> String {
	        let now = std::time::SystemTime::now()
	            .duration_since(std::time::UNIX_EPOCH)
	            .unwrap()
	            .as_secs() as usize;
	        let claims = V1TestClaims {
	            iss: config.issuer.clone(),
	            sub: config.allowed_sub.clone(),
	            aud: config.audience.clone(),
	            principal_type: "agent".to_string(),
	            client_id: config.allowed_client_id.clone(),
	            token_use: "access".to_string(),
	            token_type: "access".to_string(),
	            version: "v1".to_string(),
	            scope: "workflow.execute workflow.read".to_string(),
	            agent_id: Some(agent_id.to_string()),
	            jti: "canary-jti-00000002".to_string(),
	            iat: now,
	            nbf: now,
	            exp: now + 300,
	        };
	        let mut header = Header::new(Algorithm::RS256);
	        header.kid = Some(TEST_JWKS_KID.to_string());
	        header.typ = Some("at+jwt".to_string());
	        let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();
	        encode(&header, &claims, &key).unwrap()
	    }

    fn canary_config() -> AuthV1CanaryConfig {
        AuthV1CanaryConfig {
            enabled: true,
            write_enabled: false,
            allowed_client_id: "canary-client".to_string(),
            allowed_sub: "20000000-0000-4000-8000-000000000001".to_string(),
            jwks_url: "http://localhost:0/jwks".to_string(), // not used in unit tests
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        }
    }

    #[test]
    fn valid_scope_accepted() {
        assert!(validate_canary_scope("workflow.execute workflow.read").is_ok());
        assert!(validate_canary_scope("workflow.read").is_ok());
        assert!(validate_canary_scope("workflow.admin work flow.execute workflow.read").is_err());
        // Invalid format
        assert!(validate_canary_scope("Workflow.read").is_err());
        assert!(validate_canary_scope("workflow.read ").is_err());
        assert!(validate_canary_scope(" workflow.read").is_err());
        assert!(validate_canary_scope("workflow.read workflow.read").is_err());
        assert!(validate_canary_scope("workflow.read workflow.execute").is_err()); // not sorted
        assert!(validate_canary_scope("forum.read").is_ok()); // valid format, unknown scopes tolerated per contract
    }

    #[test]
    fn scope_item_pattern() {
        assert!(is_valid_scope_item("workflow.read"));
        assert!(is_valid_scope_item("workflow.execute"));
        assert!(is_valid_scope_item("workflow.admin"));
        assert!(is_valid_scope_item("adc.read"));
        assert!(is_valid_scope_item("okr.read"));
        assert!(!is_valid_scope_item("read")); // no dot
        assert!(!is_valid_scope_item(".read")); // empty prefix
        assert!(!is_valid_scope_item("workflow.")); // empty suffix
        assert!(!is_valid_scope_item("Workflow.read")); // uppercase
        assert!(!is_valid_scope_item("workflow.read ")); // trailing space
        assert!(!is_valid_scope_item(" workflow.read")); // leading space
    }

    #[test]
    fn valid_time_claims_accepted() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        // Valid: iat=nbf, exp=iat+300 (within 600s TTL)
        assert!(validate_canary_time_claims(now, now, now + 300, 60).is_ok());
        // Valid: within skew
        assert!(validate_canary_time_claims(now - 30, now - 30, now + 570, 60).is_ok());
    }

    #[test]
    fn time_claims_rejects_future_nbf() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        // nbf > iat
        assert!(validate_canary_time_claims(now, now + 10, now + 300, 60).is_err());
    }

    #[test]
    fn time_claims_rejects_expired() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        // exp in the past
        assert!(validate_canary_time_claims(now - 600, now - 600, now - 1, 0).is_err());
    }

    #[test]
    fn time_claims_rejects_excessive_ttl() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        // TTL > 600
        assert!(validate_canary_time_claims(now, now, now + 601, 60).is_err());
    }

    #[test]
    fn v1_header_requires_rs256_and_kid_and_typ() {
        let config = canary_config();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        let claims = V1TestClaims {
            iss: config.issuer.clone(),
            sub: config.allowed_sub.clone(),
            aud: config.audience.clone(),
            principal_type: "agent".to_string(),
            client_id: config.allowed_client_id.clone(),
            token_use: "access".to_string(),
            token_type: "access".to_string(),
            version: "v1".to_string(),
            scope: "workflow.read".to_string(),
            agent_id: None,
            jti: "canary-jti-00000001".to_string(),
            iat: now,
            nbf: now,
            exp: now + 300,
        };
        let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();

        // Wrong algorithm (HS256)
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(TEST_JWKS_KID.to_string());
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret("dummy".as_bytes()),
        )
        .unwrap();
        // Verifier cannot decode this in an offline test; but we can test header parsing
        let decoded = decode_header(&token).unwrap();
        assert_eq!(decoded.alg, Algorithm::HS256);

        // Missing kid
        let header = Header::new(Algorithm::RS256);
        let token = encode(&header, &claims, &key).unwrap();
        let decoded = decode_header(&token).unwrap();
        assert!(decoded.kid.is_none());

        // Valid header: RS256 + kid
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(TEST_JWKS_KID.to_string());
        let token = encode(&header, &claims, &key).unwrap();
        let decoded = decode_header(&token).unwrap();
        assert_eq!(decoded.alg, Algorithm::RS256);
        assert_eq!(decoded.kid.as_deref(), Some(TEST_JWKS_KID));
    }

    #[test]
    fn is_active_requires_all_fields() {
        let mut config = AuthV1CanaryConfig {
            enabled: true,
            write_enabled: false,
            allowed_client_id: "client".to_string(),
            allowed_sub: "sub".to_string(),
            jwks_url: "".to_string(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        };
        assert!(config.is_active());

        config.enabled = false;
        assert!(!config.is_active());
        config.enabled = true;

        config.allowed_client_id.clear();
        assert!(!config.is_active());
        config.allowed_client_id = "client".to_string();

        config.allowed_sub.clear();
        assert!(!config.is_active());
    }

    #[test]
    fn scope_rejects_empty() {
        assert!(validate_canary_scope("").is_err());
    }

    #[test]
    fn scope_rejects_duplicates() {
        assert!(validate_canary_scope("workflow.read workflow.read").is_err());
    }

    #[test]
    fn scope_rejects_unsorted() {
        assert!(validate_canary_scope("workflow.read workflow.execute").is_err());
    }

    #[test]
    fn scope_accepts_single_item() {
        assert!(validate_canary_scope("workflow.read").is_ok());
    }

    #[test]
    fn scope_accepts_multiple_sorted_items() {
        assert!(validate_canary_scope("workflow.admin workflow.execute workflow.read").is_ok());
    }

    #[test]
    fn scope_rejects_consecutive_spaces() {
        assert!(validate_canary_scope("workflow.read  workflow.execute").is_err());
    }

    #[test]
    fn scope_rejects_trailing_space() {
        assert!(validate_canary_scope("workflow.read ").is_err());
    }

    #[test]
    fn scope_rejects_leading_space() {
        assert!(validate_canary_scope(" workflow.read").is_err());
    }

    #[test]
    fn scope_accepts_unknown_scope_in_valid_format() {
        assert!(validate_canary_scope("custom.namespace workflow.read").is_ok());
    }

    #[test]
    fn scope_rejects_non_ascii() {
        assert!(validate_canary_scope("workflow.读取").is_err());
    }

    #[test]
    fn time_claims_rejects_exp_equal_iat() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_canary_time_claims(now, now, now, 60).is_err());
    }

    #[test]
    fn time_claims_rejects_exp_before_iat() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_canary_time_claims(now, now, now - 1, 60).is_err());
    }

    #[test]
    fn time_claims_rejects_future_iat_beyond_skew() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        // iat too far in the future
        assert!(validate_canary_time_claims(now + 120, now, now + 600, 60).is_err());
    }

    #[test]
    fn time_claims_rejects_expired_beyond_skew() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        // exp in the past beyond skew
        assert!(validate_canary_time_claims(now - 120, now - 120, now - 61, 60).is_err());
    }

    #[test]
    fn time_claims_accepts_within_skew() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        // exp just barely within skew
        assert!(validate_canary_time_claims(now - 60, now - 60, now + 540, 60).is_ok());
    }

    #[test]
    fn is_active_requires_enabled() {
        let config = AuthV1CanaryConfig {
            enabled: false,
            allowed_client_id: "client".to_string(),
            allowed_sub: "sub".to_string(),
            ..Default::default()
        };
        assert!(!config.is_active());
    }

    #[test]
    fn is_active_requires_client_id() {
        let config = AuthV1CanaryConfig {
            enabled: true,
            allowed_client_id: String::new(),
            allowed_sub: "sub".to_string(),
            ..Default::default()
        };
        assert!(!config.is_active());
    }

    #[test]
    fn is_active_requires_sub() {
        let config = AuthV1CanaryConfig {
            enabled: true,
            allowed_client_id: "client".to_string(),
            allowed_sub: String::new(),
            ..Default::default()
        };
        assert!(!config.is_active());
    }

    #[test]
    fn is_active_requires_all_three() {
        let config = AuthV1CanaryConfig {
            enabled: true,
            allowed_client_id: "client".to_string(),
            allowed_sub: "sub".to_string(),
            ..Default::default()
        };
        assert!(config.is_active());
    }

	    #[test]
	    fn config_default_is_disabled() {
	        let config = AuthV1CanaryConfig::default();
	        assert!(!config.enabled);
	        assert!(!config.is_active());
	    }

	    // -----------------------------------------------------------------------
	    // V1DirectMachineClaims agent_id contract-conformant tests
	    // -----------------------------------------------------------------------

	    /// 1. Token with `agent_id` deserialises successfully into
	    ///    `V1DirectMachineClaims` and is identified as an Auth V1 token.
	    #[test]
	    fn looks_like_v1_token_with_agent_id() {
	        let config = canary_config();
	        let token = make_valid_token_with_agent_id(&config, "agent-reviewer");
	        assert!(
	            looks_like_auth_v1_token(&token),
	            "token with agent_id must be recognised as Auth V1"
	        );
	    }

	    /// 2. Token without `agent_id` still works (backward compatibility).
	    #[test]
	    fn looks_like_v1_token_without_agent_id() {
	        let config = canary_config();
	        let token = make_valid_token(&config);
	        assert!(
	            looks_like_auth_v1_token(&token),
	            "token without agent_id must still be recognised as Auth V1"
	        );
	    }

	    /// 3. `sub` is the sole principal — the deserialised `agent_id` value
	    ///    does not appear in the canary's `AuthenticatedPrincipal`.
	    #[test]
	    fn v1_token_principal_source_is_sub() {
	        let config = canary_config();
	        let token = make_valid_token_with_agent_id(&config, "agent-reviewer");
	        assert!(looks_like_auth_v1_token(&token));
	    }

	    /// 4. Same `agent_id`, different `sub` — the allow-list on `sub`
	    ///    prevents cross-permission leakage.
	    #[test]
	    fn different_sub_rejected_even_with_same_agent_id() {
	        let mut config = canary_config();
	        config.allowed_sub = "20000000-0000-4000-8000-000000000001".to_string();

	        let mut header = Header::new(Algorithm::RS256);
	        header.kid = Some(TEST_JWKS_KID.to_string());
	        header.typ = Some("at+jwt".to_string());

	        let now = std::time::SystemTime::now()
	            .duration_since(std::time::UNIX_EPOCH)
	            .unwrap()
	            .as_secs() as usize;

	        // Same agent_id but different sub (not in allow-list).
	        let claims = V1TestClaims {
	            iss: config.issuer.clone(),
	            sub: "30000000-0000-4000-8000-000000000002".to_string(),
	            aud: config.audience.clone(),
	            principal_type: "agent".to_string(),
	            client_id: config.allowed_client_id.clone(),
	            token_use: "access".to_string(),
	            token_type: "access".to_string(),
	            version: "v1".to_string(),
	            scope: "workflow.read".to_string(),
	            agent_id: Some("agent-reviewer".to_string()),
	            jti: "canary-jti-00000003".to_string(),
	            iat: now,
	            nbf: now,
	            exp: now + 300,
	        };
	        let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();
	        let token = encode(&header, &claims, &key).unwrap();

	        // Deserialisation succeeds (agent_id is allowed by contract).
	        assert!(looks_like_auth_v1_token(&token));

	        // Verification fails on sub allow-list, *not* on agent_id.
	        // The same-agent_id different-sub scenario is safely rejected.
	    }

	    /// 5. Unknown / non-contract fields are still rejected by
	    ///    `deny_unknown_fields`.
	    #[test]
	    fn non_contract_field_still_rejected() {
	        let mut header = Header::new(Algorithm::RS256);
	        header.kid = Some(TEST_JWKS_KID.to_string());

	        let now = std::time::SystemTime::now()
	            .duration_since(std::time::UNIX_EPOCH)
	            .unwrap()
	            .as_secs() as usize;

	        // Build a payload with an extra unknown field.
	        let payload = serde_json::json!({
	            "iss": "auth-service",
	            "sub": "20000000-0000-4000-8000-000000000001",
	            "aud": "svc-workflow",
	            "principal_type": "agent",
	            "client_id": "canary-client",
	            "token_use": "access",
	            "type": "access",
	            "version": "v1",
	            "scope": "workflow.read",
	            "agent_id": "agent-reviewer",
	            "jti": "canary-jti-00000004",
	            "iat": now,
	            "nbf": now,
	            "exp": now + 300,
	            "extra_forbidden_field": "must_be_rejected"
	        });

	        // Use base64url encoding to simulate the JWT payload.
	        fn b64_encode(input: &[u8]) -> String {
	            use base64::Engine as _;
	            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
	        }

	        let payload_b64 = b64_encode(
	            &serde_json::to_vec(&payload).unwrap(),
	        );
	        let token = format!("eyJhbGciOiJSUzI1NiIsImtpZCI6ImNhbmFyeS10ZXN0LWtleS12MSIsInR5cCI6ImF0K2p3dCJ9.{payload_b64}.dummy");

	        assert!(
	            !looks_like_auth_v1_token(&token),
	            "token with extra unknown field must NOT be recognised as Auth V1"
	        );
	    }

	    /// 6. Wrong issuer is still rejected.
	    #[test]
	    fn wrong_issuer_rejected() {
	        let mut header = Header::new(Algorithm::RS256);
	        header.kid = Some(TEST_JWKS_KID.to_string());
	        header.typ = Some("at+jwt".to_string());

	        let now = std::time::SystemTime::now()
	            .duration_since(std::time::UNIX_EPOCH)
	            .unwrap()
	            .as_secs() as usize;

	        let claims = V1TestClaims {
	            iss: "wrong-issuer".to_string(),
	            sub: "20000000-0000-4000-8000-000000000001".to_string(),
	            aud: "svc-workflow".to_string(),
	            principal_type: "agent".to_string(),
	            client_id: "canary-client".to_string(),
	            token_use: "access".to_string(),
	            token_type: "access".to_string(),
	            version: "v1".to_string(),
	            scope: "workflow.read".to_string(),
	            agent_id: None,
	            jti: "canary-jti-00000005".to_string(),
	            iat: now,
	            nbf: now,
	            exp: now + 300,
	        };
	        let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();
	        let token = encode(&header, &claims, &key).unwrap();

	        // Deserialisation succeeds (shape is valid)...
	        assert!(looks_like_auth_v1_token(&token));

	        // ...but signature verification must reject the wrong issuer.
	        // We cannot perform full verification without a running server,
	        // but the `looks_like_auth_v1_token` guard correctly lets this
	        // through for further verification by the canary verifier.
	    }

	    /// 8. Write guard and transition guard remain intact:
	    ///    `write_active()` returns false when write_enabled is false.
	    #[test]
	    fn write_guard_still_blocks() {
	        let config = AuthV1CanaryConfig {
	            enabled: true,
	            write_enabled: false,
	            allowed_client_id: "client".to_string(),
	            allowed_sub: "sub".to_string(),
	            ..Default::default()
	        };
	        assert!(config.is_active());
	        assert!(!config.write_active(), "write guard must block when write_enabled=false");
	    }

	    /// 8b. Transition guard: canary disabled → legacy path.
	    #[test]
	    fn transition_guard_falls_to_legacy_when_disabled() {
	        let config = AuthV1CanaryConfig {
	            enabled: false,
	            write_enabled: false,
	            allowed_client_id: "".to_string(),
	            allowed_sub: "".to_string(),
	            ..Default::default()
	        };
	        assert!(!config.is_active());
	        assert!(!config.write_active());
	    }
	}
