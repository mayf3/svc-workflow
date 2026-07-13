//! CreateWorkflowInstance application service.
//!
//! Orchestrates the full creation workflow:
//! 1. Pre-validate principal existence and enabled status
//! 2. Compute request hash for idempotency
//! 3. Delegate to the atomic creation transaction
//! 4. Map result to the public response type

use sqlx::PgPool;

use crate::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use crate::domain::workflow_instance::errors::CreateWorkflowInstanceError;
use crate::store::postgres::workflow_instance_repository::create_transaction;

use super::idempotency::compute_request_hash;

/// Result of a successful CreateWorkflowInstance command.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateWorkflowInstanceResult {
    pub workflow_instance_id: uuid::Uuid,
    pub workflow_state_version: i32,
    pub current_context_revision_id: uuid::Uuid,
    pub current_node_visit_id: uuid::Uuid,
    pub event_sequence: i32,
}

impl From<create_transaction::CreateResult> for CreateWorkflowInstanceResult {
    fn from(r: create_transaction::CreateResult) -> Self {
        Self {
            workflow_instance_id: r.workflow_instance_id,
            workflow_state_version: r.workflow_state_version,
            current_context_revision_id: r.current_context_revision_id,
            current_node_visit_id: r.current_node_visit_id,
            event_sequence: r.event_sequence,
        }
    }
}

/// Create a new workflow instance atomically.
///
/// # Errors
///
/// Returns `CreateWorkflowInstanceError` for all validation, authorization,
/// and infrastructure failures.
pub async fn create_workflow_instance(
    pool: &PgPool,
    command: CreateWorkflowInstanceCommand,
) -> Result<CreateWorkflowInstanceResult, CreateWorkflowInstanceError> {
    // 1. Pre-validate principal existence and enabled status
    // (fast-fail before entering the transaction)
    let principal_uuid = command.principal_id.into_uuid();
    pre_validate_principal(pool, principal_uuid).await?;

    // 2. Compute request hash for idempotency
    let request_hash = compute_request_hash(
        &command.command_schema_version,
        &command.idempotency_key,
        &command.principal_id,
        &command.domain_id,
        &command.definition_version_id,
        &command.context_payload,
        &command.metadata,
        &command.external_reference,
        &command.external_url,
    )?;

    // 3. Pre-validate size limits (fast-fail before entering the transaction)
    validate_context_size(&command)?;

    // 4. Execute atomic creation
    let outcome =
        create_transaction::create_workflow_instance_atomically(pool, command, &request_hash)
            .await?;

    // 4. Map outcome to public result
    match outcome {
        create_transaction::CreateOutcome::Created(result) => Ok(result.into()),
        create_transaction::CreateOutcome::Replayed(result) => Ok(result.into()),
        create_transaction::CreateOutcome::ReplayedFailure(status, body) => {
            // Map deterministic failure status back to domain error
            let error_code = body["error"].as_str().unwrap_or("unknown");
            Err(match (status, error_code) {
                (404, "domain_not_found") => CreateWorkflowInstanceError::DomainNotFound,
                (403, "domain_disabled") => CreateWorkflowInstanceError::DomainDisabled,
                (404, "principal_not_found") => CreateWorkflowInstanceError::PrincipalNotFound,
                (403, "principal_disabled") => CreateWorkflowInstanceError::PrincipalDisabled,
                (403, "domain_membership_required") => {
                    CreateWorkflowInstanceError::DomainMembershipRequired
                }
                (403, "cross_domain_violation") => {
                    CreateWorkflowInstanceError::CrossDomainViolation
                }
                (404, "definition_version_not_found") => {
                    CreateWorkflowInstanceError::DefinitionVersionNotFound
                }
                (409, "version_not_published") => CreateWorkflowInstanceError::VersionNotPublished,
                (422, "context_validation_failed") => {
                    CreateWorkflowInstanceError::ContextValidationFailed(
                        body["error"].as_str().unwrap_or("unknown").to_string(),
                    )
                }
                (413, "size_limit_exceeded") => CreateWorkflowInstanceError::SizeLimitExceeded(
                    body["error"].as_str().unwrap_or("unknown").to_string(),
                ),
                (422, "assignee_resolution_failed") => {
                    CreateWorkflowInstanceError::AssigneeResolutionFailed(
                        body["error"].as_str().unwrap_or("unknown").to_string(),
                    )
                }
                _ => CreateWorkflowInstanceError::StorageError(format!(
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
) -> Result<(), CreateWorkflowInstanceError> {
    let row: Option<(bool,)> =
        sqlx::query_as("SELECT enabled FROM principals WHERE principal_id = $1")
            .bind(principal_uuid)
            .fetch_optional(pool)
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    match row {
        None => Err(CreateWorkflowInstanceError::PrincipalNotFound),
        Some((enabled,)) if !enabled => Err(CreateWorkflowInstanceError::PrincipalDisabled),
        _ => Ok(()),
    }
}

/// Validate context and metadata size limits at the service layer (pre-transaction).
fn validate_context_size(
    cmd: &CreateWorkflowInstanceCommand,
) -> Result<(), CreateWorkflowInstanceError> {
    let context_bytes = serde_json::to_vec(&cmd.context_payload)
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
    if context_bytes.len() > 1024 * 1024 {
        return Err(CreateWorkflowInstanceError::SizeLimitExceeded(
            "context_payload exceeds 1 MiB".to_string(),
        ));
    }

    let metadata_bytes = serde_json::to_vec(&cmd.metadata)
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
    if metadata_bytes.len() > 64 * 1024 {
        return Err(CreateWorkflowInstanceError::SizeLimitExceeded(
            "metadata exceeds 64 KiB".to_string(),
        ));
    }

    Ok(())
}
