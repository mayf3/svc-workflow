use super::*;

use svc_workflow::application::workflow_instance::admin_recovery::rebuild_projection;
use svc_workflow::domain::definition::digest;
use svc_workflow::domain::workflow_instance::recovery::{RebuildProjectionCommand, RecoveryError};

async fn tamper_event(
    fixture: &ImportFixture,
    result: &ImportLegacyWorkflowInstanceResult,
    field: &str,
    value: serde_json::Value,
) {
    let mut data: serde_json::Value =
        sqlx::query_scalar("SELECT event_data FROM workflow_events WHERE event_id=$1")
            .bind(result.event_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    data[field] = value;
    let event_digest = digest::compute_json_digest(&data).unwrap();
    let mut transaction = fixture.pool.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("UPDATE workflow_events SET event_data=$2, event_data_digest=$3 WHERE event_id=$1")
        .bind(result.event_id)
        .bind(data)
        .bind(event_digest)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
}

async fn assert_rebuild_rejects(field: &str, value: serde_json::Value) {
    let fixture = fixture(ImportedNodeKind::Normal).await;
    let result = run(&fixture).await.unwrap();
    tamper_event(&fixture, &result, field, value).await;
    sqlx::query(
        "INSERT INTO domain_role_bindings
         (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1,$2,$3,'WORKFLOW_ADMIN',TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(fixture.domain)
    .bind(fixture.owner)
    .execute(&fixture.pool)
    .await
    .unwrap();
    let error = rebuild_projection(
        &fixture.pool,
        RebuildProjectionCommand {
            principal_id: PrincipalId::from_uuid(fixture.owner),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(result.workflow_instance_id),
            expected_before_snapshot_digest: None,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(error, RecoveryError::InvalidImmutableFacts(_)));
}

#[tokio::test]
async fn strict_rebuild_rejects_each_invalid_import_event_value() {
    assert_rebuild_rejects("legacySystem", serde_json::json!("ADC")).await;
    assert_rebuild_rejects(
        "legacyRecordId",
        serde_json::json!(Uuid::new_v4().to_string().to_uppercase()),
    )
    .await;
    assert_rebuild_rejects("legacySnapshotDigest", serde_json::json!("A".repeat(64))).await;
    assert_rebuild_rejects(
        "importedNodeId",
        serde_json::json!(Uuid::new_v4().to_string()),
    )
    .await;
    assert_rebuild_rejects("importedAt", serde_json::json!("2026-07-15T01:02:03.123Z")).await;
    assert_rebuild_rejects("creatorResolution", serde_json::json!("MIGRATION_SERVICE")).await;
}
