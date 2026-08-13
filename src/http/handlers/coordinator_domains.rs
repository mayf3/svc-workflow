//! GLOBAL_WORKFLOW_COORDINATOR domain management handlers (agent-facing).
//!
//! Non-admin endpoints that let a verified `GLOBAL_WORKFLOW_COORDINATOR`
//! create domains and set domain owners through the regular Broker path.
//!
//! Authorization model (frozen):
//!   - Auth layer keeps coarse scopes only: `workflow.execute`.
//!   - The business role (`GLOBAL_WORKFLOW_COORDINATOR`) is verified
//!     server-side from `global_role_bindings` — never carried in the JWT.
//!   - The existing `workflow.admin` provisioning endpoints are unchanged;
//!     these endpoints are strictly narrower (create domain / set owner only).
//!
//! Both handlers reuse the same idempotent receipt machinery as the admin
//! provisioning endpoints (`workflow_command_receipts`, Idempotency-Key),
//! so a duplicate request cannot create a second domain or re-fire the
//! owner swap.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use super::definitions::require_direct_token;
use super::{idempotency_key, require_scope};
use crate::application::provisioning::{provision_domain, replace_owner};
use crate::auth::AuthenticatedPrincipal;
use crate::domain::ids::{DomainId, PrincipalId};
use crate::domain::provisioning::{ProvisionDomainCommand, ReplaceOwnerCommand};
use crate::http::dto::{ProvisionDomainRequest, ReplaceOwnerRequest};
use crate::http::error::ApiError;
use crate::http::AppState;
use crate::store::postgres::provisioning_repository;

/// Verify the caller holds an enabled `GLOBAL_WORKFLOW_COORDINATOR` binding.
async fn require_global_coordinator(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
) -> Result<(), ApiError> {
    let is_coordinator =
        provisioning_repository::check_global_coordinator(&state.pool, principal.principal_id.into_uuid())
            .await
            .map_err(ApiError::from_provisioning)?;
    if !is_coordinator {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "global_coordinator_required",
            "caller must hold the GLOBAL_WORKFLOW_COORDINATOR role",
        ));
    }
    Ok(())
}

/// POST /internal/v1/domains
///
/// Create a domain. Same contract as the admin provisioning endpoint
/// (`POST /internal/v1/admin/domains`) but gated by
/// `workflow.execute` scope + `GLOBAL_WORKFLOW_COORDINATOR` instead of
/// `workflow.admin` + allow-list.
pub(crate) async fn create_domain(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: axum::http::HeaderMap,
    payload: Result<Json<ProvisionDomainRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    require_global_coordinator(&state, &principal).await?;

    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let key = idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    if req.domain_key.is_empty()
        || req.domain_key.len() > 128
        || req.domain_key.chars().any(char::is_whitespace)
        || req.domain_key.chars().any(char::is_control)
        || req.display_name.as_ref().is_some_and(|name| {
            name.is_empty()
                || name != name.trim()
                || name.len() > 256
                || name.chars().any(char::is_control)
        })
    {
        return Err(ApiError::unprocessable(
            "invalid_input",
            "domainKey or displayName is invalid",
        ));
    }

    let cmd = ProvisionDomainCommand {
        domain_id: DomainId::from_uuid(req.domain_id),
        domain_key: req.domain_key,
        display_name: req.display_name,
        enabled: req.enabled,
    };

    match provision_domain(
        &state.pool,
        &cmd,
        &key,
        request_id,
        &principal.principal_id,
    )
    .await
    {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}

/// PUT /internal/v1/domains/{domainId}/owner
///
/// Atomically replace the domain owner. Same contract as the admin
/// provisioning endpoint (`PUT /internal/v1/admin/domains/{domainId}/owner`)
/// but gated by `workflow.execute` scope + `GLOBAL_WORKFLOW_COORDINATOR`.
pub(crate) async fn set_domain_owner(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: axum::http::HeaderMap,
    Path(domain_id): Path<Uuid>,
    payload: Result<Json<ReplaceOwnerRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    require_global_coordinator(&state, &principal).await?;

    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let key = idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let cmd = ReplaceOwnerCommand {
        domain_id: DomainId::from_uuid(domain_id),
        new_owner_id: PrincipalId::from_uuid(req.new_owner_principal_id),
    };

    match replace_owner(
        &state.pool,
        &cmd,
        &key,
        request_id,
        &principal.principal_id,
    )
    .await
    {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}
