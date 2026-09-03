//! Idempotency key and request hash computation.
//!
//! Implements the request hash algorithm per the frozen contract:
//!
//! ```json
//! JCS({
//!   "commandSchemaVersion": "...",
//!   "commandType": "CREATE_WORKFLOW_INSTANCE",
//!   "routeParameters": {},
//!   "requestBody": {
//!     "principalId": "...",
//!     "domainId": "...",
//!     "definitionVersionId": "...",
//!     "contextPayload": ...,
//!     "metadata": ...,
//!     "externalReference": null,
//!     "externalUrl": null
//!   }
//! }) → SHA-256
//! ```
//!
//! For `routeParameters`, we use a stable empty object `{}` since this
//! command has no HTTP route parameters.

use serde::Serialize;

use crate::domain::ids::{
    DefinitionVersionId, DomainId, PrincipalId, TransitionId, WorkflowInstanceId,
};
use crate::domain::workflow_instance::combined_errors::ReviseContextAndTransitionError;
use crate::domain::workflow_instance::errors::ArchiveWorkflowInstanceError;
use crate::domain::workflow_instance::errors::CancelWorkflowInstanceError;
use crate::domain::workflow_instance::errors::CreateWorkflowInstanceError;
use crate::domain::workflow_instance::errors::ExecuteWorkflowTransitionError;
use crate::domain::workflow_instance::errors::ReviseWorkflowContextError;
use crate::domain::workflow_instance::events::COMMAND_TYPE_ARCHIVE_WORKFLOW_INSTANCE;
use crate::domain::workflow_instance::events::COMMAND_TYPE_CANCEL_WORKFLOW_INSTANCE;
use crate::domain::workflow_instance::events::COMMAND_TYPE_CREATE_INSTANCE;
use crate::domain::workflow_instance::events::COMMAND_TYPE_EXECUTE_TRANSITION;
use crate::domain::workflow_instance::events::COMMAND_TYPE_REVISE_CONTEXT;
use crate::domain::workflow_instance::events::COMMAND_TYPE_REVISE_CONTEXT_AND_TRANSITION;

/// The canonical request envelope used for hash computation.
///
/// Per the frozen contract, `requestBody` is a nested object containing
/// all request fields except the idempotency key.
#[derive(Debug, Clone, Serialize)]
struct RequestEnvelope {
    command_schema_version: String,
    command_type: String,
    route_parameters: serde_json::Value,
    request_body: RequestBody,
}

/// The body of the request without the idempotency key.
#[derive(Debug, Clone, Serialize)]
struct RequestBody {
    principal_id: String,
    domain_id: String,
    definition_version_id: String,
    context_payload: serde_json::Value,
    metadata: serde_json::Value,
    external_reference: Option<String>,
    external_url: Option<String>,
}

/// Compute the canonical request hash for idempotency.
///
/// The hash covers all command parameters except the idempotency key itself.
pub fn compute_request_hash(
    command_schema_version: &str,
    _idempotency_key: &str,
    principal_id: &PrincipalId,
    domain_id: &DomainId,
    definition_version_id: &DefinitionVersionId,
    context_payload: &serde_json::Value,
    metadata: &serde_json::Value,
    external_reference: &Option<String>,
    external_url: &Option<String>,
) -> Result<String, CreateWorkflowInstanceError> {
    let envelope = RequestEnvelope {
        command_schema_version: command_schema_version.to_string(),
        command_type: COMMAND_TYPE_CREATE_INSTANCE.to_string(),
        route_parameters: serde_json::json!({}),
        request_body: RequestBody {
            principal_id: principal_id.to_string(),
            domain_id: domain_id.to_string(),
            definition_version_id: definition_version_id.to_string(),
            context_payload: context_payload.clone(),
            metadata: metadata.clone(),
            external_reference: external_reference.clone(),
            external_url: external_url.clone(),
        },
    };

    jcs_canonicalize::sha256_jcs_hex(&envelope).map_err(|e| {
        CreateWorkflowInstanceError::StorageError(format!("request hash computation failed: {}", e))
    })
}

/// The canonical request envelope for ReviseWorkflowContext.
#[derive(Debug, Clone, Serialize)]
struct ReviseRequestEnvelope {
    command_schema_version: String,
    command_type: String,
    route_parameters: serde_json::Value,
    request_body: ReviseRequestBody,
}

/// The body of the ReviseWorkflowContext request without the idempotency key.
#[derive(Debug, Clone, Serialize)]
struct ReviseRequestBody {
    principal_id: String,
    workflow_instance_id: String,
    expected_workflow_state_version: i32,
    context_payload: serde_json::Value,
}

/// Compute the canonical request hash for ReviseWorkflowContext idempotency.
pub fn compute_revise_request_hash(
    command_schema_version: &str,
    _idempotency_key: &str,
    principal_id: &PrincipalId,
    workflow_instance_id: &WorkflowInstanceId,
    expected_workflow_state_version: i32,
    context_payload: &serde_json::Value,
) -> Result<String, ReviseWorkflowContextError> {
    let envelope = ReviseRequestEnvelope {
        command_schema_version: command_schema_version.to_string(),
        command_type: COMMAND_TYPE_REVISE_CONTEXT.to_string(),
        route_parameters: serde_json::json!({}),
        request_body: ReviseRequestBody {
            principal_id: principal_id.to_string(),
            workflow_instance_id: workflow_instance_id.to_string(),
            expected_workflow_state_version,
            context_payload: context_payload.clone(),
        },
    };

    jcs_canonicalize::sha256_jcs_hex(&envelope).map_err(|e| {
        ReviseWorkflowContextError::StorageError(format!("request hash computation failed: {}", e))
    })
}

/// The canonical request envelope for ExecuteWorkflowTransition.
#[derive(Debug, Clone, Serialize)]
struct TransitionRequestEnvelope {
    command_schema_version: String,
    command_type: String,
    route_parameters: serde_json::Value,
    request_body: TransitionRequestBody,
}

/// The body of the ExecuteWorkflowTransition request without the idempotency key.
#[derive(Debug, Clone, Serialize)]
struct TransitionRequestBody {
    principal_id: String,
    workflow_instance_id: String,
    expected_workflow_state_version: i32,
    transition_definition_id: String,
    submission_payload: Option<serde_json::Value>,
}

/// Compute the canonical request hash for ExecuteWorkflowTransition idempotency.
pub fn compute_transition_request_hash(
    command_schema_version: &str,
    _idempotency_key: &str,
    principal_id: &PrincipalId,
    workflow_instance_id: &WorkflowInstanceId,
    expected_workflow_state_version: i32,
    transition_definition_id: &TransitionId,
    submission_payload: &Option<serde_json::Value>,
) -> Result<String, ExecuteWorkflowTransitionError> {
    let envelope = TransitionRequestEnvelope {
        command_schema_version: command_schema_version.to_string(),
        command_type: COMMAND_TYPE_EXECUTE_TRANSITION.to_string(),
        route_parameters: serde_json::json!({}),
        request_body: TransitionRequestBody {
            principal_id: principal_id.to_string(),
            workflow_instance_id: workflow_instance_id.to_string(),
            expected_workflow_state_version,
            transition_definition_id: transition_definition_id.to_string(),
            submission_payload: submission_payload.clone(),
        },
    };

    jcs_canonicalize::sha256_jcs_hex(&envelope).map_err(|e| {
        ExecuteWorkflowTransitionError::StorageError(format!(
            "request hash computation failed: {}",
            e
        ))
    })
}

#[derive(Debug, Clone, Serialize)]
struct CombinedRequestEnvelope {
    command_schema_version: String,
    command_type: String,
    route_parameters: serde_json::Value,
    request_body: CombinedRequestBody,
}

#[derive(Debug, Clone, Serialize)]
struct CombinedRequestBody {
    principal_id: String,
    workflow_instance_id: String,
    expected_workflow_state_version: i32,
    transition_definition_id: String,
    context_payload: serde_json::Value,
    submission_payload: serde_json::Value,
}

/// Compute the canonical request hash for ReviseContextAndTransition.
pub fn compute_combined_request_hash(
    command_schema_version: &str,
    principal_id: &PrincipalId,
    workflow_instance_id: &WorkflowInstanceId,
    expected_workflow_state_version: i32,
    transition_definition_id: &TransitionId,
    context_payload: &serde_json::Value,
    submission_payload: &serde_json::Value,
) -> Result<String, ReviseContextAndTransitionError> {
    let envelope = CombinedRequestEnvelope {
        command_schema_version: command_schema_version.to_string(),
        command_type: COMMAND_TYPE_REVISE_CONTEXT_AND_TRANSITION.to_string(),
        route_parameters: serde_json::json!({}),
        request_body: CombinedRequestBody {
            principal_id: principal_id.to_string(),
            workflow_instance_id: workflow_instance_id.to_string(),
            expected_workflow_state_version,
            transition_definition_id: transition_definition_id.to_string(),
            context_payload: context_payload.clone(),
            submission_payload: submission_payload.clone(),
        },
    };

    jcs_canonicalize::sha256_jcs_hex(&envelope).map_err(|error| {
        ReviseContextAndTransitionError::StorageError(format!(
            "request hash computation failed: {}",
            error
        ))
    })
}

/// The canonical request envelope for CancelWorkflowInstance.
#[derive(Debug, Clone, Serialize)]
struct CancelRequestEnvelope {
    command_schema_version: String,
    command_type: String,
    route_parameters: serde_json::Value,
    request_body: CancelRequestBody,
}

/// The body of the CancelWorkflowInstance request without the idempotency key.
#[derive(Debug, Clone, Serialize)]
struct CancelRequestBody {
    principal_id: String,
    workflow_instance_id: String,
    reason: String,
}

/// Compute the canonical request hash for CancelWorkflowInstance idempotency.
pub fn compute_cancel_request_hash(
    command_schema_version: &str,
    principal_id: &PrincipalId,
    workflow_instance_id: &WorkflowInstanceId,
    reason: &str,
) -> Result<String, CancelWorkflowInstanceError> {
    let envelope = CancelRequestEnvelope {
        command_schema_version: command_schema_version.to_string(),
        command_type: COMMAND_TYPE_CANCEL_WORKFLOW_INSTANCE.to_string(),
        route_parameters: serde_json::json!({}),
        request_body: CancelRequestBody {
            principal_id: principal_id.to_string(),
            workflow_instance_id: workflow_instance_id.to_string(),
            reason: reason.to_string(),
        },
    };

    jcs_canonicalize::sha256_jcs_hex(&envelope).map_err(|e| {
        CancelWorkflowInstanceError::StorageError(format!("request hash computation failed: {}", e))
    })
}

/// The canonical request envelope for ArchiveWorkflowInstance.
#[derive(Debug, Clone, Serialize)]
struct ArchiveRequestEnvelope {
    command_schema_version: String,
    command_type: String,
    route_parameters: serde_json::Value,
    request_body: ArchiveRequestBody,
}

/// The body of the ArchiveWorkflowInstance request without the idempotency key.
#[derive(Debug, Clone, Serialize)]
struct ArchiveRequestBody {
    principal_id: String,
    workflow_instance_id: String,
    reason: String,
}

/// Compute the canonical request hash for ArchiveWorkflowInstance idempotency.
pub fn compute_archive_request_hash(
    command_schema_version: &str,
    principal_id: &PrincipalId,
    workflow_instance_id: &WorkflowInstanceId,
    reason: &str,
) -> Result<String, ArchiveWorkflowInstanceError> {
    let envelope = ArchiveRequestEnvelope {
        command_schema_version: command_schema_version.to_string(),
        command_type: COMMAND_TYPE_ARCHIVE_WORKFLOW_INSTANCE.to_string(),
        route_parameters: serde_json::json!({}),
        request_body: ArchiveRequestBody {
            principal_id: principal_id.to_string(),
            workflow_instance_id: workflow_instance_id.to_string(),
            reason: reason.to_string(),
        },
    };

    jcs_canonicalize::sha256_jcs_hex(&envelope).map_err(|e| {
        ArchiveWorkflowInstanceError::StorageError(format!(
            "request hash computation failed: {}",
            e
        ))
    })
}


/// The canonical request envelope for WakeDispatchIntent.
#[derive(Debug, Clone, Serialize)]
struct WakeRequestEnvelope {
    command_schema_version: String,
    command_type: String,
    route_parameters: serde_json::Value,
    request_body: WakeRequestBody,
}

/// The body of the wake request without the idempotency key.
#[derive(Debug, Clone, Serialize)]
struct WakeRequestBody {
    principal_id: String,
    workflow_instance_id: String,
    node_visit_id: String,
    expected_workflow_state_version: i32,
    cause: Option<String>,
}

/// Compute the canonical request hash for WakeDispatchIntent idempotency.
pub fn compute_wake_request_hash(
    command_schema_version: &str,
    _idempotency_key: &str,
    principal_id: &PrincipalId,
    workflow_instance_id: &WorkflowInstanceId,
    node_visit_id: &crate::domain::ids::NodeVisitId,
    expected_workflow_state_version: i32,
    cause: &Option<String>,
) -> Result<String, String> {
    let envelope = WakeRequestEnvelope {
        command_schema_version: command_schema_version.to_string(),
        command_type: crate::domain::workflow_instance::events::COMMAND_TYPE_WAKE_DISPATCH_INTENT
            .to_string(),
        route_parameters: serde_json::json!({}),
        request_body: WakeRequestBody {
            principal_id: principal_id.to_string(),
            workflow_instance_id: workflow_instance_id.to_string(),
            node_visit_id: node_visit_id.to_string(),
            expected_workflow_state_version,
            cause: cause.clone(),
        },
    };

    jcs_canonicalize::sha256_jcs_hex(&envelope)
        .map_err(|e| format!("request hash computation failed: {}", e))
}
