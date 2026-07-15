use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::workflow_instance::import::LegacyImportError;

type StoredReceipt = (Uuid, String, String, Option<i32>, Option<serde_json::Value>);

pub(super) enum Acquired {
    Owned(Uuid),
    Replay(Uuid, i32, serde_json::Value),
    Conflict(Uuid),
    Processing(Uuid),
}

impl Acquired {
    pub(super) fn command_id(&self) -> Uuid {
        match self {
            Self::Owned(id) | Self::Replay(id, ..) | Self::Conflict(id) | Self::Processing(id) => {
                *id
            }
        }
    }

    pub(super) fn is_owned(&self) -> bool {
        matches!(self, Self::Owned(_))
    }
}

fn storage(error: sqlx::Error) -> LegacyImportError {
    LegacyImportError::StorageError(error.to_string())
}

pub(super) async fn acquire(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    key: &str,
    command_type: &str,
    request_hash: &str,
) -> Result<Acquired, LegacyImportError> {
    let proposed = Uuid::new_v4();
    let inserted: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO workflow_command_receipts
         (command_id, principal_id, idempotency_key, command_type, request_hash, receipt_status)
         VALUES ($1, $2, $3, $4, $5, 'PROCESSING')
         ON CONFLICT (principal_id, idempotency_key) DO NOTHING RETURNING command_id",
    )
    .bind(proposed)
    .bind(actor)
    .bind(key)
    .bind(command_type)
    .bind(request_hash)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    if let Some(id) = inserted {
        return Ok(Acquired::Owned(id));
    }
    let stored: StoredReceipt = sqlx::query_as(
        "SELECT command_id, receipt_status::text, request_hash, response_status, response_body
         FROM workflow_command_receipts WHERE principal_id = $1 AND idempotency_key = $2
         FOR UPDATE",
    )
    .bind(actor)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    .ok_or_else(|| LegacyImportError::InternalConsistency("receipt disappeared".to_string()))?;
    let (id, status, original_hash, response_status, response_body) = stored;
    if original_hash != request_hash {
        return Ok(Acquired::Conflict(id));
    }
    if status == "PROCESSING" {
        return Ok(Acquired::Processing(id));
    }
    let status = response_status.ok_or_else(|| {
        LegacyImportError::InternalConsistency("completed receipt has no status".to_string())
    })?;
    let body = response_body.ok_or_else(|| {
        LegacyImportError::InternalConsistency("completed receipt has no body".to_string())
    })?;
    Ok(Acquired::Replay(id, status, body))
}

pub(super) async fn complete(
    tx: &mut Transaction<'_, Postgres>,
    command_id: Uuid,
    status: i32,
    body: &serde_json::Value,
) -> Result<(), LegacyImportError> {
    let response_digest =
        digest::compute_json_digest(body).map_err(LegacyImportError::StorageError)?;
    let affected = sqlx::query(
        "UPDATE workflow_command_receipts SET receipt_status = 'COMPLETED',
         response_status = $2, response_body = $3, response_digest = $4
         WHERE command_id = $1 AND receipt_status = 'PROCESSING'",
    )
    .bind(command_id)
    .bind(status)
    .bind(body)
    .bind(response_digest)
    .execute(&mut **tx)
    .await
    .map_err(storage)?
    .rows_affected();
    if affected != 1 {
        return Err(LegacyImportError::InternalConsistency(
            "receipt completion affected an unexpected row count".to_string(),
        ));
    }
    Ok(())
}

pub(super) async fn write_attempt(
    tx: &mut Transaction<'_, Postgres>,
    acquired: &Acquired,
    actor: Uuid,
    key: &str,
    request_hash: &str,
    attempt_type: &str,
    error: &LegacyImportError,
) -> Result<(), LegacyImportError> {
    sqlx::query(
        "INSERT INTO workflow_command_attempt_audits
         (audit_id, command_id, principal_id, idempotency_key, attempt_type,
          failure_reason, request_hash, details)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(acquired.command_id())
    .bind(actor)
    .bind(key)
    .bind(attempt_type)
    .bind(error.label())
    .bind(request_hash)
    .bind(error_body(error))
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}

pub(super) async fn write_security(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    instance_id: Uuid,
    details: &serde_json::Value,
) -> Result<(), LegacyImportError> {
    sqlx::query(
        "INSERT INTO workflow_security_audits
         (audit_id, principal_id, action, resource_type, resource_id, details)
         VALUES ($1, $2, 'LEGACY_WORKFLOW_IMPORT_COMMITTED', 'WORKFLOW_INSTANCE', $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(actor)
    .bind(instance_id.to_string())
    .bind(details)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}

pub(super) fn error_body(error: &LegacyImportError) -> serde_json::Value {
    let mut body = serde_json::json!({"error": error.label()});
    if let LegacyImportError::SnapshotDigestMismatch { expected, actual } = error {
        body["expected"] = expected.clone().into();
        body["actual"] = actual.clone().into();
    }
    body
}

pub(super) fn error_from_body(body: &serde_json::Value) -> LegacyImportError {
    let detail = "replayed deterministic failure".to_string();
    match body["error"].as_str().unwrap_or("") {
        "principal_not_found" => LegacyImportError::PrincipalNotFound,
        "principal_disabled" => LegacyImportError::PrincipalDisabled,
        "principal_type_not_allowed" => LegacyImportError::PrincipalTypeNotAllowed,
        "migration_binding_invalid" => LegacyImportError::MigrationBindingInvalid,
        "permission_denied" => LegacyImportError::PermissionDenied,
        "domain_not_found" => LegacyImportError::DomainNotFound,
        "domain_disabled" => LegacyImportError::DomainDisabled,
        "definition_version_not_found" => LegacyImportError::DefinitionVersionNotFound,
        "version_not_published" => LegacyImportError::VersionNotPublished,
        "imported_node_not_found" => LegacyImportError::ImportedNodeNotFound,
        "invalid_input" => LegacyImportError::InvalidInput(detail),
        "snapshot_digest_mismatch" => LegacyImportError::SnapshotDigestMismatch {
            expected: body["expected"].as_str().unwrap_or("").to_string(),
            actual: body["actual"].as_str().unwrap_or("").to_string(),
        },
        "creator_resolution_failed" => LegacyImportError::CreatorResolutionFailed(detail),
        "assignee_resolution_failed" => LegacyImportError::AssigneeResolutionFailed(detail),
        "context_validation_failed" => LegacyImportError::ContextValidationFailed(detail),
        "size_limit_exceeded" => LegacyImportError::SizeLimitExceeded(detail),
        "external_reference_conflict" => LegacyImportError::ExternalReferenceConflict,
        _ => LegacyImportError::InternalConsistency("unknown replayed error".to_string()),
    }
}
