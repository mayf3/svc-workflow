//! Recovery-replay history tests for Assistance events (Blocker 3).
//!
//! These tests prove the replay engine reconstructs a per-`assistanceCaseId`
//! state machine and rejects event histories the live system could never emit.
//! Each negative case uses the real write path to produce a legitimate prefix,
//! then injects a single forged Assistance/transition event directly into the
//! immutable `workflow_events` log (with a correctly chained state version and a
//! matching `event_data_digest`), and asserts `rebuild_projection` fails with
//! `InvalidImmutableFacts`.

use sqlx::PgPool;
use uuid::Uuid;

use svc_workflow::application::workflow_instance::admin_recovery::{
    rebuild_projection, RebuildProjectionResult,
};
use svc_workflow::domain::definition::digest::compute_json_digest;
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::recovery::{
    RecoveryError, RebuildProjectionCommand,
};

use super::helpers::*;

struct Anchor {
    instance: Uuid,
    visit: Uuid,
    context: Uuid,
    node_id: Uuid,
    version: i32,
    actor: Uuid,
}

async fn anchor(pool: &PgPool, f: &Fixture) -> Anchor {
    let (visit, context, node_id, version): (Uuid, Uuid, Uuid, i32) = sqlx::query_as(
        "SELECT wi.current_node_visit_id, wi.current_context_revision_id,
                nv.node_id, wi.workflow_state_version
         FROM workflow_instances wi
         JOIN workflow_node_visits nv ON nv.node_visit_id = wi.current_node_visit_id
         WHERE wi.workflow_instance_id = $1",
    )
    .bind(f.instance)
    .fetch_one(pool)
    .await
    .unwrap();
    Anchor {
        instance: f.instance,
        visit,
        context,
        node_id,
        version,
        actor: f.agent,
    }
}

fn assistance_data(
    case_id: Uuid,
    previous_status: Option<&str>,
    new_status: &str,
) -> serde_json::Value {
    serde_json::json!({
        "assistanceCaseId": case_id.to_string(),
        "previousStatus": previous_status,
        "newStatus": new_status,
        "payloadDigest": request_hash("injected assistance payload"),
    })
}

/// Inject a forged Assistance event at the next sequence, chained onto the
/// current replay anchor. The event is structurally well-formed (correct keys,
/// visit/context binding, matching digest) so that only the per-case state
/// machine can reject it.
async fn inject_assistance_event(
    pool: &PgPool,
    a: &Anchor,
    event_type: &str,
    case_id: Uuid,
    previous_status: Option<&str>,
    new_status: &str,
) {
    let data = assistance_data(case_id, previous_status, new_status);
    let digest = compute_json_digest(&data).unwrap();
    let new_version = a.version + 1;
    sqlx::query(
        "INSERT INTO workflow_events
         (event_id, workflow_instance_id, event_sequence, event_schema_version,
          command_id, event_type, transition_effect,
          source_node_visit_id, target_node_visit_id,
          context_revision_id, submission_id, event_data, event_data_digest,
          actor_principal_id, from_node_id, to_node_id,
          old_workflow_state_version, new_workflow_state_version)
         VALUES ($1,$2,$3,'v1',NULL,$4,NULL::transition_effect,$5,$5,$6,NULL,$7,$8,$9,$10,$10,$11,$3)",
    )
    .bind(Uuid::new_v4())
    .bind(a.instance)
    .bind(new_version)
    .bind(event_type)
    .bind(a.visit)
    .bind(a.context)
    .bind(&data)
    .bind(&digest)
    .bind(a.actor)
    .bind(a.node_id)
    .bind(a.version)
    .execute(pool)
    .await
    .unwrap();
}

/// Inject a forged ADVANCE transition event whose source visit still has an
/// open assistance case. A real transition could never be emitted in this state
/// (the runtime gate refuses with `AssistanceOpen`), so replay must reject it.
async fn inject_transition_event_after_open_case(pool: &PgPool, f: &Fixture) {
    let a = anchor(pool, f).await;
    let draft_node: Uuid = sqlx::query_scalar(
        "SELECT node_id FROM workflow_node_visits WHERE node_visit_id = $1",
    )
    .bind(a.visit)
    .fetch_one(pool)
    .await
    .unwrap();
    // Materialise the target terminal visit so the event's deferred FK holds.
    let target_visit = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_visits
         (node_visit_id, workflow_instance_id, node_id, visit_number,
          assignee_principal_id, entered_by_transition_id)
         VALUES ($1,$2,$3,2,NULL,$4)",
    )
    .bind(target_visit)
    .bind(f.instance)
    .bind(f.terminal_node)
    .bind(f.transition)
    .execute(pool)
    .await
    .unwrap();
    let data = serde_json::json!({
        "transition_definition_id": f.transition,
        "transition_key": "advance",
        "transition_effect": "ADVANCE",
        "source_node_id": draft_node,
        "target_node_id": f.terminal_node,
        "source_node_visit_id": a.visit,
        "target_node_visit_id": target_visit,
        "context_revision_id": a.context,
        "submission_payload_digest": serde_json::Value::Null,
    });
    let digest = compute_json_digest(&data).unwrap();
    let new_version = a.version + 1;
    sqlx::query(
        "INSERT INTO workflow_events
         (event_id, workflow_instance_id, event_sequence, event_schema_version,
          command_id, event_type, transition_effect,
          source_node_visit_id, target_node_visit_id,
          context_revision_id, submission_id, event_data, event_data_digest,
          actor_principal_id, from_node_id, to_node_id,
          old_workflow_state_version, new_workflow_state_version)
         VALUES ($1,$2,$3,'v1',NULL,'WORKFLOW_TRANSITION_COMMITTED','ADVANCE'::transition_effect,
                 $4,$5,$6,NULL,$7,$8,$9,$10,$11,$12,$3)",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(new_version)
    .bind(a.visit)
    .bind(target_visit)
    .bind(a.context)
    .bind(&data)
    .bind(&digest)
    .bind(a.actor)
    .bind(draft_node)
    .bind(f.terminal_node)
    .bind(a.version)
    .execute(pool)
    .await
    .unwrap();
}

fn rebuild_command(f: &Fixture) -> RebuildProjectionCommand {
    RebuildProjectionCommand {
        principal_id: PrincipalId::from_uuid(f.admin),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(f.instance),
        expected_before_snapshot_digest: None,
    }
}

fn assert_invalid_facts(result: Result<RebuildProjectionResult, RecoveryError>) {
    assert!(
        matches!(result, Err(RecoveryError::InvalidImmutableFacts(_))),
        "expected InvalidImmutableFacts, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Positive: a legitimate escalate/resolve sequence still replays cleanly.
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn replay_accepts_request_escalate_resolve_history(pool: PgPool) {
    let f = setup(&pool).await;
    let requested = request_case(&pool, &f).await;
    let escalated = escalate(&pool, &f, requested.assistance_case_id, 2).await;
    let resolved = resolve(&pool, &f, escalated.assistance_case_id, 3).await;
    assert_eq!(resolved.workflow_state_version, 4);

    let rebuilt = rebuild_projection(&pool, rebuild_command(&f)).await.unwrap();
    // Projection is already correct, so the rebuild is a confirming no-op.
    assert!(!rebuilt.changed);
    assert_eq!(rebuilt.after_projection.current_node_visit_id, Some(f.visit));
    assert_eq!(rebuilt.after_projection.workflow_state_version, 4);
}

// ---------------------------------------------------------------------------
// Negative: the per-case state machine rejects impossible histories.
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn replay_rejects_escalate_as_the_first_case_event(pool: PgPool) {
    let f = setup(&pool).await;
    let a = anchor(&pool, &f).await;
    inject_assistance_event(
        &pool,
        &a,
        "ASSISTANCE_ESCALATED_TO_HUMAN",
        Uuid::new_v4(),
        Some("OWNER_PENDING"),
        "HUMAN_REQUIRED",
    )
    .await;
    assert_invalid_facts(rebuild_projection(&pool, rebuild_command(&f)).await);
}

#[sqlx::test]
async fn replay_rejects_resolve_as_the_first_case_event(pool: PgPool) {
    let f = setup(&pool).await;
    let a = anchor(&pool, &f).await;
    inject_assistance_event(
        &pool,
        &a,
        "ASSISTANCE_RESOLVED",
        Uuid::new_v4(),
        Some("OWNER_PENDING"),
        "RESOLVED",
    )
    .await;
    assert_invalid_facts(rebuild_projection(&pool, rebuild_command(&f)).await);
}

#[sqlx::test]
async fn replay_rejects_duplicate_request_for_one_case(pool: PgPool) {
    let f = setup(&pool).await;
    let requested = request_case(&pool, &f).await; // legitimate REQUESTED (v1 -> 2)
    let a = anchor(&pool, &f).await;
    inject_assistance_event(
        &pool,
        &a,
        "ASSISTANCE_REQUESTED",
        requested.assistance_case_id,
        None,
        "OWNER_PENDING",
    )
    .await;
    assert_invalid_facts(rebuild_projection(&pool, rebuild_command(&f)).await);
}

#[sqlx::test]
async fn replay_rejects_duplicate_escalate_for_one_case(pool: PgPool) {
    let f = setup(&pool).await;
    let requested = request_case(&pool, &f).await; // v1 -> 2
    escalate(&pool, &f, requested.assistance_case_id, 2).await; // v2 -> 3
    let a = anchor(&pool, &f).await;
    inject_assistance_event(
        &pool,
        &a,
        "ASSISTANCE_ESCALATED_TO_HUMAN",
        requested.assistance_case_id,
        Some("OWNER_PENDING"),
        "HUMAN_REQUIRED",
    )
    .await;
    assert_invalid_facts(rebuild_projection(&pool, rebuild_command(&f)).await);
}

#[sqlx::test]
async fn replay_rejects_resolving_a_case_that_was_never_opened(pool: PgPool) {
    let f = setup(&pool).await;
    let opened = request_case(&pool, &f).await; // case A opened (v1 -> 2)
    let a = anchor(&pool, &f).await;
    // Forged RESOLVED for a *different* case id that was never requested.
    inject_assistance_event(
        &pool,
        &a,
        "ASSISTANCE_RESOLVED",
        Uuid::new_v4(),
        Some("OWNER_PENDING"),
        "RESOLVED",
    )
    .await;
    let _ = opened;
    assert_invalid_facts(rebuild_projection(&pool, rebuild_command(&f)).await);
}

#[sqlx::test]
async fn replay_rejects_assistance_event_after_resolution(pool: PgPool) {
    let f = setup(&pool).await;
    let requested = request_case(&pool, &f).await; // v1 -> 2
    resolve(&pool, &f, requested.assistance_case_id, 2).await; // v2 -> 3 (RESOLVED)
    let a = anchor(&pool, &f).await;
    inject_assistance_event(
        &pool,
        &a,
        "ASSISTANCE_ESCALATED_TO_HUMAN",
        requested.assistance_case_id,
        Some("OWNER_PENDING"),
        "HUMAN_REQUIRED",
    )
    .await;
    assert_invalid_facts(rebuild_projection(&pool, rebuild_command(&f)).await);
}

#[sqlx::test]
async fn replay_rejects_previous_status_that_disagrees_with_replayed_state(pool: PgPool) {
    let f = setup(&pool).await;
    let requested = request_case(&pool, &f).await; // v1 -> 2 (OWNER_PENDING)
    escalate(&pool, &f, requested.assistance_case_id, 2).await; // v2 -> 3 (HUMAN_REQUIRED)
    let a = anchor(&pool, &f).await;
    // Shape-valid resolve (previousStatus OWNER_PENDING is allowed), but the
    // replayed status is HUMAN_REQUIRED — the embedded previousStatus must
    // match the tracked state.
    inject_assistance_event(
        &pool,
        &a,
        "ASSISTANCE_RESOLVED",
        requested.assistance_case_id,
        Some("OWNER_PENDING"),
        "RESOLVED",
    )
    .await;
    assert_invalid_facts(rebuild_projection(&pool, rebuild_command(&f)).await);
}

#[sqlx::test]
async fn replay_rejects_transition_while_a_case_is_still_open(pool: PgPool) {
    let f = setup(&pool).await;
    request_case(&pool, &f).await; // v1 -> 2, case still OWNER_PENDING on V1
    inject_transition_event_after_open_case(&pool, &f).await; // forged V1 -> V2
    assert_invalid_facts(rebuild_projection(&pool, rebuild_command(&f)).await);
}
