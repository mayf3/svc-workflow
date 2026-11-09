//! Query/read operations for definitions and versions.
//!
//! Handles get_definition, get_definition_version, list_definition_versions,
//! and get_complete_version_graph with domain authorization (H-5).

use crate::domain::definition::error::DefinitionError;
use crate::domain::definition::model::WorkflowGraph;

use super::super::queries::{
    DefinitionQueryResult, DomainDefinitionListResult, GetCompleteVersionGraph, GetDefinition,
    GetDefinitionVersion, GetPublishedVersionInputContract, GraphQueryResult,
    ListDefinitionVersions, ListDomainDefinitions, PublishedVersionInputContract,
    VersionListResult, VersionQueryResult,
};
use super::super::repository::DefinitionData;
use super::super::repository::DefinitionRepository;
use super::super::service::DefinitionService;

impl<R: DefinitionRepository> DefinitionService<R> {
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

        // H-5: Domain authorization — caller must be domain owner
        self.ensure_definition_read(query.actor_principal_id, definition.domain_id.into_uuid())
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
        self.ensure_definition_read(query.actor_principal_id, def.domain_id.into_uuid())
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
        self.ensure_definition_read(query.actor_principal_id, def.domain_id.into_uuid())
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
        self.ensure_definition_read(query.actor_principal_id, def.domain_id.into_uuid())
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

    /// Get the input contract of a PUBLISHED definition version.
    ///
    /// Admission applies the same five conditions and the same membership SQL
    /// as create-instance admission (SVC_WORKFLOW_DEFINITION_INPUT_CONTRACT_
    /// MEMBER_READ_V1) — the AUTHORIZED SET is identical; the ORDER below is
    /// this read's own member-safe sequence, not create's internal one
    /// (create checks sizes -> version exists + domain-match + PUBLISHED
    /// inside `lock_and_validate_version`, THEN domain enabled -> principal
    /// enabled -> membership, so observable error precedence differs on
    /// pathological inputs; order equivalence is NOT claimed): principal
    /// enabled -> version exists -> active domain membership binding -> domain
    /// enabled -> version PUBLISHED. Denials for non-members stay in the
    /// opaque-404 mapper bucket; only the five contract fields are returned.
    pub async fn get_published_version_input_contract(
        &self,
        query: GetPublishedVersionInputContract,
    ) -> Result<PublishedVersionInputContract, DefinitionError> {
        // The repository owns the snapshot boundary: separate pool reads can
        // combine gates that never held together. No management read shares
        // this member-facing operation.
        self.repo
            .get_published_version_input_contract(
                query.actor_principal_id,
                query.definition_version_id,
            )
            .await
    }

    /// List all definitions in a domain (paginated).
    pub async fn list_domain_definitions(
        &self,
        query: ListDomainDefinitions,
    ) -> Result<DomainDefinitionListResult, DefinitionError> {
        self.ensure_principal_enabled(query.actor_principal_id)
            .await?;

        // Domain owner check happens here.
        self.ensure_definition_read(query.actor_principal_id, query.domain_id)
            .await?;
        self.ensure_domain_enabled(query.domain_id).await?;

        let (definitions, cursor) = self
            .repo
            .list_definitions_by_domain(
                query.domain_id,
                query.before_created_at,
                query.before_id,
                query.limit,
                query.include_archived,
            )
            .await?;

        let results = definitions
            .into_iter()
            .map(|d| DefinitionData {
                definition: d,
                version: None,
                nodes: vec![],
                transitions: vec![],
            })
            .collect();

        Ok(DomainDefinitionListResult {
            definitions: results,
            next_cursor: cursor,
        })
    }
}
