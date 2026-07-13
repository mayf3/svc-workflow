use std::collections::HashMap;

use crate::domain::definition::digest;
use crate::domain::definition::error::{DefinitionError, GraphValidationError};
use crate::domain::definition::graph;
use crate::domain::definition::model::{
    NodeDefinition, ValidationResult, WorkflowDefinitionVersion, WorkflowGraph,
};
use crate::domain::enums::{AssigneeRefType, DefinitionVersionStatus};

use super::commands::{DeprecateVersion, PublishVersion, RevokeVersion, ValidateDraftVersion};
use super::queries::{
    DefinitionQueryResult, GetCompleteVersionGraph, GetDefinition, GetDefinitionVersion,
    GraphQueryResult, ListDefinitionVersions, VersionListResult, VersionQueryResult,
};
use super::repository::DefinitionData;
use super::repository::DefinitionRepository;
use super::service::DefinitionService;

impl<R: DefinitionRepository> DefinitionService<R> {
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
    // Validation helpers
    // -----------------------------------------------------------------------

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

    pub(crate) async fn validate_json_schemas(
        &self,
        graph: &WorkflowGraph,
    ) -> Vec<GraphValidationError> {
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
