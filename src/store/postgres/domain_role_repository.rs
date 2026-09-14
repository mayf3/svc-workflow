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

/// Check whether a principal exists and is enabled (tx-scoped variant for
/// use inside an open transaction).
///
/// Returns `Ok(Some(true))` present+enabled, `Ok(Some(false))` present but
/// disabled, `Ok(None)` absent.
pub(crate) async fn check_principal_enabled_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
) -> Result<Option<bool>, ProvisioningError> {
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM principals WHERE principal_id = $1 FOR SHARE")
            .bind(principal_id)
            .fetch_optional(&mut **tx)
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

/// Check whether `actor` may perform a governance write on `domain_id`:
/// enabled `DOMAIN_OWNER` of that domain OR enabled
/// `GLOBAL_WORKFLOW_COORDINATOR` (SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1
/// W-widening, CTR-CP-001). Server-side role bindings only — never the JWT.
pub(crate) async fn check_domain_write_role(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    domain_id: Uuid,
) -> Result<bool, ProvisioningError> {
    sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM domain_role_bindings
           WHERE domain_id = $1 AND principal_id = $2
             AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE)
         OR EXISTS(
           SELECT 1 FROM global_role_bindings g
           JOIN principals p ON p.principal_id = g.principal_id
           WHERE g.principal_id = $2
             AND g.role_key = 'GLOBAL_WORKFLOW_COORDINATOR' AND g.enabled = TRUE
             AND p.enabled = TRUE)",
    )
    .bind(domain_id)
    .bind(actor)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)
}

/// Pool-level variant of `check_domain_owner` (read-only surfaces such as
/// get-owner).
pub(crate) async fn pool_check_domain_owner(
    pool: &PgPool,
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
    .fetch_one(pool)
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
// Caller-scoped domain discovery
// ---------------------------------------------------------------------------

/// A single caller-scoped domain membership row.
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct MyDomainRow {
    pub domain_id: Uuid,
    pub domain_key: String,
    pub display_name: String,
    pub role_key: String,
    pub binding_created_at: chrono::DateTime<chrono::Utc>,
}

/// List the enabled role bindings of `principal_id` joined with the
/// domain's basic info.
///
/// Caller-scoped discovery: returns every domain where the principal has an
/// enabled binding (DOMAIN_OWNER / DOMAIN_MEMBER).  Disabled bindings and
/// disabled domains are excluded — matching the semantics of
/// `check_domain_owner` / `check_has_role`, so the list only ever contains
/// domains the caller can actually act on.
pub(crate) async fn list_my_domains(
    pool: &PgPool,
    principal_id: Uuid,
) -> Result<Vec<MyDomainRow>, ProvisioningError> {
    let rows: Vec<MyDomainRow> = sqlx::query_as(
        r#"
        SELECT b.domain_id,
               d.domain_key,
               d.display_name,
               b.role_key,
               b.created_at AS binding_created_at
        FROM domain_role_bindings b
        JOIN domains d ON d.domain_id = b.domain_id
        WHERE b.principal_id = $1
          AND b.enabled = TRUE
          AND d.enabled = TRUE
        ORDER BY d.domain_key ASC, b.role_key ASC
        "#,
    )
    .bind(principal_id)
    .fetch_all(pool)
    .await
    .map_err(storage)?;
    Ok(rows)
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

// ---------------------------------------------------------------------------
// Coordinator control plane reads/writes (SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1)
// ---------------------------------------------------------------------------

/// Full governance-metadata row of a domain.
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct DomainFullRow {
    pub domain_id: Uuid,
    pub domain_key: String,
    pub display_name: String,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

const DOMAIN_FULL_COLUMNS: &str =
    "domain_id, domain_key, display_name, enabled, created_at, updated_at";

/// Fetch one domain's full governance metadata.
pub(crate) async fn get_domain_full(
    pool: &PgPool,
    domain_id: Uuid,
) -> Result<Option<DomainFullRow>, ProvisioningError> {
    let row: Option<DomainFullRow> = sqlx::query_as(&format!(
        "SELECT {DOMAIN_FULL_COLUMNS} FROM domains WHERE domain_id = $1"
    ))
    .bind(domain_id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?;
    Ok(row)
}

/// List all domains ordered by (created_at, domain_id) descending with
/// keyset cursor (limit+1 technique), mirroring `list_member_bindings`.
pub(crate) async fn list_domains(
    pool: &PgPool,
    before: Option<ListMembersCursor>,
    limit: u32,
) -> Result<Vec<DomainFullRow>, ProvisioningError> {
    let query_limit = limit as i64 + 1;
    let rows: Vec<DomainFullRow> = sqlx::query_as(&format!(
        r#"
        SELECT {DOMAIN_FULL_COLUMNS}
        FROM domains
        WHERE ($1::timestamptz IS NULL
               OR (created_at, domain_id) < ($1, $2))
        ORDER BY created_at DESC, domain_id DESC
        LIMIT $3
        "#
    ))
    .bind(before.as_ref().map(|c| c.created_at))
    .bind(before.as_ref().map(|c| c.id))
    .bind(query_limit)
    .fetch_all(pool)
    .await
    .map_err(storage)?;
    Ok(rows)
}

/// Update a domain's display name (V1: the only mutable governance field).
///
/// Row is locked FOR UPDATE and `updated_at` is bumped. Returns the
/// post-image row.
pub(crate) async fn update_domain_display_name(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    display_name: &str,
) -> Result<DomainFullRow, ProvisioningError> {
    let row: DomainFullRow = sqlx::query_as(&format!(
        "UPDATE domains SET display_name = $2, updated_at = now()
         WHERE domain_id = $1
         RETURNING {DOMAIN_FULL_COLUMNS}"
    ))
    .bind(domain_id)
    .bind(display_name)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    .ok_or(ProvisioningError::DomainNotFound)?;
    Ok(row)
}

/// The enabled DOMAIN_OWNER projection of a domain (joined with the
/// principal display name). `None` when the domain has no enabled owner.
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct DomainOwnerRow {
    pub principal_id: Uuid,
    pub display_name: Option<String>,
    pub enabled: bool,
}

/// Read the current enabled DOMAIN_OWNER of a domain.
pub(crate) async fn get_domain_owner(
    pool: &PgPool,
    domain_id: Uuid,
) -> Result<Option<DomainOwnerRow>, ProvisioningError> {
    let row: Option<DomainOwnerRow> = sqlx::query_as(
        "SELECT b.principal_id, p.display_name, p.enabled
         FROM domain_role_bindings b
         JOIN principals p ON p.principal_id = b.principal_id
         WHERE b.domain_id = $1 AND b.role_key = 'DOMAIN_OWNER' AND b.enabled = TRUE",
    )
    .bind(domain_id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?;
    Ok(row)
}

/// Fetch a specific role binding (any enabled state) with its `binding_id`
/// and `enabled` flag — the reconcile preimage read.
pub(crate) async fn get_role_binding(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    principal_id: Uuid,
    role_key: &str,
) -> Result<Option<(Uuid, bool)>, ProvisioningError> {
    let row: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT binding_id, enabled FROM domain_role_bindings
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = $3
         FOR UPDATE",
    )
    .bind(domain_id)
    .bind(principal_id)
    .bind(role_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(row)
}

/// Disable a specific role binding (identity matched by binding_id so the
/// preimage cannot drift between read and write).
pub(crate) async fn disable_role_binding_by_id(
    tx: &mut Transaction<'_, Postgres>,
    binding_id: Uuid,
) -> Result<u64, ProvisioningError> {
    let affected = sqlx::query(
        "UPDATE domain_role_bindings SET enabled = FALSE, disabled_at = now()
         WHERE binding_id = $1 AND enabled = TRUE",
    )
    .bind(binding_id)
    .execute(&mut **tx)
    .await
    .map_err(storage)?
    .rows_affected();
    Ok(affected)
}

/// Establish an enabled role binding.
///
/// Upsert (PR #42 review P1): a disabled historical row for the same
/// `(domain_id, principal_id, role_key)` — unique index from migration 0001 —
/// is re-enabled in place instead of triggering a plain-INSERT unique
/// violation. Enabled duplicates cannot be reached here: callers must have
/// already ruled them out (and the DOMAIN_OWNER partial single-owner index
/// backstops that invariant).
pub(crate) async fn insert_role_binding(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    principal_id: Uuid,
    role_key: &str,
) -> Result<(), ProvisioningError> {
    let binding_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1, $2, $3, $4, TRUE)
         ON CONFLICT (domain_id, principal_id, role_key) DO UPDATE
         SET enabled = TRUE, disabled_at = NULL",
    )
    .bind(binding_id)
    .bind(domain_id)
    .bind(principal_id)
    .bind(role_key)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}

/// Revocation barrier (PR #42 review P1): lock the actor's principal row and
/// coordinator binding rows FOR UPDATE and re-validate both inside the write
/// transaction, so an authority revoked between the pool-level gate and the
/// commit barrier cannot carry the write.
///
/// Returns `Err(PrincipalDisabled)` when the principal is missing or
/// disabled, `Err(NotDomainOwner-equivalent)` shape via `Ok(false)` when no
/// enabled coordinator binding remains.
pub(crate) async fn lock_and_check_coordinator(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
) -> Result<bool, ProvisioningError> {
    let principal_enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM principals WHERE principal_id = $1 FOR UPDATE")
            .bind(actor)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?;
    if principal_enabled != Some(true) {
        return Err(ProvisioningError::PrincipalDisabled);
    }
    let binding_rows: Vec<bool> = sqlx::query_scalar(
        "SELECT enabled FROM global_role_bindings
         WHERE principal_id = $1 AND role_key = 'GLOBAL_WORKFLOW_COORDINATOR'
         FOR UPDATE",
    )
    .bind(actor)
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(binding_rows.contains(&true))
}

/// Write a binding-reconciliation audit row (`resource_type =
/// 'DOMAIN_ROLE_BINDING'`), same transaction as the mutation.
pub(crate) async fn write_binding_audit(
    tx: &mut Transaction<'_, Postgres>,
    actor_principal_id: Uuid,
    action: &str,
    domain_id: Uuid,
    details: &serde_json::Value,
) -> Result<(), ProvisioningError> {
    let audit_id = Uuid::new_v4();
    let resource_id = domain_id.to_string();
    sqlx::query(
        "INSERT INTO workflow_security_audits
         (audit_id, principal_id, action, resource_type, resource_id, details)
         VALUES ($1, $2, $3, 'DOMAIN_ROLE_BINDING', $4, $5)",
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

/// Write a domain-governance audit row (`resource_type = 'DOMAIN'`).
pub(crate) async fn write_domain_audit(
    tx: &mut Transaction<'_, Postgres>,
    actor_principal_id: Uuid,
    action: &str,
    domain_id: Uuid,
    details: &serde_json::Value,
) -> Result<(), ProvisioningError> {
    let audit_id = Uuid::new_v4();
    let resource_id = domain_id.to_string();
    sqlx::query(
        "INSERT INTO workflow_security_audits
         (audit_id, principal_id, action, resource_type, resource_id, details)
         VALUES ($1, $2, $3, 'DOMAIN', $4, $5)",
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
