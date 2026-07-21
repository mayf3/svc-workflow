use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::domain::provisioning::ProvisioningError;

use super::storage;

type StoredReceipt = (Uuid, String, String, Option<i32>, Option<serde_json::Value>);

pub(crate) enum AcquireReceipt {
    Owned(Uuid),
    Replay {
        command_id: Uuid,
        response_status: i32,
        response_body: serde_json::Value,
    },
    Conflict {
        command_id: Uuid,
    },
    Processing {
        command_id: Uuid,
    },
}

impl AcquireReceipt {
    pub(crate) fn command_id(&self) -> Uuid {
        match self {
            Self::Owned(command_id)
            | Self::Replay { command_id, .. }
            | Self::Conflict { command_id }
            | Self::Processing { command_id } => *command_id,
        }
    }

    pub(crate) fn is_owned(&self) -> bool {
        matches!(self, Self::Owned(_))
    }
}

pub(crate) async fn acquire_receipt(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    idempotency_key: &str,
    command_type: &str,
    request_hash: &str,
) -> Result<AcquireReceipt, ProvisioningError> {
    let proposed = Uuid::new_v4();
    let inserted: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO workflow_command_receipts
         (command_id, principal_id, idempotency_key, command_type, request_hash, receipt_status)
         VALUES ($1, $2, $3, $4, $5, 'PROCESSING')
         ON CONFLICT (principal_id, idempotency_key) DO NOTHING
         RETURNING command_id",
    )
    .bind(proposed)
    .bind(principal_id)
    .bind(idempotency_key)
    .bind(command_type)
    .bind(request_hash)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    if let Some(command_id) = inserted {
        return Ok(AcquireReceipt::Owned(command_id));
    }

    let existing: Option<StoredReceipt> = sqlx::query_as(
        "SELECT command_id, receipt_status::text, request_hash,
                response_status, response_body
         FROM workflow_command_receipts
         WHERE principal_id = $1 AND idempotency_key = $2
         FOR UPDATE",
    )
    .bind(principal_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    let (command_id, status, original_hash, response_status, response_body) = existing
        .ok_or_else(|| ProvisioningError::InternalConsistency("receipt disappeared".to_string()))?;
    if original_hash != request_hash {
        return Ok(AcquireReceipt::Conflict { command_id });
    }
    if status == "PROCESSING" {
        return Ok(AcquireReceipt::Processing { command_id });
    }
    Ok(AcquireReceipt::Replay {
        command_id,
        response_status: response_status.ok_or_else(|| {
            ProvisioningError::InternalConsistency("completed receipt missing status".to_string())
        })?,
        response_body: response_body.ok_or_else(|| {
            ProvisioningError::InternalConsistency("completed receipt missing body".to_string())
        })?,
    })
}

pub(crate) async fn complete_receipt(
    tx: &mut Transaction<'_, Postgres>,
    command_id: Uuid,
    response_status: i32,
    response_body: &serde_json::Value,
) -> Result<(), ProvisioningError> {
    use sha2::{Digest, Sha256};
    let json_bytes = serde_json::to_vec(response_body)
        .map_err(|error| ProvisioningError::InternalConsistency(error.to_string()))?;
    let response_digest = hex::encode(Sha256::digest(&json_bytes));
    let affected = sqlx::query(
        "UPDATE workflow_command_receipts
         SET receipt_status = 'COMPLETED', response_status = $2,
             response_body = $3, response_digest = $4
         WHERE command_id = $1 AND receipt_status = 'PROCESSING'",
    )
    .bind(command_id)
    .bind(response_status)
    .bind(response_body)
    .bind(response_digest)
    .execute(&mut **tx)
    .await
    .map_err(storage)?
    .rows_affected();
    if affected != 1 {
        return Err(ProvisioningError::InternalConsistency(
            "receipt completion affected unexpected row count".to_string(),
        ));
    }
    Ok(())
}
