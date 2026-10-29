//! GCRS relational provenance, serialized admission, terminal proof and review.
use super::*;
use crate::domain::workflow_instance::governance_child::{
    declares_acceptance, digest_shape, BindGovernanceChildCommand, GovernanceAcceptance,
    GovernanceChildReview, Verdict,
};
use serde_json::{json, Value};

mod binding;
mod create_bind;
pub(crate) use create_bind::create_and_bind;
mod review;
mod terminal;
pub(crate) use binding::admit as bind_on_tx;
pub(crate) use binding::bind;
pub(crate) use review::{prepare_delivery, queue_owner_replacement};
pub(crate) use terminal::{record_terminal, validate_resolve, validate_selected_submission};

#[derive(Debug, FromRow)]
pub(super) struct Binding {
    binding_id: Uuid,
    assistance_case_id: Uuid,
    source_instance_id: Uuid,
    source_node_visit_id: Uuid,
    request_context_revision_id: Uuid,
    source_definition_version_id: Uuid,
    request_command_id: Uuid,
    request_event_id: Uuid,
    request_payload_digest: String,
    governance_child_instance_id: Uuid,
    child_definition_version_id: Uuid,
    acceptance_transition_id: Uuid,
    acceptance_contract_digest: String,
    receipt: Value,
    terminal_event_id: Option<Uuid>,
    terminal_proof: Option<Value>,
}
async fn by_child(
    tx: &mut Transaction<'_, Postgres>,
    child: Uuid,
) -> Result<Option<Binding>, AssistanceError> {
    sqlx::query_as("SELECT * FROM workflow_governance_child_bindings WHERE governance_child_instance_id=$1 FOR UPDATE").bind(child).fetch_optional(&mut **tx).await.map_err(storage)
}
fn invalid(detail: &str) -> AssistanceError {
    AssistanceError::InvalidPayload(detail.into())
}
fn conflict() -> AssistanceError {
    AssistanceError::AssistanceStatusConflict
}

/// Verify the original request and its Context through immutable service facts.
async fn source_valid(
    tx: &mut Transaction<'_, Postgres>,
    b: &Binding,
) -> Result<bool, AssistanceError> {
    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_assistance_cases c
 JOIN workflow_instances wi ON wi.workflow_instance_id=c.workflow_instance_id
 JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id AND v.workflow_instance_id=wi.workflow_instance_id
 JOIN workflow_node_definitions nd ON nd.node_id=v.node_id AND nd.definition_version_id=wi.definition_version_id
 JOIN workflow_events e ON e.event_id=$2 AND e.command_id=c.request_command_id
 JOIN workflow_context_revisions cr ON cr.context_revision_id=e.context_revision_id AND cr.workflow_instance_id=wi.workflow_instance_id
 JOIN workflow_command_receipts r ON r.command_id=e.command_id
 WHERE c.assistance_case_id=$1 AND c.status IN ('OWNER_PENDING','HUMAN_REQUIRED')
 AND wi.workflow_instance_id=$3 AND wi.current_node_visit_id=$4 AND wi.definition_version_id=$5
 AND wi.cancelled=FALSE AND wi.archived_at IS NULL AND nd.node_type<>'TERMINAL'
 AND v.assignee_principal_id=c.requested_by_principal_id
 AND c.request_command_id=$6 AND c.request_payload_digest=$7
 AND e.context_revision_id=$8 AND e.workflow_instance_id=wi.workflow_instance_id
 AND e.source_node_visit_id=c.node_visit_id AND e.target_node_visit_id=c.node_visit_id
 AND e.from_node_id=v.node_id AND e.to_node_id=v.node_id
 AND e.actor_principal_id=c.requested_by_principal_id AND e.event_type='ASSISTANCE_REQUESTED'
 AND e.event_data->>'assistanceCaseId'=c.assistance_case_id::text AND e.event_data->>'payloadDigest'=c.request_payload_digest
 AND r.command_type='REQUEST_WORKFLOW_ASSISTANCE' AND r.principal_id=c.requested_by_principal_id
 AND r.receipt_status='COMPLETED' AND r.response_status BETWEEN 200 AND 299)")
 .bind(b.assistance_case_id).bind(b.request_event_id).bind(b.source_instance_id).bind(b.source_node_visit_id).bind(b.source_definition_version_id).bind(b.request_command_id).bind(&b.request_payload_digest).bind(b.request_context_revision_id).fetch_one(&mut **tx).await.map_err(storage)?;
    Ok(valid)
}
/// Project the immutable receipt plus the current recipient's delivery state.
pub(crate) async fn detail(
    pool: &PgPool,
    case: Uuid,
) -> Result<Option<(Value, Option<Value>)>, AssistanceError> {
    let row:Option<(Value,Option<Value>,Option<Value>)>=sqlx::query_as("SELECT b.receipt,b.terminal_proof,
 (SELECT jsonb_build_object('reviewWakeId',r.review_wake_id,'ownerPrincipalId',r.owner_principal_id,'ownerBindingId',r.owner_binding_id,
 'deliveryStatus',CASE WHEN r.retired_reason IS NOT NULL THEN 'RETIRED' WHEN o.delivered_at IS NOT NULL THEN 'DELIVERED' ELSE 'PENDING' END,
 'reviewStatus',CASE WHEN c.status='RESOLVED' THEN 'RESOLVED' WHEN c.status='HUMAN_REQUIRED' THEN 'ESCALATED' WHEN c.status='VOIDED' THEN 'VOIDED' ELSE 'OUTSTANDING' END)
 FROM workflow_governance_child_reviews r JOIN workflow_outbox o USING(outbox_id)
 JOIN workflow_assistance_cases c ON c.assistance_case_id=b.assistance_case_id
 WHERE r.binding_id=b.binding_id ORDER BY (r.retired_reason IS NULL) DESC,r.created_at DESC,r.review_wake_id DESC LIMIT 1)
 FROM workflow_governance_child_bindings b WHERE b.assistance_case_id=$1").bind(case).fetch_optional(pool).await.map_err(storage)?;
    Ok(row.map(|(mut receipt, proof, review)| {
        if let Some(r) = review {
            receipt["currentReview"] = r;
        }
        (receipt, proof)
    }))
}
/// Read-only managed REVIEW proof producer; no invented wake/delivery fact.
pub(crate) async fn managed_review_proof(
    tx: &mut Transaction<'_, Postgres>,
    case: Uuid,
    binding: Uuid,
    terminal: Option<Uuid>,
) -> Result<Option<Value>, AssistanceError> {
    let b:Binding=sqlx::query_as("SELECT * FROM workflow_governance_child_bindings WHERE binding_id=$1 AND assistance_case_id=$2").bind(binding).bind(case).fetch_optional(&mut **tx).await.map_err(storage)?.ok_or_else(conflict)?;
    if b.terminal_event_id != terminal || !source_valid(tx, &b).await? {
        return Err(conflict());
    }
    let p = terminal::verified_proof(tx, &b).await?;
    if b.terminal_proof.as_ref() != Some(&p) {
        return Err(conflict());
    }
    let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_instances WHERE workflow_instance_id=$1 AND current_node_visit_id=$2 AND NOT cancelled AND archived_at IS NULL)").bind(b.governance_child_instance_id).bind(p["terminalNodeVisitId"].as_str().and_then(|s|s.parse::<Uuid>().ok())).fetch_one(&mut **tx).await.map_err(storage)?;
    if !current {
        return Err(conflict());
    }
    if p["verdict"] == "NOT_ACCEPTED" {
        return Ok(None);
    }
    if p["verdict"] != "ACCEPTED" {
        return Err(conflict());
    }
    Ok(Some(
        json!({"bindingId":b.binding_id,"terminalEventId":p["terminalEventId"],"submissionId":p["submissionId"],"submissionDigest":p["submissionDigest"],"acceptanceContractDigest":b.acceptance_contract_digest}),
    ))
}
