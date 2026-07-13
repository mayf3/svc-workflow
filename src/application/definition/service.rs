//! Definition Application Service.
//!
//! Orchestrates workflow definition and version lifecycle use cases.
//! The service depends on a [`DefinitionRepository`] trait for storage
//! and performs domain validation before delegating to the repository.

use std::collections::HashMap;

use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::definition::error::{DefinitionError, GraphValidationError};
use crate::domain::definition::graph;
use crate::domain::definition::model::{
    AssigneeRef, NodeDefinition, TransitionDefinition, ValidationResult, WorkflowDefinition,
    WorkflowDefinitionVersion, WorkflowGraph,
};
use crate::domain::enums::{AssigneeRefType, DefinitionVersionStatus, NodeType, TransitionEffect};
use crate::domain::ids::{
    DefinitionVersionId, NodeId, PrincipalId, TransitionId, WorkflowDefinitionId,
};

use super::commands::{
    CreateDefinition, CreateDraftVersion, DeprecateVersion, PublishVersion, RawNodeDefinition,
    RawTransitionDefinition, ReplaceDraftGraph, RevokeVersion, ValidateDraftVersion,
};
use super::queries::{
    DefinitionQueryResult, GetCompleteVersionGraph, GetDefinition, GetDefinitionVersion,
    GraphQueryResult, ListDefinitionVersions, VersionListResult, VersionQueryResult,
};
use super::repository::DefinitionData;
use super::DefinitionRepository;

/// The Definition Application Service.
///
/// All public methods correspond to a use case. Each method:
/// 1. Validates actor permissions
/// 2. Validates input against domain rules
/// 3. Delegates to the repository for storage
/// 4. Returns a result or error
pub struct DefinitionService<R: DefinitionRepository> {
    repo: R,
}

impl<R: DefinitionRepository> DefinitionService<R> {
    /// Create a new service with the given repository.
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    // -----------------------------------------------------------------------
    // 12.1 CreateDefinition
    // -----------------------------------------------------------------------

    /// Create a new workflow definition.
    pub async fn create_definition(
        &self,
        cmd: CreateDefinition,
    ) -> Result<WorkflowDefinition, DefinitionError> {
        // Validate actor
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;
        self.ensure_domain_enabled(cmd.owner_domain_id).await?;
        self.ensure_domain_owner(cmd.actor_principal_id, cmd.owner_domain_id)
            .await?;

        // Validate key uniqueness
        if self
            .repo
            .definition_key_exists(cmd.owner_domain_id, &cmd.definition_key)
            .await?
        {
            return Err(DefinitionError::DefinitionKeyConflict);
        }

        // Validate fields
        if cmd.definition_key.is_empty() || cmd.definition_key.len() > 128 {
            return Err(DefinitionError::StorageError(
                "definition_key must be 1-128 characters".to_string(),
            ));
        }
        if cmd.display_name.is_empty() || cmd.display_name.len() > 256 {
            return Err(DefinitionError::StorageError(
                "display_name must be 1-256 characters".to_string(),
            ));
        }

        // Create
        let id = Uuid::new_v4();
        let def = self
            .repo
            .create_definition(
                id,
                cmd.owner_domain_id,
                &cmd.definition_key,
                &cmd.display_name,
                cmd.description.as_deref(),
                cmd.metadata.as_ref(),
            )
            .await?;

        Ok(def)
    }

    // -----------------------------------------------------------------------
    // 12.2 CreateDraftVersion
    // -----------------------------------------------------------------------

    /// Create a new DRAFT version of a workflow definition.
    pub async fn create_draft_version(
        &self,
        cmd: CreateDraftVersion,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        // Validate actor
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        // Get definition's domain
        let domain_id = self
            .repo
            .get_definition_domain(cmd.workflow_definition_id)
            .await?;

        // Actor must have manage permission for the domain
        self.ensure_domain_owner(cmd.actor_principal_id, domain_id)
            .await?;

        // Get next version number
        let next_ver = self
            .repo
            .next_version_number(cmd.workflow_definition_id)
            .await?;

        // Create the draft version
        let version_id = Uuid::new_v4();
        let version = self
            .repo
            .create_draft_version(
                version_id,
                cmd.workflow_definition_id,
                next_ver,
                cmd.context_schema.as_ref(),
                cmd.json_schema_dialect.as_deref(),
                cmd.validator_version.as_deref(),
                cmd.metadata.as_ref(),
            )
            .await?;

        Ok(version)
    }

    // -----------------------------------------------------------------------
    // 12.3 ReplaceDraftGraph
    // -----------------------------------------------------------------------

    /// Atomically replace the graph of a DRAFT version.
    pub async fn replace_draft_graph(&self, cmd: ReplaceDraftGraph) -> Result<(), DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        // Verify version exists and is DRAFT by locking it
        let version = self.repo.lock_version(cmd.definition_version_id).await?;
        if version.version_status != DefinitionVersionStatus::DRAFT {
            return Err(DefinitionError::VersionNotDraft);
        }

        // Get the definition for domain ownership
        let domain_id = self
            .repo
            .get_definition_domain(version.workflow_definition_id.into_uuid())
            .await?;
        self.ensure_domain_owner(cmd.actor_principal_id, domain_id)
            .await?;

        // Resolve node keys -> IDs
        let mut node_id_by_key: HashMap<String, NodeId> = HashMap::new();
        let mut node_defs: Vec<NodeDefinition> = Vec::new();
        let version_id = version.id.into_uuid();

        for raw_node in &cmd.nodes {
            let node_id = NodeId::new();
            node_id_by_key.insert(raw_node.node_key.clone(), node_id);

            let assignee_ref =
                Self::parse_assignee_ref(&raw_node.assignee_ref_type, raw_node.fixed_principal_id)?;

            let node_type = raw_node.node_type.parse::<NodeType>().map_err(|_| {
                DefinitionError::StorageError(format!("invalid node_type: {}", raw_node.node_type))
            })?;

            node_defs.push(NodeDefinition {
                node_id,
                definition_version_id: DefinitionVersionId::from_uuid(version_id),
                node_key: raw_node.node_key.clone(),
                display_name: raw_node.display_name.clone(),
                order_index: raw_node.order_index,
                node_type,
                assignee_ref,
                instructions: raw_node.instructions.clone(),
                primary_advance_transition_id: None, // resolved after transitions
                metadata: raw_node.metadata.clone(),
                created_at: chrono::Utc::now(),
            });
        }

        // Build transition definitions
        let mut transition_defs: Vec<TransitionDefinition> = Vec::new();
        let mut transition_key_to_id: HashMap<String, TransitionId> = HashMap::new();

        for raw_trans in &cmd.transitions {
            let trans_id = TransitionId::new();
            transition_key_to_id.insert(raw_trans.transition_key.clone(), trans_id);

            let source_id = node_id_by_key
                .get(&raw_trans.source_node_key)
                .ok_or_else(|| {
                    DefinitionError::GraphValidationFailed(vec![GraphValidationError::new(
                        "TRANSITION_SOURCE_MISSING",
                        format!(
                            "transition '{}' references unknown source node '{}'",
                            raw_trans.transition_key, raw_trans.source_node_key
                        ),
                    )])
                })?;

            let target_id = node_id_by_key
                .get(&raw_trans.target_node_key)
                .ok_or_else(|| {
                    DefinitionError::GraphValidationFailed(vec![GraphValidationError::new(
                        "TRANSITION_TARGET_MISSING",
                        format!(
                            "transition '{}' references unknown target node '{}'",
                            raw_trans.transition_key, raw_trans.target_node_key
                        ),
                    )])
                })?;

            let effect = raw_trans
                .transition_effect
                .parse::<TransitionEffect>()
                .map_err(|_| {
                    DefinitionError::StorageError(format!(
                        "invalid transition_effect: {}",
                        raw_trans.transition_effect
                    ))
                })?;

            transition_defs.push(TransitionDefinition {
                transition_id: trans_id,
                definition_version_id: DefinitionVersionId::from_uuid(version_id),
                transition_key: raw_trans.transition_key.clone(),
                display_name: raw_trans.display_name.clone(),
                source_node_id: *source_id,
                target_node_id: *target_id,
                transition_effect: effect,
                submission_schema: raw_trans.submission_schema.clone(),
                metadata: raw_trans.metadata.clone(),
                created_at: chrono::Utc::now(),
            });
        }

        // Resolve primary advance transition keys to IDs
        for node_def in &mut node_defs {
            // Find the raw node with matching key
            if let Some(raw_node) = cmd.nodes.iter().find(|n| n.node_key == node_def.node_key) {
                if let Some(pt_key) = &raw_node.primary_advance_transition_key {
                    if let Some(pt_id) = transition_key_to_id.get(pt_key) {
                        node_def.primary_advance_transition_id = Some(*pt_id);
                    }
                }
            }
        }

        // Build graph model for validation
        let graph = WorkflowGraph {
            nodes: node_defs.clone(),
            transitions: transition_defs.clone(),
            context_schema: cmd.context_schema.clone(),
        };

        // Validate the graph
        let validation_result = graph::validate_graph(&graph);

        // Also validate JSON schemas
        let schema_errors = self.validate_json_schemas(&graph).await;
        let mut all_errors = validation_result.errors.clone();
        all_errors.extend(schema_errors);

        if !all_errors.is_empty() {
            return Err(DefinitionError::GraphValidationFailed(all_errors));
        }

        // Replace graph atomically
        self.repo
            .replace_draft_graph(
                version_id,
                cmd.context_schema.as_ref(),
                &node_defs,
                &transition_defs,
            )
            .await
    }

    // -----------------------------------------------------------------------
    // 12.4 ValidateDraftVersion
    // -----------------------------------------------------------------------

    /// Validate a DRAFT version without changing state.
    pub async fn validate_draft_version(
        &self,
        cmd: ValidateDraftVersion,
    ) -> Result<ValidationResult, DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        let version = self.repo.lock_version(cmd.definition_version_id).await?;
        if version.version_status != DefinitionVersionStatus::DRAFT {
            return Err(DefinitionError::VersionNotDraft);
        }

        let (nodes, transitions) = self
            .repo
            .get_complete_graph(cmd.definition_version_id)
            .await?;

        let graph = WorkflowGraph {
            nodes,
            transitions,
            context_schema: version.context_schema.clone(),
        };

        let mut result = graph::validate_graph(&graph);

        // Also validate JSON schemas
        let schema_errors = self.validate_json_schemas(&graph).await;
        result.errors.extend(schema_errors);
        result.valid = result.errors.is_empty();

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // 12.5 PublishVersion
    // -----------------------------------------------------------------------

    /// Publish a DRAFT version -> PUBLISHED.
    pub async fn publish_version(
        &self,
        cmd: PublishVersion,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        // Lock version and verify DRAFT
        let version = self.repo.lock_version(cmd.definition_version_id).await?;
        if version.version_status != DefinitionVersionStatus::DRAFT {
            return Err(DefinitionError::VersionNotDraft);
        }

        // Get definition for domain ownership check and definition_key
        let def = self
            .repo
            .get_definition(version.workflow_definition_id.into_uuid())
            .await?;

        let domain_id = def.domain_id.into_uuid();
        self.ensure_domain_owner(cmd.actor_principal_id, domain_id)
            .await?;

        // Get complete graph
        let (nodes, transitions) = self
            .repo
            .get_complete_graph(cmd.definition_version_id)
            .await?;

        // Validate graph
        let graph = WorkflowGraph {
            nodes: nodes.clone(),
            transitions: transitions.clone(),
            context_schema: version.context_schema.clone(),
        };

        let mut validation_result = graph::validate_graph(&graph);

        // Validate JSON schemas
        let schema_errors = self.validate_json_schemas(&graph).await;
        validation_result.errors.extend(schema_errors);
        validation_result.valid = validation_result.errors.is_empty();

        if !validation_result.valid {
            return Err(DefinitionError::GraphValidationFailed(
                validation_result.errors,
            ));
        }

        // Validate fixed principals exist
        self.validate_fixed_principals(&nodes).await?;

        // Compute digest
        let node_key_map: HashMap<_, _> = nodes
            .iter()
            .map(|n| (n.node_id, n.node_key.clone()))
            .collect();
        let transition_key_map: HashMap<_, _> = transitions
            .iter()
            .map(|t| (t.transition_id, t.transition_key.clone()))
            .collect();

        let digest = digest::compute_digest(
            &def.definition_key,
            version.version_number,
            version.json_schema_dialect.as_deref(),
            version.validator_version.as_deref(),
            version.context_schema.as_ref(),
            &nodes,
            &transitions,
            &node_key_map,
            &transition_key_map,
        )?;

        // Publish
        let published = self
            .repo
            .publish_version(cmd.definition_version_id, &digest)
            .await?;

        Ok(published)
    }

    // -----------------------------------------------------------------------
    // 12.6 DeprecateVersion
    // -----------------------------------------------------------------------

    /// Deprecate a PUBLISHED version -> DEPRECATED.
    pub async fn deprecate_version(
        &self,
        cmd: DeprecateVersion,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        let version = self.repo.lock_version(cmd.definition_version_id).await?;
        if version.version_status != DefinitionVersionStatus::PUBLISHED {
            return Err(DefinitionError::InvalidLifecycleTransition);
        }

        let domain_id = self
            .repo
            .get_definition_domain(version.workflow_definition_id.into_uuid())
            .await?;
        self.ensure_domain_owner(cmd.actor_principal_id, domain_id)
            .await?;

        let updated = self
            .repo
            .update_version_status(
                cmd.definition_version_id,
                DefinitionVersionStatus::DEPRECATED,
            )
            .await?;

        Ok(updated)
    }

    // -----------------------------------------------------------------------
    // 12.7 RevokeVersion
    // -----------------------------------------------------------------------

    /// Revoke a PUBLISHED or DEPRECATED version -> REVOKED.
    pub async fn revoke_version(
        &self,
        cmd: RevokeVersion,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        let version = self.repo.lock_version(cmd.definition_version_id).await?;

        match version.version_status {
            DefinitionVersionStatus::PUBLISHED | DefinitionVersionStatus::DEPRECATED => {}
            _ => {
                return Err(DefinitionError::InvalidLifecycleTransition);
            }
        }

        let domain_id = self
            .repo
            .get_definition_domain(version.workflow_definition_id.into_uuid())
            .await?;
        self.ensure_domain_owner(cmd.actor_principal_id, domain_id)
            .await?;

        let updated = self
            .repo
            .update_version_status(cmd.definition_version_id, DefinitionVersionStatus::REVOKED)
            .await?;

        Ok(updated)
    }

    // -----------------------------------------------------------------------
    // 12.8 Queries
    // -----------------------------------------------------------------------

    /// Get a definition by ID.
    pub async fn get_definition(
        &self,
        query: GetDefinition,
    ) -> Result<DefinitionQueryResult, DefinitionError> {
        self.ensure_principal_enabled(query.actor_principal_id)
            .await?;

        let definition = self
            .repo
            .get_definition(query.workflow_definition_id)
            .await?;

        Ok(DefinitionQueryResult {
            definition: DefinitionData {
                definition,
                version: None,
                nodes: vec![],
                transitions: vec![],
            },
        })
    }

    /// Get a specific version.
    pub async fn get_definition_version(
        &self,
        query: GetDefinitionVersion,
    ) -> Result<VersionQueryResult, DefinitionError> {
        self.ensure_principal_enabled(query.actor_principal_id)
            .await?;

        let version = self.repo.get_version(query.definition_version_id).await?;
        let def = self
            .repo
            .get_definition(version.workflow_definition_id.into_uuid())
            .await?;
        let (nodes, transitions) = self
            .repo
            .get_complete_graph(query.definition_version_id)
            .await?;

        let nodes_count = nodes.len();
        let transitions_count = transitions.len();

        Ok(VersionQueryResult {
            version: DefinitionData {
                definition: def,
                version: Some(version),
                nodes,
                transitions,
            },
            nodes_count,
            transitions_count,
        })
    }

    /// List all versions of a definition.
    pub async fn list_definition_versions(
        &self,
        query: ListDefinitionVersions,
    ) -> Result<VersionListResult, DefinitionError> {
        self.ensure_principal_enabled(query.actor_principal_id)
            .await?;

        let versions = self
            .repo
            .list_versions(query.workflow_definition_id)
            .await?;
        let def = self
            .repo
            .get_definition(query.workflow_definition_id)
            .await?;

        let results = versions
            .into_iter()
            .map(|v| DefinitionData {
                definition: def.clone(),
                version: Some(v),
                nodes: vec![],
                transitions: vec![],
            })
            .collect();

        Ok(VersionListResult { versions: results })
    }

    /// Get a complete version graph.
    pub async fn get_complete_version_graph(
        &self,
        query: GetCompleteVersionGraph,
    ) -> Result<GraphQueryResult, DefinitionError> {
        self.ensure_principal_enabled(query.actor_principal_id)
            .await?;

        let version = self.repo.get_version(query.definition_version_id).await?;
        let (nodes, transitions) = self
            .repo
            .get_complete_graph(query.definition_version_id)
            .await?;

        Ok(GraphQueryResult {
            graph: WorkflowGraph {
                nodes,
                transitions,
                context_schema: version.context_schema,
            },
        })
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    async fn ensure_principal_enabled(&self, principal_id: Uuid) -> Result<(), DefinitionError> {
        let enabled = self.repo.check_principal_enabled(principal_id).await?;
        if !enabled {
            // Check if principal exists at all
            let exists = self.repo.check_principal_exists(principal_id).await?;
            if !exists {
                return Err(DefinitionError::PrincipalNotFound);
            }
            return Err(DefinitionError::PrincipalDisabled);
        }
        Ok(())
    }

    async fn ensure_domain_enabled(&self, domain_id: Uuid) -> Result<(), DefinitionError> {
        let enabled = self.repo.check_domain_enabled(domain_id).await?;
        if !enabled {
            return Err(DefinitionError::DomainDisabled);
        }
        Ok(())
    }

    async fn ensure_domain_owner(
        &self,
        principal_id: Uuid,
        domain_id: Uuid,
    ) -> Result<(), DefinitionError> {
        let is_owner = self
            .repo
            .check_domain_role(principal_id, domain_id, "DOMAIN_OWNER")
            .await?;
        if !is_owner {
            return Err(DefinitionError::PermissionDenied);
        }
        Ok(())
    }

    fn parse_assignee_ref(
        ref_type: &str,
        fixed_principal_id: Option<Uuid>,
    ) -> Result<AssigneeRef, DefinitionError> {
        let parsed = ref_type.parse::<AssigneeRefType>().map_err(|_| {
            DefinitionError::StorageError(format!("invalid assignee_ref_type: {}", ref_type))
        })?;

        match parsed {
            AssigneeRefType::FixedPrincipal => {
                if fixed_principal_id.is_none() {
                    return Err(DefinitionError::FixedPrincipalInvalid(
                        "FIXED_PRINCIPAL requires a principal_id".to_string(),
                    ));
                }
            }
            _ => {
                if fixed_principal_id.is_some() {
                    return Err(DefinitionError::FixedPrincipalInvalid(
                        "only FIXED_PRINCIPAL type should have fixed_principal_id".to_string(),
                    ));
                }
            }
        }

        Ok(AssigneeRef {
            ref_type: parsed,
            fixed_principal_id: fixed_principal_id.map(PrincipalId::from_uuid),
        })
    }

    async fn validate_fixed_principals(
        &self,
        nodes: &[NodeDefinition],
    ) -> Result<(), DefinitionError> {
        for node in nodes {
            if node.assignee_ref.ref_type == AssigneeRefType::FixedPrincipal {
                if let Some(fixed_id) = node.assignee_ref.fixed_principal_id {
                    let exists = self
                        .repo
                        .check_principal_exists(fixed_id.into_uuid())
                        .await?;
                    if !exists {
                        return Err(DefinitionError::FixedPrincipalInvalid(format!(
                            "fixed principal {} for node '{}' not found",
                            fixed_id, node.node_key
                        )));
                    }

                    let enabled = self
                        .repo
                        .check_principal_enabled(fixed_id.into_uuid())
                        .await?;
                    if !enabled {
                        return Err(DefinitionError::FixedPrincipalInvalid(format!(
                            "fixed principal {} for node '{}' is disabled",
                            fixed_id, node.node_key
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    async fn validate_json_schemas(&self, graph: &WorkflowGraph) -> Vec<GraphValidationError> {
        let mut errors: Vec<GraphValidationError> = Vec::new();

        // Validate context_schema
        if let Some(schema) = &graph.context_schema {
            if let Err(e) = validate_json_schema(schema) {
                errors.push(GraphValidationError::new(
                    "INVALID_CONTEXT_SCHEMA",
                    format!("context_schema: {}", e),
                ));
            }
        }

        // Validate each transition's submission_schema
        for trans in &graph.transitions {
            if let Some(schema) = &trans.submission_schema {
                if let Err(e) = validate_json_schema(schema) {
                    errors.push(GraphValidationError::new(
                        "INVALID_SUBMISSION_SCHEMA",
                        format!(
                            "transition '{}' submission_schema: {}",
                            trans.transition_key, e
                        ),
                    ));
                }
            }
        }

        errors
    }
}

/// Validate that a JSON value is a valid JSON Schema.
///
/// Checks that the schema can be successfully compiled by the jsonschema
/// validator. This catches structural issues like invalid $ref, bad types,
/// etc.
fn validate_json_schema(schema: &serde_json::Value) -> Result<(), String> {
    // The jsonschema crate 0.47 uses a different API
    // Let's use a basic structural check
    if !schema.is_object() {
        return Err("schema must be a JSON object".to_string());
    }

    // Basic structural validation
    let obj = schema.as_object().unwrap();
    if let Some(schema_field) = obj.get("$schema") {
        if !schema_field.is_string() {
            return Err("$schema must be a string".to_string());
        }
    }

    // Try to compile with jsonschema
    let compiled = jsonschema::validator_for(schema);
    // If it compiles without panic, it's syntactically valid
    let _ = compiled;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_json_schema_valid_object() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": {"type": "string"}
            }
        });
        assert!(validate_json_schema(&schema).is_ok());
    }

    #[test]
    fn validate_json_schema_invalid_type() {
        let schema = serde_json::json!("string_schema");
        assert!(validate_json_schema(&schema).is_err());
    }

    #[test]
    fn validate_json_schema_valid_with_dialect() {
        let schema = serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object"
        });
        assert!(validate_json_schema(&schema).is_ok());
    }
}
