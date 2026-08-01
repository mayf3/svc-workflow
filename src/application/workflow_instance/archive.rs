//! Application service for archiving terminal workflow instances.
//!
//! Only DOMAIN_OWNER may archive instances in their domain.
//! Archiving adds governance metadata without changing the business state.

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::workflow_instance::commands::ArchiveWorkflowInstanceCommand;
use crate::domain::workflow_instance::errors::ArchiveWorkflowInstanceError;
use crate::store::postgres::workflow_instance_repository::archive_transaction;

/// Outcome of an archive attempt.
#[derive(Debug, Clone)]
pub struct ArchiveWorkflowInstanceResult {
    pub workflow_instance_id: Uuid,
    pub workflow_state_version: i32,
    pub event_sequence: i32,
    pub replayed: bool,
}

/// Archive a terminal workflow instance.
///
/// The caller must be a DOMAIN_OWNER of the instance's domain.
/// The instance must be in a terminal state (cancelled OR node_type == TERMINAL).
pub async fn archive_workflow_instance(
    pool: &PgPool,
    cmd: ArchiveWorkflowInstanceCommand,
    request_hash: &str,
) -> Result<ArchiveWorkflowInstanceResult, ArchiveWorkflowInstanceError> {
    let result = archive_transaction::archive_workflow_instance_atomically(pool, cmd, request_hash).await?;

    Ok(ArchiveWorkflowInstanceResult {
        workflow_instance_id: result.workflow_instance_id,
        workflow_state_version: result.workflow_state_version,
        event_sequence: result.event_sequence,
        replayed: result.replayed,
    })
}
