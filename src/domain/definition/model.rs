//! Domain model types for workflow definition entities.
//!
//! These types represent the business concepts of Workflow Definition,
//! Definition Version, Node Definitions, and Transition Definitions.
//! They are independent of any storage or serialization format.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::enums::{AssigneeRefType, DefinitionVersionStatus, NodeType, TransitionEffect};
use crate::domain::ids::{
    DefinitionVersionId, DomainId, NodeId, PrincipalId, TransitionId, WorkflowDefinitionId,
};

// ---------------------------------------------------------------------------
// Workflow Definition
// ---------------------------------------------------------------------------

/// A workflow definition (template) that belongs to a domain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowDefinition {
    pub id: WorkflowDefinitionId,
    pub domain_id: DomainId,
    pub definition_key: String,
    pub display_name: String,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub archived_by_principal_id: Option<PrincipalId>,
}

// ---------------------------------------------------------------------------
// Semantic Model Version
// ---------------------------------------------------------------------------

/// The semantic model under which a workflow definition version must be
/// interpreted.
///
/// This is a first-class, immutable fact on `WorkflowDefinitionVersion` —
/// never inferred from node shapes, publishing time, or DB state.
///
/// * `1` = Legacy semantics: all pre-existing definitions and every version
///   created until Minimal semantics ships. Legacy rules (DRAFT node type,
///   DOMAIN_OWNER assignee, primary ADVANCE transition, orderIndex
///   execution) remain frozen and unchanged.
/// * `2` = Minimal semantics: a defined version number only. Minimal
///   runtime/validator behavior is NOT implemented yet and must not be
///   produced by production create paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum SemanticModelVersion {
    Legacy = 1,
    Minimal = 2,
}

impl SemanticModelVersion {
    /// Version used by every production create path today.
    pub const DEFAULT: Self = Self::Legacy;

    pub fn as_i16(self) -> i16 {
        self as i16
    }
}

impl TryFrom<i16> for SemanticModelVersion {
    type Error = String;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Legacy),
            2 => Ok(Self::Minimal),
            other => Err(format!("invalid semantic_model_version: {other}")),
        }
    }
}

impl serde::Serialize for SemanticModelVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i16(self.as_i16())
    }
}

impl<'de> serde::Deserialize<'de> for SemanticModelVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = i16::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Workflow Definition Version
// ---------------------------------------------------------------------------

/// A versioned snapshot of a workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowDefinitionVersion {
    pub id: DefinitionVersionId,
    pub workflow_definition_id: WorkflowDefinitionId,
    pub version_number: i32,
    pub version_status: DefinitionVersionStatus,
    /// Semantic interpretation version (1 = Legacy, 2 = Minimal). Immutable
    /// for the lifetime of the version; absent in pre-0019 rows it defaults
    /// to Legacy on read.
    #[serde(default = "default_semantic_model_version")]
    pub semantic_model_version: SemanticModelVersion,
    pub definition_digest: Option<String>,
    pub json_schema_dialect: Option<String>,
    pub validator_version: Option<String>,
    pub context_schema: Option<serde_json::Value>,
    pub submission_schema: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub deprecated_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub published_by_principal_id: Option<PrincipalId>,
    pub deprecated_by_principal_id: Option<PrincipalId>,
    pub revoked_by_principal_id: Option<PrincipalId>,
}

fn default_semantic_model_version() -> SemanticModelVersion {
    SemanticModelVersion::DEFAULT
}

// ---------------------------------------------------------------------------
// Node Definition
// ---------------------------------------------------------------------------

/// A node (step) within a workflow definition version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeDefinition {
    pub node_id: NodeId,
    pub definition_version_id: DefinitionVersionId,
    pub node_key: String,
    pub display_name: String,
    pub order_index: i32,
    pub node_type: NodeType,
    /// Terminal nodes have no assignee reference. Non-terminal nodes must have one.
    pub assignee_ref: Option<AssigneeRef>,
    pub instructions: Option<String>,
    pub primary_advance_transition_id: Option<TransitionId>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Represents how a node's assignee is resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssigneeRef {
    pub ref_type: AssigneeRefType,
    pub fixed_principal_id: Option<PrincipalId>,
    /// Key into the instance's context_payload that carries the assignee's
    /// stable Principal UUID. Required iff `ref_type == InstanceInputPrincipal`.
    pub assignee_input_key: Option<String>,
}

// ---------------------------------------------------------------------------
// Transition Definition
// ---------------------------------------------------------------------------

/// A transition (edge) connecting two nodes in a workflow definition version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransitionDefinition {
    pub transition_id: TransitionId,
    pub definition_version_id: DefinitionVersionId,
    pub transition_key: String,
    pub display_name: String,
    pub source_node_id: NodeId,
    pub target_node_id: NodeId,
    pub transition_effect: TransitionEffect,
    pub submission_schema: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Helper types for graph representation
// ---------------------------------------------------------------------------

/// A complete workflow graph bundled with its context schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowGraph {
    pub nodes: Vec<NodeDefinition>,
    pub transitions: Vec<TransitionDefinition>,
    pub context_schema: Option<serde_json::Value>,
}

/// Result of graph validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<super::error::GraphValidationError>,
    pub warnings: Vec<String>,
    pub computed_digest: Option<String>,
}

/// Result of publishing a version.
#[derive(Debug, Clone)]
pub struct PublishResult {
    pub version: WorkflowDefinitionVersion,
    pub digest: String,
}

// ---------------------------------------------------------------------------
// Sort keys helpers (used in digest computation)
// ---------------------------------------------------------------------------

impl NodeDefinition {
    /// Stable sort key for canonical ordering.
    pub fn sort_key(&self) -> &str {
        &self.node_key
    }
}

impl TransitionDefinition {
    /// Stable sort key for canonical ordering.
    pub fn sort_key(&self) -> &str {
        &self.transition_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_model_version_roundtrips_as_integer() {
        assert_eq!(SemanticModelVersion::Legacy.as_i16(), 1);
        assert_eq!(SemanticModelVersion::Minimal.as_i16(), 2);
        assert_eq!(SemanticModelVersion::DEFAULT, SemanticModelVersion::Legacy);

        assert_eq!(
            SemanticModelVersion::try_from(1_i16).unwrap(),
            SemanticModelVersion::Legacy
        );
        assert_eq!(
            SemanticModelVersion::try_from(2_i16).unwrap(),
            SemanticModelVersion::Minimal
        );
        assert!(SemanticModelVersion::try_from(0_i16).is_err());
        assert!(SemanticModelVersion::try_from(3_i16).is_err());

        // Serializes as the DB integer, not a disguised string.
        let json = serde_json::to_string(&SemanticModelVersion::Legacy).unwrap();
        assert_eq!(json, "1");
        let parsed: SemanticModelVersion = serde_json::from_str("2").unwrap();
        assert_eq!(parsed, SemanticModelVersion::Minimal);
        assert!(serde_json::from_str::<SemanticModelVersion>("3").is_err());
    }

    #[test]
    fn missing_semantic_model_version_deserializes_to_legacy() {
        // Pre-0019 JSON (no field) must read back as Legacy, never fail.
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "workflow_definition_id": "00000000-0000-0000-0000-000000000002",
            "version_number": 1,
            "version_status": "DRAFT",
            "definition_digest": null,
            "json_schema_dialect": null,
            "validator_version": null,
            "context_schema": null,
            "submission_schema": null,
            "metadata": null,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "published_at": null,
            "deprecated_at": null,
            "revoked_at": null,
            "published_by_principal_id": null,
            "deprecated_by_principal_id": null,
            "revoked_by_principal_id": null
        }"#;
        let version: WorkflowDefinitionVersion =
            serde_json::from_str(json).expect("legacy JSON without the field must parse");
        assert_eq!(
            version.semantic_model_version,
            SemanticModelVersion::Legacy
        );
    }
}
