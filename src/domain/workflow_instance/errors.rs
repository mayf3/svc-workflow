//! Stable error types for the Workflow Instance domain.

use std::fmt;

/// Top-level error type for workflow instance creation operations.
#[derive(Debug, Clone)]
pub enum CreateWorkflowInstanceError {
    /// Principal does not exist.
    PrincipalNotFound,
    /// Principal exists but is disabled.
    PrincipalDisabled,
    /// Domain does not exist.
    DomainNotFound,
    /// Domain exists but is disabled.
    DomainDisabled,
    /// Caller has no membership binding for the target domain.
    DomainMembershipRequired,
    /// Workflow definition version not found.
    DefinitionVersionNotFound,
    /// The version is not in PUBLISHED state.
    VersionNotPublished,
    /// The definition version does not belong to the specified domain.
    CrossDomainViolation,
    /// Context payload failed schema validation.
    ContextValidationFailed(String),
    /// Request payload exceeds size limits.
    SizeLimitExceeded(String),
    /// Assignee could not be resolved (not found, disabled, or ambiguous).
    AssigneeResolutionFailed(String),
    /// Idempotency key conflict: same key, different request hash.
    IdempotencyConflict {
        original_command_id: uuid::Uuid,
        original_request_hash: String,
    },
    /// A previous request with this idempotency key is still processing.
    CommandStillProcessing,
    /// Internal consistency error (defensive check failed).
    InternalConsistency(String),
    /// Generic storage or infrastructure error.
    StorageError(String),
}

impl fmt::Display for CreateWorkflowInstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrincipalNotFound => write!(f, "principal not found"),
            Self::PrincipalDisabled => write!(f, "principal is disabled"),
            Self::DomainNotFound => write!(f, "domain not found"),
            Self::DomainDisabled => write!(f, "domain is disabled"),
            Self::DomainMembershipRequired => {
                write!(f, "caller must have an active domain membership binding")
            }
            Self::DefinitionVersionNotFound => write!(f, "definition version not found"),
            Self::VersionNotPublished => write!(f, "definition version is not PUBLISHED"),
            Self::CrossDomainViolation => {
                write!(
                    f,
                    "definition version does not belong to the specified domain"
                )
            }
            Self::ContextValidationFailed(detail) => {
                write!(f, "context validation failed: {}", detail)
            }
            Self::SizeLimitExceeded(detail) => write!(f, "size limit exceeded: {}", detail),
            Self::AssigneeResolutionFailed(detail) => {
                write!(f, "assignee resolution failed: {}", detail)
            }
            Self::IdempotencyConflict {
                original_command_id,
                original_request_hash,
            } => {
                write!(
                    f,
                    "idempotency conflict: original command_id={}, request_hash={}",
                    original_command_id, original_request_hash
                )
            }
            Self::CommandStillProcessing => {
                write!(f, "command with this idempotency key is still processing")
            }
            Self::InternalConsistency(detail) => {
                write!(f, "internal consistency error: {}", detail)
            }
            Self::StorageError(detail) => write!(f, "storage error: {}", detail),
        }
    }
}

impl std::error::Error for CreateWorkflowInstanceError {}

/// Error type for workflow context revision operations.
#[derive(Debug, Clone)]
pub enum ReviseWorkflowContextError {
    /// Principal does not exist.
    PrincipalNotFound,
    /// Principal exists but is disabled.
    PrincipalDisabled,
    /// Workflow instance not found.
    InstanceNotFound,
    /// Current node visit not found for the instance.
    CurrentVisitNotFound,
    /// Current node is not of type DRAFT.
    CurrentNodeNotDraft,
    /// Definition version is REVOKED (blocks normal commands).
    DefinitionVersionRevoked,
    /// Definition version is DRAFT (defensive — instance should not reference it).
    DefinitionVersionDraft,
    /// Expected workflow state version does not match current.
    WorkflowStateVersionConflict { expected: i32, actual: i32 },
    /// Context payload failed schema validation.
    ContextValidationFailed(String),
    /// Request payload exceeds size limits.
    SizeLimitExceeded(String),
    /// Internal consistency error (defensive check failed).
    InternalConsistency(String),
    /// Idempotency key conflict: same key, different request hash.
    IdempotencyConflict {
        original_command_id: uuid::Uuid,
        original_request_hash: String,
    },
    /// A previous request with this idempotency key is still processing.
    CommandStillProcessing,
    /// Generic storage or infrastructure error.
    StorageError(String),
}

impl fmt::Display for ReviseWorkflowContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrincipalNotFound => write!(f, "principal not found"),
            Self::PrincipalDisabled => write!(f, "principal is disabled"),
            Self::InstanceNotFound => write!(f, "workflow instance not found"),
            Self::CurrentVisitNotFound => write!(f, "current node visit not found"),
            Self::CurrentNodeNotDraft => write!(f, "current node is not DRAFT"),
            Self::DefinitionVersionRevoked => write!(f, "definition version is REVOKED"),
            Self::DefinitionVersionDraft => write!(f, "definition version is DRAFT"),
            Self::WorkflowStateVersionConflict { expected, actual } => {
                write!(
                    f,
                    "workflow state version conflict: expected={}, actual={}",
                    expected, actual
                )
            }
            Self::ContextValidationFailed(detail) => {
                write!(f, "context validation failed: {}", detail)
            }
            Self::SizeLimitExceeded(detail) => write!(f, "size limit exceeded: {}", detail),
            Self::InternalConsistency(detail) => {
                write!(f, "internal consistency error: {}", detail)
            }
            Self::IdempotencyConflict {
                original_command_id,
                original_request_hash,
            } => {
                write!(
                    f,
                    "idempotency conflict: original command_id={}, request_hash={}",
                    original_command_id, original_request_hash
                )
            }
            Self::CommandStillProcessing => {
                write!(f, "command with this idempotency key is still processing")
            }
            Self::StorageError(detail) => write!(f, "storage error: {}", detail),
        }
    }
}

impl std::error::Error for ReviseWorkflowContextError {}

/// Map a ReviseWorkflowContextError to an HTTP-style status code.
pub fn revise_error_code(err: &ReviseWorkflowContextError) -> i32 {
    match err {
        ReviseWorkflowContextError::PrincipalNotFound => 404,
        ReviseWorkflowContextError::PrincipalDisabled => 403,
        ReviseWorkflowContextError::InstanceNotFound => 404,
        ReviseWorkflowContextError::CurrentVisitNotFound => 404,
        ReviseWorkflowContextError::CurrentNodeNotDraft => 409,
        ReviseWorkflowContextError::DefinitionVersionRevoked => 409,
        ReviseWorkflowContextError::DefinitionVersionDraft => 500,
        ReviseWorkflowContextError::WorkflowStateVersionConflict { .. } => 409,
        ReviseWorkflowContextError::ContextValidationFailed(_) => 422,
        ReviseWorkflowContextError::SizeLimitExceeded(_) => 413,
        ReviseWorkflowContextError::InternalConsistency(_) => 500,
        ReviseWorkflowContextError::IdempotencyConflict { .. } => 409,
        ReviseWorkflowContextError::CommandStillProcessing => 425,
        ReviseWorkflowContextError::StorageError(_) => 500,
    }
}

/// Map a ReviseWorkflowContextError to a stable string label.
pub fn revise_error_label(err: &ReviseWorkflowContextError) -> &'static str {
    match err {
        ReviseWorkflowContextError::PrincipalNotFound => "principal_not_found",
        ReviseWorkflowContextError::PrincipalDisabled => "principal_disabled",
        ReviseWorkflowContextError::InstanceNotFound => "instance_not_found",
        ReviseWorkflowContextError::CurrentVisitNotFound => "current_visit_not_found",
        ReviseWorkflowContextError::CurrentNodeNotDraft => "current_node_not_draft",
        ReviseWorkflowContextError::DefinitionVersionRevoked => "definition_version_revoked",
        ReviseWorkflowContextError::DefinitionVersionDraft => "definition_version_draft",
        ReviseWorkflowContextError::WorkflowStateVersionConflict { .. } => {
            "workflow_state_version_conflict"
        }
        ReviseWorkflowContextError::ContextValidationFailed(_) => "context_validation_failed",
        ReviseWorkflowContextError::SizeLimitExceeded(_) => "size_limit_exceeded",
        ReviseWorkflowContextError::InternalConsistency(_) => "internal_consistency_error",
        ReviseWorkflowContextError::IdempotencyConflict { .. } => "idempotency_conflict",
        ReviseWorkflowContextError::CommandStillProcessing => "command_still_processing",
        ReviseWorkflowContextError::StorageError(_) => "storage_error",
    }
}
