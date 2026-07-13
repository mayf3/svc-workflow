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
