//! Caller-scoped closed projections from actual managed/receipt facts.
use super::{storage, OwnerError};
use crate::domain::definition::{digest, owner_obligation as contract};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
#[derive(Debug, FromRow)]
pub(crate) struct Snapshot {
    pub instance: Uuid,
    pub kind: String,
    pub case_id: Uuid,
    pub policy_digest: String,
    pub policy_version: Uuid,
    pub configured_actor: Uuid,
    pub version: Uuid,
    pub domain: Uuid,
    pub state_version: i32,
    pub visit: Uuid,
    pub node: Uuid,
    pub node_type: String,
    pub metadata: Option<Value>,
    pub assignee: Option<Uuid>,
    pub owner: Uuid,
    pub owner_binding: Uuid,
    pub source: Uuid,
    pub source_visit: Uuid,
    pub source_version: Uuid,
    pub binding: Uuid,
    pub terminal: Option<Uuid>,
    pub case_status: String,
    pub intent_state: String,
    pub intent_id: Uuid,
    pub stop_code: Option<String>,
    pub replacement: Option<Uuid>,
    pub stop_visit: Option<Uuid>,
    pub stop_attempt: Option<String>,
    pub stop_effect: Option<Uuid>,
    pub graph_digest: String,
    pub context: Option<Value>,
    pub context_digest: Option<String>,
}
pub(crate) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    instance: Uuid,
) -> Result<Snapshot, OwnerError> {
    let allowed = super::super::prerequisite::reader_allowed(tx, actor, instance)
        .await
        .map_err(storage)?;
    if !allowed {
        return Err(OwnerError::new(
            404,
            "workflow_instance_not_found_or_not_visible",
        ));
    }
    let row:Option<Snapshot>=sqlx::query_as("SELECT wi.workflow_instance_id AS instance,m.kind,m.case_id,m.policy_digest,m.policy_source_version_id AS policy_version,p.configured_policy_actor_principal_id AS configured_actor,wi.definition_version_id AS version,wi.domain_id AS domain,wi.workflow_state_version AS state_version,nv.node_visit_id AS visit,n.node_id AS node,n.node_type::text AS node_type,n.metadata,nv.assignee_principal_id AS assignee,b.principal_id AS owner,b.binding_id AS owner_binding,c.workflow_instance_id AS source,c.node_visit_id AS source_visit,gb.source_definition_version_id AS source_version,m.binding_id AS binding,m.child_terminal_event_id AS terminal,c.status::text AS case_status,i.state AS intent_state,i.intent_id,i.stop_code,i.owner_replacement_command_id AS replacement,i.node_visit_id AS stop_visit,i.last_attempt_id AS stop_attempt,(SELECT effect_id FROM workflow_operation_effects e WHERE e.command_id=i.last_command_id) AS stop_effect,CASE WHEN m.kind='INTAKE' THEN p.intake_graph_digest ELSE p.review_graph_digest END AS graph_digest,cr.payload AS context,cr.payload_digest AS context_digest FROM workflow_owner_managed_instances m JOIN workflow_owner_policy_approvals p ON p.source_definition_version_id=m.policy_source_version_id AND p.policy_digest=m.policy_digest JOIN workflow_instances wi ON wi.workflow_instance_id=m.workflow_instance_id JOIN workflow_definition_versions dv ON dv.definition_version_id=wi.definition_version_id AND dv.definition_digest=CASE WHEN m.kind='INTAKE' THEN p.intake_target_version_digest ELSE p.review_target_version_digest END JOIN workflow_node_visits nv ON nv.node_visit_id=wi.current_node_visit_id AND nv.workflow_instance_id=wi.workflow_instance_id JOIN workflow_node_definitions n ON n.node_id=nv.node_id AND n.definition_version_id=wi.definition_version_id JOIN workflow_assistance_cases c ON c.assistance_case_id=m.case_id JOIN workflow_governance_child_bindings gb ON gb.binding_id=m.binding_id AND gb.assistance_case_id=c.assistance_case_id JOIN workflow_owner_obligation_intents i ON i.managed_instance_id=m.workflow_instance_id AND i.kind=m.kind JOIN domains d ON d.domain_id=wi.domain_id AND d.enabled JOIN domain_role_bindings b ON b.domain_id=d.domain_id AND b.role_key='DOMAIN_OWNER' AND b.enabled JOIN principals o ON o.principal_id=b.principal_id AND o.enabled LEFT JOIN workflow_context_revisions cr ON cr.context_revision_id=wi.current_context_revision_id AND cr.workflow_instance_id=wi.workflow_instance_id WHERE m.workflow_instance_id=$1").bind(instance).fetch_optional(&mut **tx).await.map_err(storage)?;
    if let Some(row) = row {
        return Ok(row);
    }
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_managed_instances WHERE workflow_instance_id=$1)").bind(instance).fetch_one(&mut **tx).await.map_err(storage)?;
    Err(if exists {
        OwnerError::new(503, "authoritative_reference_unavailable")
    } else {
        OwnerError::new(404, "owner_obligation_not_managed")
    })
}
pub(crate) async fn own_case_effect(
    tx: &mut Transaction<'_, Postgres>,
    s: &Snapshot,
) -> Result<bool, OwnerError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_operation_effects e JOIN workflow_command_receipts r ON r.command_id=e.command_id JOIN workflow_assistance_cases c ON c.assistance_case_id=e.case_id WHERE e.managed_instance_id=$1 AND e.state='SUCCEEDED' AND r.receipt_status='COMPLETED' AND r.response_status BETWEEN 200 AND 299 AND ((r.command_type='RESOLVE_WORKFLOW_ASSISTANCE' AND c.resolution_command_id=r.command_id) OR (r.command_type='ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN' AND c.escalation_command_id=r.command_id)))").bind(s.instance).fetch_one(&mut **tx).await.map_err(storage)
}
pub(crate) async fn active(
    tx: &mut Transaction<'_, Postgres>,
    s: &Snapshot,
) -> Result<bool, OwnerError> {
    if s.intent_state == "STOPPED" || s.intent_state == "RETIRED" {
        return Ok(false);
    }
    let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_instances WHERE workflow_instance_id=$1 AND current_node_visit_id=$2 AND NOT cancelled AND archived_at IS NULL)").bind(s.source).bind(s.source_visit).fetch_one(&mut **tx).await.map_err(storage)?;
    let managed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_instances w JOIN principals p ON p.principal_id=$2 AND p.enabled AND p.principal_type='AGENT' WHERE workflow_instance_id=$1 AND NOT cancelled AND archived_at IS NULL)").bind(s.instance).bind(s.configured_actor).fetch_one(&mut **tx).await.map_err(storage)?;
    Ok(managed
        && ((s.case_status == "OWNER_PENDING" && current)
            || (s.kind == "REVIEW" && own_case_effect(tx, s).await?)))
}
pub(crate) async fn effect_projection(
    tx: &mut Transaction<'_, Postgres>,
    s: &Snapshot,
    node: Uuid,
    slot: &str,
) -> Result<Value, OwnerError> {
    let row:Option<Value>=sqlx::query_scalar("SELECT to_jsonb(e) FROM workflow_operation_effects e WHERE case_id=$1 AND managed_instance_id=$2 AND definition_version_id=$3 AND node_definition_id=$4 AND slot_key=$5 AND logical_occurrence=1").bind(s.case_id).bind(s.instance).bind(s.version).bind(node).bind(slot).fetch_optional(&mut **tx).await.map_err(storage)?;
    let Some(r) = row else {
        return Ok(json!({"state":"NOT_STARTED"}));
    };
    let state = r["state"]
        .as_str()
        .ok_or_else(|| OwnerError::new(503, "authoritative_reference_unavailable"))?;
    let mut out = json!({"state":state,"effectId":r["effect_id"]});
    for (a, b) in [
        ("concrete_request_digest", "requestDigest"),
        ("command_id", "commandId"),
    ] {
        if !r[a].is_null() {
            out[b] = r[a].clone()
        }
    }
    if state == "SUCCEEDED" {
        let command: Uuid = serde_json::from_value(r["command_id"].clone()).map_err(storage)?;
        let actual:Option<(Value,String)>=sqlx::query_as("SELECT response_body,response_digest FROM workflow_command_receipts WHERE command_id=$1 AND principal_id=$2 AND receipt_status='COMPLETED' AND response_status BETWEEN 200 AND 299").bind(command).bind(serde_json::from_value::<Uuid>(r["actor_principal_id"].clone()).map_err(storage)?).fetch_optional(&mut **tx).await.map_err(storage)?;
        if actual.as_ref()
            != Some(&(
                r["result"].clone(),
                r["result_digest"].as_str().unwrap_or("").into(),
            ))
        {
            return Err(OwnerError::new(503, "authoritative_reference_unavailable"));
        }
        if digest::compute_json_digest(&r["result"]).map_err(storage)?
            != r["result_digest"].as_str().unwrap_or("")
        {
            return Err(OwnerError::new(503, "authoritative_reference_unavailable"));
        }
        out["result"] = r["result"].clone();
        out["resultDigest"] = r["result_digest"].clone();
    } else if state == "REJECTED_ZERO_FACTS" {
        let command: Uuid = serde_json::from_value(r["command_id"].clone()).map_err(storage)?;
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_command_receipts c WHERE c.command_id=$1 AND c.receipt_status='COMPLETED' AND c.response_status NOT BETWEEN 200 AND 299 AND c.response_body=$2 AND NOT EXISTS(SELECT 1 FROM workflow_events e WHERE e.command_id=c.command_id))").bind(command).bind(&r["failure"]).fetch_one(&mut **tx).await.map_err(storage)?;
        if !valid {
            return Err(OwnerError::new(503, "authoritative_reference_unavailable"));
        }
        out["failure"] = r["failure"].clone()
    }
    Ok(out)
}
pub async fn managed(
    pool: &PgPool,
    actor: Uuid,
    instance: Uuid,
    configured_actor: Option<Uuid>,
) -> Result<Value, OwnerError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    // Leaf provenance is also service-owned and visible only through ordinary current visibility.
    if super::super::prerequisite::reader_allowed(&mut tx, actor, instance)
        .await
        .map_err(storage)?
    {
        let leaf:Option<(Uuid,Uuid,String,Uuid,i32,Uuid)>=sqlx::query_as("SELECT w.definition_version_id,o.case_id,m.policy_digest,v.node_visit_id,w.workflow_state_version,v.node_id FROM workflow_owner_dependency_origins o JOIN workflow_owner_managed_instances m ON m.workflow_instance_id=o.parent_managed_instance_id JOIN workflow_instances w ON w.workflow_instance_id=o.created_instance_id JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id WHERE o.created_instance_id=$1").bind(instance).fetch_optional(&mut *tx).await.map_err(storage)?;
        if let Some((version, case, digest, visit, state, node)) = leaf {
            tx.commit().await.map_err(storage)?;
            return Ok(
                json!({"version":1,"managedKind":"DEPENDENCY","workflowInstanceId":instance,"definitionVersionId":version,"caseId":case,"policyDigest":digest,"state":"ACTIVE","currentNode":{"nodeVisitId":visit,"nodeDefinitionId":node,"workflowStateVersion":state}}),
            );
        }
    }
    let s = load(&mut tx, actor, instance).await?;
    if configured_actor != Some(s.configured_actor)
        && !matches!(s.intent_state.as_str(), "STOPPED" | "RETIRED" | "COMPLETED")
    {
        return Err(OwnerError::new(409, "POLICY_ACTOR_MISMATCH"));
    }
    let live = active(&mut tx, &s).await?;
    let state = if s.intent_state == "STOPPED" {
        "STOPPED"
    } else if s.intent_state == "COMPLETED" || s.node_type == "TERMINAL" {
        "COMPLETED"
    } else if !live {
        "RETIRED"
    } else {
        "ACTIVE"
    };
    let mut out = json!({"version":1,"managedKind":s.kind,"workflowInstanceId":instance,"definitionVersionId":s.version,"caseId":s.case_id,"policyDigest":s.policy_digest,"state":state});
    if let Some(code) = &s.stop_code {
        let mut stop = json!({"code":code,"responsibilityId":s.intent_id});
        if let Some(cmd) = s.replacement {
            stop["ownerReplacementCommandId"] = json!(cmd)
        }
        if let Some(v) = s.stop_visit {
            stop["nodeVisitId"] = json!(v)
        }
        if let Some(v) = &s.stop_attempt {
            stop["attemptId"] = json!(v)
        }
        if let Some(v) = s.stop_effect {
            stop["effectId"] = json!(v)
        }
        out["stop"] = stop
    }
    if s.node_type != "TERMINAL" {
        let mut current = json!({"nodeVisitId":s.visit,"nodeDefinitionId":s.node,"workflowStateVersion":s.state_version});
        if let Some(op) = contract::operation(s.metadata.as_ref()).map_err(storage)? {
            let effect = effect_projection(&mut tx, &s, s.node, &op.slot_key).await?;
            let mut operation = json!({"slotKey":op.slot_key,"allowedOperations":op.allowed_operations,"effect":effect});
            if let Some(id) = op.target_definition_version_id {
                operation["targetDefinitionVersionId"] = json!(id)
            }
            if let Some(d) = op.target_definition_digest {
                operation["targetDefinitionDigest"] = json!(d)
            }
            current["operation"] = operation;
            if s.kind == "REVIEW"
                && live
                && s.owner == actor
                && s.assignee == Some(actor)
                && s.case_status == "OWNER_PENDING"
                && op
                    .allowed_operations
                    .iter()
                    .any(|d| d.key == "workflow.assistance.resolve_current_review")
            {
                let proof =
                    super::super::assistance_transaction::governance_child::managed_review_proof(
                        &mut tx, s.case_id, s.binding, s.terminal,
                    )
                    .await
                    .map_err(OwnerError::from)?;
                if let Some(proof) = proof {
                    current["reviewAdmission"] = json!({"version":1,"status":"ADMISSIBLE","source":{"workflowInstanceId":s.source,"nodeVisitId":s.source_visit,"definitionVersionId":s.source_version},"owner":{"principalId":s.owner,"bindingId":s.owner_binding},"proof":proof});
                }
            }
        }
        out["currentNode"] = current;
    }
    tx.commit().await.map_err(storage)?;
    Ok(out)
}
pub async fn effect(
    pool: &PgPool,
    actor: Uuid,
    instance: Uuid,
    effect: Uuid,
) -> Result<Value, OwnerError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    let s = load(&mut tx, actor, instance).await?;
    let (node,slot):(Uuid,String)=sqlx::query_as("SELECT node_definition_id,slot_key FROM workflow_operation_effects WHERE effect_id=$1 AND managed_instance_id=$2").bind(effect).bind(instance).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or_else(||OwnerError::new(404,"authoritative_reference_unavailable"))?;
    let result = effect_projection(&mut tx, &s, node, &slot).await?;
    tx.commit().await.map_err(storage)?;
    Ok(result)
}
