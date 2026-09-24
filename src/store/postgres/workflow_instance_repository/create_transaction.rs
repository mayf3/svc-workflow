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
/// The caller pre-validates principal existence because the receipt has a principal FK.
/// Enabled status is validated after receipt ownership for stable deterministic replay.
pub(crate) async fn create_workflow_instance_atomically(
    pool: &PgPool,
    admission: AdmissionGate<'_>,
    cmd: CreateWorkflowInstanceCommand,
    request_hash: &str,
) -> Result<CreateOutcome, CreateWorkflowInstanceError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // CTR-CIR-003: the database statement deadline must be no later than the
    // 5-second admission-through-commit bound. No-op in dormant mode.
    admission
        .bind_statement_deadline(&mut tx)
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // Pre-generate all IDs
    let command_id = Uuid::new_v4();
    let workflow_instance_id = Uuid::new_v4();
    let context_revision_id = Uuid::new_v4();
    let node_visit_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    let activation_id = Uuid::new_v4();

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

    // Every deterministic check below runs before the first runtime fact write.
    macro_rules! deterministic_failure {
        ($err:expr) => {{
            let err = $err;
            validation_helpers::persist_deterministic_failure(tx, actual_command_id, &err).await?;
            return Err(err);
        }};
    }
    macro_rules! validation_result {
        ($result:expr) => {{
            match $result {
                Ok(value) => value,
                Err(err) if validation_helpers::is_deterministic_error(&err) => {
                    deterministic_failure!(err)
                }
                Err(err) => return Err(err),
            }
        }};
    }

    validation_result!(validation_helpers::validate_request_sizes(&cmd));
    let version_info = validation_result!(
        lock_and_validate_version(&mut tx, definition_version_uuid, domain_uuid).await
    );
    if let Some(err) =
        validation_result!(validation_helpers::validate_domain_enabled(&mut tx, domain_uuid).await)
    {
        deterministic_failure!(err);
    }
    if let Some(err) = validation_result!(
        validation_helpers::validate_principal_enabled(&mut tx, principal_uuid).await
    ) {
        deterministic_failure!(err);
    }
    if let Some(err) = validation_result!(
        validation_helpers::validate_domain_membership(&mut tx, domain_uuid, principal_uuid).await
    ) {
        deterministic_failure!(err);
    }
    // Work execution class marking (SVC_WORKFLOW_WORK_EXECUTION_CLASS_V1,
    // CTR-WEC-002): NON_BUSINESS_TEST is a governance decision reserved to
    // the enabled DOMAIN_OWNER of the target domain — the exact in-tx
    // predicate family as cancel/archive. An ordinary member must not be
    // able to suppress work (assigned to any principal) from the automated
    // business dispatcher.
    if cmd.execution_class == WorkflowExecutionClass::NonBusinessTest {
        let is_owner: bool = sqlx::query_scalar(
            "SELECT EXISTS(
               SELECT 1 FROM domain_role_bindings
               WHERE domain_id = $1 AND principal_id = $2
                 AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE)",
        )
        .bind(domain_uuid)
        .bind(principal_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        if !is_owner {
            deterministic_failure!(CreateWorkflowInstanceError::NotDomainOwner);
        }
    }
    // Semantic model dispatch: entry node comes from the definition's
    // semantic model, never guessed from node shape.
    let entry_node = match version_info.semantic_model_version {
        1 => validation_result!(read_draft_node(&mut tx, definition_version_uuid).await),
        2 => {
            // Minimal definitions are trusted to satisfy the Minimal
            // contract — graph legality is the caller's responsibility
            // (they may run validate_minimal_graph before submission).
            // The Runtime keeps only cheap execution fail-closeds: the
            // entry TASK must be determinable, and forbidden DOMAIN_OWNER
            // assignment on the entry is rejected outright.
            let entry =
                validation_result!(read_minimal_entry_node(&mut tx, definition_version_uuid).await);
            if entry.assignee_ref_type == crate::domain::enums::AssigneeRefType::DomainOwner {
                deterministic_failure!(CreateWorkflowInstanceError::AssigneeResolutionFailed(
                    "Minimal (V2) definition entry uses forbidden DOMAIN_OWNER assignee"
                        .to_string(),
                ));
            }
            entry
        }
        3 => {
            // VISIT_ACTIVATION_V1: entry must be a TASK of the new model.
            // Owner references outside the closed set are rejected here as
            // well (the validator already rejects them at publish time).
            let entry = validation_result!(
                read_visit_activation_entry_node(&mut tx, definition_version_uuid).await
            );
            if entry.assignee_ref_type
                == crate::domain::enums::AssigneeRefType::InstanceInputPrincipal
            {
                deterministic_failure!(CreateWorkflowInstanceError::AssigneeResolutionFailed(
                    "VISIT_ACTIVATION_V1 entry uses INSTANCE_INPUT_PRINCIPAL, which is not                      in the new-model owner reference set"
                        .to_string(),
                ));
            }
            entry
        }
        other => {
            return Err(CreateWorkflowInstanceError::InternalConsistency(format!(
                "unknown semantic_model_version {other} for definition version"
            )))
        }
    };
    let resolved_assignee_id = validation_result!(
        validation_helpers::resolve_assignee(
            &mut tx,
            &entry_node,
            principal_uuid,
            domain_uuid,
            &cmd.context_payload,
        )
        .await
    );
    // VISIT_ACTIVATION_V1 owners must resolve to an enabled canonical HUMAN
    // or AGENT Principal; SERVICE can never own new-model work.
    if version_info.semantic_model_version == 3 {
        let owner_type_check =
            activation_facts::validate_owner_is_human_or_agent(&mut tx, resolved_assignee_id)
                .await
                .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        if let Err(reason) = owner_type_check {
            deterministic_failure!(CreateWorkflowInstanceError::AssigneeResolutionFailed(
                reason
            ));
        }
    }
    validation_result!(validation_helpers::validate_context_schema(
        &version_info.context_schema,
        &cmd,
    ));
    // The definition's real node requirements (INSTANCE_INPUT_PRINCIPAL
    // assignee keys) are a hard create-time invariant: every future node
    // that reads its assignee from the context payload must already be
    // resolvable, or the instance would be created half-legal and the
    // read path would fail with an internal consistency error later.
    validation_result!(
        validation_helpers::validate_instance_input_principal_keys(
            &mut tx,
            definition_version_uuid,
            &cmd.context_payload,
        )
        .await
    );

    // ---------------------------------------------------------------
    // CTR-CIR-003: canonical identity admission, inside the committing
    // transaction and before the first runtime-fact write. The command's
    // Agent Principal set is the resolved entry-visit assignee (including a
    // WORKFLOW_CREATOR resolution — it is an assignment fact) plus every
    // INSTANCE_INPUT_PRINCIPAL identity value persisted in the context
    // payload. Per-command identical IDs share one still-current observation;
    // any admission failure rolls the whole transaction back (zero business
    // delta — no partial write, no deterministic failure receipt, no retry).
    // ---------------------------------------------------------------
    let input_keys =
        admission_gate_module::required_input_principal_keys(&mut tx, definition_version_uuid)
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
    let input_principal_ids = admission_gate_module::extract_required_input_principal_ids(
        &input_keys,
        &cmd.context_payload,
    )
    .map_err(CreateWorkflowInstanceError::AssigneeResolutionFailed)?;
    let mut admission_principals = BTreeSet::from([resolved_assignee_id]);
    admission_principals.extend(input_principal_ids);
    // T72 (WF-GS-07): resolve lineage through the COMMITTING transaction —
    // never borrow a second pool connection while this transaction holds one
    // (self-starves a max_connections=1 deployment until acquire timeout).
    admission
        .admit_on_tx(&mut tx, admission_principals)
        .await
        .map_err(CreateWorkflowInstanceError::AdmissionFailed)?;

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
             external_reference, external_url, metadata,
             semantic_model_version, execution_class)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12::workflow_execution_class)
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
    .bind(version_info.semantic_model_version)
    .bind(cmd.execution_class.as_str())
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
    .bind(entry_node.node_id)
    .bind(visit_number)
    .bind(resolved_assignee_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // ---------------------------------------------------------------
    // Step 11b: Canonical activation for VISIT_ACTIVATION_V1 (same tx)
    // ---------------------------------------------------------------
    if version_info.semantic_model_version == 3 {
        // Kind derives only from the resolved canonical Principal type,
        // never from a caller field or node name.
        let owner_type: (String,) =
            sqlx::query_as("SELECT principal_type::text FROM principals WHERE principal_id = $1")
                .bind(resolved_assignee_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        let activation_kind = if owner_type.0 == "AGENT" {
            crate::domain::enums::ActivationKind::DispatchIntent
        } else {
            crate::domain::enums::ActivationKind::HumanWorkItem
        };
        activation_facts::insert_activation(
            &mut tx,
            activation_id,
            workflow_instance_id,
            node_visit_id,
            activation_kind,
            resolved_assignee_id,
            actual_command_id,
        )
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        // CTR-SWEC-007: push-first kick for a first DISPATCH_INTENT.
        if activation_kind == crate::domain::enums::ActivationKind::DispatchIntent
            && cmd.execution_class != WorkflowExecutionClass::NonBusinessTest
        {
            super::super::outbox::queue_execution_kick(
                &mut tx,
                workflow_instance_id,
                node_visit_id,
                activation_id,
            )
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        }
    }

    // ---------------------------------------------------------------
    // Step 12: Insert INSTANCE_CREATED WorkflowEvent #1
    // ---------------------------------------------------------------
    let event_sequence = 1i32;
    let definition_digest_str = version_info.definition_digest.as_deref().unwrap_or("");

    let event_data = InstanceCreatedEventData {
        definition_version_id: definition_version_uuid.to_string(),
        definition_digest: definition_digest_str.to_string(),
        initial_node_id: entry_node.node_id.to_string(),
        assignee_resolution_type: entry_node.assignee_ref_type.to_string(),
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
    // Step 12b: WORKFLOW_EXECUTION_CONTROL_V1 outbox facts
    // (CTR-SWEC-001/002). BUSINESS instances get the PENDING canonical
    // forum binding + the workflow_created forum event. Non-BUSINESS
    // (canary) instances get nothing — no forum surface for canaries.
    // ---------------------------------------------------------------
    if cmd.execution_class != WorkflowExecutionClass::NonBusinessTest {
        super::super::outbox::queue_forum_binding(&mut tx, workflow_instance_id)
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        super::super::outbox::queue_forum_event(
            &mut tx,
            workflow_instance_id,
            &format!("wf_created:{workflow_instance_id}"),
            serde_json::json!({
                "eventType": "workflow_created",
                "workflowInstanceId": workflow_instance_id,
                "definitionVersionId": definition_version_uuid,
                "initialNodeId": entry_node.node_id,
            }),
        )
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
    }

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
        workflow_instance_id,
        workflow_state_version,
        current_context_revision_id: context_revision_id,
        current_node_visit_id: node_visit_id,
        event_sequence,
    }))
}
