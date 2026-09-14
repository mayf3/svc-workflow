//! GLOBAL_WORKFLOW_COORDINATOR control-plane application service
//! (SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1).
//!
//! Coordinator-facing narrow governance surfaces on top of the existing
//! identity tables: domain list/get, displayName-only update, get owner,
//! and binding reconciliation (plan / apply). All authorization is
//! server-side (`global_role_bindings` / `domain_role_bindings`); the
//! HTTP layer carries only coarse scopes. Every write rides the shared
//! `workflow_command_receipts` idempotency machinery (command types
//! `domain.update` / `domain.binding_reconcile`) plus a durable
//! `workflow_security_audits` row in the same transaction.

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::provisioning::ProvisioningError;
use crate::store::postgres::domain_role_repository;
use crate::store::postgres::provisioning_repository::{
    acquire_receipt, complete_receipt, check_global_coordinator, AcquireReceipt,
};

const COMMAND_TYPE_DOMAIN_UPDATE: &str = "domain.update";
const COMMAND_TYPE_BINDING_RECONCILE: &str = "domain.binding_reconcile";

/// Roles a binding reconciliation may migrate.
pub(crate) const RECONCILE_ROLES: [&str; 2] = ["DOMAIN_OWNER", "DOMAIN_MEMBER"];

// ---------------------------------------------------------------------------
// Attempt / read audit helpers
// ---------------------------------------------------------------------------

/// Write a durable attempt-audit row (`workflow_command_attempt_audits`) for
/// a non-owned receipt outcome — CTR-CP-004 requires the attempt trail to
/// cover replays and conflicts, not only first attempts (PR #42 review P1).
async fn write_attempt_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command_id: Uuid,
    actor: Uuid,
    idempotency_key: &str,
    attempt_type: &str,
    request_hash: &str,
) -> Result<(), CoordinatorControlPlaneError> {
    sqlx::query(
        r#"
        INSERT INTO workflow_command_attempt_audits
            (audit_id, command_id, principal_id, idempotency_key, attempt_type,
             failure_reason, request_hash, details)
        VALUES ($1, $2, $3, $4, $5, NULL, $6, NULL)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(command_id)
    .bind(actor)
    .bind(idempotency_key)
    .bind(attempt_type)
    .bind(request_hash)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}

/// Durable audit-before-return for the coordinator read surfaces
/// (PR #42 review P1: protected directory reads must commit an audit row
/// before publication and fail closed on audit-store failure).
async fn audit_read(
    pool: &PgPool,
    actor: Uuid,
    action: &str,
    domain_id: Option<Uuid>,
) -> Result<(), CoordinatorControlPlaneError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
    domain_role_repository::write_domain_audit(
        &mut tx,
        actor,
        action,
        domain_id.unwrap_or(Uuid::nil()),
        &serde_json::json!({
            "operation": action,
            "actorPrincipalId": actor,
            "authorityBasis": "GLOBAL_WORKFLOW_COORDINATOR",
            "read": true,
        }),
    )
    .await
    .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    tx.commit()
        .await
        .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by the coordinator control-plane surfaces. Labels are
/// the stable wire codes frozen in CTR-CP-003.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatorControlPlaneError {
    DomainNotFound,
    DomainOwnerMissing,
    NotDomainOwner,
    IdentityNotFound,
    PrincipalDisabled,
    BindingConflict,
    InvalidInput(String),
    IdempotencyConflict,
    CommandStillProcessing,
    InternalConsistency(String),
    StorageError(String),
}

impl CoordinatorControlPlaneError {
    /// Stable HTTP-compatible error label (static wire codes, CTR-CP-003).
    pub fn label(&self) -> &'static str {
        match self {
            Self::DomainNotFound => "domain_not_found",
            Self::DomainOwnerMissing => "domain_owner_missing",
            Self::NotDomainOwner => "not_domain_owner",
            Self::IdentityNotFound => "identity_not_found",
            Self::PrincipalDisabled => "principal_disabled",
            Self::BindingConflict => "binding_conflict",
            Self::InvalidInput(_) => "invalid_input",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::CommandStillProcessing => "command_still_processing",
            Self::InternalConsistency(_) => "internal_consistency_error",
            Self::StorageError(_) => "service_unavailable",
        }
    }

    /// HTTP status for the error.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::DomainNotFound | Self::DomainOwnerMissing | Self::IdentityNotFound => 404,
            Self::NotDomainOwner | Self::PrincipalDisabled => 403,
            Self::BindingConflict | Self::IdempotencyConflict => 409,
            Self::InvalidInput(_) => 422,
            Self::CommandStillProcessing => 425,
            Self::InternalConsistency(_) => 500,
            Self::StorageError(_) => 503,
        }
    }

    fn from_provisioning(error: ProvisioningError) -> Self {
        use ProvisioningError as E;
        match error {
            E::DomainNotFound => Self::DomainNotFound,
            E::PrincipalDisabled => Self::PrincipalDisabled,
            E::IdempotencyConflict => Self::IdempotencyConflict,
            E::CommandStillProcessing => Self::CommandStillProcessing,
            E::InternalConsistency(d) => Self::InternalConsistency(d),
            E::StorageError(d) => Self::StorageError(d),
            other => Self::InternalConsistency(format!(
                "unexpected provisioning error: {}",
                other.label()
            )),
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::InvalidInput(d) | Self::InternalConsistency(d) | Self::StorageError(d) => {
                Some(d)
            }
            _ => None,
        }
    }
}

fn storage(error: sqlx::Error) -> CoordinatorControlPlaneError {
    CoordinatorControlPlaneError::StorageError(error.to_string())
}

// ---------------------------------------------------------------------------
// Shared validation
// ---------------------------------------------------------------------------

/// Mirror of the create-domain displayName validation (1..=256, trimmed,
/// no control characters).
fn validate_display_name(name: &str) -> Result<(), CoordinatorControlPlaneError> {
    if name.is_empty()
        || name != name.trim()
        || name.len() > 256
        || name.chars().any(char::is_control)
    {
        return Err(CoordinatorControlPlaneError::InvalidInput(
            "displayName is invalid".to_string(),
        ));
    }
    Ok(())
}

/// Reconcile role must be one of the frozen two; reason must be a
/// non-empty bounded string (malformed input -> `invalid_input` 422,
/// per CTR-CP-003).
fn validate_reconcile_input(
    role: &str,
    reason: &str,
) -> Result<(), CoordinatorControlPlaneError> {
    if !RECONCILE_ROLES.contains(&role) {
        return Err(CoordinatorControlPlaneError::InvalidInput(
            "role must be DOMAIN_OWNER or DOMAIN_MEMBER".to_string(),
        ));
    }
    if reason.is_empty() || reason.len() > 512 || reason.chars().any(char::is_control) {
        return Err(CoordinatorControlPlaneError::InvalidInput(
            "reason is invalid".to_string(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Read surfaces
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainGovernanceEntry {
    pub domain_id: Uuid,
    pub domain_key: String,
    pub display_name: String,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn entry_json(row: &domain_role_repository::DomainFullRow) -> DomainGovernanceEntry {
    DomainGovernanceEntry {
        domain_id: row.domain_id,
        domain_key: row.domain_key.clone(),
        display_name: row.display_name.clone(),
        enabled: row.enabled,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

/// Verify the caller holds the enabled GLOBAL_WORKFLOW_COORDINATOR
/// binding (server-side; never the JWT).
async fn require_global_coordinator(
    pool: &PgPool,
    actor: Uuid,
) -> Result<(), CoordinatorControlPlaneError> {
    let is_coordinator = check_global_coordinator(pool, actor)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    if !is_coordinator {
        return Err(CoordinatorControlPlaneError::NotDomainOwner);
    }
    Ok(())
}

/// GET /internal/v1/domains — keyset-paged list of minimal governance
/// metadata (CTR-CP-002).
pub async fn list_domains(
    pool: &PgPool,
    actor: Uuid,
    before_created_at: Option<chrono::DateTime<chrono::Utc>>,
    before_id: Option<Uuid>,
    limit: u32,
) -> Result<serde_json::Value, CoordinatorControlPlaneError> {
    require_global_coordinator(pool, actor).await?;
    let before = before_created_at.zip(before_id).map(|(created_at, id)| {
        domain_role_repository::ListMembersCursor { created_at, id }
    });
    let rows = domain_role_repository::list_domains(pool, before, limit)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?;

    let has_more = rows.len() > limit as usize;
    let items: Vec<serde_json::Value> = rows
        .iter()
        .take(limit as usize)
        .map(|r| serde_json::to_value(entry_json(r)).expect("serializable entry"))
        .collect();
    let mut body = serde_json::json!({ "items": items });
    if has_more && limit > 0 {
        // Cursor is built from the last INCLUDED item, not rows.last():
        // rows holds limit+1 records and the extra record is never exposed,
        // so keying the cursor on it would skip that domain permanently
        // under the strict `<` keyset comparison (PR #42 review P2).
        if let Some(last) = rows.get(limit as usize - 1) {
            body["nextBeforeCreatedAt"] =
                serde_json::Value::String(last.created_at.to_rfc3339());
            body["nextBeforeId"] = serde_json::Value::String(last.domain_id.to_string());
        }
    }
    audit_read(pool, actor, "coordinator_domain_list", None).await?;
    Ok(body)
}

/// GET /internal/v1/domains/{domainId} — one domain's governance metadata.
pub async fn get_domain(
    pool: &PgPool,
    actor: Uuid,
    domain_id: Uuid,
) -> Result<serde_json::Value, CoordinatorControlPlaneError> {
    require_global_coordinator(pool, actor).await?;
    let row = domain_role_repository::get_domain_full(pool, domain_id)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?
        .ok_or(CoordinatorControlPlaneError::DomainNotFound)?;
    audit_read(pool, actor, "coordinator_domain_get", Some(domain_id)).await?;
    Ok(serde_json::to_value(entry_json(&row)).expect("serializable entry"))
}

/// GET /internal/v1/domains/{domainId}/owner — coordinator OR the domain's
/// own enabled owner (CTR-CP-001 / DEC-CP-004). Returns
/// `domain_owner_missing` (404) when no enabled owner exists.
pub async fn get_domain_owner(
    pool: &PgPool,
    actor: Uuid,
    domain_id: Uuid,
) -> Result<serde_json::Value, CoordinatorControlPlaneError> {
    let is_coordinator = check_global_coordinator(pool, actor)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    if !is_coordinator {
        let is_owner = domain_role_repository::pool_check_domain_owner(pool, actor, domain_id)
            .await
            .map_err(CoordinatorControlPlaneError::from_provisioning)?;
        if !is_owner {
            return Err(CoordinatorControlPlaneError::NotDomainOwner);
        }
    }

    // Domain existence gate (a disabled domain is still discoverable for
    // owner lookup by its coordinator; a missing one is not).
    domain_role_repository::get_domain_full(pool, domain_id)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?
        .ok_or(CoordinatorControlPlaneError::DomainNotFound)?;

    let owner = domain_role_repository::get_domain_owner(pool, domain_id)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?
        .ok_or(CoordinatorControlPlaneError::DomainOwnerMissing)?;

    audit_read(pool, actor, "coordinator_domain_owner_get", Some(domain_id)).await?;
    Ok(serde_json::json!({
        "domainId": domain_id,
        "ownerPrincipalId": owner.principal_id,
        "ownerDisplayName": owner.display_name,
        "ownerEnabled": owner.enabled,
    }))
}

// ---------------------------------------------------------------------------
// domain.update (receipted write)
// ---------------------------------------------------------------------------

/// PATCH /internal/v1/domains/{domainId} — displayName-only governance
/// update (DEC-CP-003). Receipt command type `domain.update`; a durable
/// security audit row is written in the same transaction.
pub async fn update_domain(
    pool: &PgPool,
    actor: Uuid,
    domain_id: Uuid,
    display_name: &str,
    idempotency_key: &str,
    request_id: &str,
) -> Result<serde_json::Value, CoordinatorControlPlaneError> {
    require_global_coordinator(pool, actor).await?;
    validate_display_name(display_name)?;

    let request_hash = compute_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_DOMAIN_UPDATE,
        "domainId": domain_id,
        "displayName": display_name,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
    let receipt = acquire_receipt(
        &mut tx,
        actor,
        idempotency_key,
        COMMAND_TYPE_DOMAIN_UPDATE,
        &request_hash,
    )
    .await
    .map_err(CoordinatorControlPlaneError::from_provisioning)?;

    if !receipt.is_owned() {
        // CTR-CP-004: the attempt trail covers replays too — record the
        // attempt before returning the durable outcome (PR #42 review P1).
        // The preimage transaction is committed FIRST: acquire_receipt holds
        // a lock on the receipt row, and the attempt-audit insert takes an
        // FK key-share on that same row, so writing it on a second
        // connection before committing would self-deadlock.
        tx.commit()
            .await
            .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
        if let AcquireReceipt::Replay { .. } = receipt {
            let mut audit_tx = pool
                .begin()
                .await
                .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
            write_attempt_audit(
                &mut audit_tx,
                receipt.command_id(),
                actor,
                idempotency_key,
                "replay",
                &request_hash,
            )
            .await?;
            audit_tx
                .commit()
                .await
                .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
        }
        return handle_receipt_result(receipt);
    }

    // Revocation barrier (PR #42 review P1): re-lock the actor's principal
    // and coordinator binding rows inside the write transaction; an
    // authority revoked after the pool-level gate must not carry the write.
    let still_coordinator = domain_role_repository::lock_and_check_coordinator(&mut tx, actor)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    if !still_coordinator {
        return fail_receipt(
            tx,
            receipt,
            CoordinatorControlPlaneError::NotDomainOwner,
        )
        .await;
    }

    let row = match domain_role_repository::update_domain_display_name(
        &mut tx,
        domain_id,
        display_name,
    )
    .await
    {
        Ok(row) => row,
        // Expected business failure: complete + commit the receipt so the
        // same key replays the stable 404 instead of minting a fresh attempt
        // (CTR-CP-004; PR #42 review P2).
        Err(ProvisioningError::DomainNotFound) => {
            return fail_receipt(
                tx,
                receipt,
                CoordinatorControlPlaneError::DomainNotFound,
            )
            .await
        }
        // Infrastructure failure: roll the transaction (and the receipt
        // acquisition) back.
        Err(e) => return Err(CoordinatorControlPlaneError::from_provisioning(e)),
    };

    domain_role_repository::write_domain_audit(
        &mut tx,
        actor,
        "domain_updated",
        domain_id,
        &serde_json::json!({
            "operation": "domain_updated",
            "actorPrincipalId": actor,
            "domainId": domain_id,
            "displayName": display_name,
            "requestId": request_id,
            "result": "success",
        }),
    )
    .await
    .map_err(CoordinatorControlPlaneError::from_provisioning)?;

    let response = serde_json::to_value(entry_json(&row)).expect("serializable entry");
    complete_receipt(&mut tx, receipt.command_id(), 200, &response)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?;

    tx.commit()
        .await
        .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;

    tracing::info!(
        request_id = request_id,
        actor = %actor,
        operation = "domain_updated",
        domain = %domain_id,
        result = "success",
        "domain governance metadata updated"
    );

    Ok(response)
}

// ---------------------------------------------------------------------------
// Binding reconciliation (plan read-only; apply receipted)
// ---------------------------------------------------------------------------

/// Read-only reconciliation judgment for one (domain, role,
/// fromPrincipalId, toPrincipalId) tuple (DEC-CP-005/DEC-CP-007).
///
/// The source principal's `enabled` state is reported but is NEVER a
/// blocker — a disabled stale principal with an exact active source
/// binding is precisely the repairable input. The target principal must
/// exist and be enabled for the apply to be legal.
pub async fn reconcile_plan(
    pool: &PgPool,
    actor: Uuid,
    domain_id: Uuid,
    role: &str,
    from_principal_id: Uuid,
    to_principal_id: Uuid,
    reason: &str,
) -> Result<serde_json::Value, CoordinatorControlPlaneError> {
    require_global_coordinator(pool, actor).await?;
    validate_reconcile_input(role, reason)?;

    // Domain existence gate (read-only).
    domain_role_repository::get_domain_full(pool, domain_id)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?
        .ok_or(CoordinatorControlPlaneError::DomainNotFound)?;

    // One read-only snapshot transaction for the binding reads.
    let mut read_tx = pool
        .begin()
        .await
        .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
    let source_enabled = domain_role_repository::check_principal_enabled_tx(
        &mut read_tx,
        from_principal_id,
    )
    .await
    .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    let target_enabled = domain_role_repository::check_principal_enabled_tx(
        &mut read_tx,
        to_principal_id,
    )
    .await
    .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    let source_binding = domain_role_repository::get_role_binding(
        &mut read_tx,
        domain_id,
        from_principal_id,
        role,
    )
    .await
    .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    let target_has_enabled_binding =
        domain_role_repository::check_has_role(&mut read_tx, domain_id, to_principal_id, role)
            .await
            .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    read_tx
        .commit()
        .await
        .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;

    let source_principal_exists = source_enabled.is_some();
    let source_principal_enabled = source_enabled.unwrap_or(false);
    let target_principal_exists = target_enabled.is_some();
    let target_principal_enabled = target_enabled.unwrap_or(false);
    let source_binding_exists = source_binding.is_some();
    let source_binding_enabled = source_binding.map(|(_, enabled)| enabled).unwrap_or(false);

    let mut blockers: Vec<&str> = Vec::new();
    if !source_principal_exists {
        blockers.push("source_principal_not_found");
    }
    if !source_binding_exists {
        blockers.push("source_binding_missing");
    } else if !source_binding_enabled {
        blockers.push("source_binding_disabled");
    }
    if !target_principal_exists {
        blockers.push("target_principal_not_found");
    } else if !target_principal_enabled {
        blockers.push("target_principal_disabled");
    }
    let single_owner_invariant_ok = if role == "DOMAIN_OWNER"
        && target_has_enabled_binding
        && from_principal_id != to_principal_id
    {
        blockers.push("single_owner_invariant_conflict");
        false
    } else {
        if target_has_enabled_binding && from_principal_id != to_principal_id {
            blockers.push("target_already_has_enabled_binding");
        }
        true
    };

    let plan = if blockers.is_empty() {
        format!(
            "disable the enabled {} binding of {from_principal_id} and establish the enabled {} binding for {to_principal_id} in one transaction",
            role, role
        )
    } else {
        "no-op — blockers present".to_string()
    };

    audit_read(pool, actor, "coordinator_reconcile_plan", Some(domain_id)).await?;
    Ok(serde_json::json!({
        "domainId": domain_id,
        "role": role,
        "sourcePrincipalExists": source_principal_exists,
        "sourcePrincipalEnabled": source_principal_enabled,
        "sourceBindingExists": source_binding_exists,
        "sourceBindingEnabled": source_binding_enabled,
        "targetPrincipalExists": target_principal_exists,
        "targetPrincipalEnabled": target_principal_enabled,
        "targetHasEnabledBinding": target_has_enabled_binding,
        "singleOwnerInvariantOk": single_owner_invariant_ok,
        "plan": plan,
        "blockers": blockers,
        "reason": reason,
    }))
}

/// POST /internal/v1/domains/{domainId}/binding-reconcile/apply — atomic
/// binding migration behind exact-preimage re-assertion (DEC-CP-005/006).
///
/// Receipt command type `domain.binding_reconcile`. Outcomes:
/// `applied` (real migration), `noop` (from == to, nothing to migrate);
/// same-key replay returns the original receipt response. Any preimage
/// drift is a 409 `binding_conflict` with zero mutation.
pub async fn reconcile_apply(
    pool: &PgPool,
    actor: Uuid,
    domain_id: Uuid,
    role: &str,
    from_principal_id: Uuid,
    to_principal_id: Uuid,
    reason: &str,
    idempotency_key: &str,
    request_id: &str,
) -> Result<serde_json::Value, CoordinatorControlPlaneError> {
    require_global_coordinator(pool, actor).await?;
    validate_reconcile_input(role, reason)?;

    let request_hash = compute_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_BINDING_RECONCILE,
        "domainId": domain_id,
        "role": role,
        "fromPrincipalId": from_principal_id,
        "toPrincipalId": to_principal_id,
        "reason": reason,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
    let receipt = acquire_receipt(
        &mut tx,
        actor,
        idempotency_key,
        COMMAND_TYPE_BINDING_RECONCILE,
        &request_hash,
    )
    .await
    .map_err(CoordinatorControlPlaneError::from_provisioning)?;

    if !receipt.is_owned() {
        // CTR-CP-004: the attempt trail covers replays too — record the
        // attempt before returning the durable outcome (PR #42 review P1).
        // The preimage transaction is committed FIRST: acquire_receipt holds
        // a lock on the receipt row, and the attempt-audit insert takes an
        // FK key-share on that same row, so writing it on a second
        // connection before committing would self-deadlock.
        tx.commit()
            .await
            .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
        if let AcquireReceipt::Replay { .. } = receipt {
            let mut audit_tx = pool
                .begin()
                .await
                .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
            write_attempt_audit(
                &mut audit_tx,
                receipt.command_id(),
                actor,
                idempotency_key,
                "replay",
                &request_hash,
            )
            .await?;
            audit_tx
                .commit()
                .await
                .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
        }
        return handle_receipt_result(receipt);
    }

    // Revocation barrier (PR #42 review P1): same in-tx revalidation as
    // domain.update — revoked authority must not commit.
    let still_coordinator = domain_role_repository::lock_and_check_coordinator(&mut tx, actor)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    if !still_coordinator {
        return fail_receipt(
            tx,
            receipt,
            CoordinatorControlPlaneError::NotDomainOwner,
        )
        .await;
    }

    // Domain enabled gate (mirrors member management's step 1b).
    let domain_enabled = domain_role_repository::check_domain_enabled(&mut tx, domain_id)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    if !domain_enabled {
        return fail_receipt(
            tx,
            receipt,
            CoordinatorControlPlaneError::DomainNotFound,
        )
        .await;
    }

    // Exact source-binding preimage: must exist AND be enabled.
    let source_binding =
        domain_role_repository::get_role_binding(&mut tx, domain_id, from_principal_id, role)
            .await
            .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    let (source_binding_id, source_enabled) = match source_binding {
        Some(pair) => pair,
        None => {
            // A missing binding with a missing source principal is an
            // identity miss (404), not a preimage conflict (PR #42 review
            // P2: DEC-CP-007 / CTR-CP-003 — an absent exact UUID on either
            // side returns identity_not_found).
            let source_exists = domain_role_repository::check_principal_enabled_tx(
                &mut tx,
                from_principal_id,
            )
            .await
            .map_err(CoordinatorControlPlaneError::from_provisioning)?
            .is_some();
            return fail_receipt(
                tx,
                receipt,
                if source_exists {
                    CoordinatorControlPlaneError::BindingConflict
                } else {
                    CoordinatorControlPlaneError::IdentityNotFound
                },
            )
            .await;
        }
    };
    if !source_enabled {
        // Completed-migration recognition (CTR-CP-002 `already_applied`;
        // PR #42 review P2): a NEW key repeating an already-applied
        // reconciliation sees a disabled source binding, but the target now
        // holds the enabled binding — return the stable business outcome
        // instead of a 409.
        if from_principal_id != to_principal_id {
            let target_has_enabled =
                domain_role_repository::check_has_role(&mut tx, domain_id, to_principal_id, role)
                    .await
                    .map_err(CoordinatorControlPlaneError::from_provisioning)?;
            if target_has_enabled {
                domain_role_repository::write_binding_audit(
                    &mut tx,
                    actor,
                    "binding_reconciled",
                    domain_id,
                    &serde_json::json!({
                        "operation": "binding_reconciled",
                        "actorPrincipalId": actor,
                        "domainId": domain_id,
                        "role": role,
                        "fromPrincipalId": from_principal_id,
                        "toPrincipalId": to_principal_id,
                        "requestId": request_id,
                        "reason": reason,
                        "authorityBasis": "GLOBAL_WORKFLOW_COORDINATOR",
                        "result": "already_applied",
                    }),
                )
                .await
                .map_err(CoordinatorControlPlaneError::from_provisioning)?;
                let response = serde_json::json!({
                    "domainId": domain_id,
                    "role": role,
                    "fromPrincipalId": from_principal_id,
                    "toPrincipalId": to_principal_id,
                    "outcome": "already_applied",
                });
                complete_receipt(&mut tx, receipt.command_id(), 200, &response)
                    .await
                    .map_err(CoordinatorControlPlaneError::from_provisioning)?;
                tx.commit()
                    .await
                    .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
                return Ok(response);
            }
        }
        return fail_receipt(
            tx,
            receipt,
            CoordinatorControlPlaneError::BindingConflict,
        )
        .await;
    }

    // Target principal: must exist; must be enabled (from disabled is NOT
    // an error — it is the repairable input).
    match domain_role_repository::check_principal_enabled_tx(&mut tx, to_principal_id)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?
    {
        None => {
            return fail_receipt(
                tx,
                receipt,
                CoordinatorControlPlaneError::IdentityNotFound,
            )
            .await
        }
        Some(false) => {
            return fail_receipt(
                tx,
                receipt,
                CoordinatorControlPlaneError::PrincipalDisabled,
            )
            .await
        }
        Some(true) => {}
    }

    let outcome = if from_principal_id == to_principal_id {
        // Nothing to migrate; the binding is already canonical.
        "noop"
    } else {
        // Target must not already hold an enabled binding of this role.
        let target_has_enabled =
            domain_role_repository::check_has_role(&mut tx, domain_id, to_principal_id, role)
                .await
                .map_err(CoordinatorControlPlaneError::from_provisioning)?;
        if target_has_enabled {
            return fail_receipt(
                tx,
                receipt,
                CoordinatorControlPlaneError::BindingConflict,
            )
            .await;
        }

        // Atomic disable-old + establish-new. For DOMAIN_OWNER the
        // `idx_drb_single_owner` partial unique index backstops the
        // single-enabled-owner invariant inside this same transaction.
        domain_role_repository::disable_role_binding_by_id(&mut tx, source_binding_id)
            .await
            .map_err(CoordinatorControlPlaneError::from_provisioning)?;
        domain_role_repository::insert_role_binding(&mut tx, domain_id, to_principal_id, role)
            .await
            .map_err(CoordinatorControlPlaneError::from_provisioning)?;
        "applied"
    };

    domain_role_repository::write_binding_audit(
        &mut tx,
        actor,
        "binding_reconciled",
        domain_id,
        &serde_json::json!({
            "operation": "binding_reconciled",
            "actorPrincipalId": actor,
            "domainId": domain_id,
            "role": role,
            "fromPrincipalId": from_principal_id,
            "toPrincipalId": to_principal_id,
            "requestId": request_id,
            "reason": reason,
            "authorityBasis": "GLOBAL_WORKFLOW_COORDINATOR",
            "result": outcome,
        }),
    )
    .await
    .map_err(CoordinatorControlPlaneError::from_provisioning)?;

    let response = serde_json::json!({
        "domainId": domain_id,
        "role": role,
        "fromPrincipalId": from_principal_id,
        "toPrincipalId": to_principal_id,
        "outcome": outcome,
    });
    complete_receipt(&mut tx, receipt.command_id(), 200, &response)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?;

    tx.commit()
        .await
        .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;

    tracing::info!(
        request_id = request_id,
        actor = %actor,
        operation = "binding_reconciled",
        domain = %domain_id,
        role = %role,
        result = %outcome,
        "domain binding reconciled"
    );

    Ok(response)
}

/// Complete the receipt with a failure response and return the error —
/// the fail-loud counterpart of the provisioning `should_complete_receipt`
/// flow, so a preimage conflict is durably recorded AND surfaced. The
/// transaction is committed with the failure receipt (zero business
/// mutation happened on this path).
async fn fail_receipt(
    mut tx: sqlx::Transaction<'_, sqlx::Postgres>,
    receipt: AcquireReceipt,
    error: CoordinatorControlPlaneError,
) -> Result<serde_json::Value, CoordinatorControlPlaneError> {
    let command_id = match &receipt {
        AcquireReceipt::Owned(command_id) => *command_id,
        _ => unreachable!("fail_receipt is only called on owned receipts"),
    };
    let body = serde_json::json!({ "error": error.label() });
    let status = error.status_code() as i32;
    complete_receipt(&mut tx, command_id, status, &body)
        .await
        .map_err(CoordinatorControlPlaneError::from_provisioning)?;
    tx.commit()
        .await
        .map_err(|e| CoordinatorControlPlaneError::StorageError(e.to_string()))?;
    Err(error)
}

// ---------------------------------------------------------------------------
// Receipt helpers (mirrors provisioning/receipt.rs)
// ---------------------------------------------------------------------------

fn compute_hash(body: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let canonical = serde_json::to_string(body).unwrap_or_else(|_| body.to_string());
    hex::encode(Sha256::digest(canonical.as_bytes()))
}

fn handle_receipt_result(
    receipt: AcquireReceipt,
) -> Result<serde_json::Value, CoordinatorControlPlaneError> {
    match receipt {
        AcquireReceipt::Replay {
            response_status: 200,
            response_body,
            ..
        } => Ok(response_body),
        AcquireReceipt::Replay { response_body, .. } => Err(error_from_body(&response_body)),
        AcquireReceipt::Conflict { .. } => Err(CoordinatorControlPlaneError::IdempotencyConflict),
        AcquireReceipt::Processing { .. } => {
            Err(CoordinatorControlPlaneError::CommandStillProcessing)
        }
        AcquireReceipt::Owned(_) => unreachable!("owned receipt handled by caller"),
    }
}

fn error_from_body(body: &serde_json::Value) -> CoordinatorControlPlaneError {
    match body.get("error").and_then(serde_json::Value::as_str) {
        Some("domain_not_found") => CoordinatorControlPlaneError::DomainNotFound,
        Some("domain_owner_missing") => CoordinatorControlPlaneError::DomainOwnerMissing,
        Some("not_domain_owner") => CoordinatorControlPlaneError::NotDomainOwner,
        Some("identity_not_found") => CoordinatorControlPlaneError::IdentityNotFound,
        Some("principal_disabled") => CoordinatorControlPlaneError::PrincipalDisabled,
        Some("binding_conflict") => CoordinatorControlPlaneError::BindingConflict,
        Some("invalid_input") => {
            CoordinatorControlPlaneError::InvalidInput("see original response".to_string())
        }
        Some("idempotency_conflict") => CoordinatorControlPlaneError::IdempotencyConflict,
        Some("command_still_processing") => {
            CoordinatorControlPlaneError::CommandStillProcessing
        }
        _ => CoordinatorControlPlaneError::InternalConsistency(
            "completed receipt contains an unknown error".to_string(),
        ),
    }
}

