//! Idempotent receipt helpers for domain membership operations.
//!
//! Wraps the generic `acquire_receipt` / `complete_receipt` primitives
//! from the provisioning repository with domain-membership-specific
//! error conversion and replay handling.

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::provisioning::ProvisioningError;
use crate::store::postgres::provisioning_repository::{
    acquire_receipt, complete_receipt, AcquireReceipt,
};

use super::DomainMembershipError;

/// Compute a SHA-256 hex digest of a canonical JSON payload for
/// idempotent receipt comparison.
pub(super) fn compute_receipt_hash(payload: &serde_json::Value) -> String {
    let canonical = serde_json::to_string(payload).unwrap_or_else(|_| payload.to_string());
    hex::encode(Sha256::digest(canonical.as_bytes()))
}

/// Map a non-owned receipt outcome (replay, conflict, processing) to
/// the appropriate error or response.
pub(super) fn handle_receipt_result(
    receipt: AcquireReceipt,
) -> Result<serde_json::Value, DomainMembershipError> {
    match receipt {
        AcquireReceipt::Replay {
            response_status: 200,
            response_body,
            ..
        } => Ok(response_body),
        AcquireReceipt::Replay { response_body, .. } => {
            Err(error_from_receipt_body(&response_body))
        }
        AcquireReceipt::Conflict { .. } => Err(DomainMembershipError::IdempotencyConflict),
        AcquireReceipt::Processing { .. } => Err(DomainMembershipError::CommandStillProcessing),
        AcquireReceipt::Owned(_) => unreachable!("owned receipt handled by caller"),
    }
}

fn error_from_receipt_body(body: &serde_json::Value) -> DomainMembershipError {
    match body.get("error").and_then(serde_json::Value::as_str) {
        Some("principal_not_registered") => DomainMembershipError::PrincipalNotRegistered,
        Some("principal_disabled") => DomainMembershipError::PrincipalDisabled,
        Some("principal_projection_conflict") => DomainMembershipError::PrincipalProjectionConflict,
        Some("domain_not_found") => DomainMembershipError::DomainNotFound,
        Some("not_domain_owner") => DomainMembershipError::NotDomainOwner,
        Some("principal_is_owner") => DomainMembershipError::PrincipalIsOwner,
        Some("member_not_found") => DomainMembershipError::MemberNotFound,
        Some("direct_token_required") => DomainMembershipError::DirectTokenRequired,
        Some("idempotency_conflict") => DomainMembershipError::IdempotencyConflict,
        Some("command_still_processing") => DomainMembershipError::CommandStillProcessing,
        _ => DomainMembershipError::InternalConsistency(
            "completed receipt contains an unknown error".to_string(),
        ),
    }
}

/// Complete the receipt with an error response and commit the transaction.
///
/// Takes ownership of the transaction so it can call `commit()`.
pub(super) async fn complete_and_return_error(
    mut tx: sqlx::Transaction<'_, sqlx::Postgres>,
    command_id: Uuid,
    error: &DomainMembershipError,
) -> Result<(), DomainMembershipError> {
    let response = serde_json::json!({"error": error.label()});
    let status = error.status_code() as i32;
    if !matches!(
        error,
        DomainMembershipError::StorageError(_) | DomainMembershipError::InternalConsistency(_)
    ) {
        complete_receipt(&mut tx, command_id, status, &response)
            .await
            .map_err(DomainMembershipError::from)?;
    }
    tx.commit()
        .await
        .map_err(|e| DomainMembershipError::StorageError(e.to_string()))?;
    Ok(())
}
