use super::*;
#[derive(sqlx::FromRow)]
struct Source {
    domain_id: Uuid,
    definition_domain: Uuid,
    definition_version_id: Uuid,
    semantic_model_version: i16,
    version_status: String,
    enabled: bool,
    cancelled: bool,
    archived: bool,
    node_id: Option<Uuid>,
    node_type: Option<String>,
    visit_instance: Option<Uuid>,
    node_version: Option<Uuid>,
}
#[derive(sqlx::FromRow)]
struct Proof {
    event_id: Uuid,
    submission_id: Uuid,
    payload: Value,
    payload_digest: String,
    event_data: Value,
    event_data_digest: String,
}
pub(super) async fn evaluate(
    tx: &mut Transaction<'_, Postgres>,
    domain: Uuid,
    r: &Reference,
) -> Result<Value, WorkflowQueryError> {
    let unknown = || shape(r, "UNKNOWN", Some("LINEAGE_INCONSISTENT"));
    let row:Option<Source>=sqlx::query_as("SELECT wi.domain_id,wd.domain_id AS definition_domain,wi.definition_version_id,wi.semantic_model_version,dv.version_status::text AS version_status,d.enabled,wi.cancelled,wi.archived_at IS NOT NULL AS archived,v.node_id,n.node_type::text AS node_type,v.workflow_instance_id AS visit_instance,n.definition_version_id AS node_version FROM workflow_instances wi JOIN domains d USING(domain_id) JOIN workflow_definition_versions dv USING(definition_version_id) JOIN workflow_definitions wd ON wd.workflow_definition_id=dv.workflow_definition_id LEFT JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id LEFT JOIN workflow_node_definitions n ON n.node_id=v.node_id WHERE wi.workflow_instance_id=$1").bind(r.workflow_instance_id).fetch_optional(&mut **tx).await.map_err(storage)?;
    let Some(s) = row else { return Ok(unknown()) };
    if s.domain_id != domain || s.definition_domain != domain {
        return Ok(shape(r, "UNKNOWN", Some("SOURCE_NOT_VISIBLE")));
    }
    if s.definition_version_id != r.definition_version_id
        || s.semantic_model_version != 3
        || s.version_status != "PUBLISHED"
        || s.visit_instance != Some(r.workflow_instance_id)
        || s.node_version != Some(r.definition_version_id)
    {
        return Ok(unknown());
    }
    let transition:Option<Uuid>=sqlx::query_scalar("SELECT t.source_node_id FROM workflow_transition_definitions t JOIN workflow_node_definitions n ON n.node_id=t.target_node_id AND n.definition_version_id=t.definition_version_id AND n.node_type='TERMINAL' WHERE t.transition_id=$1 AND t.definition_version_id=$2 AND t.submission_schema IS NOT NULL").bind(r.transition_id).bind(r.definition_version_id).fetch_optional(&mut **tx).await.map_err(storage)?;
    let Some(selected_source) = transition else {
        return Ok(unknown());
    };
    // Positive proof precedes lifecycle closure and graph reachability. Compare JSON text,
    // never cast untrusted persisted event JSON to UUID in SQL.
    let proofs:Vec<Proof>=sqlx::query_as("SELECT e.event_id,s.submission_id,s.payload,s.payload_digest,e.event_data,e.event_data_digest FROM workflow_events e
 JOIN workflow_instances wi ON wi.workflow_instance_id=e.workflow_instance_id
 JOIN workflow_node_visits sv ON sv.node_visit_id=e.source_node_visit_id AND sv.workflow_instance_id=wi.workflow_instance_id
 JOIN workflow_node_visits tv ON tv.node_visit_id=e.target_node_visit_id AND tv.workflow_instance_id=wi.workflow_instance_id
 JOIN workflow_node_definitions sn ON sn.node_id=sv.node_id AND sn.definition_version_id=wi.definition_version_id
 JOIN workflow_node_definitions tn ON tn.node_id=tv.node_id AND tn.definition_version_id=wi.definition_version_id AND tn.node_type='TERMINAL'
 JOIN workflow_context_revisions cr ON cr.context_revision_id=e.context_revision_id AND cr.workflow_instance_id=wi.workflow_instance_id
 JOIN workflow_transition_definitions t ON t.transition_id::text=e.event_data->>'transition_definition_id' AND t.definition_version_id=wi.definition_version_id AND t.source_node_id=sv.node_id AND t.target_node_id=tv.node_id AND t.transition_effect::text=e.transition_effect::text
 JOIN workflow_submissions s ON s.submission_id=e.submission_id AND s.workflow_instance_id=wi.workflow_instance_id AND s.source_node_visit_id=sv.node_visit_id AND s.context_revision_id=cr.context_revision_id AND s.author_principal_id=e.actor_principal_id AND s.transition_id=t.transition_id
 JOIN workflow_command_receipts receipt ON receipt.command_id=e.command_id AND receipt.principal_id=e.actor_principal_id AND receipt.receipt_status='COMPLETED' AND receipt.response_status BETWEEN 200 AND 299 AND ((e.event_type='WORKFLOW_TRANSITION_COMMITTED' AND receipt.command_type='EXECUTE_WORKFLOW_TRANSITION') OR (e.event_type='WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED' AND receipt.command_type='REVISE_CONTEXT_AND_TRANSITION'))
 WHERE wi.workflow_instance_id=$1 AND wi.definition_version_id=$2 AND t.transition_id=$3 AND e.event_type IN ('WORKFLOW_TRANSITION_COMMITTED','WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED') AND sv.assignee_principal_id=e.actor_principal_id AND e.from_node_id=sv.node_id AND e.to_node_id=tv.node_id AND tv.entered_by_transition_id=t.transition_id").bind(r.workflow_instance_id).bind(r.definition_version_id).bind(r.transition_id).fetch_all(&mut **tx).await.map_err(storage)?;
    if proofs.len() == 1 {
        let p = &proofs[0];
        if digest::compute_json_digest(&p.payload).ok().as_ref() != Some(&p.payload_digest)
            || digest::compute_json_digest(&p.event_data).ok().as_ref()
                != Some(&p.event_data_digest)
        {
            return Ok(unknown());
        }
        let mut v = shape(r, "READY", None);
        v["eventId"] = json!(p.event_id);
        v["submissionId"] = json!(p.submission_id);
        v["submissionSha256"] = json!(p.payload_digest);
        return Ok(v);
    }
    let selected_events:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1 AND event_data->>'transition_definition_id'=$2").bind(r.workflow_instance_id).bind(r.transition_id.to_string()).fetch_one(&mut **tx).await.map_err(storage)?;
    if selected_events > 0 || proofs.len() > 1 {
        return Ok(unknown());
    }
    let reason = if !s.enabled {
        Some("SOURCE_DOMAIN_DISABLED")
    } else if s.cancelled {
        Some("SOURCE_CANCELLED")
    } else if s.archived {
        Some("SOURCE_ARCHIVED")
    } else if s.node_type.as_deref() == Some("TERMINAL") {
        let terminated:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_events WHERE workflow_instance_id=$1 AND transition_effect='TERMINATE')").bind(r.workflow_instance_id).fetch_one(&mut **tx).await.map_err(storage)?;
        Some(if terminated {
            "SOURCE_TERMINATED"
        } else {
            "OTHER_TERMINAL_TRANSITION"
        })
    } else {
        None
    };
    if let Some(reason) = reason {
        return Ok(shape(r, "CLOSED", Some(reason)));
    }
    let Some(node) = s.node_id else {
        return Ok(unknown());
    };
    let reachable:bool=sqlx::query_scalar("WITH RECURSIVE reachable(node_id) AS (SELECT $1::uuid UNION SELECT t.target_node_id FROM reachable r JOIN workflow_node_definitions n ON n.node_id=r.node_id AND n.definition_version_id=$2 AND n.node_type<>'TERMINAL' JOIN workflow_transition_definitions t ON t.source_node_id=r.node_id AND t.definition_version_id=$2) SELECT EXISTS(SELECT 1 FROM reachable WHERE node_id=$3)").bind(node).bind(r.definition_version_id).bind(selected_source).fetch_one(&mut **tx).await.map_err(storage)?;
    Ok(if reachable {
        shape(r, "WAITING", Some("PREREQUISITE_PENDING"))
    } else {
        shape(r, "CLOSED", Some("TRANSITION_UNREACHABLE"))
    })
}
