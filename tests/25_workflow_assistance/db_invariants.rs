//! DB-level invariant tests for Workflow Assistance V1 (migration 0022).
//!
//! These tests prove the 0022 triggers reject impossible AssistanceCase
//! histories / command-receipt bindings at the DB layer itself — not via the
//! application API. Each negative case constructs a row that is otherwise
//! internally consistent (it passes the 0021 column CHECKs and FKs), so the
//! only thing rejecting it is the new 0022 invariant.

use sqlx::PgPool;
use uuid::Uuid;

use svc_workflow::application::workflow_instance::archive::archive_workflow_instance;
use svc_workflow::application::workflow_instance::cancel::cancel_workflow_instance;
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::{
    ArchiveWorkflowInstanceCommand, CancelWorkflowInstanceCommand,
};

use super::helpers::*;

/// Seed a command receipt in a chosen terminal state, owned by `principal`.
async fn seed_receipt(
    pool: &PgPool,
    principal: Uuid,
    command_type: &str,
    completed: bool,
) -> Uuid {
    let id = Uuid::new_v4();
    if completed {
        sqlx::query(
            "INSERT INTO workflow_command_receipts
             (command_id, principal_id, idempotency_key, command_type, request_hash,
              receipt_status, response_status, response_body, response_digest, completed_at)
             VALUES ($1,$2,$3,$4,$5,'COMPLETED',200,'{}'::jsonb,$6,now())",
        )
        .bind(id)
        .bind(principal)
        .bind(Uuid::new_v4().to_string())
        .bind(command_type)
        .bind(request_hash("synthetic request"))
        .bind(request_hash("synthetic response"))
        .execute(pool)
        .await
        .unwrap();
    } else {
        sqlx::query(
            "INSERT INTO workflow_command_receipts
             (command_id, principal_id, idempotency_key, command_type, request_hash,
              receipt_status)
             VALUES ($1,$2,$3,$4,$5,'PROCESSING')",
        )
        .bind(id)
        .bind(principal)
        .bind(Uuid::new_v4().to_string())
        .bind(command_type)
        .bind(request_hash("synthetic request"))
        .execute(pool)
        .await
        .unwrap();
    }
    id
}

/// Insert a fully-valid OWNER_PENDING case bound to `visit` with an explicit
/// request command id. Passes both 0022 triggers; used as a building block.
async fn insert_valid_owner_pending(
    pool: &PgPool,
    f: &Fixture,
    visit: Uuid,
    request_command_id: Uuid,
) -> Uuid {
    let case_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest,
          request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(case_id)
    .bind(f.instance)
    .bind(visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"valid owner-pending case"}))
    .bind(request_hash("valid payload"))
    .bind(request_command_id)
    .execute(pool)
    .await
    .unwrap();
    case_id
}

fn db_message(err: &sqlx::Error) -> String {
    err.as_database_error()
        .map(|e| e.message().to_string())
        .unwrap_or_default()
}

/// Assert the DB error message contains `needle`, printing the full message on
/// failure so a regression points at the wrong rejector.
fn assert_rejected(err: sqlx::Error, needle: &str) {
    let message = db_message(&err);
    assert!(
        message.contains(needle),
        "expected error message to contain {needle:?}, got: {message}"
    );
}

// ---------------------------------------------------------------------------
// Blocker 1 — initial-state + Visit/lifecycle invariants on INSERT
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn insert_accepts_valid_owner_pending_on_current_live_visit(pool: PgPool) {
    let f = setup(&pool).await;
    let cmd = seed_receipt(&pool, f.agent, "REQUEST_WORKFLOW_ASSISTANCE", true).await;
    let case_id = insert_valid_owner_pending(&pool, &f, f.visit, cmd).await;
    assert_eq!(
        case_status(&pool, case_id).await,
        ("OWNER_PENDING".to_string(), None)
    );
}

#[sqlx::test]
async fn insert_rejects_human_required_initial_status(pool: PgPool) {
    let f = setup(&pool).await;
    let req = seed_receipt(&pool, f.agent, "REQUEST_WORKFLOW_ASSISTANCE", true).await;
    let esc = seed_receipt(&pool, f.owner, "ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN", true).await;
    // Row satisfies the 0021 status CHECK for HUMAN_REQUIRED (escalation group
    // fully populated) — only the 0022 INSERT trigger can reject it.
    let err = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id,
          escalated_by_principal_id, escalation_payload, escalation_payload_digest,
          escalation_command_id, escalated_at)
         VALUES ($1,$2,$3,'HUMAN_REQUIRED',$4,$5,$6,$7,$8,$9,$10,$11,now())",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(f.visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"m"}))
    .bind(request_hash("p"))
    .bind(req)
    .bind(f.owner)
    .bind(serde_json::json!({"message":"e"}))
    .bind(request_hash("e"))
    .bind(esc)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "must start as OWNER_PENDING");
}

#[sqlx::test]
async fn insert_rejects_resolved_initial_status(pool: PgPool) {
    let f = setup(&pool).await;
    let req = seed_receipt(&pool, f.agent, "REQUEST_WORKFLOW_ASSISTANCE", true).await;
    let res = seed_receipt(&pool, f.owner, "RESOLVE_WORKFLOW_ASSISTANCE", true).await;
    let err = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id,
          resolved_by_principal_id, resolution_payload, resolution_payload_digest,
          resolution_command_id, resolved_at)
         VALUES ($1,$2,$3,'RESOLVED',$4,$5,$6,$7,$8,$9,$10,$11,now())",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(f.visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"m"}))
    .bind(request_hash("p"))
    .bind(req)
    .bind(f.owner)
    .bind(serde_json::json!({"message":"r"}))
    .bind(request_hash("r"))
    .bind(res)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "must start as OWNER_PENDING");
}

#[sqlx::test]
async fn insert_rejects_voided_initial_status(pool: PgPool) {
    let f = setup(&pool).await;
    let req = seed_receipt(&pool, f.agent, "REQUEST_WORKFLOW_ASSISTANCE", true).await;
    let void_cmd = seed_receipt(&pool, f.owner, "CANCEL_WORKFLOW_INSTANCE", true).await;
    let err = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id,
          voided_by_principal_id, void_reason_code, voided_by_command_id, voided_at)
         VALUES ($1,$2,$3,'VOIDED',$4,$5,$6,$7,$8,'INSTANCE_CANCELLED',$9,now())",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(f.visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"m"}))
    .bind(request_hash("p"))
    .bind(req)
    .bind(f.owner)
    .bind(void_cmd)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "must start as OWNER_PENDING");
}

#[sqlx::test]
async fn insert_rejects_case_bound_to_a_non_current_visit(pool: PgPool) {
    let f = setup(&pool).await;
    // Advance to the terminal visit; the draft visit is now a valid but
    // non-current visit. The INSERT trigger must reject a case bound to it.
    assert_eq!(transition(&pool, &f, 1).await, 2);
    let req = seed_receipt(&pool, f.agent, "REQUEST_WORKFLOW_ASSISTANCE", true).await;
    let err = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(f.visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"stale"}))
    .bind(request_hash("p"))
    .bind(req)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "must equal instance.current_node_visit_id");
}

#[sqlx::test]
async fn insert_rejects_case_on_a_cancelled_instance(pool: PgPool) {
    let f = setup(&pool).await;
    cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(f.owner),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(f.instance),
            expected_workflow_state_version: 1,
            reason: "cancel for invariant test".to_string(),
        },
        &request_hash("cancel for invariant test"),
    )
    .await
    .unwrap();
    let req = seed_receipt(&pool, f.agent, "REQUEST_WORKFLOW_ASSISTANCE", true).await;
    let err = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(f.visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"after cancel"}))
    .bind(request_hash("p"))
    .bind(req)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "cancelled instance");
}

#[sqlx::test]
async fn insert_rejects_case_on_an_archived_instance(pool: PgPool) {
    let f = setup(&pool).await;
    assert_eq!(transition(&pool, &f, 1).await, 2);
    let terminal_visit: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(f.instance)
    .fetch_one(&pool)
    .await
    .unwrap();
    archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(f.owner),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(f.instance),
            expected_workflow_state_version: 2,
            reason: "archive for invariant test".to_string(),
        },
        &request_hash("archive for invariant test"),
    )
    .await
    .unwrap();
    let req = seed_receipt(&pool, f.agent, "REQUEST_WORKFLOW_ASSISTANCE", true).await;
    let err = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(terminal_visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"after archive"}))
    .bind(request_hash("p"))
    .bind(req)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "archived instance");
}

#[sqlx::test]
async fn insert_rejects_case_on_a_terminal_visit(pool: PgPool) {
    let f = setup(&pool).await;
    assert_eq!(transition(&pool, &f, 1).await, 2);
    let terminal_visit: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(f.instance)
    .fetch_one(&pool)
    .await
    .unwrap();
    let req = seed_receipt(&pool, f.agent, "REQUEST_WORKFLOW_ASSISTANCE", true).await;
    let err = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(terminal_visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"terminal"}))
    .bind(request_hash("p"))
    .bind(req)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "terminal visit");
}

// ---------------------------------------------------------------------------
// Blocker 2 — command-receipt / actor / status integrity (deferred trigger)
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn deferred_trigger_rejects_wrong_request_command_type(pool: PgPool) {
    let f = setup(&pool).await;
    // A COMPLETED receipt, but of the wrong command_type for the request stage.
    let wrong = seed_receipt(&pool, f.agent, "RESOLVE_WORKFLOW_ASSISTANCE", true).await;
    let err = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(f.visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"m"}))
    .bind(request_hash("p"))
    .bind(wrong)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "REQUEST_WORKFLOW_ASSISTANCE receipt");
}

#[sqlx::test]
async fn deferred_trigger_rejects_request_actor_mismatch(pool: PgPool) {
    let f = setup(&pool).await;
    // Correct type and COMPLETED, but the receipt actor is the owner, not the
    // requesting agent.
    let other_actor =
        seed_receipt(&pool, f.owner, "REQUEST_WORKFLOW_ASSISTANCE", true).await;
    let err = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(f.visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"m"}))
    .bind(request_hash("p"))
    .bind(other_actor)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "REQUEST_WORKFLOW_ASSISTANCE receipt");
}

#[sqlx::test]
async fn deferred_trigger_rejects_request_receipt_that_never_completes(pool: PgPool) {
    let f = setup(&pool).await;
    // A still-PROCESSING request receipt: the deferred trigger fires at COMMIT
    // and rejects because the receipt never reached COMPLETED.
    let processing = seed_receipt(&pool, f.agent, "REQUEST_WORKFLOW_ASSISTANCE", false).await;
    let err = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest, request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(f.instance)
    .bind(f.visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"m"}))
    .bind(request_hash("p"))
    .bind(processing)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "REQUEST_WORKFLOW_ASSISTANCE receipt");
}

#[sqlx::test]
async fn deferred_trigger_rejects_cross_stage_command_reuse(pool: PgPool) {
    let f = setup(&pool).await;
    // A legitimately opened case whose request command id is then reused as
    // the resolution command id. The BEFORE UPDATE trigger permits the
    // OWNER_PENDING -> RESOLVED transition; the deferred trigger rejects at
    // COMMIT because that receipt is type REQUEST, not RESOLVE.
    let request_cmd = seed_receipt(&pool, f.agent, "REQUEST_WORKFLOW_ASSISTANCE", true).await;
    let case_id = insert_valid_owner_pending(&pool, &f, f.visit, request_cmd).await;
    let err = sqlx::query(
        "UPDATE workflow_assistance_cases
         SET status='RESOLVED', resolved_by_principal_id=$2,
             resolution_payload=$3, resolution_payload_digest=$4,
             resolution_command_id=$5, resolved_at=now(), updated_at=now()
         WHERE assistance_case_id=$1",
    )
    .bind(case_id)
    .bind(f.owner)
    .bind(serde_json::json!({"message":"r"}))
    .bind(request_hash("r"))
    .bind(request_cmd) // reused across stages
    .execute(&pool)
    .await
    .unwrap_err();
    assert_rejected(err, "RESOLVE_WORKFLOW_ASSISTANCE receipt");
    // The case is unchanged: still OWNER_PENDING.
    assert_eq!(
        case_status(&pool, case_id).await,
        ("OWNER_PENDING".to_string(), None)
    );
}
