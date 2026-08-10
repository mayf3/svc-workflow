//! Atomic workflow instance archive transaction.
//!
//! Domain Owner archives a terminal instance (completed, cancelled, or failed):
//! 1. Acquires idempotency receipt
//! 2. Locks the WorkflowInstance
//! 3. Validates domain owner, instance is terminal (cancelled OR node_type == TERMINAL)
//! 4. Updates instance: archived_at, archived_by_principal_id, archive_reason, state_version+1
//! 5. Writes WORKFLOW_INSTANCE_ARCHIVED event to timeline
//! 6. Completes receipt

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::workflow_instance::commands::ArchiveWorkflowInstanceCommand;
use crate::domain::workflow_instance::errors::ArchiveWorkflowInstanceError;
use crate::domain::workflow_instance::events::{
    WorkflowInstanceArchivedEventData, COMMAND_TYPE_ARCHIVE_WORKFLOW_INSTANCE,
    EVENT_SCHEMA_VERSION, WORKFLOW_INSTANCE_ARCHIVED_EVENT_TYPE,
};

fn storage(error: sqlx::Error) -> ArchiveWorkflowInstanceError {
    ArchiveWorkflowInstanceError::StorageError(error.to_string())
}

/// Validate the archive reason.
fn validate_reason(reason: &str) -> Result<(), ArchiveWorkflowInstanceError> {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return Err(ArchiveWorkflowInstanceError::InvalidReason(
            "reason must not be empty".to_string(),
        ));
    }
    if trimmed.len() > 2000 {
        return Err(ArchiveWorkflowInstanceError::InvalidReason(
            "reason must not exceed 2000 characters".to_string(),
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(ArchiveWorkflowInstanceError::InvalidReason(
            "reason must not contain control characters".to_string(),
        ));
    }
    Ok(())
}

/// Acquire an idempotency receipt for ARCHIVE.
async fn acquire_receipt(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    idempotency_key: &str,
    command_type: &str,
    request_hash: &str,
) -> Result<AcquireResult, ArchiveWorkflowInstanceError> {
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
        return Ok(AcquireResult::Owned(command_id));
    }

    let existing: Option<(Uuid, String, String, Option<i32>, Option<serde_json::Value>)> =
        sqlx::query_as(
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

    let (command_id, status, original_hash, response_status, response_body) =
        existing.ok_or_else(|| {
            ArchiveWorkflowInstanceError::InternalConsistency("receipt disappeared".to_string())
        })?;

    if original_hash != request_hash {
        return Ok(AcquireResult::Conflict { command_id });
    }
    if status == "PROCESSING" {
        return Ok(AcquireResult::Processing { command_id });
    }
    let response_status = response_status.ok_or_else(|| {
        ArchiveWorkflowInstanceError::InternalConsistency(
            "completed receipt has no status".to_string(),
        )
    })?;
    let response_body = response_body.unwrap_or(serde_json::Value::Null);

    Ok(AcquireResult::Replay {
        command_id,
        response_status,
        response_body,
    })
}

enum AcquireResult {
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

async fn complete_receipt(
    tx: &mut Transaction<'_, Postgres>,
    command_id: Uuid,
    response_status: i32,
    response_body: &serde_json::Value,
) -> Result<(), ArchiveWorkflowInstanceError> {
    let response_digest = digest::compute_json_digest(response_body)
        .map_err(|e| ArchiveWorkflowInstanceError::StorageError(e.to_string()))?;
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
        return Err(ArchiveWorkflowInstanceError::InternalConsistency(
            "receipt completion affected an unexpected row count".to_string(),
        ));
    }
    Ok(())
}

pub(crate) async fn archive_workflow_instance_atomically(
    pool: &PgPool,
    cmd: ArchiveWorkflowInstanceCommand,
    request_hash: &str,
) -> Result<ArchiveResult, ArchiveWorkflowInstanceError> {
    let principal_uuid = cmd.principal_id.into_uuid();
    let instance_uuid = cmd.workflow_instance_id.into_uuid();
    let event_id = Uuid::new_v4();

    let mut tx = pool.begin().await.map_err(storage)?;

    // Step 1: Acquire command receipt (idempotency gate)
    let receipt = acquire_receipt(
        &mut tx,
        principal_uuid,
        &cmd.idempotency_key,
        COMMAND_TYPE_ARCHIVE_WORKFLOW_INSTANCE,
        request_hash,
    )
    .await?;

    let actual_command_id = match receipt {
        AcquireResult::Owned(cmd_id) => cmd_id,
        AcquireResult::Replay {
            command_id: _replayed_cmd_id,
            response_status,
            response_body,
        } => {
            tx.commit().await.map_err(storage)?;
            if response_status != 200 {
                return Err(error_from_body(&response_body));
            }
            let mut result: ArchiveResult = serde_json::from_value(response_body)
                .map_err(|e| ArchiveWorkflowInstanceError::InternalConsistency(e.to_string()))?;
            result.replayed = true;
            return Ok(result);
        }
        AcquireResult::Conflict { command_id } => {
            tx.commit().await.map_err(storage)?;
            return Err(ArchiveWorkflowInstanceError::IdempotencyConflict {
                original_command_id: command_id,
                original_request_hash: String::new(),
            });
        }
        AcquireResult::Processing { .. } => {
            tx.commit().await.map_err(storage)?;
            return Err(ArchiveWorkflowInstanceError::CommandStillProcessing);
        }
    };

    // Step 2: Validate reason
    validate_reason(&cmd.reason)?;

    // Step 3: Load and lock instance FOR UPDATE
    let instance_row: Option<(
        Uuid,
        Uuid,
        i32,
        bool,
        Option<String>,
        bool,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        "SELECT wi.workflow_instance_id, wi.domain_id,
                    wi.workflow_state_version,
                    wi.cancelled,
                    nd.node_type::text,
                    COALESCE(wi.archived_at IS NOT NULL, FALSE),
                    wi.archived_at
             FROM workflow_instances wi
             LEFT JOIN workflow_node_visits nv ON nv.node_visit_id = wi.current_node_visit_id
             LEFT JOIN workflow_node_definitions nd ON nd.node_id = nv.node_id
             WHERE wi.workflow_instance_id = $1
             FOR UPDATE OF wi",
    )
    .bind(instance_uuid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage)?;

    let (_, domain_id, current_state_version, is_cancelled, node_type, is_archived, _archived_at) =
        instance_row.ok_or(ArchiveWorkflowInstanceError::InstanceNotFound)?;

    // Step 4: Validate workflow_state_version.
    //
    // `expected_workflow_state_version == 0` is the HTTP adapter sentinel for
    // "no client-side optimistic version": the archive contract does not carry
    // a state version, so authoritative state checks run atomically under the
    // row lock below. Explicit non-zero expectations are validated strictly.
    if cmd.expected_workflow_state_version != 0
        && cmd.expected_workflow_state_version != current_state_version
    {
        return Err(ArchiveWorkflowInstanceError::WorkflowStateVersionConflict {
            expected: cmd.expected_workflow_state_version,
            actual: current_state_version,
        });
    }

    // Step 5: Check domain owner
    let is_owner: bool = sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM domain_role_bindings
           WHERE domain_id = $1 AND principal_id = $2
             AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE)",
    )
    .bind(domain_id)
    .bind(principal_uuid)
    .fetch_one(&mut *tx)
    .await
    .map_err(storage)?;

    if !is_owner {
        return Err(ArchiveWorkflowInstanceError::NotDomainOwner);
    }

    // Step 6: Check instance is in a terminal state (cancelled OR node_type == TERMINAL)
    let is_terminal = is_cancelled || node_type.as_deref() == Some("TERMINAL");
    if !is_terminal {
        return Err(ArchiveWorkflowInstanceError::InstanceNotTerminal);
    }

    // Step 6a: Archive is a one-shot lifecycle change, symmetric with cancel's
    // InstanceArchived guard. Under the FOR UPDATE row lock, an already
    // archived instance is rejected: a new idempotency key must not overwrite
    // archived_at/archive_reason, append a second archive event, or grow
    // workflow_state_version. The error path rolls back the whole transaction,
    // so no success receipt is created.
    if is_archived {
        return Err(ArchiveWorkflowInstanceError::AlreadyArchived);
    }

    // Step 7: Update instance — set archive metadata, increment state version
    let old_state_version = current_state_version;
    let new_state_version = old_state_version + 1;
    let now = chrono::Utc::now();

    let updated = sqlx::query(
        "UPDATE workflow_instances
         SET archived_at = $2,
             archived_by_principal_id = $3,
             archive_reason = $4,
             workflow_state_version = $5,
             updated_at = now()
         WHERE workflow_instance_id = $1
           AND workflow_state_version = $6",
    )
    .bind(instance_uuid)
    .bind(now)
    .bind(principal_uuid)
    .bind(&cmd.reason)
    .bind(new_state_version)
    .bind(old_state_version)
    .execute(&mut *tx)
    .await
    .map_err(storage)?
    .rows_affected();

    if updated != 1 {
        return Err(ArchiveWorkflowInstanceError::InternalConsistency(
            "archive update affected an unexpected row count".to_string(),
        ));
    }

    // Step 8: Write WORKFLOW_INSTANCE_ARCHIVED event
    let event_data = WorkflowInstanceArchivedEventData {
        reason: cmd.reason.clone(),
        archived_by_principal_id: principal_uuid.to_string(),
        was_cancelled: is_cancelled,
    };

    let event_data_json = serde_json::to_value(&event_data)
        .map_err(|e| ArchiveWorkflowInstanceError::StorageError(e.to_string()))?;
    let event_data_digest = digest::compute_json_digest(&event_data_json)
        .map_err(|e| ArchiveWorkflowInstanceError::StorageError(e.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO workflow_events
            (event_id, workflow_instance_id, event_sequence, event_schema_version,
             command_id, event_type, transition_effect,
             source_node_visit_id, target_node_visit_id,
             context_revision_id, submission_id,
             event_data, event_data_digest,
             actor_principal_id, from_node_id, to_node_id,
             old_workflow_state_version, new_workflow_state_version)
        VALUES ($1, $2, $3, $4, $5, $6, NULL::transition_effect,
                NULL, NULL, NULL, NULL, $7, $8, $9, NULL, NULL, $10, $11)
        "#,
    )
    .bind(event_id)
    .bind(instance_uuid)
    .bind(new_state_version)
    .bind(EVENT_SCHEMA_VERSION)
    .bind(actual_command_id)
    .bind(WORKFLOW_INSTANCE_ARCHIVED_EVENT_TYPE)
    .bind(&event_data_json)
    .bind(event_data_digest)
    .bind(principal_uuid)
    .bind(old_state_version)
    .bind(new_state_version)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        ArchiveWorkflowInstanceError::StorageError(format!("event insert failed: {}", e))
    })?;

    // Step 9: Complete the receipt
    let result = ArchiveResult {
        workflow_instance_id: instance_uuid,
        workflow_state_version: new_state_version,
        event_sequence: new_state_version,
        replayed: false,
    };

    let response_body = serde_json::to_value(&result)
        .map_err(|e| ArchiveWorkflowInstanceError::StorageError(e.to_string()))?;

    complete_receipt(&mut tx, actual_command_id, 200, &response_body).await?;

    // Step 10: Commit
    tx.commit().await.map_err(storage)?;

    Ok(result)
}

fn error_from_body(body: &serde_json::Value) -> ArchiveWorkflowInstanceError {
    let detail = || {
        body["detail"]
            .as_str()
            .unwrap_or("replayed failure")
            .to_string()
    };
    match body["error"]
        .as_str()
        .unwrap_or("internal_consistency_error")
    {
        "not_domain_owner" => ArchiveWorkflowInstanceError::NotDomainOwner,
        "instance_not_terminal" => ArchiveWorkflowInstanceError::InstanceNotTerminal,
        "already_archived" => ArchiveWorkflowInstanceError::AlreadyArchived,
        "invalid_reason" => ArchiveWorkflowInstanceError::InvalidReason(detail()),
        "workflow_state_version_conflict" => {
            ArchiveWorkflowInstanceError::WorkflowStateVersionConflict {
                expected: body["expected"].as_i64().unwrap_or_default() as i32,
                actual: body["actual"].as_i64().unwrap_or_default() as i32,
            }
        }
        _ => ArchiveWorkflowInstanceError::InternalConsistency(detail()),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ArchiveResult {
    pub workflow_instance_id: Uuid,
    pub workflow_state_version: i32,
    pub event_sequence: i32,
    #[serde(default)]
    pub replayed: bool,
}
