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
pub(super) fn handle_receipt_result<T>(
    receipt: AcquireReceipt,
    diagnostic_hash: &str,
) -> Result<T, DefinitionGovernanceError>
where
    T: serde::de::DeserializeOwned,
{
    if let AcquireReceipt::Replay {
        response_status,
        response_body,
        ..
    } = &receipt
    {
        if response_body
            .get("error")
            .and_then(serde_json::Value::as_str)
            == Some("graph_validation_failed")
            && *response_status != 422
        {
            return Err(invalid_graph_receipt());
        }
    }
    match receipt {
        AcquireReceipt::Replay {
            response_status: 200,
            response_body,
            ..
        } => serde_json::from_value(response_body).map_err(|_| {
            DefinitionGovernanceError::InternalConsistency(
                "failed to deserialize replayed response".to_string(),
            )
        }),
        AcquireReceipt::Replay {
            response_status,
            response_body,
            ..
        } => {
            if response_body
                .get("error")
                .and_then(serde_json::Value::as_str)
                == Some("graph_validation_failed")
            {
                if response_status != 422 {
                    return Err(invalid_graph_receipt());
                }
                let Some(stored_hash) = response_body
                    .get("graphInputHash")
                    .and_then(serde_json::Value::as_str)
                else {
                    return Err(invalid_graph_receipt());
                };
                if stored_hash.len() != 64
                    || !stored_hash
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err(invalid_graph_receipt());
                }
                let decoded = error_from_receipt_body(&response_body);
                if matches!(
                    decoded,
                    DefinitionGovernanceError::InvalidGraphDiagnosticReceipt
                ) {
                    return Err(decoded);
                }
                if stored_hash != diagnostic_hash {
                    return Err(DefinitionGovernanceError::IdempotencyConflict);
                }
                return Err(decoded);
            }
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
        Some("graph_validation_failed") => {
            if body.as_object().map(|o| o.len()) != Some(3) {
                return invalid_graph_receipt();
            }
            match body
                .get("details")
                .cloned()
                .and_then(super::GraphDiagnostics::from_receipt)
            {
                Some(details) => DefinitionGovernanceError::GraphValidationFailed(details),
                None => invalid_graph_receipt(),
            }
        }
        Some("not_domain_owner") => DefinitionGovernanceError::NotDomainOwner,
        Some("domain_disabled") => DefinitionGovernanceError::DomainDisabled,
        Some("definition_not_found") => DefinitionGovernanceError::DefinitionNotFound,
        Some("definition_not_editable") => DefinitionGovernanceError::DefinitionNotEditable,
        Some("definition_version_immutable") => {
            DefinitionGovernanceError::DefinitionVersionImmutable
        }
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

fn invalid_graph_receipt() -> DefinitionGovernanceError {
    DefinitionGovernanceError::InvalidGraphDiagnosticReceipt
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn historical_error_decodes_and_new_receipt_is_closed() {
        assert_eq!(
            error_from_receipt_body(&serde_json::json!({"error":"definition_not_found"})),
            DefinitionGovernanceError::DefinitionNotFound
        );
        let raw = serde_json::json!({"error":"graph_validation_failed","details":{"errors":[{"code":"SELF_LOOP","message":"SQL secret"}],"truncated":false}});
        assert_eq!(error_from_receipt_body(&raw), invalid_graph_receipt());
        let result: Result<(), _> = handle_receipt_result(
            AcquireReceipt::Replay {
                response_status: 200,
                response_body: raw,
                command_id: uuid::Uuid::new_v4(),
            },
            "unused",
        );
        assert!(result.is_err());
    }
}
