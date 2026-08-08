//! PublishVersion application flow.
//!
//! Handles the publish workflow: schema/graph validation, digest precomputation,
//! and delegation to the repository's atomic_publish.

use std::collections::HashMap;

use crate::domain::definition::digest;
use crate::domain::definition::error::DefinitionError;
use crate::domain::definition::graph;
use crate::domain::definition::model::WorkflowGraph;
use crate::domain::enums::DefinitionVersionStatus;

use super::super::commands::PublishVersion;
use super::super::repository::DefinitionRepository;
use super::super::service::DefinitionService;

impl<R: DefinitionRepository> DefinitionService<R> {
    /// Publish a DRAFT version -> PUBLISHED.
    ///
    /// B-1: The actual DB write happens inside `atomic_publish` which
    /// holds the version row lock across digest consistency check and
    /// status update in a single transaction, serializing against any
    /// concurrent ReplaceDraftGraph.
    pub async fn publish_version(
        &self,
        cmd: PublishVersion,
    ) -> Result<crate::domain::definition::model::WorkflowDefinitionVersion, DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        // Lock version (autocommit — gives us a consistent snapshot for validation)
        let version = self.repo.lock_version(cmd.definition_version_id).await?;
        if version.version_status != DefinitionVersionStatus::DRAFT {
            return Err(DefinitionError::VersionNotDraft);
        }

        // Get definition for definition_key
        let def = self
            .repo
            .get_definition(version.workflow_definition_id.into_uuid())
            .await?;

        // Check that definition is not archived
        if def.archived {
            return Err(DefinitionError::DefinitionArchived);
        }

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

        // Semantic-model dispatch for publication validation: Legacy (1)
        // uses the Legacy validator, Minimal (2) the Minimal validator.
        // Graph legality remains the caller's responsibility; this is the
        // publication governance check, not runtime validation.
        let mut validation_result = match version.semantic_model_version {
            crate::domain::definition::model::SemanticModelVersion::Legacy => {
                graph::validate_graph(&graph)
            }
            crate::domain::definition::model::SemanticModelVersion::Minimal => {
                graph::validate_minimal_graph(&graph)
            }
        };

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

        let computed_digest = digest::compute_digest(
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
        // expected_revision is verified inside the transaction alongside
        // the digest consistency check, eliminating any race window.
        let published = self
            .repo
            .atomic_publish(
                cmd.definition_version_id,
                cmd.actor_principal_id,
                &computed_digest,
                cmd.expected_revision.as_deref(),
            )
            .await?;

        Ok(published)
    }
}
