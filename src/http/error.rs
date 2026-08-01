//! Stable, redacted HTTP error envelope and domain mappings.

use axum::extract::rejection::JsonRejection;
use axum::extract::rejection::QueryRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::application::definition_governance::DefinitionGovernanceError as DGError;
use crate::application::domain_membership::DomainMembershipError as DMError;
use crate::application::workflow_instance::query_types::WorkflowQueryError;
use crate::domain::provisioning::ProvisioningError as PError;
use crate::domain::workflow_instance::errors::{
    ArchiveWorkflowInstanceError, CancelWorkflowInstanceError, CreateWorkflowInstanceError,
    ExecuteWorkflowTransitionError,
};

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    details: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn bad_request(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn unauthorized(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }

    pub fn unauthorized_with_details(
        code: &'static str,
        message: &'static str,
        details: serde_json::Value,
    ) -> Self {
        Self::unauthorized(code, message).with_details(details)
    }

    pub fn forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "required scope is missing",
        )
    }

    pub fn service_unavailable(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, code, message)
    }

    pub fn unprocessable(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, code, message)
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn from_json_rejection(rejection: JsonRejection) -> Self {
        let status = rejection.status();
        let text = rejection.body_text();
        if status == StatusCode::PAYLOAD_TOO_LARGE {
            return Self::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "size_limit_exceeded",
                "request body exceeds the configured limit",
            )
            .with_details(serde_json::json!({ "field": "request_body" }));
        }
        if text.contains("unknown field") {
            return Self::bad_request("unknown_field", "request contains an unknown field");
        }
        Self::bad_request(
            "invalid_json",
            "request body is not valid for this endpoint",
        )
    }

    pub fn from_query_rejection(rejection: QueryRejection) -> Self {
        tracing::debug!(error = %rejection.body_text(), "invalid query parameters");
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_pagination",
            "pagination parameters are invalid",
        )
    }

    pub fn from_create(error: CreateWorkflowInstanceError) -> Self {
        use CreateWorkflowInstanceError as E;
        match error {
            E::PrincipalNotFound => not_found("principal_not_found", "principal not found"),
            E::PrincipalDisabled => forbidden("principal_disabled", "principal is disabled"),
            E::DomainNotFound => not_found("domain_not_found", "domain not found"),
            E::DomainDisabled => forbidden("domain_disabled", "domain is disabled"),
            E::DomainMembershipRequired => forbidden(
                "domain_membership_required",
                "active domain membership is required",
            ),
            E::DefinitionVersionNotFound => not_found(
                "definition_version_not_found",
                "definition version not found",
            ),
            E::VersionNotPublished => conflict(
                "version_not_published",
                "definition version is not published",
            ),
            E::CrossDomainViolation => forbidden(
                "cross_domain_violation",
                "definition version does not belong to the domain",
            ),
            E::ContextValidationFailed(_) => unprocessable(
                "context_validation_failed",
                "context payload failed validation",
            ),
            E::SizeLimitExceeded(detail) => size_limit(size_field(&detail)),
            E::AssigneeResolutionFailed(_) => unprocessable(
                "assignee_resolution_failed",
                "initial assignee could not be resolved",
            ),
            E::IdempotencyConflict { .. } => {
                conflict("idempotency_conflict", "idempotency key was reused")
            }
            E::CommandStillProcessing => Self::new(
                StatusCode::TOO_EARLY,
                "command_still_processing",
                "command is still processing",
            ),
            E::InternalConsistency(detail) => {
                tracing::error!(error = %detail, "workflow create consistency failure");
                internal("internal_consistency_error", "internal consistency error")
            }
            E::StorageError(detail) => {
                tracing::error!(error = %detail, "workflow create storage failure");
                Self::service_unavailable("service_unavailable", "storage is unavailable")
            }
        }
    }

    pub fn from_transition(error: ExecuteWorkflowTransitionError) -> Self {
        use ExecuteWorkflowTransitionError as E;
        match error {
            E::PrincipalNotFound => not_found("principal_not_found", "principal not found"),
            E::PrincipalDisabled => forbidden("principal_disabled", "principal is disabled"),
            E::InstanceNotFound => not_found("instance_not_found", "workflow instance not found"),
            E::CurrentVisitNotFound => {
                not_found("current_visit_not_found", "current node visit not found")
            }
            E::PrincipalNotAssignee => forbidden(
                "principal_not_assignee",
                "principal is not the current assignee",
            ),
            E::SourceNodeTerminal => {
                conflict("source_node_terminal", "terminal nodes cannot transition")
            }
            E::DefinitionVersionRevoked => conflict(
                "definition_version_revoked",
                "definition version is revoked",
            ),
            E::DefinitionVersionDraft | E::DefinitionVersionDeprecated => internal(
                "internal_consistency_error",
                "invalid definition version state",
            ),
            E::WorkflowStateVersionConflict { expected, actual } => conflict(
                "workflow_state_version_conflict",
                "workflow state version does not match",
            )
            .with_details(serde_json::json!({ "expected": expected, "actual": actual })),
            E::TransitionNotApplicable(_) => {
                conflict("transition_not_applicable", "transition is not applicable")
            }
            E::SubmissionRequired => {
                unprocessable("submission_required", "submission payload is required")
            }
            E::SubmissionValidationFailed(_) => unprocessable(
                "submission_validation_failed",
                "submission payload failed validation",
            ),
            E::SizeLimitExceeded(detail) => size_limit(size_field(&detail)),
            E::InvalidReturnReferences(_) => {
                unprocessable("invalid_return_references", "return references are invalid")
            }
            E::AssigneeResolutionFailed(_) => unprocessable(
                "assignee_resolution_failed",
                "target assignee could not be resolved",
            ),
            E::IdempotencyConflict { .. } => {
                conflict("idempotency_conflict", "idempotency key was reused")
            }
            E::CommandStillProcessing => Self::new(
                StatusCode::TOO_EARLY,
                "command_still_processing",
                "command is still processing",
            ),
            E::InternalConsistency(detail) => {
                tracing::error!(error = %detail, "workflow transition consistency failure");
                internal("internal_consistency_error", "internal consistency error")
            }
            E::StorageError(detail) => {
                tracing::error!(error = %detail, "workflow transition storage failure");
                Self::service_unavailable("service_unavailable", "storage is unavailable")
            }
        }
    }

    pub fn from_provisioning(error: PError) -> Self {
        use PError as E;
        let status_code = error.status_code();
        let (code, message) = match &error {
            E::PrincipalNotFound => ("principal_not_found", "principal not found"),
            E::PrincipalDisabled => ("principal_disabled", "principal is disabled"),
            E::PrincipalTypeConflict => (
                "principal_type_conflict",
                "principal type cannot be changed",
            ),
            E::PrincipalTypeInvalid => ("principal_type_invalid", "invalid principal type"),
            E::DomainNotFound => ("domain_not_found", "domain not found"),
            E::DomainDisabled => ("domain_disabled", "domain is disabled"),
            E::DomainIdentityConflict => (
                "domain_identity_conflict",
                "domain key belongs to another domain",
            ),
            E::DomainOwnerConflict => (
                "domain_owner_conflict",
                "domain already has an active owner",
            ),
            E::BindingAlreadyExists => ("binding_already_exists", "role binding already exists"),
            E::BindingNotFound => ("binding_not_found", "role binding not found"),
            E::RoleKeyInvalid => ("role_key_invalid", "invalid role key"),
            E::DefinitionVersionNotFound => (
                "definition_version_not_found",
                "definition version not found",
            ),
            E::PermissionDenied => ("permission_denied", "provisioning not allowed"),
            E::PrincipalTypeNotAllowed => (
                "principal_type_not_allowed",
                "principal type is not allowed for this operation",
            ),
            E::InvalidInput(_) => ("invalid_input", "invalid request input"),
            E::IdempotencyConflict => ("idempotency_conflict", "idempotency key was reused"),
            E::CommandStillProcessing => {
                ("command_still_processing", "command is still processing")
            }
            E::InternalConsistency(_) => {
                ("internal_consistency_error", "internal consistency error")
            }
            E::StorageError(_) => ("service_unavailable", "storage is unavailable"),
        };
        let mut api_error = Self::new(
            StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            code,
            message,
        );
        if let Some(detail) = error.detail() {
            api_error = api_error.with_details(serde_json::json!({ "detail": detail }));
        }
        api_error
    }

    pub fn from_domain_membership(error: DMError) -> Self {
        use DMError as E;
        let status_code = error.status_code();
        let (code, message) = match &error {
            E::PrincipalNotRegistered => (
                "principal_not_registered",
                "principal has not completed self-projection",
            ),
            E::PrincipalDisabled => ("principal_disabled", "principal is disabled"),
            E::PrincipalProjectionConflict => (
                "principal_projection_conflict",
                "principal exists with a different type",
            ),
            E::DomainNotFound => ("domain_not_found", "domain not found"),
            E::NotDomainOwner => ("not_domain_owner", "caller is not a domain owner"),
            E::PrincipalIsOwner => (
                "principal_is_owner",
                "principal is a domain owner and cannot be a member",
            ),
            E::MemberNotFound => ("member_not_found", "member binding not found"),
            E::DirectTokenRequired => (
                "direct_token_required",
                "only direct access tokens may perform this operation",
            ),
            E::IdempotencyConflict => ("idempotency_conflict", "idempotency key was reused"),
            E::CommandStillProcessing => {
                ("command_still_processing", "command is still processing")
            }
            E::InternalConsistency(_) => {
                ("internal_consistency_error", "internal consistency error")
            }
            E::StorageError(_) => ("service_unavailable", "storage is unavailable"),
        };
        let mut api_error = Self::new(
            StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            code,
            message,
        );
        if let Some(detail) = error.detail() {
            api_error = api_error.with_details(serde_json::json!({ "detail": detail }));
        }
        api_error
    }

    pub fn from_definition_governance(error: DGError) -> Self {
        let status_code = error.status_code();
        let (code, message) = match &error {
            DGError::NotDomainOwner => ("not_domain_owner", "caller is not a domain owner"),
            DGError::DomainDisabled => ("domain_disabled", "domain is disabled"),
            DGError::DefinitionNotFound => {
                ("definition_not_found", "workflow definition not found")
            }
            DGError::DefinitionArchived | DGError::DefinitionNotEditable => (
                "definition_not_editable",
                "workflow definition is not editable",
            ),
            DGError::DefinitionVersionImmutable => (
                "definition_version_immutable",
                "definition version is not in DRAFT status",
            ),
            DGError::DefinitionKeyConflict => (
                "definition_key_conflict",
                "definition key already exists in this domain",
            ),
            DGError::RevisionConflict => (
                "revision_conflict",
                "concurrent modification detected, retry with fresh data",
            ),
            DGError::DirectTokenRequired => (
                "direct_token_required",
                "only direct access tokens may perform this operation",
            ),
            DGError::IdempotencyConflict => ("idempotency_conflict", "idempotency key was reused"),
            DGError::CommandStillProcessing => {
                ("command_still_processing", "command is still processing")
            }
            DGError::InternalConsistency(_) => {
                ("internal_consistency_error", "internal consistency error")
            }
            DGError::StorageError(_) => ("service_unavailable", "storage is unavailable"),
        };
        let mut api_error = Self::new(
            StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            code,
            message,
        );
        if let Some(detail) = match &error {
            DGError::InternalConsistency(d) => Some(d.as_str()),
            DGError::StorageError(d) => Some(d.as_str()),
            _ => None,
        } {
            api_error = api_error.with_details(serde_json::json!({ "detail": detail }));
        }
        api_error
    }

    pub fn from_cancel(error: CancelWorkflowInstanceError) -> Self {
        use CancelWorkflowInstanceError as E;
        match error {
            E::PrincipalNotFound => not_found("principal_not_found", "principal not found"),
            E::PrincipalDisabled => forbidden("principal_disabled", "principal is disabled"),
            E::InstanceNotFound => not_found("instance_not_found", "workflow instance not found"),
            E::CurrentVisitNotFound => not_found("current_visit_not_found", "current node visit not found"),
            E::NotDomainOwner => forbidden("not_domain_owner", "caller is not a domain owner"),
            E::SourceNodeTerminal => conflict("source_node_terminal", "instance source node is terminal"),
            E::AlreadyCancelled => conflict("already_cancelled", "instance is already cancelled"),
            E::InstanceArchived => conflict("instance_archived", "instance is archived"),
            E::WorkflowStateVersionConflict { expected, actual } => conflict(
                "workflow_state_version_conflict",
                "workflow state version does not match",
            )
            .with_details(serde_json::json!({ "expected": expected, "actual": actual })),
            E::InvalidReason(detail) => unprocessable("invalid_reason", "cancel reason is invalid")
                .with_details(serde_json::json!({ "detail": detail })),
            E::InternalConsistency(detail) => {
                tracing::error!(error = %detail, "cancel consistency failure");
                internal("internal_consistency_error", "internal consistency error")
            }
            E::IdempotencyConflict { .. } => {
                conflict("idempotency_conflict", "idempotency key was reused")
            }
            E::CommandStillProcessing => Self::new(
                StatusCode::TOO_EARLY,
                "command_still_processing",
                "command is still processing",
            ),
            E::StorageError(detail) => {
                tracing::error!(error = %detail, "cancel storage failure");
                Self::service_unavailable("service_unavailable", "storage is unavailable")
            }
        }
    }

    pub fn from_archive(error: ArchiveWorkflowInstanceError) -> Self {
        use ArchiveWorkflowInstanceError as E;
        match error {
            E::PrincipalNotFound => not_found("principal_not_found", "principal not found"),
            E::PrincipalDisabled => forbidden("principal_disabled", "principal is disabled"),
            E::InstanceNotFound => not_found("instance_not_found", "workflow instance not found"),
            E::NotDomainOwner => forbidden("not_domain_owner", "caller is not a domain owner"),
            E::InstanceNotTerminal => conflict("instance_not_terminal", "instance is not in a terminal state"),
            E::AlreadyArchived => conflict("already_archived", "instance is already archived"),
            E::WorkflowStateVersionConflict { expected, actual } => conflict(
                "workflow_state_version_conflict",
                "workflow state version does not match",
            )
            .with_details(serde_json::json!({ "expected": expected, "actual": actual })),
            E::InvalidReason(detail) => unprocessable("invalid_reason", "archive reason is invalid")
                .with_details(serde_json::json!({ "detail": detail })),
            E::InternalConsistency(detail) => {
                tracing::error!(error = %detail, "archive consistency failure");
                internal("internal_consistency_error", "internal consistency error")
            }
            E::IdempotencyConflict { .. } => {
                conflict("idempotency_conflict", "idempotency key was reused")
            }
            E::CommandStillProcessing => Self::new(
                StatusCode::TOO_EARLY,
                "command_still_processing",
                "command is still processing",
            ),
            E::StorageError(detail) => {
                tracing::error!(error = %detail, "archive storage failure");
                Self::service_unavailable("service_unavailable", "storage is unavailable")
            }
        }
    }

    pub fn from_query(error: WorkflowQueryError) -> Self {
        use WorkflowQueryError as E;
        match error {
            E::PrincipalNotFound => not_found("principal_not_found", "principal not found"),
            E::PrincipalDisabled => forbidden("principal_disabled", "principal is disabled"),
            E::WorkflowInstanceNotFoundOrNotVisible => not_found(
                "workflow_instance_not_found_or_not_visible",
                "workflow instance not found or not visible",
            ),
            E::RestrictedHistoryNotVisible => forbidden(
                "restricted_history_not_visible",
                "restricted workflow history is not visible",
            ),
            E::InvalidPagination(_) => {
                unprocessable("invalid_pagination", "pagination parameters are invalid")
            }
            E::InternalConsistency(detail) => {
                tracing::error!(error = %detail, "workflow query consistency failure");
                internal("internal_consistency_error", "internal consistency error")
            }
            E::StorageError(detail) => {
                tracing::error!(error = %detail, "workflow query storage failure");
                Self::service_unavailable("service_unavailable", "storage is unavailable")
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: self.message,
                details: self.details,
            },
        };
        (self.status, Json(body)).into_response()
    }
}

fn not_found(code: &'static str, message: &'static str) -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, code, message)
}

fn forbidden(code: &'static str, message: &'static str) -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, code, message)
}

fn conflict(code: &'static str, message: &'static str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, code, message)
}

fn unprocessable(code: &'static str, message: &'static str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, code, message)
}

fn internal(code: &'static str, message: &'static str) -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, message)
}

fn size_limit(field: &'static str) -> ApiError {
    ApiError::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "size_limit_exceeded",
        "request field exceeds its size limit",
    )
    .with_details(serde_json::json!({ "field": field }))
}

fn size_field(detail: &str) -> &'static str {
    if detail.contains("metadata") {
        "metadata"
    } else if detail.contains("submission") {
        "submissionPayload"
    } else {
        "contextPayload"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotency_conflict_is_opaque() {
        let error =
            ApiError::from_transition(ExecuteWorkflowTransitionError::IdempotencyConflict {
                original_command_id: uuid::Uuid::new_v4(),
                original_request_hash: "secret-hash".to_string(),
            });
        assert_eq!(error.status, StatusCode::CONFLICT);
        assert_eq!(error.code, "idempotency_conflict");
        assert!(error.details.is_none());
    }

    #[test]
    fn storage_is_retryable_but_consistency_is_not() {
        let storage = ApiError::from_query(WorkflowQueryError::StorageError("db down".into()));
        assert_eq!(storage.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(storage.code, "service_unavailable");
        let consistency = ApiError::from_query(WorkflowQueryError::InternalConsistency(
            "broken projection".into(),
        ));
        assert_eq!(consistency.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(consistency.code, "internal_consistency_error");
    }
}
