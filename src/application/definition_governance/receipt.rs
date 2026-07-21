//! Idempotent receipt helpers for definition governance operations.
//!
//! Wraps the generic `acquire_receipt` / `complete_receipt` primitives
//! with governance-specific error conversion and replay handling.

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::store::postgres::provisioning_repository::AcquireReceipt;

use super::DefinitionGovernanceError;

/// Compute a SHA-256 hex digest of a canonical JSON payload for
/// idempotent receipt comparison.
pub(super) fn compute_receipt_hash(payload: &serde_json::Value) -> String {
    let canonical = serde_json::to_string(payload).unwrap_or_else(|_| payload.to_string());
    hex::encode(Sha256::digest(canonical.as_bytes()))
}

/// Map a non-owned receipt outcome to the appropriate error or response.
pub(super) fn handle_receipt_result<T>(receipt: AcquireReceipt) -> Result<T, DefinitionGovernanceError>
where
    T: serde::de::DeserializeOwned,
{
    match receipt {
        AcquireReceipt::Replay {
            response_status: 200,
            response_body,
            ..
        } => {
            serde_json::from_value(response_body).map_err(|_| {
                DefinitionGovernanceError::InternalConsistency(
                    "failed to deserialize replayed response".to_string(),
                )
            })
        }
        AcquireReceipt::Replay { response_body, .. } => {
            Err(error_from_receipt_body(&response_body))
        }
        AcquireReceipt::Conflict { .. } => Err(DefinitionGovernanceError::IdempotencyConflict),
        AcquireReceipt::Processing { .. } => Err(DefinitionGovernanceError::CommandStillProcessing),
        AcquireReceipt::Owned(_) => {
            unreachable!("owned receipt handled by caller")
        }
    }
}

fn error_from_receipt_body(body: &serde_json::Value) -> DefinitionGovernanceError {
    match body.get("error").and_then(serde_json::Value::as_str) {
        Some("not_domain_owner") => DefinitionGovernanceError::NotDomainOwner,
        Some("domain_disabled") => DefinitionGovernanceError::DomainDisabled,
        Some("definition_not_found") => DefinitionGovernanceError::DefinitionNotFound,
        Some("definition_not_editable") => DefinitionGovernanceError::DefinitionNotEditable,
        Some("definition_key_conflict") => DefinitionGovernanceError::DefinitionKeyConflict,
        Some("revision_conflict") => DefinitionGovernanceError::RevisionConflict,
        Some("direct_token_required") => DefinitionGovernanceError::DirectTokenRequired,
        Some("idempotency_conflict") => DefinitionGovernanceError::IdempotencyConflict,
        Some("command_still_processing") => DefinitionGovernanceError::CommandStillProcessing,
        _ => DefinitionGovernanceError::InternalConsistency(
            "completed receipt contains an unknown error".to_string(),
        ),
    }
}
