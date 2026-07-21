use crate::domain::provisioning::ProvisioningError;
use crate::store::postgres::provisioning_repository::AcquireReceipt;

pub(super) fn compute_hash(body: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let canonical = serde_json::to_string(body).unwrap_or_else(|_| body.to_string());
    hex::encode(Sha256::digest(canonical.as_bytes()))
}

pub(super) fn provisioning_status<T>(result: &Result<T, ProvisioningError>) -> i32 {
    match result {
        Ok(_) => 200,
        Err(error) => error.status_code() as i32,
    }
}

pub(super) fn should_complete_receipt<T>(result: &Result<T, ProvisioningError>) -> bool {
    !matches!(
        result,
        Err(ProvisioningError::StorageError(_)) | Err(ProvisioningError::InternalConsistency(_))
    )
}

pub(super) fn handle_receipt_result(
    receipt: AcquireReceipt,
) -> Result<serde_json::Value, ProvisioningError> {
    match receipt {
        AcquireReceipt::Replay {
            response_status: 200,
            response_body,
            ..
        } => Ok(response_body),
        AcquireReceipt::Replay { response_body, .. } => Err(error_from_body(&response_body)),
        AcquireReceipt::Conflict { .. } => Err(ProvisioningError::IdempotencyConflict),
        AcquireReceipt::Processing { .. } => Err(ProvisioningError::CommandStillProcessing),
        AcquireReceipt::Owned(_) => unreachable!("owned receipt handled by caller"),
    }
}

fn error_from_body(body: &serde_json::Value) -> ProvisioningError {
    match body.get("error").and_then(serde_json::Value::as_str) {
        Some("principal_not_found") => ProvisioningError::PrincipalNotFound,
        Some("principal_disabled") => ProvisioningError::PrincipalDisabled,
        Some("principal_type_conflict") => ProvisioningError::PrincipalTypeConflict,
        Some("principal_type_invalid") => ProvisioningError::PrincipalTypeInvalid,
        Some("domain_not_found") => ProvisioningError::DomainNotFound,
        Some("domain_disabled") => ProvisioningError::DomainDisabled,
        Some("domain_identity_conflict") => ProvisioningError::DomainIdentityConflict,
        Some("domain_owner_conflict") => ProvisioningError::DomainOwnerConflict,
        Some("binding_already_exists") => ProvisioningError::BindingAlreadyExists,
        Some("binding_not_found") => ProvisioningError::BindingNotFound,
        Some("role_key_invalid") => ProvisioningError::RoleKeyInvalid,
        Some("definition_version_not_found") => ProvisioningError::DefinitionVersionNotFound,
        Some("permission_denied") => ProvisioningError::PermissionDenied,
        Some("principal_type_not_allowed") => ProvisioningError::PrincipalTypeNotAllowed,
        Some("idempotency_conflict") => ProvisioningError::IdempotencyConflict,
        Some("command_still_processing") => ProvisioningError::CommandStillProcessing,
        _ => ProvisioningError::InternalConsistency(
            "completed provisioning receipt contains an unknown error".to_string(),
        ),
    }
}
