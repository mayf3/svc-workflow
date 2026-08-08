//! Definition version lookup inside the atomic creation transaction.
//!
//! Reads the definition version, validates domain ownership, status,
//! and resolves the initial DRAFT node configuration.

use sqlx::{Postgres, Transaction};

use crate::domain::workflow_instance::errors::CreateWorkflowInstanceError;

use super::row_types::*;

/// Result of a definition version lookup inside the transaction.
pub(super) struct DefinitionVersionInfo {
    pub version_status: crate::domain::enums::DefinitionVersionStatus,
    /// 1 = Legacy, 2 = Minimal (semantic dispatch source of truth).
    pub semantic_model_version: i16,
    pub definition_digest: Option<String>,
    pub context_schema: Option<serde_json::Value>,
    pub json_schema_dialect: Option<String>,
    pub version_number: i32,
}

/// Result of the initial DRAFT node lookup.
pub(super) struct DraftNodeInfo {
    pub node_id: uuid::Uuid,
    pub assignee_ref_type: crate::domain::enums::AssigneeRefType,
    pub fixed_principal_id: Option<uuid::Uuid>,
    pub assignee_input_key: Option<String>,
}

/// Look up the definition version, lock it, and validate basic constraints.
///
/// Steps:
/// 1. Lock the version row with FOR UPDATE
/// 2. Verify the version exists
/// 3. Verify the version belongs to the specified domain
/// 4. Verify the version is PUBLISHED
pub(super) async fn lock_and_validate_version(
    tx: &mut Transaction<'_, Postgres>,
    definition_version_id: uuid::Uuid,
    domain_id: uuid::Uuid,
) -> Result<DefinitionVersionInfo, CreateWorkflowInstanceError> {
    // Lock and read version
    let version: Option<DefinitionVersionStatusRow> = sqlx::query_as(
        "SELECT definition_version_id, workflow_definition_id, version_number, \
         version_status::TEXT AS version_status, semantic_model_version, \
         definition_digest, json_schema_dialect, validator_version, context_schema \
         FROM workflow_definition_versions \
         WHERE definition_version_id = $1 FOR UPDATE",
    )
    .bind(definition_version_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    let version = version.ok_or(CreateWorkflowInstanceError::DefinitionVersionNotFound)?;

    // Verify version belongs to the specified domain
    let def: Option<WorkflowDefinitionDomainRow> = sqlx::query_as(
        "SELECT workflow_definition_id, domain_id FROM workflow_definitions \
         WHERE workflow_definition_id = $1",
    )
    .bind(version.workflow_definition_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    let def = def.ok_or(CreateWorkflowInstanceError::DefinitionVersionNotFound)?;

    if def.domain_id != domain_id {
        return Err(CreateWorkflowInstanceError::CrossDomainViolation);
    }

    // Verify PUBLISHED status
    let status = version.version_status_enum();
    if status != crate::domain::enums::DefinitionVersionStatus::PUBLISHED {
        return Err(CreateWorkflowInstanceError::VersionNotPublished);
    }

    Ok(DefinitionVersionInfo {
        version_status: status,
        semantic_model_version: version.semantic_model_version,
        definition_digest: version.definition_digest,
        context_schema: version.context_schema,
        json_schema_dialect: version.json_schema_dialect,
        version_number: version.version_number,
    })
}

/// Read the unique DRAFT node from a definition version.
///
/// Returns an error if there is not exactly one DRAFT node.
pub(super) async fn read_draft_node(
    tx: &mut Transaction<'_, Postgres>,
    definition_version_id: uuid::Uuid,
) -> Result<DraftNodeInfo, CreateWorkflowInstanceError> {
    let nodes: Vec<DraftNodeRow> = sqlx::query_as(
        "SELECT node_id, node_type::TEXT AS node_type, \
         assignee_ref_type::TEXT AS assignee_ref_type, fixed_principal_id, assignee_input_key \
         FROM workflow_node_definitions \
         WHERE definition_version_id = $1 AND node_type = 'DRAFT'",
    )
    .bind(definition_version_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    if nodes.is_empty() {
        return Err(CreateWorkflowInstanceError::InternalConsistency(
            "definition version has no DRAFT node".to_string(),
        ));
    }

    if nodes.len() > 1 {
        return Err(CreateWorkflowInstanceError::InternalConsistency(
            "definition version has multiple DRAFT nodes".to_string(),
        ));
    }

    let node = &nodes[0];

    if node.node_type_enum() == crate::domain::enums::NodeType::TERMINAL {
        return Err(CreateWorkflowInstanceError::InternalConsistency(
            "DRAFT node has TERMINAL type (impossible state)".to_string(),
        ));
    }

    Ok(DraftNodeInfo {
        node_id: node.node_id,
        assignee_ref_type: node.assignee_ref_type_enum(),
        fixed_principal_id: node.fixed_principal_id,
        assignee_input_key: node.assignee_input_key.clone(),
    })
}

/// Read the complete published graph of a definition version as domain
/// types (nodes + transitions). Used to run the full Minimal validator
/// before any V2 instance state is written.
pub(super) async fn read_full_graph(
    tx: &mut Transaction<'_, Postgres>,
    definition_version_id: uuid::Uuid,
) -> Result<crate::domain::definition::model::WorkflowGraph, CreateWorkflowInstanceError> {
    let nodes: Vec<crate::domain::definition::model::NodeDefinition> = sqlx::query_as::<_, super::row_types::GraphNodeRow>(
        "SELECT node_id, definition_version_id, node_key, display_name, order_index, \
         node_type::TEXT AS node_type, assignee_ref_type::TEXT AS assignee_ref_type, \
         fixed_principal_id, assignee_input_key, instructions, primary_advance_transition_id, \
         metadata, created_at \
         FROM workflow_node_definitions WHERE definition_version_id = $1 ORDER BY order_index",
    )
    .bind(definition_version_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?
    .into_iter()
    .map(|r| r.into_domain())
    .collect();

    let transitions: Vec<crate::domain::definition::model::TransitionDefinition> = sqlx::query_as::<_, super::row_types::GraphTransitionRow>(
        "SELECT transition_id, definition_version_id, transition_key, display_name, \
         source_node_id, target_node_id, transition_effect::TEXT AS transition_effect, \
         submission_schema, metadata, created_at \
         FROM workflow_transition_definitions WHERE definition_version_id = $1 ORDER BY transition_key",
    )
    .bind(definition_version_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?
    .into_iter()
    .map(|r| r.into_domain())
    .collect();

    Ok(crate::domain::definition::model::WorkflowGraph {
        nodes,
        transitions,
        context_schema: None,
    })
}

/// Read the unique Minimal (V2) entry TASK from a definition version.
///
/// The entry is the unique TASK (NORMAL node) with no incoming ADVANCE edge
/// in the ADVANCE graph — never inferred from DRAFT, orderIndex, or primary
/// advance. The Minimal validator guarantees exactly one; the runtime fails
/// closed if the invariant is violated (e.g. a V2 version that bypassed the
/// validator).
pub(super) async fn read_minimal_entry_node(
    tx: &mut Transaction<'_, Postgres>,
    definition_version_id: uuid::Uuid,
) -> Result<DraftNodeInfo, CreateWorkflowInstanceError> {
    let nodes: Vec<DraftNodeRow> = sqlx::query_as(
        "SELECT node_id, node_type::TEXT AS node_type, \
         assignee_ref_type::TEXT AS assignee_ref_type, fixed_principal_id, assignee_input_key \
         FROM workflow_node_definitions \
         WHERE definition_version_id = $1 AND node_type = 'NORMAL' \
           AND node_id NOT IN ( \
               SELECT target_node_id FROM workflow_transition_definitions \
               WHERE definition_version_id = $1 AND transition_effect = 'ADVANCE' \
           )",
    )
    .bind(definition_version_id)
    .bind(definition_version_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    if nodes.is_empty() {
        return Err(CreateWorkflowInstanceError::InternalConsistency(
            "Minimal (V2) definition version has no entry TASK".to_string(),
        ));
    }
    if nodes.len() > 1 {
        return Err(CreateWorkflowInstanceError::InternalConsistency(format!(
            "Minimal (V2) definition version has {} entry TASKs (exactly one required)",
            nodes.len()
        )));
    }

    let node = &nodes[0];
    if node.node_type_enum() == crate::domain::enums::NodeType::TERMINAL {
        return Err(CreateWorkflowInstanceError::InternalConsistency(
            "Minimal entry node has TERMINAL type (impossible state)".to_string(),
        ));
    }

    Ok(DraftNodeInfo {
        node_id: node.node_id,
        assignee_ref_type: node.assignee_ref_type_enum(),
        fixed_principal_id: node.fixed_principal_id,
        assignee_input_key: node.assignee_input_key.clone(),
    })
}

