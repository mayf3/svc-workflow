//! WORKFLOW_EXECUTION_CONTROL_V1 (CTR-SWEC-004) — the system
//! execution-escalation application entry: hash + delegate to the store
//! command. Authorization lives in the HTTP adapter (workflow.execute +
//! direct token + GLOBAL_SCHEDULER_READ, mirroring wake).

use sqlx::PgPool;

use crate::domain::workflow_instance::assistance::AssistanceError;
use crate::store::postgres::workflow_instance_repository::execution_escalation::{
    system_execution_escalation, SystemEscalationCommand, SystemEscalationOutcome,
};

/// Compute the canonical request hash over the idempotency-independent body.
pub fn compute_escalation_request_hash(
    workflow_instance_id: &uuid::Uuid,
    node_visit_id: &uuid::Uuid,
    reason: &str,
    attempt_count: Option<i64>,
    last_attempt_id: Option<uuid::Uuid>,
    dispatch_intent_id: Option<uuid::Uuid>,
) -> Result<String, AssistanceError> {
    let envelope = serde_json::json!({
        "commandType": "SYSTEM_EXECUTION_ESCALATION",
        "workflowInstanceId": workflow_instance_id,
        "nodeVisitId": node_visit_id,
        "reason": reason,
        "attemptCount": attempt_count,
        "lastAttemptId": last_attempt_id,
        "dispatchIntentId": dispatch_intent_id,
    });
    jcs_canonicalize::sha256_jcs_hex(&envelope).map_err(|e| {
        AssistanceError::InternalConsistency(format!("request hash computation failed: {e}"))
    })
}

pub async fn create_system_execution_escalation(
    pool: &PgPool,
    command: SystemEscalationCommand,
) -> Result<SystemEscalationOutcome, AssistanceError> {
    system_execution_escalation(pool, command).await
}
