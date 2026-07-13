//! Atomicity tests: event failure, infrastructure failure, and deterministic failure.

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

#[tokio::test]
async fn test_event_failure_rolls_back_everything() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;

    // Install a temporary trigger that fails on event INSERT
    sqlx::query(
        r#"CREATE OR REPLACE FUNCTION fn_test_fail_event() RETURNS TRIGGER AS $$
        BEGIN RAISE EXCEPTION 'test_injected_failure: event insert blocked' USING ERRCODE = '23000'; END;
        $$ LANGUAGE plpgsql"#,
    ).execute(&pool).await.expect("create function");
    sqlx::query(
        "CREATE OR REPLACE TRIGGER trg_test_fail_event BEFORE INSERT ON workflow_events FOR EACH ROW EXECUTE FUNCTION fn_test_fail_event()",
    ).execute(&pool).await.expect("create trigger");

    let cmd = make_command(principal_id, domain_id, ver_id);
    let err = create_workflow_instance(&pool, cmd).await;

    // Clean up trigger
    sqlx::query("DROP TRIGGER IF EXISTS trg_test_fail_event ON workflow_events")
        .execute(&pool)
        .await
        .expect("drop trigger");
    sqlx::query("DROP FUNCTION IF EXISTS fn_test_fail_event()")
        .execute(&pool)
        .await
        .expect("drop function");

    assert!(
        err.is_err(),
        "creation must fail when event insert is blocked"
    );

    // No instance for this principal
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

    // Install a trigger that fails on instance INSERT
    sqlx::query(
        r#"CREATE OR REPLACE FUNCTION fn_test_fail_instance() RETURNS TRIGGER AS $$
        BEGIN RAISE EXCEPTION 'test_injected_failure: instance insert blocked' USING ERRCODE = '23000'; END;
        $$ LANGUAGE plpgsql"#,
    ).execute(&pool).await.expect("create function");
    sqlx::query(
        "CREATE OR REPLACE TRIGGER trg_test_fail_instance BEFORE INSERT ON workflow_instances FOR EACH ROW EXECUTE FUNCTION fn_test_fail_instance()",
    ).execute(&pool).await.expect("create trigger");

    let cmd = make_command(principal_id, domain_id, ver_id);
    let err = create_workflow_instance(&pool, cmd).await;

    // Clean up
    sqlx::query("DROP TRIGGER IF EXISTS trg_test_fail_instance ON workflow_instances")
        .execute(&pool)
        .await
        .expect("drop trigger");
    sqlx::query("DROP FUNCTION IF EXISTS fn_test_fail_instance()")
        .execute(&pool)
        .await
        .expect("drop function");

    assert!(err.is_err(), "infrastructure failure must return error");

    // No receipt for this principal (transaction fully rolled back)
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

    // Receipt exists (deterministic failure is persisted)
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
