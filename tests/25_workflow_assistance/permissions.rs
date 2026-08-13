use sqlx::PgPool;
use uuid::Uuid;

use svc_workflow::application::provisioning::replace_owner;
use svc_workflow::application::workflow_instance::assistance::{
    escalate_assistance_to_human, get_assistance_case, list_human_required_assistance,
    request_assistance,
};
use svc_workflow::domain::ids::*;
use svc_workflow::domain::provisioning::ReplaceOwnerCommand;
use svc_workflow::domain::workflow_instance::assistance::{
    AssistanceError, EscalateAssistanceCommand,
};

use super::helpers::*;

#[sqlx::test]
async fn migration_enforces_single_open_case_and_forward_only_history(pool: PgPool) {
    let fixture = setup(&pool).await;
    let first = insert_open_case(&pool, &fixture, fixture.visit).await;
    let mut tx = pool.begin().await.unwrap();
    let command_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_command_receipts
         (command_id, principal_id, idempotency_key, command_type, request_hash,
          receipt_status, response_status, response_body, response_digest, completed_at)
         VALUES ($1,$2,$3,'REQUEST_WORKFLOW_ASSISTANCE',$4,
                 'COMPLETED',201,'{}'::jsonb,$5,now())",
    )
    .bind(command_id)
    .bind(fixture.agent)
    .bind(Uuid::new_v4().to_string())
    .bind(request_hash("duplicate request"))
    .bind(request_hash("duplicate response"))
    .execute(&mut *tx)
    .await
    .unwrap();
    let duplicate = sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest,
          request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(fixture.instance)
    .bind(fixture.visit)
    .bind(fixture.agent)
    .bind(serde_json::json!({"message":"duplicate open case"}))
    .bind(request_hash("duplicate payload"))
    .bind(command_id)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    assert_eq!(
        duplicate
            .as_database_error()
            .and_then(|value| value.constraint()),
        Some("uq_assistance_one_open_per_visit")
    );
    tx.rollback().await.unwrap();

    assert!(
        sqlx::query("DELETE FROM workflow_assistance_cases WHERE assistance_case_id=$1")
            .bind(first)
            .execute(&pool)
            .await
            .is_err(),
        "assistance history must be undeletable"
    );
    assert!(
        sqlx::query(
            "UPDATE workflow_assistance_cases SET updated_at=now()
             WHERE assistance_case_id=$1",
        )
        .bind(first)
        .execute(&pool)
        .await
        .is_err(),
        "status must only move through a frozen edge"
    );
}

#[sqlx::test]
async fn permission_negatives_missing_owner_and_idempotent_replay(pool: PgPool) {
    let fixture = setup(&pool).await;
    let mut non_assignee = request_command(&fixture, Uuid::new_v4().to_string());
    non_assignee.principal_id = PrincipalId::from_uuid(fixture.outsider);
    assert!(matches!(
        request_assistance(&pool, non_assignee).await.unwrap_err(),
        AssistanceError::PrincipalNotAssignee
    ));

    sqlx::query(
        "UPDATE domain_role_bindings SET enabled=FALSE, disabled_at=now()
         WHERE domain_id=$1 AND role_key='DOMAIN_OWNER' AND enabled=TRUE",
    )
    .bind(fixture.domain)
    .execute(&pool)
    .await
    .unwrap();
    let ownerless_command = request_command(&fixture, Uuid::new_v4().to_string());
    assert!(matches!(
        request_assistance(&pool, ownerless_command.clone())
            .await
            .unwrap_err(),
        AssistanceError::DomainOwnerMissing
    ));
    sqlx::query(
        "UPDATE domain_role_bindings SET enabled=TRUE, disabled_at=NULL
         WHERE domain_id=$1 AND principal_id=$2 AND role_key='DOMAIN_OWNER'",
    )
    .bind(fixture.domain)
    .bind(fixture.owner)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        request_assistance(&pool, ownerless_command)
            .await
            .unwrap_err(),
        AssistanceError::DomainOwnerMissing
    ));

    let mut invalid = request_command(&fixture, Uuid::new_v4().to_string());
    invalid.request.message = " untrimmed ".to_string();
    assert!(matches!(
        request_assistance(&pool, invalid.clone())
            .await
            .unwrap_err(),
        AssistanceError::InvalidPayload(_)
    ));
    assert!(matches!(
        request_assistance(&pool, invalid).await.unwrap_err(),
        AssistanceError::InvalidPayload(_)
    ));

    let command = request_command(&fixture, Uuid::new_v4().to_string());
    let first = request_assistance(&pool, command.clone()).await.unwrap();
    let replay = request_assistance(&pool, command.clone()).await.unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.assistance_case_id, first.assistance_case_id);
    assert_eq!(replay.workflow_state_version, first.workflow_state_version);
    let mut conflict = command;
    conflict.request.message = "different request under same key".to_string();
    assert!(matches!(
        request_assistance(&pool, conflict).await.unwrap_err(),
        AssistanceError::IdempotencyConflict
    ));

    let denied = escalate_assistance_to_human(
        &pool,
        EscalateAssistanceCommand {
            principal_id: PrincipalId::from_uuid(fixture.outsider),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            assistance_case_id: AssistanceCaseId::from_uuid(first.assistance_case_id),
            expected_workflow_state_version: 2,
            escalation: payload("not authorized"),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(denied, AssistanceError::NotDomainOwner));
    assert!(matches!(
        get_assistance_case(&pool, fixture.outsider, first.assistance_case_id)
            .await
            .unwrap_err(),
        AssistanceError::AssistanceCaseNotFoundOrNotVisible
    ));
    assert!(matches!(
        list_human_required_assistance(&pool, fixture.outsider, None, 50,)
            .await
            .unwrap_err(),
        AssistanceError::GlobalCoordinatorRequired
    ));
}

#[sqlx::test]
async fn owner_replacement_race_serializes_current_owner_authority(pool: PgPool) {
    for _ in 0..12 {
        let fixture = setup(&pool).await;
        let case = request_case(&pool, &fixture).await;
        let resolve_pool = pool.clone();
        let replace_pool = pool.clone();
        let old_owner = fixture.owner;
        let new_owner = fixture.replacement_owner;
        let case_id = case.assistance_case_id;
        let instance = fixture.instance;
        let resolve_task = tokio::spawn(async move {
            svc_workflow::application::workflow_instance::assistance::resolve_assistance(
                &resolve_pool,
                svc_workflow::domain::workflow_instance::assistance::ResolveAssistanceCommand {
                    principal_id: PrincipalId::from_uuid(old_owner),
                    idempotency_key: Uuid::new_v4().to_string(),
                    command_schema_version: "v1".to_string(),
                    assistance_case_id: AssistanceCaseId::from_uuid(case_id),
                    expected_workflow_state_version: 2,
                    resolution: payload("old owner concurrent resolution"),
                },
            )
            .await
        });
        let replace_task = tokio::spawn(async move {
            replace_owner(
                &replace_pool,
                &ReplaceOwnerCommand {
                    domain_id: DomainId::from_uuid(fixture.domain),
                    new_owner_id: PrincipalId::from_uuid(new_owner),
                },
                &Uuid::new_v4().to_string(),
                "owner-replacement-race",
                &PrincipalId::from_uuid(fixture.agent),
            )
            .await
        });
        let old_resolution = resolve_task.await.unwrap();
        replace_task.await.unwrap().unwrap();

        let enabled_owner: Uuid = sqlx::query_scalar(
            "SELECT principal_id FROM domain_role_bindings
             WHERE domain_id=$1 AND role_key='DOMAIN_OWNER' AND enabled=TRUE",
        )
        .bind(fixture.domain)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(enabled_owner, fixture.replacement_owner);
        match old_resolution {
            Ok(resolved) => assert_eq!(resolved.workflow_state_version, 3),
            Err(AssistanceError::NotDomainOwner) => {
                let current_version: i32 = sqlx::query_scalar(
                    "SELECT workflow_state_version FROM workflow_instances
                     WHERE workflow_instance_id=$1",
                )
                .bind(instance)
                .fetch_one(&pool)
                .await
                .unwrap();
                svc_workflow::application::workflow_instance::assistance::resolve_assistance(
                    &pool,
                    svc_workflow::domain::workflow_instance::assistance::ResolveAssistanceCommand {
                        principal_id: PrincipalId::from_uuid(fixture.replacement_owner),
                        idempotency_key: Uuid::new_v4().to_string(),
                        command_schema_version: "v1".to_string(),
                        assistance_case_id: AssistanceCaseId::from_uuid(case_id),
                        expected_workflow_state_version: current_version,
                        resolution: payload("new owner resolution"),
                    },
                )
                .await
                .unwrap();
            }
            other => panic!("unexpected old-owner race result: {other:?}"),
        }
        let status: String = sqlx::query_scalar(
            "SELECT status FROM workflow_assistance_cases WHERE assistance_case_id=$1",
        )
        .bind(case_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status, "RESOLVED");
    }
}
