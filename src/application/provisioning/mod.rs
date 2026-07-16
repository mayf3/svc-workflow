//! Identity provisioning application service.
//!
//! Orchestrates principal, domain, and role binding provisioning operations
//! with idempotent receipt handling and structured audit logging.

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::ids::{DefinitionVersionId, DomainId, PrincipalId};
use crate::domain::provisioning::*;
use crate::store::postgres::provisioning_repository;

/// Configuration for provisioning authorization.
#[derive(Debug, Clone)]
pub struct ProvisioningConfig {
    pub allowlist: Vec<PrincipalId>,
}

impl ProvisioningConfig {
    /// Create a ProvisioningConfig with an explicit allow-list (for testing).
    pub fn new(allowlist: Vec<PrincipalId>) -> Self {
        Self { allowlist }
    }

    pub fn from_env() -> Result<Self, String> {
        let raw = std::env::var("WORKFLOW_PROVISIONING_PRINCIPAL_IDS").unwrap_or_default();
        let allowlist = if raw.is_empty() {
            return Err("WORKFLOW_PROVISIONING_PRINCIPAL_IDS is required".to_string());
        } else {
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .map(|s| {
                    Uuid::parse_str(&s)
                        .map(PrincipalId::from_uuid)
                        .map_err(|_| {
                            format!("invalid UUID in WORKFLOW_PROVISIONING_PRINCIPAL_IDS: {s}")
                        })
                })
                .collect::<Result<Vec<_>, String>>()?
        };
        if allowlist.is_empty() {
            return Err(
                "WORKFLOW_PROVISIONING_PRINCIPAL_IDS must contain at least one UUID".to_string(),
            );
        }
        Ok(Self { allowlist })
    }

    pub fn is_allowed(&self, principal_id: &PrincipalId) -> bool {
        self.allowlist.contains(principal_id)
    }
}

/// Check if a provisioning log is needed and write it.
fn log_provisioning(
    request_id: &str,
    actor: &PrincipalId,
    operation: &str,
    target: &str,
    result: &str,
) {
    tracing::info!(
        request_id = request_id,
        actor = %actor,
        operation = operation,
        target = target,
        result = result,
        "provisioning operation"
    );
}

/// Compute a SHA-256 request hash from a JSON-serializable body.
fn compute_hash(body: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let canonical = serde_json::to_string(body).unwrap_or_else(|_| body.to_string());
    hex::encode(Sha256::digest(canonical.as_bytes()))
}

/// Provision (upsert) a principal.
pub async fn provision_principal(
    pool: &PgPool,
    cmd: &ProvisionPrincipalCommand,
    idempotency_key: &str,
    request_id: &str,
    actor_principal_id: &PrincipalId,
) -> Result<serde_json::Value, ProvisioningError> {
    let principal_uuid = cmd.principal_id.into_uuid();
    let request_hash = compute_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_PROVISION_PRINCIPAL,
        "principalId": principal_uuid,
        "principalType": cmd.principal_type,
        "enabled": cmd.enabled,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    let receipt = provisioning_repository::acquire_receipt(
        &mut tx,
        actor_principal_id.into_uuid(),
        idempotency_key,
        COMMAND_TYPE_PROVISION_PRINCIPAL,
        &request_hash,
    )
    .await?;

    if !receipt.is_owned() {
        tx.commit()
            .await
            .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
        return handle_receipt_result(receipt);
    }

    let result = provisioning_repository::upsert_principal(
        &mut tx,
        principal_uuid,
        &cmd.principal_type,
        cmd.enabled,
        &cmd.source,
        cmd.source_revision.as_deref(),
    )
    .await;

    let (response, status): (serde_json::Value, i32) = match &result {
        Ok(id) => (
            serde_json::json!({"principalId": id, "enabled": cmd.enabled}),
            200,
        ),
        Err(e) => (
            serde_json::json!({"error": e.label()}),
            e.status_code() as i32,
        ),
    };

    if result.is_ok() {
        provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
            .await?;
    }

    tx.commit()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;

    log_provisioning(
        request_id,
        &cmd.principal_id,
        "provision_principal",
        &principal_uuid.to_string(),
        if result.is_ok() { "success" } else { "failure" },
    );

    result.map(|_| response)
}

/// Get a principal by ID.
pub async fn get_principal(
    pool: &PgPool,
    principal_id: PrincipalId,
) -> Result<serde_json::Value, ProvisioningError> {
    let uuid = principal_id.into_uuid();
    let row = provisioning_repository::get_principal(pool, uuid).await?;
    match row {
        Some(r) => Ok(serde_json::json!({
            "principalId": r.principal_id,
            "principalType": r.principal_type,
            "enabled": r.enabled,
        })),
        None => Err(ProvisioningError::PrincipalNotFound),
    }
}

/// Provision (upsert) a domain.
pub async fn provision_domain(
    pool: &PgPool,
    cmd: &ProvisionDomainCommand,
    idempotency_key: &str,
    request_id: &str,
    actor_principal_id: &PrincipalId,
) -> Result<serde_json::Value, ProvisioningError> {
    let domain_uuid = cmd.domain_id.into_uuid();
    let request_hash = compute_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_PROVISION_DOMAIN,
        "domainId": domain_uuid,
        "domainKey": cmd.domain_key,
        "enabled": cmd.enabled,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    let receipt = provisioning_repository::acquire_receipt(
        &mut tx,
        actor_principal_id.into_uuid(),
        idempotency_key,
        COMMAND_TYPE_PROVISION_DOMAIN,
        &request_hash,
    )
    .await?;

    if !receipt.is_owned() {
        tx.commit()
            .await
            .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
        return handle_receipt_result(receipt);
    }

    let result = provisioning_repository::upsert_domain(
        &mut tx,
        domain_uuid,
        &cmd.domain_key,
        cmd.display_name.as_deref(),
        cmd.enabled,
    )
    .await;

    let response = match &result {
        Ok(id) => {
            serde_json::json!({"domainId": id, "domainKey": cmd.domain_key, "enabled": cmd.enabled})
        }
        Err(e) => serde_json::json!({"error": e.label()}),
    };

    let status = provisioning_status(&result);

    if result.is_ok() {
        provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
            .await?;
    }

    tx.commit()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;

    log_provisioning(
        request_id,
        actor_principal_id,
        "provision_domain",
        &domain_uuid.to_string(),
        if result.is_ok() { "success" } else { "failure" },
    );
    result.map(|_| response)
}

/// Get a domain by ID.
pub async fn get_domain(
    pool: &PgPool,
    domain_id: DomainId,
) -> Result<serde_json::Value, ProvisioningError> {
    let uuid = domain_id.into_uuid();
    let row = provisioning_repository::get_domain(pool, uuid).await?;
    match row {
        Some(r) => Ok(serde_json::json!({
            "domainId": r.domain_id,
            "domainKey": r.domain_key,
            "displayName": r.display_name,
            "enabled": r.enabled,
        })),
        None => Err(ProvisioningError::DomainNotFound),
    }
}

/// Provision a role binding.
pub async fn provision_role_binding(
    pool: &PgPool,
    cmd: &ProvisionRoleBindingCommand,
    idempotency_key: &str,
    request_id: &str,
    actor: &PrincipalId,
) -> Result<serde_json::Value, ProvisioningError> {
    let domain_uuid = cmd.domain_id.into_uuid();
    let principal_uuid = cmd.principal_id.into_uuid();
    let request_hash = compute_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_PROVISION_ROLE_BINDING,
        "domainId": domain_uuid,
        "principalId": principal_uuid,
        "roleKey": cmd.role_key,
        "enabled": cmd.enabled,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    let receipt = provisioning_repository::acquire_receipt(
        &mut tx,
        actor.into_uuid(),
        idempotency_key,
        COMMAND_TYPE_PROVISION_ROLE_BINDING,
        &request_hash,
    )
    .await?;

    if !receipt.is_owned() {
        tx.commit()
            .await
            .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
        return handle_receipt_result(receipt);
    }

    let result = provisioning_repository::upsert_role_binding(
        &mut tx,
        domain_uuid,
        principal_uuid,
        &cmd.role_key,
        cmd.enabled,
    )
    .await;

    let response = match &result {
        Ok(_) => {
            serde_json::json!({"domainId": domain_uuid, "principalId": principal_uuid, "roleKey": cmd.role_key, "enabled": cmd.enabled})
        }
        Err(e) => serde_json::json!({"error": e.label()}),
    };

    let status = provisioning_status(&result);

    if result.is_ok() {
        provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
            .await?;
    }

    tx.commit()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    log_provisioning(
        request_id,
        actor,
        "provision_role_binding",
        &format!("{domain_uuid}/{principal_uuid}"),
        if result.is_ok() { "success" } else { "failure" },
    );
    result.map(|_| response)
}

/// Revoke a role binding.
pub async fn revoke_role_binding(
    pool: &PgPool,
    cmd: &RevokeRoleBindingCommand,
    idempotency_key: &str,
    request_id: &str,
    actor: &PrincipalId,
) -> Result<serde_json::Value, ProvisioningError> {
    let domain_uuid = cmd.domain_id.into_uuid();
    let principal_uuid = cmd.principal_id.into_uuid();
    let request_hash = compute_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_REVOKE_ROLE_BINDING,
        "domainId": domain_uuid,
        "principalId": principal_uuid,
        "roleKey": cmd.role_key,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    let receipt = provisioning_repository::acquire_receipt(
        &mut tx,
        actor.into_uuid(),
        idempotency_key,
        COMMAND_TYPE_REVOKE_ROLE_BINDING,
        &request_hash,
    )
    .await?;

    if !receipt.is_owned() {
        tx.commit()
            .await
            .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
        return handle_receipt_result(receipt);
    }

    let result = provisioning_repository::disable_role_binding(
        &mut tx,
        domain_uuid,
        principal_uuid,
        &cmd.role_key,
    )
    .await;

    let response = match &result {
        Ok(_) => {
            serde_json::json!({"domainId": domain_uuid, "principalId": principal_uuid, "roleKey": cmd.role_key, "enabled": false})
        }
        Err(e) => serde_json::json!({"error": e.label()}),
    };

    let status = provisioning_status(&result);

    if result.is_ok() {
        provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
            .await?;
    }

    tx.commit()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    log_provisioning(
        request_id,
        actor,
        "revoke_role_binding",
        &format!("{domain_uuid}/{principal_uuid}"),
        if result.is_ok() { "success" } else { "failure" },
    );
    result.map(|_| response)
}

/// Atomically replace a domain owner.
pub async fn replace_owner(
    pool: &PgPool,
    cmd: &ReplaceOwnerCommand,
    idempotency_key: &str,
    request_id: &str,
    actor: &PrincipalId,
) -> Result<serde_json::Value, ProvisioningError> {
    let domain_uuid = cmd.domain_id.into_uuid();
    let new_owner_uuid = cmd.new_owner_id.into_uuid();
    let request_hash = compute_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_REPLACE_OWNER,
        "domainId": domain_uuid,
        "newOwnerId": new_owner_uuid,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    let receipt = provisioning_repository::acquire_receipt(
        &mut tx,
        actor.into_uuid(),
        idempotency_key,
        COMMAND_TYPE_REPLACE_OWNER,
        &request_hash,
    )
    .await?;

    if !receipt.is_owned() {
        tx.commit()
            .await
            .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
        return handle_receipt_result(receipt);
    }

    let result = provisioning_repository::replace_domain_owner(
        &mut tx,
        domain_uuid,
        cmd.current_owner_id.map(|id| id.into_uuid()),
        new_owner_uuid,
    )
    .await;

    let response = match &result {
        Ok(_) => serde_json::json!({"domainId": domain_uuid, "newOwnerId": new_owner_uuid}),
        Err(e) => serde_json::json!({"error": e.label()}),
    };

    let status = provisioning_status(&result);

    if result.is_ok() {
        provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
            .await?;
    }

    tx.commit()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    log_provisioning(
        request_id,
        actor,
        "replace_owner",
        &domain_uuid.to_string(),
        if result.is_ok() { "success" } else { "failure" },
    );
    result.map(|_| response)
}

/// Get a definition version summary.
pub async fn get_definition_version(
    pool: &PgPool,
    version_id: DefinitionVersionId,
) -> Result<serde_json::Value, ProvisioningError> {
    let uuid = version_id.into_uuid();
    let summary = provisioning_repository::get_definition_version_summary(pool, uuid).await?;
    match summary {
        Some(s) => {
            let (nodes, transitions) =
                provisioning_repository::get_definition_graph_counts(pool, uuid).await?;
            let can_create = s.version_status == "PUBLISHED";
            Ok(serde_json::json!({
                "definitionVersionId": s.definition_version_id,
                "definitionKey": s.definition_key,
                "versionNumber": s.version_number,
                "versionStatus": s.version_status,
                "digest": s.digest,
                "nodeCount": nodes,
                "transitionCount": transitions,
                "canCreateInstances": can_create,
            }))
        }
        None => Err(ProvisioningError::DefinitionVersionNotFound),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Helper: get the HTTP status code from a provisioning result.
fn provisioning_status<T>(result: &Result<T, ProvisioningError>) -> i32 {
    match result {
        Ok(_) => 200,
        Err(e) => e.status_code() as i32,
    }
}

fn handle_receipt_result(
    receipt: provisioning_repository::AcquireReceipt,
) -> Result<serde_json::Value, ProvisioningError> {
    match receipt {
        provisioning_repository::AcquireReceipt::Replay { response_body, .. } => Ok(response_body),
        provisioning_repository::AcquireReceipt::Conflict { .. } => {
            Err(ProvisioningError::IdempotencyConflict)
        }
        provisioning_repository::AcquireReceipt::Processing { .. } => {
            Err(ProvisioningError::CommandStillProcessing)
        }
        provisioning_repository::AcquireReceipt::Owned(_) => unreachable!("handled above"),
    }
}
