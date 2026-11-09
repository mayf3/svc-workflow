//! Member input-contract read with one snapshot for state and admission.

use crate::application::definition::queries::PublishedVersionInputContract;
use crate::domain::definition::error::DefinitionError;
use crate::domain::enums::DefinitionVersionStatus;
use uuid::Uuid;

use super::error_mapping::map_db_error;
use super::PgDefinitionRepository;

impl PgDefinitionRepository {
    pub(super) async fn get_published_version_input_contract_inner(
        &self,
        actor_principal_id: Uuid,
        definition_version_id: Uuid,
    ) -> Result<PublishedVersionInputContract, DefinitionError> {
        // Same snapshot pattern as workflow query_visibility::begin_snapshot.
        // READ COMMITTED alone would still allow one new snapshot per SELECT.
        // The first principal SELECT establishes the snapshot; later revocation
        // does not invalidate a request that was authorized in that snapshot.
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

        // 1. Principal existence and enabled state, including the distinction
        // preserved by ensure_principal_enabled (both map to opaque 404).
        let principal: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM principals WHERE principal_id = $1")
                .bind(actor_principal_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
        match principal {
            None => return Err(DefinitionError::PrincipalNotFound),
            Some((false,)) => return Err(DefinitionError::PrincipalDisabled),
            Some((true,)) => {}
        }

        // 2. Only the necessary version fields and its stored definition/domain
        // linkage; no graph, lifecycle operators, digest or metadata is loaded.
        let version: Option<(Uuid, i32, String, Option<serde_json::Value>)> = sqlx::query_as(
            "SELECT workflow_definition_id, version_number, version_status::TEXT, context_schema \
             FROM workflow_definition_versions WHERE definition_version_id = $1",
        )
        .bind(definition_version_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let (definition_id, version_number, status, context_schema) =
            version.ok_or(DefinitionError::DefinitionVersionNotFound)?;
        let domain: Option<(Uuid,)> = sqlx::query_as(
            "SELECT domain_id FROM workflow_definitions WHERE workflow_definition_id = $1",
        )
        .bind(definition_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let (domain_id,) = domain.ok_or(DefinitionError::DefinitionNotFound)?;

        // 3. Byte-identical create-instance membership predicate; no role gate.
        let membership: Option<(bool,)> = sqlx::query_as(
            "SELECT enabled FROM domain_role_bindings \
             WHERE domain_id = $1 AND principal_id = $2 AND enabled = TRUE \
             LIMIT 1",
        )
        .bind(domain_id)
        .bind(actor_principal_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if membership.is_none() {
            return Err(DefinitionError::PermissionDenied);
        }

        // 4. Preserve ensure_domain_enabled's error and its precedence over 409.
        let domain_enabled: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM domains WHERE domain_id = $1")
                .bind(domain_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
        if !domain_enabled.is_some_and(|(enabled,)| enabled) {
            return Err(DefinitionError::DomainDisabled);
        }

        // 5. Check status in the same snapshot as every admission gate.
        if status != "PUBLISHED" {
            return Err(DefinitionError::VersionNotPublished);
        }
        tx.commit().await.map_err(map_db_error)?;

        Ok(PublishedVersionInputContract {
            definition_id,
            definition_version_id,
            version_number,
            version_status: DefinitionVersionStatus::PUBLISHED,
            context_schema,
        })
    }
}
