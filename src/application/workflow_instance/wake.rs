//! WakeDispatchIntent application service (VISIT_ACTIVATION_V1).
//!
//! Orchestrates the authorized early wake of a DISPATCH_INTENT:
//! 1. compute the request hash for idempotency;
//! 2. delegate to the atomic wake transaction;
//! 3. map the outcome to the public result.

use sqlx::PgPool;

use crate::domain::workflow_instance::commands::WakeDispatchIntentCommand;
use crate::domain::workflow_instance::errors::WakeDispatchIntentError;
use crate::store::postgres::workflow_instance_repository::wake_transaction::{
    self, WakeAppliedResult, WakeNoOpResult, WakeOutcome,
};

/// Public result of a wake attempt: either applied (state advanced) or a
/// durable no-op with a machine-readable reason.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WakeDispatchIntentResult {
    pub wake_applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub workflow_instance_id: uuid::Uuid,
    pub node_visit_id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_state_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_sequence: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_eligible_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing)]
    pub replayed: bool,
}

/// Execute the wake command.
pub async fn wake_dispatch_intent(
    pool: &PgPool,
    command: WakeDispatchIntentCommand,
) -> Result<WakeDispatchIntentResult, WakeDispatchIntentError> {
    let request_hash = crate::application::workflow_instance::idempotency::compute_wake_request_hash(
        &command.command_schema_version,
        &command.idempotency_key,
        &command.principal_id,
        &command.workflow_instance_id,
        &command.node_visit_id,
        command.expected_workflow_state_version,
        &command.cause,
    )
    .map_err(WakeDispatchIntentError::StorageError)?;

    let outcome =
        wake_transaction::wake_dispatch_intent_atomically(pool, command, &request_hash).await?;

    Ok(match outcome {
        WakeOutcome::Applied(WakeAppliedResult {
            workflow_instance_id,
            node_visit_id,
            workflow_state_version,
            event_sequence,
            next_eligible_at,
            replayed,
        }) => WakeDispatchIntentResult {
            wake_applied: true,
            reason: None,
            workflow_instance_id,
            node_visit_id,
            workflow_state_version: Some(workflow_state_version),
            event_sequence: Some(event_sequence),
            next_eligible_at: Some(next_eligible_at),
            replayed,
        },
        WakeOutcome::NoOp(WakeNoOpResult {
            workflow_instance_id,
            node_visit_id,
            reason,
            replayed,
        }) => WakeDispatchIntentResult {
            wake_applied: false,
            reason: Some(reason),
            workflow_instance_id,
            node_visit_id,
            workflow_state_version: None,
            event_sequence: None,
            next_eligible_at: None,
            replayed,
        },
    })
}
