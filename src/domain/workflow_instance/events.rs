//! Event type constants and event data structures for the Workflow Instance domain.

use serde::{Deserialize, Serialize};

/// Stable schema version for all events created by this service.
pub const EVENT_SCHEMA_VERSION: &str = "v1";

/// Stable command type string for CreateWorkflowInstance.
pub const COMMAND_TYPE_CREATE_INSTANCE: &str = "CREATE_WORKFLOW_INSTANCE";

/// Stable command type string for ReviseWorkflowContext.
pub const COMMAND_TYPE_REVISE_CONTEXT: &str = "REVISE_WORKFLOW_CONTEXT";

/// Event type for instance creation events.
pub const INSTANCE_CREATED_EVENT_TYPE: &str = "INSTANCE_CREATED";

/// Event type for context revision events.
pub const CONTEXT_REVISED_EVENT_TYPE: &str = "CONTEXT_REVISED";

/// Non-sensitive event data embedded in the INSTANCE_CREATED event.
///
/// This is the stable, serialized content of `event_data`. It must
/// never include the full context payload, credentials, or secrets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceCreatedEventData {
    /// The definition version ID that was instantiated.
    pub definition_version_id: String,
    /// The SHA-256 digest of the definition version at creation time.
    pub definition_digest: String,
    /// The node ID of the initial DRAFT node.
    pub initial_node_id: String,
    /// How the initial assignee was resolved (WORKFLOW_CREATOR, DOMAIN_OWNER, FIXED_PRINCIPAL).
    pub assignee_resolution_type: String,
}

/// Non-sensitive event data embedded in the CONTEXT_REVISED event.
///
/// Contains only stable identifiers and digests — never the full context payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRevisedEventData {
    /// The context revision ID that was the current revision before this command.
    pub previous_context_revision_id: String,
    /// The context revision ID created by this command.
    pub new_context_revision_id: String,
    /// SHA-256 digest of the previous context payload.
    pub previous_payload_digest: String,
    /// SHA-256 digest of the new context payload.
    pub new_payload_digest: String,
    /// The node ID of the current node visit (unchanged by this command).
    pub current_node_id: String,
}
