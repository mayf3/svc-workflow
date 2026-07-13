//! Context validation tests.
//!
//! Covers: valid context, size limits, pre-transaction rejection,
//! and non-null context_schema validation (both valid and invalid payloads).

use super::*;

/// JSON Schema that requires title (string, minLength 1) and priority (integer >= 0),
/// and forbids additional properties.
fn required_fields_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["title", "priority"],
        "properties": {
            "title": {"type": "string", "minLength": 1},
            "priority": {"type": "integer", "minimum": 0}
        },
        "additionalProperties": false
    })
}

#[tokio::test]
async fn test_valid_context_accepted() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let mut cmd = make_command(principal_id, domain_id, ver_id);
    cmd.context_payload = serde_json::json!({"any": "value"});
    let result = create_workflow_instance(&pool, cmd)
        .await
        .expect("should succeed");
    verify_creation(&pool, &result, principal_id, domain_id, ver_id).await;
}

#[tokio::test]
async fn test_context_payload_too_large_rejected() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let big_str = "x".repeat(1024 * 1024 + 1);
    let mut cmd = make_command(principal_id, domain_id, ver_id);
    cmd.context_payload = serde_json::json!({"data": big_str});
    let err = create_workflow_instance(&pool, cmd).await.unwrap_err();
    assert!(matches!(
        err,
        CreateWorkflowInstanceError::SizeLimitExceeded(_)
    ));
}

#[tokio::test]
async fn test_metadata_too_large_rejected() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let big_str = "x".repeat(64 * 1024 + 1);
    let mut cmd = make_command(principal_id, domain_id, ver_id);
    cmd.metadata = serde_json::json!({"data": big_str});
    let err = create_workflow_instance(&pool, cmd).await.unwrap_err();
    assert!(matches!(
        err,
        CreateWorkflowInstanceError::SizeLimitExceeded(_)
    ));
}

#[tokio::test]
async fn test_failure_no_runtime_artifacts_left() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let big_str = "x".repeat(64 * 1024 + 1);
    let mut cmd = make_command(principal_id, domain_id, ver_id);
    cmd.metadata = serde_json::json!({"data": big_str});
    let idem_key = cmd.idempotency_key.clone();
    let err = create_workflow_instance(&pool, cmd).await;
    assert!(err.is_err());
    let receipt_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflow_command_receipts WHERE principal_id = $1 AND idempotency_key = $2)",
    ).bind(principal_id).bind(&idem_key).fetch_one(&pool).await.expect("check");
    assert!(!receipt_exists, "no receipt for pre-transaction failure");
}

// ---------------------------------------------------------------------------
// Non-null context_schema tests (H2 coverage)
// ---------------------------------------------------------------------------

/// Helper: attempt creation with a context_payload and assert schema validation fails.
///
/// Note: The current code path for schema validation failure does NOT persist
/// the PROCESSING receipt (the error propagates via `?` without calling
/// `complete_receipt` + `commit`). As a result, the entire transaction
/// (including the receipt INSERT) is rolled back.
/// This differs from other deterministic failures (domain disabled, etc.)
/// that explicitly complete the receipt before returning.
async fn assert_schema_rejection(
    pool: &PgPool,
    cmd: CreateWorkflowInstanceCommand,
    principal_id: Uuid,
) {
    let idem_key = cmd.idempotency_key.clone();
    let err = create_workflow_instance(pool, cmd).await;

    assert!(
        matches!(
            err,
            Err(CreateWorkflowInstanceError::ContextValidationFailed(_))
        ),
        "expected ContextValidationFailed, got {:?}",
        err
    );

    // No receipt — the transaction (including the PROCESSING receipt INSERT) is rolled back
    let receipt_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflow_command_receipts WHERE principal_id = $1 AND idempotency_key = $2)",
    ).bind(principal_id).bind(&idem_key).fetch_one(pool).await.expect("check");
    assert!(
        !receipt_exists,
        "no receipt after schema rejection (tx fully rolled back)"
    );

    // No runtime facts created
    let instance_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_instances WHERE created_by_principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .expect("count");
    assert_eq!(instance_count, 0, "no instance after schema rejection");

    let ctx_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_context_revisions cr \
         JOIN workflow_instances i ON i.workflow_instance_id = cr.workflow_instance_id \
         WHERE i.created_by_principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .expect("count");
    assert_eq!(ctx_count, 0, "no context revision after schema rejection");

    let visit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_node_visits nv \
         JOIN workflow_instances i ON i.workflow_instance_id = nv.workflow_instance_id \
         WHERE i.created_by_principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .expect("count");
    assert_eq!(visit_count, 0, "no visit after schema rejection");

    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_events e \
         JOIN workflow_instances i ON i.workflow_instance_id = e.workflow_instance_id \
         WHERE i.created_by_principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .expect("count");
    assert_eq!(event_count, 0, "no event after schema rejection");
}

#[tokio::test]
async fn test_context_schema_valid_accepted() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) =
        seed_published_definition_with_schema(&pool, domain_id, &required_fields_schema()).await;

    let mut cmd = make_command(principal_id, domain_id, ver_id);
    cmd.context_payload = serde_json::json!({"title": "test", "priority": 1});

    let result = create_workflow_instance(&pool, cmd)
        .await
        .expect("valid schema context should succeed");
    verify_creation(&pool, &result, principal_id, domain_id, ver_id).await;
}

#[tokio::test]
async fn test_context_schema_required_field_missing() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) =
        seed_published_definition_with_schema(&pool, domain_id, &required_fields_schema()).await;

    let mut cmd = make_command(principal_id, domain_id, ver_id);
    cmd.context_payload = serde_json::json!({"priority": 1});

    assert_schema_rejection(&pool, cmd, principal_id).await;
}

#[tokio::test]
async fn test_context_schema_type_error_rejected() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) =
        seed_published_definition_with_schema(&pool, domain_id, &required_fields_schema()).await;

    let mut cmd = make_command(principal_id, domain_id, ver_id);
    cmd.context_payload = serde_json::json!({"title": "x", "priority": "high"});

    assert_schema_rejection(&pool, cmd, principal_id).await;
}

#[tokio::test]
async fn test_context_schema_additional_properties_rejected() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) =
        seed_published_definition_with_schema(&pool, domain_id, &required_fields_schema()).await;

    let mut cmd = make_command(principal_id, domain_id, ver_id);
    cmd.context_payload = serde_json::json!({"title": "x", "priority": 1, "extra": "oops"});

    assert_schema_rejection(&pool, cmd, principal_id).await;
}

#[tokio::test]
async fn test_context_schema_local_ref_accepted() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;

    // Schema with a local $ref using #/$defs
    let schema = serde_json::json!({
        "$defs": {
            "positiveInt": {
                "type": "integer",
                "minimum": 1
            }
        },
        "type": "object",
        "properties": {
            "count": {"$ref": "#/$defs/positiveInt"}
        },
        "additionalProperties": false
    });

    let (_d, ver_id) = seed_published_definition_with_schema(&pool, domain_id, &schema).await;

    let mut cmd = make_command(principal_id, domain_id, ver_id);
    cmd.context_payload = serde_json::json!({"count": 5});

    let result = create_workflow_instance(&pool, cmd)
        .await
        .expect("local $ref context should succeed");
    verify_creation(&pool, &result, principal_id, domain_id, ver_id).await;
}
