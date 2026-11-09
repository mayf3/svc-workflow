use super::*;

pub(crate) async fn bind(
    pool: &PgPool,
    c: BindGovernanceChildCommand,
    hash: &str,
) -> Result<Value, AssistanceError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    let actor = c.principal_id.into_uuid();
    let case = c.assistance_case_id.into_uuid();
    let child = c.governance_child_instance_id.into_uuid();
    let command = match acquire_receipt(
        &mut tx,
        actor,
        &c.idempotency_key,
        "BIND_GOVERNANCE_CHILD",
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
    let result = admit(&mut tx, &c, command, actor, case, child).await;
    match result {
        Ok(body) => {
            complete_receipt(&mut tx, command, 201, &body).await?;
            tx.commit().await.map_err(storage)?;
            Ok(body)
        }
        Err(e @ AssistanceError::StorageError(_)) => Err(e),
        Err(e @ AssistanceError::InternalConsistency(_)) => Err(e),
        Err(e) => {
            complete_receipt(&mut tx, command, e.status_code(), &error_body(&e)).await?;
            tx.commit().await.map_err(storage)?;
            Err(e)
        }
    }
}
pub(crate) async fn admit(
    tx: &mut Transaction<'_, Postgres>,
    c: &BindGovernanceChildCommand,
    command: Uuid,
    actor: Uuid,
    case: Uuid,
    child: Uuid,
) -> Result<Value, AssistanceError> {
    validate_actor_enabled(tx, actor).await?;
    let source: Uuid = sqlx::query_scalar(
        "SELECT workflow_instance_id FROM workflow_assistance_cases WHERE assistance_case_id=$1",
    )
    .bind(case)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    .ok_or(AssistanceError::AssistanceCaseNotFoundOrNotVisible)?;
    if source == child {
        return Err(conflict());
    }
    // All competing binders lock both instances in a deterministic UUID order.
    let mut ids = [source, child];
    ids.sort();
    for id in ids {
        lock_instance(tx, id).await?;
    }
    sqlx::query("SELECT nv.node_visit_id FROM workflow_node_visits nv JOIN workflow_instances wi ON wi.current_node_visit_id=nv.node_visit_id WHERE wi.workflow_instance_id=ANY($1) ORDER BY wi.workflow_instance_id FOR SHARE OF nv")
        .bind(&ids[..]).fetch_all(&mut **tx).await.map_err(storage)?;
    let src = lock_instance(tx, source).await?;
    let ch = lock_instance(tx, child).await?;
    if src.workflow_state_version != c.expected_workflow_state_version {
        return Err(AssistanceError::WorkflowStateVersionConflict {
            expected: c.expected_workflow_state_version,
            actual: src.workflow_state_version,
        });
    }
    if effective_owner(tx, src.domain_id).await? != actor {
        let target: Uuid = sqlx::query_scalar(
            "SELECT definition_version_id FROM workflow_instances WHERE workflow_instance_id=$1",
        )
        .bind(child)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
        if !crate::store::postgres::workflow_instance_repository::owner_obligation::intents::policy_actor_authorized(tx,command,src.domain_id,target,actor).await.map_err(storage)?{return Err(AssistanceError::NotDomainOwner)}
    }
    super::super::super::owner_obligation::intents::require_auto_bind(tx, case, command).await?;
    if src.domain_id != ch.domain_id
        || src.current_context_revision_id.is_none()
        || ch.current_node_visit_id.is_none()
        || src.cancelled
        || ch.cancelled
        || src.archived_at.is_some()
        || ch.archived_at.is_some()
        || src.node_type.as_deref() == Some("TERMINAL")
        || ch.node_type.as_deref() == Some("TERMINAL")
    {
        return Err(conflict());
    }
    let (status,requester,visit,request_command,payload_digest):(String,Uuid,Uuid,Uuid,String)=sqlx::query_as("SELECT status,requested_by_principal_id,node_visit_id,request_command_id,request_payload_digest FROM workflow_assistance_cases WHERE assistance_case_id=$1 FOR UPDATE").bind(case).fetch_one(&mut **tx).await.map_err(storage)?;
    if status != "OWNER_PENDING"
        || src.current_node_visit_id != Some(visit)
        || src.assignee_principal_id != Some(requester)
    {
        return Err(conflict());
    }
    let used:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_governance_child_bindings WHERE assistance_case_id=$1 OR governance_child_instance_id IN ($2,$3) OR source_instance_id=$3)").bind(case).bind(source).bind(child).fetch_one(&mut **tx).await.map_err(storage)?;
    if used {
        return Err(conflict());
    }
    // Read the request's exact event/Context; never substitute current Context.
    let events:Vec<(Uuid,Uuid,i32,Uuid)>=sqlx::query_as("SELECT e.event_id,e.context_revision_id,e.event_sequence,nd.definition_version_id FROM workflow_events e
 JOIN workflow_node_visits v ON v.node_visit_id=e.source_node_visit_id AND v.workflow_instance_id=e.workflow_instance_id
 JOIN workflow_node_definitions nd ON nd.node_id=v.node_id
 JOIN workflow_instances wi ON wi.workflow_instance_id=e.workflow_instance_id AND wi.definition_version_id=nd.definition_version_id
 JOIN workflow_context_revisions cr ON cr.context_revision_id=e.context_revision_id AND cr.workflow_instance_id=e.workflow_instance_id
 JOIN workflow_command_receipts r ON r.command_id=e.command_id
 WHERE e.command_id=$1 AND e.workflow_instance_id=$2 AND e.event_type='ASSISTANCE_REQUESTED'
 AND e.source_node_visit_id=$3 AND e.target_node_visit_id=$3 AND e.actor_principal_id=$4
 AND e.from_node_id=v.node_id AND e.to_node_id=v.node_id
 AND e.event_data->>'assistanceCaseId'=$5 AND e.event_data->>'payloadDigest'=$6
 AND r.principal_id=$4 AND r.command_type='REQUEST_WORKFLOW_ASSISTANCE' AND r.receipt_status='COMPLETED' AND r.response_status BETWEEN 200 AND 299")
 .bind(request_command).bind(source).bind(visit).bind(requester).bind(case.to_string()).bind(&payload_digest).fetch_all(&mut **tx).await.map_err(storage)?;
    if events.len() != 1 {
        return Err(conflict());
    }
    let (request_event, request_context, request_sequence, source_version) = events[0];
    let transition = c.acceptance_transition_id.into_uuid();
    let selected:Option<(Uuid,Value)>=sqlx::query_as("SELECT wi.definition_version_id,t.submission_schema FROM workflow_instances wi
 JOIN workflow_definition_versions dv ON dv.definition_version_id=wi.definition_version_id AND dv.version_status='PUBLISHED'
 JOIN workflow_transition_definitions t ON t.definition_version_id=dv.definition_version_id AND t.transition_id=$2
 JOIN workflow_node_definitions n ON n.node_id=t.target_node_id AND n.definition_version_id=dv.definition_version_id AND n.node_type='TERMINAL'
 WHERE wi.workflow_instance_id=$1 AND t.submission_schema IS NOT NULL FOR SHARE OF dv,t,n")
 .bind(child).bind(transition).fetch_optional(&mut **tx).await.map_err(storage)?;
    let (child_version, schema) = selected.ok_or_else(conflict)?;
    if !declares_acceptance(&schema) {
        return Err(invalid(
            "selected terminal transition must declare the closed governanceAcceptance schema",
        ));
    }
    let binding = Uuid::new_v4();
    let binding_event = Uuid::new_v4();
    let source_tuple = json!({"workflowInstanceId":source,"nodeVisitId":visit,"requestContextRevisionId":request_context,"definitionVersionId":source_version,"requestPayloadDigest":payload_digest,"requestCommandId":request_command,"requestEventSequence":request_sequence});
    let child_tuple = json!({"workflowInstanceId":child,"definitionVersionId":child_version,"contextRevisionAtBind":ch.current_context_revision_id.ok_or_else(conflict)?,"acceptanceTransitionId":transition});
    let contract_digest = digest::compute_json_digest(
        &json!({"source":source_tuple,"child":child_tuple,"submissionSchema":schema}),
    )
    .map_err(AssistanceError::InternalConsistency)?;
    let receipt = json!({"bindingId":binding,"assistanceCaseId":case,"source":source_tuple,"child":child_tuple,"acceptanceContractDigest":contract_digest,"status":"BOUND"});
    sqlx::query("INSERT INTO workflow_governance_child_bindings (binding_id,assistance_case_id,source_instance_id,source_node_visit_id,request_context_revision_id,source_definition_version_id,request_command_id,request_event_id,request_payload_digest,governance_child_instance_id,child_definition_version_id,child_context_revision_at_bind,acceptance_transition_id,acceptance_contract_digest,binding_principal_id,binding_command_id,binding_event_id,receipt)
 VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)")
 .bind(binding).bind(case).bind(source).bind(visit).bind(request_context).bind(source_version).bind(request_command).bind(request_event).bind(payload_digest).bind(child).bind(child_version).bind(ch.current_context_revision_id).bind(transition).bind(&contract_digest).bind(actor).bind(command).bind(binding_event).bind(&receipt).execute(&mut **tx).await.map_err(storage)?;
    // Same visit, one new version/event. The binding event carries its receipt.
    let new_version = src.workflow_state_version + 1;
    sqlx::query("UPDATE workflow_instances SET workflow_state_version=$2,updated_at=now() WHERE workflow_instance_id=$1 AND workflow_state_version=$3").bind(source).bind(new_version).bind(src.workflow_state_version).execute(&mut **tx).await.map_err(storage)?;
    let event_data = json!({"assistanceCaseId":case,"previousStatus":"OWNER_PENDING","newStatus":"OWNER_PENDING","payloadDigest":contract_digest,"governanceBinding":receipt});
    let event_digest =
        digest::compute_json_digest(&event_data).map_err(AssistanceError::InternalConsistency)?;
    sqlx::query("INSERT INTO workflow_events (event_id,workflow_instance_id,event_sequence,event_schema_version,command_id,event_type,source_node_visit_id,target_node_visit_id,context_revision_id,event_data,event_data_digest,actor_principal_id,from_node_id,to_node_id,old_workflow_state_version,new_workflow_state_version)
 VALUES($1,$2,$3,'v1',$4,'GOVERNANCE_CHILD_BOUND',$5,$5,$6,$7,$8,$9,$10,$10,$11,$3)")
 .bind(binding_event).bind(source).bind(new_version).bind(command).bind(visit).bind(src.current_context_revision_id).bind(event_data).bind(event_digest).bind(actor).bind(src.node_id).bind(src.workflow_state_version).execute(&mut **tx).await.map_err(storage)?;
    Ok(receipt)
}
