//! POST /internal/v1/workflow-instances/{workflowInstanceId}/execution-escalations
//!
//! WORKFLOW_EXECUTION_CONTROL_V1 system ingress (CTR-SWEC-004): the
//! execution runtime reports an exhausted attempt policy; the CURRENT visit
//! is escalated to HUMAN_REQUIRED through the existing assistance machinery.
//! Gate mirrors wake exactly: `workflow.execute` scope + direct token +
//! server-side `GLOBAL_SCHEDULER_READ` binding (fail-closed; denied attempts
//! get the same durable security audit). Idempotent: an open case on the
//! visit replays as escalated=false with the existing case id.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::application::workflow_instance::execution_escalation::{
    compute_escalation_request_hash, create_system_execution_escalation,
};
use crate::auth::AuthenticatedPrincipal;
use crate::http::dto::ExecutionEscalationResponse;
use crate::http::error::ApiError;
use crate::http::AppState;
use crate::store::postgres::workflow_instance_repository::execution_escalation::SystemEscalationCommand;

use super::{idempotency_key, require_scope, wake};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExecutionEscalationBody {
    node_visit_id: Uuid,
    reason: String,
    attempt_count: Option<i64>,
    last_attempt_id: Option<Uuid>,
    dispatch_intent_id: Option<Uuid>,
}

pub(crate) async fn create(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(workflow_instance_id): Path<Uuid>,
    headers: HeaderMap,
    payload: Result<Json<ExecutionEscalationBody>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ExecutionEscalationResponse>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    crate::http::handlers::definitions::require_direct_token(&principal)?;
    wake::require_global_scheduler_read(&state, &principal).await?;

    let Json(body) = payload.map_err(ApiError::from_json_rejection)?;
    let idempotency_key = idempotency_key(&headers)?;
    let request_hash = compute_escalation_request_hash(
        &workflow_instance_id,
        &body.node_visit_id,
        &body.reason,
        body.attempt_count,
        body.last_attempt_id,
        body.dispatch_intent_id,
    )
    .map_err(ApiError::from_assistance)?;

    let outcome = create_system_execution_escalation(
        &state.pool,
        SystemEscalationCommand {
            principal_id: principal.principal_id.into_uuid(),
            idempotency_key,
            request_hash,
            workflow_instance_id,
            node_visit_id: body.node_visit_id,
            reason: body.reason,
            attempt_count: body.attempt_count,
            last_attempt_id: body.last_attempt_id,
            dispatch_intent_id: body.dispatch_intent_id,
        },
    )
    .await
    .map_err(ApiError::from_assistance)?;

    Ok(Json(ExecutionEscalationResponse {
        escalated: outcome.escalated,
        assistance_case_id: outcome.assistance_case_id,
        workflow_state_version: outcome.workflow_state_version,
        event_sequence: outcome.event_sequence,
    }))
}
