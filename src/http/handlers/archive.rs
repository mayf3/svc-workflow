//! Archive workflow instance endpoint.
//!
//! POST /internal/v1/workflow-instances/{workflowInstanceId}/archive

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::application::workflow_instance::archive::archive_workflow_instance;
use crate::application::workflow_instance::idempotency::compute_archive_request_hash;
use crate::auth::AuthenticatedPrincipal;
use crate::domain::ids::WorkflowInstanceId;
use crate::domain::workflow_instance::commands::ArchiveWorkflowInstanceCommand;
use crate::http::error::ApiError;
use crate::http::AppState;

use super::{idempotency_key, path_uuid, require_scope};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ArchiveRequest {
    pub reason: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchiveResponse {
    pub workflow_instance_id: uuid::Uuid,
    pub workflow_state_version: i32,
    pub event_sequence: i32,
    pub replayed: bool,
}

pub(crate) async fn archive(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: HeaderMap,
    Path(workflow_instance_id): Path<String>,
    payload: Json<ArchiveRequest>,
) -> Result<impl IntoResponse, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    let key = idempotency_key(&headers)?;
    let workflow_instance_id = path_uuid(&workflow_instance_id)?;
    let instance_id = WorkflowInstanceId::from_uuid(workflow_instance_id);

    // Canonical 64-hex request hash (JCS envelope per the frozen idempotency
    // contract); workflow_command_receipts.request_hash requires ^[0-9a-f]{64}$.
    let request_hash =
        compute_archive_request_hash("v1", &principal.principal_id, &instance_id, &payload.reason)
            .map_err(ApiError::from_archive)?;

    let command = ArchiveWorkflowInstanceCommand {
        principal_id: principal.principal_id,
        idempotency_key: key,
        command_schema_version: "v1".to_string(),
        workflow_instance_id: instance_id,
        // 0 is the adapter sentinel for "no client-side optimistic version";
        // all state checks are performed atomically under the row lock.
        expected_workflow_state_version: 0,
        reason: payload.reason.clone(),
    };

    let result = archive_workflow_instance(&state.pool, command, &request_hash)
        .await
        .map_err(ApiError::from_archive)?;

    let response = ArchiveResponse {
        workflow_instance_id: result.workflow_instance_id,
        workflow_state_version: result.workflow_state_version,
        event_sequence: result.event_sequence,
        replayed: result.replayed,
    };

    Ok((StatusCode::OK, Json(response)))
}
