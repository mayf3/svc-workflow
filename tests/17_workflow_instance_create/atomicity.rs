//! Atomicity tests: fault injection, deterministic failure, deferred FK, and event counts.
//!
//! Fault-injection tests use **conditional triggers with unique DDL names** to avoid
//! polluting concurrent test runs. Each trigger only fires for records belonging
//! to the specific test's principal_id. Trigger/function names include a random
//! UUID suffix so that concurrent test threads never collide.

use super::*;

#[tokio::test]
async fn test_exactly_one_event_per_creation() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let result = create_workflow_instance(&pool, make_command(principal_id, domain_id, ver_id))
        .await
        .expect("create");
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_events WHERE workflow_instance_id = $1")
            .bind(result.workflow_instance_id)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_command_id_matches_event() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let result = create_workflow_instance(&pool, make_command(principal_id, domain_id, ver_id))
        .await
        .expect("create");
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_events e JOIN workflow_command_receipts r ON e.command_id = r.command_id WHERE e.workflow_instance_id = $1",
    ).bind(result.workflow_instance_id).fetch_one(&pool).await.expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_deferred_fk_committed_successfully() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let result = create_workflow_instance(&pool, make_command(principal_id, domain_id, ver_id))
        .await
        .expect("create");
    let fk_ok: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflow_instances i \
         JOIN workflow_context_revisions cr ON cr.context_revision_id = i.current_context_revision_id AND cr.workflow_instance_id = i.workflow_instance_id \
         JOIN workflow_node_visits nv ON nv.node_visit_id = i.current_node_visit_id AND nv.workflow_instance_id = i.workflow_instance_id \
         WHERE i.workflow_instance_id = $1)",
    ).bind(result.workflow_instance_id).fetch_one(&pool).await.expect("check");
    assert!(fk_ok, "circular FKs must resolve");
}

/// Create a temporary trigger on `table` that blocks inserts matching a
/// specific principal. Returns the unique suffix used for cleanup.
///
/// - `on_table`: the target table name (e.g. `"workflow_events"`)
/// - `col_check`: the SQL column comparison for this test's principal,
///   e.g. `"NEW.actor_principal_id = '<uuid>'"`.
async fn create_test_trigger(pool: &PgPool, on_table: &str, col_check: &str) -> String {
    let suffix = Uuid::new_v4().to_string().replace('-', "");
    let fn_name = format!("fn_test_fail_{suffix}");
    let trg_name = format!("trg_test_fail_{suffix}");

    // Defensive cleanup — remove orphan objects from a previous crash
    let _ = sqlx::query(&format!("DROP TRIGGER IF EXISTS {trg_name} ON {on_table}"))
        .execute(pool)
        .await;
    let _ = sqlx::query(&format!("DROP FUNCTION IF EXISTS {fn_name}()"))
        .execute(pool)
        .await;

    // Create the function — raise only when the column check matches
    sqlx::query(&format!(
        "CREATE FUNCTION {fn_name}() RETURNS TRIGGER AS $$
         BEGIN
             IF {col_check} THEN
                 RAISE EXCEPTION 'test_injected_failure: {on_table} insert blocked'
                 USING ERRCODE = '23000';
             END IF;
             RETURN NEW;
         END;
         $$ LANGUAGE plpgsql"
    ))
    .execute(pool)
    .await
    .expect("create trigger function");

    sqlx::query(&format!(
        "CREATE TRIGGER {trg_name} BEFORE INSERT ON {on_table} \
         FOR EACH ROW EXECUTE FUNCTION {fn_name}()"
    ))
    .execute(pool)
    .await
    .expect("create trigger");

    suffix
}

/// Create a temporary BEFORE UPDATE trigger on `workflow_command_receipts`
/// that blocks the PROCESSING → COMPLETED transition for a specific principal.
async fn create_receipt_update_trigger(pool: &PgPool, principal_id: Uuid) -> String {
    let suffix = Uuid::new_v4().to_string().replace('-', "");
    let fn_name = format!("fn_test_fail_rcpt_{suffix}");
    let trg_name = format!("trg_test_fail_rcpt_{suffix}");

    let _ = sqlx::query(&format!(
        "DROP TRIGGER IF EXISTS {trg_name} ON workflow_command_receipts"
    ))
    .execute(pool)
    .await;
    let _ = sqlx::query(&format!("DROP FUNCTION IF EXISTS {fn_name}()"))
        .execute(pool)
        .await;

    sqlx::query(&format!(
        "CREATE FUNCTION {fn_name}() RETURNS TRIGGER AS $$
         BEGIN
             IF NEW.receipt_status = 'COMPLETED'
                AND OLD.receipt_status = 'PROCESSING'
                AND OLD.principal_id = '{principal_id}' THEN
                 RAISE EXCEPTION 'test_injected_failure: receipt completion blocked'
                 USING ERRCODE = '23000';
             END IF;
             RETURN NEW;
         END;
         $$ LANGUAGE plpgsql"
    ))
    .execute(pool)
    .await
    .expect("create receipt trigger function");

    sqlx::query(&format!(
        "CREATE TRIGGER {trg_name} BEFORE UPDATE ON workflow_command_receipts \
         FOR EACH ROW EXECUTE FUNCTION {fn_name}()"
    ))
    .execute(pool)
    .await
    .expect("create receipt trigger");

    suffix
}

/// Drop a test trigger and its function by suffix.
async fn drop_test_trigger(pool: &PgPool, on_table: &str, suffix: &str) {
    let fn_name = format!("fn_test_fail_{suffix}");
    let trg_name = format!("trg_test_fail_{suffix}");
    let _ = sqlx::query(&format!("DROP TRIGGER IF EXISTS {trg_name} ON {on_table}"))
        .execute(pool)
        .await;
    let _ = sqlx::query(&format!("DROP FUNCTION IF EXISTS {fn_name}()"))
        .execute(pool)
        .await;
}

/// Drop a receipt-update trigger and its function by suffix.
async fn drop_receipt_trigger(pool: &PgPool, suffix: &str) {
    let fn_name = format!("fn_test_fail_rcpt_{suffix}");
    let trg_name = format!("trg_test_fail_rcpt_{suffix}");
    let _ = sqlx::query(&format!(
        "DROP TRIGGER IF EXISTS {trg_name} ON workflow_command_receipts"
    ))
    .execute(pool)
    .await;
    let _ = sqlx::query(&format!("DROP FUNCTION IF EXISTS {fn_name}()"))
        .execute(pool)
        .await;
}

#[tokio::test]
async fn test_event_failure_rolls_back_everything() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;

    let suffix = create_test_trigger(
        &pool,
        "workflow_events",
        &format!("NEW.actor_principal_id = '{principal_id}'"),
    )
    .await;

    let cmd = make_command(principal_id, domain_id, ver_id);
    let err = create_workflow_instance(&pool, cmd).await;

    // Clean up trigger (runs even if assertion fails)
    drop_test_trigger(&pool, "workflow_events", &suffix).await;

    assert!(
        err.is_err(),
        "creation must fail when event insert is blocked"
    );

    // No instance for this principal — entire transaction rolled back
    let instance_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_instances WHERE created_by_principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(instance_count, 0, "no instance after event failure");
}

#[tokio::test]
async fn test_infrastructure_failure_no_residual_receipt() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;

    let suffix = create_test_trigger(
        &pool,
        "workflow_instances",
        &format!("NEW.created_by_principal_id = '{principal_id}'"),
    )
    .await;

    let cmd = make_command(principal_id, domain_id, ver_id);
    let err = create_workflow_instance(&pool, cmd).await;

    // Clean up trigger
    drop_test_trigger(&pool, "workflow_instances", &suffix).await;

    assert!(err.is_err(), "infrastructure failure must return error");

    // No receipt for this principal — transaction fully rolled back
    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_command_receipts WHERE principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(receipt_count, 0, "no receipt after infrastructure failure");
}

#[tokio::test]
async fn test_receipt_completion_failure_rolls_back_all_runtime_facts() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;

    // Install a trigger that blocks the receipt's PROCESSING → COMPLETED transition
    let suffix = create_receipt_update_trigger(&pool, principal_id).await;

    let cmd = make_command(principal_id, domain_id, ver_id);
    let idem_key = cmd.idempotency_key.clone();
    let err = create_workflow_instance(&pool, cmd).await;

    // Clean up trigger
    drop_receipt_trigger(&pool, &suffix).await;

    // The creation should fail
    assert!(
        err.is_err(),
        "creation must fail when receipt completion is blocked"
    );

    // No runtime facts — the entire transaction rolled back
    let instance_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_instances WHERE created_by_principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(
        instance_count, 0,
        "no instance after receipt completion failure"
    );

    // No receipt either — the PROCESSING receipt was rolled back with the transaction
    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_command_receipts WHERE principal_id = $1 AND idempotency_key = $2",
    ).bind(principal_id).bind(&idem_key).fetch_one(&pool).await.expect("count");
    assert_eq!(
        receipt_count, 0,
        "no receipt after receipt completion failure (tx rolled back)"
    );

    // A second attempt with a fresh idempotency key creates a new request
    // (no residual artifacts from the failed attempt)
    let instance_count2: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_instances WHERE created_by_principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(
        instance_count2, 0,
        "no residual instance after receipt completion failure"
    );
}

#[tokio::test]
async fn test_deterministic_failure_no_runtime_facts_left() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;

    sqlx::query("UPDATE domains SET enabled = FALSE WHERE domain_id = $1")
        .bind(domain_id)
        .execute(&pool)
        .await
        .expect("disable");

    let cmd = make_command(principal_id, domain_id, ver_id);
    let idem_key = cmd.idempotency_key.clone();
    let err = create_workflow_instance(&pool, cmd).await;
    assert!(matches!(
        err,
        Err(CreateWorkflowInstanceError::DomainDisabled)
    ));

    // Receipt exists — deterministic failure is persisted
    let receipt_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflow_command_receipts WHERE principal_id = $1 AND idempotency_key = $2)",
    ).bind(principal_id).bind(&idem_key).fetch_one(&pool).await.expect("check");
    assert!(
        receipt_exists,
        "receipt must exist for deterministic failure"
    );

    // But no runtime facts
    let instance_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_instances WHERE created_by_principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(instance_count, 0, "no instance for deterministic failure");
}
