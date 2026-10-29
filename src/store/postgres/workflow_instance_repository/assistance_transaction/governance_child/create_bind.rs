//! One receipt + one transaction: child facts never escape without a binding.
use super::*;
use crate::{
    domain::{
        enums::WorkflowExecutionClass,
        ids::{DomainId, WorkflowInstanceId},
        workflow_instance::{
            commands::CreateWorkflowInstanceCommand, errors::CreateWorkflowInstanceError,
            governance_child::CreateAndBindGovernanceChildCommand,
        },
    },
    store::postgres::admission_gate::AdmissionGate,
};
use sqlx::Acquire;
fn create_error(e: CreateWorkflowInstanceError) -> AssistanceError {
    match e {
        CreateWorkflowInstanceError::StorageError(s) => AssistanceError::StorageError(s),
        CreateWorkflowInstanceError::InternalConsistency(s) => {
            AssistanceError::InternalConsistency(s)
        }
        CreateWorkflowInstanceError::AdmissionFailed(e) => {
            AssistanceError::StorageError(e.sanitized_code().into())
        }
        CreateWorkflowInstanceError::NotDomainOwner => AssistanceError::NotDomainOwner,
        e => invalid(&e.to_string()),
    }
}
pub(crate) async fn create_and_bind(
    pool: &PgPool,
    admission: AdmissionGate<'_>,
    c: CreateAndBindGovernanceChildCommand,
    hash: &str,
) -> Result<Value, AssistanceError> {
    // Retry only PostgreSQL's explicit aborts; never replay an uncertain COMMIT.
    for attempt in 0..3 {
        match once(pool, admission, &c, hash).await {
            Err(AssistanceError::StorageError(s))
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
async fn once(
    pool: &PgPool,
    admission: AdmissionGate<'_>,
    c: &CreateAndBindGovernanceChildCommand,
    hash: &str,
) -> Result<Value, AssistanceError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    admission
        .bind_statement_deadline(&mut tx)
        .await
        .map_err(|e| AssistanceError::StorageError(e.to_string()))?;
    let actor = c.principal_id.into_uuid();
    let command = match acquire_receipt(
        &mut tx,
        actor,
        &c.idempotency_key,
        "CREATE_AND_BIND_GOVERNANCE_CHILD",
        hash,
    )
    .await?
    {
        ReceiptAcquire::Owned(id) => id,
        ReceiptAcquire::Replay(status, body) => {
            tx.commit().await.map_err(storage)?;
            return if (200..300).contains(&status) {
                Ok(body)
            } else {
                Err(replay_error(status, &body))
            };
        }
        ReceiptAcquire::Conflict => return Err(AssistanceError::IdempotencyConflict),
        ReceiptAcquire::Processing => return Err(AssistanceError::CommandStillProcessing),
    };
    // Keep the owned receipt outside the savepoint. Any later deterministic failure
    // rolls back *all* child/activation/outbox/binding facts before failure completion.
    let mut facts = tx.begin().await.map_err(storage)?;
    let result = admit_create(&mut facts, admission, c, command).await;
    let (status, body) = match result {
        Ok(body) => {
            facts.commit().await.map_err(storage)?;
            (201, body)
        }
        Err(e @ AssistanceError::StorageError(_))
        | Err(e @ AssistanceError::InternalConsistency(_)) => return Err(e),
        Err(e) => {
            facts.rollback().await.map_err(storage)?;
            complete_receipt(&mut tx, command, e.status_code(), &error_body(&e)).await?;
            tx.commit().await.map_err(storage)?;
            return Err(e);
        }
    };
    complete_receipt(&mut tx, command, status, &body).await?;
    admission
        .check_commit_budget()
        .map_err(|e| AssistanceError::StorageError(e.sanitized_code().into()))?;
    if admission.is_enabled() {
        tokio::time::timeout(
            std::time::Duration::from_millis(admission.remaining_budget_ms()),
            tx.commit(),
        )
        .await
        .map_err(|_| AssistanceError::StorageError("commit outcome unknown".into()))?
        .map_err(storage)?;
    } else {
        tx.commit().await.map_err(storage)?;
    }
    Ok(body)
}
async fn admit_create(
    tx: &mut Transaction<'_, Postgres>,
    admission: AdmissionGate<'_>,
    c: &CreateAndBindGovernanceChildCommand,
    command: Uuid,
) -> Result<Value, AssistanceError> {
    let actor = c.principal_id.into_uuid();
    let case = c.assistance_case_id.into_uuid();
    validate_actor_enabled(tx, actor).await?;
    let source: Uuid = sqlx::query_scalar(
        "SELECT workflow_instance_id FROM workflow_assistance_cases WHERE assistance_case_id=$1",
    )
    .bind(case)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    .ok_or(AssistanceError::AssistanceCaseNotFoundOrNotVisible)?;
    let src = lock_instance(tx, source).await?;
    if effective_owner(tx, src.domain_id).await? != actor {
        return Err(AssistanceError::NotDomainOwner);
    }
    let status:Option<(String,Uuid)>=sqlx::query_as("SELECT status,node_visit_id FROM workflow_assistance_cases WHERE assistance_case_id=$1 FOR UPDATE").bind(case).fetch_optional(&mut **tx).await.map_err(storage)?;
    if status.as_ref().map(|(s, v)| (s.as_str(), Some(*v)))
        != Some(("OWNER_PENDING", src.current_node_visit_id))
        || src.cancelled
        || src.archived_at.is_some()
        || src.node_type.as_deref() == Some("TERMINAL")
    {
        return Err(conflict());
    }
    if src.workflow_state_version != c.expected_workflow_state_version {
        return Err(AssistanceError::WorkflowStateVersionConflict {
            expected: c.expected_workflow_state_version,
            actual: src.workflow_state_version,
        });
    }
    let cmd = CreateWorkflowInstanceCommand {
        principal_id: c.principal_id,
        idempotency_key: c.idempotency_key.clone(),
        command_schema_version: c.command_schema_version.clone(),
        domain_id: DomainId::from_uuid(src.domain_id),
        definition_version_id: c.definition_version_id,
        execution_class: WorkflowExecutionClass::Business,
        external_reference: None,
        external_url: None,
        metadata: json!({}),
        context_payload: c.context_payload.clone(),
    };
    let child = super::super::super::create_transaction::create_on_tx(tx, admission, &cmd, command)
        .await
        .map_err(create_error)?;
    let bind = BindGovernanceChildCommand {
        principal_id: c.principal_id,
        idempotency_key: c.idempotency_key.clone(),
        command_schema_version: c.command_schema_version.clone(),
        assistance_case_id: c.assistance_case_id,
        governance_child_instance_id: WorkflowInstanceId::from_uuid(child.workflow_instance_id),
        acceptance_transition_id: c.acceptance_transition_id,
        expected_workflow_state_version: c.expected_workflow_state_version,
    };
    let mut receipt =
        super::binding::admit(tx, &bind, command, actor, case, child.workflow_instance_id).await?;
    if let Some(v) = super::super::super::prerequisite::project(tx, child.workflow_instance_id)
        .await
        .map_err(|e| AssistanceError::StorageError(e.to_string()))?
    {
        receipt["prerequisiteReadiness"] = v;
    }
    Ok(receipt)
}
