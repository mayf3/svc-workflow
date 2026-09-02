//! Atomic WAKE_DISPATCH_INTENT transaction (VISIT_ACTIVATION_V1).
//!
//! Implements CTR-VAI-008 of SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1:
//! - binds the exact Instance + nodeVisitId;
//! - applies only when the activation exists, is a DISPATCH_INTENT, is
//!   active, the Visit is current, the caller's expected
//!   workflowStateVersion matches, and the intent is not already due;
//! - writes one immutable eligibility fact (previous = current,
//!   new = authoritative server now), increments workflowStateVersion once,
//!   and writes exactly one WAKE_DISPATCH_INTENT Event — atomically;
//! - stale / closed / version-mismatch / already-due wake is a durable
//!   no-op (200 with `wakeApplied: false`, attempt audit, NO version
//!   increment, NO Event, NO fact row).

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::workflow_instance::commands::WakeDispatchIntentCommand;
use crate::domain::workflow_instance::errors::WakeDispatchIntentError;
use crate::domain::workflow_instance::events::{
    WakeDispatchIntentEventData, COMMAND_TYPE_WAKE_DISPATCH_INTENT, EVENT_SCHEMA_VERSION,
    WAKE_DISPATCH_INTENT_EVENT_TYPE,
};

use super::activation_facts::{self, CAUSE_CLASS_WAKE};

/// Outcome of a wake attempt against storage.
pub enum WakeOutcome {
    /// Wake applied: eligibility fact + version + Event committed.
    Applied(WakeAppliedResult),
    /// Durable no-op: receipt + audit committed, no workflow state change.
    NoOp(WakeNoOpResult),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeAppliedResult {
    pub workflow_instance_id: Uuid,
    pub node_visit_id: Uuid,
    pub workflow_state_version: i32,
    pub event_sequence: i32,
    pub next_eligible_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing, default)]
    pub replayed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeNoOpResult {
    pub workflow_instance_id: Uuid,
    pub node_visit_id: Uuid,
    pub reason: String,
    #[serde(skip_serializing, default)]
    pub replayed: bool,
}

fn storage(error: sqlx::Error) -> WakeDispatchIntentError {
    WakeDispatchIntentError::StorageError(error.to_string())
}

/// Insert (or look up) the command receipt for this wake attempt.
/// Returns `Ok(Some(command_id))` when the caller owns the request.
async fn acquire_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command_id: Uuid,
    principal_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
) -> Result<ReceiptAcquire, WakeDispatchIntentError> {
    let inserted: Option<(Uuid, String, Option<i32>, Option<serde_json::Value>)> = sqlx::query_as(
        "INSERT INTO workflow_command_receipts
             (command_id, principal_id, idempotency_key, command_type,
              request_hash, receipt_status)
         VALUES ($1, $2, $3, $4, $5, 'PROCESSING')
         ON CONFLICT (principal_id, idempotency_key) DO NOTHING
         RETURNING command_id, command_type, response_status, response_body",
    )
    .bind(command_id)
    .bind(principal_id)
    .bind(idempotency_key)
    .bind(COMMAND_TYPE_WAKE_DISPATCH_INTENT)
    .bind(request_hash)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;

    if inserted.is_some() {
        return Ok(ReceiptAcquire::Owned(command_id));
    }

    // Someone else owns this (principal, key) — classify replay.
    let existing: Option<(Uuid, String, String, Option<i32>, Option<serde_json::Value>)> =
        sqlx::query_as(
            "SELECT command_id, command_type, request_hash, response_status, response_body
               FROM workflow_command_receipts
              WHERE principal_id = $1 AND idempotency_key = $2",
        )
        .bind(principal_id)
        .bind(idempotency_key)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?;

    match existing {
        None => Err(WakeDispatchIntentError::InternalConsistency(
            "receipt insert neither inserted nor found an existing row".to_string(),
        )),
        Some((existing_command_id, _, existing_hash, status, body)) => {
            if existing_hash != request_hash {
                return Ok(ReceiptAcquire::Conflict {
                    command_id: existing_command_id,
                });
            }
            match (status, body) {
                (Some(200), Some(body)) => Ok(ReceiptAcquire::ReplaySuccess { body }),
                (Some(_), Some(body)) => Ok(ReceiptAcquire::ReplayFailure { body }),
                _ => Ok(ReceiptAcquire::Processing),
            }
        }
    }
}

enum ReceiptAcquire {
    Owned(Uuid),
    ReplaySuccess { body: serde_json::Value },
    ReplayFailure { body: serde_json::Value },
    Conflict { command_id: Uuid },
    Processing,
}

async fn complete_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command_id: Uuid,
    status: i32,
    body: &serde_json::Value,
) -> Result<(), WakeDispatchIntentError> {
    let response_digest = digest::compute_json_digest(body)
        .map_err(WakeDispatchIntentError::StorageError)?;
    sqlx::query(
        "UPDATE workflow_command_receipts
            SET receipt_status = 'COMPLETED', response_status = $2,
                response_body = $3, response_digest = $4, completed_at = now()
          WHERE command_id = $1",
    )
    .bind(command_id)
    .bind(status)
    .bind(body)
    .bind(&response_digest)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn write_attempt_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    audit_id: Uuid,
    command_id: Uuid,
    principal_id: Uuid,
    idempotency_key: &str,
    outcome: &str,
    detail: Option<&str>,
    request_hash: &str,
) -> Result<(), WakeDispatchIntentError> {
    sqlx::query(
        "INSERT INTO workflow_command_attempt_audits
             (audit_id, command_id, principal_id, idempotency_key,
              attempt_type, failure_reason, request_hash)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(audit_id)
    .bind(command_id)
    .bind(principal_id)
    .bind(idempotency_key)
    .bind(outcome)
    .bind(detail)
    .bind(request_hash)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}

/// Deterministic failure: persist the failure receipt, commit, and return
/// the error (the caller's HTTP mapping keeps the original class).
macro_rules! deterministic_failure {
    ($tx:expr, $command_id:expr, $principal:expr, $key:expr, $hash:expr,
     $status:expr, $code:expr, $err:expr) => {{
        let body = serde_json::json!({ "error": $code });
        complete_receipt($tx, $command_id, $status, &body).await?;
        return Err($err);
    }};
}

/// Execute the atomic wake.
pub async fn wake_dispatch_intent_atomically(
    pool: &PgPool,
    cmd: WakeDispatchIntentCommand,
    request_hash: &str,
) -> Result<WakeOutcome, WakeDispatchIntentError> {
    let principal_uuid = cmd.principal_id.into_uuid();
    let instance_uuid = cmd.workflow_instance_id.into_uuid();
    let visit_uuid = cmd.node_visit_id.into_uuid();
    let mut tx = pool.begin().await.map_err(storage)?;

    // ---------------------------------------------------------------
    // Step 1: Acquire command receipt (idempotency gate)
    // ---------------------------------------------------------------
    let command_id = Uuid::new_v4();
    let receipt = acquire_receipt(
        &mut tx,
        command_id,
        principal_uuid,
        &cmd.idempotency_key,
        request_hash,
    )
    .await?;

    let actual_command_id = match receipt {
        ReceiptAcquire::Owned(id) => id,
        ReceiptAcquire::ReplaySuccess { body } => {
            tx.commit().await.map_err(storage)?;
            let mut result: WakeAppliedResult = serde_json::from_value(body)
                .map_err(|e| WakeDispatchIntentError::InternalConsistency(e.to_string()))?;
            result.replayed = true;
            return Ok(WakeOutcome::Applied(result));
        }
        ReceiptAcquire::ReplayFailure { body } => {
            tx.commit().await.map_err(storage)?;
            let mut result: WakeNoOpResult = serde_json::from_value(body)
                .map_err(|e| WakeDispatchIntentError::InternalConsistency(e.to_string()))?;
            result.replayed = true;
            return Ok(WakeOutcome::NoOp(result));
        }
        ReceiptAcquire::Conflict { command_id } => {
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
            return Err(WakeDispatchIntentError::IdempotencyConflict {
                original_command_id: command_id,
                original_request_hash: String::new(),
            });
        }
        ReceiptAcquire::Processing => {
            tx.commit().await.map_err(storage)?;
            return Err(WakeDispatchIntentError::CommandStillProcessing);
        }
    };

    // ---------------------------------------------------------------
    // Step 2: Validate the optional wake cause (deterministic failure).
    // ---------------------------------------------------------------
    if let Some(cause) = &cmd.cause {
        let invalid = cause.is_empty()
            || cause.len() > 64
            || cause.chars().any(|c| c.is_control());
        if invalid {
            deterministic_failure!(
                &mut tx, actual_command_id, principal_uuid, &cmd.idempotency_key,
                request_hash, 422, "invalid_cause",
                WakeDispatchIntentError::InvalidCause(cause.clone())
            );
        }
    }

    // ---------------------------------------------------------------
    // Step 3: Validate actor principal (existence + enabled).
    // ---------------------------------------------------------------
    let actor_row: Option<(bool,)> =
        sqlx::query_as("SELECT enabled FROM principals WHERE principal_id = $1")
            .bind(principal_uuid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?;
    match actor_row {
        None => deterministic_failure!(
            &mut tx, actual_command_id, principal_uuid, &cmd.idempotency_key,
            request_hash, 404, "principal_not_found",
            WakeDispatchIntentError::PrincipalNotFound
        ),
        Some((false,)) => deterministic_failure!(
            &mut tx, actual_command_id, principal_uuid, &cmd.idempotency_key,
            request_hash, 403, "principal_disabled",
            WakeDispatchIntentError::PrincipalDisabled
        ),
        Some((true,)) => {}
    }

    // ---------------------------------------------------------------
    // Step 4: Lock the instance and read projections.
    // ---------------------------------------------------------------
    let instance_row: Option<(i32, Option<Uuid>, bool, Option<chrono::DateTime<chrono::Utc>>)> =
        sqlx::query_as(
            "SELECT workflow_state_version, current_node_visit_id, cancelled, archived_at
               FROM workflow_instances
              WHERE workflow_instance_id = $1
              FOR UPDATE",
        )
        .bind(instance_uuid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?;

    let Some((current_state_version, current_visit_id, cancelled, archived_at)) = instance_row
    else {
        deterministic_failure!(
            &mut tx, actual_command_id, principal_uuid, &cmd.idempotency_key,
            request_hash, 404, "instance_not_found",
            WakeDispatchIntentError::InstanceNotFound
        );
    };

    // ---------------------------------------------------------------
    // Step 5: Resolve the DISPATCH_INTENT activation for this Visit.
    // ---------------------------------------------------------------
    let activation = activation_facts::find_dispatch_activation_for_wake(
        &mut tx,
        instance_uuid,
        visit_uuid,
    )
    .await
    .map_err(storage)?;

    let Some(activation) = activation else {
        deterministic_failure!(
            &mut tx, actual_command_id, principal_uuid, &cmd.idempotency_key,
            request_hash, 404, "dispatch_intent_not_found",
            WakeDispatchIntentError::DispatchIntentNotFound
        );
    };

    // ---------------------------------------------------------------
    // Step 6: Classify durable no-ops (200 wakeApplied=false, no mutation).
    // ---------------------------------------------------------------
    // Authoritative server now comes from the database clock so the
    // eligibility fact, the due comparison, and the Event share one instant.
    let (db_now,): (chrono::DateTime<chrono::Utc>,) =
        sqlx::query_as("SELECT now()::timestamptz")
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;

    let no_op_reason: Option<&'static str> = if cancelled || archived_at.is_some() {
        Some("INSTANCE_CLOSED")
    } else if activation.closed_at.is_some() {
        Some("ACTIVATION_CLOSED")
    } else if current_visit_id != Some(visit_uuid) {
        Some("VISIT_NOT_CURRENT")
    } else if cmd.expected_workflow_state_version != current_state_version {
        Some("VERSION_MISMATCH")
    } else if let Some(current) = activation.current_next_eligible_at {
        if current <= db_now {
            Some("ALREADY_DUE")
        } else {
            None
        }
    } else {
        Some("ACTIVATION_CLOSED")
    };

    if let Some(reason) = no_op_reason {
        let body = serde_json::json!({
            "wakeApplied": false,
            "reason": reason,
            "workflowInstanceId": instance_uuid,
            "nodeVisitId": visit_uuid,
        });
        let audit_id = Uuid::new_v4();
        write_attempt_audit(
            &mut tx,
            audit_id,
            actual_command_id,
            principal_uuid,
            &cmd.idempotency_key,
            "WAKE_NO_OP",
            Some(reason),
            request_hash,
        )
        .await?;
        complete_receipt(&mut tx, actual_command_id, 200, &body).await?;
        tx.commit().await.map_err(storage)?;
        return Ok(WakeOutcome::NoOp(WakeNoOpResult {
            workflow_instance_id: instance_uuid,
            node_visit_id: visit_uuid,
            reason: reason.to_string(),
            replayed: false,
        }));
    }

    // ---------------------------------------------------------------
    // Step 7: Apply the wake — eligibility fact + version + Event, atomic.
    // ---------------------------------------------------------------
    let previous = activation
        .current_next_eligible_at
        .ok_or_else(|| {
            WakeDispatchIntentError::InternalConsistency(
                "active DISPATCH_INTENT without a current nextEligibleAt".to_string(),
            )
        })?;

    let eligibility_event_id = Uuid::new_v4();
    activation_facts::insert_eligibility_event(
        &mut tx,
        eligibility_event_id,
        activation.activation_id,
        previous,
        db_now,
        CAUSE_CLASS_WAKE,
        actual_command_id,
    )
    .await
    .map_err(storage)?;

    let new_state_version = current_state_version
        .checked_add(1)
        .ok_or_else(|| {
            WakeDispatchIntentError::InternalConsistency(
                "workflow state version overflow".to_string(),
            )
        })?;

    let affected = sqlx::query(
        "UPDATE workflow_instances
            SET workflow_state_version = $2, updated_at = now()
          WHERE workflow_instance_id = $1 AND workflow_state_version = $3",
    )
    .bind(instance_uuid)
    .bind(new_state_version)
    .bind(current_state_version)
    .execute(&mut *tx)
    .await
    .map_err(storage)?
    .rows_affected();
    if affected != 1 {
        return Err(WakeDispatchIntentError::InternalConsistency(
            "wake projection update affected an unexpected row count".to_string(),
        ));
    }

    let event_id = Uuid::new_v4();
    let event_sequence = new_state_version;
    let event_data = WakeDispatchIntentEventData {
        activation_id: activation.activation_id.to_string(),
        node_visit_id: visit_uuid.to_string(),
        previous_next_eligible_at: previous.to_rfc3339(),
        new_next_eligible_at: db_now.to_rfc3339(),
        cause_class: CAUSE_CLASS_WAKE.to_string(),
    };
    let event_data_json = serde_json::to_value(&event_data)
        .map_err(|e| WakeDispatchIntentError::StorageError(e.to_string()))?;
    let event_data_digest = digest::compute_json_digest(&event_data_json)
        .map_err(WakeDispatchIntentError::StorageError)?;

    sqlx::query(
        "INSERT INTO workflow_events
             (event_id, workflow_instance_id, event_sequence, event_schema_version,
              command_id, event_type, source_node_visit_id, event_data,
              event_data_digest, actor_principal_id,
              old_workflow_state_version, new_workflow_state_version)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(event_id)
    .bind(instance_uuid)
    .bind(event_sequence)
    .bind(EVENT_SCHEMA_VERSION)
    .bind(actual_command_id)
    .bind(WAKE_DISPATCH_INTENT_EVENT_TYPE)
    .bind(visit_uuid)
    .bind(&event_data_json)
    .bind(&event_data_digest)
    .bind(principal_uuid)
    .bind(current_state_version)
    .bind(new_state_version)
    .execute(&mut *tx)
    .await
    .map_err(storage)?;

    // ---------------------------------------------------------------
    // Step 8: Complete the receipt and commit.
    // ---------------------------------------------------------------
    let body = serde_json::json!({
        "wakeApplied": true,
        "workflowInstanceId": instance_uuid,
        "nodeVisitId": visit_uuid,
        "workflowStateVersion": new_state_version,
        "eventSequence": event_sequence,
        "nextEligibleAt": db_now.to_rfc3339(),
    });
    complete_receipt(&mut tx, actual_command_id, 200, &body).await?;
    tx.commit().await.map_err(storage)?;

    Ok(WakeOutcome::Applied(WakeAppliedResult {
        workflow_instance_id: instance_uuid,
        node_visit_id: visit_uuid,
        workflow_state_version: new_state_version,
        event_sequence,
        next_eligible_at: db_now,
        replayed: false,
    }))
}
