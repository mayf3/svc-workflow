use super::*;
use crate::domain::workflow_instance::governance_child::GovernanceReviewWake;
use sha2::{Digest, Sha256};

async fn owner_coordinate(
    tx: &mut Transaction<'_, Postgres>,
    domain: Uuid,
) -> Result<(Uuid, Uuid), AssistanceError> {
    let owner = effective_owner(tx, domain).await?;
    let binding=sqlx::query_scalar("SELECT binding_id FROM domain_role_bindings WHERE domain_id=$1 AND principal_id=$2 AND role_key='DOMAIN_OWNER' AND enabled=TRUE").bind(domain).bind(owner).fetch_one(&mut **tx).await.map_err(storage)?;
    Ok((owner, binding))
}
// A replacement command participates even when an old role binding is reactivated.
fn wake_id(binding: Uuid, event: Uuid, owner: Uuid, command: Option<Uuid>) -> Uuid {
    let digest = Sha256::digest(
        format!("governance-review:{binding}:{event}:{owner}:{command:?}").as_bytes(),
    );
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}
pub(super) async fn queue_current(
    tx: &mut Transaction<'_, Postgres>,
    b: &Binding,
    replacement: Option<Uuid>,
) -> Result<(), AssistanceError> {
    let Some(event) = b.terminal_event_id else {
        return Ok(());
    };
    let (domain,class):(Uuid,String)=sqlx::query_as("SELECT domain_id,execution_class::text FROM workflow_instances WHERE workflow_instance_id=$1").bind(b.source_instance_id).fetch_one(&mut **tx).await.map_err(storage)?;
    if class != "BUSINESS" || !source_valid(tx, b).await? {
        return Ok(());
    };
    let (owner, owner_binding) = owner_coordinate(tx, domain).await?;
    let id = wake_id(b.binding_id, event, owner, replacement);
    let p = b.terminal_proof.as_ref().ok_or_else(conflict)?;
    let mut payload = json!({"phase":"GOVERNANCE_CHILD_REVIEW","workflowInstanceId":b.source_instance_id,"nodeVisitId":b.source_node_visit_id,"assistanceCaseId":b.assistance_case_id,"ownerPrincipalId":owner,"ownerBindingId":owner_binding,"reviewWakeId":id,"bindingId":b.binding_id,"governanceChildInstanceId":b.governance_child_instance_id,"childNodeVisitId":p["nodeVisitId"],"childDefinitionVersionId":b.child_definition_version_id,"childTerminalEventId":event,"acceptanceTransitionId":b.acceptance_transition_id,"acceptanceContractDigest":b.acceptance_contract_digest});
    if let (Some(sub), Some(dig)) = (p.get("submissionId"), p.get("submissionDigest")) {
        payload["childSubmissionId"] = sub.clone();
        payload["childSubmissionDigest"] = dig.clone();
    }
    GovernanceReviewWake::parse(&payload).map_err(|e| invalid(&e))?;
    let outbox = Uuid::new_v4();
    let key = format!(
        "owner-assistance-review:{}:{event}:{owner}:{id}",
        b.binding_id
    );
    let actual:Uuid=sqlx::query_scalar("INSERT INTO workflow_outbox(outbox_id,workflow_instance_id,outbox_kind,event_key,payload) VALUES($1,$2,'OWNER_ASSISTANCE_WAKE',$3,$4) ON CONFLICT(outbox_kind,event_key) DO UPDATE SET event_key=EXCLUDED.event_key RETURNING outbox_id").bind(outbox).bind(b.source_instance_id).bind(key).bind(payload).fetch_one(&mut **tx).await.map_err(storage)?;
    sqlx::query("INSERT INTO workflow_governance_child_reviews(review_wake_id,binding_id,terminal_event_id,owner_principal_id,owner_binding_id,owner_replacement_command_id,outbox_id) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(review_wake_id) DO NOTHING").bind(id).bind(b.binding_id).bind(event).bind(owner).bind(owner_binding).bind(replacement).bind(actual).execute(&mut **tx).await.map_err(storage)?;
    // A newer assignment retires pending and already-delivered historical recipients.
    sqlx::query("UPDATE workflow_governance_child_reviews SET retired_reason='OWNER_REPLACED' WHERE binding_id=$1 AND review_wake_id<>$2 AND retired_reason IS NULL").bind(b.binding_id).bind(id).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}
/// Called inside the actual successful Owner-replacement transaction, with its
/// domain FOR UPDATE guard still held. No-op replacement never calls this.
pub(crate) async fn queue_owner_replacement(
    tx: &mut Transaction<'_, Postgres>,
    domain: Uuid,
    command: Uuid,
) -> Result<(), AssistanceError> {
    let bindings:Vec<Binding>=sqlx::query_as("SELECT b.* FROM workflow_governance_child_bindings b JOIN workflow_instances wi ON wi.workflow_instance_id=b.source_instance_id JOIN workflow_assistance_cases c ON c.assistance_case_id=b.assistance_case_id WHERE wi.domain_id=$1 AND b.terminal_event_id IS NOT NULL AND c.status IN ('OWNER_PENDING','HUMAN_REQUIRED') ORDER BY b.binding_id FOR UPDATE OF b").bind(domain).fetch_all(&mut **tx).await.map_err(storage)?;
    for b in bindings {
        queue_current(tx, &b, Some(command)).await?;
    }
    Ok(())
}
/// Re-read all authoritative coordinates immediately before the HTTP send.
/// False means the durable row was retired with a structured reason; it must
/// not be acknowledged as completed review responsibility.
pub(crate) async fn prepare_delivery(
    pool: &PgPool,
    outbox: Uuid,
    payload: &Value,
) -> Result<bool, AssistanceError> {
    let parsed = GovernanceReviewWake::parse(payload).map_err(|e| invalid(&e))?;
    let mut tx = pool.begin().await.map_err(storage)?;
    let binding = payload["bindingId"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| invalid("invalid review binding identity"))?;
    let domain:Uuid=sqlx::query_scalar("SELECT wi.domain_id FROM workflow_governance_child_bindings b JOIN workflow_instances wi ON wi.workflow_instance_id=b.source_instance_id WHERE b.binding_id=$1").bind(binding).fetch_one(&mut *tx).await.map_err(storage)?;
    let (owner, owner_binding) = owner_coordinate(&mut tx, domain).await?;
    let b: Binding = sqlx::query_as(
        "SELECT * FROM workflow_governance_child_bindings WHERE binding_id=$1 FOR UPDATE",
    )
    .bind(binding)
    .fetch_one(&mut *tx)
    .await
    .map_err(storage)?;
    let review:Option<(Uuid,Option<String>)>=sqlx::query_as("SELECT review_wake_id,retired_reason FROM workflow_governance_child_reviews WHERE outbox_id=$1 AND binding_id=$2").bind(outbox).bind(binding).fetch_optional(&mut *tx).await.map_err(storage)?;
    let (wake, retired) = review.ok_or_else(conflict)?;
    let mut reason = retired;
    if reason.is_none() && !source_valid(&mut tx, &b).await? {
        reason = Some("STALE_SOURCE_OR_CASE".into());
    }
    if reason.is_none()
        && (payload["childTerminalEventId"] != json!(b.terminal_event_id)
            || payload["governanceChildInstanceId"] != json!(b.governance_child_instance_id)
            || payload["reviewWakeId"] != json!(wake))
    {
        reason = Some("STALE_BINDING_OR_TERMINAL".into());
    }
    if reason.is_none()
        && (parsed.workflow_instance_id != b.source_instance_id
            || parsed.node_visit_id != b.source_node_visit_id
            || parsed.assistance_case_id != b.assistance_case_id
            || parsed.child_definition_version_id != b.child_definition_version_id
            || parsed.acceptance_transition_id != b.acceptance_transition_id
            || parsed.acceptance_contract_digest != b.acceptance_contract_digest)
    {
        reason = Some("STALE_BINDING_OR_TERMINAL".into());
    }
    if reason.is_none() {
        let proof = terminal::verified_proof(&mut tx, &b).await?;
        if b.terminal_proof.as_ref() != Some(&proof)
            || payload["childNodeVisitId"] != proof["nodeVisitId"]
            || payload.get("childSubmissionId") != proof.get("submissionId")
            || payload.get("childSubmissionDigest") != proof.get("submissionDigest")
        {
            reason = Some("STALE_TERMINAL_PROOF".into());
        }
    }
    if reason.is_none() {
        // Rebuild exact proof from immutable event/submission provenance.
        let event_exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_events e JOIN workflow_node_visits v ON v.node_visit_id=e.target_node_visit_id JOIN workflow_node_definitions n ON n.node_id=v.node_id AND n.node_type='TERMINAL' WHERE e.event_id=$1 AND e.workflow_instance_id=$2 AND e.context_revision_id=($3->>'contextRevisionId')::uuid AND e.source_node_visit_id=($3->>'nodeVisitId')::uuid)").bind(b.terminal_event_id).bind(b.governance_child_instance_id).bind(&b.terminal_proof).fetch_one(&mut *tx).await.map_err(storage)?;
        if !event_exists {
            reason = Some("STALE_TERMINAL".into());
        }
    }
    if reason.is_none()
        && (payload["ownerPrincipalId"] != json!(owner)
            || payload["ownerBindingId"] != json!(owner_binding))
    {
        // Normally the provisioning hook already queued this assignment. Reconcile
        // an older pending row to the latest successful replacement command.
        let command:Option<Uuid>=sqlx::query_scalar("SELECT command_id FROM workflow_command_receipts WHERE command_type='PROVISION_REPLACE_OWNER' AND receipt_status='COMPLETED' AND response_status BETWEEN 200 AND 299 AND response_body->>'domainId'=$1 AND response_body->>'newOwnerId'=$2 ORDER BY completed_at DESC,command_id DESC LIMIT 1").bind(domain.to_string()).bind(owner.to_string()).fetch_optional(&mut *tx).await.map_err(storage)?;
        queue_current(&mut tx, &b, command).await?;
        reason = Some("OWNER_REPLACED".into());
    }
    if let Some(reason) = reason {
        sqlx::query(
            "UPDATE workflow_governance_child_reviews SET retired_reason=$2 WHERE outbox_id=$1",
        )
        .bind(outbox)
        .bind(&reason)
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
        sqlx::query("UPDATE workflow_outbox SET delivered_at=now(),last_error=$2 WHERE outbox_id=$1 AND delivered_at IS NULL").bind(outbox).bind(json!({"suppressed":reason}).to_string()).execute(&mut *tx).await.map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        return Ok(false);
    }
    tx.commit().await.map_err(storage)?;
    Ok(true)
}
