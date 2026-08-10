//! Application service for cancelling active workflow instances.
//!
//! Only DOMAIN_OWNER may cancel instances in their domain.
//! Cancellation closes the current work item and prevents further transitions.

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::workflow_instance::commands::CancelWorkflowInstanceCommand;
use crate::domain::workflow_instance::errors::CancelWorkflowInstanceError;
use crate::store::postgres::workflow_instance_repository::cancel_transaction;

/// Outcome of a cancel attempt.
#[derive(Debug, Clone)]
pub struct CancelWorkflowInstanceResult {
    pub workflow_instance_id: Uuid,
    pub workflow_state_version: i32,
    pub event_sequence: i32,
    pub replayed: bool,
}

/// Cancel an active workflow instance.
///
/// The caller must be a DOMAIN_OWNER of the instance's domain.
pub async fn cancel_workflow_instance(
    pool: &PgPool,
    cmd: CancelWorkflowInstanceCommand,
    request_hash: &str,
) -> Result<CancelWorkflowInstanceResult, CancelWorkflowInstanceError> {
    let result =
        cancel_transaction::cancel_workflow_instance_atomically(pool, cmd, request_hash).await?;

    Ok(CancelWorkflowInstanceResult {
        workflow_instance_id: result.workflow_instance_id,
        workflow_state_version: result.workflow_state_version,
        event_sequence: result.event_sequence,
        replayed: result.replayed,
    })
}
