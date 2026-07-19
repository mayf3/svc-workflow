//! Auth V1 write gate middleware.
//!
//! ## Design
//!
//! This middleware checks the `AUTH_V1_CANARY_WRITE_ENABLED` flag on write
//! endpoints (create / transition).  When the flag is false, the endpoint
//! returns 403 before any token verification.
//!
//! Token verification is handled by the `AuthenticatedPrincipal` extractor.
//! This middleware does NOT inspect or verify tokens — it only enforces the
//! write gate.
//!
//! Profile guessing (`looks_like_auth_v1_token`) and legacy fallthrough
//! (`FallThrough`) have been removed — Auth V1 is the only path.

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::http::error::ApiError;
use crate::http::AppState;

/// Middleware for write endpoints (create, transition).
///
/// If `AUTH_V1_CANARY_WRITE_ENABLED` is false, all write requests are
/// rejected with 403 before reaching the handler.
pub(crate) async fn canary_write_guard(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if !state.auth_v1_canary_config.write_active() {
        return ApiError::new(
            StatusCode::FORBIDDEN,
            "canary_read_only",
            "Auth V1 write operations are not enabled",
        )
        .into_response();
    }
    next.run(request).await
}
