//! Stable semantic slots own actual existing business command receipts.
use super::{intents, reads, storage, OwnerError};
use crate::{
    domain::{
        definition::{digest, owner_obligation as contract},
        enums::WorkflowExecutionClass,
        ids::*,
        workflow_instance::{
            assistance::AssistancePayload, commands::CreateWorkflowInstanceCommand,
        },
    },
    store::postgres::{
        admission_gate::AdmissionGate,
        provisioning_repository::{acquire_receipt, AcquireReceipt},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Acquire, PgPool, Postgres, Transaction};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Execute {
    pub expected_workflow_state_version: i32,
    pub expected_source_workflow_state_version: i32,
    pub operation: contract::Descriptor,
    pub arguments: Value,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateArgs {
    domain_id: Uuid,
    definition_version_id: Uuid,
    context_payload: Value,
    metadata: Value,
    external_reference: Option<String>,
    external_url: Option<String>,
    execution_class: Option<WorkflowExecutionClass>,
}
fn created_body(r: super::super::create_transaction::CreateResult) -> Value {
    let mut body = json!({"workflowInstanceId":r.workflow_instance_id,"workflowStateVersion":r.workflow_state_version,"currentContextRevisionId":r.current_context_revision_id,"currentNodeVisitId":r.current_node_visit_id,"eventSequence":r.event_sequence});
    if let Some(v) = r.admission_observation {
        body["admissionObservation"] = v
    }
    body
}
fn created_error(
    e: crate::domain::workflow_instance::errors::CreateWorkflowInstanceError,
) -> OwnerError {
    use super::super::validation_helpers as v;
    if v::is_deterministic_error(&e) {
        OwnerError::new(
            v::deterministic_error_code(&e) as u16,
            v::deterministic_error_label(&e),
        )
        .with_body(v::deterministic_error_body(&e))
    } else {
        storage(e)
    }
}
async fn finish(
    tx: &mut Transaction<'_, Postgres>,
    cmd: Uuid,
    status: u16,
    body: &Value,
) -> Result<String, OwnerError> {
    let hash = digest::compute_json_digest(body).map_err(storage)?;
    super::super::command_receipt::complete_receipt(tx, cmd, status as i32, body, &hash)
        .await
        .map_err(storage)?;
    Ok(hash)
}
pub async fn execute(
    pool: &PgPool,
    actor: Uuid,
    configured_actor: Option<Uuid>,
    admission: AdmissionGate<'_>,
    instance: Uuid,
    visit: Uuid,
    slot: &str,
    body: Execute,
) -> Result<Value, OwnerError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    // Lock mutable authority and invocation coordinates before claim; no model identity in body.
    sqlx::query("SELECT d.domain_id FROM workflow_instances w JOIN domains d USING(domain_id) WHERE w.workflow_instance_id=$1 FOR UPDATE OF d").bind(instance).fetch_optional(&mut *tx).await.map_err(storage)?;
    let ids:Vec<Uuid>=sqlx::query_scalar("SELECT w.workflow_instance_id FROM workflow_instances w WHERE w.workflow_instance_id=$1 OR w.workflow_instance_id=(SELECT c.workflow_instance_id FROM workflow_owner_managed_instances m JOIN workflow_assistance_cases c ON c.assistance_case_id=m.case_id WHERE m.workflow_instance_id=$1) ORDER BY w.workflow_instance_id FOR UPDATE").bind(instance).fetch_all(&mut *tx).await.map_err(storage)?;
    if ids.is_empty() {
        return Err(OwnerError::new(
            404,
            "workflow_instance_not_found_or_not_visible",
        ));
    }
    let leaf:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_dependency_origins WHERE created_instance_id=$1)").bind(instance).fetch_one(&mut *tx).await.map_err(storage)?;
    if leaf {
        return Err(OwnerError::new(409, "GOVERNED_OPERATION_REQUIRED"));
    }
    let s = reads::load(&mut tx, actor, instance).await?;
    if s.owner != actor || s.assignee != Some(actor) {
        return Err(OwnerError::new(403, "not_domain_owner"));
    }
    if s.visit != visit {
        return Err(OwnerError::new(409, "visit_not_current"));
    }
    let op = contract::operation(s.metadata.as_ref())
        .map_err(storage)?
        .ok_or_else(|| OwnerError::new(409, "GOVERNED_OPERATION_REQUIRED"))?;
    if op.slot_key != slot
        || !op.allowed_operations.contains(&body.operation)
        || !contract::registered(&body.operation)
        || !contract::validate_arguments(&body.operation, &body.arguments)
    {
        return Err(OwnerError::new(422, "invalid_arguments"));
    }
    let business_hash = digest::compute_json_digest(
        &json!({"operation":body.operation,"arguments":body.arguments}),
    )
    .map_err(storage)?;
    let prior:Option<(Uuid,String,String,Option<Uuid>)>=sqlx::query_as("SELECT effect_id,state,business_intent_digest,command_id FROM workflow_operation_effects WHERE case_id=$1 AND managed_instance_id=$2 AND definition_version_id=$3 AND node_definition_id=$4 AND slot_key=$5 AND logical_occurrence=1 FOR UPDATE").bind(s.case_id).bind(instance).bind(s.version).bind(s.node).bind(slot).fetch_optional(&mut *tx).await.map_err(storage)?;
    if let Some((_id, state, hash, _cmd)) = prior {
        if hash != business_hash {
            return Err(OwnerError::new(409, "idempotency_conflict"));
        }
        let result = reads::effect_projection(&mut tx, &s, s.node, slot).await?;
        if state == "SUCCEEDED" {
            let out = json!({"effectId":result["effectId"],"state":"SUCCEEDED","commandId":result["commandId"],"result":result["result"],"resultDigest":result["resultDigest"],"replayed":true});
            tx.commit().await.map_err(storage)?;
            return Ok(out);
        }
        if state == "REJECTED_ZERO_FACTS" {
            return Err(OwnerError::new(409, "OPERATION_REJECTED_ZERO_FACTS"));
        }
        return Err(OwnerError::new(503, "outcome_unknown"));
    }
    if configured_actor != Some(s.configured_actor) {
        intents::stop(
            &mut tx,
            instance,
            "POLICY_ACTOR_MISMATCH",
            None,
            None,
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(storage)?;
        return Err(OwnerError::new(409, "POLICY_ACTOR_MISMATCH"));
    }
    let enabled:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM principals WHERE principal_id=$1 AND enabled AND principal_type='AGENT')").bind(s.configured_actor).fetch_one(&mut *tx).await.map_err(storage)?;
    if !enabled {
        intents::stop(
            &mut tx,
            instance,
            "SYSTEM_POLICY_ACTOR_UNAVAILABLE",
            None,
            None,
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(storage)?;
        return Err(OwnerError::new(409, "SYSTEM_POLICY_ACTOR_UNAVAILABLE"));
    }
    if !reads::active(&mut tx, &s).await? || s.node_type == "TERMINAL" {
        return Err(OwnerError::new(409, "GOVERNED_OPERATION_REQUIRED"));
    }
    let effect = Uuid::new_v4();
    let concrete = digest::compute_json_digest(
        &json!({"managedInstanceId":instance,"nodeVisitId":visit,"slotKey":slot,"body":body}),
    )
    .map_err(storage)?;
    sqlx::query("INSERT INTO workflow_operation_effects(effect_id,case_id,managed_instance_id,definition_version_id,node_definition_id,slot_key,descriptor_key,descriptor_version,descriptor_hash,definition_graph_digest,first_authorizing_visit_id,actor_principal_id,business_intent_digest,concrete_request_digest,state) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,'PREPARED')").bind(effect).bind(s.case_id).bind(instance).bind(s.version).bind(s.node).bind(slot).bind(&body.operation.key).bind(body.operation.version as i32).bind(&body.operation.hash).bind(&s.graph_digest).bind(visit).bind(actor).bind(business_hash).bind(concrete).execute(&mut *tx).await.map_err(storage)?;
    let key = format!("owner-effect:{effect}");
    let create = if body.operation.key == "workflow.create_instance" {
        let args: CreateArgs = serde_json::from_value(body.arguments.clone())
            .map_err(|_| OwnerError::new(422, "invalid_arguments"))?;
        Some(CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(actor),
            domain_id: DomainId::from_uuid(args.domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(args.definition_version_id),
            idempotency_key: key.clone(),
            command_schema_version: "v1".into(),
            execution_class: args
                .execution_class
                .unwrap_or(WorkflowExecutionClass::Business),
            external_reference: args.external_reference,
            external_url: args.external_url,
            metadata: args.metadata,
            context_payload: args.context_payload,
        })
    } else {
        None
    };
    let kind = if create.is_some() {
        "CREATE_WORKFLOW_INSTANCE"
    } else if body.operation.key == "workflow.assistance.resolve_current_review" {
        "RESOLVE_WORKFLOW_ASSISTANCE"
    } else {
        "ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN"
    };
    let hash = if let Some(c) = &create {
        crate::application::workflow_instance::idempotency::compute_request_hash(
            "v1",
            &key,
            &c.principal_id,
            &c.domain_id,
            &c.execution_class,
            &c.definition_version_id,
            &c.context_payload,
            &c.metadata,
            &c.external_reference,
            &c.external_url,
        )
        .map_err(created_error)?
    } else {
        let mut args = body.arguments.clone();
        args.as_object_mut()
            .ok_or_else(|| OwnerError::new(422, "invalid_arguments"))?
            .remove("assistanceCaseId");
        crate::application::workflow_instance::assistance::command_hash(
            kind,
            "v1",
            actor,
            json!({"assistanceCaseId":s.case_id}),
            args,
        )
        .map_err(OwnerError::from)?
    };
    let command = match acquire_receipt(&mut tx, actor, &key, kind, &hash)
        .await
        .map_err(storage)?
    {
        AcquireReceipt::Owned(id) => id,
        _ => return Err(OwnerError::new(503, "outcome_unknown")),
    };
    sqlx::query(
        "UPDATE workflow_operation_effects SET state='STARTED',command_id=$2 WHERE effect_id=$1",
    )
    .bind(effect)
    .bind(command)
    .execute(&mut *tx)
    .await
    .map_err(storage)?;
    let mut facts = tx.begin().await.map_err(storage)?;
    let attempt=async {
  let actual_source:i32=sqlx::query_scalar("SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1").bind(s.source).fetch_one(&mut *facts).await.map_err(storage)?;
  if s.state_version!=body.expected_workflow_state_version||actual_source!=body.expected_source_workflow_state_version{return Err(OwnerError::new(409,"workflow_state_version_conflict"))}
  if let Some(c)=&create {
   if c.domain_id.into_uuid()!=s.domain||Some(c.definition_version_id.into_uuid())!=op.target_definition_version_id{return Err(OwnerError::new(422,"OPERATION_TARGET_MISMATCH"))}
   let actual:Option<String>=sqlx::query_scalar("SELECT definition_digest FROM workflow_definition_versions WHERE definition_version_id=$1").bind(c.definition_version_id.into_uuid()).fetch_one(&mut *facts).await.map_err(storage)?;
   if actual!=op.target_definition_digest{return Err(OwnerError::new(422,"OPERATION_TARGET_MISMATCH"))}
   super::guards::validate_operation_prerequisite(&mut facts,&s,&op,&c.context_payload).await?;
   let r=super::super::create_transaction::create_on_tx(&mut facts,admission,c,command).await.map_err(created_error)?;let child=r.workflow_instance_id;
   // Effect success and origin are deferred-checked against the actual receipt at commit.
   sqlx::query("INSERT INTO workflow_owner_dependency_origins(created_instance_id,parent_effect_id,parent_managed_instance_id,case_id,create_command_id,target_definition_version_id) VALUES($1,$2,$3,$4,$5,$6)").bind(child).bind(effect).bind(instance).bind(s.case_id).bind(command).bind(c.definition_version_id.into_uuid()).execute(&mut *facts).await.map_err(storage)?;Ok(created_body(r))
  }else{
   if s.kind!="REVIEW"||body.arguments["assistanceCaseId"]!=json!(s.case_id)||body.arguments["expectedWorkflowStateVersion"]!=json!(body.expected_source_workflow_state_version){return Err(OwnerError::new(409,"GOVERNED_OPERATION_REQUIRED"))}
   let action=if kind=="RESOLVE_WORKFLOW_ASSISTANCE"{
    let expected=super::super::assistance_transaction::governance_child::managed_review_proof(&mut facts,s.case_id,s.binding,s.terminal).await.map_err(OwnerError::from)?.ok_or_else(||OwnerError::new(409,"authoritative_reference_unavailable"))?;
    if body.arguments.pointer("/resolution/supportingPayload/governanceChildReview")!=Some(&expected){return Err(OwnerError::new(409,"authoritative_reference_unavailable"))}
    super::super::assistance_transaction::OwnerAction::Resolve(serde_json::from_value::<AssistancePayload>(body.arguments["resolution"].clone()).map_err(|_|OwnerError::new(422,"invalid_arguments"))?)
   }else{super::super::assistance_transaction::OwnerAction::Escalate(serde_json::from_value::<AssistancePayload>(body.arguments["escalation"].clone()).map_err(|_|OwnerError::new(422,"invalid_arguments"))?)};
   let r=super::super::assistance_transaction::owner_action_on_tx(&mut facts,actor,s.case_id,body.expected_source_workflow_state_version,action,command).await.map_err(OwnerError::from)?;serde_json::to_value(r).map_err(storage)
  }
 }.await;
    match attempt {
        Ok(result) => {
            facts.commit().await.map_err(storage)?;
            let result_digest = finish(&mut tx, command, 200, &result).await?;
            sqlx::query("UPDATE workflow_operation_effects SET state='SUCCEEDED',result=$2,result_digest=$3 WHERE effect_id=$1").bind(effect).bind(&result).bind(&result_digest).execute(&mut *tx).await.map_err(storage)?;
            admission.check_commit_budget().map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(
                json!({"effectId":effect,"state":"SUCCEEDED","commandId":command,"result":result,"resultDigest":result_digest,"replayed":false}),
            )
        }
        Err(e) if e.status < 500 => {
            facts.rollback().await.map_err(storage)?;
            let failure = e.body();
            finish(&mut tx, command, e.status, &failure).await?;
            sqlx::query("UPDATE workflow_operation_effects SET state='REJECTED_ZERO_FACTS',failure=$2,last_outcome_code=$3 WHERE effect_id=$1").bind(effect).bind(failure).bind(&e.code).execute(&mut *tx).await.map_err(storage)?;
            intents::stop(
                &mut tx,
                instance,
                "OPERATION_REJECTED_ZERO_FACTS",
                Some(command),
                None,
                None,
                None,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Err(e)
        }
        Err(e) => Err(e),
    }
}
