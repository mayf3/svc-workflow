//! RS256 JWKS verifier with Auth V1 profile enforcement.
//!
//! This is the sole authentication verifier — service no longer supports
//! HS256 or legacy claims.  Tokens are validated against the strict
//! `V1DirectMachineClaims` (Direct, `token_use=access`) or
//! `V1OboMachineClaims` (OBO, `token_use=workflow_obo`) structs,
//! both with RS256 / `deny_unknown_fields`.
//!
//! Profile dispatch is based exclusively on `token_use` — no guessing
//! or fallback between profiles.

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
use super::auth_mode::JwksConfig;
use super::canary::AuthV1CanaryConfig;
use super::claims::{self, V1DirectMachineClaims, V1OboMachineClaims};
use super::AuthenticatedPrincipal;

/// Maximum response body size for JWKS fetch (1 MB).
const MAX_JWKS_BODY_BYTES: usize = 1_048_576;

/// A raw JWK entry from the JWKS endpoint.
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

/// JWKS response body.
#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<RawJwk>,
}

/// A cached JWK with its `DecodingKey`.
struct JwkKey {
    kid: String,
    decoding_key: DecodingKey,
}

/// State of the JWKS cache.
struct JwksCacheState {
    keys: Vec<JwkKey>,
    fetched_at: Instant,
}

/// RS256 JWKS verifier with Auth V1 profile enforcement.
pub struct JwksVerifier {
    cache: Arc<tokio::sync::RwLock<Option<JwksCacheState>>>,
    http_client: reqwest::Client,
    jwks_url: String,
    cache_ttl: Duration,
    max_stale: Duration,
    refresh_lock: Arc<Mutex<()>>,
    issuer: String,
    audience: String,
    clock_skew_seconds: u64,
    /// Auth V1 feature flags and allow-list.
    config: AuthV1CanaryConfig,
}

impl JwksVerifier {
    /// Create a new `JwksVerifier`.
    ///
    /// An initial JWKS fetch is attempted eagerly in the background.
    pub fn new(config: &JwksConfig, canary_config: &AuthV1CanaryConfig) -> Self {
        let cache: Arc<tokio::sync::RwLock<Option<JwksCacheState>>> =
            Arc::new(tokio::sync::RwLock::new(None));
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.http_timeout_secs))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("reqwest::Client builder with valid config");
        let verifier = Self {
            cache,
            http_client,
            jwks_url: config.jwks_url.clone(),
            cache_ttl: Duration::from_secs(config.cache_ttl_secs),
            max_stale: Duration::from_secs(config.max_stale_secs),
            refresh_lock: Arc::new(Mutex::new(())),
            issuer: config.issuer.clone(),
            audience: config.audience.clone(),
            clock_skew_seconds: config.clock_skew_seconds,
            config: canary_config.clone(),
        };
        // Eagerly attempt initial fetch.
        let eager = verifier.clone();
        tokio::spawn(async move {
            let _ = eager.fetch_jwks().await;
        });
        verifier
    }

    /// Check whether the verifier has at least one cached key within max-stale.
    ///
    /// A stale or missing cache triggers a best-effort JWKS refresh before
    /// answering, so readiness recovers on its own instead of waiting for the
    /// next authenticated request (which may never come).
    pub async fn is_ready(&self) -> bool {
        {
            let guard = self.cache.read().await;
            if let Some(state) = guard.as_ref() {
                if state.fetched_at.elapsed() <= self.max_stale {
                    return true;
                }
            }
        }
        // Stale or missing — refresh once, serialized with refresh_and_find.
        // A failed fetch keeps the previous cache state (and previous result).
        let _lock = self.refresh_lock.lock().await;
        {
            let guard = self.cache.read().await;
            if let Some(state) = guard.as_ref() {
                if state.fetched_at.elapsed() <= self.max_stale {
                    return true;
                }
            }
        }
        let _ = self.fetch_jwks().await;
        match self.cache.read().await.as_ref() {
            Some(state) => state.fetched_at.elapsed() <= self.max_stale,
            None => false,
        }
    }

    /// Verify a JWT against the Auth V1 profile.
    ///
    /// Algorithm must be RS256. Token must include a `kid` that maps to
    /// a cached JWKS key. Unknown kids trigger a controlled refresh.
    ///
    /// Profile (`Direct` vs `OBO`) is determined exclusively by the
    /// `token_use` claim — no guessing or fallback.
    ///
    /// Returns `AuthenticatedPrincipal` with `principal_id = token.sub`.
    pub async fn verify(&self, token: &str) -> Result<AuthenticatedPrincipal, ApiError> {
        // 1. Master switch check.
        if !self.config.enabled {
            return Err(ApiError::unauthorized(
                "auth_v1_disabled",
                "Auth V1 authentication is not enabled",
            ));
        }

        // 2. Decode and validate the JWT header.
        let header = decode_header(token)
            .map_err(|_| ApiError::unauthorized("malformed_token", "malformed JWT header"))?;

        // Algorithm must be RS256 (contract: signing.algorithm = RS256).
        if header.alg != Algorithm::RS256 {
            return Err(ApiError::unauthorized(
                "algorithm_not_allowed",
                "only RS256 algorithm is accepted",
            ));
        }

        // Kid is required for JWKS-based verification.
        let kid = header.kid.as_deref().ok_or_else(|| {
            ApiError::unauthorized("malformed_token", "JWT must include a kid header")
        })?;

        // 3. Look up the key, optionally refreshing cache.
        let key = self.lookup_key(kid).await?;

        // 4. Build validation — standard JWT checks.
        let mut validation = Validation::new(Algorithm::RS256);
        validation.algorithms = vec![Algorithm::RS256];
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        for claim in ["exp", "iat", "nbf", "sub"] {
            validation.required_spec_claims.insert(claim.to_string());
        }
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = self.clock_skew_seconds;

        // 5. Decode as serde_json::Value to verify signature + standard claims.
        // The Value is then used to extract token_use for profile routing.
        // Note: we let jsonwebtoken validate iss/aud values, but then we enforce
        // single-string audience via manual check below (rejecting arrays).
        let data = decode::<serde_json::Value>(token, &key, &validation).map_err(|error| {
            match error.kind() {
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
                _ => {
                    let msg = format!("invalid token: {}", error);
                    ApiError::unauthorized("malformed_token", Box::leak(msg.into_boxed_str()))
                }
            }
        })?;

        let claims_value = &data.claims;

        // 6. Enforce single-string audience (jsonwebtoken accepts arrays).
        // jsonwebtoken's set_audience allows both single string and arrays
        // containing the expected value. We require a single string here.
        if claims_value.get("aud").and_then(|v| v.as_str()).is_none() {
            return Err(ApiError::unauthorized(
                "wrong_audience",
                "audience must be a single string",
            ));
        }

        // 7. Determine profile by token_use — no fallback.
        let token_use = claims_value
            .get("token_use")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::unauthorized("malformed_token", "token_use is required"))?;

        match token_use {
            "access" => self.verify_direct(claims_value.clone()),
            "workflow_obo" => self.verify_obo(claims_value.clone()),
            _ => Err(ApiError::unauthorized(
                "invalid_token_use",
                "unknown token_use value",
            )),
        }
    }

    /// Verify a Direct Machine token (`token_use=access`).
    ///
    /// Deserializes the verified Value as `V1DirectMachineClaims` (strict,
    /// `deny_unknown_fields`) and runs the full Direct profile validation.
    fn verify_direct(
        &self,
        claims_value: serde_json::Value,
    ) -> Result<AuthenticatedPrincipal, ApiError> {
        // Deserialize as strict Direct claims.
        let claims: V1DirectMachineClaims =
            serde_json::from_value(claims_value).map_err(|error| {
                let msg = format!("invalid access token: {error}");
                ApiError::unauthorized("malformed_token", Box::leak(msg.into_boxed_str()))
            })?;

        // Validate V1 profile claims.
        if claims.principal_type != "agent" {
            return Err(ApiError::unauthorized(
                "invalid_principal_type",
                "principal_type must be agent",
            ));
        }
        if claims.token_type != "access" {
            return Err(ApiError::unauthorized(
                "invalid_token_type",
                "token type must be access",
            ));
        }
        if claims.version != "v1" {
            return Err(ApiError::unauthorized(
                "malformed_token",
                "token version must be v1",
            ));
        }
        if claims.jti.len() < 16 {
            return Err(ApiError::unauthorized(
                "malformed_token",
                "jti must contain at least 16 characters",
            ));
        }

        // Validate sub is a valid UUID.
        let sub_uuid = Uuid::parse_str(&claims.sub)
            .map_err(|_| ApiError::unauthorized("malformed_token", "sub must be a valid UUID"))?;

        // Validate allow-list.
        if !self.config.allowed_client_id.is_empty()
            && claims.client_id != self.config.allowed_client_id
        {
            return Err(ApiError::unauthorized(
                "unauthorized_client",
                "client_id is not authorized",
            ));
        }
        if !self.config.allowed_sub.is_empty() && claims.sub != self.config.allowed_sub {
            return Err(ApiError::unauthorized(
                "unauthorized_principal",
                "sub is not authorized",
            ));
        }

        // Validate scope wire format.
        claims::validate_v1_scope(&claims.scope)?;

        // Validate time claims.
        claims::validate_v1_time_claims(
            claims.iat,
            claims.nbf,
            claims.exp,
            self.clock_skew_seconds,
        )?;

        // Build scopes set.
        let scopes: HashSet<String> = claims.scope.split(' ').map(str::to_owned).collect();

        // Build auth context — Direct profile, no delegation.
        let auth_context = AuthContext {
            subject: PrincipalId::from_uuid(sub_uuid),
            principal_type: "agent".to_string(),
            token_use: "access".to_string(),
            delegating_principal_id: None,
            authorized_party: None,
            client_id: Some(claims.client_id),
            token_id: Some(claims.jti),
            audience: self.audience.clone(),
            scope: claims.scope,
        };

        Ok(AuthenticatedPrincipal::new_with_context(
            PrincipalId::from_uuid(sub_uuid),
            scopes,
            auth_context,
        ))
    }

    /// Verify an OBO token (`token_use=workflow_obo`).
    ///
    /// Deserializes the verified Value as `V1OboMachineClaims` (strict,
    /// `deny_unknown_fields`, `act.sub` required).  The domain actor is
    /// always `token.sub` — `act.sub` is used only for audit as the
    /// delegating principal.
    fn verify_obo(
        &self,
        claims_value: serde_json::Value,
    ) -> Result<AuthenticatedPrincipal, ApiError> {
        // Deserialize as strict OBO claims.
        let claims: V1OboMachineClaims = serde_json::from_value(claims_value).map_err(|error| {
            let msg = format!("invalid obo token: {error}");
            ApiError::unauthorized("malformed_token", Box::leak(msg.into_boxed_str()))
        })?;

        // Validate V1 profile claims (shared with Direct).
        if claims.principal_type != "agent" {
            return Err(ApiError::unauthorized(
                "invalid_principal_type",
                "principal_type must be agent",
            ));
        }
        if claims.token_type != "access" {
            return Err(ApiError::unauthorized(
                "invalid_token_type",
                "token type must be access",
            ));
        }
        if claims.version != "v1" {
            return Err(ApiError::unauthorized(
                "malformed_token",
                "token version must be v1",
            ));
        }
        if claims.jti.is_empty() {
            return Err(ApiError::unauthorized("malformed_token", "jti is required"));
        }

        // Validate sub is a valid UUID.
        let sub_uuid = Uuid::parse_str(&claims.sub)
            .map_err(|_| ApiError::unauthorized("malformed_token", "sub must be a valid UUID"))?;

        // Validate act.sub is a valid UUID.
        let act_sub_uuid = Uuid::parse_str(&claims.act.sub).map_err(|_| {
            ApiError::unauthorized("malformed_token", "act.sub must be a valid UUID")
        })?;

        // Validate allow-lists — OBO requires both allowed_sub and
        // allowed_delegating_sub (when either is configured).
        if !self.config.allowed_sub.is_empty() && claims.sub != self.config.allowed_sub {
            return Err(ApiError::unauthorized(
                "unauthorized_principal",
                "sub is not authorized",
            ));
        }
        if !self.config.allowed_delegating_sub.is_empty()
            && claims.act.sub != self.config.allowed_delegating_sub
        {
            return Err(ApiError::unauthorized(
                "unauthorized_delegating_principal",
                "delegating principal is not authorized",
            ));
        }

        // Validate scope wire format.
        claims::validate_v1_scope(&claims.scope)?;

        // Validate time claims.
        claims::validate_v1_time_claims(
            claims.iat,
            claims.nbf,
            claims.exp,
            self.clock_skew_seconds,
        )?;

        // Build scopes set.
        let scopes: HashSet<String> = claims.scope.split(' ').map(str::to_owned).collect();

        // Build auth context — OBO profile with delegation audit trail.
        let auth_context = AuthContext {
            subject: PrincipalId::from_uuid(sub_uuid),
            principal_type: "agent".to_string(),
            token_use: "workflow_obo".to_string(),
            delegating_principal_id: Some(PrincipalId::from_uuid(act_sub_uuid)),
            authorized_party: claims.azp.clone(),
            client_id: claims.client_id.clone(),
            token_id: Some(claims.jti),
            audience: self.audience.clone(),
            scope: claims.scope,
        };

        Ok(AuthenticatedPrincipal::new_with_context(
            PrincipalId::from_uuid(sub_uuid),
            scopes,
            auth_context,
        ))
    }

    // ---- JWKS cache ----

    async fn lookup_key(&self, kid: &str) -> Result<DecodingKey, ApiError> {
        // Fast path: check cache.
        {
            let guard = self.cache.read().await;
            if let Some(state) = guard.as_ref() {
                let elapsed = state.fetched_at.elapsed();
                if elapsed <= self.cache_ttl {
                    if let Some(key) = find_key(&state.keys, kid) {
                        return Ok(key);
                    }
                }
                if elapsed <= self.max_stale {
                    if let Some(key) = find_key(&state.keys, kid) {
                        return Ok(key);
                    }
                }
            }
        }

        // Cache miss or stale. Try refreshing.
        let result = self.refresh_and_find(kid).await;
        match result {
            Ok(key) => Ok(key),
            Err(_) => {
                let guard = self.cache.read().await;
                match guard.as_ref() {
                    Some(state) if state.fetched_at.elapsed() <= self.max_stale => Err(
                        ApiError::unauthorized("unknown_kid", "unknown key ID after JWKS refresh"),
                    ),
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

        // Double-check after acquiring lock — only a fresh cache may short
        // circuit. A stale cache must pass through fetch_jwks so fetched_at
        // advances and readiness (is_ready) can recover.
        {
            let guard = self.cache.read().await;
            if let Some(state) = guard.as_ref() {
                if state.fetched_at.elapsed() <= self.max_stale {
                    if let Some(key) = find_key(&state.keys, kid) {
                        return Ok(key);
                    }
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
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(url = %self.jwks_url, error = %error, "JWKS fetch failed");
            })?;

        let status = response.status();
        if !status.is_success() {
            tracing::warn!(url = %self.jwks_url, http_status = %status, "JWKS endpoint returned non-success");
            return Err(());
        }

        let body = response.bytes().await.map_err(|error| {
            tracing::warn!(url = %self.jwks_url, error = %error, "JWKS response body read failed");
        })?;

        if body.len() > MAX_JWKS_BODY_BYTES {
            tracing::warn!(url = %self.jwks_url, size = body.len(), "JWKS response exceeds size limit");
            return Err(());
        }

        let jwks: JwksResponse = serde_json::from_slice(&body).map_err(|error| {
            tracing::warn!(url = %self.jwks_url, error = %error, "JWKS JSON parse failed");
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
                tracing::warn!(url = %self.jwks_url, kid = %kid, "duplicate kid in JWKS response");
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
                    tracing::warn!(kid = %kid, error = %error, "failed to build RSA decoding key");
                    continue;
                }
            };
            keys.push(JwkKey { kid, decoding_key });
        }

        if keys.is_empty() {
            tracing::warn!(
                url = %self.jwks_url,
                "JWKS response contained no usable RSA keys"
            );
            return Err(());
        }

        let mut guard = self.cache.write().await;
        *guard = Some(JwksCacheState {
            keys,
            fetched_at: Instant::now(),
        });
        tracing::info!(url = %self.jwks_url, "JWKS cache updated successfully");
        Ok(())
    }
}

impl Clone for JwksVerifier {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            http_client: self.http_client.clone(),
            jwks_url: self.jwks_url.clone(),
            cache_ttl: self.cache_ttl,
            max_stale: self.max_stale,
            refresh_lock: self.refresh_lock.clone(),
            issuer: self.issuer.clone(),
            audience: self.audience.clone(),
            clock_skew_seconds: self.clock_skew_seconds,
            config: self.config.clone(),
        }
    }
}

fn find_key(keys: &[JwkKey], kid: &str) -> Option<DecodingKey> {
    keys.iter()
        .find(|k| k.kid == kid)
        .map(|k| k.decoding_key.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_key_matches_kid() {
        let key1 = JwkKey {
            kid: "key-1".to_string(),
            decoding_key: DecodingKey::from_rsa_components("dGVzdA", "AQAB").unwrap(),
        };
        let key2 = JwkKey {
            kid: "key-2".to_string(),
            decoding_key: DecodingKey::from_rsa_components("dGVzdA", "AQAB").unwrap(),
        };
        let keys = vec![key1, key2];
        assert!(find_key(&keys, "key-1").is_some());
        assert!(find_key(&keys, "key-2").is_some());
        assert!(find_key(&keys, "key-3").is_none());
    }
}
