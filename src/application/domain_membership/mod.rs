//! Domain membership application service.
//!
//! Orchestrates Agent self-projection and Domain Owner member management
//! with idempotent receipt handling and durable audit logging.
//!
//! Receipts use the generic idempotency primitives from the provisioning
//! repository — replay-safe, conflict-detecting, processing-guarded.
//! The receipt is a pure command-journal facility with no provisioning
//! authorization semantic.

mod receipt;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use self::receipt::{complete_and_return_error, compute_receipt_hash, handle_receipt_result};
use crate::domain::provisioning::ProvisioningError;
use crate::store::postgres::domain_role_repository;
use crate::store::postgres::domain_role_repository::{
    ListMembersCursor, ListMembersResult, ProjectionResult,
};
use crate::store::postgres::provisioning_repository::{
    acquire_receipt, complete_receipt, AcquireReceipt,
};

// ---------------------------------------------------------------------------
// Command type constants for idempotent receipts
// ---------------------------------------------------------------------------

const COMMAND_TYPE_MEMBER_ADD: &str = "DOMAIN_MEMBER_ADD";
const COMMAND_TYPE_MEMBER_REMOVE: &str = "DOMAIN_MEMBER_REMOVE";

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during domain membership operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainMembershipError {
    PrincipalNotRegistered,
    PrincipalDisabled,
    PrincipalProjectionConflict,
    DomainNotFound,
    NotDomainOwner,
    PrincipalIsOwner,
    MemberNotFound,
    DirectTokenRequired,
    IdempotencyConflict,
    CommandStillProcessing,
    InternalConsistency(String),
    StorageError(String),
}

impl DomainMembershipError {
    /// Stable HTTP-compatible error label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::PrincipalNotRegistered => "principal_not_registered",
            Self::PrincipalDisabled => "principal_disabled",
            Self::PrincipalProjectionConflict => "principal_projection_conflict",
            Self::DomainNotFound => "domain_not_found",
            Self::NotDomainOwner => "not_domain_owner",
            Self::PrincipalIsOwner => "principal_is_owner",
            Self::MemberNotFound => "member_not_found",
            Self::DirectTokenRequired => "direct_token_required",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::CommandStillProcessing => "command_still_processing",
            Self::InternalConsistency(_) => "internal_consistency_error",
            Self::StorageError(_) => "service_unavailable",
        }
    }

    /// Human-readable detail for error responses.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::InternalConsistency(d) => Some(d),
            Self::StorageError(d) => Some(d),
            _ => None,
        }
    }

    /// HTTP status code.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::PrincipalNotRegistered | Self::DomainNotFound | Self::MemberNotFound => 404,
            Self::PrincipalDisabled | Self::NotDomainOwner | Self::DirectTokenRequired => 403,
            Self::PrincipalProjectionConflict
            | Self::PrincipalIsOwner
            | Self::IdempotencyConflict => 409,
            Self::CommandStillProcessing => 425,
            Self::InternalConsistency(_) => 500,
            Self::StorageError(_) => 503,
        }
    }
}

impl From<ProvisioningError> for DomainMembershipError {
    fn from(e: ProvisioningError) -> Self {
        match e {
            ProvisioningError::PrincipalDisabled => Self::PrincipalDisabled,
            ProvisioningError::PrincipalTypeConflict => Self::PrincipalProjectionConflict,
            ProvisioningError::DomainNotFound => Self::DomainNotFound,
            ProvisioningError::IdempotencyConflict => Self::IdempotencyConflict,
            ProvisioningError::CommandStillProcessing => Self::CommandStillProcessing,
            ProvisioningError::InternalConsistency(d) => Self::InternalConsistency(d),
            ProvisioningError::StorageError(d) => Self::StorageError(d),
            _ => Self::InternalConsistency(format!("unexpected provisioning error: {}", e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SelfProjectionResponse {
    pub principal_id: Uuid,
    pub created: bool,
}

#[derive(Debug, Serialize)]
pub struct MemberItem {
    pub principal_id: Uuid,
    pub principal_type: String,
    pub display_name: String,
    pub role: String,
    pub binding_created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MemberListPage {
    pub items: Vec<MemberItem>,
    pub next_cursor: Option<MemberListCursor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemberListCursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

/// A domain the caller has an enabled role binding in.
#[derive(Debug, Serialize)]
pub struct MyDomainItem {
    pub domain_id: Uuid,
    pub domain_key: String,
    pub display_name: String,
    pub caller_role: String,
    pub binding_created_at: DateTime<Utc>,
}

/// List the caller's own domain memberships.
///
/// Caller-scoped discovery: every domain where `principal_id` has an
/// enabled `DOMAIN_OWNER` / `DOMAIN_MEMBER` binding, joined with the
/// domain's basic info.  Disabled bindings and disabled domains are
/// excluded.  The subject comes exclusively from the verified token —
/// no further authorization check applies.
pub async fn list_my_domains(
    pool: &PgPool,
    principal_id: Uuid,
) -> Result<Vec<MyDomainItem>, DomainMembershipError> {
    let rows = domain_role_repository::list_my_domains(pool, principal_id).await?;
    Ok(rows
        .into_iter()
        .map(|row| MyDomainItem {
            domain_id: row.domain_id,
            domain_key: row.domain_key,
            display_name: row.display_name,
            caller_role: row.role_key,
            binding_created_at: row.binding_created_at,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Self-Projection
// ---------------------------------------------------------------------------

/// Project the caller's verified token identity into the local
/// `principals` table.
///
/// No domain role binding is created.  The projection is a pure
/// identity record.
pub async fn self_project(
    pool: &PgPool,
    principal_id: Uuid,
) -> Result<SelfProjectionResponse, DomainMembershipError> {
    let result = domain_role_repository::upsert_principal_projection(pool, principal_id).await?;
    let created = matches!(result, ProjectionResult::Created);

    // Write a standalone audit record.
    // Self-projection does not use a command receipt since it is
    // purely idempotent via ON CONFLICT and has no request-level
    // idempotency key.
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;
    let details = serde_json::json!({
        "operation": "self_projection",
        "result": if created { "created" } else { "already_exists" },
    });
    domain_role_repository::write_security_audit(
        &mut tx,
        principal_id, // actor = target
        "self_projection",
        principal_id,
        Uuid::default(), // no domain context
        &details,
    )
    .await?;
    tx.commit()
        .await
        .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;

    Ok(SelfProjectionResponse {
        principal_id,
        created,
    })
}

// ---------------------------------------------------------------------------
// Domain Member List
// ---------------------------------------------------------------------------

/// List enabled DOMAIN_MEMBER bindings for a domain.
pub async fn list_members(
    pool: &PgPool,
    actor_id: Uuid,
    domain_id: Uuid,
    before_created_at: Option<DateTime<Utc>>,
    before_id: Option<Uuid>,
    limit: u32,
) -> Result<MemberListPage, DomainMembershipError> {
    // Verify caller is a domain owner inside a read transaction.
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;
    let is_owner = domain_role_repository::check_domain_owner(&mut tx, actor_id, domain_id).await?;
    if !is_owner {
        return Err(DomainMembershipError::NotDomainOwner);
    }
    tx.commit()
        .await
        .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;

    let cursor = match (before_created_at, before_id) {
        (Some(created_at), Some(id)) => Some(ListMembersCursor { created_at, id }),
        _ => None,
    };

    let result: ListMembersResult =
        domain_role_repository::list_member_bindings(pool, domain_id, cursor, limit).await?;

    let items: Vec<MemberItem> = result
        .items
        .into_iter()
        .map(|row| MemberItem {
            principal_id: row.principal_id,
            principal_type: row.principal_type,
            display_name: row.display_name,
            role: row.role,
            binding_created_at: row.binding_created_at,
        })
        .collect();

    let next_cursor = result.next_cursor.map(|c| MemberListCursor {
        created_at: c.created_at,
        id: c.id,
    });

    Ok(MemberListPage { items, next_cursor })
}

// ---------------------------------------------------------------------------
// Domain Member Add
// ---------------------------------------------------------------------------

/// Add a principal as a DOMAIN_MEMBER of a domain.
///
/// Transaction: verify owner + receipt + check target + upsert + audit + complete.
pub async fn add_member(
    pool: &PgPool,
    actor_id: Uuid,
    idempotency_key: &str,
    domain_id: Uuid,
    target_principal_id: Uuid,
    request_id: &str,
) -> Result<serde_json::Value, DomainMembershipError> {
    // Business transaction (auth checks done at HTTP layer).
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;

    // 1. Verify caller is still DOMAIN_OWNER
    let is_owner = domain_role_repository::check_domain_owner(&mut tx, actor_id, domain_id).await?;
    if !is_owner {
        return Err(DomainMembershipError::NotDomainOwner);
    }

    // 1b. Verify domain is enabled
    let domain_enabled = domain_role_repository::check_domain_enabled(&mut tx, domain_id).await?;
    if !domain_enabled {
        return Err(DomainMembershipError::DomainNotFound);
    }

    // 2. Acquire idempotent receipt
    let request_hash = compute_receipt_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_MEMBER_ADD,
        "actorId": actor_id,
        "domainId": domain_id,
        "targetPrincipalId": target_principal_id,
    }));
    let receipt = acquire_receipt(
        &mut tx,
        actor_id,
        idempotency_key,
        COMMAND_TYPE_MEMBER_ADD,
        &request_hash,
    )
    .await?;

    if !receipt.is_owned() {
        tx.commit()
            .await
            .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;
        return handle_receipt_result(receipt);
    }

    // 3. Verify target principal exists and is enabled
    let target_enabled =
        domain_role_repository::check_principal_enabled(pool, target_principal_id).await?;
    match target_enabled {
        None => {
            let err = DomainMembershipError::PrincipalNotRegistered;
            complete_and_return_error(tx, receipt.command_id(), &err).await?;
            return Err(err);
        }
        Some(false) => {
            let err = DomainMembershipError::PrincipalDisabled;
            complete_and_return_error(tx, receipt.command_id(), &err).await?;
            return Err(err);
        }
        Some(true) => {}
    }

    // 4. Verify target is not already DOMAIN_OWNER
    let is_owner = domain_role_repository::check_has_role(
        &mut tx,
        domain_id,
        target_principal_id,
        "DOMAIN_OWNER",
    )
    .await?;
    if is_owner {
        let err = DomainMembershipError::PrincipalIsOwner;
        complete_and_return_error(tx, receipt.command_id(), &err).await?;
        return Err(err);
    }

    // 5. Upsert DOMAIN_MEMBER binding
    domain_role_repository::insert_member_binding(&mut tx, domain_id, target_principal_id).await?;

    // 6. Write security audit
    let details = serde_json::json!({
        "operation": "member_added",
        "actorPrincipalId": actor_id,
        "targetPrincipalId": target_principal_id,
        "domainId": domain_id,
        "requestId": request_id,
        "result": "success",
    });
    domain_role_repository::write_security_audit(
        &mut tx,
        actor_id,
        "member_added",
        target_principal_id,
        domain_id,
        &details,
    )
    .await?;

    // 7. Complete receipt
    let response = serde_json::json!({
        "domainId": domain_id,
        "principalId": target_principal_id,
        "role": "DOMAIN_MEMBER",
    });
    complete_receipt(&mut tx, receipt.command_id(), 200, &response).await?;

    tx.commit()
        .await
        .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;

    // Operational log (not audit)
    tracing::info!(
        request_id = request_id,
        actor = %actor_id,
        operation = "member_added",
        target = %target_principal_id,
        domain = %domain_id,
        result = "success",
        "domain member added"
    );

    Ok(response)
}

// ---------------------------------------------------------------------------
// Domain Member Remove
// ---------------------------------------------------------------------------

/// Remove a DOMAIN_MEMBER binding.
///
/// Transaction: verify owner + receipt + soft-delete + audit + complete.
pub async fn remove_member(
    pool: &PgPool,
    actor_id: Uuid,
    idempotency_key: &str,
    domain_id: Uuid,
    target_principal_id: Uuid,
    request_id: &str,
) -> Result<serde_json::Value, DomainMembershipError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;

    // 1. Verify caller is still DOMAIN_OWNER
    let is_owner = domain_role_repository::check_domain_owner(&mut tx, actor_id, domain_id).await?;
    if !is_owner {
        return Err(DomainMembershipError::NotDomainOwner);
    }

    // 2. Acquire idempotent receipt
    let request_hash = compute_receipt_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_MEMBER_REMOVE,
        "actorId": actor_id,
        "domainId": domain_id,
        "targetPrincipalId": target_principal_id,
    }));
    let receipt = acquire_receipt(
        &mut tx,
        actor_id,
        idempotency_key,
        COMMAND_TYPE_MEMBER_REMOVE,
        &request_hash,
    )
    .await?;

    if !receipt.is_owned() {
        tx.commit()
            .await
            .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;
        return handle_receipt_result(receipt);
    }

    // 3. Soft-delete DOMAIN_MEMBER binding
    let affected =
        domain_role_repository::delete_member_binding(&mut tx, domain_id, target_principal_id)
            .await?;
    if affected == 0 {
        let err = DomainMembershipError::MemberNotFound;
        complete_and_return_error(tx, receipt.command_id(), &err).await?;
        return Err(err);
    }

    // 4. Write security audit
    let details = serde_json::json!({
        "operation": "member_removed",
        "actorPrincipalId": actor_id,
        "targetPrincipalId": target_principal_id,
        "domainId": domain_id,
        "requestId": request_id,
        "result": "success",
    });
    domain_role_repository::write_security_audit(
        &mut tx,
        actor_id,
        "member_removed",
        target_principal_id,
        domain_id,
        &details,
    )
    .await?;

    // 5. Complete receipt
    let response = serde_json::json!({
        "domainId": domain_id,
        "principalId": target_principal_id,
        "role": "DOMAIN_MEMBER",
        "enabled": false,
    });
    complete_receipt(&mut tx, receipt.command_id(), 200, &response).await?;

    tx.commit()
        .await
        .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;

    // Operational log
    tracing::info!(
        request_id = request_id,
        actor = %actor_id,
        operation = "member_removed",
        target = %target_principal_id,
        domain = %domain_id,
        result = "success",
        "domain member removed"
    );

    Ok(response)
}
