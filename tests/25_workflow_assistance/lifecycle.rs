use sqlx::PgPool;
use uuid::Uuid;

use svc_workflow::application::workflow_instance::admin_recovery::{
    admin_emergency_override, rebuild_projection,
};
use svc_workflow::application::workflow_instance::archive::archive_workflow_instance;
use svc_workflow::application::workflow_instance::cancel::cancel_workflow_instance;
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::{
    ArchiveWorkflowInstanceCommand, CancelWorkflowInstanceCommand,
};
use svc_workflow::domain::workflow_instance::recovery::{
    AdminEmergencyOperation, AdminEmergencyOverrideCommand, RebuildProjectionCommand,
};

use super::helpers::*;

#[sqlx::test]
async fn cancel_voids_open_assistance(pool: PgPool) {
    let fixture = setup(&pool).await;
    let case = request_case(&pool, &fixture).await;
    cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(fixture.owner),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(fixture.instance),
            expected_workflow_state_version: 2,
            reason: "cancel assistance test".to_string(),
        },
        &request_hash("cancel assistance test"),
    )
    .await
    .unwrap();
    assert_eq!(
        case_status(&pool, case.assistance_case_id).await,
        ("VOIDED".to_string(), Some("INSTANCE_CANCELLED".to_string()))
    );
}

#[sqlx::test]
async fn admin_override_voids_open_assistance(pool: PgPool) {
    let fixture = setup(&pool).await;
    let case = request_case(&pool, &fixture).await;
    admin_emergency_override(
        &pool,
        AdminEmergencyOverrideCommand {
            principal_id: PrincipalId::from_uuid(fixture.admin),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(fixture.instance),
            expected_workflow_state_version: 2,
            operation: AdminEmergencyOperation::TerminateInstance,
            target_node_id: NodeId::from_uuid(fixture.terminal_node),
            reason: "operator-approved assistance recovery".to_string(),
            related_references: Vec::new(),
            expected_before_snapshot_digest: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        case_status(&pool, case.assistance_case_id).await,
        (
            "VOIDED".to_string(),
            Some("ADMIN_EMERGENCY_OVERRIDE".to_string())
        )
    );
}

#[sqlx::test]
async fn archive_defensively_voids_open_case_on_terminal_visit(pool: PgPool) {
    let fixture = setup(&pool).await;
    assert_eq!(transition(&pool, &fixture, 1).await, 2);
    let terminal_visit: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(fixture.instance)
    .fetch_one(&pool)
    .await
    .unwrap();
    let case_id = insert_open_case(&pool, &fixture, terminal_visit).await;
    archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(fixture.owner),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(fixture.instance),
            expected_workflow_state_version: 2,
            reason: "archive terminal assistance test".to_string(),
        },
        &request_hash("archive terminal assistance test"),
    )
    .await
    .unwrap();
    assert_eq!(
        case_status(&pool, case_id).await,
        ("VOIDED".to_string(), Some("INSTANCE_ARCHIVED".to_string()))
    );
}

#[sqlx::test]
async fn projection_rebuild_replays_assistance_events_and_voids_stale_case(pool: PgPool) {
    let fixture = setup(&pool).await;
    let requested = request_case(&pool, &fixture).await;
    let resolved = resolve(&pool, &fixture, requested.assistance_case_id, 2).await;
    assert_eq!(resolved.workflow_state_version, 3);
    assert_eq!(transition(&pool, &fixture, 3).await, 4);
    let terminal_visit: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(fixture.instance)
    .fetch_one(&pool)
    .await
    .unwrap();
    let stale_case = insert_open_case(&pool, &fixture, fixture.visit).await;
    sqlx::query(
        "UPDATE workflow_instances SET current_node_visit_id=$2
         WHERE workflow_instance_id=$1",
    )
    .bind(fixture.instance)
    .bind(fixture.visit)
    .execute(&pool)
    .await
    .unwrap();

    let rebuilt = rebuild_projection(
        &pool,
        RebuildProjectionCommand {
            principal_id: PrincipalId::from_uuid(fixture.admin),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(fixture.instance),
            expected_before_snapshot_digest: None,
        },
    )
    .await
    .unwrap();
    assert!(rebuilt.changed);
    assert_eq!(
        rebuilt.after_projection.current_node_visit_id,
        Some(terminal_visit)
    );
    assert_eq!(rebuilt.after_projection.workflow_state_version, 4);
    assert_eq!(
        case_status(&pool, stale_case).await,
        (
            "VOIDED".to_string(),
            Some("ADMIN_PROJECTION_REBUILD".to_string())
        )
    );
}
