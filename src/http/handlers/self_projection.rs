//! Self-scoped principal handlers.
//!
//! An Agent uses its own Direct Machine Token to project its verified
//! identity into the local `principals` table (`PUT /principals/me`) and
//! to discover its own domain memberships (`GET /principals/me/domains`).

use axum::extract::State;
use axum::Json;
use serde_json::Value;

use crate::application::domain_membership::{list_my_domains, self_project, DomainMembershipError};
use crate::auth::AuthenticatedPrincipal;
use crate::http::error::ApiError;
use crate::http::AppState;

use super::require_scope;

/// PUT /internal/v1/principals/me
///
/// Creates or confirms a local principal projection from the caller's
/// verified Direct Machine Token.
///
/// Requirements:
/// - `token_use=access` (Direct token, not OBO)
/// - `principal_type=agent` (enforced by auth verification)
/// - `scope=workflow.read`
///
/// The projected `principal_id` is `token.sub`.  No request body is
/// accepted — the identity comes exclusively from the verified JWT.
pub(crate) async fn self_project_handler(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;

    // Reject OBO tokens — only direct access tokens may self-project.
    if principal.auth_context.token_use != "access"
        || principal.auth_context.delegating_principal_id.is_some()
    {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "direct_token_required",
            "only direct access tokens may self-project",
        ));
    }

    let result = self_project(&state.pool, principal.principal_id.into_uuid())
        .await
        .map_err(ApiError::from_domain_membership)?;

    Ok(Json(serde_json::json!({
        "principalId": result.principal_id,
        "created": result.created,
    })))
}

/// GET /internal/v1/principals/me/domains
///
/// Returns the caller's own domain memberships — every domain where the
/// verified principal has an enabled `DOMAIN_OWNER` / `DOMAIN_MEMBER`
/// binding, joined with domain basic info:
/// `{domainId, domainKey, displayName, callerRole, bindingCreatedAt}`.
///
/// Caller-scoped: the subject comes exclusively from the verified JWT;
/// no path/query input identifies it.  Requires `workflow.read`.  Both
/// direct and OBO tokens are accepted (read-only, no member mutation).
pub(crate) async fn list_my_domains_handler(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;

    let items = list_my_domains(&state.pool, principal.principal_id.into_uuid())
        .await
        .map_err(ApiError::from_domain_membership)?;

    Ok(Json(serde_json::json!({ "items": items })))
}
