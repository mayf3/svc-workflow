//! PostgreSQL repository for domain role queries and member management.
//!
//! Provides data access primitives for self-projection, domain-owner
//! verification, and DOMAIN_MEMBER binding management.  All functions
//! accept raw UUIDs to avoid coupling to domain ID types.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::provisioning::ProvisioningError;

// ---------------------------------------------------------------------------
// Public error type alias — reuses ProvisioningError for storage errors
// ---------------------------------------------------------------------------

fn storage(error: sqlx::Error) -> ProvisioningError {
    ProvisioningError::StorageError(error.to_string())
}

// ---------------------------------------------------------------------------
// Principal operations
// ---------------------------------------------------------------------------

/// Result of a principal self-projection upsert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProjectionResult {
    /// New projection was created.
    Created,
    /// Projection already exists, same type, enabled — no-op success.
    AlreadyExists,
}

/// Upsert a principal self-projection.
///
/// Only creates or confirms an AGENT-type principal.  An existing
/// disabled principal is NOT re-enabled.  An existing principal with
/// a non-AGENT type is rejected.
///
/// Returns:
/// - `Ok(ProjectionResult)` — success
/// - `Err(ProvisioningError::PrincipalDisabled)` — exists but disabled
/// - `Err(ProvisioningError::PrincipalTypeConflict)` — exists with different type
/// - `Err(ProvisioningError::StorageError)` — infrastructure error
pub(crate) async fn upsert_principal_projection(
    pool: &PgPool,
    principal_id: Uuid,
) -> Result<ProjectionResult, ProvisioningError> {
    // Use an advisory lock to serialise concurrent self-projection attempts.
    // A constant lock serialises all self-projections, which is acceptable
    // since this is not a hot path.
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SELECT pg_advisory_xact_lock(8601117002)")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;

    let existing: Option<(bool, String)> = sqlx::query_as(
        "SELECT enabled, principal_type::text FROM principals WHERE principal_id = $1 FOR UPDATE",
    )
    .bind(principal_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage)?;

    match existing {
        // Does not exist → create AGENT projection
        None => {
            let display_name = format!("agent-{}", &principal_id.to_string()[..8]);
            sqlx::query(
                r#"INSERT INTO principals (principal_id, principal_type, display_name, email, enabled, metadata)
                   VALUES ($1, 'AGENT'::principal_type, $2, NULL, TRUE,
                           jsonb_build_object('identitySource', 'AUTH'::text))"#,
            )
            .bind(principal_id)
            .bind(&display_name)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(ProjectionResult::Created)
        }
        // Exists, AGENT, enabled → idempotent success
        Some((true, ref t)) if t == "AGENT" => {
            tx.commit().await.map_err(storage)?;
            Ok(ProjectionResult::AlreadyExists)
        }
        // Exists, AGENT, disabled → reject, do NOT re-enable
        Some((false, ref t)) if t == "AGENT" => {
            tx.commit().await.map_err(storage)?;
            Err(ProvisioningError::PrincipalDisabled)
        }
        // Exists but different type → reject
        Some(_) => {
            tx.commit().await.map_err(storage)?;
            Err(ProvisioningError::PrincipalTypeConflict)
        }
    }
}

/// Check whether a principal exists and is enabled.
///
/// Returns `Ok(true)` if present and enabled, `Ok(false)` if present but
/// disabled, `Ok(None)` if absent.
pub(crate) async fn check_principal_enabled(
    pool: &PgPool,
    principal_id: Uuid,
) -> Result<Option<bool>, ProvisioningError> {
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM principals WHERE principal_id = $1")
            .bind(principal_id)
            .fetch_optional(pool)
            .await
            .map_err(storage)?;
    Ok(enabled)
}

// ---------------------------------------------------------------------------
// Domain operations
// ---------------------------------------------------------------------------

/// Check that a domain exists and is enabled.
pub(crate) async fn check_domain_enabled(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
) -> Result<bool, ProvisioningError> {
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM domains WHERE domain_id = $1 FOR UPDATE")
            .bind(domain_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?;
    Ok(enabled.unwrap_or(false))
}

// ---------------------------------------------------------------------------
// Domain-owner check (shared semantic with query_visibility::check_domain_owner)
// ---------------------------------------------------------------------------

/// Check whether `actor` has an enabled `DOMAIN_OWNER` binding for `domain_id`.
///
/// Uses the exact same SQL as the existing `check_domain_owner` in
/// `query_visibility.rs` to guarantee consistent semantics across
/// domain-list and member management.
pub(crate) async fn check_domain_owner(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    domain_id: Uuid,
) -> Result<bool, ProvisioningError> {
    sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM domain_role_bindings
           WHERE domain_id = $1 AND principal_id = $2
             AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE)",
    )
    .bind(domain_id)
    .bind(actor)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)
}

// ---------------------------------------------------------------------------
// Role binding queries
// ---------------------------------------------------------------------------

/// Check whether `principal_id` has an enabled binding with `role_key`
/// in `domain_id`.
pub(crate) async fn check_has_role(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    principal_id: Uuid,
    role_key: &str,
) -> Result<bool, ProvisioningError> {
    sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM domain_role_bindings
           WHERE domain_id = $1 AND principal_id = $2
             AND role_key = $3 AND enabled = TRUE)",
    )
    .bind(domain_id)
    .bind(principal_id)
    .bind(role_key)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)
}

// ---------------------------------------------------------------------------
// Member listing
// ---------------------------------------------------------------------------

/// A single domain-member row.
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct MemberRow {
    pub principal_id: Uuid,
    pub principal_type: String,
    pub display_name: String,
    pub role: String,
    pub binding_created_at: chrono::DateTime<chrono::Utc>,
}

/// Result of listing domain members.
pub(crate) struct ListMembersResult {
    pub items: Vec<MemberRow>,
    pub next_cursor: Option<ListMembersCursor>,
}

/// Cursor for member-list pagination.
#[derive(Debug, Clone)]
pub(crate) struct ListMembersCursor {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub id: Uuid,
}

/// List enabled `DOMAIN_MEMBER` bindings for a domain, with cursor
/// pagination.
///
/// Follows the same cursor convention as `query_domain_instances`:
/// composite `(created_at, principal_id)` descending, limit + 1
/// technique.
pub(crate) async fn list_member_bindings(
    pool: &PgPool,
    domain_id: Uuid,
    before: Option<ListMembersCursor>,
    limit: u32,
) -> Result<ListMembersResult, ProvisioningError> {
    let query_limit = limit as i64 + 1;
    let rows: Vec<MemberRow> = sqlx::query_as(
        r#"
        SELECT b.principal_id,
               p.principal_type::text AS principal_type,
               p.display_name,
               'DOMAIN_MEMBER' AS role,
               b.created_at AS binding_created_at
        FROM domain_role_bindings b
        JOIN principals p ON p.principal_id = b.principal_id
        WHERE b.domain_id = $1
          AND b.role_key = 'DOMAIN_MEMBER'
          AND b.enabled = TRUE
          AND ($2::timestamptz IS NULL
               OR (b.created_at, b.principal_id) < ($2, $3))
        ORDER BY b.created_at DESC, b.principal_id DESC
        LIMIT $4
        "#,
    )
    .bind(domain_id)
    .bind(before.as_ref().map(|c| c.created_at))
    .bind(before.as_ref().map(|c| c.id))
    .bind(query_limit)
    .fetch_all(pool)
    .await
    .map_err(storage)?;

    let has_more = rows.len() > limit as usize;
    let items: Vec<MemberRow> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = has_more.then(|| {
        let last = items.last().expect("non-empty page after has_more check");
        ListMembersCursor {
            created_at: last.binding_created_at,
            id: last.principal_id,
        }
    });

    Ok(ListMembersResult { items, next_cursor })
}

// ---------------------------------------------------------------------------
// Member binding mutations
// ---------------------------------------------------------------------------

/// Insert or re-enable a DOMAIN_MEMBER binding.
///
/// The UNIQUE index on `(domain_id, principal_id, role_key)` guarantees
/// this UPSERT can only match a row with `role_key = 'DOMAIN_MEMBER'`,
/// never `DOMAIN_OWNER`.
pub(crate) async fn insert_member_binding(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    principal_id: Uuid,
) -> Result<(), ProvisioningError> {
    let binding_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
           VALUES ($1, $2, $3, 'DOMAIN_MEMBER', TRUE)
           ON CONFLICT (domain_id, principal_id, role_key) DO UPDATE
           SET enabled = TRUE, disabled_at = NULL"#,
    )
    .bind(binding_id)
    .bind(domain_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}

/// Soft-delete a DOMAIN_MEMBER binding.
///
/// Returns the number of rows affected (0 if no active binding existed).
pub(crate) async fn delete_member_binding(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    principal_id: Uuid,
) -> Result<u64, ProvisioningError> {
    let affected = sqlx::query(
        "UPDATE domain_role_bindings
         SET enabled = FALSE, disabled_at = now()
         WHERE domain_id = $1 AND principal_id = $2
           AND role_key = 'DOMAIN_MEMBER' AND enabled = TRUE",
    )
    .bind(domain_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await
    .map_err(storage)?
    .rows_affected();
    Ok(affected)
}

// ---------------------------------------------------------------------------
// Security audit
// ---------------------------------------------------------------------------

/// Write a durable audit record to `workflow_security_audits`.
///
/// This INSERT runs inside the same transaction as the business mutation,
/// providing atomic audit + data change.
pub(crate) async fn write_security_audit(
    tx: &mut Transaction<'_, Postgres>,
    actor_principal_id: Uuid,
    action: &str,
    target_principal_id: Uuid,
    domain_id: Uuid,
    details: &serde_json::Value,
) -> Result<(), ProvisioningError> {
    let audit_id = Uuid::new_v4();
    let resource_id = format!("{}/{}", domain_id, target_principal_id);
    sqlx::query(
        "INSERT INTO workflow_security_audits
         (audit_id, principal_id, action, resource_type, resource_id, details)
         VALUES ($1, $2, $3, 'DOMAIN_MEMBERSHIP', $4, $5)",
    )
    .bind(audit_id)
    .bind(actor_principal_id)
    .bind(action)
    .bind(&resource_id)
    .bind(details)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}
