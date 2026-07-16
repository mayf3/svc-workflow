//! Domain provisioning handlers.

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use super::ProvisioningAuth;
use crate::application::provisioning::{get_domain, provision_domain};
use crate::domain::ids::DomainId;
use crate::domain::provisioning::ProvisionDomainCommand;
use crate::http::dto::ProvisionDomainRequest;
use crate::http::error::ApiError;
use crate::http::AppState;

/// POST /internal/v1/admin/domains
pub(crate) async fn create(
    State(state): State<AppState>,
    auth: ProvisioningAuth,
    headers: axum::http::HeaderMap,
    Json(req): Json<ProvisionDomainRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let key = super::idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    if req.domain_key.is_empty() {
        return Err(ApiError::bad_request(
            "invalid_input",
            "domain_key is required",
        ));
    }

    let cmd = ProvisionDomainCommand {
        domain_id: DomainId::from_uuid(req.domain_id),
        domain_key: req.domain_key,
        display_name: req.display_name,
        enabled: req.enabled,
    };

    match provision_domain(&state.pool, &cmd, &key, request_id, &auth.0.principal_id).await {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}

/// GET /internal/v1/admin/domains/{domainId}
pub(crate) async fn get(
    State(state): State<AppState>,
    _auth: ProvisioningAuth,
    Path(domain_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match get_domain(&state.pool, DomainId::from_uuid(domain_id)).await {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}
