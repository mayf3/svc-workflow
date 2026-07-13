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
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Map database errors from PostgreSQL operations to typed [`DefinitionError`] variants.
///
/// Handles:
/// - `23505` unique violations → `DefinitionKeyConflict` / `ConcurrentModification`
/// - Trigger errors with `graph_immutable:` prefix → `VersionNotDraft`
/// - Trigger errors with `status_transition:` prefix → `InvalidLifecycleTransition`
/// - All other errors → `StorageError(raw)`
fn map_db_error(e: sqlx::Error) -> DefinitionError {
    if let sqlx::Error::Database(ref db_err) = e {
        if let Some(code) = db_err.code() {
            if code == "23505" {
                let msg = db_err.message();
                if msg.contains("definition_key") {
                    return DefinitionError::DefinitionKeyConflict;
                }
                if msg.contains("version_number") {
                    return DefinitionError::ConcurrentModification(
                        "duplicate version number".to_string(),
                    );
                }
            }
            let msg = db_err.message();
            if msg.contains("graph_immutable:") {
                return DefinitionError::VersionNotDraft;
            }
            if msg.contains("status_transition:") {
                return DefinitionError::InvalidLifecycleTransition;
            }
        }
    }
    DefinitionError::StorageError(e.to_string())
}
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
                .map_err(map_db_error)?;

        Ok(row.map(|r| r.0).unwrap_or(false))
    }
    async fn check_domain_enabled(&self, domain_id: Uuid) -> Result<bool, DefinitionError> {
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM domains WHERE domain_id = $1")
                .bind(domain_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_db_error)?;

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
        .map_err(map_db_error)?;

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
            map_db_error(e)
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
        .map_err(map_db_error)?;

        Ok(row.map(|r| r.0 > 0).unwrap_or(false))
    }
    async fn get_definition(&self, id: Uuid) -> Result<WorkflowDefinition, DefinitionError> {
        let row: Option<WorkflowDefinition> = sqlx::query_as::<_, WorkflowDefinitionRow>(
            "SELECT * FROM workflow_definitions WHERE workflow_definition_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .map(|r| r.into_domain());

        row.ok_or(DefinitionError::DefinitionNotFound)
    }
    async fn get_version(
        &self,
        version_id: Uuid,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        let row: Option<WorkflowDefinitionVersion> = sqlx::query_as::<_, WorkflowDefinitionVersionRow>(
            "SELECT definition_version_id, workflow_definition_id, version_number, version_status::TEXT AS version_status, definition_digest, json_schema_dialect, validator_version, context_schema, submission_schema, metadata, created_at, updated_at, published_at, deprecated_at, revoked_at, published_by_principal_id, deprecated_by_principal_id, revoked_by_principal_id FROM workflow_definition_versions WHERE definition_version_id = $1",
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
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
        .map_err(map_db_error)?;

        row.map(|r| r.0).ok_or(DefinitionError::DefinitionNotFound)
    }
    async fn get_version_definition_id(&self, version_id: Uuid) -> Result<Uuid, DefinitionError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT workflow_definition_id FROM workflow_definition_versions WHERE definition_version_id = $1",
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?;

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
            map_db_error(e)
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
        .map_err(map_db_error)?;

        Ok(row.and_then(|r| r.0).unwrap_or(0) + 1)
    }
    async fn list_versions(
        &self,
        workflow_definition_id: Uuid,
    ) -> Result<Vec<WorkflowDefinitionVersion>, DefinitionError> {
        let rows: Vec<WorkflowDefinitionVersion> = sqlx::query_as::<_, WorkflowDefinitionVersionRow>(
            "SELECT definition_version_id, workflow_definition_id, version_number, version_status::TEXT AS version_status, definition_digest, json_schema_dialect, validator_version, context_schema, submission_schema, metadata, created_at, updated_at, published_at, deprecated_at, revoked_at, published_by_principal_id, deprecated_by_principal_id, revoked_by_principal_id FROM workflow_definition_versions WHERE workflow_definition_id = $1 ORDER BY version_number DESC",
        )
        .bind(workflow_definition_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?
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
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        // Lock version and verify DRAFT
        let version: Option<(String,)> = sqlx::query_as(
            "SELECT version_status::TEXT FROM workflow_definition_versions WHERE definition_version_id = $1 FOR UPDATE",
        )
        .bind(version_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;

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
            .map_err(map_db_error)?;

        sqlx::query("DELETE FROM workflow_node_definitions WHERE definition_version_id = $1")
            .bind(version_id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

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
            .map_err(map_db_error)?;
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
            .map_err(map_db_error)?;
        }

        // M-1: Update context schema with patch semantics
        // - None (field not provided) → keep existing value (skip)
        // - Some(Value::Null) (explicit null) → clear to NULL
        // - Some(object) → replace with new schema
        if let Some(schema) = context_schema {
            sqlx::query(
                "UPDATE workflow_definition_versions SET context_schema = $1, updated_at = now() WHERE definition_version_id = $2",
            )
            .bind(schema)
            .bind(version_id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        tx.commit().await.map_err(map_db_error)?;

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
        .map_err(map_db_error)?
        .into_iter()
        .map(|r| r.into_domain())
        .collect();

        let transitions: Vec<TransitionDefinition> = sqlx::query_as::<_, TransitionDefinitionRow>(
            "SELECT transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect::TEXT AS transition_effect, submission_schema, metadata, created_at FROM workflow_transition_definitions WHERE definition_version_id = $1 ORDER BY transition_key",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?
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
            "SELECT definition_version_id, workflow_definition_id, version_number, version_status::TEXT AS version_status, definition_digest, json_schema_dialect, validator_version, context_schema, submission_schema, metadata, created_at, updated_at, published_at, deprecated_at, revoked_at, published_by_principal_id, deprecated_by_principal_id, revoked_by_principal_id FROM workflow_definition_versions WHERE definition_version_id = $1 FOR UPDATE",
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .map(|r| r.into_domain());

        row.ok_or(DefinitionError::DefinitionVersionNotFound)
    }
    async fn publish_version(
        &self,
        version_id: Uuid,
        digest: &str,
        actor_principal_id: Uuid,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        sqlx::query(
            r#"
            UPDATE workflow_definition_versions
            SET version_status = 'PUBLISHED', definition_digest = $1, published_at = now(),
                published_by_principal_id = $2, updated_at = now()
            WHERE definition_version_id = $3 AND version_status = 'DRAFT'
            "#,
        )
        .bind(digest)
        .bind(actor_principal_id)
        .bind(version_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_error)?;

        self.get_version(version_id).await
    }
    async fn update_version_status(
        &self,
        version_id: Uuid,
        new_status: DefinitionVersionStatus,
        actor_principal_id: Uuid,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        let (status_col, principal_col) = match new_status {
            DefinitionVersionStatus::DEPRECATED => ("deprecated_at", "deprecated_by_principal_id"),
            DefinitionVersionStatus::REVOKED => ("revoked_at", "revoked_by_principal_id"),
            _ => {
                return Err(DefinitionError::InvalidLifecycleTransition);
            }
        };

        let query = format!(
            "UPDATE workflow_definition_versions SET version_status = $1::definition_version_status, {} = now(), {} = $2, updated_at = now() WHERE definition_version_id = $3",
            status_col, principal_col
        );

        sqlx::query(&query)
            .bind(new_status.to_string())
            .bind(actor_principal_id)
            .bind(version_id)
            .execute(&self.pool)
            .await
            .map_err(map_db_error)?;

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
        .map_err(map_db_error)?
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
        .map_err(map_db_error)?
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
                .map_err(map_db_error)?;

        Ok(row.map(|r| r.0 > 0).unwrap_or(false))
    }

    // -----------------------------------------------------------------------
    // B-1: Atomic lifecycle operations
    // -----------------------------------------------------------------------

    async fn begin_tx(&self) -> Result<Transaction<'_, Postgres>, DefinitionError> {
        self.pool.begin().await.map_err(map_db_error)
    }

    async fn atomic_publish(
        &self,
        version_id: Uuid,
        actor_principal_id: Uuid,
        precomputed_digest: &str,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        // 1. Lock version with FOR UPDATE and verify DRAFT
        let version: Option<WorkflowDefinitionVersion> =
            sqlx::query_as::<_, WorkflowDefinitionVersionRow>(
                "SELECT definition_version_id, workflow_definition_id, version_number, version_status::TEXT AS version_status, definition_digest, json_schema_dialect, validator_version, context_schema, submission_schema, metadata, created_at, updated_at, published_at, deprecated_at, revoked_at, published_by_principal_id, deprecated_by_principal_id, revoked_by_principal_id FROM workflow_definition_versions WHERE definition_version_id = $1 FOR UPDATE",
            )
            .bind(version_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .map(|r| r.into_domain());

        let version = match version {
            None => return Err(DefinitionError::DefinitionVersionNotFound),
            Some(v) if v.version_status != DefinitionVersionStatus::DRAFT => {
                return Err(DefinitionError::VersionNotDraft);
            }
            Some(v) => v,
        };

        // 2. Read definition inside tx
        let def: Option<WorkflowDefinition> = sqlx::query_as::<_, WorkflowDefinitionRow>(
            "SELECT * FROM workflow_definitions WHERE workflow_definition_id = $1",
        )
        .bind(version.workflow_definition_id.into_uuid())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .map(|r| r.into_domain());

        let def = def.ok_or(DefinitionError::DefinitionNotFound)?;
        let domain_id = def.domain_id.into_uuid();

        // 3. Verify domain enabled
        let domain: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM domains WHERE domain_id = $1")
                .bind(domain_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;

        match domain {
            None => return Err(DefinitionError::DomainNotFound),
            Some((enabled,)) if !enabled => return Err(DefinitionError::DomainDisabled),
            _ => {}
        }

        // 4. Verify domain owner
        let is_owner: Option<(bool,)> = sqlx::query_as(
            "SELECT enabled FROM domain_role_bindings WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_OWNER'",
        )
        .bind(domain_id)
        .bind(actor_principal_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;

        match is_owner {
            None => return Err(DefinitionError::PermissionDenied),
            Some((enabled,)) if !enabled => return Err(DefinitionError::PermissionDenied),
            _ => {}
        }

        // 5. Re-read complete graph inside tx
        let nodes: Vec<NodeDefinition> = sqlx::query_as::<_, NodeDefinitionRow>(
            "SELECT node_id, definition_version_id, node_key, display_name, order_index, node_type::TEXT AS node_type, assignee_ref_type::TEXT AS assignee_ref_type, fixed_principal_id, instructions, primary_advance_transition_id, metadata, created_at FROM workflow_node_definitions WHERE definition_version_id = $1 ORDER BY order_index",
        )
        .bind(version_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(|r| r.into_domain())
        .collect();

        let transitions: Vec<TransitionDefinition> = sqlx::query_as::<_, TransitionDefinitionRow>(
            "SELECT transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect::TEXT AS transition_effect, submission_schema, metadata, created_at FROM workflow_transition_definitions WHERE definition_version_id = $1 ORDER BY transition_key",
        )
        .bind(version_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(|r| r.into_domain())
        .collect();

        // 6. Re-compute digest from data read inside tx to verify consistency
        let node_key_map: std::collections::HashMap<_, _> = nodes
            .iter()
            .map(|n| (n.node_id, n.node_key.clone()))
            .collect();
        let transition_key_map: std::collections::HashMap<_, _> = transitions
            .iter()
            .map(|t| (t.transition_id, t.transition_key.clone()))
            .collect();

        let actual_digest = crate::domain::definition::digest::compute_digest(
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

        if actual_digest != precomputed_digest {
            return Err(DefinitionError::ConcurrentModification(
                "definition graph changed during publish; retry with fresh data".to_string(),
            ));
        }

        // 7. Write publish status inside tx
        sqlx::query(
            r#"
            UPDATE workflow_definition_versions
            SET version_status = 'PUBLISHED', definition_digest = $1, published_at = now(),
                published_by_principal_id = $2, updated_at = now()
            WHERE definition_version_id = $3 AND version_status = 'DRAFT'
            "#,
        )
        .bind(precomputed_digest)
        .bind(actor_principal_id)
        .bind(version_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        // 8. Commit
        tx.commit().await.map_err(map_db_error)?;

        // Re-read and return
        self.get_version(version_id).await
    }

    async fn atomic_deprecate(
        &self,
        version_id: Uuid,
        actor_principal_id: Uuid,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        // Lock + verify PUBLISHED status
        let version: Option<WorkflowDefinitionVersion> =
            sqlx::query_as::<_, WorkflowDefinitionVersionRow>(
                "SELECT definition_version_id, workflow_definition_id, version_number, version_status::TEXT AS version_status, definition_digest, json_schema_dialect, validator_version, context_schema, submission_schema, metadata, created_at, updated_at, published_at, deprecated_at, revoked_at, published_by_principal_id, deprecated_by_principal_id, revoked_by_principal_id FROM workflow_definition_versions WHERE definition_version_id = $1 FOR UPDATE",
            )
            .bind(version_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .map(|r| r.into_domain());

        let version = match version {
            None => return Err(DefinitionError::DefinitionVersionNotFound),
            Some(v) if v.version_status != DefinitionVersionStatus::PUBLISHED => {
                return Err(DefinitionError::InvalidLifecycleTransition);
            }
            Some(v) => v,
        };

        // Check domain enabled + domain owner inside tx
        let domain_id = {
            let def: Option<(uuid::Uuid,)> = sqlx::query_as(
                "SELECT domain_id FROM workflow_definitions WHERE workflow_definition_id = $1",
            )
            .bind(version.workflow_definition_id.into_uuid())
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?;
            match def {
                None => return Err(DefinitionError::DefinitionNotFound),
                Some((id,)) => id,
            }
        };

        let domain: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM domains WHERE domain_id = $1")
                .bind(domain_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;

        match domain {
            None => return Err(DefinitionError::DomainNotFound),
            Some((enabled,)) if !enabled => return Err(DefinitionError::DomainDisabled),
            _ => {}
        }

        let is_owner: Option<(bool,)> = sqlx::query_as(
            "SELECT enabled FROM domain_role_bindings WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_OWNER'",
        )
        .bind(domain_id)
        .bind(actor_principal_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;

        match is_owner {
            None => return Err(DefinitionError::PermissionDenied),
            Some((enabled,)) if !enabled => return Err(DefinitionError::PermissionDenied),
            _ => {}
        }

        // Write deprecate status inside tx
        sqlx::query(
            r#"
            UPDATE workflow_definition_versions
            SET version_status = 'DEPRECATED', deprecated_at = now(),
                deprecated_by_principal_id = $1, updated_at = now()
            WHERE definition_version_id = $2
            "#,
        )
        .bind(actor_principal_id)
        .bind(version_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        tx.commit().await.map_err(map_db_error)?;

        self.get_version(version_id).await
    }

    async fn atomic_revoke(
        &self,
        version_id: Uuid,
        actor_principal_id: Uuid,
    ) -> Result<WorkflowDefinitionVersion, DefinitionError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        // Lock + verify PUBLISHED or DEPRECATED status
        let version: Option<WorkflowDefinitionVersion> =
            sqlx::query_as::<_, WorkflowDefinitionVersionRow>(
                "SELECT definition_version_id, workflow_definition_id, version_number, version_status::TEXT AS version_status, definition_digest, json_schema_dialect, validator_version, context_schema, submission_schema, metadata, created_at, updated_at, published_at, deprecated_at, revoked_at, published_by_principal_id, deprecated_by_principal_id, revoked_by_principal_id FROM workflow_definition_versions WHERE definition_version_id = $1 FOR UPDATE",
            )
            .bind(version_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .map(|r| r.into_domain());

        let version = match version {
            None => return Err(DefinitionError::DefinitionVersionNotFound),
            Some(v)
                if v.version_status != DefinitionVersionStatus::PUBLISHED
                    && v.version_status != DefinitionVersionStatus::DEPRECATED =>
            {
                return Err(DefinitionError::InvalidLifecycleTransition);
            }
            Some(v) => v,
        };

        // Check domain enabled + domain owner inside tx
        let domain_id = {
            let def: Option<(uuid::Uuid,)> = sqlx::query_as(
                "SELECT domain_id FROM workflow_definitions WHERE workflow_definition_id = $1",
            )
            .bind(version.workflow_definition_id.into_uuid())
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?;
            match def {
                None => return Err(DefinitionError::DefinitionNotFound),
                Some((id,)) => id,
            }
        };

        let domain: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM domains WHERE domain_id = $1")
                .bind(domain_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;

        match domain {
            None => return Err(DefinitionError::DomainNotFound),
            Some((enabled,)) if !enabled => return Err(DefinitionError::DomainDisabled),
            _ => {}
        }

        let is_owner: Option<(bool,)> = sqlx::query_as(
            "SELECT enabled FROM domain_role_bindings WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_OWNER'",
        )
        .bind(domain_id)
        .bind(actor_principal_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;

        match is_owner {
            None => return Err(DefinitionError::PermissionDenied),
            Some((enabled,)) if !enabled => return Err(DefinitionError::PermissionDenied),
            _ => {}
        }

        // Write revoke status inside tx
        sqlx::query(
            r#"
            UPDATE workflow_definition_versions
            SET version_status = 'REVOKED', revoked_at = now(),
                revoked_by_principal_id = $1, updated_at = now()
            WHERE definition_version_id = $2
            "#,
        )
        .bind(actor_principal_id)
        .bind(version_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        tx.commit().await.map_err(map_db_error)?;

        self.get_version(version_id).await
    }
}
