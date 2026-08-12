//! Visit-scoped Workflow Assistance V1 domain contract.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::ids::{AssistanceCaseId, NodeVisitId, PrincipalId, WorkflowInstanceId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssistanceCaseStatus {
    OwnerPending,
    HumanRequired,
    Resolved,
    Voided,
}

impl AssistanceCaseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OwnerPending => "OWNER_PENDING",
            Self::HumanRequired => "HUMAN_REQUIRED",
            Self::Resolved => "RESOLVED",
            Self::Voided => "VOIDED",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AssistanceError> {
        match value {
            "OWNER_PENDING" => Ok(Self::OwnerPending),
            "HUMAN_REQUIRED" => Ok(Self::HumanRequired),
            "RESOLVED" => Ok(Self::Resolved),
            "VOIDED" => Ok(Self::Voided),
            other => Err(AssistanceError::InternalConsistency(format!(
                "unknown assistance status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssistancePayload {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supporting_payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct RequestAssistanceCommand {
    pub principal_id: PrincipalId,
    pub idempotency_key: String,
    pub command_schema_version: String,
    pub workflow_instance_id: WorkflowInstanceId,
    pub current_node_visit_id: NodeVisitId,
    pub expected_workflow_state_version: i32,
    pub request: AssistancePayload,
}

#[derive(Debug, Clone)]
pub struct EscalateAssistanceCommand {
    pub principal_id: PrincipalId,
    pub idempotency_key: String,
    pub command_schema_version: String,
    pub assistance_case_id: AssistanceCaseId,
    pub expected_workflow_state_version: i32,
    pub escalation: AssistancePayload,
}

#[derive(Debug, Clone)]
pub struct ResolveAssistanceCommand {
    pub principal_id: PrincipalId,
    pub idempotency_key: String,
    pub command_schema_version: String,
    pub assistance_case_id: AssistanceCaseId,
    pub expected_workflow_state_version: i32,
    pub resolution: AssistancePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistanceCommandResult {
    pub assistance_case_id: Uuid,
    pub workflow_instance_id: Uuid,
    pub node_visit_id: Uuid,
    pub status: AssistanceCaseStatus,
    pub workflow_state_version: i32,
    pub event_sequence: i32,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing)]
    pub replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssistanceError {
    PrincipalNotFound,
    PrincipalDisabled,
    InstanceNotFound,
    CurrentVisitNotFound,
    CurrentNodeVisitMismatch,
    PrincipalNotAssignee,
    SourceNodeTerminal,
    InstanceCancelled,
    InstanceArchived,
    WorkflowStateVersionConflict { expected: i32, actual: i32 },
    DomainOwnerMissing,
    NotDomainOwner,
    AssistanceAlreadyOpen,
    AssistanceCaseNotFoundOrNotVisible,
    AssistanceStatusConflict,
    GlobalCoordinatorRequired,
    InvalidPayload(String),
    SizeLimitExceeded,
    InvalidPagination(String),
    IdempotencyConflict,
    CommandStillProcessing,
    InternalConsistency(String),
    StorageError(String),
}

impl AssistanceError {
    pub fn status_code(&self) -> i32 {
        match self {
            Self::PrincipalNotFound | Self::InstanceNotFound | Self::CurrentVisitNotFound => 404,
            Self::PrincipalDisabled
            | Self::PrincipalNotAssignee
            | Self::NotDomainOwner
            | Self::GlobalCoordinatorRequired => 403,
            Self::CurrentNodeVisitMismatch
            | Self::SourceNodeTerminal
            | Self::InstanceCancelled
            | Self::InstanceArchived
            | Self::WorkflowStateVersionConflict { .. }
            | Self::DomainOwnerMissing
            | Self::AssistanceAlreadyOpen
            | Self::AssistanceStatusConflict
            | Self::IdempotencyConflict => 409,
            Self::AssistanceCaseNotFoundOrNotVisible => 404,
            Self::InvalidPayload(_) | Self::InvalidPagination(_) => 422,
            Self::SizeLimitExceeded => 413,
            Self::CommandStillProcessing => 425,
            Self::InternalConsistency(_) => 500,
            Self::StorageError(_) => 503,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::PrincipalNotFound => "principal_not_found",
            Self::PrincipalDisabled => "principal_disabled",
            Self::InstanceNotFound => "instance_not_found",
            Self::CurrentVisitNotFound => "current_visit_not_found",
            Self::CurrentNodeVisitMismatch => "current_node_visit_mismatch",
            Self::PrincipalNotAssignee => "principal_not_assignee",
            Self::SourceNodeTerminal => "source_node_terminal",
            Self::InstanceCancelled => "instance_cancelled",
            Self::InstanceArchived => "instance_archived",
            Self::WorkflowStateVersionConflict { .. } => "workflow_state_version_conflict",
            Self::DomainOwnerMissing => "domain_owner_missing",
            Self::NotDomainOwner => "not_domain_owner",
            Self::AssistanceAlreadyOpen => "assistance_already_open",
            Self::AssistanceCaseNotFoundOrNotVisible => "assistance_case_not_found_or_not_visible",
            Self::AssistanceStatusConflict => "assistance_status_conflict",
            Self::GlobalCoordinatorRequired => "global_coordinator_required",
            Self::InvalidPayload(_) => "invalid_assistance_payload",
            Self::SizeLimitExceeded => "size_limit_exceeded",
            Self::InvalidPagination(_) => "invalid_pagination",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::CommandStillProcessing => "command_still_processing",
            Self::InternalConsistency(_) => "internal_consistency_error",
            Self::StorageError(_) => "service_unavailable",
        }
    }
}

impl std::fmt::Display for AssistanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())?;
        match self {
            Self::WorkflowStateVersionConflict { expected, actual } => {
                write!(f, ": expected={expected}, actual={actual}")
            }
            Self::InvalidPayload(detail)
            | Self::InvalidPagination(detail)
            | Self::InternalConsistency(detail)
            | Self::StorageError(detail) => write!(f, ": {detail}"),
            _ => Ok(()),
        }
    }
}

impl std::error::Error for AssistanceError {}
