use super::*;
#[tokio::test]
async fn readiness_http_waiting_then_exact_submission_and_archive() {
    let p = pool().await;
    let f = setup(&p).await;
    let (v, t) = version(&p, &f, false, false).await;
    let source = create(&p, &f, v, json!({})).await;
    let (dv, _) = version(&p, &f, true, false).await;
    let dependent = create(&p, &f, dv, reference(source, v, t)).await;
    let mock = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &mock).await;
    sqlx::query("INSERT INTO domain_role_bindings(binding_id,domain_id,principal_id,role_key,enabled) VALUES($1,$2,$3,'DOMAIN_MEMBER',true)").bind(Uuid::new_v4()).bind(f.domain).bind(f.outsider).execute(&p).await.unwrap();
    let path = format!("/internal/v1/workflow-instances/{dependent}/readiness");
    let (s, b) = http(
        router.clone(),
        f.agent,
        &mock,
        "GET",
        &path,
        None,
        json!({}),
    )
    .await;
    assert_eq!(s, 200, "{b}");
    assert_eq!(b["prerequisiteReadiness"]["status"], "WAITING");
    assert_eq!(
        http(
            router.clone(),
            f.outsider,
            &mock,
            "GET",
            &path,
            None,
            json!({})
        )
        .await
        .0,
        404
    );
    let visit: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let sf = Fixture {
        instance: source,
        visit,
        transition: t,
        ..f
    };
    let mut command = transition_command(&sf, 1);
    command.submission_payload = Some(json!({"artifact":"actual"}));
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &p,
        AdmissionGate::disabled(),
        command,
    )
    .await
    .unwrap();
    let (s, b) = http(
        router.clone(),
        f.owner,
        &mock,
        "GET",
        &path,
        None,
        json!({}),
    )
    .await;
    assert_eq!(s, 200);
    assert_eq!(b["prerequisiteReadiness"]["status"], "READY", "{b}");
    let proof:(Uuid,Uuid,String)=sqlx::query_as("SELECT e.event_id,s.submission_id,s.payload_digest FROM workflow_events e JOIN workflow_submissions s USING(submission_id) WHERE e.workflow_instance_id=$1 AND s.workflow_instance_id=$1").bind(source).fetch_one(&p).await.unwrap();
    assert_eq!(b["prerequisiteReadiness"]["eventId"], json!(proof.0));
    assert_eq!(b["prerequisiteReadiness"]["submissionId"], json!(proof.1));
    assert_eq!(b["prerequisiteReadiness"]["submissionSha256"], proof.2);
    assert!(b["prerequisiteReadiness"].get("reason").is_none());
    assert_eq!(b["prerequisiteReadiness"].as_object().unwrap().len(), 7);
    let before = b;
    sqlx::query("UPDATE workflow_instances SET archived_at=now() WHERE workflow_instance_id=$1")
        .bind(source)
        .execute(&p)
        .await
        .unwrap();
    let rebuilt = http_support::app(&p, &mock).await;
    assert_eq!(
        http(rebuilt, f.agent, &mock, "GET", &path, None, json!({}))
            .await
            .1,
        before
    );
    assert!(create_workflow_instance(
        &p,
        AdmissionGate::disabled(),
        create_cmd(&f, dv, reference(source, v, t))
    )
    .await
    .is_ok());
}
