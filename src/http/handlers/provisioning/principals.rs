//! Principal provisioning handlers.

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use super::ProvisioningAuth;
use crate::application::provisioning::{get_principal, provision_principal};
use crate::domain::ids::PrincipalId;
use crate::domain::provisioning::{ProvisionPrincipalCommand, ProvisioningError};
use crate::http::dto::ProvisionPrincipalRequest;
use crate::http::error::ApiError;
use crate::http::AppState;

/// POST /internal/v1/admin/principals
pub(crate) async fn create(
    State(state): State<AppState>,
    auth: ProvisioningAuth,
    headers: axum::http::HeaderMap,
    Json(req): Json<ProvisionPrincipalRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let key = super::idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    if req.principal_type != "human" && req.principal_type != "agent" {
        return Err(ApiError::from_provisioning(
            ProvisioningError::PrincipalTypeInvalid,
        ));
    }
    if req.source.is_empty() {
        return Err(ApiError::bad_request("invalid_input", "source is required"));
    }

    let cmd = ProvisionPrincipalCommand {
        principal_id: PrincipalId::from_uuid(req.principal_id),
        principal_type: req.principal_type,
        enabled: req.enabled,
        source: req.source,
        source_revision: req.source_revision,
    };

    match provision_principal(&state.pool, &cmd, &key, request_id, &auth.0.principal_id).await {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}

/// GET /internal/v1/admin/principals/{principalId}
pub(crate) async fn get(
    State(state): State<AppState>,
    _auth: ProvisioningAuth,
    Path(principal_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match get_principal(&state.pool, PrincipalId::from_uuid(principal_id)).await {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}
