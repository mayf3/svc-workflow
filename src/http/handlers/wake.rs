//! POST /internal/v1/workflow-instances/{workflowInstanceId}/node-visits/{nodeVisitId}/wake
//!
//! Authorized early wake of a DISPATCH_INTENT (VISIT_ACTIVATION_V1).
//! Fail-closed server-side gate: `workflow.execute` scope + direct token +
//! an enabled `GLOBAL_SCHEDULER_READ` binding (verified server-side from
//! `global_role_bindings`; never from the token).

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::domain::workflow_instance::commands::WakeDispatchIntentCommand;
use crate::domain::ids::{NodeVisitId, WorkflowInstanceId};
use crate::http::dto::{WakeDispatchIntentRequest, WakeDispatchIntentResponse};
use crate::http::error::ApiError;
use crate::http::handlers::{idempotency_key, require_scope};
use crate::http::AppState;

const WAKE_COMMAND_SCHEMA_VERSION: &str = "v1";

/// Fail-closed GLOBAL_SCHEDULER_READ binding check (server-side).
async fn require_global_scheduler_read(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
) -> Result<(), ApiError> {
    let has_role = crate::store::postgres::provisioning_repository::check_global_scheduler_read(
        &state.pool,
        principal.principal_id.into_uuid(),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "scheduler role check failed");
        ApiError::service_unavailable("service_unavailable", "storage is unavailable")
    })?;
    if !has_role {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "scheduler_read_role_required",
            "caller must hold the GLOBAL_SCHEDULER_READ role",
        ));
    }
    Ok(())
}

pub(crate) async fn wake(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path((workflow_instance_id, node_visit_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    payload: Result<Json<WakeDispatchIntentRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<WakeDispatchIntentResponse>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    crate::http::handlers::definitions::require_direct_token(&principal)?;
    require_global_scheduler_read(&state, &principal).await?;

    let Json(req) = payload.map_err(ApiError::from_json_rejection)?;
    let idempotency_key = idempotency_key(&headers)?;

    let command = WakeDispatchIntentCommand {
        principal_id: principal.principal_id,
        idempotency_key,
        command_schema_version: WAKE_COMMAND_SCHEMA_VERSION.to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(workflow_instance_id),
        node_visit_id: NodeVisitId::from_uuid(node_visit_id),
        expected_workflow_state_version: req.expected_workflow_state_version,
        cause: req.cause,
    };

    let result = crate::application::workflow_instance::wake::wake_dispatch_intent(
        &state.pool,
        command,
    )
    .await
    .map_err(ApiError::from_wake)?;

    Ok(Json(WakeDispatchIntentResponse {
        wake_applied: result.wake_applied,
        reason: result.reason,
        workflow_instance_id: result.workflow_instance_id,
        node_visit_id: result.node_visit_id,
        workflow_state_version: result.workflow_state_version,
        event_sequence: result.event_sequence,
        next_eligible_at: result
            .next_eligible_at
            .map(|t| t.to_rfc3339()),
    }))
}
