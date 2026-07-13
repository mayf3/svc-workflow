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

        // H-5 + M-4: Domain authorization and enabled check
        let domain_id = self
            .repo
            .get_definition_domain(version.workflow_definition_id.into_uuid())
            .await?;
        self.ensure_domain_enabled(domain_id).await?;
        self.ensure_domain_owner(cmd.actor_principal_id, domain_id)
            .await?;

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
    ///
    /// B-1: The actual DB write happens inside `atomic_publish` which
    /// holds the version row lock across digest consistency check and
    /// status update in a single transaction, serializing against any
    /// concurrent ReplaceDraftGraph.
    pub async fn publish_version(
        &self,
        cmd: PublishVersion,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        // Lock version (autocommit — gives us a consistent snapshot for validation)
        // B-1: atomic_publish will re-read inside a transaction and verify consistency
        let version = self.repo.lock_version(cmd.definition_version_id).await?;
        if version.version_status != DefinitionVersionStatus::DRAFT {
            return Err(DefinitionError::VersionNotDraft);
        }

        // Get definition for definition_key
        let def = self
            .repo
            .get_definition(version.workflow_definition_id.into_uuid())
            .await?;

        // Get complete graph for validation
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

        // B-1: Atomic publish inside a single transaction.
        // Locks the version row, re-reads graph, re-computes digest,
        // and writes status atomically.  If a concurrent ReplaceDraftGraph
        // changed the graph since we read it, the digest inside the tx
        // won't match `digest` and we get ConcurrentModification (retry).
        let published = self
            .repo
            .atomic_publish(cmd.definition_version_id, cmd.actor_principal_id, &digest)
            .await?;

        Ok(published)
    }

    // -----------------------------------------------------------------------
    // 12.6 DeprecateVersion
    // -----------------------------------------------------------------------

    /// Deprecate a PUBLISHED version -> DEPRECATED.
    ///
    /// B-1: Uses atomic_deprecate which locks the version row across
    /// all checks and writes in a single transaction.
    pub async fn deprecate_version(
        &self,
        cmd: DeprecateVersion,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        let updated = self
            .repo
            .atomic_deprecate(cmd.definition_version_id, cmd.actor_principal_id)
            .await?;

        Ok(updated)
    }

    // -----------------------------------------------------------------------
    // 12.7 RevokeVersion
    // -----------------------------------------------------------------------

    /// Revoke a PUBLISHED or DEPRECATED version -> REVOKED.
    ///
    /// B-1: Uses atomic_revoke which locks the version row across
    /// all checks and writes in a single transaction.
    pub async fn revoke_version(
        &self,
        cmd: RevokeVersion,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        let updated = self
            .repo
            .atomic_revoke(cmd.definition_version_id, cmd.actor_principal_id)
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

        // H-5: Domain authorization — caller must be domain owner or member
        self.ensure_domain_owner(query.actor_principal_id, definition.domain_id.into_uuid())
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

        // H-5: Domain authorization
        self.ensure_domain_owner(query.actor_principal_id, def.domain_id.into_uuid())
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

        let def = self
            .repo
            .get_definition(query.workflow_definition_id)
            .await?;

        // H-5: Domain authorization — check before listing versions
        self.ensure_domain_owner(query.actor_principal_id, def.domain_id.into_uuid())
            .await?;

        let versions = self
            .repo
            .list_versions(query.workflow_definition_id)
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
        let def = self
            .repo
            .get_definition(version.workflow_definition_id.into_uuid())
            .await?;

        // H-5: Domain authorization
        self.ensure_domain_owner(query.actor_principal_id, def.domain_id.into_uuid())
            .await?;

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
/// Performs two checks:
/// 1. Recursively inspects all `$ref`, `$dynamicRef`, and `$recursiveRef` values
///    to reject external references (http://, https://, file://, relative paths).
///    Only local fragment references starting with `#` are allowed.
/// 2. Compiles the schema with `jsonschema::validator_for`, propagating any
///    compilation error (invalid keywords, unresolved local refs, etc.).
///
/// Compilation failure is returned as a typed error with the schema location.
fn validate_json_schema(schema: &serde_json::Value) -> Result<(), String> {
    if !schema.is_object() {
        return Err("schema must be a JSON object".to_string());
    }

    // Recursively check for external references
    check_external_refs(schema)?;

    // Actually compile the schema to verify it's structurally valid
    jsonschema::validator_for(schema).map_err(|e| format!("schema failed to compile: {}", e))?;

    Ok(())
}

/// Recursively check a schema for external `$ref`, `$dynamicRef`, `$recursiveRef` values.
///
/// Only local fragment references starting with `#/` or bare `#` are allowed.
/// Rejects:
/// - `http://` / `https://`
/// - `file://`
/// - Relative paths (not starting with `#`)
fn check_external_refs(value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::Object(map) => {
            // Check ref-like keys
            for ref_key in ["$ref", "$dynamicRef", "$recursiveRef"] {
                if let Some(ref_val) = map.get(ref_key) {
                    if let Some(ref_str) = ref_val.as_str() {
                        if !ref_str.starts_with('#') {
                            return Err(format!(
                                "external {} '{}' is not allowed; only local fragment refs (#/...) are permitted",
                                ref_key, ref_str
                            ));
                        }
                    }
                }
            }
            // Recurse into all properties
            for val in map.values() {
                check_external_refs(val)?;
            }
            Ok(())
        }
        serde_json::Value::Array(arr) => {
            for val in arr {
                check_external_refs(val)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
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

    #[test]
    fn validate_json_schema_rejects_https_ref() {
        let schema = serde_json::json!({
            "$ref": "https://example.com/schema.json"
        });
        let err = validate_json_schema(&schema).unwrap_err();
        assert!(err.contains("https://"), "got: {}", err);
        assert!(err.contains("external"), "got: {}", err);
    }

    #[test]
    fn validate_json_schema_rejects_file_ref() {
        let schema = serde_json::json!({
            "$ref": "file:///etc/passwd"
        });
        let err = validate_json_schema(&schema).unwrap_err();
        assert!(err.contains("file://"), "got: {}", err);
    }

    #[test]
    fn validate_json_schema_rejects_relative_ref() {
        let schema = serde_json::json!({
            "$ref": "../other/schema.json"
        });
        let err = validate_json_schema(&schema).unwrap_err();
        assert!(err.contains("../other"), "got: {}", err);
    }

    #[test]
    fn validate_json_schema_allows_local_fragment() {
        let schema = serde_json::json!({
            "$defs": {
                "User": {"type": "object"}
            },
            "$ref": "#/$defs/User"
        });
        assert!(
            validate_json_schema(&schema).is_ok(),
            "local fragment should pass"
        );
    }

    #[test]
    fn validate_json_schema_allows_bare_hash() {
        let schema = serde_json::json!({
            "$ref": "#"
        });
        assert!(validate_json_schema(&schema).is_ok(), "bare # should pass");
    }

    #[test]
    fn validate_json_schema_rejects_nested_external_ref() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "user": {"$ref": "https://example.com/user.json"}
            }
        });
        let err = validate_json_schema(&schema).unwrap_err();
        assert!(err.contains("https://"), "got: {}", err);
    }

    #[test]
    fn validate_json_schema_rejects_dynamic_ref_external() {
        let schema = serde_json::json!({
            "$dynamicRef": "https://example.com/dynamic"
        });
        let err = validate_json_schema(&schema).unwrap_err();
        assert!(err.contains("https://"), "got: {}", err);
    }

    #[test]
    fn validate_json_schema_rejects_invalid_keyword_structure() {
        // type: "object" but properties contains a non-object entry — some schemas
        // with genuinely invalid keyword values will fail compilation
        let schema = serde_json::json!({
            "type": "object",
            "properties": "not-an-object"
        });
        let err = validate_json_schema(&schema);
        assert!(err.is_err(), "invalid keyword structure should fail");
    }

    #[test]
    fn check_external_refs_empty_object() {
        let val = serde_json::json!({});
        assert!(check_external_refs(&val).is_ok());
    }

    #[test]
    fn check_external_refs_nested_local_ref() {
        let val = serde_json::json!({
            "properties": {
                "user": {"$ref": "#/$defs/User"}
            },
            "$defs": {
                "User": {"type": "object"}
            }
        });
        assert!(check_external_refs(&val).is_ok());
    }
}
