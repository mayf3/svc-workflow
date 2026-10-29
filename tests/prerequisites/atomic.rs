use super::*;
#[tokio::test]
async fn create_bind_http_atomic_replay_and_rollback() {
    let p = pool().await;
    let f = setup(&p).await;
    let case = request_case(&p, &f).await;
    let (v, t) = version(&p, &f, false, true).await;
    let mock = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &mock).await;
    let path = format!(
        "/internal/v1/assistance-cases/{}/governance-child:create-and-bind",
        case.assistance_case_id
    );
    let body = json!({"definitionVersionId":v,"contextPayload":{},"acceptanceTransitionId":t,"expectedWorkflowStateVersion":2});
    let key = Uuid::new_v4().to_string();
    let (s, b) = http(
        router.clone(),
        f.owner,
        &mock,
        "POST",
        &path,
        Some(&key),
        body.clone(),
    )
    .await;
    assert_eq!(s, 201, "{b}");
    let (s2, b2) = http(
        router.clone(),
        f.owner,
        &mock,
        "POST",
        &path,
        Some(&key),
        body.clone(),
    )
    .await;
    assert_eq!(s2, 201);
    assert_eq!(b, b2);
    let mut changed = body.clone();
    changed["contextPayload"] = json!({"changed":true});
    assert_eq!(
        http(
            router.clone(),
            f.owner,
            &mock,
            "POST",
            &path,
            Some(&key),
            changed
        )
        .await
        .0,
        409
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_instances WHERE definition_version_id=$1",
    )
    .bind(v)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(count, 1);
    let f2 = setup(&p).await;
    let case2 = request_case(&p, &f2).await;
    let (v2, _) = version(&p, &f2, false, false).await;
    let invalid = json!({"definitionVersionId":v2,"contextPayload":{},"acceptanceTransitionId":Uuid::new_v4(),"expectedWorkflowStateVersion":2});
    let path2 = format!(
        "/internal/v1/assistance-cases/{}/governance-child:create-and-bind",
        case2.assistance_case_id
    );
    assert!(
        http(
            router,
            f2.owner,
            &mock,
            "POST",
            &path2,
            Some(&Uuid::new_v4().to_string()),
            invalid
        )
        .await
        .0 >= 400
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM workflow_instances WHERE definition_version_id=$1"
        )
        .bind(v2)
        .fetch_one(&p)
        .await
        .unwrap(),
        0
    );
}
#[tokio::test]
async fn feasibility_conflict_rolls_back_all_new_facts() {
    let p = pool().await;
    let f = setup(&p).await;
    let case = request_case(&p, &f).await;
    let (v, t) = version(&p, &f, false, true).await;
    let cmd=svc_workflow::domain::workflow_instance::governance_child::CreateAndBindGovernanceChildCommand {principal_id:PrincipalId::from_uuid(f.owner),idempotency_key:Uuid::new_v4().to_string(),command_schema_version:"v1".into(),assistance_case_id:AssistanceCaseId::from_uuid(case.assistance_case_id),definition_version_id:DefinitionVersionId::from_uuid(v),context_payload:json!({}),acceptance_transition_id:TransitionId::from_uuid(t),expected_workflow_state_version:2};
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_outbox")
        .fetch_one(&p)
        .await
        .unwrap();
    let result =
        svc_workflow::application::workflow_instance::assistance::create_and_bind_governance_child(
            &p,
            AdmissionGate::disabled(),
            cmd.clone(),
        )
        .await;
    assert!(
        matches!(result,Err(svc_workflow::domain::workflow_instance::assistance::AssistanceError::StorageError(ref e)) if e.contains("idx_wf_event_unique_command")),
        "{result:?}"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM workflow_instances WHERE definition_version_id=$1"
        )
        .bind(v)
        .fetch_one(&p)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM workflow_governance_child_bindings WHERE assistance_case_id=$1"
        )
        .bind(case.assistance_case_id)
        .fetch_one(&p)
        .await
        .unwrap(),
        0
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2").bind(f.owner).bind(cmd.idempotency_key).fetch_one(&p).await.unwrap(),0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM workflow_outbox")
            .fetch_one(&p)
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i32>(
            "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1"
        )
        .bind(f.instance)
        .fetch_one(&p)
        .await
        .unwrap(),
        2
    );
}
