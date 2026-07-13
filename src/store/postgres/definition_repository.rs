//! PostgreSQL implementation of the [`DefinitionRepository`] trait.
#![allow(clippy::needless_borrow)]
use super::repository_rows::*;
use crate::application::definition::repository::DefinitionData;
use crate::application::definition::DefinitionRepository;
use crate::domain::definition::error::DefinitionError;
use crate::domain::definition::model::{
    AssigneeRef, NodeDefinition, TransitionDefinition, WorkflowDefinition,
    WorkflowDefinitionVersion,
};
use crate::domain::enums::DefinitionVersionStatus;
use crate::domain::ids::{
    DefinitionVersionId, DomainId, NodeId, PrincipalId, TransitionId, WorkflowDefinitionId,
};
use sqlx::PgPool;
use uuid::Uuid;
/// Initialize the PgDefinitionRepository with a database connection.
pub struct PgDefinitionRepository {
    pool: PgPool,
}
impl PgDefinitionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[allow(async_fn_in_trait)]
impl DefinitionRepository for PgDefinitionRepository {
    // -- Principals & Domains -------------------------------------------------

    async fn check_principal_enabled(&self, principal_id: Uuid) -> Result<bool, DefinitionError> {
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM principals WHERE principal_id = $1")
                .bind(principal_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        Ok(row.map(|r| r.0).unwrap_or(false))
    }
    async fn check_domain_enabled(&self, domain_id: Uuid) -> Result<bool, DefinitionError> {
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM domains WHERE domain_id = $1")
                .bind(domain_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        Ok(row.map(|r| r.0).unwrap_or(false))
    }
    async fn check_domain_role(
        &self,
        principal_id: Uuid,
        domain_id: Uuid,
        role_key: &str,
    ) -> Result<bool, DefinitionError> {
        let row: Option<(bool,)> = sqlx::query_as(
            "SELECT enabled FROM domain_role_bindings WHERE domain_id = $1 AND principal_id = $2 AND role_key = $3",
        )
        .bind(domain_id)
        .bind(principal_id)
        .bind(role_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        Ok(row.map(|r| r.0).unwrap_or(false))
    }

    // -- Definition CRUD -------------------------------------------------------

    async fn create_definition(
        &self,
        id: Uuid,
        domain_id: Uuid,
        definition_key: &str,
        display_name: &str,
        description: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<WorkflowDefinition, DefinitionError> {
        sqlx::query(
            r#"
            INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name, description, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(domain_id)
        .bind(definition_key)
        .bind(display_name)
        .bind(description)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if let Some(code) = db_err.code() {
                    if code == "23505" {
                        return DefinitionError::DefinitionKeyConflict;
                    }
                }
            }
            DefinitionError::StorageError(e.to_string())
        })?;

        self.get_definition(id).await
    }
    async fn definition_key_exists(
        &self,
        domain_id: Uuid,
        definition_key: &str,
    ) -> Result<bool, DefinitionError> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM workflow_definitions WHERE domain_id = $1 AND definition_key = $2",
        )
        .bind(domain_id)
        .bind(definition_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        Ok(row.map(|r| r.0 > 0).unwrap_or(false))
    }
    async fn get_definition(&self, id: Uuid) -> Result<WorkflowDefinition, DefinitionError> {
        let row: Option<WorkflowDefinition> = sqlx::query_as::<_, WorkflowDefinitionRow>(
            "SELECT * FROM workflow_definitions WHERE workflow_definition_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?
        .map(|r| r.into_domain());

        row.ok_or(DefinitionError::DefinitionNotFound)
    }
    async fn get_version(
        &self,
        version_id: Uuid,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        let row: Option<WorkflowDefinitionVersion> = sqlx::query_as::<_, WorkflowDefinitionVersionRow>(
            "SELECT definition_version_id, workflow_definition_id, version_number, version_status::TEXT AS version_status, definition_digest, json_schema_dialect, validator_version, context_schema, submission_schema, metadata, created_at, updated_at, published_at, deprecated_at, revoked_at FROM workflow_definition_versions WHERE definition_version_id = $1",
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?
        .map(|r| r.into_domain());

        row.ok_or(DefinitionError::DefinitionVersionNotFound)
    }
    async fn get_definition_domain(&self, definition_id: Uuid) -> Result<Uuid, DefinitionError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT domain_id FROM workflow_definitions WHERE workflow_definition_id = $1",
        )
        .bind(definition_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        row.map(|r| r.0).ok_or(DefinitionError::DefinitionNotFound)
    }
    async fn get_version_definition_id(&self, version_id: Uuid) -> Result<Uuid, DefinitionError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT workflow_definition_id FROM workflow_definition_versions WHERE definition_version_id = $1",
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        row.map(|r| r.0)
            .ok_or(DefinitionError::DefinitionVersionNotFound)
    }

    // -- Version CRUD ----------------------------------------------------------

    async fn create_draft_version(
        &self,
        id: Uuid,
        workflow_definition_id: Uuid,
        version_number: i32,
        context_schema: Option<&serde_json::Value>,
        json_schema_dialect: Option<&str>,
        validator_version: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        sqlx::query(
            r#"
            INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema, json_schema_dialect, validator_version, metadata)
            VALUES ($1, $2, $3, 'DRAFT', $4, $5, $6, $7)
            "#,
        )
        .bind(id)
        .bind(workflow_definition_id)
        .bind(version_number)
        .bind(context_schema)
        .bind(json_schema_dialect)
        .bind(validator_version)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if let Some(code) = db_err.code() {
                    if code == "23505" {
                        return DefinitionError::ConcurrentModification(
                            "duplicate version number".to_string(),
                        );
                    }
                }
            }
            DefinitionError::StorageError(e.to_string())
        })?;

        self.get_version(id).await
    }
    async fn next_version_number(
        &self,
        workflow_definition_id: Uuid,
    ) -> Result<i32, DefinitionError> {
        let row: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT MAX(version_number) FROM workflow_definition_versions WHERE workflow_definition_id = $1",
        )
        .bind(workflow_definition_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        Ok(row.and_then(|r| r.0).unwrap_or(0) + 1)
    }
    async fn list_versions(
        &self,
        workflow_definition_id: Uuid,
    ) -> Result<Vec<WorkflowDefinitionVersion>, DefinitionError> {
        let rows: Vec<WorkflowDefinitionVersion> = sqlx::query_as::<_, WorkflowDefinitionVersionRow>(
            "SELECT definition_version_id, workflow_definition_id, version_number, version_status::TEXT AS version_status, definition_digest, json_schema_dialect, validator_version, context_schema, submission_schema, metadata, created_at, updated_at, published_at, deprecated_at, revoked_at FROM workflow_definition_versions WHERE workflow_definition_id = $1 ORDER BY version_number DESC",
        )
        .bind(workflow_definition_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?
        .into_iter()
        .map(|r| r.into_domain())
        .collect();

        Ok(rows)
    }

    // -- Graph operations ------------------------------------------------------

    async fn replace_draft_graph(
        &self,
        version_id: Uuid,
        context_schema: Option<&serde_json::Value>,
        nodes: &[NodeDefinition],
        transitions: &[TransitionDefinition],
    ) -> Result<(), DefinitionError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        // Lock version and verify DRAFT
        let version: Option<(String,)> = sqlx::query_as(
            "SELECT version_status::TEXT FROM workflow_definition_versions WHERE definition_version_id = $1 FOR UPDATE",
        )
        .bind(version_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        match version {
            None => return Err(DefinitionError::DefinitionVersionNotFound),
            Some((status,)) if status != "DRAFT" => return Err(DefinitionError::VersionNotDraft),
            _ => {}
        }

        // Delete old nodes (this will cascade delete old transitions via FK)
        // Actually, transitions have FK to nodes, so delete transitions first
        sqlx::query("DELETE FROM workflow_transition_definitions WHERE definition_version_id = $1")
            .bind(version_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        sqlx::query("DELETE FROM workflow_node_definitions WHERE definition_version_id = $1")
            .bind(version_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        // Insert new nodes
        for node in nodes {
            sqlx::query(
                r#"
                INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id, instructions, primary_advance_transition_id, metadata)
                VALUES ($1, $2, $3, $4, $5, $6::node_type, $7::assignee_ref_type, $8, $9, $10, $11)
                "#,
            )
            .bind(node.node_id.into_uuid())
            .bind(version_id)
            .bind(&node.node_key)
            .bind(&node.display_name)
            .bind(node.order_index)
            .bind(node.node_type.to_string())
            .bind(node.assignee_ref.ref_type.to_string())
            .bind(node.assignee_ref.fixed_principal_id.map(|id| id.into_uuid()))
            .bind(&node.instructions)
            .bind(node.primary_advance_transition_id.map(|id| id.into_uuid()))
            .bind(&node.metadata)
            .execute(&mut *tx)
            .await
            .map_err(|e| DefinitionError::StorageError(e.to_string()))?;
        }

        // Insert new transitions
        for trans in transitions {
            sqlx::query(
                r#"
                INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7::transition_effect, $8, $9)
                "#,
            )
            .bind(trans.transition_id.into_uuid())
            .bind(version_id)
            .bind(&trans.transition_key)
            .bind(&trans.display_name)
            .bind(trans.source_node_id.into_uuid())
            .bind(trans.target_node_id.into_uuid())
            .bind(trans.transition_effect.to_string())
            .bind(&trans.submission_schema)
            .bind(&trans.metadata)
            .execute(&mut *tx)
            .await
            .map_err(|e| DefinitionError::StorageError(e.to_string()))?;
        }

        // Update context schema
        if let Some(schema) = context_schema {
            sqlx::query(
                "UPDATE workflow_definition_versions SET context_schema = $1, updated_at = now() WHERE definition_version_id = $2",
            )
            .bind(schema)
            .bind(version_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DefinitionError::StorageError(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        Ok(())
    }
    async fn get_complete_graph(
        &self,
        version_id: Uuid,
    ) -> Result<(Vec<NodeDefinition>, Vec<TransitionDefinition>), DefinitionError> {
        let nodes: Vec<NodeDefinition> = sqlx::query_as::<_, NodeDefinitionRow>(
            "SELECT node_id, definition_version_id, node_key, display_name, order_index, node_type::TEXT AS node_type, assignee_ref_type::TEXT AS assignee_ref_type, fixed_principal_id, instructions, primary_advance_transition_id, metadata, created_at FROM workflow_node_definitions WHERE definition_version_id = $1 ORDER BY order_index",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?
        .into_iter()
        .map(|r| r.into_domain())
        .collect();

        let transitions: Vec<TransitionDefinition> = sqlx::query_as::<_, TransitionDefinitionRow>(
            "SELECT transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect::TEXT AS transition_effect, submission_schema, metadata, created_at FROM workflow_transition_definitions WHERE definition_version_id = $1 ORDER BY transition_key",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?
        .into_iter()
        .map(|r| r.into_domain())
        .collect();

        Ok((nodes, transitions))
    }

    // -- Lifecycle operations --------------------------------------------------

    async fn lock_version(
        &self,
        version_id: Uuid,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        let row: Option<WorkflowDefinitionVersion> = sqlx::query_as::<_, WorkflowDefinitionVersionRow>(
            "SELECT definition_version_id, workflow_definition_id, version_number, version_status::TEXT AS version_status, definition_digest, json_schema_dialect, validator_version, context_schema, submission_schema, metadata, created_at, updated_at, published_at, deprecated_at, revoked_at FROM workflow_definition_versions WHERE definition_version_id = $1 FOR UPDATE",
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?
        .map(|r| r.into_domain());

        row.ok_or(DefinitionError::DefinitionVersionNotFound)
    }
    async fn publish_version(
        &self,
        version_id: Uuid,
        digest: &str,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        sqlx::query(
            r#"
            UPDATE workflow_definition_versions
            SET version_status = 'PUBLISHED', definition_digest = $1, published_at = now(), updated_at = now()
            WHERE definition_version_id = $2 AND version_status = 'DRAFT'
            "#,
        )
        .bind(digest)
        .bind(version_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        self.get_version(version_id).await
    }
    async fn update_version_status(
        &self,
        version_id: Uuid,
        new_status: DefinitionVersionStatus,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        let (status_col, _) = match new_status {
            DefinitionVersionStatus::DEPRECATED => ("deprecated_at", "DEPRECATED"),
            DefinitionVersionStatus::REVOKED => ("revoked_at", "REVOKED"),
            _ => {
                return Err(DefinitionError::InvalidLifecycleTransition);
            }
        };

        let query = format!(
            "UPDATE workflow_definition_versions SET version_status = $1::definition_version_status, {} = now(), updated_at = now() WHERE definition_version_id = $2",
            status_col
        );

        sqlx::query(&query)
            .bind(new_status.to_string())
            .bind(version_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        self.get_version(version_id).await
    }
    async fn get_nodes_by_version(
        &self,
        version_id: Uuid,
    ) -> Result<Vec<NodeDefinition>, DefinitionError> {
        let nodes: Vec<NodeDefinition> = sqlx::query_as::<_, NodeDefinitionRow>(
            "SELECT node_id, definition_version_id, node_key, display_name, order_index, node_type::TEXT AS node_type, assignee_ref_type::TEXT AS assignee_ref_type, fixed_principal_id, instructions, primary_advance_transition_id, metadata, created_at FROM workflow_node_definitions WHERE definition_version_id = $1 ORDER BY order_index",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?
        .into_iter()
        .map(|r| r.into_domain())
        .collect();

        Ok(nodes)
    }
    async fn get_transitions_by_version(
        &self,
        version_id: Uuid,
    ) -> Result<Vec<TransitionDefinition>, DefinitionError> {
        let transitions: Vec<TransitionDefinition> = sqlx::query_as::<_, TransitionDefinitionRow>(
            "SELECT transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect::TEXT AS transition_effect, submission_schema, metadata, created_at FROM workflow_transition_definitions WHERE definition_version_id = $1 ORDER BY transition_key",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DefinitionError::StorageError(e.to_string()))?
        .into_iter()
        .map(|r| r.into_domain())
        .collect();

        Ok(transitions)
    }
    async fn check_principal_exists(&self, principal_id: Uuid) -> Result<bool, DefinitionError> {
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT COUNT(*) FROM principals WHERE principal_id = $1")
                .bind(principal_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| DefinitionError::StorageError(e.to_string()))?;

        Ok(row.map(|r| r.0 > 0).unwrap_or(false))
    }
}
