//! Existing transition/ready producers share these narrow managed predicates.
use super::{reads, storage, OwnerError};
use crate::domain::definition::{digest, owner_obligation as contract, prerequisite::Reference};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
async fn producer_reference(
    tx: &mut Transaction<'_, Postgres>,
    s: &reads::Snapshot,
    p: &contract::Producer,
    context: &Value,
) -> Result<Reference, OwnerError> {
    let r: Reference = serde_json::from_value(
        context
            .get("workflowPrerequisite")
            .cloned()
            .ok_or_else(|| OwnerError::new(422, "owner_node_gate_binding_required"))?,
    )
    .map_err(|_| OwnerError::new(422, "owner_node_gate_binding_mismatch"))?;
    let actual:Option<(Uuid,Uuid)>=sqlx::query_as("SELECT o.created_instance_id,o.target_definition_version_id FROM workflow_operation_effects e JOIN workflow_owner_dependency_origins o ON o.parent_effect_id=e.effect_id AND o.create_command_id=e.command_id JOIN workflow_command_receipts c ON c.command_id=e.command_id AND c.receipt_status='COMPLETED' AND c.response_status BETWEEN 200 AND 299 AND c.command_type='CREATE_WORKFLOW_INSTANCE' WHERE e.managed_instance_id=$1 AND e.node_definition_id=$2 AND e.slot_key=$3 AND e.state='SUCCEEDED' AND e.result=c.response_body AND e.result_digest=c.response_digest").bind(s.instance).bind(p.node_id).bind(&p.slot_key).fetch_optional(&mut **tx).await.map_err(storage)?;
    let terminal:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_transition_definitions t JOIN workflow_node_definitions n ON n.node_id=t.target_node_id AND n.definition_version_id=t.definition_version_id WHERE t.transition_id=$1 AND t.definition_version_id=$2 AND t.transition_effect='ADVANCE' AND n.node_type='TERMINAL' AND t.submission_schema IS NOT NULL)").bind(r.transition_id).bind(r.definition_version_id).fetch_one(&mut **tx).await.map_err(storage)?;
    if actual != Some((r.workflow_instance_id, r.definition_version_id)) || !terminal {
        return Err(OwnerError::new(422, "owner_node_gate_binding_mismatch"));
    }
    Ok(r)
}
pub(crate) async fn validate_operation_prerequisite(
    tx: &mut Transaction<'_, Postgres>,
    s: &reads::Snapshot,
    op: &contract::Operation,
    context: &Value,
) -> Result<(), OwnerError> {
    if let Some(p) = &op.prerequisite_producer {
        producer_reference(tx, s, p, context).await?;
    }
    Ok(())
}
pub(crate) async fn before_transition(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    instance: Uuid,
    target: Uuid,
    effect: &str,
    context: Option<&Value>,
    submission: Option<&Value>,
) -> Result<(), OwnerError> {
    let managed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_managed_instances WHERE workflow_instance_id=$1)").bind(instance).fetch_one(&mut **tx).await.map_err(storage)?;
    if !managed {
        return Ok(());
    }
    let s = reads::load(tx, actor, instance).await?;
    if !reads::active(tx, &s).await? {
        return Err(OwnerError::new(409, "GOVERNED_OPERATION_REQUIRED"));
    }
    if effect == "ADVANCE" {
        if let Some(op) = contract::operation(s.metadata.as_ref()).map_err(storage)? {
            if reads::effect_projection(tx, &s, s.node, &op.slot_key).await?["state"] != "SUCCEEDED"
            {
                return Err(OwnerError::new(409, "GOVERNED_OPERATION_REQUIRED"));
            }
        }
    }
    let target_metadata:Option<Value>=sqlx::query_scalar("SELECT metadata FROM workflow_node_definitions WHERE node_id=$1 AND definition_version_id=$2").bind(target).bind(s.version).fetch_one(&mut **tx).await.map_err(storage)?;
    if let Some(g) = contract::gate(target_metadata.as_ref()).map_err(storage)? {
        let c = context
            .or(s.context.as_ref())
            .ok_or_else(|| OwnerError::new(422, "owner_node_gate_binding_required"))?;
        producer_reference(tx, &s, &g.producer, c).await?;
    }
    let terminal:bool=sqlx::query_scalar("SELECT node_type='TERMINAL' FROM workflow_node_definitions WHERE node_id=$1 AND definition_version_id=$2").bind(target).bind(s.version).fetch_one(&mut **tx).await.map_err(storage)?;
    if terminal && s.kind == "REVIEW" {
        let outcome = if s.case_status == "RESOLVED" {
            "RESOLVED"
        } else if s.case_status == "HUMAN_REQUIRED" {
            "HUMAN_REQUIRED"
        } else {
            return Err(OwnerError::new(409, "GOVERNED_OPERATION_REQUIRED"));
        };
        if !reads::own_case_effect(tx, &s).await?
            || submission.and_then(|v| v.get("reviewOutcome")) != Some(&json!(outcome))
        {
            return Err(OwnerError::new(422, "review_outcome_mismatch"));
        }
    }
    Ok(())
}
/// None for unmanaged; real state and producer proof are evaluated before due LIMIT.
pub(crate) async fn readiness(
    tx: &mut Transaction<'_, Postgres>,
    instance: Uuid,
) -> Result<Option<Value>, OwnerError> {
    let m:Option<(Uuid,Uuid)>=sqlx::query_as("SELECT v.assignee_principal_id,w.workflow_instance_id FROM workflow_owner_managed_instances m JOIN workflow_instances w ON w.workflow_instance_id=m.workflow_instance_id JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id WHERE m.workflow_instance_id=$1").bind(instance).fetch_optional(&mut **tx).await.map_err(storage)?;
    let Some((actor, _)) = m else { return Ok(None) };
    let s = match reads::load(tx, actor, instance).await {
        Ok(s) => s,
        Err(e) if e.status < 500 || e.code == "authoritative_reference_unavailable" => {
            return Ok(Some(
                json!({"status":"UNKNOWN","reason":"LINEAGE_INCONSISTENT"}),
            ))
        }
        Err(e) => return Err(e),
    };
    if !reads::active(tx, &s).await? {
        return Ok(Some(
            json!({"status":"CLOSED","reason":"MANAGED_OBLIGATION_STOPPED"}),
        ));
    }
    let Some(g) = contract::gate(s.metadata.as_ref()).map_err(storage)? else {
        return Ok(None);
    };
    let Some(context) = s.context.as_ref() else {
        return Ok(Some(
            json!({"status":"UNKNOWN","reason":"LINEAGE_INCONSISTENT"}),
        ));
    };
    if digest::compute_json_digest(context).ok() != s.context_digest {
        return Ok(Some(
            json!({"status":"UNKNOWN","reason":"LINEAGE_INCONSISTENT"}),
        ));
    }
    match producer_reference(tx, &s, &g.producer, context).await {
        Ok(r) => super::super::prerequisite::evaluate_reference(tx, s.domain, &r)
            .await
            .map(Some)
            .map_err(storage),
        Err(e) if e.status < 500 => Ok(Some(
            json!({"status":"UNKNOWN","reason":"LINEAGE_INCONSISTENT"}),
        )),
        Err(e) => Err(e),
    }
}
