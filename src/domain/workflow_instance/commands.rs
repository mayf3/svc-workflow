//! Command input types for the Workflow Instance domain.

use crate::domain::ids::{DefinitionVersionId, DomainId, PrincipalId, WorkflowInstanceId};

/// Command to create a new workflow instance from a published definition version.
///
/// This is the sole command for PR 3A. All fields are required except
/// where explicitly marked as optional.
#[derive(Debug, Clone)]
pub struct CreateWorkflowInstanceCommand {
    /// The principal initiating the command.
    pub principal_id: PrincipalId,

    /// Client-supplied idempotency key, unique per principal.
    pub idempotency_key: String,

    /// Schema version of this command structure.
    pub command_schema_version: String,

    /// Target domain for the new instance.
    pub domain_id: DomainId,

    /// Published definition version to instantiate.
    pub definition_version_id: DefinitionVersionId,

    /// Optional caller-supplied external reference identifier.
    pub external_reference: Option<String>,

    /// Optional external URL associated with the instance.
    pub external_url: Option<String>,

    /// Arbitrary metadata attached to the instance.
    pub metadata: serde_json::Value,

    /// Initial context payload (validated against the definition's context_schema).
    pub context_payload: serde_json::Value,
}

/// Command to create a new revision of the workflow context for an existing instance.
///
/// This is the sole command for PR 3B. Only the Workflow Creator (the principal
/// whose ID equals `workflow_instance.created_by_principal_id`) may revise the
/// context, and only while the current node is of type DRAFT.
#[derive(Debug, Clone)]
pub struct ReviseWorkflowContextCommand {
    /// The principal initiating the command.
    pub principal_id: PrincipalId,

    /// Client-supplied idempotency key, unique per principal.
    pub idempotency_key: String,

    /// Schema version of this command structure.
    pub command_schema_version: String,

    /// The target workflow instance.
    pub workflow_instance_id: WorkflowInstanceId,

    /// The caller's expected current workflow state version (optimistic concurrency).
    pub expected_workflow_state_version: i32,

    /// The new context payload to store.
    pub context_payload: serde_json::Value,
}
