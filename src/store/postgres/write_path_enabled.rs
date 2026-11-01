//! Transaction primitives for the seven WORKFLOW_WRITE_PATH_ENABLED_ENFORCEMENT_V1 gates.

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Hold eligibility until the command commits. KEY SHARE does not conflict
/// with a non-key UPDATE of enabled; SHARE does.
pub(crate) async fn principal_enabled(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar("SELECT enabled FROM principals WHERE principal_id = $1 FOR SHARE")
        .bind(principal_id)
        .fetch_optional(&mut **tx)
        .await
}

/// The owned receipt supplies the actual actor, operation and request identity.
/// Call only for a newly added gate, before completing that receipt in this tx.
pub(crate) async fn audit_denial(
    tx: &mut Transaction<'_, Postgres>,
    command_id: Uuid,
    code: &str,
    details: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let affected = sqlx::query(
        "INSERT INTO workflow_command_attempt_audits
         (audit_id, command_id, principal_id, idempotency_key, attempt_type,
          failure_reason, request_hash, details)
         SELECT $1, command_id, principal_id, idempotency_key, 'PERMISSION_DENIED',
                $3, request_hash,
                $4 || jsonb_build_object('commandType', command_type, 'actorPrincipalId', principal_id)
         FROM workflow_command_receipts
         WHERE command_id = $2 AND receipt_status = 'PROCESSING'",
    )
    .bind(Uuid::new_v4())
    .bind(command_id)
    .bind(code)
    .bind(details)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

/// Some existing completion helpers do not check affected rows. Enforce that
/// this new denial really completed, without changing those older paths.
pub(crate) async fn verify_denial_receipt(
    tx: &mut Transaction<'_, Postgres>,
    command_id: Uuid,
    status: i32,
    body: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let digest = crate::domain::definition::digest::compute_json_digest(body)
        .map_err(sqlx::Error::Protocol)?;
    let completed: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflow_command_receipts
         WHERE command_id = $1 AND receipt_status = 'COMPLETED'
           AND response_status = $2 AND response_body = $3 AND response_digest = $4)",
    )
    .bind(command_id)
    .bind(status)
    .bind(body)
    .bind(digest)
    .fetch_one(&mut **tx)
    .await?;
    if !completed {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}
