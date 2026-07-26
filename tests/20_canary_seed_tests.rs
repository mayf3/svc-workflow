#![allow(clippy::needless_borrow)]
//! Test: Canary seed data integrity and re-execution idempotency.
//!
//! Verifies that the seed script in scripts/canary/seed_canary_test_data.sql
//! creates consistent workflow instances with proper initial events,
//! and that re-executing the seed does not produce duplicate data.

mod common;

/// Known UUIDs used by the seed script.
const DOMAIN_ID: &str = "cccccccc-cccc-4ccc-cccc-cccccccccccc";
const DEFINITION_VERSION_ID: &str = "eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee";
const INSTANCE_1_ID: &str = "11111111-1111-4111-1111-111111111111";
const INSTANCE_2_ID: &str = "44444444-4444-4444-4444-444444444444";
const VISIT_1_ID: &str = "33333333-3333-4333-3333-333333333333";
const VISIT_2_ID: &str = "66666666-6666-4666-6666-666666666666";
const EVENT_1_ID: &str = "00000000-0000-4000-a000-000000000001";
const EVENT_2_ID: &str = "00000000-0000-4000-a000-000000000002";

/// Parse a UUID string constant into a uuid::Uuid.
fn uid(s: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(s).expect("valid UUID constant")
}

/// Run the canary seed SQL against the given pool.
async fn run_seed(pool: &sqlx::PgPool) {
    let seed_sql = std::fs::read_to_string("scripts/canary/seed_canary_test_data.sql")
        .expect("failed to read seed_canary_test_data.sql");

    // Execute the full seed script.
    // PostgreSQL allows multiple statements in a single execute via the simple query protocol.
    let result = sqlx::raw_sql(&seed_sql).execute(pool).await;
    result.expect("seed SQL execution failed");
}

/// Count the number of rows in a table for the given instance.
async fn count_events(pool: &sqlx::PgPool, instance_id: uuid::Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::int8 FROM workflow_events WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .expect("query failed")
}

/// Get the workflow_state_version for an instance.
async fn get_state_version(pool: &sqlx::PgPool, instance_id: uuid::Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT workflow_state_version::int8 FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .expect("query failed")
}

// ============================================================
// Tests
// ============================================================

#[tokio::test]
async fn seed_instance_has_initial_created_event() {
    let pool = common::create_pool().await;
    run_seed(&pool).await;

    let cnt1 = count_events(&pool, uid(INSTANCE_1_ID)).await;
    let cnt2 = count_events(&pool, uid(INSTANCE_2_ID)).await;

    assert!(
        cnt1 >= 1,
        "instance 1 must have at least 1 event (WORKFLOW_INSTANCE_CREATED), got {}",
        cnt1
    );
    assert!(
        cnt2 >= 1,
        "instance 2 must have at least 1 event (WORKFLOW_INSTANCE_CREATED), got {}",
        cnt2
    );

    // Verify the event is the expected type
    let event_type: String =
        sqlx::query_scalar("SELECT event_type FROM workflow_events WHERE event_id = $1")
            .bind(uid(EVENT_1_ID))
            .fetch_one(&pool)
            .await
            .expect("query failed");
    assert_eq!(
        event_type, "WORKFLOW_INSTANCE_CREATED",
        "event 1 must be WORKFLOW_INSTANCE_CREATED"
    );

    let event_type: String =
        sqlx::query_scalar("SELECT event_type FROM workflow_events WHERE event_id = $1")
            .bind(uid(EVENT_2_ID))
            .fetch_one(&pool)
            .await
            .expect("query failed");
    assert_eq!(
        event_type, "WORKFLOW_INSTANCE_CREATED",
        "event 2 must be WORKFLOW_INSTANCE_CREATED"
    );
}

#[tokio::test]
async fn seed_event_count_matches_state_version() {
    let pool = common::create_pool().await;
    run_seed(&pool).await;

    // Instance 1
    let cnt1 = count_events(&pool, uid(INSTANCE_1_ID)).await;
    let sv1 = get_state_version(&pool, uid(INSTANCE_1_ID)).await;
    assert_eq!(
        cnt1, sv1,
        "instance 1: event_count ({}) must equal workflow_state_version ({})",
        cnt1, sv1
    );

    // Instance 2
    let cnt2 = count_events(&pool, uid(INSTANCE_2_ID)).await;
    let sv2 = get_state_version(&pool, uid(INSTANCE_2_ID)).await;
    assert_eq!(
        cnt2, sv2,
        "instance 2: event_count ({}) must equal workflow_state_version ({})",
        cnt2, sv2
    );
}

#[tokio::test]
async fn seed_reexecution_idempotent() {
    let pool = common::create_pool().await;

    // First execution
    run_seed(&pool).await;

    let cnt1_first = count_events(&pool, uid(INSTANCE_1_ID)).await;
    let cnt2_first = count_events(&pool, uid(INSTANCE_2_ID)).await;

    // Second execution (must not create duplicates)
    run_seed(&pool).await;

    let cnt1_second = count_events(&pool, uid(INSTANCE_1_ID)).await;
    let cnt2_second = count_events(&pool, uid(INSTANCE_2_ID)).await;

    assert_eq!(
        cnt1_first, cnt1_second,
        "instance 1 event count must not change after re-execution"
    );
    assert_eq!(
        cnt2_first, cnt2_second,
        "instance 2 event count must not change after re-execution"
    );
}

#[tokio::test]
async fn seeded_worklist_query_pass() {
    let pool = common::create_pool().await;
    run_seed(&pool).await;

    // Verify that the worklist-assigned-to-me query can run without error.
    // We query the raw tables with the same consistency checks the application
    // uses (event_count, event_sequence, workflow_state_version alignment).

    // Check min_event_sequence = 1 and max_event_sequence = workflow_state_version
    // for instance 1
    let row: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::int8 AS event_count,
            COALESCE(MIN(event_sequence), 0)::int8 AS min_seq,
            COALESCE(MAX(event_sequence), 0)::int8 AS max_seq
        FROM workflow_events
        WHERE workflow_instance_id = $1
        "#,
    )
    .bind(uid(INSTANCE_1_ID))
    .fetch_one(&pool)
    .await
    .expect("query failed");

    let (cnt, min_seq, max_seq) = row;
    assert_eq!(min_seq, 1, "instance 1: min event_sequence must be 1");
    assert_eq!(
        max_seq, cnt,
        "instance 1: max event_sequence ({}) must equal event_count ({})",
        max_seq, cnt
    );

    // Check for instance 2
    let row: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::int8 AS event_count,
            COALESCE(MIN(event_sequence), 0)::int8 AS min_seq,
            COALESCE(MAX(event_sequence), 0)::int8 AS max_seq
        FROM workflow_events
        WHERE workflow_instance_id = $1
        "#,
    )
    .bind(uid(INSTANCE_2_ID))
    .fetch_one(&pool)
    .await
    .expect("query failed");

    let (cnt, min_seq, max_seq) = row;
    assert_eq!(min_seq, 1, "instance 2: min event_sequence must be 1");
    assert_eq!(
        max_seq, cnt,
        "instance 2: max event_sequence ({}) must equal event_count ({})",
        max_seq, cnt
    );
}

#[tokio::test]
async fn seeded_detail_query_pass() {
    let pool = common::create_pool().await;
    run_seed(&pool).await;

    // Verify instance detail fields can be queried and return expected values.
    let row: (
        uuid::Uuid,
        uuid::Uuid,
        uuid::Uuid,
        i32,
        uuid::Uuid,
        uuid::Uuid,
    ) = sqlx::query_as(
        r#"
        SELECT
            workflow_instance_id,
            domain_id,
            definition_version_id,
            workflow_state_version,
            current_context_revision_id,
            current_node_visit_id
        FROM workflow_instances
        WHERE workflow_instance_id = $1
        "#,
    )
    .bind(uid(INSTANCE_1_ID))
    .fetch_one(&pool)
    .await
    .expect("instance 1 detail query failed");

    let (inst_id, dom_id, def_ver_id, state_ver, ctx_id, visit_id) = row;
    assert_eq!(inst_id, uid(INSTANCE_1_ID));
    assert_eq!(dom_id, uid(DOMAIN_ID));
    assert_eq!(def_ver_id, uid(DEFINITION_VERSION_ID));
    assert_eq!(state_ver, 1);
    assert_eq!(ctx_id, uid("22222222-2222-4222-2222-222222222222"));
    assert_eq!(visit_id, uid(VISIT_1_ID));

    // Instance 2
    let row: (
        uuid::Uuid,
        uuid::Uuid,
        uuid::Uuid,
        i32,
        uuid::Uuid,
        uuid::Uuid,
    ) = sqlx::query_as(
        r#"
        SELECT
            workflow_instance_id,
            domain_id,
            definition_version_id,
            workflow_state_version,
            current_context_revision_id,
            current_node_visit_id
        FROM workflow_instances
        WHERE workflow_instance_id = $1
        "#,
    )
    .bind(uid(INSTANCE_2_ID))
    .fetch_one(&pool)
    .await
    .expect("instance 2 detail query failed");

    let (inst_id, dom_id, def_ver_id, state_ver, ctx_id, visit_id) = row;
    assert_eq!(inst_id, uid(INSTANCE_2_ID));
    assert_eq!(dom_id, uid(DOMAIN_ID));
    assert_eq!(def_ver_id, uid(DEFINITION_VERSION_ID));
    assert_eq!(state_ver, 1);
    assert_eq!(ctx_id, uid("55555555-5555-4555-5555-555555555555"));
    assert_eq!(visit_id, uid(VISIT_2_ID));
}
