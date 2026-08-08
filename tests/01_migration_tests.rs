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
async fn test_migration_0016_applied() {
    let pool = common::create_pool().await;

    // Verify the canonical 0016 (instance assignees & artifact lineage
    // reconciliation) is in the ledger and successful.
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM _sqlx_migrations WHERE version = 16 AND success",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert_eq!(row.0, 1, "migration 0016 must be applied and successful");
}

#[tokio::test]
async fn test_migration_0017_applied() {
    let pool = common::create_pool().await;

    // Verify the constraint reconciliation migration 0017 is in the ledger.
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM _sqlx_migrations WHERE version = 17 AND success",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert_eq!(row.0, 1, "migration 0017 must be applied and successful");
}

#[tokio::test]
async fn test_migration_0017_reconciles_canonical_checks_exactly_once() {
    let pool = common::create_pool().await;

    // The 4 canonical CHECK constraints must exist exactly once each,
    // with the canonical names and definitions.
    let expected = [
        (
            "workflow_instances_subject_id_check",
            "subject_id",
            "512",
        ),
        (
            "workflow_instances_artifact_id_check",
            "artifact_id",
            "512",
        ),
        (
            "workflow_instances_artifact_version_check",
            "artifact_version",
            "512",
        ),
        (
            "workflow_instances_artifact_digest_check",
            "artifact_digest",
            "64",
        ),
    ];
    for (name, column, limit) in expected {
        // Exactly one constraint with the canonical name on workflow_instances.
        let rows: Vec<(bool, String, String)> = sqlx::query_as(
            "SELECT c.convalidated, \
                    (SELECT string_agg(a.attname, ',' ORDER BY u.ord) \
                     FROM unnest(c.conkey) WITH ORDINALITY AS u(attnum, ord) \
                     JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = u.attnum), \
                    pg_get_constraintdef(c.oid) \
             FROM pg_constraint c \
             WHERE c.conrelid = 'workflow_instances'::regclass AND c.conname = $1",
        )
        .bind(name)
        .fetch_all(&pool)
        .await
        .expect("query failed");
        assert_eq!(rows.len(), 1, "constraint '{name}' must exist exactly once");
        let (validated, columns, definition) = &rows[0];
        assert!(
            *validated,
            "constraint '{name}' must be validated (not NOT VALID)"
        );
        assert_eq!(
            columns, column,
            "constraint '{name}' must be attached to column '{column}'"
        );
        // pg_get_constraintdef returns the normalized form (extra parens,
        // explicit ::text casts); check the column and the length/pattern
        // limit survive in the definition.
        let normalized: String = definition.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            normalized.contains(column) && normalized.contains(limit),
            "constraint '{name}' definition mismatch: {definition}"
        );
    }
}

#[tokio::test]
async fn test_migration_0016_schema_objects_exist() {
    let pool = common::create_pool().await;

    // 0016 reconciles the historical B schema: the per-instance node
    // assignees table plus the artifact-binding columns on workflow_instances.
    let table_row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM pg_tables \
         WHERE tablename = 'workflow_instance_node_assignees' AND schemaname = 'public'",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");
    assert_eq!(
        table_row.0, 1,
        "table 'workflow_instance_node_assignees' must exist after migration 0016"
    );

    let columns = vec![
        "require_explicit_node_assignees",
        "subject_id",
        "artifact_id",
        "artifact_version",
        "artifact_digest",
        "require_artifact_binding",
    ];
    for column in &columns {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::int8 FROM information_schema.columns \
             WHERE table_name = 'workflow_instances' AND column_name = $1",
        )
        .bind(column)
        .fetch_one(&pool)
        .await
        .expect("query failed");
        assert_eq!(
            row.0, 1,
            "column '{column}' must exist on workflow_instances after migration 0016"
        );
    }
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
