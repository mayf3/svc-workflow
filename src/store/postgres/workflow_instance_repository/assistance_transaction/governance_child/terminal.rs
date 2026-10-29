use super::*;

/// The child instance lock is already held by either real transition writer.
/// Take the Domain guard before the relation lock, matching Owner replacement.
async fn lock_child_binding(
    tx: &mut Transaction<'_, Postgres>,
    child: Uuid,
) -> Result<Option<Binding>, AssistanceError> {
    let domain:Option<Uuid>=sqlx::query_scalar("SELECT wi.domain_id FROM workflow_governance_child_bindings b JOIN workflow_instances wi ON wi.workflow_instance_id=b.governance_child_instance_id WHERE b.governance_child_instance_id=$1").bind(child).fetch_optional(&mut **tx).await.map_err(storage)?;
    if let Some(domain) = domain {
        effective_owner(tx, domain).await?;
        by_child(tx, child).await
    } else {
        Ok(None)
    }
}
pub(crate) async fn validate_selected_submission(
    tx: &mut Transaction<'_, Postgres>,
    child: Uuid,
    transition: Uuid,
    payload: Option<&Value>,
) -> Result<(), AssistanceError> {
    let Some(b) = lock_child_binding(tx, child).await? else {
        return Ok(());
    };
    if b.acceptance_transition_id == transition {
        acceptance(&b, payload)?;
    }
    Ok(())
}
fn acceptance(
    b: &Binding,
    payload: Option<&Value>,
) -> Result<GovernanceAcceptance, AssistanceError> {
    let value = payload
        .and_then(|p| p.get("governanceAcceptance"))
        .ok_or_else(|| invalid("governanceAcceptance is required"))?;
    let a: GovernanceAcceptance = serde_json::from_value(value.clone())
        .map_err(|_| invalid("governanceAcceptance must be a closed exact reference"))?;
    if a.binding_id != b.binding_id
        || a.acceptance_contract_digest != b.acceptance_contract_digest
        || a.source_request_payload_digest != b.request_payload_digest
        || !digest_shape(&a.acceptance_contract_digest)
        || !digest_shape(&a.source_request_payload_digest)
    {
        return Err(invalid(
            "governanceAcceptance disagrees with the immutable binding",
        ));
    }
    Ok(a)
}
#[derive(FromRow)]
struct TerminalFacts {
    event_id: Uuid,
    source_node_visit_id: Uuid,
    target_node_visit_id: Uuid,
    context_revision_id: Uuid,
    definition_version_id: Uuid,
    actor_principal_id: Uuid,
    transition_id: Uuid,
    submission_id: Option<Uuid>,
    payload_digest: Option<String>,
    payload: Option<Value>,
}
async fn facts(
    tx: &mut Transaction<'_, Postgres>,
    child: Uuid,
    event: Uuid,
) -> Result<TerminalFacts, AssistanceError> {
    sqlx::query_as("SELECT e.event_id,e.source_node_visit_id,e.target_node_visit_id,e.context_revision_id,wi.definition_version_id,e.actor_principal_id,t.transition_id,s.submission_id,s.payload_digest,s.payload
 FROM workflow_events e JOIN workflow_instances wi ON wi.workflow_instance_id=e.workflow_instance_id
 JOIN workflow_node_visits sv ON sv.node_visit_id=e.source_node_visit_id AND sv.workflow_instance_id=wi.workflow_instance_id
 JOIN workflow_node_visits tv ON tv.node_visit_id=e.target_node_visit_id AND tv.workflow_instance_id=wi.workflow_instance_id
 JOIN workflow_node_definitions nd ON nd.node_id=tv.node_id AND nd.definition_version_id=wi.definition_version_id AND nd.node_type='TERMINAL'
 JOIN workflow_transition_definitions t ON t.transition_id=(e.event_data->>'transition_definition_id')::uuid AND t.definition_version_id=wi.definition_version_id AND t.source_node_id=sv.node_id AND t.target_node_id=tv.node_id
 LEFT JOIN workflow_submissions s ON s.submission_id=e.submission_id AND s.workflow_instance_id=wi.workflow_instance_id AND s.source_node_visit_id=e.source_node_visit_id AND s.context_revision_id=e.context_revision_id AND s.author_principal_id=e.actor_principal_id AND s.transition_id=t.transition_id
 WHERE e.workflow_instance_id=$1 AND e.event_id=$2 AND e.event_type IN ('WORKFLOW_TRANSITION_COMMITTED','WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED') AND sv.assignee_principal_id=e.actor_principal_id AND e.from_node_id=sv.node_id AND e.to_node_id=tv.node_id")
 .bind(child).bind(event).fetch_optional(&mut **tx).await.map_err(storage)?.ok_or_else(conflict)
}
fn proof(b: &Binding, f: &TerminalFacts) -> Result<Value, AssistanceError> {
    if f.definition_version_id != b.child_definition_version_id {
        return Err(conflict());
    }
    let mut p = json!({"terminalEventId":f.event_id,"nodeVisitId":f.source_node_visit_id,"terminalNodeVisitId":f.target_node_visit_id,"definitionVersionId":f.definition_version_id,"contextRevisionId":f.context_revision_id,"transitionId":f.transition_id,"acceptanceContractDigest":b.acceptance_contract_digest,"submittedByPrincipalId":f.actor_principal_id});
    if f.transition_id == b.acceptance_transition_id {
        let a = acceptance(b, f.payload.as_ref())?;
        let id = f.submission_id.ok_or_else(conflict)?;
        let dig = f.payload_digest.as_ref().ok_or_else(conflict)?;
        let actual = digest::compute_json_digest(f.payload.as_ref().ok_or_else(conflict)?)
            .map_err(AssistanceError::InternalConsistency)?;
        if actual != *dig {
            return Err(conflict());
        }
        p["submissionId"] = json!(id);
        p["submissionDigest"] = json!(dig);
        p["verdict"] = serde_json::to_value(a.verdict)
            .map_err(|e| AssistanceError::InternalConsistency(e.to_string()))?;
    }
    Ok(p)
}
pub(crate) async fn record_terminal(
    tx: &mut Transaction<'_, Postgres>,
    child: Uuid,
    event: Uuid,
) -> Result<(), AssistanceError> {
    super::super::super::owner_obligation::intents::completed(tx, child, event).await?;
    let Some(b) = lock_child_binding(tx, child).await? else {
        return Ok(());
    };
    let f = facts(tx, child, event).await?;
    let p = proof(&b, &f)?;
    if b.terminal_event_id.is_some() {
        return if b.terminal_event_id == Some(event) && b.terminal_proof.as_ref() == Some(&p) {
            Ok(())
        } else {
            Err(conflict())
        };
    }
    sqlx::query("UPDATE workflow_governance_child_bindings SET terminal_event_id=$2,terminal_proof=$3 WHERE binding_id=$1 AND terminal_event_id IS NULL").bind(b.binding_id).bind(event).bind(&p).execute(&mut **tx).await.map_err(storage)?;
    let b = by_child(tx, child).await?.ok_or_else(conflict)?;
    review::queue_current(tx, &b, None).await?;
    Ok(())
}
pub(crate) async fn validate_resolve(
    tx: &mut Transaction<'_, Postgres>,
    case: Uuid,
    payload: &Value,
) -> Result<Option<Value>, AssistanceError> {
    let child:Option<Uuid>=sqlx::query_scalar("SELECT governance_child_instance_id FROM workflow_governance_child_bindings WHERE assistance_case_id=$1").bind(case).fetch_optional(&mut **tx).await.map_err(storage)?;
    let Some(child) = child else { return Ok(None) };
    lock_instance(tx, child).await?;
    let b = by_child(tx, child).await?.ok_or_else(conflict)?;
    if !source_valid(tx, &b).await? {
        return Err(conflict());
    }
    let r: GovernanceChildReview = serde_json::from_value(
        payload
            .pointer("/supportingPayload/governanceChildReview")
            .cloned()
            .ok_or_else(|| invalid("linked resolve requires governanceChildReview"))?,
    )
    .map_err(|_| invalid("governanceChildReview must be a closed exact reference"))?;
    let event = b.terminal_event_id.ok_or_else(conflict)?;
    let f = facts(tx, child, event).await?;
    let p = proof(&b, &f)?;
    let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_instances WHERE workflow_instance_id=$1 AND definition_version_id=$2 AND current_node_visit_id=$3 AND cancelled=FALSE AND archived_at IS NULL)").bind(child).bind(b.child_definition_version_id).bind(f.target_node_visit_id).fetch_one(&mut **tx).await.map_err(storage)?;
    if !current
        || b.terminal_proof.as_ref() != Some(&p)
        || f.transition_id != b.acceptance_transition_id
        || p["verdict"] != "ACCEPTED"
        || r.binding_id != b.binding_id
        || r.terminal_event_id != event
        || Some(r.submission_id) != f.submission_id
        || Some(&r.submission_digest) != f.payload_digest.as_ref()
        || r.acceptance_contract_digest != b.acceptance_contract_digest
    {
        return Err(conflict());
    }
    Ok(payload
        .pointer("/supportingPayload/governanceChildReview")
        .cloned())
}

pub(super) async fn verified_proof(
    tx: &mut Transaction<'_, Postgres>,
    b: &Binding,
) -> Result<Value, AssistanceError> {
    let f = facts(
        tx,
        b.governance_child_instance_id,
        b.terminal_event_id.ok_or_else(conflict)?,
    )
    .await?;
    proof(b, &f)
}
