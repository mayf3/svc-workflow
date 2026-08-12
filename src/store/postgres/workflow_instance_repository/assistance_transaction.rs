//! Atomic Workflow Assistance V1 writes and caller-scoped reads.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::workflow_instance::assistance::{
    AssistanceCaseStatus, AssistanceCommandResult, AssistanceError, AssistancePayload,
    EscalateAssistanceCommand, RequestAssistanceCommand, ResolveAssistanceCommand,
};
use crate::domain::workflow_instance::events::{
    AssistanceEventData, ASSISTANCE_ESCALATED_TO_HUMAN_EVENT_TYPE, ASSISTANCE_REQUESTED_EVENT_TYPE,
    ASSISTANCE_RESOLVED_EVENT_TYPE, COMMAND_TYPE_ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN,
    COMMAND_TYPE_REQUEST_WORKFLOW_ASSISTANCE, COMMAND_TYPE_RESOLVE_WORKFLOW_ASSISTANCE,
    EVENT_SCHEMA_VERSION,
};

const MAX_PAYLOAD_BYTES: usize = 65_536;

fn storage(error: sqlx::Error) -> AssistanceError {
    AssistanceError::StorageError(error.to_string())
}

fn validate_payload(payload: &AssistancePayload) -> Result<serde_json::Value, AssistanceError> {
    let message = payload.message.as_str();
    if message.trim() != message || message.is_empty() || message.chars().count() > 2000 {
        return Err(AssistanceError::InvalidPayload(
            "message must be trimmed and contain 1-2000 characters".to_string(),
        ));
    }
    if message.chars().any(char::is_control) {
        return Err(AssistanceError::InvalidPayload(
            "message must not contain control characters".to_string(),
        ));
    }
    if let Some(value) = &payload.supporting_payload {
        if !value.is_object() {
            return Err(AssistanceError::InvalidPayload(
                "supportingPayload must be an object".to_string(),
            ));
        }
    }
    let value = serde_json::to_value(payload)
        .map_err(|error| AssistanceError::InternalConsistency(error.to_string()))?;
    let size = serde_json::to_vec(&value)
        .map_err(|error| AssistanceError::InternalConsistency(error.to_string()))?
        .len();
    if size > MAX_PAYLOAD_BYTES {
        return Err(AssistanceError::SizeLimitExceeded);
    }
    Ok(value)
}

enum ReceiptAcquire {
    Owned(Uuid),
    Replay(i32, serde_json::Value),
    Conflict,
    Processing,
}

async fn acquire_receipt(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    idempotency_key: &str,
    command_type: &str,
    request_hash: &str,
) -> Result<ReceiptAcquire, AssistanceError> {
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
        return Ok(ReceiptAcquire::Owned(command_id));
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
        return Ok(ReceiptAcquire::Conflict);
    }
    if status == "PROCESSING" {
        return Ok(ReceiptAcquire::Processing);
    }
    Ok(ReceiptAcquire::Replay(
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
    let affected = sqlx::query(
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
    .map_err(storage)?
    .rows_affected();
    if affected != 1 {
        return Err(AssistanceError::InternalConsistency(
            "receipt completion affected unexpected row count".to_string(),
        ));
    }
    Ok(())
}

fn error_body(error: &AssistanceError) -> serde_json::Value {
    match error {
        AssistanceError::WorkflowStateVersionConflict { expected, actual } => serde_json::json!({
            "error": error.code(), "expected": expected, "actual": actual
        }),
        AssistanceError::InvalidPayload(detail)
        | AssistanceError::InvalidPagination(detail)
        | AssistanceError::InternalConsistency(detail) => {
            serde_json::json!({"error": error.code(), "detail": detail})
        }
        _ => serde_json::json!({"error": error.code()}),
    }
}

fn replay_error(status: i32, body: &serde_json::Value) -> AssistanceError {
    let code = body
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    match code {
        "principal_disabled" => AssistanceError::PrincipalDisabled,
        "instance_not_found" => AssistanceError::InstanceNotFound,
        "current_visit_not_found" => AssistanceError::CurrentVisitNotFound,
        "current_node_visit_mismatch" => AssistanceError::CurrentNodeVisitMismatch,
        "principal_not_assignee" => AssistanceError::PrincipalNotAssignee,
        "source_node_terminal" => AssistanceError::SourceNodeTerminal,
        "instance_cancelled" => AssistanceError::InstanceCancelled,
        "instance_archived" => AssistanceError::InstanceArchived,
        "workflow_state_version_conflict" => AssistanceError::WorkflowStateVersionConflict {
            expected: body
                .get("expected")
                .and_then(|v| v.as_i64())
                .unwrap_or_default() as i32,
            actual: body
                .get("actual")
                .and_then(|v| v.as_i64())
                .unwrap_or_default() as i32,
        },
        "domain_owner_missing" => AssistanceError::DomainOwnerMissing,
        "not_domain_owner" => AssistanceError::NotDomainOwner,
        "assistance_already_open" => AssistanceError::AssistanceAlreadyOpen,
        "assistance_case_not_found_or_not_visible" => {
            AssistanceError::AssistanceCaseNotFoundOrNotVisible
        }
        "assistance_status_conflict" => AssistanceError::AssistanceStatusConflict,
        "invalid_assistance_payload" => AssistanceError::InvalidPayload(
            body.get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("invalid payload")
                .to_string(),
        ),
        "size_limit_exceeded" => AssistanceError::SizeLimitExceeded,
        _ => AssistanceError::InternalConsistency(format!(
            "unknown replayed assistance failure: status={status}, code={code}"
        )),
    }
}

async fn replay_or_ownership<'a>(
    tx: Transaction<'a, Postgres>,
    acquire: ReceiptAcquire,
) -> Result<(Transaction<'a, Postgres>, Uuid), Result<AssistanceCommandResult, AssistanceError>> {
    match acquire {
        ReceiptAcquire::Owned(command_id) => Ok((tx, command_id)),
        ReceiptAcquire::Replay(status, body) => {
            if status == 200 || status == 201 {
                let mut result = serde_json::from_value::<AssistanceCommandResult>(body)
                    .map_err(|error| AssistanceError::InternalConsistency(error.to_string()))
                    .map_err(Err)?;
                result.replayed = true;
                tx.commit().await.map_err(storage).map_err(Err)?;
                Err(Ok(result))
            } else {
                let error = replay_error(status, &body);
                tx.commit().await.map_err(storage).map_err(Err)?;
                Err(Err(error))
            }
        }
        ReceiptAcquire::Conflict => {
            tx.commit().await.map_err(storage).map_err(Err)?;
            Err(Err(AssistanceError::IdempotencyConflict))
        }
        ReceiptAcquire::Processing => {
            tx.commit().await.map_err(storage).map_err(Err)?;
            Err(Err(AssistanceError::CommandStillProcessing))
        }
    }
}

#[derive(Debug, FromRow)]
struct LockedInstance {
    workflow_instance_id: Uuid,
    domain_id: Uuid,
    current_context_revision_id: Option<Uuid>,
    current_node_visit_id: Option<Uuid>,
    workflow_state_version: i32,
    cancelled: bool,
    archived_at: Option<DateTime<Utc>>,
    node_id: Option<Uuid>,
    node_type: Option<String>,
    assignee_principal_id: Option<Uuid>,
}

async fn lock_instance(
    tx: &mut Transaction<'_, Postgres>,
    instance_id: Uuid,
) -> Result<LockedInstance, AssistanceError> {
    sqlx::query_as(
        "SELECT wi.workflow_instance_id, wi.domain_id,
                wi.current_context_revision_id, wi.current_node_visit_id,
                wi.workflow_state_version, wi.cancelled, wi.archived_at,
                nv.node_id, nd.node_type::text AS node_type, nv.assignee_principal_id
         FROM workflow_instances wi
         LEFT JOIN workflow_node_visits nv ON nv.node_visit_id = wi.current_node_visit_id
         LEFT JOIN workflow_node_definitions nd ON nd.node_id = nv.node_id
         WHERE wi.workflow_instance_id = $1
         FOR UPDATE OF wi",
    )
    .bind(instance_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    .ok_or(AssistanceError::InstanceNotFound)
}

async fn validate_actor_enabled(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
) -> Result<(), AssistanceError> {
    match sqlx::query_scalar::<_, bool>("SELECT enabled FROM principals WHERE principal_id = $1")
        .bind(actor)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
    {
        None => Err(AssistanceError::PrincipalNotFound),
        Some(false) => Err(AssistanceError::PrincipalDisabled),
        Some(true) => Ok(()),
    }
}

async fn effective_owner(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
) -> Result<Uuid, AssistanceError> {
    // `domains` is the owner-replacement serialization point. Keep this as a
    // separate statement: after waiting for a concurrent replacement, the
    // following binding query receives a fresh READ COMMITTED snapshot and
    // therefore observes either the complete old owner or the complete new
    // owner, never the transient EvalPlanQual gap between them.
    let domain_enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM domains WHERE domain_id=$1 FOR SHARE")
            .bind(domain_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?;
    if domain_enabled != Some(true) {
        return Err(AssistanceError::DomainOwnerMissing);
    }
    let owners: Vec<Uuid> = sqlx::query_scalar(
        "SELECT b.principal_id
         FROM domain_role_bindings b
         JOIN principals p ON p.principal_id = b.principal_id AND p.enabled = TRUE
         WHERE b.domain_id = $1 AND b.role_key = 'DOMAIN_OWNER' AND b.enabled = TRUE
         ORDER BY b.principal_id
         FOR SHARE OF b, p",
    )
    .bind(domain_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    if owners.len() != 1 {
        return Err(AssistanceError::DomainOwnerMissing);
    }
    Ok(owners[0])
}

async fn increment_instance_and_event(
    tx: &mut Transaction<'_, Postgres>,
    instance: &LockedInstance,
    command_id: Uuid,
    actor: Uuid,
    case_id: Uuid,
    event_type: &str,
    previous_status: Option<&str>,
    new_status: &str,
    payload_digest: &str,
) -> Result<i32, AssistanceError> {
    let old_version = instance.workflow_state_version;
    let new_version = old_version + 1;
    let affected = sqlx::query(
        "UPDATE workflow_instances
         SET workflow_state_version = $2, updated_at = now()
         WHERE workflow_instance_id = $1 AND workflow_state_version = $3",
    )
    .bind(instance.workflow_instance_id)
    .bind(new_version)
    .bind(old_version)
    .execute(&mut **tx)
    .await
    .map_err(storage)?
    .rows_affected();
    if affected != 1 {
        return Err(AssistanceError::InternalConsistency(
            "assistance state-version update affected unexpected row count".to_string(),
        ));
    }
    let current_visit = instance
        .current_node_visit_id
        .ok_or(AssistanceError::CurrentVisitNotFound)?;
    let event_data = serde_json::to_value(AssistanceEventData {
        assistance_case_id: case_id.to_string(),
        previous_status: previous_status.map(ToOwned::to_owned),
        new_status: new_status.to_string(),
        payload_digest: payload_digest.to_string(),
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
    .bind(instance.workflow_instance_id)
    .bind(new_version)
    .bind(EVENT_SCHEMA_VERSION)
    .bind(command_id)
    .bind(event_type)
    .bind(current_visit)
    .bind(instance.current_context_revision_id)
    .bind(event_data)
    .bind(event_digest)
    .bind(actor)
    .bind(instance.node_id)
    .bind(old_version)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(new_version)
}

async fn deterministic_failure(
    mut tx: Transaction<'_, Postgres>,
    command_id: Uuid,
    error: AssistanceError,
) -> Result<AssistanceCommandResult, AssistanceError> {
    complete_receipt(
        &mut tx,
        command_id,
        error.status_code(),
        &error_body(&error),
    )
    .await?;
    tx.commit().await.map_err(storage)?;
    Err(error)
}

pub(crate) async fn has_open_assistance(
    tx: &mut Transaction<'_, Postgres>,
    node_visit_id: Uuid,
) -> Result<bool, AssistanceError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflow_assistance_cases
         WHERE node_visit_id=$1 AND status IN ('OWNER_PENDING','HUMAN_REQUIRED'))",
    )
    .bind(node_visit_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)
}

pub(crate) async fn void_open_cases(
    tx: &mut Transaction<'_, Postgres>,
    instance_id: Uuid,
    actor: Uuid,
    command_id: Uuid,
    reason: &str,
) -> Result<u64, AssistanceError> {
    sqlx::query(
        "UPDATE workflow_assistance_cases
         SET status='VOIDED', voided_by_principal_id=$2, void_reason_code=$4,
             voided_by_command_id=$3, voided_at=now(), updated_at=now()
         WHERE workflow_instance_id=$1 AND status IN ('OWNER_PENDING','HUMAN_REQUIRED')",
    )
    .bind(instance_id)
    .bind(actor)
    .bind(command_id)
    .bind(reason)
    .execute(&mut **tx)
    .await
    .map_err(storage)
    .map(|result| result.rows_affected())
}

mod query;
mod write;

pub(crate) use query::{
    get_assistance_case, get_human_required_assistance_case, list_assistance,
    list_human_required_assistance,
};
pub use query::{
    AssistanceCaseView, AssistanceCursor, AssistanceListView, AssistanceNodeSummary,
    AssistancePage, HumanRequiredAssistanceCaseView, HumanRequiredAssistancePage,
};
pub(crate) use write::{escalate_assistance, request_assistance, resolve_assistance};
