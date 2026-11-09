//! Sidecar responsibility producers, always inside their real originating command.
use crate::domain::workflow_instance::assistance::AssistanceError;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
fn storage(e: sqlx::Error) -> AssistanceError {
    AssistanceError::StorageError(e.to_string())
}
pub(crate) async fn request(
    tx: &mut Transaction<'_, Postgres>,
    case: Uuid,
    command: Uuid,
) -> Result<bool, AssistanceError> {
    let source: Uuid = sqlx::query_scalar(
        "SELECT workflow_instance_id FROM workflow_assistance_cases WHERE assistance_case_id=$1",
    )
    .bind(case)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;
    let parent:Option<Uuid>=sqlx::query_scalar("SELECT parent_managed_instance_id FROM workflow_owner_dependency_origins WHERE created_instance_id=$1").bind(source).fetch_optional(&mut **tx).await.map_err(storage)?;
    if let Some(parent) = parent {
        let visit: Uuid = sqlx::query_scalar(
            "SELECT node_visit_id FROM workflow_assistance_cases WHERE assistance_case_id=$1",
        )
        .bind(case)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
        let event: Option<Uuid> = sqlx::query_scalar(
            "SELECT event_id FROM workflow_events WHERE command_id=$1 AND workflow_instance_id=$2",
        )
        .bind(command)
        .bind(source)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?;
        write_stop(
            tx,
            parent,
            Some(visit),
            "NESTED_ASSISTANCE_UNSUPPORTED",
            Some(command),
            None,
            event,
            None,
        )
        .await?;
        return Ok(true);
    }
    let managed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_managed_instances WHERE workflow_instance_id=$1)").bind(source).fetch_one(&mut **tx).await.map_err(storage)?;
    if managed {
        stop(
            tx,
            source,
            "NESTED_ASSISTANCE_UNSUPPORTED",
            Some(command),
            None,
            None,
            None,
        )
        .await?;
        return Ok(true);
    }
    let n=sqlx::query("INSERT INTO workflow_owner_obligation_intents(intent_id,kind,case_id,source_instance_id,source_node_visit_id,request_event_id,request_context_revision_id,source_definition_version_id,request_digest,policy_digest,policy_source_version_id,selected_definition_version_id,selected_transition_id,state) SELECT $2,'INTAKE',c.assistance_case_id,wi.workflow_instance_id,c.node_visit_id,e.event_id,e.context_revision_id,wi.definition_version_id,c.request_payload_digest,a.policy_digest,a.source_definition_version_id,a.intake_definition_version_id,a.intake_acceptance_transition_id,'PENDING' FROM workflow_assistance_cases c JOIN workflow_instances wi USING(workflow_instance_id) JOIN workflow_owner_policy_approvals a ON a.source_definition_version_id=wi.definition_version_id JOIN workflow_events e ON e.command_id=c.request_command_id AND e.event_type='ASSISTANCE_REQUESTED' AND e.workflow_instance_id=wi.workflow_instance_id WHERE c.assistance_case_id=$1 ON CONFLICT DO NOTHING").bind(case).bind(Uuid::new_v4()).execute(&mut **tx).await.map_err(storage)?.rows_affected();
    Ok(n > 0)
}
pub(crate) async fn terminal(
    tx: &mut Transaction<'_, Postgres>,
    binding: Uuid,
    event: Uuid,
) -> Result<bool, AssistanceError> {
    let n=sqlx::query("INSERT INTO workflow_owner_obligation_intents(intent_id,kind,case_id,source_instance_id,source_node_visit_id,request_event_id,request_context_revision_id,source_definition_version_id,request_digest,policy_digest,policy_source_version_id,selected_definition_version_id,selected_transition_id,binding_id,child_terminal_event_id,state) SELECT $3,'REVIEW',b.assistance_case_id,b.source_instance_id,b.source_node_visit_id,b.request_event_id,b.request_context_revision_id,b.source_definition_version_id,b.request_payload_digest,a.policy_digest,a.source_definition_version_id,a.review_definition_version_id,a.review_completion_transition_id,b.binding_id,$2,'PENDING' FROM workflow_governance_child_bindings b JOIN workflow_owner_managed_instances m ON m.binding_id=b.binding_id AND m.kind='INTAKE' JOIN workflow_owner_policy_approvals a ON a.source_definition_version_id=m.policy_source_version_id WHERE b.binding_id=$1 AND b.terminal_event_id=$2 ON CONFLICT DO NOTHING").bind(binding).bind(event).bind(Uuid::new_v4()).execute(&mut **tx).await.map_err(storage)?.rows_affected();
    if n > 0 {
        return Ok(true);
    }
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflow_owner_managed_instances WHERE binding_id=$1)",
    )
    .bind(binding)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn stop(
    tx: &mut Transaction<'_, Postgres>,
    managed: Uuid,
    code: &str,
    command: Option<Uuid>,
    replacement: Option<Uuid>,
    event: Option<Uuid>,
    attempt: Option<&str>,
) -> Result<(), AssistanceError> {
    let origin_visit: Option<Uuid> = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(managed)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    .flatten();
    write_stop(
        tx,
        managed,
        origin_visit,
        code,
        command,
        replacement,
        event,
        attempt,
    )
    .await
}
#[allow(clippy::too_many_arguments)]
async fn write_stop(
    tx: &mut Transaction<'_, Postgres>,
    managed: Uuid,
    origin_visit: Option<Uuid>,
    code: &str,
    command: Option<Uuid>,
    replacement: Option<Uuid>,
    event: Option<Uuid>,
    attempt: Option<&str>,
) -> Result<(), AssistanceError> {
    sqlx::query("INSERT INTO workflow_owner_obligation_intents(intent_id,kind,case_id,source_instance_id,source_node_visit_id,request_event_id,request_context_revision_id,source_definition_version_id,request_digest,policy_digest,policy_source_version_id,binding_id,managed_instance_id,state,outcome,stop_code,last_command_id,owner_replacement_command_id,origin_event_id,node_visit_id,last_attempt_id) SELECT $2,'STOP',case_id,source_instance_id,source_node_visit_id,request_event_id,request_context_revision_id,source_definition_version_id,request_digest,policy_digest,policy_source_version_id,binding_id,managed_instance_id,'STOPPED',$3,$3,$4,$5,$6,$7,$8 FROM workflow_owner_obligation_intents WHERE managed_instance_id=$1 AND kind IN ('INTAKE','REVIEW') AND state NOT IN ('STOPPED','RETIRED','COMPLETED') ON CONFLICT DO NOTHING").bind(managed).bind(Uuid::new_v4()).bind(code).bind(command).bind(replacement).bind(event).bind(origin_visit).bind(attempt).execute(&mut **tx).await.map_err(storage)?;
    sqlx::query("UPDATE workflow_owner_obligation_intents SET state='STOPPED',outcome=$2,stop_code=$2,last_command_id=$3,owner_replacement_command_id=$4,origin_event_id=$5,last_attempt_id=$6,node_visit_id=$7,updated_at=now() WHERE managed_instance_id=$1 AND state NOT IN ('STOPPED','RETIRED','COMPLETED')").bind(managed).bind(code).bind(command).bind(replacement).bind(event).bind(attempt).bind(origin_visit).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}
pub(crate) async fn owner_changed(
    tx: &mut Transaction<'_, Postgres>,
    domain: Uuid,
    command: Uuid,
) -> Result<(), AssistanceError> {
    let ids:Vec<Uuid>=sqlx::query_scalar("SELECT i.managed_instance_id FROM workflow_owner_obligation_intents i JOIN workflow_instances w ON w.workflow_instance_id=i.source_instance_id WHERE w.domain_id=$1 AND i.kind IN ('INTAKE','REVIEW') AND i.managed_instance_id IS NOT NULL AND i.state NOT IN ('STOPPED','RETIRED','COMPLETED') ORDER BY i.managed_instance_id").bind(domain).fetch_all(&mut **tx).await.map_err(storage)?;
    for managed in ids {
        stop(
            tx,
            managed,
            "OWNER_CHANGED_UNSUPPORTED",
            Some(command),
            Some(command),
            None,
            None,
        )
        .await?;
    }

    sqlx::query("UPDATE workflow_owner_obligation_intents i SET state='STOPPED',outcome='OWNER_CHANGED_UNSUPPORTED',stop_code='OWNER_CHANGED_UNSUPPORTED',last_command_id=$2,owner_replacement_command_id=$2,updated_at=now() FROM workflow_instances wi WHERE wi.workflow_instance_id=i.source_instance_id AND wi.domain_id=$1 AND i.state NOT IN ('STOPPED','RETIRED','COMPLETED')").bind(domain).bind(command).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}
/// Only the actual two internal AUTO command receipts can bypass ordinary membership.
pub(crate) async fn policy_actor_authorized(
    tx: &mut Transaction<'_, Postgres>,
    command: Uuid,
    domain: Uuid,
    target: Uuid,
    actor: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_obligation_intents i JOIN workflow_owner_policy_approvals p ON p.source_definition_version_id=i.policy_source_version_id JOIN workflow_command_receipts r ON r.command_id=i.last_command_id JOIN principals a ON a.principal_id=p.configured_policy_actor_principal_id AND a.enabled AND a.principal_type='AGENT' WHERE r.command_id=$1 AND r.receipt_status='PROCESSING' AND r.principal_id=$4 AND p.configured_policy_actor_principal_id=$4 AND p.domain_id=$2 AND i.selected_definition_version_id=$3 AND i.state='PENDING' AND ((i.kind='INTAKE' AND r.command_type='AUTO_CREATE_AND_BIND_OWNER_INTAKE') OR (i.kind='REVIEW' AND r.command_type='AUTO_CREATE_OWNER_REVIEW')))").bind(command).bind(domain).bind(target).bind(actor).fetch_one(&mut **tx).await
}
/// Opted-in cases can be changed only by the actual STARTED effect command.
pub(crate) async fn require_effect(
    tx: &mut Transaction<'_, Postgres>,
    case: Uuid,
    command: Uuid,
) -> Result<(), AssistanceError> {
    require_effect_for_command(tx, case, Some(command)).await
}
/// Read-only callers supply no command provenance; only the real writer may
/// provide its actual STARTED effect command identity.
pub(crate) async fn require_effect_for_command(
    tx: &mut Transaction<'_, Postgres>,
    case: Uuid,
    command: Option<Uuid>,
) -> Result<(), AssistanceError> {
    let (managed,claimed):(bool,bool)=sqlx::query_as("SELECT EXISTS(SELECT 1 FROM workflow_owner_obligation_intents WHERE case_id=$1 AND kind='INTAKE'),EXISTS(SELECT 1 FROM workflow_operation_effects e JOIN workflow_owner_managed_instances m ON m.workflow_instance_id=e.managed_instance_id AND m.kind='REVIEW' WHERE e.case_id=$1 AND e.command_id=$2 AND e.state='STARTED')").bind(case).bind(command).fetch_one(&mut **tx).await.map_err(storage)?;
    if managed && !claimed {
        return Err(AssistanceError::GovernedOperationRequired);
    }
    Ok(())
}
pub(crate) async fn require_auto_bind(
    tx: &mut Transaction<'_, Postgres>,
    case: Uuid,
    command: Uuid,
) -> Result<(), AssistanceError> {
    let (managed,claimed):(bool,bool)=sqlx::query_as("SELECT EXISTS(SELECT 1 FROM workflow_owner_obligation_intents WHERE case_id=$1 AND kind='INTAKE'),EXISTS(SELECT 1 FROM workflow_owner_obligation_intents i JOIN workflow_command_receipts r ON r.command_id=i.last_command_id WHERE i.case_id=$1 AND i.kind='INTAKE' AND i.state='PENDING' AND r.command_id=$2 AND r.command_type='AUTO_CREATE_AND_BIND_OWNER_INTAKE' AND r.receipt_status='PROCESSING')").bind(case).bind(command).fetch_one(&mut **tx).await.map_err(storage)?;
    if managed && !claimed {
        return Err(AssistanceError::GovernedOperationRequired);
    }
    Ok(())
}
pub(crate) async fn completed(
    tx: &mut Transaction<'_, Postgres>,
    instance: Uuid,
    event: Uuid,
) -> Result<(), AssistanceError> {
    sqlx::query("UPDATE workflow_owner_obligation_intents SET state='COMPLETED',outcome='TERMINAL',origin_event_id=$2,updated_at=now() WHERE managed_instance_id=$1 AND kind IN ('INTAKE','REVIEW') AND state='MATERIALIZED'").bind(instance).bind(event).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}
/// Parent lifecycle producer; cleanup is a separate existing cancel command.
pub(crate) async fn retire(
    tx: &mut Transaction<'_, Postgres>,
    source: Uuid,
    command: Uuid,
    reason: &str,
) -> Result<(), AssistanceError> {
    sqlx::query("UPDATE workflow_owner_obligation_intents i SET state='RETIRED',outcome=$3,last_command_id=$2,updated_at=now() WHERE i.source_instance_id=$1 AND i.kind IN ('INTAKE','REVIEW') AND i.state NOT IN ('STOPPED','RETIRED','COMPLETED') AND NOT (i.kind='REVIEW' AND EXISTS(SELECT 1 FROM workflow_operation_effects e WHERE e.managed_instance_id=i.managed_instance_id AND e.command_id=$2 AND e.state IN ('STARTED','SUCCEEDED')))").bind(source).bind(command).bind(reason).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}
/// Existing WEC/RETURN escalation is the trusted producer of a bounded STOP.
pub(crate) async fn policy_escalation(
    tx: &mut Transaction<'_, Postgres>,
    instance: Uuid,
    visit: Uuid,
    command: Uuid,
    evidence: &serde_json::Value,
) -> Result<bool, AssistanceError> {
    let parent:Option<Uuid>=sqlx::query_scalar("SELECT parent_managed_instance_id FROM workflow_owner_dependency_origins WHERE created_instance_id=$1").bind(instance).fetch_optional(&mut **tx).await.map_err(storage)?;
    let managed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_managed_instances WHERE workflow_instance_id=$1)").bind(instance).fetch_one(&mut **tx).await.map_err(storage)?;
    if parent.is_none() && !managed {
        return Ok(false);
    }
    let target = parent.unwrap_or(instance);
    let reason = evidence["reason"].as_str().unwrap_or("");
    let code = if parent.is_some() {
        "NESTED_ASSISTANCE_UNSUPPORTED"
    } else if reason == "RETURN_POLICY_EXHAUSTED" {
        "RETURN_LIMIT_REACHED"
    } else {
        "ATTEMPT_LIMIT_REACHED"
    };
    let event:Option<Uuid>=sqlx::query_scalar("SELECT event_id FROM workflow_events WHERE command_id=$1 AND workflow_instance_id=$2 ORDER BY event_sequence DESC LIMIT 1").bind(command).bind(instance).fetch_optional(&mut **tx).await.map_err(storage)?;
    write_stop(
        tx,
        target,
        Some(visit),
        code,
        Some(command),
        None,
        event,
        evidence["lastAttemptId"].as_str(),
    )
    .await?;
    Ok(true)
}
/// Match the actual Owner-replacement Domain fence before instance locks.
pub(crate) async fn fence(
    tx: &mut Transaction<'_, Postgres>,
    instance: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT d.domain_id FROM workflow_instances w JOIN domains d USING(domain_id) WHERE w.workflow_instance_id=$1 AND (EXISTS(SELECT 1 FROM workflow_owner_policy_approvals p WHERE p.source_definition_version_id=w.definition_version_id) OR EXISTS(SELECT 1 FROM workflow_owner_managed_instances m WHERE m.workflow_instance_id=w.workflow_instance_id) OR EXISTS(SELECT 1 FROM workflow_owner_dependency_origins o WHERE o.created_instance_id=w.workflow_instance_id)) FOR UPDATE OF d").bind(instance).fetch_optional(&mut **tx).await?;
    Ok(())
}
/// Narrow existing cancel authorization: retired managed facts and the approved actor only.
pub(crate) async fn cleanup_authorized(
    tx: &mut Transaction<'_, Postgres>,
    instance: Uuid,
    actor: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_managed_instances m JOIN workflow_owner_policy_approvals p ON p.source_definition_version_id=m.policy_source_version_id JOIN workflow_owner_obligation_intents i ON i.managed_instance_id=m.workflow_instance_id AND i.kind=m.kind JOIN principals a ON a.principal_id=p.configured_policy_actor_principal_id AND a.enabled AND a.principal_type='AGENT' WHERE m.workflow_instance_id=$1 AND p.configured_policy_actor_principal_id=$2 AND i.state='RETIRED')").bind(instance).bind(actor).fetch_one(&mut **tx).await
}
