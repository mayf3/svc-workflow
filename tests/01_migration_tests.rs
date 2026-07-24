#![allow(clippy::needless_borrow)]
//! Test: migrations apply cleanly on an empty database.

mod common;

#[tokio::test]
async fn test_migrations_apply_successfully() {
    let pool = common::create_pool().await;

    // Verify that key tables exist
    let tables = vec![
        "principals",
        "domains",
        "domain_role_bindings",
        "workflow_definitions",
        "workflow_definition_versions",
        "workflow_node_definitions",
        "workflow_transition_definitions",
        "workflow_instances",
        "workflow_context_revisions",
        "workflow_node_visits",
        "workflow_submissions",
        "workflow_events",
        "workflow_command_receipts",
        "workflow_command_attempt_audits",
        "workflow_security_audits",
    ];

    for table in &tables {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::int8 FROM pg_tables WHERE tablename = $1 AND schemaname = 'public'",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .expect("query failed");

        assert!(row.0 > 0, "table '{}' does not exist", table);
    }
}

#[tokio::test]
async fn test_migration_0012_applied() {
    let pool = common::create_pool().await;

    // Verify migration 12 (restore_workflow_state_version_constraint) is in the ledger
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM _sqlx_migrations WHERE version = 12 AND success",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert_eq!(row.0, 1, "migration 0012 must be applied and successful");
}

#[tokio::test]
async fn test_migration_0013_applied() {
    let pool = common::create_pool().await;

    // Verify migration 13 (INSTANCE_INPUT_PRINCIPAL enum) is in the ledger
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM _sqlx_migrations WHERE version = 13 AND success",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert_eq!(row.0, 1, "migration 0013 must be applied and successful");
}

#[tokio::test]
async fn test_migration_0014_applied() {
    let pool = common::create_pool().await;

    // Verify migration 14 (assignee_input_key column + constraint) is in the ledger
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM _sqlx_migrations WHERE version = 14 AND success",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert_eq!(row.0, 1, "migration 0014 must be applied and successful");
}

#[tokio::test]
async fn test_instance_input_principal_enum_exists() {
    let pool = common::create_pool().await;

    // Verify the INSTANCE_INPUT_PRINCIPAL enum value exists after migration 0013
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM pg_enum e JOIN pg_type t ON e.enumtypid = t.oid \
         WHERE t.typname = 'assignee_ref_type' AND e.enumlabel = 'INSTANCE_INPUT_PRINCIPAL'",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert_eq!(
        row.0, 1,
        "enum value 'INSTANCE_INPUT_PRINCIPAL' must exist in assignee_ref_type"
    );
}

#[tokio::test]
async fn test_assignee_input_key_column_exists() {
    let pool = common::create_pool().await;

    // Verify the assignee_input_key column exists after migration 0014
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM information_schema.columns \
         WHERE table_name = 'workflow_node_definitions' AND column_name = 'assignee_input_key'",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert_eq!(
        row.0, 1,
        "column 'assignee_input_key' must exist on workflow_node_definitions"
    );
}

#[tokio::test]
async fn test_workflow_state_version_constraint_present() {
    let pool = common::create_pool().await;

    // Verify the constraint exists after migration 0012
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM pg_constraint WHERE conname = 'workflow_instances_workflow_state_version_check'",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert_eq!(
        row.0, 1,
        "constraint 'workflow_instances_workflow_state_version_check' must exist"
    );
}

#[tokio::test]
async fn test_invalid_state_version_rejected() {
    let pool = common::create_pool().await;

    let (creator_id, domain_id) = common::seed_principal_and_domain(&pool).await;
    let (_, def_ver_id, _node_id, _) = common::seed_workflow_definition(&pool, domain_id).await;

    let instance_id = uuid::Uuid::new_v4();

    // Try to insert with workflow_state_version = 0
    let result = sqlx::query(
        r#"
        INSERT INTO workflow_instances
            (workflow_instance_id, domain_id, definition_version_id,
             created_by_principal_id, workflow_state_version)
        VALUES ($1, $2, $3, $4, 0)
        "#,
    )
    .bind(instance_id)
    .bind(domain_id)
    .bind(def_ver_id)
    .bind(creator_id)
    .execute(&pool)
    .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("check constraint") || err_str.contains("CHECK"),
                "expected CHECK constraint violation for version=0, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected CHECK constraint failure for workflow_state_version=0"),
    }
}

#[tokio::test]
async fn test_enums_exist() {
    let pool = common::create_pool().await;

    let enums = vec![
        "principal_type",
        "definition_version_status",
        "node_type",
        "assignee_ref_type",
        "transition_effect",
        "receipt_status",
    ];

    for enum_name in &enums {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::int8 FROM pg_type WHERE typname = $1 AND typtype = 'e'",
        )
        .bind(enum_name)
        .fetch_one(&pool)
        .await
        .expect("query failed");

        assert!(row.0 > 0, "enum '{}' does not exist", enum_name);
    }
}
