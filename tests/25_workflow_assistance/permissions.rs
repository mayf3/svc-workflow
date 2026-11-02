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

#[sqlx::test]
async fn canonical_successor_assistance_keeps_request_and_owner_roles_separate(pool: PgPool) {
    use svc_workflow::application::workflow_instance::{
        authorized_actions::{ActionCredential, AuthorizedAction, AuthorizedInstanceDetail},
        query_service::WorkflowQueryService,
        query_types::GetWorkflowInstanceDetail,
    };
    use svc_workflow::domain::workflow_instance::assistance::RequestAssistanceCommand;

    async fn assert_successor_refused(
        pool: &PgPool,
        f: &Fixture,
        command: RequestAssistanceCommand,
    ) {
        let detail = WorkflowQueryService::new(pool.clone())
            .get_workflow_instance_detail_with_authorized_actions(
                GetWorkflowInstanceDetail {
                    actor_principal_id: *command.principal_id.as_uuid(),
                    workflow_instance_id: f.instance,
                },
                ActionCredential {
                    execute_scope: true,
                    write_gate_open: true,
                    max_returns_per_edge: 3,
                },
            )
            .await
            .unwrap();
        let AuthorizedInstanceDetail::Full(detail) = detail else {
            panic!("canonical successor retains full current-assignee visibility");
        };
        assert!(!detail.authorized_actions.iter().any(|action| matches!(
            action,
            AuthorizedAction::RequestAssistance { .. }
        )));
        let facts = "SELECT workflow_state_version, (SELECT count(*) FROM workflow_assistance_cases WHERE workflow_instance_id=$1), (SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1) FROM workflow_instances WHERE workflow_instance_id=$1";
        let before: (i32, i64, i64) = sqlx::query_as(facts)
            .bind(f.instance).fetch_one(pool).await.unwrap();
        let error = request_assistance(pool, command).await.unwrap_err();
        assert_eq!(error, AssistanceError::PrincipalNotAssignee);
        assert_eq!(error.status_code(), 403);
        let after: (i32, i64, i64) = sqlx::query_as(facts)
            .bind(f.instance).fetch_one(pool).await.unwrap();
        assert_eq!(before, after, "refusal must not advance state or create case/event");
    }

    let f = setup(&pool).await;
    let q = seed_agent(&pool, "successor").await;
    sqlx::query("INSERT INTO workflow_identity_successor_lines (source_principal_id,successor_principal_id,legacy_agent_id,canonical_agent_id,classification,evidence,repair_reason) VALUES($1,$2,$3,$4,'STALE_PRINCIPAL_WITH_UNIQUE_REPAIR',jsonb_build_object('source','owned T2 fixture'),'owned one-edge fixture')")
        .bind(f.agent).bind(q).bind(format!("legacy-{}",f.agent)).bind(format!("canonical-{q}")).execute(&pool).await.unwrap();
    for actor in [f.outsider, f.owner, f.coordinator, f.admin] {
        let mut command = request_command(&f, Uuid::new_v4().to_string());
        command.principal_id = PrincipalId::from_uuid(actor);
        assert_eq!(
            request_assistance(&pool, command).await.unwrap_err(),
            AssistanceError::PrincipalNotAssignee
        );
    }
    let mut command = request_command(&f, Uuid::new_v4().to_string());
    command.principal_id = PrincipalId::from_uuid(q);
    assert_successor_refused(&pool, &f, command.clone()).await;
    assert_successor_refused(&pool, &f, command).await;
    // CTR-AA-005: enabled stored P remains the legitimate opener despite P -> Q.
    let command = request_command(&f, Uuid::new_v4().to_string());
    let opened = request_assistance(&pool, command.clone()).await.unwrap();
    assert_eq!(opened.workflow_state_version, 2);
    let replay = request_assistance(&pool, command).await.unwrap();
    assert_eq!(replay.assistance_case_id, opened.assistance_case_id);
    assert!(replay.replayed);
    let (requester,stored): (Uuid,Uuid) = sqlx::query_as("SELECT a.requested_by_principal_id,v.assignee_principal_id FROM workflow_assistance_cases a JOIN workflow_node_visits v USING(node_visit_id) WHERE assistance_case_id=$1").bind(opened.assistance_case_id).fetch_one(&pool).await.unwrap();
    assert_eq!((requester, stored), (f.agent, f.agent));
    let copied_grants: i64 =
        sqlx::query_scalar("SELECT count(*) FROM domain_role_bindings WHERE principal_id=$1")
            .bind(q)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(copied_grants, 0);
    let mut duplicate = request_command(&f, Uuid::new_v4().to_string());
    duplicate.principal_id = PrincipalId::from_uuid(q);
    duplicate.expected_workflow_state_version = 2;
    assert_successor_refused(&pool, &f, duplicate.clone()).await;
    let mut raw_duplicate = duplicate.clone();
    raw_duplicate.principal_id = PrincipalId::from_uuid(f.agent);
    assert_eq!(
        request_assistance(&pool, raw_duplicate.clone())
            .await
            .unwrap_err(),
        AssistanceError::AssistanceAlreadyOpen
    );
    raw_duplicate.idempotency_key = Uuid::new_v4().to_string();
    raw_duplicate.expected_workflow_state_version = 1;
    assert_eq!(
        request_assistance(&pool, raw_duplicate).await.unwrap_err(),
        AssistanceError::WorkflowStateVersionConflict { expected: 1, actual: 2 }
    );
    sqlx::query("UPDATE principals SET enabled=false WHERE principal_id=$1")
        .bind(q)
        .execute(&pool)
        .await
        .unwrap();
    duplicate.idempotency_key = Uuid::new_v4().to_string();
    assert_eq!(
        request_assistance(&pool, duplicate).await.unwrap_err(),
        AssistanceError::PrincipalDisabled
    );
    sqlx::query("UPDATE principals SET enabled=true WHERE principal_id=$1")
        .bind(q)
        .execute(&pool)
        .await
        .unwrap();
    let attempt = EscalateAssistanceCommand {
        principal_id: PrincipalId::from_uuid(q),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".into(),
        assistance_case_id: AssistanceCaseId::from_uuid(opened.assistance_case_id),
        expected_workflow_state_version: 2,
        escalation: payload("not an Owner"),
    };
    assert_eq!(
        escalate_assistance_to_human(&pool, attempt)
            .await
            .unwrap_err(),
        AssistanceError::NotDomainOwner
    );
    let closed = resolve(&pool, &f, opened.assistance_case_id, 2).await;
    assert_eq!(closed.workflow_state_version, 3);
    let mut next = request_command(&f, Uuid::new_v4().to_string());
    next.principal_id = PrincipalId::from_uuid(q);
    next.expected_workflow_state_version = 3;
    assert_successor_refused(&pool, &f, next.clone()).await;
    next.principal_id = PrincipalId::from_uuid(f.agent);
    let reopened = request_assistance(&pool, next).await.unwrap();
    assert_ne!(reopened.assistance_case_id, opened.assistance_case_id);
    assert_eq!(reopened.workflow_state_version, 4);
    assert_eq!(
        case_status(&pool, opened.assistance_case_id).await.0,
        "RESOLVED"
    );
}
