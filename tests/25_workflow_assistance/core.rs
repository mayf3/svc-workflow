use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::Barrier;
use uuid::Uuid;

use svc_workflow::application::workflow_instance::assistance::{
    get_assistance_case, list_assistance, list_human_required_assistance, request_assistance,
    resolve_assistance, AssistanceListView,
};
use svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition;
use svc_workflow::application::workflow_instance::revise_and_transition::revise_context_and_transition;
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::assistance::{
    AssistanceCaseStatus, AssistanceError, ResolveAssistanceCommand,
};
use svc_workflow::domain::workflow_instance::combined_errors::ReviseContextAndTransitionError;
use svc_workflow::domain::workflow_instance::commands::ReviseContextAndTransitionCommand;
use svc_workflow::domain::workflow_instance::errors::ExecuteWorkflowTransitionError;

use super::helpers::*;

#[sqlx::test]
async fn request_vs_transition_is_serialized_without_orphan_case(pool: PgPool) {
    let fixture = setup(&pool).await;
    let barrier = Arc::new(Barrier::new(3));
    let request_pool = pool.clone();
    let transition_pool = pool.clone();
    let request_barrier = Arc::clone(&barrier);
    let transition_barrier = Arc::clone(&barrier);
    let request = request_command(&fixture, Uuid::new_v4().to_string());
    let transition = transition_command(&fixture, 1);
    let request_task = tokio::spawn(async move {
        request_barrier.wait().await;
        request_assistance(&request_pool, request).await
    });
    let transition_task = tokio::spawn(async move {
        transition_barrier.wait().await;
        execute_workflow_transition(&transition_pool, transition).await
    });
    barrier.wait().await;
    let request_result = request_task.await.unwrap();
    let transition_result = transition_task.await.unwrap();

    match (request_result, transition_result) {
        (
            Ok(case),
            Err(ExecuteWorkflowTransitionError::WorkflowStateVersionConflict { actual, .. }),
        ) => {
            assert_eq!(actual, 2);
            assert_eq!(case.workflow_state_version, 2);
            let row: (i64, Uuid) = sqlx::query_as(
                "SELECT COUNT(ac.assistance_case_id), wi.current_node_visit_id
                 FROM workflow_instances wi
                 LEFT JOIN workflow_assistance_cases ac
                   ON ac.workflow_instance_id=wi.workflow_instance_id
                 WHERE wi.workflow_instance_id=$1 GROUP BY wi.current_node_visit_id",
            )
            .bind(fixture.instance)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(row, (1, fixture.visit));
        }
        (Err(AssistanceError::WorkflowStateVersionConflict { actual, .. }), Ok(transition)) => {
            assert_eq!(actual, 2);
            assert_eq!(transition.workflow_state_version, 2);
            let cases: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM workflow_assistance_cases
                 WHERE workflow_instance_id=$1",
            )
            .bind(fixture.instance)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(cases, 0);
            assert_ne!(transition.current_node_visit_id, fixture.visit);
        }
        other => panic!("request and transition must have exactly one winner: {other:?}"),
    }
}

#[sqlx::test]
async fn owner_resolve_detail_latest_version_then_agent_transition(pool: PgPool) {
    let fixture = setup(&pool).await;
    let case = request_case(&pool, &fixture).await;
    let transition_error = execute_workflow_transition(&pool, transition_command(&fixture, 2))
        .await
        .unwrap_err();
    assert!(matches!(
        transition_error,
        ExecuteWorkflowTransitionError::AssistanceOpen
    ));

    let combined_error = revise_context_and_transition(
        &pool,
        ReviseContextAndTransitionCommand {
            principal_id: PrincipalId::from_uuid(fixture.agent),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(fixture.instance),
            expected_workflow_state_version: 2,
            transition_definition_id: TransitionId::from_uuid(fixture.transition),
            context_payload: serde_json::json!({"revised":true}),
            submission_payload: serde_json::json!({}),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        combined_error,
        ReviseContextAndTransitionError::AssistanceOpen
    ));

    let owner_inbox = list_assistance(
        &pool,
        fixture.owner,
        AssistanceListView::OwnerInbox,
        None,
        None,
        50,
    )
    .await
    .unwrap();
    assert_eq!(owner_inbox.items.len(), 1);
    let resolved = resolve(&pool, &fixture, case.assistance_case_id, 2).await;
    assert_eq!(resolved.workflow_state_version, 3);

    let detail = get_assistance_case(&pool, fixture.agent, case.assistance_case_id)
        .await
        .unwrap();
    assert_eq!(detail.status, AssistanceCaseStatus::Resolved);
    assert_eq!(detail.workflow_state_version, 3);
    assert_eq!(detail.current_node_visit_id, fixture.visit);
    assert_eq!(
        transition(&pool, &fixture, detail.workflow_state_version).await,
        4
    );
}

#[sqlx::test]
async fn human_required_coordinator_query_owner_resolve_then_agent_transition(pool: PgPool) {
    let fixture = setup(&pool).await;
    let case = request_case(&pool, &fixture).await;
    let escalated = escalate(&pool, &fixture, case.assistance_case_id, 2).await;
    assert_eq!(escalated.workflow_state_version, 3);

    let denied = resolve_assistance(
        &pool,
        ResolveAssistanceCommand {
            principal_id: PrincipalId::from_uuid(fixture.coordinator),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            assistance_case_id: AssistanceCaseId::from_uuid(case.assistance_case_id),
            expected_workflow_state_version: 3,
            resolution: payload("Coordinator must remain read-only"),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(denied, AssistanceError::NotDomainOwner));

    let human_inbox = list_human_required_assistance(&pool, fixture.coordinator, None, 50)
        .await
        .unwrap();
    assert_eq!(human_inbox.items.len(), 1);
    let exposed = serde_json::to_value(&human_inbox.items[0]).unwrap();
    assert!(exposed.get("workflowContext").is_none());
    assert!(exposed.get("contextPayload").is_none());
    assert!(exposed.get("submissions").is_none());
    assert!(exposed.get("transitions").is_none());
    assert!(exposed.get("nodeVisitId").is_none());
    assert!(exposed.get("resolvedByPrincipalId").is_none());
    assert!(exposed.get("resolution").is_none());
    assert!(exposed.get("workflowStateVersion").is_none());
    assert!(exposed.get("currentNodeVisitId").is_none());
    assert!(exposed.get("voidedAt").is_none());
    assert_eq!(exposed.as_object().unwrap().len(), 11);

    let resolved = resolve(&pool, &fixture, case.assistance_case_id, 3).await;
    assert_eq!(resolved.workflow_state_version, 4);
    let detail = get_assistance_case(&pool, fixture.agent, case.assistance_case_id)
        .await
        .unwrap();
    assert_eq!(detail.workflow_state_version, 4);
    assert_eq!(detail.resolved_by_principal_id, Some(fixture.owner));
    assert_eq!(
        transition(&pool, &fixture, detail.workflow_state_version).await,
        5
    );
}
