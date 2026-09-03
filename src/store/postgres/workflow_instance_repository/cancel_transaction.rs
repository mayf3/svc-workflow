//! Atomic workflow instance cancellation transaction.
//!
//! Domain Owner cancels an active instance:
//! 1. Acquires idempotency receipt
//! 2. Locks the WorkflowInstance
//! 3. Validates domain owner, instance not terminal/cancelled/archived
//! 4. Clears current node visit assignee (closes work item)
//! 5. Updates instance: cancelled=TRUE, cancel_reason, state_version+1
//! 6. Writes WORKFLOW_INSTANCE_CANCELLED event to timeline
//! 7. Completes receipt

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::workflow_instance::commands::CancelWorkflowInstanceCommand;
use crate::domain::workflow_instance::errors::CancelWorkflowInstanceError;
use crate::domain::workflow_instance::events::{
    WorkflowInstanceCancelledEventData, COMMAND_TYPE_CANCEL_WORKFLOW_INSTANCE,
    EVENT_SCHEMA_VERSION, WORKFLOW_INSTANCE_CANCELLED_EVENT_TYPE,
};

fn storage(error: sqlx::Error) -> CancelWorkflowInstanceError {
    CancelWorkflowInstanceError::StorageError(error.to_string())
}

/// Validate the cancel reason.
fn validate_reason(reason: &str) -> Result<(), CancelWorkflowInstanceError> {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return Err(CancelWorkflowInstanceError::InvalidReason(
            "reason must not be empty".to_string(),
        ));
    }
    if trimmed.len() > 2000 {
        return Err(CancelWorkflowInstanceError::InvalidReason(
            "reason must not exceed 2000 characters".to_string(),
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(CancelWorkflowInstanceError::InvalidReason(
            "reason must not contain control characters".to_string(),
        ));
    }
    Ok(())
}

/// Acquire an idempotency receipt for CANCEL.
async fn acquire_receipt(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    idempotency_key: &str,
    command_type: &str,
    request_hash: &str,
) -> Result<AcquireResult, CancelWorkflowInstanceError> {
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
            CancelWorkflowInstanceError::InternalConsistency("receipt disappeared".to_string())
        })?;

    if original_hash != request_hash {
        return Ok(AcquireResult::Conflict { command_id });
    }
    if status == "PROCESSING" {
        return Ok(AcquireResult::Processing { command_id });
    }
    let response_status = response_status.ok_or_else(|| {
        CancelWorkflowInstanceError::InternalConsistency(
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
) -> Result<(), CancelWorkflowInstanceError> {
    let response_digest = digest::compute_json_digest(response_body)
        .map_err(|e| CancelWorkflowInstanceError::StorageError(e.to_string()))?;
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
        return Err(CancelWorkflowInstanceError::InternalConsistency(
            "receipt completion affected an unexpected row count".to_string(),
        ));
    }
    Ok(())
}

pub(crate) async fn cancel_workflow_instance_atomically(
    pool: &PgPool,
    cmd: CancelWorkflowInstanceCommand,
    request_hash: &str,
) -> Result<CancelResult, CancelWorkflowInstanceError> {
    let principal_uuid = cmd.principal_id.into_uuid();
    let instance_uuid = cmd.workflow_instance_id.into_uuid();
    let event_id = Uuid::new_v4();

    let mut tx = pool.begin().await.map_err(storage)?;

    // Step 1: Acquire command receipt (idempotency gate)
    let receipt = acquire_receipt(
        &mut tx,
        principal_uuid,
        &cmd.idempotency_key,
        COMMAND_TYPE_CANCEL_WORKFLOW_INSTANCE,
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
            let mut result: CancelResult = serde_json::from_value(response_body)
                .map_err(|e| CancelWorkflowInstanceError::InternalConsistency(e.to_string()))?;
            result.replayed = true;
            return Ok(result);
        }
        AcquireResult::Conflict { command_id } => {
            let _ = write_attempt_audit(
                &mut tx,
                Uuid::new_v4(),
                command_id,
                principal_uuid,
                &cmd.idempotency_key,
                "IDEMPOTENCY_CONFLICT",
                Some("request hash mismatch"),
                request_hash,
            )
            .await;
            tx.commit().await.map_err(storage)?;
            return Err(CancelWorkflowInstanceError::IdempotencyConflict {
                original_command_id: command_id,
                original_request_hash: String::new(),
            });
        }
        AcquireResult::Processing { .. } => {
            tx.commit().await.map_err(storage)?;
            return Err(CancelWorkflowInstanceError::CommandStillProcessing);
        }
    };

    // Step 2: Validate reason
    validate_reason(&cmd.reason)?;

    // Step 3: Load and lock instance FOR UPDATE
    let instance_row: Option<(
        Uuid,
        Uuid,
        Option<Uuid>,
        i32,
        bool,
        Option<String>,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        "SELECT wi.workflow_instance_id, wi.domain_id,
                    wi.current_node_visit_id, wi.workflow_state_version,
                    wi.cancelled,
                    nd.node_type::text,
                    nd.node_key,
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

    let (
        _,
        domain_id,
        current_node_visit_id,
        current_state_version,
        is_cancelled,
        current_node_type,
        current_node_key,
        archived_at,
    ) = instance_row.ok_or(CancelWorkflowInstanceError::InstanceNotFound)?;

    // Step 4: Validate workflow_state_version.
    //
    // `expected_workflow_state_version == 0` is the HTTP adapter sentinel for
    // "no client-side optimistic version": the cancel contract does not carry a
    // state version, so authoritative state checks run atomically under the row
    // lock below. Explicit non-zero expectations are validated strictly.
    if cmd.expected_workflow_state_version != 0
        && cmd.expected_workflow_state_version != current_state_version
    {
        return Err(CancelWorkflowInstanceError::WorkflowStateVersionConflict {
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
        return Err(CancelWorkflowInstanceError::NotDomainOwner);
    }

    // Step 6: Check instance state
    if is_cancelled {
        return Err(CancelWorkflowInstanceError::AlreadyCancelled);
    }
    if current_node_type.as_deref() == Some("TERMINAL") {
        return Err(CancelWorkflowInstanceError::SourceNodeTerminal);
    }
    if archived_at.is_some() {
        return Err(CancelWorkflowInstanceError::InstanceArchived);
    }

    let source_visit_id = current_node_visit_id.ok_or_else(|| {
        CancelWorkflowInstanceError::InternalConsistency(
            "instance has no current node visit".to_string(),
        )
    })?;

    let node_key = current_node_key.unwrap_or_default();

    super::assistance_transaction::void_open_cases(
        &mut tx,
        instance_uuid,
        principal_uuid,
        actual_command_id,
        "INSTANCE_CANCELLED",
    )
    .await
    .map_err(|error| CancelWorkflowInstanceError::StorageError(error.to_string()))?;

    // VISIT_ACTIVATION_V1: close the current activation in this same
    // transaction (v0.4.0 §5.9; CTR-VAI-006). The closure's Event FK is
    // deferred, so writing it before the Step 9 Event insert is valid.
    let (semantic_model_version,): (i16,) = sqlx::query_as(
        "SELECT wdv.semantic_model_version \
         FROM workflow_instances wi \
         JOIN workflow_definition_versions wdv \
           ON wdv.definition_version_id = wi.definition_version_id \
         WHERE wi.workflow_instance_id = $1",
    )
    .bind(instance_uuid)
    .fetch_one(&mut *tx)
    .await
    .map_err(storage)?;
    if semantic_model_version == 3 {
        let closed = super::activation_facts::close_activation_by_visit_required(
            &mut tx,
            instance_uuid,
            source_visit_id,
            super::activation_facts::CLOSURE_REASON_CANCELLED,
            actual_command_id,
            Some(event_id),
        )
        .await
        .map_err(storage)?;
        if !closed {
            return Err(CancelWorkflowInstanceError::InternalConsistency(
                "VISIT_ACTIVATION_V1 current visit has no active activation to close"
                    .to_string(),
            ));
        }
    }

    // Step 7: Update instance — set cancelled flag, increment state version.
    // Note: we deliberately do NOT null the node visit assignee here because the
    // database enforces a CHECK constraint that non-terminal node visits must have
    // a non-null assignee.  The cancelled flag + worklist filter + transition block
    // effectively close the work item without violating the constraint.
    let old_state_version = current_state_version;
    let new_state_version = old_state_version + 1;
    let now = chrono::Utc::now();

    let updated = sqlx::query(
        "UPDATE workflow_instances
         SET cancelled = TRUE,
             cancelled_at = $2,
             cancelled_by_principal_id = $3,
             cancel_reason = $4,
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
        return Err(CancelWorkflowInstanceError::InternalConsistency(
            "cancel update affected an unexpected row count".to_string(),
        ));
    }

    // Step 9: Write WORKFLOW_INSTANCE_CANCELLED event
    let event_data = WorkflowInstanceCancelledEventData {
        reason: cmd.reason.clone(),
        cancelled_by_principal_id: principal_uuid.to_string(),
        cancelled_from_node_key: node_key,
    };

    let event_data_json = serde_json::to_value(&event_data)
        .map_err(|e| CancelWorkflowInstanceError::StorageError(e.to_string()))?;
    let event_data_digest = digest::compute_json_digest(&event_data_json)
        .map_err(|e| CancelWorkflowInstanceError::StorageError(e.to_string()))?;

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
                $7, NULL, NULL, NULL, $8, $9, $10, NULL, NULL, $11, $12)
        "#,
    )
    .bind(event_id)
    .bind(instance_uuid)
    .bind(new_state_version)
    .bind(EVENT_SCHEMA_VERSION)
    .bind(actual_command_id)
    .bind(WORKFLOW_INSTANCE_CANCELLED_EVENT_TYPE)
    .bind(source_visit_id)
    .bind(&event_data_json)
    .bind(event_data_digest)
    .bind(principal_uuid)
    .bind(old_state_version)
    .bind(new_state_version)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        CancelWorkflowInstanceError::StorageError(format!("event insert failed: {}", e))
    })?;

    // Step 10: Complete the receipt
    let result = CancelResult {
        workflow_instance_id: instance_uuid,
        workflow_state_version: new_state_version,
        event_sequence: new_state_version,
        replayed: false,
    };

    let response_body = serde_json::to_value(&result)
        .map_err(|e| CancelWorkflowInstanceError::StorageError(e.to_string()))?;

    complete_receipt(&mut tx, actual_command_id, 200, &response_body).await?;

    // Step 11: Commit
    tx.commit().await.map_err(storage)?;

    Ok(result)
}

async fn write_attempt_audit(
    tx: &mut Transaction<'_, Postgres>,
    audit_id: Uuid,
    command_id: Uuid,
    principal_id: Uuid,
    idempotency_key: &str,
    attempt_type: &str,
    failure_reason: Option<&str>,
    request_hash: &str,
) -> Result<(), CancelWorkflowInstanceError> {
    sqlx::query(
        r#"
        INSERT INTO workflow_command_attempt_audits
            (audit_id, command_id, principal_id, idempotency_key, attempt_type,
             failure_reason, request_hash, details)
        VALUES ($1, $2, $3, $4, $5, $6, $7, '{}'::jsonb)
        "#,
    )
    .bind(audit_id)
    .bind(command_id)
    .bind(principal_id)
    .bind(idempotency_key)
    .bind(attempt_type)
    .bind(failure_reason)
    .bind(request_hash)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;

    Ok(())
}

fn error_from_body(body: &serde_json::Value) -> CancelWorkflowInstanceError {
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
        "not_domain_owner" => CancelWorkflowInstanceError::NotDomainOwner,
        "already_cancelled" => CancelWorkflowInstanceError::AlreadyCancelled,
        "source_node_terminal" => CancelWorkflowInstanceError::SourceNodeTerminal,
        "instance_archived" => CancelWorkflowInstanceError::InstanceArchived,
        "invalid_reason" => CancelWorkflowInstanceError::InvalidReason(detail()),
        "workflow_state_version_conflict" => {
            CancelWorkflowInstanceError::WorkflowStateVersionConflict {
                expected: body["expected"].as_i64().unwrap_or_default() as i32,
                actual: body["actual"].as_i64().unwrap_or_default() as i32,
            }
        }
        _ => CancelWorkflowInstanceError::InternalConsistency(detail()),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct CancelResult {
    pub workflow_instance_id: Uuid,
    pub workflow_state_version: i32,
    pub event_sequence: i32,
    #[serde(default)]
    pub replayed: bool,
}
