//! Global (domain-independent) role binding provisioning handlers.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use super::ProvisioningAuth;
use crate::application::provisioning::{provision_global_role_binding, revoke_global_role_binding};
use crate::domain::ids::PrincipalId;
use crate::domain::provisioning::{
    ProvisionGlobalRoleBindingCommand, RevokeGlobalRoleBindingCommand,
    GLOBAL_WORKFLOW_COORDINATOR_ROLE, GLOBAL_WORKFLOW_READER_ROLE,
};
use crate::http::dto::{ProvisionGlobalRoleBindingRequest, RevokeGlobalRoleBindingRequest};
use crate::http::error::ApiError;
use crate::http::AppState;

/// PUT /internal/v1/admin/global-role-bindings/{principalId}
pub(crate) async fn create(
    State(state): State<AppState>,
    auth: ProvisioningAuth,
    headers: axum::http::HeaderMap,
    Path(principal_id): Path<Uuid>,
    payload: Result<Json<ProvisionGlobalRoleBindingRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let key = super::idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    if req.role_key != GLOBAL_WORKFLOW_COORDINATOR_ROLE
        && req.role_key != GLOBAL_WORKFLOW_READER_ROLE
        && req.role_key != crate::domain::provisioning::GLOBAL_SCHEDULER_READ_ROLE
    {
        return Err(ApiError::unprocessable(
            "role_key_invalid",
            "roleKey must be GLOBAL_WORKFLOW_COORDINATOR, GLOBAL_WORKFLOW_READER, or GLOBAL_SCHEDULER_READ",
        ));
    }

    let cmd = ProvisionGlobalRoleBindingCommand {
        principal_id: PrincipalId::from_uuid(principal_id),
        role_key: req.role_key,
        enabled: req.enabled,
    };

    match provision_global_role_binding(
        &state.pool,
        &cmd,
        &key,
        request_id,
        &auth.principal.principal_id,
    )
    .await
    {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}

/// DELETE /internal/v1/admin/global-role-bindings/{principalId}
pub(crate) async fn delete(
    State(state): State<AppState>,
    auth: ProvisioningAuth,
    headers: axum::http::HeaderMap,
    Path(principal_id): Path<Uuid>,
    payload: Result<Json<RevokeGlobalRoleBindingRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let key = super::idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    if req.role_key != GLOBAL_WORKFLOW_COORDINATOR_ROLE
        && req.role_key != GLOBAL_WORKFLOW_READER_ROLE
        && req.role_key != crate::domain::provisioning::GLOBAL_SCHEDULER_READ_ROLE
    {
        return Err(ApiError::unprocessable(
            "role_key_invalid",
            "roleKey must be GLOBAL_WORKFLOW_COORDINATOR, GLOBAL_WORKFLOW_READER, or GLOBAL_SCHEDULER_READ",
        ));
    }

    let cmd = RevokeGlobalRoleBindingCommand {
        principal_id: PrincipalId::from_uuid(principal_id),
        role_key: req.role_key,
    };

    match revoke_global_role_binding(
        &state.pool,
        &cmd,
        &key,
        request_id,
        &auth.principal.principal_id,
    )
    .await
    {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}
