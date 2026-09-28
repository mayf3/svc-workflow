//! WORKFLOW_EXECUTION_CONTROL_V1 owner-attention policy primitive.
//!
//! One shared transaction-scoped helper opens an OWNER_PENDING assistance
//! case in the CALLER's transaction (system attempt-limit ingress and the
//! RETURN-policy threshold). Domain Owner then uses the EXISTING assistance
//! escalation/resolve commands to either solve the case or explicitly move it
//! to HUMAN_REQUIRED. Open-case fail-close and version-bump behavior remain.

use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::workflow_instance::assistance::AssistanceError;
use crate::domain::workflow_instance::events::{
    AssistanceEventData, ASSISTANCE_REQUESTED_EVENT_TYPE, EVENT_SCHEMA_VERSION,
};

pub(crate) const MAX_ESCALATION_MESSAGE_CHARS: usize = 2000;

fn storage(error: sqlx::Error) -> AssistanceError {
    AssistanceError::StorageError(error.to_string())
}

/// 64-hex per the receipts table CHECK; deterministic per input.
fn hex_digest(material: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(material.as_bytes());
    hex::encode(hasher.finalize())
}

/// The outcome of one policy escalation for the caller's response/eventing.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct PolicyEscalationOutcome {
    pub assistance_case_id: Uuid,
    pub owner_principal_id: Uuid,
    /// false ⇒ an open case already existed (idempotent replay).
    pub escalated: bool,
    pub workflow_state_version: i32,
    pub event_sequence: i32,
    /// The command id the escalation event was written under.
    pub receipt_command_id: Uuid,
}

/// Open ONE OWNER_PENDING assistance case for `node_visit_id` inside the
/// caller's transaction. Idempotent: an existing open case on the visit is
/// returned with `escalated = false` and NO new rows or version bump.
///
/// Caller contract: `node_visit_id` MUST be the instance's CURRENT visit and
/// the instance must be unlocked-in-tx (this helper takes its own
/// `FOR UPDATE` lock — re-locking the same row in the same transaction is
/// safe). `actor` is the command principal (poller principal for the system
/// ingress, transitioning principal for the RETURN policy).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn open_owner_pending_visit_tx(
    tx: &mut Transaction<'_, Postgres>,
    workflow_instance_id: Uuid,
    node_visit_id: Uuid,
    actor: Uuid,
    command_id: Uuid,
    message: &str,
    supporting_payload: serde_json::Value,
) -> Result<PolicyEscalationOutcome, AssistanceError> {
    if message.trim() != message
        || message.is_empty()
        || message.chars().count() > MAX_ESCALATION_MESSAGE_CHARS
    {
        return Err(AssistanceError::InvalidPayload(
            "escalation message must be trimmed and contain 1-2000 characters".to_string(),
        ));
    }
    let request_payload = serde_json::json!({
        "message": message,
        "supportingPayload": supporting_payload,
    });
    let payload_digest = digest::compute_json_digest(&request_payload)
        .map_err(AssistanceError::InternalConsistency)?;

    // Lock the instance row (same discipline as the assistance commands).
    let instance: Option<(
        Uuid,
        Uuid,
        Option<Uuid>,
        i32,
        bool,
        Option<DateTime<Utc>>,
        Option<String>,
        Option<Uuid>,
        Option<Uuid>,
    )> = sqlx::query_as(
        "SELECT wi.workflow_instance_id, wi.domain_id, wi.current_node_visit_id, wi.workflow_state_version,
                wi.cancelled, wi.archived_at, nd.node_type::text AS node_type,
                wi.current_context_revision_id, nv.node_id
           FROM workflow_instances wi
           LEFT JOIN workflow_node_visits nv ON nv.node_visit_id = wi.current_node_visit_id
           LEFT JOIN workflow_node_definitions nd ON nd.node_id = nv.node_id
          WHERE wi.workflow_instance_id = $1
          FOR UPDATE OF wi",
    )
    .bind(workflow_instance_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    let (
        _,
        domain_id,
        current_visit,
        version,
        cancelled,
        archived_at,
        node_type,
        context_revision_id,
        node_id,
    ) = instance.ok_or(AssistanceError::AssistanceCaseNotFoundOrNotVisible)?;
    if current_visit.is_none() {
        return Err(AssistanceError::CurrentVisitNotFound);
    }
    if current_visit != Some(node_visit_id) {
        return Err(AssistanceError::CurrentNodeVisitMismatch);
    }
    if cancelled {
        return Err(AssistanceError::InstanceCancelled);
    }
    if archived_at.is_some() {
        return Err(AssistanceError::InstanceArchived);
    }
    if node_type.as_deref() == Some("TERMINAL") {
        return Err(AssistanceError::SourceNodeTerminal);
    }

    // OWNER_PENDING must always have a live authority to handle it. Resolve
    // the current enabled Domain Owner before opening/replaying the case;
    // never create an owner-attention dead end.
    let owner_principal_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT b.principal_id
           FROM domain_role_bindings b
           JOIN domains d ON d.domain_id=b.domain_id AND d.enabled=TRUE
           JOIN principals p ON p.principal_id=b.principal_id AND p.enabled=TRUE
          WHERE b.domain_id=$1 AND b.role_key='DOMAIN_OWNER' AND b.enabled=TRUE",
    )
    .bind(domain_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    let owner_principal_id = owner_principal_id.ok_or(AssistanceError::DomainOwnerMissing)?;

    // Idempotency: an open case on this visit replays as escalated=false.
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT assistance_case_id FROM workflow_assistance_cases
          WHERE node_visit_id = $1 AND status IN ('OWNER_PENDING','HUMAN_REQUIRED')",
    )
    .bind(node_visit_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    if let Some(case_id) = existing {
        return Ok(PolicyEscalationOutcome {
            assistance_case_id: case_id,
            owner_principal_id,
            escalated: false,
            workflow_state_version: version,
            event_sequence: version,
            receipt_command_id: command_id,
        });
    }

    // 0022 governance (fn_validate_assistance_command_refs): the request
    // command ref must point at a COMPLETED receipt of the exact type held by
    // THIS actor. The policy request therefore mints and completes one
    // synthetic assistance-request receipt inside the caller's transaction;
    // the outer receipt stays the caller's idempotency anchor.
    let body_json = serde_json::json!({ "source": "execution_policy" });
    let receipt_digest =
        digest::compute_json_digest(&body_json).map_err(AssistanceError::InternalConsistency)?;
    let request_receipt_id = Uuid::new_v4();
    for (receipt_id, command_type, idem_suffix) in [
        (request_receipt_id, "REQUEST_WORKFLOW_ASSISTANCE", "req"),
    ] {
        sqlx::query(
            "INSERT INTO workflow_command_receipts
                 (command_id, principal_id, idempotency_key, command_type, request_hash, receipt_status)
             VALUES ($1, $2, $3, $4, $5, 'PROCESSING')",
        )
        .bind(receipt_id)
        .bind(actor)
        .bind(format!("policy-{idem_suffix}:{command_id}"))
        .bind(command_type)
        .bind(hex_digest(&format!("policy-{idem_suffix}:{command_id}")))
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
        sqlx::query(
            "UPDATE workflow_command_receipts
                SET receipt_status = 'COMPLETED', response_status = 200,
                    response_body = $2, response_digest = $3, completed_at = now()
              WHERE command_id = $1",
        )
        .bind(receipt_id)
        .bind(&body_json)
        .bind(&receipt_digest)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    }

    // OWNER_PENDING is the terminal state of the system policy action.
    // Domain Owner decides the next step through the normal assistance API.
    let case_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_assistance_cases
             (assistance_case_id, workflow_instance_id, node_visit_id, status,
              requested_by_principal_id, request_payload, request_payload_digest, request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(case_id)
    .bind(workflow_instance_id)
    .bind(node_visit_id)
    .bind(actor)
    .bind(&request_payload)
    .bind(&payload_digest)
    .bind(request_receipt_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| {
        if error.as_database_error().and_then(|e| e.constraint())
            == Some("uq_assistance_one_open_per_visit")
        {
            AssistanceError::AssistanceAlreadyOpen
        } else {
            storage(error)
        }
    })?;

    // Version bump + event — same reader contract as the manual escalation.
    let new_version = version + 1;
    let affected = sqlx::query(
        "UPDATE workflow_instances
            SET workflow_state_version = $2, updated_at = now()
          WHERE workflow_instance_id = $1 AND workflow_state_version = $3",
    )
    .bind(workflow_instance_id)
    .bind(new_version)
    .bind(version)
    .execute(&mut **tx)
    .await
    .map_err(storage)?
    .rows_affected();
    if affected != 1 {
        return Err(AssistanceError::InternalConsistency(
            "policy escalation state-version update affected unexpected row count".to_string(),
        ));
    }

    let event_data = serde_json::to_value(AssistanceEventData {
        assistance_case_id: case_id.to_string(),
        previous_status: None,
        new_status: "OWNER_PENDING".to_string(),
        payload_digest: payload_digest.clone(),
    })
    .map_err(|error| AssistanceError::InternalConsistency(error.to_string()))?;
    let event_digest =
        digest::compute_json_digest(&event_data).map_err(AssistanceError::InternalConsistency)?;
    sqlx::query(
        "INSERT INTO workflow_events
             (event_id, workflow_instance_id, event_sequence, event_schema_version,
              command_id, event_type, transition_effect,
              source_node_visit_id, target_node_visit_id,
              context_revision_id, submission_id, event_data, event_data_digest,
              actor_principal_id, from_node_id, to_node_id,
              old_workflow_state_version, new_workflow_state_version)
         VALUES ($1,$2,$3,$4,$5,$6,NULL::transition_effect,$7,$7,$8,NULL,$9,$10,$11,$12,$12,$13,$3)",
    )
    .bind(Uuid::new_v4())
    .bind(workflow_instance_id)
    .bind(new_version)
    .bind(EVENT_SCHEMA_VERSION)
    // The policy-created request has its own synthetic request receipt.
    .bind(request_receipt_id)
    .bind(ASSISTANCE_REQUESTED_EVENT_TYPE)
    .bind(node_visit_id)
    .bind(context_revision_id)
    .bind(&event_data)
    .bind(&event_digest)
    .bind(actor)
    .bind(node_id)
    .bind(version)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;

    let wake_reason = request_payload
        .get("supportingPayload")
        .and_then(|value| value.get("reason"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("OWNER_ATTENTION_REQUIRED");
    super::super::outbox::queue_owner_assistance_wake(
        tx,
        workflow_instance_id,
        node_visit_id,
        case_id,
        owner_principal_id,
        wake_reason,
    )
    .await
    .map_err(storage)?;

    Ok(PolicyEscalationOutcome {
        assistance_case_id: case_id,
        owner_principal_id,
        escalated: true,
        workflow_state_version: new_version,
        event_sequence: new_version,
        receipt_command_id: command_id,
    })
}

// ── CTR-SWEC-004: the system escalation ingress command ───────────────────

/// Stable command type for the system execution-escalation ingress.
pub(crate) const COMMAND_TYPE_SYSTEM_EXECUTION_ESCALATION: &str = "SYSTEM_EXECUTION_ESCALATION";

/// Evidence-carrying escalation request (caller = GLOBAL_SCHEDULER_READ
/// principal, typically the execution-runtime poller).
#[derive(Debug, Clone)]
pub struct SystemEscalationCommand {
    pub principal_id: Uuid,
    pub idempotency_key: String,
    pub request_hash: String,
    pub workflow_instance_id: Uuid,
    pub node_visit_id: Uuid,
    /// `ATTEMPTS_EXHAUSTED` | `STALE_LOOP_EXHAUSTED`
    pub reason: String,
    pub attempt_count: Option<i64>,
    pub last_attempt_id: Option<Uuid>,
    pub dispatch_intent_id: Option<Uuid>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemEscalationOutcome {
    pub assistance_case_id: Uuid,
    /// None only when replaying a receipt written by a pre-owner-attention build.
    pub owner_principal_id: Option<Uuid>,
    pub escalated: bool,
    pub workflow_state_version: i32,
    pub event_sequence: i32,
}

enum Receipt {
    Owned(Uuid),
    Replay(i32, serde_json::Value),
    Conflict,
    Processing,
}

async fn acquire_receipt(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
) -> Result<Receipt, AssistanceError> {
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
    .bind(COMMAND_TYPE_SYSTEM_EXECUTION_ESCALATION)
    .bind(request_hash)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    if let Some(command_id) = inserted {
        return Ok(Receipt::Owned(command_id));
    }
    let row: Option<(String, String, Option<i32>, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT receipt_status::text, request_hash, response_status, response_body
         FROM workflow_command_receipts
         WHERE principal_id = $1 AND idempotency_key = $2
         FOR UPDATE",
    )
    .bind(principal_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    let (status, original_hash, response_status, response_body) = row.ok_or_else(|| {
        AssistanceError::InternalConsistency("receipt disappeared during acquire".to_string())
    })?;
    if original_hash != request_hash {
        return Ok(Receipt::Conflict);
    }
    if status == "PROCESSING" {
        return Ok(Receipt::Processing);
    }
    Ok(Receipt::Replay(
        response_status.ok_or_else(|| {
            AssistanceError::InternalConsistency("completed receipt has no status".to_string())
        })?,
        response_body.unwrap_or(serde_json::Value::Null),
    ))
}

async fn complete_receipt(
    tx: &mut Transaction<'_, Postgres>,
    command_id: Uuid,
    status: i32,
    body: &serde_json::Value,
) -> Result<(), AssistanceError> {
    let response_digest =
        digest::compute_json_digest(body).map_err(AssistanceError::InternalConsistency)?;
    sqlx::query(
        "UPDATE workflow_command_receipts
         SET receipt_status = 'COMPLETED', response_status = $2,
             response_body = $3, response_digest = $4, completed_at = now()
         WHERE command_id = $1 AND receipt_status = 'PROCESSING'",
    )
    .bind(command_id)
    .bind(status)
    .bind(body)
    .bind(response_digest)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}

/// Execute the system escalation ingress: validate, open OWNER_PENDING on the
/// CURRENT visit (idempotent on an open case), queue the forum projection row,
/// complete the receipt. One transaction.
pub(crate) async fn system_execution_escalation(
    pool: &sqlx::PgPool,
    command: SystemEscalationCommand,
) -> Result<SystemEscalationOutcome, AssistanceError> {
    if command.reason != "ATTEMPTS_EXHAUSTED" && command.reason != "STALE_LOOP_EXHAUSTED" {
        return Err(AssistanceError::InvalidPayload(
            "reason must be ATTEMPTS_EXHAUSTED or STALE_LOOP_EXHAUSTED".to_string(),
        ));
    }
    let mut tx = pool.begin().await.map_err(storage)?;

    match acquire_receipt(
        &mut tx,
        command.principal_id,
        &command.idempotency_key,
        &command.request_hash,
    )
    .await?
    {
        Receipt::Owned(command_id) => {
            let outcome = escalate_and_project(&mut tx, command_id, &command).await?;
            let body = serde_json::to_value(&outcome)
                .map_err(|e| AssistanceError::InternalConsistency(e.to_string()))?;
            complete_receipt(&mut tx, command_id, 200, &body).await?;
            tx.commit().await.map_err(storage)?;
            Ok(outcome)
        }
        Receipt::Replay(_status, body) => {
            let mut outcome: SystemEscalationOutcome = serde_json::from_value(body).map_err(|e| {
                AssistanceError::InternalConsistency(format!(
                    "escalation replay body mismatch: {e}"
                ))
            })?;
            // Pre-owner-attention receipts do not carry ownerPrincipalId. Enrich
            // the replay from current authoritative Domain bindings without
            // mutating the historical receipt.
            if outcome.owner_principal_id.is_none() {
                outcome.owner_principal_id = sqlx::query_scalar(
                    "SELECT b.principal_id
                       FROM workflow_assistance_cases ac
                       JOIN workflow_instances wi ON wi.workflow_instance_id=ac.workflow_instance_id
                       JOIN domain_role_bindings b ON b.domain_id=wi.domain_id
                       JOIN domains d ON d.domain_id=b.domain_id AND d.enabled=TRUE
                       JOIN principals p ON p.principal_id=b.principal_id AND p.enabled=TRUE
                      WHERE ac.assistance_case_id=$1
                        AND b.role_key='DOMAIN_OWNER' AND b.enabled=TRUE",
                )
                .bind(outcome.assistance_case_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(outcome)
        }
        Receipt::Conflict => {
            tx.commit().await.map_err(storage)?;
            Err(AssistanceError::IdempotencyConflict)
        }
        Receipt::Processing => {
            tx.commit().await.map_err(storage)?;
            Err(AssistanceError::CommandStillProcessing)
        }
    }
}

async fn escalate_and_project(
    tx: &mut Transaction<'_, Postgres>,
    command_id: Uuid,
    command: &SystemEscalationCommand,
) -> Result<SystemEscalationOutcome, AssistanceError> {
    let message = format!(
        "Execution policy assistance ({}) on the current node visit — Domain Owner attention required",
        command.reason
    );
    let supporting = serde_json::json!({
        "source": "execution_policy",
        "reason": command.reason,
        "attemptCount": command.attempt_count,
        "lastAttemptId": command.last_attempt_id,
        "dispatchIntentId": command.dispatch_intent_id,
    });
    let outcome = open_owner_pending_visit_tx(
        tx,
        command.workflow_instance_id,
        command.node_visit_id,
        command.principal_id,
        command_id,
        &message,
        supporting,
    )
    .await?;

    // Forum projection row for the owner-attention request.
    super::super::outbox::queue_forum_event(
        tx,
        command.workflow_instance_id,
        &format!("assistance_requested:{}", outcome.assistance_case_id),
        serde_json::json!({
            "eventType": "owner_attention_requested",
            "workflowInstanceId": command.workflow_instance_id,
            "nodeVisitId": command.node_visit_id,
            "assistanceCaseId": outcome.assistance_case_id,
            "ownerPrincipalId": outcome.owner_principal_id,
            "reason": command.reason,
            "attemptCount": command.attempt_count,
        }),
    )
    .await
    .map_err(storage)?;

    Ok(SystemEscalationOutcome {
        assistance_case_id: outcome.assistance_case_id,
        owner_principal_id: Some(outcome.owner_principal_id),
        escalated: outcome.escalated,
        workflow_state_version: outcome.workflow_state_version,
        event_sequence: outcome.event_sequence,
    })
}
