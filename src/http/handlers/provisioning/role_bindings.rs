//! Role binding provisioning handlers.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use super::ProvisioningAuth;
use crate::application::provisioning::{
    provision_role_binding, replace_owner, revoke_role_binding,
};
use crate::domain::ids::{DomainId, PrincipalId};
use crate::domain::provisioning::{
    ProvisionRoleBindingCommand, ReplaceOwnerCommand, RevokeRoleBindingCommand,
};
use crate::http::dto::{
    ProvisionRoleBindingRequest, ReplaceOwnerRequest, RevokeRoleBindingRequest,
};
use crate::http::error::ApiError;
use crate::http::AppState;

/// PUT /internal/v1/admin/domains/{domainId}/role-bindings/{principalId}
pub(crate) async fn create(
    State(state): State<AppState>,
    auth: ProvisioningAuth,
    headers: axum::http::HeaderMap,
    Path((domain_id, principal_id)): Path<(Uuid, Uuid)>,
    payload: Result<Json<ProvisionRoleBindingRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let key = super::idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    if !matches!(req.role_key.as_str(), "DOMAIN_OWNER" | "WORKFLOW_ADMIN") {
        return Err(ApiError::unprocessable(
            "role_key_invalid",
            "roleKey must be DOMAIN_OWNER or WORKFLOW_ADMIN",
        ));
    }

    let cmd = ProvisionRoleBindingCommand {
        domain_id: DomainId::from_uuid(domain_id),
        principal_id: PrincipalId::from_uuid(principal_id),
        role_key: req.role_key,
        enabled: req.enabled,
    };

    match provision_role_binding(
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

/// DELETE /internal/v1/admin/domains/{domainId}/role-bindings/{principalId}
pub(crate) async fn delete(
    State(state): State<AppState>,
    auth: ProvisioningAuth,
    headers: axum::http::HeaderMap,
    Path((domain_id, principal_id)): Path<(Uuid, Uuid)>,
    payload: Result<Json<RevokeRoleBindingRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let key = super::idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    if !matches!(req.role_key.as_str(), "DOMAIN_OWNER" | "WORKFLOW_ADMIN") {
        return Err(ApiError::unprocessable(
            "role_key_invalid",
            "roleKey must be DOMAIN_OWNER or WORKFLOW_ADMIN",
        ));
    }

    let cmd = RevokeRoleBindingCommand {
        domain_id: DomainId::from_uuid(domain_id),
        principal_id: PrincipalId::from_uuid(principal_id),
        role_key: req.role_key,
    };

    match revoke_role_binding(
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

/// PUT /internal/v1/admin/domains/{domainId}/owner
pub(crate) async fn replace_domain_owner(
    State(state): State<AppState>,
    auth: ProvisioningAuth,
    headers: axum::http::HeaderMap,
    Path(domain_id): Path<Uuid>,
    payload: Result<Json<ReplaceOwnerRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let key = super::idempotency_key(&headers)?;
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
        &auth.principal.principal_id,
    )
    .await
    {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}
