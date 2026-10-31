use super::*;
#[tokio::test]
async fn ordinary_admission_rejects_invalid_and_unauthorized_without_facts() {
    let p = pool().await;
    let f = setup(&p).await;
    let (v, t) = version(&p, &f, false, false).await;
    let source = create(&p, &f, v, json!({})).await;
    let (dv, _) = version(&p, &f, true, false).await;
    let other = setup(&p).await;
    let (ov, ot) = version(&p, &other, false, false).await;
    let foreign = create(&p, &other, ov, json!({})).await;
    let mut failures = vec![
        reference(Uuid::new_v4(), v, t),
        reference(source, Uuid::new_v4(), t),
        reference(source, v, Uuid::new_v4()),
        reference(foreign, ov, ot),
        reference(f.instance, v, t),
        json!({}),
        json!({"workflowPrerequisite":{"kind":"TERMINAL_SUBMISSION"}}),
    ];
    let mut extra = reference(source, v, t);
    extra["workflowPrerequisite"]["futureSubmissionId"] = json!(Uuid::new_v4());
    failures.push(extra);
    for c in failures {
        let cmd = create_cmd(&f, dv, c);
        assert!(create_workflow_instance(&p, crate::common::admitting_gate(), cmd)
            .await
            .is_err());
    }
    let mut member = create_cmd(&f, dv, reference(source, v, t));
    member.principal_id = PrincipalId::from_uuid(f.agent);
    assert!(matches!(create_workflow_instance(&p,crate::common::admitting_gate(),member).await,Err(svc_workflow::domain::workflow_instance::errors::CreateWorkflowInstanceError::NotDomainOwner)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM workflow_instances WHERE definition_version_id=$1"
        )
        .bind(dv)
        .fetch_one(&p)
        .await
        .unwrap(),
        0
    );
    sqlx::query("UPDATE workflow_instances SET cancelled=true WHERE workflow_instance_id=$1")
        .bind(source)
        .execute(&p)
        .await
        .unwrap();
    assert!(create_workflow_instance(
        &p,
        crate::common::admitting_gate(),
        create_cmd(&f, dv, reference(source, v, t))
    )
    .await
    .is_err());
    sqlx::query("UPDATE domains SET enabled=false WHERE domain_id=$1")
        .bind(f.domain)
        .execute(&p)
        .await
        .unwrap();
    assert!(create_workflow_instance(
        &p,
        crate::common::admitting_gate(),
        create_cmd(&f, dv, reference(source, v, t))
    )
    .await
    .is_err());
}
