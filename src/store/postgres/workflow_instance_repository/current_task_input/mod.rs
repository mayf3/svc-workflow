//! One authorized snapshot per call; no command receipt or persisted input packet.
mod bodies;
mod selection;
use super::{
    query_detail::build_full,
    query_visibility::{authorized_snapshot, reject_restricted_history},
};
use crate::application::workflow_instance::current_task_input::{
    inconsistent, BodyKind, CurrentTaskInputError, CurrentTaskInputRequest,
};
use bodies::CanonicalBody;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn read(
    pool: &PgPool,
    actor: Uuid,
    instance: Uuid,
    request: CurrentTaskInputRequest,
) -> Result<Value, CurrentTaskInputError> {
    let mut snapshot = authorized_snapshot(pool, actor, instance, "CurrentTaskInput").await?;
    if !snapshot.visibility.is_full() {
        return reject_restricted_history(snapshot, actor, "CurrentTaskInput")
            .await
            .map_err(Into::into);
    }
    let base = &snapshot.base;
    let visit = base
        .current_node_visit_id
        .ok_or_else(|| inconsistent("missing current visit"))?;
    let context = base
        .current_context_revision_id
        .ok_or_else(|| inconsistent("missing current context"))?;
    if request.expected().is_some_and(|e| {
        e.node_visit_id != visit
            || e.workflow_state_version != base.workflow_state_version
            || e.context_revision_id != context
    }) {
        return Err(CurrentTaskInputError::Stale);
    }
    let binding = json!({"workflow_instance_id":instance,"definition_version_id":base.definition_version_id,"node_visit_id":visit,"workflow_state_version":base.workflow_state_version,"context_revision_id":context});
    let mut response = match request {
        CurrentTaskInputRequest::Input { submission_ids, .. } => {
            let detail = serde_json::to_value(build_full(&mut snapshot.tx, base, actor).await?)
                .map_err(|_| inconsistent("serialize current detail"))?;
            let detail = CanonicalBody::new(BodyKind::CurrentDetail, visit, &detail, None)?;
            let selected =
                selection::select(&mut snapshot.tx, base, actor, &submission_ids).await?;
            let detail_body = detail.chunk(0, 4096)?;
            let mut budget = 8192 - detail_body.text.len();
            let mut submissions = Vec::new();
            for s in selected.submissions {
                let body = CanonicalBody::new(
                    BodyKind::SubmissionPayload,
                    s.submission_id,
                    &s.payload,
                    Some(&s.payload_digest),
                )?
                .chunk(0, budget)?;
                budget -= body.text.len();
                submissions.push(json!({"submission_id":s.submission_id,"workflow_instance_id":s.workflow_instance_id,"source_node_visit_id":s.source_node_visit_id,"context_revision_id":s.context_revision_id,"author_principal_id":s.author_principal_id,"transition_id":s.transition_id,"transition_effect":s.transition_effect,"created_at":s.created_at,"payload_digest":s.payload_digest,"body":body}));
            }
            json!({"view":"input","binding":binding,"input_status":"INCOMPLETE","detail_body":detail_body,"entry":selected.entry,"recent_submission_ids":selected.recent_ids,"recent_submissions_truncated":selected.recent_truncated,"return_feedback":selected.feedback,"return_feedback_truncated":selected.feedback_truncated,"requested_submissions":selected.requested,"submissions":submissions,"content_complete":false,"history_window_complete":!selected.recent_truncated&&!selected.feedback_truncated})
        }
        CurrentTaskInputRequest::Body {
            kind,
            id,
            sha256,
            offset,
            ..
        } => {
            let body = match kind {
                BodyKind::CurrentDetail => {
                    if id != visit {
                        return Err(CurrentTaskInputError::BodyNotFound);
                    }
                    let value =
                        serde_json::to_value(build_full(&mut snapshot.tx, base, actor).await?)
                            .map_err(|_| inconsistent("serialize current detail"))?;
                    CanonicalBody::new(kind, id, &value, None)?
                }
                BodyKind::SubmissionPayload => {
                    let s = selection::exact_submission(&mut snapshot.tx, base, id)
                        .await?
                        .ok_or(CurrentTaskInputError::BodyNotFound)?;
                    CanonicalBody::new(kind, id, &s.payload, Some(&s.payload_digest))?
                }
            };
            if body.sha256 != sha256 {
                return Err(CurrentTaskInputError::BodyChanged);
            }
            json!({"view":"body","binding":binding,"body":body.chunk(offset,8192)?})
        }
    };
    let parent:Option<Value>=sqlx::query_scalar("SELECT receipt FROM workflow_governance_child_bindings WHERE governance_child_instance_id=$1")
        .bind(instance).fetch_optional(&mut *snapshot.tx).await?;
    if let Some(parent) = parent {
        response["governanceParent"] = parent;
    }
    if response["view"] == "input" {
        bodies::bound_input_wire(&mut response)?
    }
    snapshot.tx.commit().await?;
    Ok(response)
}
