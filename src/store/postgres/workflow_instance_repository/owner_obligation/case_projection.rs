//! Existing current-Owner case reads only. Never infer UNKNOWN from a failed read.
use crate::domain::workflow_instance::assistance::AssistanceError;
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
fn unavailable() -> AssistanceError {
    AssistanceError::StorageError("Owner responsibility lineage unavailable".into())
}
fn storage(e: sqlx::Error) -> AssistanceError {
    AssistanceError::StorageError(e.to_string())
}

pub(crate) async fn project(
    tx: &mut Transaction<'_, Postgres>,
    case: Uuid,
) -> Result<Option<Value>, AssistanceError> {
    let (marked,approved):(bool,bool)=sqlx::query_as("SELECT COALESCE(v.context_schema ? 'x-workflow-owner-obligation-v1',FALSE),EXISTS(SELECT 1 FROM workflow_owner_policy_approvals p WHERE p.source_definition_version_id=v.definition_version_id) FROM workflow_assistance_cases c JOIN workflow_instances w ON w.workflow_instance_id=c.workflow_instance_id JOIN workflow_definition_versions v ON v.definition_version_id=w.definition_version_id WHERE c.assistance_case_id=$1")
        .bind(case).fetch_optional(&mut **tx).await.map_err(storage)?.ok_or_else(unavailable)?;
    if marked != approved {
        return Err(unavailable());
    }
    if !marked {
        return Ok(None);
    }
    let rows:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(i)||jsonb_build_object('_effectId',(SELECT e.effect_id FROM workflow_operation_effects e WHERE e.command_id=i.last_command_id),'_valid',EXISTS(SELECT 1 FROM workflow_assistance_cases c JOIN workflow_instances w ON w.workflow_instance_id=c.workflow_instance_id JOIN workflow_owner_policy_approvals p ON p.source_definition_version_id=w.definition_version_id JOIN workflow_events x ON x.event_id=i.request_event_id AND x.command_id=c.request_command_id JOIN workflow_command_receipts r ON r.command_id=c.request_command_id WHERE c.assistance_case_id=i.case_id AND i.source_instance_id=w.workflow_instance_id AND i.source_node_visit_id=c.node_visit_id AND i.source_definition_version_id=w.definition_version_id AND i.policy_source_version_id=p.source_definition_version_id AND i.policy_digest=p.policy_digest AND i.request_digest=c.request_payload_digest AND x.event_type='ASSISTANCE_REQUESTED' AND x.workflow_instance_id=w.workflow_instance_id AND x.source_node_visit_id=c.node_visit_id AND x.context_revision_id=i.request_context_revision_id AND r.receipt_status='COMPLETED' AND r.response_status BETWEEN 200 AND 299 AND r.command_type='REQUEST_WORKFLOW_ASSISTANCE' AND r.principal_id=c.requested_by_principal_id AND ((i.kind='INTAKE' AND i.selected_definition_version_id=p.intake_definition_version_id AND i.selected_transition_id=p.intake_acceptance_transition_id) OR (i.kind='REVIEW' AND i.selected_definition_version_id=p.review_definition_version_id AND i.selected_transition_id=p.review_completion_transition_id AND EXISTS(SELECT 1 FROM workflow_governance_child_bindings b WHERE b.binding_id=i.binding_id AND b.assistance_case_id=i.case_id AND b.terminal_event_id=i.child_terminal_event_id))) AND (i.managed_instance_id IS NULL OR EXISTS(SELECT 1 FROM workflow_owner_managed_instances m WHERE m.workflow_instance_id=i.managed_instance_id AND m.kind=i.kind AND m.case_id=i.case_id AND m.binding_id=i.binding_id AND m.policy_source_version_id=i.policy_source_version_id AND m.policy_digest=i.policy_digest)))) FROM workflow_owner_obligation_intents i WHERE i.case_id=$1 AND i.kind IN ('INTAKE','REVIEW') ORDER BY i.kind,i.intent_id")
        .bind(case).fetch_all(&mut **tx).await.map_err(storage)?;
    let review_expected:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_governance_child_bindings b JOIN workflow_owner_managed_instances m ON m.binding_id=b.binding_id AND m.kind='INTAKE' WHERE b.assistance_case_id=$1 AND b.terminal_event_id IS NOT NULL)")
        .bind(case).fetch_one(&mut **tx).await.map_err(storage)?;
    if rows.iter().filter(|r| r["kind"] == "INTAKE").count() != 1
        || rows.iter().filter(|r| r["kind"] == "REVIEW").count() != usize::from(review_expected)
    {
        return Err(unavailable());
    }
    let mut responsibilities = Vec::new();
    for row in rows {
        let state = row["state"].as_str().ok_or_else(unavailable)?;
        if row["_valid"] != true
            || !matches!(
                state,
                "PENDING"
                    | "MATERIALIZED"
                    | "BLOCKED_KNOWN"
                    | "STOPPED"
                    | "RETIRED"
                    | "COMPLETED"
                    | "UNKNOWN"
            )
            || (matches!(state, "MATERIALIZED" | "COMPLETED")
                && row["managed_instance_id"].is_null())
            || (state == "STOPPED" && !row["stop_code"].is_string())
        {
            return Err(unavailable());
        }
        let mut item =
            json!({"responsibilityId":row["intent_id"],"kind":row["kind"],"state":state});
        for (column, key) in [
            ("managed_instance_id", "managedWorkflowInstanceId"),
            ("outcome", "outcomeCode"),
        ] {
            if !row[column].is_null() {
                item[key] = row[column].clone();
            }
        }
        if row["stop_code"].is_string() {
            let mut stop = json!({"code":row["stop_code"],"responsibilityId":row["intent_id"]});
            for (column, key) in [
                ("owner_replacement_command_id", "ownerReplacementCommandId"),
                ("node_visit_id", "nodeVisitId"),
                ("last_attempt_id", "attemptId"),
                ("_effectId", "effectId"),
            ] {
                if !row[column].is_null() {
                    stop[key] = row[column].clone();
                }
            }
            item["stop"] = stop;
        }
        responsibilities.push(item);
    }
    Ok(Some(
        json!({"version":1,"responsibilities":responsibilities}),
    ))
}
