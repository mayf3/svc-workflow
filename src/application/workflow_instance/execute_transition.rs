//! ExecuteWorkflowTransition application service.
//!
//! Orchestrates the full workflow transition workflow:
//! 1. Pre-validate principal existence and enabled status
//! 2. Compute request hash for idempotency
//! 3. Delegate to the atomic transition transaction
//! 4. Map result to the public response type

use sqlx::PgPool;

use crate::domain::workflow_instance::commands::ExecuteWorkflowTransitionCommand;
use crate::domain::workflow_instance::errors::ExecuteWorkflowTransitionError;
use crate::store::postgres::workflow_instance_repository::transition_transaction;

use super::idempotency::compute_transition_request_hash;

/// Result of a successful ExecuteWorkflowTransition command.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecuteWorkflowTransitionResult {
    pub workflow_instance_id: uuid::Uuid,
    pub workflow_state_version: i32,
    pub current_context_revision_id: uuid::Uuid,
    pub source_node_visit_id: uuid::Uuid,
    pub current_node_visit_id: uuid::Uuid,
    pub submission_id: Option<uuid::Uuid>,
    pub event_sequence: i32,
}

impl From<transition_transaction::TransitionResult> for ExecuteWorkflowTransitionResult {
    fn from(r: transition_transaction::TransitionResult) -> Self {
        Self {
            workflow_instance_id: r.workflow_instance_id,
            workflow_state_version: r.workflow_state_version,
            current_context_revision_id: r.current_context_revision_id,
            source_node_visit_id: r.source_node_visit_id,
            current_node_visit_id: r.current_node_visit_id,
            submission_id: r.submission_id,
            event_sequence: r.event_sequence,
        }
    }
}

/// Execute a workflow transition atomically.
///
/// # Errors
///
/// Returns `ExecuteWorkflowTransitionError` for all validation, authorization,
/// version conflict, and infrastructure failures.
pub async fn execute_workflow_transition(
    pool: &PgPool,
    command: ExecuteWorkflowTransitionCommand,
) -> Result<ExecuteWorkflowTransitionResult, ExecuteWorkflowTransitionError> {
    // 1. Pre-validate principal existence and enabled status
    let principal_uuid = command.principal_id.into_uuid();
    pre_validate_principal(pool, principal_uuid).await?;

    // 2. Validate submission payload size (pre-transaction fast-fail)
    if let Some(ref payload) = command.submission_payload {
        let payload_bytes = serde_json::to_vec(payload)
            .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;
        if payload_bytes.len() > 1024 * 1024 {
            return Err(ExecuteWorkflowTransitionError::SizeLimitExceeded(
                "submission_payload exceeds 1 MiB".to_string(),
            ));
        }
    }

    // 3. Compute request hash for idempotency
    let request_hash = compute_transition_request_hash(
        &command.command_schema_version,
        &command.idempotency_key,
        &command.principal_id,
        &command.workflow_instance_id,
        command.expected_workflow_state_version,
        &command.transition_definition_id,
        &command.submission_payload,
    )?;

    // 4. Execute atomic transition
    let outcome = transition_transaction::execute_workflow_transition_atomically(
        pool,
        command,
        &request_hash,
    )
    .await?;

    // 5. Map outcome to public result
    match outcome {
        transition_transaction::TransitionOutcome::Executed(result) => Ok(result.into()),
        transition_transaction::TransitionOutcome::Replayed(result) => Ok(result.into()),
        transition_transaction::TransitionOutcome::ReplayedFailure(status, body) => {
            let error_code = body["error"].as_str().unwrap_or("unknown");
            Err(match (status, error_code) {
                (404, "instance_not_found") => ExecuteWorkflowTransitionError::InstanceNotFound,
                (403, "principal_disabled") => ExecuteWorkflowTransitionError::PrincipalDisabled,
                (404, "current_visit_not_found") => {
                    ExecuteWorkflowTransitionError::CurrentVisitNotFound
                }
                (403, "principal_not_assignee") => {
                    ExecuteWorkflowTransitionError::PrincipalNotAssignee
                }
                (409, "source_node_terminal") => ExecuteWorkflowTransitionError::SourceNodeTerminal,
                (409, "definition_version_revoked") => {
                    ExecuteWorkflowTransitionError::DefinitionVersionRevoked
                }
                (500, "definition_version_draft") => {
                    ExecuteWorkflowTransitionError::DefinitionVersionDraft
                }
                (409, "workflow_state_version_conflict") => {
                    let expected = body["expected"].as_i64().unwrap_or(0) as i32;
                    let actual = body["actual"].as_i64().unwrap_or(0) as i32;
                    ExecuteWorkflowTransitionError::WorkflowStateVersionConflict {
                        expected,
                        actual,
                    }
                }
                (409, "transition_not_applicable") => {
                    ExecuteWorkflowTransitionError::TransitionNotApplicable(
                        body["detail"].as_str().unwrap_or("unknown").to_string(),
                    )
                }
                (422, "submission_required") => ExecuteWorkflowTransitionError::SubmissionRequired,
                (422, "submission_validation_failed") => {
                    ExecuteWorkflowTransitionError::SubmissionValidationFailed(
                        body["detail"]
                            .as_str()
                            .unwrap_or("validation failed")
                            .to_string(),
                    )
                }
                (413, "size_limit_exceeded") => ExecuteWorkflowTransitionError::SizeLimitExceeded(
                    body["error"]
                        .as_str()
                        .unwrap_or("size limit exceeded")
                        .to_string(),
                ),
                (422, "invalid_return_references") => {
                    ExecuteWorkflowTransitionError::InvalidReturnReferences(
                        body["error"]
                            .as_str()
                            .unwrap_or("invalid references")
                            .to_string(),
                    )
                }
                (422, "assignee_resolution_failed") => {
                    ExecuteWorkflowTransitionError::AssigneeResolutionFailed(
                        body["error"]
                            .as_str()
                            .unwrap_or("resolution failed")
                            .to_string(),
                    )
                }
                _ => ExecuteWorkflowTransitionError::StorageError(format!(
                    "replayed deterministic failure: status={}, error={}",
                    status, error_code
                )),
            })
        }
    }
}

/// Fast-fail check that the principal exists and is enabled,
/// before entering the main transaction.
async fn pre_validate_principal(
    pool: &PgPool,
    principal_uuid: uuid::Uuid,
) -> Result<(), ExecuteWorkflowTransitionError> {
    let row: Option<(bool,)> =
        sqlx::query_as("SELECT enabled FROM principals WHERE principal_id = $1")
            .bind(principal_uuid)
            .fetch_optional(pool)
            .await
            .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    match row {
        None => Err(ExecuteWorkflowTransitionError::PrincipalNotFound),
        Some((enabled,)) if !enabled => Err(ExecuteWorkflowTransitionError::PrincipalDisabled),
        _ => Ok(()),
    }
}
