//! Auth V1 Canary route guard middleware — fail-closed.
//!
//! ## Design
//!
//! Both guards implement a **fail-closed** two-phase pipeline:
//!
//! 1. **Token identification** — does the request carry a JWT that matches
//!    the Auth V1 DirectMachineAccess profile shape (RS256 + kid + strict
//!    claims set)?  If yes, the token **must** pass the canary path or be
//!    rejected.  It never falls through to legacy auth.
//!
//! 2. **Canary validation** — feature flags, allow-list, V1 verification,
//!    and (for write) the write-gate flag.
//!
//! Tokens that do **not** match the V1 shape (e.g. HS256 tokens, tokens
//! with extra claims such as `agent_id`) are passed through to the legacy
//! `AuthenticatedPrincipal` extractor.
//!
//! ## Route isolation
//!
//! - `canary_worklist_guard`: attached to `GET /internal/v1/worklists/assigned-to-me`
//! - `canary_write_guard`: attached to `POST /internal/v1/workflow-instances`
//!   and `POST /internal/v1/workflow-instances/{id}/transitions`

use axum::body::Body;
use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::http::{Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::auth::looks_like_auth_v1_token;
use crate::auth::{CanaryAuthenticated, CanaryPrincipal};
use crate::http::error::ApiError;
use crate::http::AppState;

// ---------------------------------------------------------------------------
// Decision enum
// ---------------------------------------------------------------------------

/// Outcome of the canary guard's decision logic.
#[derive(Debug)]
enum CanaryDecision {
    /// The request carries a recognised Auth V1 token that was verified and
    /// authorised.  `CanaryAuthenticated` / `CanaryPrincipal` have been
    /// injected into the request extensions.  The middleware chain should
    /// continue to the handler.
    Accept,
    /// The request carries a recognised Auth V1 token but one or more checks
    /// failed.  The caller must return this response immediately.
    Reject(Response),
    /// The request does **not** carry an Auth V1 token.  The guard should
    /// pass through to legacy auth (the `AuthenticatedPrincipal` extractor).
    FallThrough,
}

// ---------------------------------------------------------------------------
// Shared decision function
// ---------------------------------------------------------------------------

/// Run the fail-closed canary decision for a request.
///
/// `is_write_guard` controls whether the write-gate flag is checked
/// (write guards only).
async fn decide_canary_action(
    state: &AppState,
    token: &str,
    request: &mut Request<Body>,
    is_write_guard: bool,
) -> CanaryDecision {
    // 1. Token identification.
    if !looks_like_auth_v1_token(token) {
        return CanaryDecision::FallThrough;
    }

    // The token looks like an Auth V1 token.  From this point onward the
    // canary path is authoritative — no fallback to legacy auth.

    // 2. Canary must be active (enabled + configured).
    let verifier = match state.auth_v1_canary_verifier.as_ref() {
        Some(v) if state.auth_v1_canary_config.is_active() => v,
        _ => {
            return CanaryDecision::Reject(
                ApiError::unauthorized(
                    "auth_v1_disabled",
                    "Auth V1 authentication is not available",
                )
                .into_response(),
            );
        }
    };

    // 3. Allow-list check (peek at client_id + sub before full verification).
    match peek_token_claims(token) {
        Ok(Some(peek)) => {
            let allowed_client = &state.auth_v1_canary_config.allowed_client_id;
            let allowed_sub = &state.auth_v1_canary_config.allowed_sub;

            let client_ok = peek.client_id.as_deref() == Some(allowed_client.as_str());
            let sub_ok = peek.sub.as_deref() == Some(allowed_sub.as_str());

            if !client_ok {
                return CanaryDecision::Reject(
                    ApiError::unauthorized(
                        "unauthorized_client",
                        "client_id is not authorised for the Auth V1 canary",
                    )
                    .into_response(),
                );
            }
            if !sub_ok {
                return CanaryDecision::Reject(
                    ApiError::unauthorized(
                        "unauthorized_principal",
                        "sub is not authorised for the Auth V1 canary",
                    )
                    .into_response(),
                );
            }
        }
        Ok(None) => {
            // V1-shaped token but claims could not be peeked — reject.
            return CanaryDecision::Reject(
                ApiError::unauthorized("malformed_token", "malformed Auth V1 token")
                    .into_response(),
            );
        }
        Err(_) => {
            // Malformed payload — reject.
            return CanaryDecision::Reject(
                ApiError::unauthorized("malformed_token", "malformed Auth V1 token")
                    .into_response(),
            );
        }
    }

    // 4. Write-gate check (only for write guards).
    if is_write_guard && !state.auth_v1_canary_config.write_enabled {
        return CanaryDecision::Reject(
            ApiError::new(
                StatusCode::FORBIDDEN,
                "canary_read_only",
                "the Auth V1 canary profile only allows read operations on assigned-to-me",
            )
            .into_response(),
        );
    }

    // 5. Full V1 verification.
    match verifier.verify(token).await {
        Ok(principal) => {
            request.extensions_mut().insert(CanaryAuthenticated);
            request.extensions_mut().insert(CanaryPrincipal(principal));
            CanaryDecision::Accept
        }
        Err(error) => CanaryDecision::Reject(error.into_response()),
    }
}

// ---------------------------------------------------------------------------
// Guards
// ---------------------------------------------------------------------------

/// Middleware for `GET /internal/v1/worklists/assigned-to-me`.
pub(crate) async fn canary_worklist_guard(
    State(state): State<AppState>,
    method: Method,
    headers: axum::http::HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Only intercept GET requests for the assigned-to-me endpoint.
    if method != Method::GET {
        return next.run(request).await;
    }

    let token = match extract_bearer_token(&headers) {
        Some(t) => t.to_owned(),
        None => return next.run(request).await,
    };

    match decide_canary_action(&state, &token, &mut request, false).await {
        CanaryDecision::Accept => next.run(request).await,
        CanaryDecision::Reject(response) => response,
        CanaryDecision::FallThrough => next.run(request).await,
    }
}

/// Middleware for write endpoints (create, transition).
pub(crate) async fn canary_write_guard(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let token = match extract_bearer_token(&headers) {
        Some(t) => t.to_owned(),
        None => return next.run(request).await,
    };

    match decide_canary_action(&state, &token, &mut request, true).await {
        CanaryDecision::Accept => next.run(request).await,
        CanaryDecision::Reject(response) => response,
        CanaryDecision::FallThrough => next.run(request).await,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract Bearer token from the Authorization header.
fn extract_bearer_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").filter(|t| !t.is_empty())
}

/// Lightweight claims peek for allow-list matching (no signature verification).
#[derive(Debug, Deserialize)]
struct PeekClaims {
    client_id: Option<String>,
    sub: Option<String>,
}

/// Decode the JWT payload (not verifying signature) to peek at `client_id` and `sub`.
fn peek_token_claims(token: &str) -> Result<Option<PeekClaims>, ()> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Ok(None);
    }
    let payload_bytes = base64_url_decode(parts[1])?;
    let claims: PeekClaims = serde_json::from_slice(&payload_bytes).map_err(|_| ())?;
    Ok(Some(claims))
}

/// Decode base64url with padding tolerance.
pub(crate) fn base64_url_decode(input: &str) -> Result<Vec<u8>, ()> {
    // Add padding if needed.
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_url_decode_valid_input() {
        let input = "eyJzdWIiOiIxMjMifQ";
        let result = base64_url_decode(input).unwrap();
        let decoded = std::str::from_utf8(&result).unwrap();
        assert_eq!(decoded, r#"{"sub":"123"}"#);
    }

    #[test]
    fn base64_url_decode_requires_padding() {
        let input = "eyJzdWIiOiIxMjMifQo";
        let result = base64_url_decode(input).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn base64_url_decode_invalid_input() {
        assert!(base64_url_decode("!!!").is_err());
    }

    #[test]
    fn extract_bearer_token_returns_some_when_valid() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer my-token"),
        );
        assert_eq!(extract_bearer_token(&headers), Some("my-token"));
    }

    #[test]
    fn extract_bearer_token_returns_none_when_missing() {
        let headers = axum::http::HeaderMap::new();
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn extract_bearer_token_returns_none_for_empty_token() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer "),
        );
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn extract_bearer_token_returns_none_for_wrong_scheme() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            axum::http::HeaderValue::from_static("Basic dGVzdA=="),
        );
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn peek_token_claims_returns_none_for_malformed_token() {
        // Token with wrong number of parts
        assert!(peek_token_claims("no-dots").unwrap().is_none());
        assert!(peek_token_claims("").unwrap().is_none());
        // Token with 3 parts but invalid base64 payload
        assert!(peek_token_claims("header.not-a-valid-base64.payload").is_err());
        // Token with properly formatted but non-JSON payload
        assert!(peek_token_claims("header.AAAA.payload").is_err());
    }
}
