//! Role binding provisioning handlers.

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
    Json(req): Json<ProvisionRoleBindingRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let key = super::idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    if req.role_key.is_empty() {
        return Err(ApiError::bad_request(
            "invalid_input",
            "role_key is required",
        ));
    }

    let cmd = ProvisionRoleBindingCommand {
        domain_id: DomainId::from_uuid(domain_id),
        principal_id: PrincipalId::from_uuid(principal_id),
        role_key: req.role_key,
        enabled: req.enabled,
    };

    match provision_role_binding(&state.pool, &cmd, &key, request_id, &auth.0.principal_id).await {
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
    Json(req): Json<RevokeRoleBindingRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let key = super::idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    if req.role_key.is_empty() {
        return Err(ApiError::bad_request(
            "invalid_input",
            "role_key is required",
        ));
    }

    let cmd = RevokeRoleBindingCommand {
        domain_id: DomainId::from_uuid(domain_id),
        principal_id: PrincipalId::from_uuid(principal_id),
        role_key: req.role_key,
    };

    match revoke_role_binding(&state.pool, &cmd, &key, request_id, &auth.0.principal_id).await {
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
    Json(req): Json<ReplaceOwnerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let key = super::idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    // Detect current owner (optional, best-effort)
    let current_owner: Option<PrincipalId> = sqlx::query_scalar::<_, Uuid>(
        "SELECT principal_id FROM domain_role_bindings
         WHERE domain_id = $1 AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE",
    )
    .bind(domain_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_e| ApiError::service_unavailable("service_unavailable", "storage error"))
    .map(|opt| opt.map(PrincipalId::from_uuid))?;

    let cmd = ReplaceOwnerCommand {
        domain_id: DomainId::from_uuid(domain_id),
        current_owner_id: current_owner,
        new_owner_id: PrincipalId::from_uuid(req.new_owner_principal_id),
    };

    match replace_owner(&state.pool, &cmd, &key, request_id, &auth.0.principal_id).await {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}
