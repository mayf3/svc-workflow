//! Identity provisioning application service.
//!
//! Orchestrates principal, domain, and role binding provisioning operations
//! with idempotent receipt handling and structured audit logging.

mod config;
mod definitions;
mod receipt;

use sqlx::PgPool;

use crate::domain::ids::{DomainId, PrincipalId};
use crate::domain::provisioning::*;
use crate::store::postgres::provisioning_repository;

pub use config::ProvisioningConfig;
pub use definitions::get_definition_version;
use receipt::{compute_hash, handle_receipt_result, provisioning_status, should_complete_receipt};

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
        "source": cmd.source,
        "sourceRevision": cmd.source_revision,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    if actor_principal_id == &cmd.principal_id {
        provisioning_repository::ensure_provisioning_actor(&mut tx, principal_uuid, &cmd.source)
            .await?;
    }
    provisioning_repository::validate_provisioning_actor(&mut tx, actor_principal_id.into_uuid())
        .await?;
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

    if !should_complete_receipt(&result) {
        log_provisioning(
            request_id,
            actor_principal_id,
            "provision_principal",
            &principal_uuid.to_string(),
            "failure",
        );
        return result.map(|_| response);
    }
    provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
        .await?;

    tx.commit()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;

    log_provisioning(
        request_id,
        actor_principal_id,
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
            "principalType": r.principal_type.to_ascii_lowercase(),
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
        "displayName": cmd.display_name,
        "enabled": cmd.enabled,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    provisioning_repository::validate_provisioning_actor(&mut tx, actor_principal_id.into_uuid())
        .await?;
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

    if !should_complete_receipt(&result) {
        log_provisioning(
            request_id,
            actor_principal_id,
            "provision_domain",
            &domain_uuid.to_string(),
            "failure",
        );
        return result.map(|_| response);
    }
    provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
        .await?;

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
    provisioning_repository::validate_provisioning_actor(&mut tx, actor.into_uuid()).await?;
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

    if !should_complete_receipt(&result) {
        log_provisioning(
            request_id,
            actor,
            "provision_role_binding",
            &format!("{domain_uuid}/{principal_uuid}"),
            "failure",
        );
        return result.map(|_| response);
    }
    provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
        .await?;

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
    provisioning_repository::validate_provisioning_actor(&mut tx, actor.into_uuid()).await?;
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

    if !should_complete_receipt(&result) {
        log_provisioning(
            request_id,
            actor,
            "revoke_role_binding",
            &format!("{domain_uuid}/{principal_uuid}"),
            "failure",
        );
        return result.map(|_| response);
    }
    provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
        .await?;

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

/// Provision a global (domain-independent) role binding.
///
/// Mirrors `provision_role_binding` minus the domain dimension. The role
/// key is validated against the supported global roles at the handler
/// layer; the receipt machinery is shared.
pub async fn provision_global_role_binding(
    pool: &PgPool,
    cmd: &ProvisionGlobalRoleBindingCommand,
    idempotency_key: &str,
    request_id: &str,
    actor: &PrincipalId,
) -> Result<serde_json::Value, ProvisioningError> {
    let principal_uuid = cmd.principal_id.into_uuid();
    let request_hash = compute_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_PROVISION_GLOBAL_ROLE_BINDING,
        "principalId": principal_uuid,
        "roleKey": cmd.role_key,
        "enabled": cmd.enabled,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    provisioning_repository::validate_provisioning_actor(&mut tx, actor.into_uuid()).await?;
    let receipt = provisioning_repository::acquire_receipt(
        &mut tx,
        actor.into_uuid(),
        idempotency_key,
        COMMAND_TYPE_PROVISION_GLOBAL_ROLE_BINDING,
        &request_hash,
    )
    .await?;

    if !receipt.is_owned() {
        tx.commit()
            .await
            .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
        return handle_receipt_result(receipt);
    }

    let result = provisioning_repository::upsert_global_role_binding(
        &mut tx,
        principal_uuid,
        &cmd.role_key,
        cmd.enabled,
    )
    .await;

    let response = match &result {
        Ok(_) => serde_json::json!({
            "principalId": principal_uuid,
            "roleKey": cmd.role_key,
            "enabled": cmd.enabled,
        }),
        Err(e) => serde_json::json!({"error": e.label()}),
    };

    let status = provisioning_status(&result);

    if !should_complete_receipt(&result) {
        log_provisioning(
            request_id,
            actor,
            "provision_global_role_binding",
            &format!("global/{principal_uuid}"),
            "failure",
        );
        return result.map(|_| response);
    }
    provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
        .await?;

    tx.commit()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    log_provisioning(
        request_id,
        actor,
        "provision_global_role_binding",
        &format!("global/{principal_uuid}"),
        if result.is_ok() { "success" } else { "failure" },
    );
    result.map(|_| response)
}

/// Revoke a global (domain-independent) role binding.
pub async fn revoke_global_role_binding(
    pool: &PgPool,
    cmd: &RevokeGlobalRoleBindingCommand,
    idempotency_key: &str,
    request_id: &str,
    actor: &PrincipalId,
) -> Result<serde_json::Value, ProvisioningError> {
    let principal_uuid = cmd.principal_id.into_uuid();
    let request_hash = compute_hash(&serde_json::json!({
        "commandType": COMMAND_TYPE_REVOKE_GLOBAL_ROLE_BINDING,
        "principalId": principal_uuid,
        "roleKey": cmd.role_key,
    }));

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    provisioning_repository::validate_provisioning_actor(&mut tx, actor.into_uuid()).await?;
    let receipt = provisioning_repository::acquire_receipt(
        &mut tx,
        actor.into_uuid(),
        idempotency_key,
        COMMAND_TYPE_REVOKE_GLOBAL_ROLE_BINDING,
        &request_hash,
    )
    .await?;

    if !receipt.is_owned() {
        tx.commit()
            .await
            .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
        return handle_receipt_result(receipt);
    }

    let result = provisioning_repository::disable_global_role_binding(
        &mut tx,
        principal_uuid,
        &cmd.role_key,
    )
    .await;

    let response = match &result {
        Ok(_) => serde_json::json!({
            "principalId": principal_uuid,
            "roleKey": cmd.role_key,
            "enabled": false,
        }),
        Err(e) => serde_json::json!({"error": e.label()}),
    };

    let status = provisioning_status(&result);

    if !should_complete_receipt(&result) {
        log_provisioning(
            request_id,
            actor,
            "revoke_global_role_binding",
            &format!("global/{principal_uuid}"),
            "failure",
        );
        return result.map(|_| response);
    }
    provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
        .await?;

    tx.commit()
        .await
        .map_err(|e| ProvisioningError::StorageError(e.to_string()))?;
    log_provisioning(
        request_id,
        actor,
        "revoke_global_role_binding",
        &format!("global/{principal_uuid}"),
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
    provisioning_repository::validate_provisioning_actor(&mut tx, actor.into_uuid()).await?;
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

    let result =
        provisioning_repository::replace_domain_owner(&mut tx, domain_uuid, new_owner_uuid).await;

    let response = match &result {
        Ok(_) => serde_json::json!({"domainId": domain_uuid, "newOwnerId": new_owner_uuid}),
        Err(e) => serde_json::json!({"error": e.label()}),
    };

    let status = provisioning_status(&result);

    if !should_complete_receipt(&result) {
        log_provisioning(
            request_id,
            actor,
            "replace_owner",
            &domain_uuid.to_string(),
            "failure",
        );
        return result.map(|_| response);
    }
    provisioning_repository::complete_receipt(&mut tx, receipt.command_id(), status, &response)
        .await?;

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
