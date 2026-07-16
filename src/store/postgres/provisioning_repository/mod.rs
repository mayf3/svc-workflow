//! PostgreSQL repository for identity provisioning operations.
//!
//! Implements idempotent receipt handling and CRUD for principals,
//! domains, and role bindings, following the admin recovery pattern.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::ids::{DefinitionVersionId, DomainId, PrincipalId};
use crate::domain::provisioning::ProvisioningError;

mod receipt;

pub(crate) use receipt::{acquire_receipt, complete_receipt, AcquireReceipt};

fn storage(e: sqlx::Error) -> ProvisioningError {
    ProvisioningError::StorageError(e.to_string())
}

// ---------------------------------------------------------------------------
// Principal operations
// ---------------------------------------------------------------------------

/// Bootstrap an allow-listed provisioning actor from its verified direct Agent
/// token when it provisions its own canonical principal ID for the first time.
/// The surrounding transaction still owns the command receipt, so a failed
/// first request cannot leave an unreceipted principal behind.
pub(crate) async fn ensure_provisioning_actor(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    source: &str,
) -> Result<(), ProvisioningError> {
    let display_name = format!("{}-{}", source, &principal_id.to_string()[..8]);
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
         VALUES ($1, 'AGENT', $2, NULL, TRUE)
         ON CONFLICT (principal_id) DO NOTHING",
    )
    .bind(principal_id)
    .bind(display_name)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}

/// Lock and revalidate the provisioning actor inside the write transaction.
/// This closes the race between HTTP extraction and a concurrent disable.
pub(crate) async fn validate_provisioning_actor(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
) -> Result<(), ProvisioningError> {
    // One control-plane lock makes disables and owner changes linearizable.
    sqlx::query("SELECT pg_advisory_xact_lock(8601117001)")
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    let actor: Option<(bool, String)> = sqlx::query_as(
        "SELECT enabled, principal_type::text FROM principals
         WHERE principal_id = $1 FOR UPDATE",
    )
    .bind(principal_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    match actor {
        Some((true, principal_type)) if principal_type == "AGENT" => Ok(()),
        Some((false, _)) => Err(ProvisioningError::PrincipalDisabled),
        Some(_) => Err(ProvisioningError::PrincipalTypeNotAllowed),
        None => Err(ProvisioningError::PermissionDenied),
    }
}

/// Principal row from the database.
#[derive(Debug, sqlx::FromRow)]
pub struct PrincipalRow {
    pub principal_id: Uuid,
    pub principal_type: String,
    pub display_name: String,
    pub email: Option<String>,
    pub enabled: bool,
    pub metadata: Option<serde_json::Value>,
}

/// Upsert a principal. Returns the principal_id on success.
pub(crate) async fn upsert_principal(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    principal_type: &str,
    enabled: bool,
    source: &str,
    source_revision: Option<&str>,
) -> Result<Uuid, ProvisioningError> {
    // Convert lowercase API types to uppercase PG enum values
    let db_type = match principal_type {
        "human" => "HUMAN",
        "agent" => "AGENT",
        _ => return Err(ProvisioningError::PrincipalTypeInvalid),
    };
    // Check for type conflict on existing principal
    let existing_type: Option<String> = sqlx::query_scalar(
        "SELECT principal_type::text FROM principals WHERE principal_id = $1 FOR UPDATE",
    )
    .bind(principal_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;

    if let Some(ref existing) = existing_type {
        if existing != db_type {
            return Err(ProvisioningError::PrincipalTypeConflict);
        }
    }

    // Build display_name from source info
    let display_name = format!("{}-{}", source, &principal_id.to_string()[..8]);

    let provenance = serde_json::json!({
        "source": source,
        "sourceRevision": source_revision,
    });
    sqlx::query(
        r#"
        INSERT INTO principals (principal_id, principal_type, display_name, email, enabled, metadata)
        VALUES ($1, $2::principal_type, $3, NULL, $4,
                jsonb_build_object('provisioning', $5::jsonb))
        ON CONFLICT (principal_id) DO UPDATE
        SET enabled = $4,
            metadata = jsonb_set(
                COALESCE(principals.metadata, '{}'::jsonb),
                '{provisioning}',
                $5::jsonb,
                TRUE
            ),
            updated_at = now()
        "#,
    )
    .bind(principal_id)
    .bind(db_type)
    .bind(&display_name)
    .bind(enabled)
    .bind(provenance)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;

    Ok(principal_id)
}

/// Get a principal by ID.
pub(crate) async fn get_principal(
    pool: &PgPool,
    principal_id: Uuid,
) -> Result<Option<PrincipalRow>, ProvisioningError> {
    let row: Option<PrincipalRow> = sqlx::query_as(
        "SELECT principal_id, principal_type::text, display_name, email, enabled, metadata
         FROM principals WHERE principal_id = $1",
    )
    .bind(principal_id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?;
    Ok(row)
}

// ---------------------------------------------------------------------------
// Domain operations
// ---------------------------------------------------------------------------

/// Domain row from the database.
#[derive(Debug, sqlx::FromRow)]
pub struct DomainRow {
    pub domain_id: Uuid,
    pub domain_key: String,
    pub display_name: String,
    pub enabled: bool,
    pub metadata: Option<serde_json::Value>,
}

/// Upsert a domain. Validates domain_key uniqueness.
pub(crate) async fn upsert_domain(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    domain_key: &str,
    display_name: Option<&str>,
    enabled: bool,
) -> Result<Uuid, ProvisioningError> {
    // Serialize by the canonical key even when no row exists yet; FOR UPDATE
    // alone cannot lock a missing row under READ COMMITTED.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(domain_key)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;

    // Check domain_key collision: if another domain already uses this key, reject.
    let existing_key_owner: Option<Uuid> =
        sqlx::query_scalar("SELECT domain_id FROM domains WHERE domain_key = $1 FOR UPDATE")
            .bind(domain_key)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?;

    if let Some(owner) = existing_key_owner {
        if owner != domain_id {
            return Err(ProvisioningError::DomainIdentityConflict);
        }
    }

    let name = display_name.unwrap_or(domain_key);

    sqlx::query(
        r#"
        INSERT INTO domains (domain_id, domain_key, display_name, enabled)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (domain_id) DO UPDATE
        SET enabled = $4,
            domain_key = $2,
            display_name = $3,
            updated_at = now()
        "#,
    )
    .bind(domain_id)
    .bind(domain_key)
    .bind(name)
    .bind(enabled)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;

    Ok(domain_id)
}

/// Get a domain by ID.
pub(crate) async fn get_domain(
    pool: &PgPool,
    domain_id: Uuid,
) -> Result<Option<DomainRow>, ProvisioningError> {
    let row: Option<DomainRow> = sqlx::query_as(
        "SELECT domain_id, domain_key, display_name, enabled, metadata
         FROM domains WHERE domain_id = $1",
    )
    .bind(domain_id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?;
    Ok(row)
}

// ---------------------------------------------------------------------------
// Role binding operations
// ---------------------------------------------------------------------------

/// Role binding row.
#[derive(Debug, sqlx::FromRow)]
pub struct RoleBindingRow {
    pub binding_id: Uuid,
    pub domain_id: Uuid,
    pub principal_id: Uuid,
    pub role_key: String,
    pub enabled: bool,
}

/// Upsert a role binding (create or re-enable).
pub(crate) async fn upsert_role_binding(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    principal_id: Uuid,
    role_key: &str,
    enabled: bool,
) -> Result<Uuid, ProvisioningError> {
    // Check principal exists and is enabled
    let principal_enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM principals WHERE principal_id = $1 FOR UPDATE")
            .bind(principal_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?;

    match principal_enabled {
        None => return Err(ProvisioningError::PrincipalNotFound),
        Some(false) => return Err(ProvisioningError::PrincipalDisabled),
        Some(true) => {}
    }

    // Check domain exists and is enabled (skip for domain-disabling operations)
    let domain_enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM domains WHERE domain_id = $1 FOR UPDATE")
            .bind(domain_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?;

    match domain_enabled {
        None => return Err(ProvisioningError::DomainNotFound),
        Some(false) if enabled => return Err(ProvisioningError::DomainDisabled),
        _ => {}
    }

    // Check DOMAIN_OWNER uniqueness
    if role_key == "DOMAIN_OWNER" && enabled {
        let existing_owner: Option<Uuid> = sqlx::query_scalar(
            "SELECT binding_id FROM domain_role_bindings
             WHERE domain_id = $1 AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE
             AND principal_id != $2
             FOR UPDATE",
        )
        .bind(domain_id)
        .bind(principal_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?;

        if existing_owner.is_some() {
            return Err(ProvisioningError::DomainOwnerConflict);
        }
    }

    // UPSERT the binding
    let binding_id = Uuid::new_v4();
    let result: Option<(Uuid,)> = sqlx::query_as(
        r#"
        INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (domain_id, principal_id, role_key) DO UPDATE
        SET enabled = $5, disabled_at = CASE WHEN $5 THEN NULL ELSE now() END
        RETURNING binding_id
        "#,
    )
    .bind(binding_id)
    .bind(domain_id)
    .bind(principal_id)
    .bind(role_key)
    .bind(enabled)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;

    Ok(result.map(|r| r.0).unwrap_or(binding_id))
}

/// Disable a role binding (soft delete).
pub(crate) async fn disable_role_binding(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    principal_id: Uuid,
    role_key: &str,
) -> Result<(), ProvisioningError> {
    let affected = sqlx::query(
        "UPDATE domain_role_bindings
         SET enabled = FALSE, disabled_at = now()
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = $3 AND enabled = TRUE",
    )
    .bind(domain_id)
    .bind(principal_id)
    .bind(role_key)
    .execute(&mut **tx)
    .await
    .map_err(storage)?
    .rows_affected();

    if affected == 0 {
        return Err(ProvisioningError::BindingNotFound);
    }
    Ok(())
}

/// Atomically replace a domain owner.
///
/// In a single transaction:
/// 1. Optionally disable the current owner (if specified).
/// 2. Enable the new owner.
pub(crate) async fn replace_domain_owner(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    new_owner_id: Uuid,
) -> Result<(), ProvisioningError> {
    // Lock in the same principal -> domain order used by role binding writes.
    let new_enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM principals WHERE principal_id = $1 FOR UPDATE")
            .bind(new_owner_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?;

    match new_enabled {
        None => return Err(ProvisioningError::PrincipalNotFound),
        Some(false) => return Err(ProvisioningError::PrincipalDisabled),
        _ => {}
    }

    // The domain row is the serialization point for all owner changes. Reading
    // the current owner outside this transaction would permit TOCTOU races.
    let domain_enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM domains WHERE domain_id = $1 FOR UPDATE")
            .bind(domain_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?;
    match domain_enabled {
        None => return Err(ProvisioningError::DomainNotFound),
        Some(false) => return Err(ProvisioningError::DomainDisabled),
        Some(true) => {}
    }

    sqlx::query(
        "UPDATE domain_role_bindings
         SET enabled = FALSE, disabled_at = now()
         WHERE domain_id = $1 AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE
           AND principal_id != $2",
    )
    .bind(domain_id)
    .bind(new_owner_id)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;

    // Enable new owner
    let binding_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
        VALUES ($1, $2, $3, 'DOMAIN_OWNER', TRUE)
        ON CONFLICT (domain_id, principal_id, role_key) DO UPDATE
        SET enabled = TRUE, disabled_at = NULL
        "#,
    )
    .bind(binding_id)
    .bind(domain_id)
    .bind(new_owner_id)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Definition version queries
// ---------------------------------------------------------------------------

/// Definition version summary row.
#[derive(Debug, sqlx::FromRow)]
pub struct DefinitionVersionSummary {
    pub definition_version_id: Uuid,
    pub workflow_definition_id: Uuid,
    pub domain_id: Uuid,
    pub domain_enabled: bool,
    pub version_status: String,
    pub version_number: i32,
    pub definition_key: String,
    pub digest: Option<String>,
}

/// Get a definition version summary.
pub(crate) async fn get_definition_version_summary(
    pool: &PgPool,
    version_id: Uuid,
) -> Result<Option<DefinitionVersionSummary>, ProvisioningError> {
    let row: Option<DefinitionVersionSummary> = sqlx::query_as(
        r#"
        SELECT v.definition_version_id, d.workflow_definition_id, d.domain_id,
               domain.enabled AS domain_enabled, v.version_status::text,
               v.version_number, d.definition_key, v.definition_digest as digest
        FROM workflow_definition_versions v
        JOIN workflow_definitions d ON d.workflow_definition_id = v.workflow_definition_id
        JOIN domains domain ON domain.domain_id = d.domain_id
        WHERE v.definition_version_id = $1
        "#,
    )
    .bind(version_id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?;
    Ok(row)
}

/// Get node/transition counts for a definition version.
pub(crate) async fn get_definition_graph_counts(
    pool: &PgPool,
    version_id: Uuid,
) -> Result<(i64, i64), ProvisioningError> {
    let nodes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_node_definitions WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .fetch_one(pool)
    .await
    .map_err(storage)?;

    let transitions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_transition_definitions WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .fetch_one(pool)
    .await
    .map_err(storage)?;

    Ok((nodes, transitions))
}
