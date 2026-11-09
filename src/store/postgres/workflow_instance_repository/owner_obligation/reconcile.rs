//! Internal durable producer; no model/API policy-execute surface.
use super::{intents, storage, OwnerError};
use crate::{
    domain::{
        enums::WorkflowExecutionClass,
        ids::*,
        workflow_instance::{
            commands::CreateWorkflowInstanceCommand, governance_child::BindGovernanceChildCommand,
        },
    },
    store::postgres::{
        admission_gate::AdmissionGate,
        provisioning_repository::{acquire_receipt, complete_receipt, AcquireReceipt},
    },
};
use serde_json::{json, Value};
use sqlx::{Acquire, PgPool};
use uuid::Uuid;
fn id(v: &Value, k: &str) -> Result<Uuid, OwnerError> {
    v[k].as_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| OwnerError::new(503, "authoritative_reference_unavailable"))
}
/// Configuration is trusted service state; no actor credential is manufactured here.
pub async fn reconcile_configured(
    pool: &PgPool,
    actor: Option<Uuid>,
    admission: AdmissionGate<'_>,
) -> Result<Vec<Value>, OwnerError> {
    stop_actor_mismatches(pool, actor).await?;
    if let Some(actor) = actor {
        reconcile_pending_inner(pool, actor, admission).await
    } else {
        Ok(Vec::new())
    }
}
async fn stop_actor_mismatches(pool: &PgPool, actor: Option<Uuid>) -> Result<(), OwnerError> {
    let rows:Vec<(Uuid,Uuid,Option<Uuid>,bool)>=sqlx::query_as("SELECT i.intent_id,i.source_instance_id,i.managed_instance_id,(p.configured_policy_actor_principal_id IS NOT DISTINCT FROM $1 AND EXISTS(SELECT 1 FROM principals a WHERE a.principal_id=p.configured_policy_actor_principal_id AND a.enabled AND a.principal_type='AGENT')) FROM workflow_owner_obligation_intents i JOIN workflow_owner_policy_approvals p ON p.source_definition_version_id=i.policy_source_version_id WHERE i.kind IN ('INTAKE','REVIEW') AND i.state IN ('PENDING','MATERIALIZED') AND (p.configured_policy_actor_principal_id IS DISTINCT FROM $1 OR NOT EXISTS(SELECT 1 FROM principals a WHERE a.principal_id=p.configured_policy_actor_principal_id AND a.enabled AND a.principal_type='AGENT')) ORDER BY i.intent_id LIMIT 64").bind(actor).fetch_all(pool).await.map_err(storage)?;
    for (intent, source, managed, valid) in rows {
        if valid {
            continue;
        }
        let mut tx = pool.begin().await.map_err(storage)?;
        intents::fence(&mut tx, source).await.map_err(storage)?;
        let invalid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_obligation_intents i JOIN workflow_owner_policy_approvals p ON p.source_definition_version_id=i.policy_source_version_id WHERE i.intent_id=$1 AND i.state IN ('PENDING','MATERIALIZED') AND (p.configured_policy_actor_principal_id IS DISTINCT FROM $2 OR NOT EXISTS(SELECT 1 FROM principals a WHERE a.principal_id=p.configured_policy_actor_principal_id AND a.enabled AND a.principal_type='AGENT')))").bind(intent).bind(actor).fetch_one(&mut *tx).await.map_err(storage)?;
        if invalid {
            if let Some(managed) = managed {
                intents::stop(
                    &mut tx,
                    managed,
                    "POLICY_ACTOR_MISMATCH",
                    None,
                    None,
                    None,
                    None,
                )
                .await?;
            } else {
                sqlx::query("UPDATE workflow_owner_obligation_intents SET state='STOPPED',stop_code='POLICY_ACTOR_MISMATCH',outcome='POLICY_ACTOR_MISMATCH' WHERE intent_id=$1").bind(intent).execute(&mut *tx).await.map_err(storage)?;
            }
        }
        tx.commit().await.map_err(storage)?;
    }
    Ok(())
}
pub async fn reconcile_pending(
    pool: &PgPool,
    actor: Uuid,
    admission: AdmissionGate<'_>,
) -> Result<Vec<Value>, OwnerError> {
    stop_actor_mismatches(pool, Some(actor)).await?;
    reconcile_pending_inner(pool, actor, admission).await
}
async fn reconcile_pending_inner(
    pool: &PgPool,
    actor: Uuid,
    admission: AdmissionGate<'_>,
) -> Result<Vec<Value>, OwnerError> {
    cleanup_retired(pool, actor).await?;
    let ids:Vec<Uuid>=sqlx::query_scalar("SELECT intent_id FROM workflow_owner_obligation_intents WHERE kind IN ('INTAKE','REVIEW') AND state='PENDING' ORDER BY created_at,intent_id LIMIT 64").fetch_all(pool).await.map_err(storage)?;
    let mut result = Vec::new();
    for i in ids {
        if let Some(v) = materialize(pool, actor, admission, i).await? {
            result.push(v)
        }
    }
    Ok(result)
}
async fn materialize(
    pool: &PgPool,
    actor: Uuid,
    admission: AdmissionGate<'_>,
    intent: Uuid,
) -> Result<Option<Value>, OwnerError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    // Same Domain fence as actual Owner replacement, before taking responsibility.
    let domain:Uuid=sqlx::query_scalar("SELECT d.domain_id FROM workflow_owner_obligation_intents i JOIN workflow_instances w ON w.workflow_instance_id=i.source_instance_id JOIN domains d USING(domain_id) WHERE i.intent_id=$1 FOR UPDATE OF d").bind(intent).fetch_one(&mut *tx).await.map_err(storage)?;
    let i: Value = sqlx::query_scalar(
        "SELECT to_jsonb(i) FROM workflow_owner_obligation_intents i WHERE intent_id=$1 FOR UPDATE",
    )
    .bind(intent)
    .fetch_one(&mut *tx)
    .await
    .map_err(storage)?;
    if i["state"] != "PENDING" {
        tx.commit().await.map_err(storage)?;
        return Ok(None);
    }
    let valid_actor:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_policy_approvals a JOIN principals p ON p.principal_id=$2 AND p.enabled AND p.principal_type='AGENT' JOIN domains d ON d.domain_id=a.domain_id AND d.enabled WHERE a.source_definition_version_id=$1 AND a.configured_policy_actor_principal_id=$2 AND a.policy_digest=$3)").bind(id(&i,"policy_source_version_id")?).bind(actor).bind(i["policy_digest"].as_str()).fetch_one(&mut *tx).await.map_err(storage)?;
    if !valid_actor {
        sqlx::query("UPDATE workflow_owner_obligation_intents SET state='STOPPED',stop_code='POLICY_ACTOR_MISMATCH',outcome='POLICY_ACTOR_MISMATCH' WHERE intent_id=$1").bind(intent).execute(&mut *tx).await.map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        return Ok(None);
    }
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_assistance_cases c JOIN workflow_instances w USING(workflow_instance_id) JOIN domain_role_bindings b ON b.domain_id=w.domain_id AND b.role_key='DOMAIN_OWNER' AND b.enabled JOIN principals p ON p.principal_id=b.principal_id AND p.enabled WHERE c.assistance_case_id=$1 AND c.status='OWNER_PENDING' AND c.node_visit_id=w.current_node_visit_id AND NOT w.cancelled AND w.archived_at IS NULL)").bind(id(&i,"case_id")?).fetch_one(&mut *tx).await.map_err(storage)?;
    if !active {
        sqlx::query("UPDATE workflow_owner_obligation_intents SET state='RETIRED',outcome='SOURCE_NOT_CURRENT' WHERE intent_id=$1").bind(intent).execute(&mut *tx).await.map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        return Ok(None);
    }
    let kind = i["kind"]
        .as_str()
        .ok_or_else(|| OwnerError::new(503, "authoritative_reference_unavailable"))?;
    let command_type = if kind == "INTAKE" {
        "AUTO_CREATE_AND_BIND_OWNER_INTAKE"
    } else {
        "AUTO_CREATE_OWNER_REVIEW"
    };
    let key = format!("owner-auto:{intent}");
    let hash=crate::domain::definition::digest::compute_json_digest(&json!({"intentId":intent,"policyDigest":i["policy_digest"],"target":i["selected_definition_version_id"]})).map_err(storage)?;
    let command = match acquire_receipt(&mut tx, actor, &key, command_type, &hash)
        .await
        .map_err(storage)?
    {
        AcquireReceipt::Owned(command_id) => command_id,
        AcquireReceipt::Replay {
            response_status,
            response_body,
            ..
        } => {
            tx.commit().await.map_err(storage)?;
            return if (200..300).contains(&response_status) {
                Ok(Some(response_body))
            } else {
                Ok(None)
            };
        }
        _ => return Err(OwnerError::new(409, "idempotency_conflict")),
    };
    sqlx::query(
        "UPDATE workflow_owner_obligation_intents SET last_command_id=$2 WHERE intent_id=$1",
    )
    .bind(intent)
    .bind(command)
    .execute(&mut *tx)
    .await
    .map_err(storage)?;
    let context: Value = sqlx::query_scalar(
        "SELECT payload FROM workflow_context_revisions WHERE context_revision_id=$1",
    )
    .bind(id(&i, "request_context_revision_id")?)
    .fetch_one(&mut *tx)
    .await
    .map_err(storage)?;
    let selected_context = context[if kind == "INTAKE" {
        "ownerIntakeContext"
    } else {
        "ownerReviewContext"
    }]
    .clone();
    let source = id(&i, "source_instance_id")?;
    let case = id(&i, "case_id")?;
    let target = id(&i, "selected_definition_version_id")?;
    let mut facts = tx.begin().await.map_err(storage)?;
    let create = CreateWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(actor),
        idempotency_key: key.clone(),
        command_schema_version: "v1".into(),
        domain_id: DomainId::from_uuid(domain),
        definition_version_id: DefinitionVersionId::from_uuid(target),
        execution_class: WorkflowExecutionClass::Business,
        external_reference: None,
        external_url: None,
        metadata: json!({}),
        context_payload: selected_context,
    };
    let attempt=async {
  if !create.context_payload.is_object(){return Err(OwnerError::new(422,"POLICY_INITIAL_CONTEXT_INVALID"))}
  let child=super::super::create_transaction::create_on_tx(&mut facts,admission,&create,command).await.map_err(|e|{use crate::domain::workflow_instance::errors::CreateWorkflowInstanceError as E;match e{E::ContextValidationFailed(ref detail)=>OwnerError::new(422,"POLICY_INITIAL_CONTEXT_INVALID").with_body(json!({"error":"policy_initial_context_invalid","detail":detail})),_ if super::super::validation_helpers::is_deterministic_error(&e)=>OwnerError::new(super::super::validation_helpers::deterministic_error_code(&e) as u16,super::super::validation_helpers::deterministic_error_label(&e)).with_body(super::super::validation_helpers::deterministic_error_body(&e)),_=>storage(e)}})?;
  let mut binding=id(&i,"binding_id").ok();
  let mut body=json!({"workflowInstanceId":child.workflow_instance_id,"workflowStateVersion":child.workflow_state_version,"currentContextRevisionId":child.current_context_revision_id,"currentNodeVisitId":child.current_node_visit_id,"eventSequence":child.event_sequence});
  if let Some(v)=&child.admission_observation{body["admissionObservation"]=v.clone()}
  if kind=="INTAKE" {
   let expected:i32=sqlx::query_scalar("SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1 FOR UPDATE").bind(source).fetch_one(&mut *facts).await.map_err(storage)?;
   let cmd=BindGovernanceChildCommand{principal_id:PrincipalId::from_uuid(actor),idempotency_key:key,command_schema_version:"v1".into(),assistance_case_id:AssistanceCaseId::from_uuid(case),governance_child_instance_id:WorkflowInstanceId::from_uuid(child.workflow_instance_id),acceptance_transition_id:TransitionId::from_uuid(id(&i,"selected_transition_id")?),expected_workflow_state_version:expected};
   body=super::super::assistance_transaction::governance_child::bind_on_tx(&mut facts,&cmd,command,actor,case,child.workflow_instance_id).await.map_err(OwnerError::from)?;
   binding=Some(id(&body,"bindingId")?);if let Some(v)=child.prerequisite_readiness {body["prerequisiteReadiness"]=v}
  }
  sqlx::query("INSERT INTO workflow_owner_managed_instances(workflow_instance_id,kind,case_id,policy_source_version_id,policy_digest,binding_id,child_terminal_event_id,creation_command_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8)").bind(child.workflow_instance_id).bind(kind).bind(case).bind(id(&i,"policy_source_version_id")?).bind(i["policy_digest"].as_str()).bind(binding).bind(i["child_terminal_event_id"].as_str().and_then(|s|s.parse::<Uuid>().ok())).bind(command).execute(&mut *facts).await.map_err(storage)?;
  sqlx::query("UPDATE workflow_owner_obligation_intents SET state='MATERIALIZED',managed_instance_id=$2,binding_id=$3,updated_at=now() WHERE intent_id=$1").bind(intent).bind(child.workflow_instance_id).bind(binding).execute(&mut *facts).await.map_err(storage)?;
  Ok::<_,OwnerError>(body)
 }.await;
    match attempt {
        Ok(body) => {
            facts.commit().await.map_err(storage)?;
            complete_receipt(
                &mut tx,
                command,
                if kind == "INTAKE" { 201 } else { 200 },
                &body,
            )
            .await
            .map_err(storage)?;
            admission.check_commit_budget().map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(Some(body))
        }
        Err(e) if e.status < 500 => {
            facts.rollback().await.map_err(storage)?;
            complete_receipt(&mut tx, command, e.status as i32, &e.body())
                .await
                .map_err(storage)?;
            sqlx::query("UPDATE workflow_owner_obligation_intents SET state='BLOCKED_KNOWN',outcome=$2 WHERE intent_id=$1").bind(intent).bind(&e.code).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Reconstruct stale responsibility from real source facts; each child closes via its own receipt.
async fn cleanup_retired(pool: &PgPool, actor: Uuid) -> Result<(), OwnerError> {
    let candidates:Vec<(Uuid,Uuid)>=sqlx::query_as("SELECT i.intent_id,i.source_instance_id FROM workflow_owner_obligation_intents i JOIN workflow_assistance_cases c ON c.assistance_case_id=i.case_id JOIN workflow_instances w ON w.workflow_instance_id=i.source_instance_id WHERE i.kind IN ('INTAKE','REVIEW') AND i.state IN ('PENDING','MATERIALIZED') AND (w.cancelled OR w.archived_at IS NOT NULL OR w.current_node_visit_id<>i.source_node_visit_id OR c.status IN ('RESOLVED','VOIDED','HUMAN_REQUIRED')) AND NOT (i.kind='REVIEW' AND EXISTS(SELECT 1 FROM workflow_operation_effects e WHERE e.managed_instance_id=i.managed_instance_id AND e.state='SUCCEEDED' AND e.command_id=COALESCE(c.resolution_command_id,c.escalation_command_id))) ORDER BY i.intent_id LIMIT 64").fetch_all(pool).await.map_err(storage)?;
    for (intent, source) in candidates {
        let mut tx = pool.begin().await.map_err(storage)?;
        intents::fence(&mut tx, source).await.map_err(storage)?;
        let stale:Option<Uuid>=sqlx::query_scalar("SELECT COALESCE(c.voided_by_command_id,c.resolution_command_id,c.escalation_command_id) FROM workflow_owner_obligation_intents i JOIN workflow_assistance_cases c ON c.assistance_case_id=i.case_id JOIN workflow_instances w ON w.workflow_instance_id=i.source_instance_id WHERE i.intent_id=$1 AND (w.cancelled OR w.archived_at IS NOT NULL OR w.current_node_visit_id<>i.source_node_visit_id OR c.status IN ('RESOLVED','VOIDED','HUMAN_REQUIRED')) AND NOT (i.kind='REVIEW' AND EXISTS(SELECT 1 FROM workflow_operation_effects e WHERE e.managed_instance_id=i.managed_instance_id AND e.state='SUCCEEDED' AND e.command_id=COALESCE(c.resolution_command_id,c.escalation_command_id)))").bind(intent).fetch_optional(&mut *tx).await.map_err(storage)?.flatten();
        if let Some(command) = stale {
            intents::retire(&mut tx, source, command, "SOURCE_NOT_CURRENT").await?;
        }
        tx.commit().await.map_err(storage)?;
    }
    let children:Vec<(Uuid,Uuid)>=sqlx::query_as("SELECT m.workflow_instance_id,i.intent_id FROM workflow_owner_managed_instances m JOIN workflow_owner_obligation_intents i ON i.managed_instance_id=m.workflow_instance_id AND i.kind=m.kind JOIN workflow_owner_policy_approvals p ON p.source_definition_version_id=m.policy_source_version_id JOIN workflow_instances w ON w.workflow_instance_id=m.workflow_instance_id JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n USING(node_id) WHERE i.state='RETIRED' AND p.configured_policy_actor_principal_id=$1 AND NOT w.cancelled AND w.archived_at IS NULL AND n.node_type<>'TERMINAL' ORDER BY m.workflow_instance_id LIMIT 64").bind(actor).fetch_all(pool).await.map_err(storage)?;
    for (instance, intent) in children {
        let key = format!("owner-cleanup:{intent}");
        let reason = "Parent owner-obligation responsibility retired";
        let hash = crate::domain::definition::digest::compute_json_digest(
            &json!({"workflowInstanceId":instance,"reason":reason}),
        )
        .map_err(storage)?;
        let cmd = crate::domain::workflow_instance::commands::CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(actor),
            idempotency_key: key,
            command_schema_version: "v1".into(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance),
            expected_workflow_state_version: 0,
            reason: reason.into(),
        };
        super::super::cancel_transaction::cancel_workflow_instance_atomically(pool, cmd, &hash)
            .await
            .map_err(storage)?;
    }
    Ok(())
}
