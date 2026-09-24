//! WORKFLOW_EXECUTION_CONTROL_V1 (CTR-SWEC-004/005) — the REQUIRE_HUMAN
//! escalation primitive.
//!
//! One shared transaction-scoped helper creates + escalates an assistance
//! case to HUMAN_REQUIRED in the CALLER's transaction (system escalation
//! ingress for the execution runtime, and the RETURN-policy threshold inside
//! the transition transaction). It rides the EXISTING assistance machinery —
//! same table, same status CHECKs, same event type — so every reader surface
//! (fail-close on open cases, human-required inbox, version bump ⇒ the
//! execution engine's stale probe sees `progressed`) works unchanged.

use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::workflow_instance::assistance::AssistanceError;
use crate::domain::workflow_instance::events::{
    AssistanceEventData, ASSISTANCE_ESCALATED_TO_HUMAN_EVENT_TYPE, EVENT_SCHEMA_VERSION,
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
    /// false ⇒ an open case already existed (idempotent replay).
    pub escalated: bool,
    pub workflow_state_version: i32,
    pub event_sequence: i32,
    /// The command id the escalation event was written under.
    pub receipt_command_id: Uuid,
}

/// Create + escalate ONE assistance case for `node_visit_id` to
/// HUMAN_REQUIRED inside the caller's transaction. Idempotent: an existing
/// open case on the visit is returned with `escalated = false` and NO new
/// rows or version bump.
///
/// Caller contract: `node_visit_id` MUST be the instance's CURRENT visit and
/// the instance must be unlocked-in-tx (this helper takes its own
/// `FOR UPDATE` lock — re-locking the same row in the same transaction is
/// safe). `actor` is the command principal (poller principal for the system
/// ingress, transitioning principal for the RETURN policy).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn escalate_visit_tx(
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
        Option<Uuid>,
        i32,
        bool,
        Option<DateTime<Utc>>,
        Option<String>,
        Option<Uuid>,
        Option<Uuid>,
    )> = sqlx::query_as(
        "SELECT wi.workflow_instance_id, wi.current_node_visit_id, wi.workflow_state_version,
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
            escalated: false,
            workflow_state_version: version,
            event_sequence: version,
            receipt_command_id: command_id,
        });
    }

    // 0022 governance (fn_validate_assistance_command_refs): the case's
    // request/escalation command refs must point at COMPLETED receipts of the
    // exact types held by THIS actor. The policy escalation therefore mints
    // and completes BOTH synthetic receipts inside the caller's transaction —
    // the outer receipt stays the caller's idempotency anchor.
    let body_json = serde_json::json!({ "source": "execution_policy" });
    let receipt_digest =
        digest::compute_json_digest(&body_json).map_err(AssistanceError::InternalConsistency)?;
    let request_receipt_id = Uuid::new_v4();
    let escalation_receipt_id = Uuid::new_v4();
    for (receipt_id, command_type, idem_suffix) in [
        (request_receipt_id, "REQUEST_WORKFLOW_ASSISTANCE", "req"),
        (
            escalation_receipt_id,
            "ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN",
            "esc",
        ),
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

    // OWNER_PENDING insert, then the SAME policy escalates it to
    // HUMAN_REQUIRED (satisfies the 0021 CHECK constraints: every escalation
    // field set, no resolution fields).
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

    sqlx::query(
        "UPDATE workflow_assistance_cases
            SET status='HUMAN_REQUIRED', escalated_by_principal_id=$2,
                escalation_payload=$3, escalation_payload_digest=$4,
                escalation_command_id=$5, escalated_at=now(), updated_at=now()
          WHERE assistance_case_id=$1 AND status='OWNER_PENDING'",
    )
    .bind(case_id)
    .bind(actor)
    .bind(&request_payload)
    .bind(&payload_digest)
    .bind(escalation_receipt_id)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;

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
        previous_status: Some("OWNER_PENDING".to_string()),
        new_status: "HUMAN_REQUIRED".to_string(),
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
    // The event rides the ESCALATION-stage receipt: workflow_events allows
    // at most one event per command (0006) and the transition command already
    // wrote its own event.
    .bind(escalation_receipt_id)
    .bind(ASSISTANCE_ESCALATED_TO_HUMAN_EVENT_TYPE)
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

    Ok(PolicyEscalationOutcome {
        assistance_case_id: case_id,
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

/// Execute the system escalation ingress: validate, escalate the CURRENT
/// visit to HUMAN_REQUIRED (idempotent on an open case), queue the forum
/// projection row, complete the receipt. One transaction.
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
            tx.commit().await.map_err(storage)?;
            let outcome: SystemEscalationOutcome = serde_json::from_value(body).map_err(|e| {
                AssistanceError::InternalConsistency(format!(
                    "escalation replay body mismatch: {e}"
                ))
            })?;
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
        "Execution policy escalation ({}) on the current node visit — a human must take over",
        command.reason
    );
    let supporting = serde_json::json!({
        "source": "execution_policy",
        "reason": command.reason,
        "attemptCount": command.attempt_count,
        "lastAttemptId": command.last_attempt_id,
        "dispatchIntentId": command.dispatch_intent_id,
    });
    let outcome = escalate_visit_tx(
        tx,
        command.workflow_instance_id,
        command.node_visit_id,
        command.principal_id,
        command_id,
        &message,
        supporting,
    )
    .await?;

    // Forum projection row for the escalation (Goal Scope G).
    super::super::outbox::queue_forum_event(
        tx,
        command.workflow_instance_id,
        &format!("assistance_escalated:{}", outcome.assistance_case_id),
        serde_json::json!({
            "eventType": "attempts_exhausted",
            "workflowInstanceId": command.workflow_instance_id,
            "nodeVisitId": command.node_visit_id,
            "assistanceCaseId": outcome.assistance_case_id,
            "reason": command.reason,
            "attemptCount": command.attempt_count,
        }),
    )
    .await
    .map_err(storage)?;

    Ok(SystemEscalationOutcome {
        assistance_case_id: outcome.assistance_case_id,
        escalated: outcome.escalated,
        workflow_state_version: outcome.workflow_state_version,
        event_sequence: outcome.event_sequence,
    })
}
