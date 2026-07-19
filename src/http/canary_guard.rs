//! Auth V1 Canary route guard middleware.
//!
//! Two guards:
//!
//! - `canary_worklist_guard`: intercepts `GET /internal/v1/worklists/assigned-to-me`
//!   when the canary is active, performs V1 token verification, and injects the
//!   canary-authenticated principal into request extensions.
//! - `canary_write_guard`: intercepts write endpoints (create, transition) and
//!   rejects requests whose token matches the canary allow-list — even before
//!   legacy auth runs.
//!
//! Both guards are no-ops when the canary is disabled.

use axum::body::Body;
use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::http::{Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::auth::{CanaryAuthenticated, CanaryPrincipal};
use crate::http::error::ApiError;
use crate::http::AppState;

/// Lightweight claims peek for allow-list matching (no signature verification).
#[derive(Debug, Deserialize)]
struct PeekClaims {
    client_id: Option<String>,
    sub: Option<String>,
}

/// Middleware for `GET /internal/v1/worklists/assigned-to-me`.
///
/// When the canary is active:
/// 1. Extract the Bearer token.
/// 2. Peek at `client_id` and `sub` to check the allow-list.
/// 3. If the allow-list matches, perform full V1 verification.
/// 4. If V1 verification succeeds, inject `CanaryAuthenticated` + `CanaryPrincipal`
///    extensions and pass through.
/// 5. If V1 verification fails, return 401.
/// 6. If the allow-list does not match, pass through to legacy auth.
///
/// When the canary is disabled, this is a no-op.
pub(crate) async fn canary_worklist_guard(
    State(state): State<AppState>,
    method: Method,
    headers: axum::http::HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // No-op if canary is not active.
    if state.auth_v1_canary_verifier.is_none() || !state.auth_v1_canary_config.is_active() {
        return next.run(request).await;
    }

    // Only intercept GET requests for the assigned-to-me endpoint.
    if method != Method::GET {
        return next.run(request).await;
    }

    // Extract Bearer token.
    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => return next.run(request).await,
    };

    // Peek at claims to check allow-list.
    match peek_token_claims(token) {
        Ok(Some(peek)) => {
            let allowed_client = &state.auth_v1_canary_config.allowed_client_id;
            let allowed_sub = &state.auth_v1_canary_config.allowed_sub;

            let matches_allow_list = peek.client_id.as_deref() == Some(allowed_client.as_str())
                && peek.sub.as_deref() == Some(allowed_sub.as_str());

            if !matches_allow_list {
                // Not a canary token — let legacy auth handle it.
                return next.run(request).await;
            }

            // Canary allow-list matches — perform full V1 verification.
            match state
                .auth_v1_canary_verifier
                .as_ref()
                .unwrap()
                .verify(token)
                .await
            {
                Ok(principal) => {
                    // Inject extensions so the handler knows we used canary.
                    request.extensions_mut().insert(CanaryAuthenticated);
                    request.extensions_mut().insert(CanaryPrincipal(principal));
                    next.run(request).await
                }
                Err(error) => error.into_response(),
            }
        }
        Ok(None) => {
            // Token could not be peek-decoded — let legacy auth try.
            next.run(request).await
        }
        Err(_) => {
            // Malformed token — let legacy auth handle it.
            next.run(request).await
        }
    }
}

/// Middleware for write endpoints (create, transition).
///
/// When the canary is active, checks if the request token matches the canary
/// allow-list.  If yes, rejects with 403 — the canary profile is read-only.
/// If no, passes through to legacy auth.
///
/// This ensures route-level isolation: a canary token cannot call write
/// endpoints regardless of its scope claims.
pub(crate) async fn canary_write_guard(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    // No-op if canary is not active.
    if state.auth_v1_canary_verifier.is_none() || !state.auth_v1_canary_config.is_active() {
        return next.run(request).await;
    }

    // Extract Bearer token.
    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => return next.run(request).await,
    };

    // Peek at claims to check allow-list.
    match peek_token_claims(token) {
        Ok(Some(peek)) => {
            let allowed_client = &state.auth_v1_canary_config.allowed_client_id;
            let allowed_sub = &state.auth_v1_canary_config.allowed_sub;

            let matches_allow_list = peek.client_id.as_deref() == Some(allowed_client.as_str())
                && peek.sub.as_deref() == Some(allowed_sub.as_str());

            if matches_allow_list {
                // This is a canary token trying to write — reject.
                return ApiError::new(
                    StatusCode::FORBIDDEN,
                    "canary_read_only",
                    "the Auth V1 canary profile only allows read operations on assigned-to-me",
                )
                .into_response();
            }
        }
        Ok(None) | Err(_) => {
            // Can't peek or token doesn't have the claims — let legacy auth handle it.
        }
    }

    next.run(request).await
}

/// Extract Bearer token from the Authorization header.
fn extract_bearer_token<'a>(headers: &'a axum::http::HeaderMap) -> Option<&'a str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").filter(|t| !t.is_empty())
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
fn base64_url_decode(input: &str) -> Result<Vec<u8>, ()> {
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
