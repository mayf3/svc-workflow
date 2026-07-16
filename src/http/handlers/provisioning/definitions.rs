//! Definition version query handler.

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use super::ProvisioningAuth;
use crate::application::provisioning::get_definition_version;
use crate::domain::ids::DefinitionVersionId;
use crate::http::error::ApiError;
use crate::http::AppState;

/// GET /internal/v1/admin/definition-versions/{definitionVersionId}
pub(crate) async fn get(
    State(state): State<AppState>,
    _auth: ProvisioningAuth,
    Path(version_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match get_definition_version(&state.pool, DefinitionVersionId::from_uuid(version_id)).await {
        Ok(body) => Ok(Json(body)),
        Err(e) => Err(ApiError::from_provisioning(e)),
    }
}
