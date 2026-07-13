//! Context validation tests (20-24).

use super::*;

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
