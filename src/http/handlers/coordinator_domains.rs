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
use axum::extract::{Path, Query, State};
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

// ---------------------------------------------------------------------------
// SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1 — control-plane surfaces
// ---------------------------------------------------------------------------

use crate::application::coordinator_control_plane::{
    self, CoordinatorControlPlaneError,
};
use crate::http::dto::{BindingReconcileRequest, UpdateDomainRequest};

impl From<CoordinatorControlPlaneError> for ApiError {
    fn from(error: CoordinatorControlPlaneError) -> Self {
        let status = error
            .status_code()
            .try_into()
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        match error.detail() {
            Some(detail) => ApiError::new(status, error.label(), detail.to_string().leak()),
            None => ApiError::new(status, error.label(), "coordinator control-plane error"),
        }
    }
}

fn request_id_of(headers: &axum::http::HeaderMap) -> &str {
    headers.get("x-request-id").and_then(|v| v.to_str().ok()).unwrap_or("-")
}

/// GET /internal/v1/domains — keyset-paged governance metadata list.
pub(crate) async fn list_domains(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<DomainListQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;
    require_direct_token(&principal)?;
    require_global_coordinator(&state, &principal).await?;

    let limit = query.limit.unwrap_or(20).min(100);
    let before_created_at: Option<chrono::DateTime<chrono::Utc>> = query
        .before_created_at
        .as_deref()
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map_err(|_| {
                    ApiError::unprocessable(
                        "invalid_cursor",
                        "beforeCreatedAt must be an RFC 3339 timestamp",
                    )
                })
                .map(|dt| dt.with_timezone(&chrono::Utc))
        })
        .transpose()?;
    if before_created_at.is_some() != query.before_id.is_some() {
        return Err(ApiError::unprocessable(
            "invalid_cursor",
            "beforeCreatedAt and beforeId must be provided together",
        ));
    }

    let body = coordinator_control_plane::list_domains(
        &state.pool,
        principal.principal_id.into_uuid(),
        before_created_at,
        query.before_id,
        limit,
    )
    .await?;
    Ok(Json(body))
}

/// Query DTO for the domain list (keyset cursor).
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DomainListQuery {
    pub before_created_at: Option<String>,
    pub before_id: Option<Uuid>,
    pub limit: Option<u32>,
}

/// GET /internal/v1/domains/{domainId} — one domain's governance metadata.
pub(crate) async fn get_domain(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(domain_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;
    require_direct_token(&principal)?;
    require_global_coordinator(&state, &principal).await?;

    let body = coordinator_control_plane::get_domain(
        &state.pool,
        principal.principal_id.into_uuid(),
        domain_id,
    )
    .await?;
    Ok(Json(body))
}

/// PATCH /internal/v1/domains/{domainId} — displayName-only governance
/// update (receipt command type `domain.update`).
pub(crate) async fn update_domain(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: axum::http::HeaderMap,
    Path(domain_id): Path<Uuid>,
    payload: Result<Json<UpdateDomainRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    require_global_coordinator(&state, &principal).await?;

    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let key = idempotency_key(&headers)?;
    let request_id = request_id_of(&headers).to_string();

    let body = coordinator_control_plane::update_domain(
        &state.pool,
        principal.principal_id.into_uuid(),
        domain_id,
        &req.display_name,
        &key,
        &request_id,
    )
    .await?;
    Ok(Json(body))
}

/// GET /internal/v1/domains/{domainId}/owner — coordinator OR the domain's
/// own enabled owner; 404 `domain_owner_missing` when absent.
pub(crate) async fn get_domain_owner(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(domain_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;
    require_direct_token(&principal)?;

    let body = coordinator_control_plane::get_domain_owner(
        &state.pool,
        principal.principal_id.into_uuid(),
        domain_id,
    )
    .await?;
    Ok(Json(body))
}

/// POST /internal/v1/domains/{domainId}/binding-reconcile/plan — read-only
/// reconciliation judgment (coordinator-only).
pub(crate) async fn binding_reconcile_plan(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(domain_id): Path<Uuid>,
    payload: Result<Json<BindingReconcileRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;
    require_direct_token(&principal)?;
    require_global_coordinator(&state, &principal).await?;

    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let body = coordinator_control_plane::reconcile_plan(
        &state.pool,
        principal.principal_id.into_uuid(),
        domain_id,
        &req.role,
        req.from_principal_id,
        req.to_principal_id,
        &req.reason,
    )
    .await?;
    Ok(Json(body))
}

/// POST /internal/v1/domains/{domainId}/binding-reconcile/apply — atomic
/// binding migration behind exact-preimage re-assertion (receipt command
/// type `domain.binding_reconcile`).
pub(crate) async fn binding_reconcile_apply(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: axum::http::HeaderMap,
    Path(domain_id): Path<Uuid>,
    payload: Result<Json<BindingReconcileRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    require_global_coordinator(&state, &principal).await?;

    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let key = idempotency_key(&headers)?;
    let request_id = request_id_of(&headers).to_string();

    let body = coordinator_control_plane::reconcile_apply(
        &state.pool,
        principal.principal_id.into_uuid(),
        domain_id,
        &req.role,
        req.from_principal_id,
        req.to_principal_id,
        &req.reason,
        &key,
        &request_id,
    )
    .await?;
    Ok(Json(body))
}

