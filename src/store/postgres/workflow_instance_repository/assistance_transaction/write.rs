//! Assistance request, escalation, and resolution transactions.

use super::*;

pub(crate) async fn request_assistance(
    pool: &PgPool,
    command: RequestAssistanceCommand,
    request_hash: &str,
) -> Result<AssistanceCommandResult, AssistanceError> {
    let actor = command.principal_id.into_uuid();
    let instance_id = command.workflow_instance_id.into_uuid();
    let requested_visit = command.current_node_visit_id.into_uuid();
    let mut tx = pool.begin().await.map_err(storage)?;
    let acquired = acquire_receipt(
        &mut tx,
        actor,
        &command.idempotency_key,
        COMMAND_TYPE_REQUEST_WORKFLOW_ASSISTANCE,
        request_hash,
    )
    .await?;
    let (mut tx, command_id) = match replay_or_ownership(tx, acquired).await {
        Ok(owned) => owned,
        Err(result) => return result,
    };
    let payload = match validate_payload(&command.request) {
        Ok(payload) => payload,
        Err(error) => return deterministic_failure(tx, command_id, error).await,
    };
    let payload_digest =
        digest::compute_json_digest(&payload).map_err(AssistanceError::InternalConsistency)?;

    let instance = match lock_instance(&mut tx, instance_id).await {
        Ok(row) => row,
        Err(error) => return deterministic_failure(tx, command_id, error).await,
    };
    if command.expected_workflow_state_version != instance.workflow_state_version {
        return deterministic_failure(
            tx,
            command_id,
            AssistanceError::WorkflowStateVersionConflict {
                expected: command.expected_workflow_state_version,
                actual: instance.workflow_state_version,
            },
        )
        .await;
    }
    if instance.current_node_visit_id != Some(requested_visit) {
        return deterministic_failure(tx, command_id, AssistanceError::CurrentNodeVisitMismatch)
            .await;
    }
    if instance.cancelled {
        return deterministic_failure(tx, command_id, AssistanceError::InstanceCancelled).await;
    }
    if instance.archived_at.is_some() {
        return deterministic_failure(tx, command_id, AssistanceError::InstanceArchived).await;
    }
    if instance.node_type.as_deref() == Some("TERMINAL") {
        return deterministic_failure(tx, command_id, AssistanceError::SourceNodeTerminal).await;
    }
    if let Err(error) = validate_actor_enabled(&mut tx, actor).await {
        return deterministic_failure(tx, command_id, error).await;
    }
    if instance.assignee_principal_id != Some(actor) {
        return deterministic_failure(tx, command_id, AssistanceError::PrincipalNotAssignee).await;
    }
    if let Err(error) = effective_owner(&mut tx, instance.domain_id).await {
        return deterministic_failure(tx, command_id, error).await;
    }
    let open_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflow_assistance_cases
         WHERE node_visit_id = $1 AND status IN ('OWNER_PENDING','HUMAN_REQUIRED'))",
    )
    .bind(requested_visit)
    .fetch_one(&mut *tx)
    .await
    .map_err(storage)?;
    if open_exists {
        return deterministic_failure(tx, command_id, AssistanceError::AssistanceAlreadyOpen).await;
    }

    let case_id = Uuid::new_v4();
    let created_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)
         RETURNING created_at",
    )
    .bind(case_id)
    .bind(instance_id)
    .bind(requested_visit)
    .bind(actor)
    .bind(&payload)
    .bind(&payload_digest)
    .bind(command_id)
    .fetch_one(&mut *tx)
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
    let new_version = increment_instance_and_event(
        &mut tx,
        &instance,
        command_id,
        actor,
        case_id,
        ASSISTANCE_REQUESTED_EVENT_TYPE,
        None,
        "OWNER_PENDING",
        &payload_digest,
    )
    .await?;
    let result = AssistanceCommandResult {
        assistance_case_id: case_id,
        workflow_instance_id: instance_id,
        node_visit_id: requested_visit,
        status: AssistanceCaseStatus::OwnerPending,
        workflow_state_version: new_version,
        event_sequence: new_version,
        created_at,
        replayed: false,
    };
    let body = serde_json::to_value(&result)
        .map_err(|error| AssistanceError::InternalConsistency(error.to_string()))?;
    complete_receipt(&mut tx, command_id, 201, &body).await?;
    tx.commit().await.map_err(storage)?;
    Ok(result)
}

#[derive(Debug, FromRow)]
struct LockedCase {
    assistance_case_id: Uuid,
    workflow_instance_id: Uuid,
    node_visit_id: Uuid,
    status: String,
    created_at: DateTime<Utc>,
}

enum OwnerAction {
    Escalate(AssistancePayload),
    Resolve(AssistancePayload),
}

async fn owner_action(
    pool: &PgPool,
    actor: Uuid,
    idempotency_key: &str,
    case_id: Uuid,
    expected_version: i32,
    action: OwnerAction,
    request_hash: &str,
) -> Result<AssistanceCommandResult, AssistanceError> {
    let (command_type, event_type, payload_ref, target_status) = match &action {
        OwnerAction::Escalate(payload) => (
            COMMAND_TYPE_ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN,
            ASSISTANCE_ESCALATED_TO_HUMAN_EVENT_TYPE,
            payload,
            AssistanceCaseStatus::HumanRequired,
        ),
        OwnerAction::Resolve(payload) => (
            COMMAND_TYPE_RESOLVE_WORKFLOW_ASSISTANCE,
            ASSISTANCE_RESOLVED_EVENT_TYPE,
            payload,
            AssistanceCaseStatus::Resolved,
        ),
    };
    let mut tx = pool.begin().await.map_err(storage)?;
    let acquired =
        acquire_receipt(&mut tx, actor, idempotency_key, command_type, request_hash).await?;
    let (mut tx, command_id) = match replay_or_ownership(tx, acquired).await {
        Ok(owned) => owned,
        Err(result) => return result,
    };
    let payload = match validate_payload(payload_ref) {
        Ok(payload) => payload,
        Err(error) => return deterministic_failure(tx, command_id, error).await,
    };
    let payload_digest =
        digest::compute_json_digest(&payload).map_err(AssistanceError::InternalConsistency)?;
    let instance_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT workflow_instance_id FROM workflow_assistance_cases WHERE assistance_case_id = $1",
    )
    .bind(case_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage)?;
    let instance_id = match instance_id {
        Some(instance_id) => instance_id,
        None => {
            return deterministic_failure(
                tx,
                command_id,
                AssistanceError::AssistanceCaseNotFoundOrNotVisible,
            )
            .await
        }
    };
    let instance = match lock_instance(&mut tx, instance_id).await {
        Ok(row) => row,
        Err(error) => return deterministic_failure(tx, command_id, error).await,
    };
    if expected_version != instance.workflow_state_version {
        return deterministic_failure(
            tx,
            command_id,
            AssistanceError::WorkflowStateVersionConflict {
                expected: expected_version,
                actual: instance.workflow_state_version,
            },
        )
        .await;
    }
    if let Err(error) = validate_actor_enabled(&mut tx, actor).await {
        return deterministic_failure(tx, command_id, error).await;
    }
    let owner = match effective_owner(&mut tx, instance.domain_id).await {
        Ok(owner) => owner,
        Err(error) => return deterministic_failure(tx, command_id, error).await,
    };
    if owner != actor {
        return deterministic_failure(tx, command_id, AssistanceError::NotDomainOwner).await;
    }
    let case: LockedCase = match sqlx::query_as(
        "SELECT assistance_case_id, workflow_instance_id, node_visit_id, status, created_at
         FROM workflow_assistance_cases WHERE assistance_case_id = $1 FOR UPDATE",
    )
    .bind(case_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage)?
    {
        Some(case) => case,
        None => {
            return deterministic_failure(
                tx,
                command_id,
                AssistanceError::AssistanceCaseNotFoundOrNotVisible,
            )
            .await
        }
    };
    if case.workflow_instance_id != instance_id
        || instance.current_node_visit_id != Some(case.node_visit_id)
    {
        return deterministic_failure(tx, command_id, AssistanceError::AssistanceStatusConflict)
            .await;
    }
    let previous_status = case.status.clone();
    let status_ok = match action {
        OwnerAction::Escalate(_) => previous_status == "OWNER_PENDING",
        OwnerAction::Resolve(_) => {
            previous_status == "OWNER_PENDING" || previous_status == "HUMAN_REQUIRED"
        }
    };
    if !status_ok {
        return deterministic_failure(tx, command_id, AssistanceError::AssistanceStatusConflict)
            .await;
    }

    let affected = match action {
        OwnerAction::Escalate(_) => sqlx::query(
            "UPDATE workflow_assistance_cases
             SET status='HUMAN_REQUIRED', escalated_by_principal_id=$2,
                 escalation_payload=$3, escalation_payload_digest=$4,
                 escalation_command_id=$5, escalated_at=now(), updated_at=now()
             WHERE assistance_case_id=$1 AND status='OWNER_PENDING'",
        )
        .bind(case_id)
        .bind(actor)
        .bind(&payload)
        .bind(&payload_digest)
        .bind(command_id)
        .execute(&mut *tx)
        .await
        .map_err(storage)?
        .rows_affected(),
        OwnerAction::Resolve(_) => sqlx::query(
            "UPDATE workflow_assistance_cases
             SET status='RESOLVED', resolved_by_principal_id=$2,
                 resolution_payload=$3, resolution_payload_digest=$4,
                 resolution_command_id=$5, resolved_at=now(), updated_at=now()
             WHERE assistance_case_id=$1 AND status IN ('OWNER_PENDING','HUMAN_REQUIRED')",
        )
        .bind(case_id)
        .bind(actor)
        .bind(&payload)
        .bind(&payload_digest)
        .bind(command_id)
        .execute(&mut *tx)
        .await
        .map_err(storage)?
        .rows_affected(),
    };
    if affected != 1 {
        return Err(AssistanceError::InternalConsistency(
            "assistance update affected unexpected row count".to_string(),
        ));
    }
    let new_version = increment_instance_and_event(
        &mut tx,
        &instance,
        command_id,
        actor,
        case_id,
        event_type,
        Some(&previous_status),
        target_status.as_str(),
        &payload_digest,
    )
    .await?;
    let result = AssistanceCommandResult {
        assistance_case_id: case.assistance_case_id,
        workflow_instance_id: case.workflow_instance_id,
        node_visit_id: case.node_visit_id,
        status: target_status,
        workflow_state_version: new_version,
        event_sequence: new_version,
        created_at: case.created_at,
        replayed: false,
    };
    let body = serde_json::to_value(&result)
        .map_err(|error| AssistanceError::InternalConsistency(error.to_string()))?;
    complete_receipt(&mut tx, command_id, 200, &body).await?;
    tx.commit().await.map_err(storage)?;
    Ok(result)
}

pub(crate) async fn escalate_assistance(
    pool: &PgPool,
    command: EscalateAssistanceCommand,
    request_hash: &str,
) -> Result<AssistanceCommandResult, AssistanceError> {
    owner_action(
        pool,
        command.principal_id.into_uuid(),
        &command.idempotency_key,
        command.assistance_case_id.into_uuid(),
        command.expected_workflow_state_version,
        OwnerAction::Escalate(command.escalation),
        request_hash,
    )
    .await
}

pub(crate) async fn resolve_assistance(
    pool: &PgPool,
    command: ResolveAssistanceCommand,
    request_hash: &str,
) -> Result<AssistanceCommandResult, AssistanceError> {
    owner_action(
        pool,
        command.principal_id.into_uuid(),
        &command.idempotency_key,
        command.assistance_case_id.into_uuid(),
        command.expected_workflow_state_version,
        OwnerAction::Resolve(command.resolution),
        request_hash,
    )
    .await
}
