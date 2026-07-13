//! Atomic workflow instance creation transaction.
//!
//! Implements the core atomic transaction that creates:
//! 1. CommandReceipt (with idempotency handling)
//! 2. WorkflowInstance
//! 3. WorkflowContextRevision #1
//! 4. NodeVisit #1 (initial DRAFT node)
//! 5. INSTANCE_CREATED WorkflowEvent #1
//! 6. Receipt completion

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use crate::domain::workflow_instance::errors::CreateWorkflowInstanceError;
use crate::domain::workflow_instance::events::{
    InstanceCreatedEventData, EVENT_SCHEMA_VERSION, INSTANCE_CREATED_EVENT_TYPE,
};

use super::command_receipt::{
    self, complete_receipt, try_insert_receipt, write_attempt_audit, ReceiptReplayResult,
};
use super::definition_lookup::{self, lock_and_validate_version, read_draft_node};
use super::validation_helpers;

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
    pub workflow_instance_id: Uuid,
    pub workflow_state_version: i32,
    pub current_context_revision_id: Uuid,
    pub current_node_visit_id: Uuid,
    pub event_sequence: i32,
}

/// Execute the full atomic creation workflow inside a single transaction.
///
/// The caller must have already validated principal existence and enabled status.
pub(crate) async fn create_workflow_instance_atomically(
    pool: &PgPool,
    cmd: CreateWorkflowInstanceCommand,
    request_hash: &str,
) -> Result<CreateOutcome, CreateWorkflowInstanceError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // Pre-generate all IDs
    let command_id = Uuid::new_v4();
    let workflow_instance_id = Uuid::new_v4();
    let context_revision_id = Uuid::new_v4();
    let node_visit_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();

    let principal_uuid = cmd.principal_id.into_uuid();
    let domain_uuid = cmd.domain_id.into_uuid();
    let definition_version_uuid = cmd.definition_version_id.into_uuid();

    // ---------------------------------------------------------------
    // Step 1: Insert command receipt (idempotency gate)
    // ---------------------------------------------------------------
    let receipt_owned = try_insert_receipt(
        &mut tx,
        command_id,
        principal_uuid,
        &cmd.idempotency_key,
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

    // ---------------------------------------------------------------
    // Steps 2-5: Validation inside the transaction
    // ---------------------------------------------------------------

    // Step 2: Lock and validate definition version
    let version_info =
        lock_and_validate_version(&mut tx, definition_version_uuid, domain_uuid).await?;

    // Step 3: Validate domain enabled
    if let Some(err) = validation_helpers::validate_domain_enabled(&mut tx, domain_uuid).await? {
        let status_code = validation_helpers::deterministic_error_code(&err);
        let error_code = validation_helpers::deterministic_error_label(&err);
        let response_body = serde_json::json!({"error": error_code});
        let response_digest = digest::compute_sha256(error_code.as_bytes());
        complete_receipt(
            &mut tx,
            actual_command_id,
            status_code,
            &response_body,
            &response_digest,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        return Err(err);
    }

    // Step 4: Verify principal enabled (inside tx for consistency)
    if let Some(err) =
        validation_helpers::validate_principal_enabled(&mut tx, principal_uuid).await?
    {
        let status_code = validation_helpers::deterministic_error_code(&err);
        let error_code = validation_helpers::deterministic_error_label(&err);
        let response_body = serde_json::json!({"error": error_code});
        let response_digest = digest::compute_sha256(error_code.as_bytes());
        complete_receipt(
            &mut tx,
            actual_command_id,
            status_code,
            &response_body,
            &response_digest,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        return Err(err);
    }

    // Step 5: Verify domain membership
    if let Some(err) =
        validation_helpers::validate_domain_membership(&mut tx, domain_uuid, principal_uuid).await?
    {
        let status_code = validation_helpers::deterministic_error_code(&err);
        let error_code = validation_helpers::deterministic_error_label(&err);
        let response_body = serde_json::json!({"error": error_code});
        let response_digest = digest::compute_sha256(error_code.as_bytes());
        complete_receipt(
            &mut tx,
            actual_command_id,
            status_code,
            &response_body,
            &response_digest,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        return Err(err);
    }

    // Step 6: Read the initial DRAFT node
    let draft_node = read_draft_node(&mut tx, definition_version_uuid).await?;

    // Step 7: Resolve the initial assignee
    let resolved_assignee_id =
        validation_helpers::resolve_assignee(&mut tx, &draft_node, principal_uuid, domain_uuid)
            .await?;

    // Step 8: Validate context payload against schema
    // This is a deterministic failure — if validation fails, we complete the
    // receipt with an error and commit, leaving a record for idempotent replay.
    if let Err(err) =
        validation_helpers::validate_context_schema(&version_info.context_schema, &cmd)
    {
        let status_code = validation_helpers::deterministic_error_code(&err);
        let error_code = validation_helpers::deterministic_error_label(&err);
        let response_body = serde_json::json!({"error": error_code});
        let response_digest = digest::compute_sha256(error_code.as_bytes());
        complete_receipt(
            &mut tx,
            actual_command_id,
            status_code,
            &response_body,
            &response_digest,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        return Err(err);
    }

    // ---------------------------------------------------------------
    // Step 9: Insert WorkflowInstance
    // ---------------------------------------------------------------
    let workflow_state_version = 1i32;

    sqlx::query(
        r#"
        INSERT INTO workflow_instances
            (workflow_instance_id, domain_id, definition_version_id,
             created_by_principal_id, workflow_state_version,
             current_context_revision_id, current_node_visit_id,
             external_reference, external_url, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(workflow_instance_id)
    .bind(domain_uuid)
    .bind(definition_version_uuid)
    .bind(principal_uuid)
    .bind(workflow_state_version)
    .bind(context_revision_id)
    .bind(node_visit_id)
    .bind(&cmd.external_reference)
    .bind(&cmd.external_url)
    .bind(&cmd.metadata)
    .execute(&mut *tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // ---------------------------------------------------------------
    // Step 10: Insert WorkflowContextRevision #1
    // ---------------------------------------------------------------
    let revision_number = 1i32;
    let payload_digest = digest::compute_json_digest(&cmd.context_payload)
        .map_err(CreateWorkflowInstanceError::StorageError)?;

    sqlx::query(
        r#"
        INSERT INTO workflow_context_revisions
            (context_revision_id, workflow_instance_id, revision_number,
             previous_revision_id, payload, payload_digest,
             created_by_principal_id)
        VALUES ($1, $2, $3, NULL, $4, $5, $6)
        "#,
    )
    .bind(context_revision_id)
    .bind(workflow_instance_id)
    .bind(revision_number)
    .bind(&cmd.context_payload)
    .bind(&payload_digest)
    .bind(principal_uuid)
    .execute(&mut *tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // ---------------------------------------------------------------
    // Step 11: Insert NodeVisit #1
    // ---------------------------------------------------------------
    let visit_number = 1i32;

    sqlx::query(
        r#"
        INSERT INTO workflow_node_visits
            (node_visit_id, workflow_instance_id, node_id, visit_number,
             assignee_principal_id, entered_by_transition_id)
        VALUES ($1, $2, $3, $4, $5, NULL)
        "#,
    )
    .bind(node_visit_id)
    .bind(workflow_instance_id)
    .bind(draft_node.node_id)
    .bind(visit_number)
    .bind(resolved_assignee_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // ---------------------------------------------------------------
    // Step 12: Insert INSTANCE_CREATED WorkflowEvent #1
    // ---------------------------------------------------------------
    let event_sequence = 1i32;
    let definition_digest_str = version_info.definition_digest.as_deref().unwrap_or("");

    let event_data = InstanceCreatedEventData {
        definition_version_id: definition_version_uuid.to_string(),
        definition_digest: definition_digest_str.to_string(),
        initial_node_id: draft_node.node_id.to_string(),
        assignee_resolution_type: draft_node.assignee_ref_type.to_string(),
    };

    let event_data_json = serde_json::to_value(&event_data)
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
    let event_data_digest = digest::compute_json_digest(&event_data_json)
        .map_err(CreateWorkflowInstanceError::StorageError)?;

    sqlx::query(
        r#"
        INSERT INTO workflow_events
            (event_id, workflow_instance_id, event_sequence, event_schema_version,
             command_id, event_type, source_node_visit_id, target_node_visit_id,
             context_revision_id, event_data, event_data_digest,
             actor_principal_id, old_workflow_state_version, new_workflow_state_version)
        VALUES ($1, $2, $3, $4, $5, $6, NULL, $7, $8, $9, $10, $11, 0, 1)
        "#,
    )
    .bind(event_id)
    .bind(workflow_instance_id)
    .bind(event_sequence)
    .bind(EVENT_SCHEMA_VERSION)
    .bind(actual_command_id)
    .bind(INSTANCE_CREATED_EVENT_TYPE)
    .bind(node_visit_id)
    .bind(context_revision_id)
    .bind(&event_data_json)
    .bind(&event_data_digest)
    .bind(principal_uuid)
    .execute(&mut *tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // ---------------------------------------------------------------
    // Step 13: Complete the command receipt
    // ---------------------------------------------------------------
    let response_body = serde_json::json!({
        "workflowInstanceId": workflow_instance_id,
        "workflowStateVersion": workflow_state_version,
        "currentContextRevisionId": context_revision_id,
        "currentNodeVisitId": node_visit_id,
        "eventSequence": event_sequence,
    });

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
    tx.commit()
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    Ok(CreateOutcome::Created(CreateResult {
        workflow_instance_id,
        workflow_state_version,
        current_context_revision_id: context_revision_id,
        current_node_visit_id: node_visit_id,
        event_sequence,
    }))
}
