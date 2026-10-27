//! Exact entry joins and bounded selections; no event replay or semantic verdict.
use super::super::{
    query_detail::{fetch_submissions, validate_submission_rows},
    query_rows::{QueryBaseRow, SubmissionRow},
};
use crate::application::workflow_instance::{
    current_task_input::{inconsistent, CurrentTaskInputError},
    query_types::SubmissionHistoryItem,
};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use std::collections::BTreeSet;
use uuid::Uuid;

type Tx<'a> = Transaction<'a, Postgres>;

#[derive(sqlx::FromRow)]
struct Event {
    event_id: Uuid,
    event_sequence: i32,
    event_type: String,
    source_node_visit_id: Option<Uuid>,
    target_node_visit_id: Option<Uuid>,
    context_revision_id: Option<Uuid>,
    submission_id: Option<Uuid>,
    transition_effect: Option<String>,
    from_node_id: Option<Uuid>,
    to_node_id: Option<Uuid>,
    event_data: Option<Value>,
}
const EVENT_COLUMNS:&str="event_id,event_sequence,event_type,source_node_visit_id,target_node_visit_id,context_revision_id,submission_id,transition_effect::text,from_node_id,to_node_id,event_data";

pub(super) async fn exact_submission(
    tx: &mut Tx<'_>,
    base: &QueryBaseRow,
    id: Uuid,
) -> Result<Option<SubmissionHistoryItem>, CurrentTaskInputError> {
    let row = sqlx::query_as::<_, SubmissionRow>(
        "SELECT s.submission_id,s.workflow_instance_id,s.source_node_visit_id,
         sv.workflow_instance_id AS source_visit_instance_id,sv.node_id AS source_node_id,
         sn.definition_version_id AS source_node_definition_version_id,
         sn.node_key AS source_node_key,sn.display_name AS source_node_display_name,
         sn.node_type::text AS source_node_type,s.context_revision_id,
         cr.workflow_instance_id AS context_instance_id,s.author_principal_id,
         s.transition_id,t.definition_version_id AS transition_definition_version_id,
         t.transition_effect::text,s.payload,s.payload_digest,s.schema_version,s.created_at
         FROM workflow_submissions s
         JOIN workflow_node_visits sv ON sv.node_visit_id=s.source_node_visit_id
         JOIN workflow_node_definitions sn ON sn.node_id=sv.node_id
         JOIN workflow_context_revisions cr ON cr.context_revision_id=s.context_revision_id
         JOIN workflow_transition_definitions t ON t.transition_id=s.transition_id
         WHERE s.workflow_instance_id=$1 AND s.submission_id=$2",
    )
    .bind(base.workflow_instance_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(row) = row {
        validate_submission_rows(std::slice::from_ref(&row), base)?;
        Ok(Some(row.into_item()))
    } else {
        Ok(None)
    }
}

pub(super) async fn entry(
    tx: &mut Tx<'_>,
    base: &QueryBaseRow,
) -> Result<Value, CurrentTaskInputError> {
    let sql=format!("SELECT {EVENT_COLUMNS} FROM workflow_events WHERE workflow_instance_id=$1 AND target_node_visit_id=$2 AND source_node_visit_id IS DISTINCT FROM target_node_visit_id ORDER BY event_sequence LIMIT 2");
    let events = sqlx::query_as::<_, Event>(&sql)
        .bind(base.workflow_instance_id)
        .bind(base.current_node_visit_id)
        .fetch_all(&mut **tx)
        .await?;
    if events.len() > 1 {
        return Err(inconsistent("more than one current visit entry event"));
    }
    let mut result = json!({"status":"UNSUPPORTED_ENTRY","event_id":null,"event_type":null,"event_sequence":null,"source_node_visit_id":null,"target_node_visit_id":base.current_node_visit_id,"context_revision_id":null,"transition_id":null,"submission_id":null,"submission_status":"UNKNOWN"});
    let Some(e) = events.first() else {
        return Ok(result);
    };
    result["event_id"] = json!(e.event_id);
    result["event_sequence"] = json!(e.event_sequence);
    result["source_node_visit_id"] = json!(e.source_node_visit_id);
    result["context_revision_id"] = json!(e.context_revision_id);
    match e.event_type.as_str() {
        "INSTANCE_CREATED" | "WORKFLOW_INSTANCE_IMPORTED" => {
            if e.source_node_visit_id.is_some()
                || e.submission_id.is_some()
                || base.entered_by_transition_id.is_some()
                || e.context_revision_id.is_none()
            {
                return Err(inconsistent("invalid initial entry"));
            }
            result["status"] = json!("INITIAL");
            result["submission_status"] = json!("NONE_RECORDED");
        }
        "WORKFLOW_TRANSITION_COMMITTED" | "WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED" => {
            validate_transition_entry(tx, base, e).await?;
            result["status"] = json!("TRANSITION");
            result["transition_id"] = json!(base.entered_by_transition_id);
            result["submission_id"] = json!(e.submission_id);
            result["submission_status"] = json!(if e.submission_id.is_some() {
                "RECORDED"
            } else {
                "NONE_RECORDED"
            });
        }
        "ADMIN_EMERGENCY_OVERRIDE_COMMITTED"
        | "HUMAN_EXECUTOR_NORMALIZED"
        | "PRINCIPAL_SUCCESSOR_MIGRATION_COMMITTED" => {
            if e.submission_id.is_some() || base.entered_by_transition_id.is_some() {
                return Err(inconsistent(
                    "unexpected transition in administrative entry",
                ));
            }
            result["status"] = json!("NON_TRANSITION_ENTRY");
            result["submission_status"] = json!("NONE_RECORDED");
        }
        _ => return Ok(result),
    }
    result["event_type"] = json!(e.event_type);
    Ok(result)
}

fn data_uuid(data: &Value, key: &str) -> Option<Uuid> {
    data.get(key)?.as_str()?.parse().ok()
}
async fn validate_transition_entry(
    tx: &mut Tx<'_>,
    base: &QueryBaseRow,
    e: &Event,
) -> Result<(), CurrentTaskInputError> {
    let data = e
        .event_data
        .as_ref()
        .ok_or_else(|| inconsistent("transition entry data missing"))?;
    let transition = base
        .entered_by_transition_id
        .ok_or_else(|| inconsistent("missing entry transition"))?;
    let source = e
        .source_node_visit_id
        .ok_or_else(|| inconsistent("missing entry source visit"))?;
    let context = e
        .context_revision_id
        .ok_or_else(|| inconsistent("missing entry context"))?;
    let target = base
        .current_node_visit_id
        .ok_or_else(|| inconsistent("missing target visit"))?;
    let context_key = if e.event_type == "WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED" {
        "new_context_revision_id"
    } else {
        "context_revision_id"
    };
    for (key, expected) in [
        ("transition_definition_id", transition),
        ("source_node_visit_id", source),
        ("target_node_visit_id", target),
        (context_key, context),
    ] {
        if data_uuid(data, key) != Some(expected) {
            return Err(inconsistent("entry event identity mismatch"));
        }
    }
    let endpoints:Option<(Uuid,Uuid,String,Uuid)>=sqlx::query_as("SELECT t.source_node_id,t.target_node_id,t.transition_effect::text,sv.node_id FROM workflow_transition_definitions t JOIN workflow_node_visits sv ON sv.node_visit_id=$2 AND sv.workflow_instance_id=$3 WHERE t.transition_id=$1 AND t.definition_version_id=$4")
        .bind(transition).bind(source).bind(base.workflow_instance_id).bind(base.definition_version_id).fetch_optional(&mut **tx).await?;
    let Some((source_node, target_node, effect, source_visit_node)) = endpoints else {
        return Err(inconsistent("entry transition relationship missing"));
    };
    if source_node != source_visit_node
        || Some(target_node) != base.current_node_id
        || e.from_node_id != Some(source_node)
        || e.to_node_id != Some(target_node)
        || e.transition_effect.as_deref() != Some(effect.as_str())
        || data.get("transition_effect").and_then(Value::as_str) != Some(effect.as_str())
    {
        return Err(inconsistent("entry transition endpoints mismatch"));
    }
    if let Some(id) = e.submission_id {
        let s = exact_submission(tx, base, id)
            .await?
            .ok_or_else(|| inconsistent("recorded entry submission missing"))?;
        if s.source_node_visit_id != source
            || s.context_revision_id != context
            || s.transition_id != transition
            || s.transition_effect != effect
            || data
                .get("submission_payload_digest")
                .and_then(Value::as_str)
                != Some(s.payload_digest.as_str())
        {
            return Err(inconsistent(
                "entry submission causal relationship mismatch",
            ));
        }
    } else if !data
        .get("submission_payload_digest")
        .is_some_and(Value::is_null)
    {
        return Err(inconsistent("entry digest without submission"));
    }
    Ok(())
}

pub(super) struct Selected {
    pub entry: Value,
    pub recent_ids: Vec<Uuid>,
    pub recent_truncated: bool,
    pub feedback: Vec<Value>,
    pub feedback_truncated: bool,
    pub requested: Vec<Value>,
    pub submissions: Vec<SubmissionHistoryItem>,
}
pub(super) async fn select(
    tx: &mut Tx<'_>,
    base: &QueryBaseRow,
    actor: Uuid,
    requested_ids: &[Uuid],
) -> Result<Selected, CurrentTaskInputError> {
    let entry = entry(tx, base).await?;
    let mut recent = fetch_submissions(tx, base, actor, true, None, 9, true).await?;
    validate_submission_rows(&recent, base)?;
    let recent_truncated = recent.len() > 8;
    recent.truncate(8);
    let recent_ids = recent.iter().map(|s| s.submission_id).collect::<Vec<_>>();
    let sql=format!("SELECT {EVENT_COLUMNS} FROM workflow_events WHERE workflow_instance_id=$1 AND transition_effect='RETURN' ORDER BY event_sequence DESC LIMIT 5");
    let mut returns = sqlx::query_as::<_, Event>(&sql)
        .bind(base.workflow_instance_id)
        .fetch_all(&mut **tx)
        .await?;
    let feedback_truncated = returns.len() > 4;
    returns.truncate(4);
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    let mut add = |id: Uuid| {
        if seen.insert(id) {
            ids.push(id)
        }
    };
    if let Some(id) = entry["submission_id"].as_str().and_then(|s| s.parse().ok()) {
        add(id)
    }
    for id in requested_ids {
        add(*id)
    }
    for id in &recent_ids {
        add(*id)
    }
    for e in &returns {
        if let Some(id) = e.submission_id {
            add(id)
        }
    }
    let mut submissions = Vec::new();
    let mut requested = Vec::new();
    for id in ids {
        let s = exact_submission(tx, base, id).await?;
        if requested_ids.contains(&id) {
            requested.push(
                json!({"id":id,"status":if s.is_some(){"FOUND"}else{"NOT_FOUND_OR_NOT_VISIBLE"}}),
            )
        }
        if let Some(s) = s {
            submissions.push(s)
        } else if !requested_ids.contains(&id) {
            return Err(inconsistent("selected fact missing"));
        }
    }
    requested.sort_by_key(|v| v["id"].as_str().unwrap().to_string());
    let mut feedback = Vec::new();
    for e in returns {
        let mut related = Vec::new();
        let mut unique = BTreeSet::new();
        if let Some(id) = e.submission_id {
            let s = submissions
                .iter()
                .find(|s| s.submission_id == id)
                .ok_or_else(|| inconsistent("return submission missing"))?;
            if let Some(value) = s.payload.get("relatedSubmissionIds") {
                let array = value
                    .as_array()
                    .ok_or_else(|| inconsistent("invalid return reference array"))?;
                let mut all = Vec::new();
                for item in array {
                    let id = item
                        .as_str()
                        .and_then(|s| s.parse::<Uuid>().ok())
                        .ok_or_else(|| inconsistent("invalid return UUID"))?;
                    if unique.insert(id) {
                        all.push(id)
                    }
                }
                if !all.is_empty() {
                    let count:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_submissions WHERE workflow_instance_id=$1 AND submission_id=ANY($2)").bind(base.workflow_instance_id).bind(&all).fetch_one(&mut **tx).await?;
                    if count as usize != all.len() {
                        return Err(inconsistent(
                            "return reference escapes instance or is missing",
                        ));
                    }
                }
                related.extend(all.into_iter().take(16));
            }
        }
        feedback.push(json!({"event_id":e.event_id,"event_sequence":e.event_sequence,"source_node_visit_id":e.source_node_visit_id,"target_node_visit_id":e.target_node_visit_id,"context_revision_id":e.context_revision_id,"submission_id":e.submission_id,"related_submission_ids":related,"related_submission_ids_truncated":unique.len()>16}));
    }
    Ok(Selected {
        entry,
        recent_ids,
        recent_truncated,
        feedback,
        feedback_truncated,
        requested,
        submissions,
    })
}
