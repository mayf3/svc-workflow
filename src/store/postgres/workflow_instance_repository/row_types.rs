//! SQLx row types for the workflow instance repository.
//!
//! These types map PostgreSQL query results to domain types.

use crate::domain::enums::{AssigneeRefType, DefinitionVersionStatus, NodeType};

/// Row type for reading a workflow definition version (subset of columns).
#[derive(Debug, sqlx::FromRow)]
pub(super) struct DefinitionVersionStatusRow {
    pub(super) definition_version_id: uuid::Uuid,
    pub(super) workflow_definition_id: uuid::Uuid,
    pub(super) version_number: i32,
    pub(super) version_status: String,
    pub(super) semantic_model_version: i16,
    pub(super) definition_digest: Option<String>,
    pub(super) json_schema_dialect: Option<String>,
    pub(super) validator_version: Option<String>,
    pub(super) context_schema: Option<serde_json::Value>,
}

impl DefinitionVersionStatusRow {
    pub(super) fn version_status_enum(&self) -> DefinitionVersionStatus {
        self.version_status
            .parse::<DefinitionVersionStatus>()
            .unwrap_or(DefinitionVersionStatus::DRAFT)
    }
}

/// Row type for reading a minimal workflow definition (domain_id).
#[derive(Debug, sqlx::FromRow)]
pub(super) struct WorkflowDefinitionDomainRow {
    pub(super) workflow_definition_id: uuid::Uuid,
    pub(super) domain_id: uuid::Uuid,
}

/// Row type for reading a DRAFT node definition.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct DraftNodeRow {
    pub(super) node_id: uuid::Uuid,
    pub(super) node_type: String,
    pub(super) assignee_ref_type: String,
    pub(super) fixed_principal_id: Option<uuid::Uuid>,
    pub(super) assignee_input_key: Option<String>,
}

/// Row type for reading a full definition graph (Minimal validation).
#[derive(Debug, sqlx::FromRow)]
pub(super) struct GraphNodeRow {
    pub(super) node_id: uuid::Uuid,
    pub(super) definition_version_id: uuid::Uuid,
    pub(super) node_key: String,
    pub(super) display_name: String,
    pub(super) order_index: i32,
    pub(super) node_type: String,
    pub(super) assignee_ref_type: Option<String>,
    pub(super) fixed_principal_id: Option<uuid::Uuid>,
    pub(super) assignee_input_key: Option<String>,
    pub(super) instructions: Option<String>,
    pub(super) primary_advance_transition_id: Option<uuid::Uuid>,
    pub(super) metadata: Option<serde_json::Value>,
    pub(super) created_at: chrono::DateTime<chrono::Utc>,
}

impl GraphNodeRow {
    pub(super) fn into_domain(self) -> crate::domain::definition::model::NodeDefinition {
        use crate::domain::definition::model::{AssigneeRef, NodeDefinition};
        crate::domain::definition::model::NodeDefinition {
            node_id: crate::domain::ids::NodeId::from_uuid(self.node_id),
            definition_version_id: crate::domain::ids::DefinitionVersionId::from_uuid(
                self.definition_version_id,
            ),
            node_key: self.node_key,
            display_name: self.display_name,
            order_index: self.order_index,
            node_type: self
                .node_type
                .parse::<crate::domain::enums::NodeType>()
                .unwrap_or(crate::domain::enums::NodeType::NORMAL),
            assignee_ref: self.assignee_ref_type.map(|t| AssigneeRef {
                ref_type: t
                    .parse::<crate::domain::enums::AssigneeRefType>()
                    .unwrap_or(crate::domain::enums::AssigneeRefType::WorkflowCreator),
                fixed_principal_id: self
                    .fixed_principal_id
                    .map(crate::domain::ids::PrincipalId::from_uuid),
                assignee_input_key: self.assignee_input_key,
            }),
            instructions: self.instructions,
            primary_advance_transition_id: self
                .primary_advance_transition_id
                .map(crate::domain::ids::TransitionId::from_uuid),
            metadata: self.metadata,
            created_at: self.created_at,
        }
    }
}

/// Row type for reading transition definitions (Minimal validation).
#[derive(Debug, sqlx::FromRow)]
pub(super) struct GraphTransitionRow {
    pub(super) transition_id: uuid::Uuid,
    pub(super) definition_version_id: uuid::Uuid,
    pub(super) transition_key: String,
    pub(super) display_name: String,
    pub(super) source_node_id: uuid::Uuid,
    pub(super) target_node_id: uuid::Uuid,
    pub(super) transition_effect: String,
    pub(super) submission_schema: Option<serde_json::Value>,
    pub(super) metadata: Option<serde_json::Value>,
    pub(super) created_at: chrono::DateTime<chrono::Utc>,
}

impl GraphTransitionRow {
    pub(super) fn into_domain(self) -> crate::domain::definition::model::TransitionDefinition {
        crate::domain::definition::model::TransitionDefinition {
            transition_id: crate::domain::ids::TransitionId::from_uuid(self.transition_id),
            definition_version_id: crate::domain::ids::DefinitionVersionId::from_uuid(
                self.definition_version_id,
            ),
            transition_key: self.transition_key,
            display_name: self.display_name,
            source_node_id: crate::domain::ids::NodeId::from_uuid(self.source_node_id),
            target_node_id: crate::domain::ids::NodeId::from_uuid(self.target_node_id),
            transition_effect: self
                .transition_effect
                .parse::<crate::domain::enums::TransitionEffect>()
                .unwrap_or(crate::domain::enums::TransitionEffect::Advance),
            submission_schema: self.submission_schema,
            metadata: self.metadata,
            created_at: self.created_at,
        }
    }
}

/// Row type for locking and reading a workflow instance.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct InstanceLockRow {
    pub(super) workflow_instance_id: uuid::Uuid,
    pub(super) created_by_principal_id: uuid::Uuid,
    pub(super) definition_version_id: uuid::Uuid,
    pub(super) current_context_revision_id: uuid::Uuid,
    pub(super) current_node_visit_id: uuid::Uuid,
    pub(super) workflow_state_version: i32,
    pub(super) cancelled: bool,
}

/// Row type for reading a node visit with its node definition.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct CurrentVisitRow {
    pub(super) node_visit_id: uuid::Uuid,
    pub(super) node_id: uuid::Uuid,
    pub(super) node_type: String,
}

impl CurrentVisitRow {
    pub(super) fn node_type_enum(&self) -> NodeType {
        self.node_type
            .parse::<NodeType>()
            .unwrap_or(NodeType::NORMAL)
    }
}

/// Row type for reading current context revision metadata.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct CurrentContextRow {
    pub(super) context_revision_id: uuid::Uuid,
    pub(super) revision_number: i32,
    pub(super) payload_digest: String,
}

impl DraftNodeRow {
    pub(super) fn node_type_enum(&self) -> NodeType {
        self.node_type
            .parse::<NodeType>()
            .unwrap_or(NodeType::NORMAL)
    }

    pub(super) fn assignee_ref_type_enum(&self) -> AssigneeRefType {
        self.assignee_ref_type
            .parse::<AssigneeRefType>()
            .unwrap_or(AssigneeRefType::WorkflowCreator)
    }
}

/// Row type for reading a domain role binding.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct DomainRoleBindingRow {
    pub(super) binding_id: uuid::Uuid,
    pub(super) domain_id: uuid::Uuid,
    pub(super) principal_id: uuid::Uuid,
    pub(super) role_key: String,
    pub(super) enabled: bool,
}

/// Row type for principal existence / enabled check.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct PrincipalRow {
    pub(super) principal_id: uuid::Uuid,
    pub(super) enabled: bool,
}
