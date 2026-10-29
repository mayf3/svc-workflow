//! Atomic workflow instance creation transaction.
//!
//! Implements the core atomic transaction that creates:
//! 1. CommandReceipt (with idempotency handling)
//! 2. WorkflowInstance
//! 3. WorkflowContextRevision #1
//! 4. NodeVisit #1 (initial DRAFT node)
//! 5. INSTANCE_CREATED WorkflowEvent #1
//! 6. Receipt completion

use std::collections::BTreeSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::enums::WorkflowExecutionClass;
use crate::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use crate::domain::workflow_instance::errors::CreateWorkflowInstanceError;
use crate::domain::workflow_instance::events::{
    InstanceCreatedEventData, COMMAND_TYPE_CREATE_INSTANCE, EVENT_SCHEMA_VERSION,
    INSTANCE_CREATED_EVENT_TYPE,
};
use crate::store::postgres::admission_gate::{self as admission_gate_module, AdmissionGate};

use super::activation_facts;
use super::command_receipt::{
    self, complete_receipt, try_insert_receipt, write_attempt_audit, ReceiptReplayResult,
};
use super::definition_lookup::{
    self, lock_and_validate_version, read_draft_node, read_minimal_entry_node,
    read_visit_activation_entry_node,
};
use super::validation_helpers;

mod facts;
pub(crate) use facts::create_on_tx;

/// Outcome of an atomic creation attempt.
pub(crate) enum CreateOutcome {
    /// Fresh successful creation.
    Created(CreateResult),
    /// Idempotent replay of a SUCCESSFUL request — the same IDs as the original.
    Replayed(CreateResult),
    /// Idempotent replay of a FAILED request — the original error should be returned.
    ReplayedFailure(i32, serde_json::Value),
}

/// Result of a successful atomic creation.
pub(crate) struct CreateResult {
    pub admission_observation: Option<serde_json::Value>,
    pub prerequisite_readiness: Option<serde_json::Value>,
    pub workflow_instance_id: Uuid,
    pub workflow_state_version: i32,
    pub current_context_revision_id: Uuid,
    pub current_node_visit_id: Uuid,
    pub event_sequence: i32,
}

/// Execute the full atomic creation workflow inside a single transaction.
///
/// The caller pre-validates principal existence because the receipt has a principal FK.
/// Enabled status is validated after receipt ownership for stable deterministic replay.
pub(crate) async fn create_workflow_instance_atomically(
    pool: &PgPool,
    admission: AdmissionGate<'_>,
    cmd: CreateWorkflowInstanceCommand,
    request_hash: &str,
) -> Result<CreateOutcome, CreateWorkflowInstanceError> {
    for attempt in 0..3 {
        match create_once(pool, admission, cmd.clone(), request_hash).await {
            Err(CreateWorkflowInstanceError::StorageError(s))
                if attempt < 2
                    && (s.contains("could not serialize access")
                        || s.contains("deadlock detected")) =>
            {
                continue
            }
            result => return result,
        }
    }
    unreachable!()
}
async fn create_once(
    pool: &PgPool,
    admission: AdmissionGate<'_>,
    cmd: CreateWorkflowInstanceCommand,
    request_hash: &str,
) -> Result<CreateOutcome, CreateWorkflowInstanceError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // CTR-CIR-003: the database statement deadline must be no later than the
    // 5-second admission-through-commit bound. No-op in dormant mode.
    admission
        .bind_statement_deadline(&mut tx)
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    let command_id = Uuid::new_v4();

    let principal_uuid = cmd.principal_id.into_uuid();

    // ---------------------------------------------------------------
    // Step 1: Insert command receipt (idempotency gate)
    // ---------------------------------------------------------------
    let receipt_owned = try_insert_receipt(
        &mut tx,
        command_id,
        principal_uuid,
        &cmd.idempotency_key,
        COMMAND_TYPE_CREATE_INSTANCE,
        request_hash,
    )
    .await?;

    let actual_command_id = match receipt_owned {
        Some(cmd_id) => cmd_id, // We own this request — proceed
        None => {
            // Another receipt exists — handle replay/conflict
            let replay = command_receipt::replay_existing_receipt(
                &mut tx,
                principal_uuid,
                &cmd.idempotency_key,
                request_hash,
            )
            .await?;

            match replay {
                ReceiptReplayResult::CompletedMatch {
                    command_id: _,
                    response_status,
                    response_body,
                } => {
                    if response_status != 200 {
                        // Deterministic failure replay — commit and return the original error
                        tx.commit().await.map_err(|e| {
                            CreateWorkflowInstanceError::StorageError(e.to_string())
                        })?;
                        return Ok(CreateOutcome::ReplayedFailure(
                            response_status,
                            response_body,
                        ));
                    }

                    // Idempotent replay of a SUCCESSFUL request — extract original IDs
                    let wf_id = response_body["workflowInstanceId"]
                        .as_str()
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .ok_or_else(|| {
                            CreateWorkflowInstanceError::StorageError(
                                "stored response missing workflowInstanceId".to_string(),
                            )
                        })?;
                    let ctx_rev_id = response_body["currentContextRevisionId"]
                        .as_str()
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .ok_or_else(|| {
                            CreateWorkflowInstanceError::StorageError(
                                "stored response missing currentContextRevisionId".to_string(),
                            )
                        })?;
                    let visit_id = response_body["currentNodeVisitId"]
                        .as_str()
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .ok_or_else(|| {
                            CreateWorkflowInstanceError::StorageError(
                                "stored response missing currentNodeVisitId".to_string(),
                            )
                        })?;
                    let state_ver =
                        response_body["workflowStateVersion"].as_i64().unwrap_or(1) as i32;
                    let ev_seq = response_body["eventSequence"].as_i64().unwrap_or(1) as i32;

                    tx.commit()
                        .await
                        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

                    return Ok(CreateOutcome::Replayed(CreateResult {
                        admission_observation: response_body.get("admissionObservation").cloned(),
                        prerequisite_readiness: None,
                        workflow_instance_id: wf_id,
                        workflow_state_version: state_ver,
                        current_context_revision_id: ctx_rev_id,
                        current_node_visit_id: visit_id,
                        event_sequence: ev_seq,
                    }));
                }
                ReceiptReplayResult::CompletedConflict {
                    command_id: cid,
                    original_request_hash: orig_hash,
                } => {
                    let audit_id = Uuid::new_v4();
                    let details = serde_json::json!({
                        "conflictType": "IDEMPOTENCY_KEY_MISMATCH",
                        "originalRequestHash": orig_hash,
                        "newRequestHash": request_hash,
                    });
                    write_attempt_audit(
                        &mut tx,
                        audit_id,
                        cid,
                        principal_uuid,
                        &cmd.idempotency_key,
                        "IDEMPOTENCY_CONFLICT",
                        Some("request hash mismatch"),
                        request_hash,
                        Some(&details),
                    )
                    .await?;

                    tx.commit()
                        .await
                        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

                    return Err(CreateWorkflowInstanceError::IdempotencyConflict {
                        original_command_id: cid,
                        original_request_hash: orig_hash,
                    });
                }
                ReceiptReplayResult::StillProcessing => {
                    tx.commit()
                        .await
                        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
                    return Err(CreateWorkflowInstanceError::CommandStillProcessing);
                }
            }
        }
    };

    let result = match create_on_tx(&mut tx, admission, &cmd, actual_command_id).await {
        Ok(result) => result,
        Err(err) if validation_helpers::is_deterministic_error(&err) => {
            validation_helpers::persist_deterministic_failure(tx, actual_command_id, &err).await?;
            return Err(err);
        }
        Err(err) => return Err(err),
    };
    let CreateResult {
        admission_observation,
        prerequisite_readiness,
        workflow_instance_id,
        workflow_state_version,
        current_context_revision_id: context_revision_id,
        current_node_visit_id: node_visit_id,
        event_sequence,
    } = result;

    // ---------------------------------------------------------------
    // Step 13: Complete the command receipt
    // ---------------------------------------------------------------
    let mut response_body = serde_json::json!({
        "workflowInstanceId": workflow_instance_id,
        "workflowStateVersion": workflow_state_version,
        "currentContextRevisionId": context_revision_id,
        "currentNodeVisitId": node_visit_id,
        "eventSequence": event_sequence,
    });

    if let Some(value) = &admission_observation {
        response_body["admissionObservation"] = value.clone();
    }

    let response_digest = digest::compute_json_digest(&response_body)
        .map_err(CreateWorkflowInstanceError::StorageError)?;

    complete_receipt(
        &mut tx,
        actual_command_id,
        200,
        &response_body,
        &response_digest,
    )
    .await?;

    // ---------------------------------------------------------------
    // Step 14: Commit
    // ---------------------------------------------------------------
    // CTR-CIR-003: total admission-through-commit is bounded to 5 seconds —
    // an exhausted budget must not commit (fail closed, zero writes).
    admission
        .check_commit_budget()
        .map_err(CreateWorkflowInstanceError::AdmissionFailed)?;

    // T69 (WF-GS-02, owner-frozen semantics): the remaining absolute budget
    // governs COMMIT itself. statement_timeout does not cover the COMMIT
    // phase — arm a client-side commit deadline; on exceed, the outcome is
    // UNCERTAIN -> fail closed as outcome-unknown (no success claim, no
    // blind retry).
    let commit_deadline_ms = if admission.is_enabled() {
        admission.remaining_budget_ms()
    } else {
        0 // dormant: no commit-phase deadline
    };
    let commit_started = std::time::Instant::now();
    if commit_deadline_ms > 0 {
        match tokio::time::timeout(
            std::time::Duration::from_millis(commit_deadline_ms),
            tx.commit(),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(CreateWorkflowInstanceError::StorageError(e.to_string()));
            }
            Err(_) => {
                return Err(CreateWorkflowInstanceError::CommitOutcomeUnknown {
                    budget_ms: commit_deadline_ms,
                });
            }
        }
    } else {
        tx.commit()
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
    }
    let _ = commit_started;

    Ok(CreateOutcome::Created(CreateResult {
        admission_observation,
        prerequisite_readiness,
        workflow_instance_id,
        workflow_state_version,
        current_context_revision_id: context_revision_id,
        current_node_visit_id: node_visit_id,
        event_sequence,
    }))
}
