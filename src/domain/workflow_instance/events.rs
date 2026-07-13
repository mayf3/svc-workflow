//! Event type constants and event data structures for the Workflow Instance domain.

use serde::{Deserialize, Serialize};

/// Stable schema version for all events created by this service.
pub const EVENT_SCHEMA_VERSION: &str = "v1";

/// Stable command type string for CreateWorkflowInstance.
pub const COMMAND_TYPE_CREATE_INSTANCE: &str = "CREATE_WORKFLOW_INSTANCE";

/// Event type for instance creation events.
pub const INSTANCE_CREATED_EVENT_TYPE: &str = "INSTANCE_CREATED";

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
