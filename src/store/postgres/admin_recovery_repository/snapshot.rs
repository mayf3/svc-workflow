use std::collections::{HashMap, HashSet};

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::workflow_instance::recovery::{
    BeforeSnapshotV1, RecoveryError, WorkflowProjection,
};

use super::rows::{ContextFact, EventFact, InstanceRow, SubmissionFact, VisitFact};

fn storage(error: sqlx::Error) -> RecoveryError {
    RecoveryError::StorageError(error.to_string())
}

pub(super) async fn lock_instance(
    tx: &mut Transaction<'_, Postgres>,
    instance_id: Uuid,
) -> Result<InstanceRow, RecoveryError> {
    sqlx::query_as(
        "SELECT workflow_instance_id, domain_id, definition_version_id,
                created_by_principal_id, current_context_revision_id,
                current_node_visit_id, workflow_state_version
         FROM workflow_instances WHERE workflow_instance_id = $1 FOR UPDATE",
    )
    .bind(instance_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    .ok_or(RecoveryError::InstanceNotFound)
}

pub(super) fn before_snapshot(instance: &InstanceRow) -> BeforeSnapshotV1 {
    BeforeSnapshotV1::new(
        instance.workflow_instance_id,
        instance.domain_id,
        instance.definition_version_id,
        instance.created_by_principal_id,
        &instance.projection(),
    )
}

pub(super) fn verify_expected_digest(
    expected: Option<&str>,
    actual: &str,
) -> Result<(), RecoveryError> {
    if let Some(expected) = expected {
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(RecoveryError::InvalidInput(
                "expected_before_snapshot_digest must be lowercase SHA-256 hex".to_string(),
            ));
        }
        if expected != actual {
            return Err(RecoveryError::BeforeSnapshotDigestMismatch {
                expected: expected.to_string(),
                actual: actual.to_string(),
            });
        }
    }
    Ok(())
}

pub(super) async fn reconstruct_projection(
    tx: &mut Transaction<'_, Postgres>,
    instance: &InstanceRow,
) -> Result<WorkflowProjection, RecoveryError> {
    let contexts = load_contexts(tx, instance.workflow_instance_id).await?;
    let visits = load_visits(tx, instance.workflow_instance_id).await?;
    let submissions = load_submissions(tx, instance.workflow_instance_id).await?;
    let events = load_events(tx, instance.workflow_instance_id).await?;
    validate_contexts(instance, &contexts)?;
    validate_visits(instance, &visits)?;
    validate_submissions(instance, &contexts, &visits, &submissions)?;
    validate_events(instance, &contexts, &visits, &submissions, &events)
}

async fn load_contexts(
    tx: &mut Transaction<'_, Postgres>,
    instance_id: Uuid,
) -> Result<Vec<ContextFact>, RecoveryError> {
    sqlx::query_as(
        "SELECT context_revision_id, workflow_instance_id, revision_number,
                previous_revision_id, payload, payload_digest
         FROM workflow_context_revisions WHERE workflow_instance_id = $1
         ORDER BY revision_number",
    )
    .bind(instance_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)
}

async fn load_visits(
    tx: &mut Transaction<'_, Postgres>,
    instance_id: Uuid,
) -> Result<Vec<VisitFact>, RecoveryError> {
    sqlx::query_as(
        "SELECT v.node_visit_id, v.workflow_instance_id, v.node_id, v.visit_number,
                v.assignee_principal_id, v.entered_by_transition_id,
                n.definition_version_id, n.node_type::text,
                t.definition_version_id AS entered_transition_definition_version_id,
                t.target_node_id AS entered_transition_target_node_id
         FROM workflow_node_visits v
         JOIN workflow_node_definitions n ON n.node_id = v.node_id
         LEFT JOIN workflow_transition_definitions t
           ON t.transition_id = v.entered_by_transition_id
         WHERE v.workflow_instance_id = $1
         ORDER BY v.created_at, v.node_visit_id",
    )
    .bind(instance_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)
}

async fn load_submissions(
    tx: &mut Transaction<'_, Postgres>,
    instance_id: Uuid,
) -> Result<Vec<SubmissionFact>, RecoveryError> {
    sqlx::query_as(
        "SELECT s.submission_id, s.workflow_instance_id, s.source_node_visit_id,
                s.context_revision_id, s.transition_id, s.payload, s.payload_digest,
                t.definition_version_id AS transition_definition_version_id,
                t.source_node_id AS transition_source_node_id
         FROM workflow_submissions s
         LEFT JOIN workflow_transition_definitions t ON t.transition_id = s.transition_id
         WHERE s.workflow_instance_id = $1 ORDER BY s.created_at, s.submission_id",
    )
    .bind(instance_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)
}

async fn load_events(
    tx: &mut Transaction<'_, Postgres>,
    instance_id: Uuid,
) -> Result<Vec<EventFact>, RecoveryError> {
    sqlx::query_as(
        "SELECT workflow_instance_id, event_sequence, event_type,
                transition_effect::text, source_node_visit_id, target_node_visit_id,
                context_revision_id, submission_id, event_data, event_data_digest,
                from_node_id, to_node_id, old_workflow_state_version,
                new_workflow_state_version
         FROM workflow_events WHERE workflow_instance_id = $1 ORDER BY event_sequence",
    )
    .bind(instance_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)
}

fn invalid(detail: impl Into<String>) -> RecoveryError {
    RecoveryError::InvalidImmutableFacts(detail.into())
}

fn validate_contexts(instance: &InstanceRow, facts: &[ContextFact]) -> Result<(), RecoveryError> {
    if facts.is_empty() {
        return Err(invalid("context fact chain is empty"));
    }
    for (index, fact) in facts.iter().enumerate() {
        let expected_number = index as i32 + 1;
        if fact.workflow_instance_id != instance.workflow_instance_id
            || fact.revision_number != expected_number
            || (index == 0 && fact.previous_revision_id.is_some())
            || (index > 0
                && fact.previous_revision_id != Some(facts[index - 1].context_revision_id))
        {
            return Err(invalid("context revision chain is not contiguous"));
        }
        let actual =
            digest::compute_json_digest(&fact.payload).map_err(RecoveryError::StorageError)?;
        if actual != fact.payload_digest {
            return Err(invalid("context payload digest mismatch"));
        }
    }
    Ok(())
}

fn validate_visits(instance: &InstanceRow, facts: &[VisitFact]) -> Result<(), RecoveryError> {
    if facts.is_empty() {
        return Err(invalid("node visit fact set is empty"));
    }
    for visit in facts {
        if visit.workflow_instance_id != instance.workflow_instance_id
            || visit.definition_version_id != instance.definition_version_id
            || visit.visit_number < 1
        {
            return Err(invalid("node visit escapes instance definition"));
        }
        if visit.node_type != "TERMINAL" && visit.assignee_principal_id.is_none() {
            return Err(invalid("non-terminal node visit has no assignee"));
        }
        if visit.entered_by_transition_id.is_some()
            && (visit.entered_transition_definition_version_id
                != Some(instance.definition_version_id)
                || visit.entered_transition_target_node_id != Some(visit.node_id))
        {
            return Err(invalid(
                "node visit entered transition relationship is invalid",
            ));
        }
    }
    Ok(())
}

fn validate_submissions(
    instance: &InstanceRow,
    contexts: &[ContextFact],
    visits: &[VisitFact],
    facts: &[SubmissionFact],
) -> Result<(), RecoveryError> {
    let context_ids: HashSet<_> = contexts
        .iter()
        .map(|fact| fact.context_revision_id)
        .collect();
    let visit_by_id: HashMap<_, _> = visits
        .iter()
        .map(|fact| (fact.node_visit_id, fact))
        .collect();
    for fact in facts {
        let source = visit_by_id.get(&fact.source_node_visit_id);
        if fact.workflow_instance_id != instance.workflow_instance_id
            || !context_ids.contains(&fact.context_revision_id)
            || fact.transition_definition_version_id != Some(instance.definition_version_id)
            || source.map(|visit| visit.node_id) != fact.transition_source_node_id
        {
            return Err(invalid("submission relationship is invalid"));
        }
        let actual =
            digest::compute_json_digest(&fact.payload).map_err(RecoveryError::StorageError)?;
        if actual != fact.payload_digest {
            return Err(invalid("submission payload digest mismatch"));
        }
    }
    Ok(())
}

fn require_keys(data: &serde_json::Value, keys: &[&str]) -> bool {
    keys.iter().all(|key| !data[*key].is_null())
}

fn validate_event_matrix(event: &EventFact) -> bool {
    let data = event.event_data.as_ref();
    match event.event_type.as_str() {
        "INSTANCE_CREATED" | "WORKFLOW_INSTANCE_CREATED" | "WORKFLOW_INSTANCE_IMPORTED" => {
            event.event_sequence == 1
                && event.old_workflow_state_version == 0
                && event.source_node_visit_id.is_none()
                && event.target_node_visit_id.is_some()
                && event.context_revision_id.is_some()
                && event.submission_id.is_none()
                && event.transition_effect.is_none()
                && event.from_node_id.is_none()
                && event.to_node_id.is_none()
                && data.is_some_and(|value| match event.event_type.as_str() {
                    "WORKFLOW_INSTANCE_IMPORTED" => require_keys(
                        value,
                        &[
                            "legacySystem",
                            "legacyRecordId",
                            "legacySnapshotDigest",
                            "importedNodeId",
                            "importedAt",
                            "creatorResolution",
                        ],
                    ),
                    _ => require_keys(value, &["definition_version_id", "initial_node_id"]),
                })
        }
        "CONTEXT_REVISED" | "WORKFLOW_CONTEXT_REVISED" => {
            event.source_node_visit_id.is_some()
                && event.source_node_visit_id == event.target_node_visit_id
                && event.context_revision_id.is_some()
                && event.submission_id.is_none()
                && event.transition_effect.is_none()
                && event.from_node_id.is_none()
                && event.to_node_id.is_none()
                && data.is_some_and(|value| {
                    require_keys(
                        value,
                        &["previous_context_revision_id", "new_context_revision_id"],
                    )
                })
        }
        "WORKFLOW_TRANSITION_COMMITTED" => {
            transition_shape(event, false)
                && data.is_some_and(|value| {
                    require_keys(
                        value,
                        &[
                            "transition_definition_id",
                            "source_node_id",
                            "target_node_id",
                        ],
                    )
                })
        }
        "WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED" => {
            transition_shape(event, true)
                && event.transition_effect.as_deref() == Some("ADVANCE")
                && data.is_some_and(|value| {
                    require_keys(
                        value,
                        &["new_context_revision_id", "transition_definition_id"],
                    )
                })
        }
        "ADMIN_EMERGENCY_OVERRIDE_COMMITTED" => {
            transition_shape(event, false)
                && event.submission_id.is_none()
                && matches!(
                    event.transition_effect.as_deref(),
                    Some("ADVANCE" | "TERMINATE")
                )
                && data.is_some_and(|value| {
                    require_keys(value, &["operation", "reason", "beforeSnapshotDigest"])
                })
        }
        _ => false,
    }
}

fn transition_shape(event: &EventFact, submission_required: bool) -> bool {
    event.source_node_visit_id.is_some()
        && event.target_node_visit_id.is_some()
        && event.context_revision_id.is_some()
        && (!submission_required || event.submission_id.is_some())
        && event.transition_effect.is_some()
        && event.from_node_id.is_some()
        && event.to_node_id.is_some()
}

fn validate_events(
    instance: &InstanceRow,
    contexts: &[ContextFact],
    visits: &[VisitFact],
    submissions: &[SubmissionFact],
    events: &[EventFact],
) -> Result<WorkflowProjection, RecoveryError> {
    if events.is_empty() {
        return Err(invalid("event fact sequence is empty"));
    }
    let context_ids: HashSet<_> = contexts
        .iter()
        .map(|fact| fact.context_revision_id)
        .collect();
    let visit_by_id: HashMap<_, _> = visits
        .iter()
        .map(|fact| (fact.node_visit_id, fact))
        .collect();
    let submission_by_id: HashMap<_, _> = submissions
        .iter()
        .map(|fact| (fact.submission_id, fact))
        .collect();
    let mut referenced_contexts = HashSet::new();
    let mut referenced_visits = HashSet::new();
    let mut referenced_submissions = HashSet::new();

    for (index, event) in events.iter().enumerate() {
        let expected = index as i32 + 1;
        if event.workflow_instance_id != instance.workflow_instance_id
            || event.event_sequence != expected
            || event.old_workflow_state_version != expected - 1
            || event.new_workflow_state_version != expected
            || !validate_event_matrix(event)
        {
            return Err(invalid("event sequence or type/field matrix is invalid"));
        }
        if let Some(data) = &event.event_data {
            let actual = digest::compute_json_digest(data).map_err(RecoveryError::StorageError)?;
            if event.event_data_digest.as_deref() != Some(actual.as_str()) {
                return Err(invalid("event data digest mismatch"));
            }
        } else if event.event_data_digest.is_some() {
            return Err(invalid("event digest exists without event data"));
        }
        if let Some(context) = event.context_revision_id {
            if !context_ids.contains(&context) {
                return Err(invalid("event references an invalid context revision"));
            }
            referenced_contexts.insert(context);
        }
        for visit_id in [event.source_node_visit_id, event.target_node_visit_id]
            .into_iter()
            .flatten()
        {
            if !visit_by_id.contains_key(&visit_id) {
                return Err(invalid("event references an invalid node visit"));
            }
            referenced_visits.insert(visit_id);
        }
        if let Some(submission_id) = event.submission_id {
            let submission = submission_by_id
                .get(&submission_id)
                .ok_or_else(|| invalid("event references an invalid submission"))?;
            if Some(submission.source_node_visit_id) != event.source_node_visit_id
                || Some(submission.context_revision_id) != event.context_revision_id
            {
                return Err(invalid("event submission relationship is invalid"));
            }
            referenced_submissions.insert(submission_id);
        }
        if event.from_node_id.is_some()
            && event.from_node_id
                != event
                    .source_node_visit_id
                    .and_then(|id| visit_by_id.get(&id).map(|visit| visit.node_id))
            || event.to_node_id.is_some()
                && event.to_node_id
                    != event
                        .target_node_visit_id
                        .and_then(|id| visit_by_id.get(&id).map(|visit| visit.node_id))
        {
            return Err(invalid("event node fields disagree with visit references"));
        }
    }
    if referenced_contexts.len() != contexts.len()
        || referenced_visits.len() != visits.len()
        || referenced_submissions.len() != submissions.len()
    {
        return Err(invalid("immutable fact exists outside the event sequence"));
    }
    let last = events.last().expect("non-empty events");
    Ok(WorkflowProjection {
        current_context_revision_id: last.context_revision_id,
        current_node_visit_id: last.target_node_visit_id,
        workflow_state_version: last.new_workflow_state_version,
    })
}
