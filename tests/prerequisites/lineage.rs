use super::*;
#[tokio::test]
async fn closed_and_corrupt_lineage_never_dispatch_and_current_task_is_bounded() {
    let p = pool().await;
    let f = setup(&p).await;
    let (v, t) = version(&p, &f, false, false).await;
    let source = create(&p, &f, v, json!({})).await;
    let (dv, _) = version(&p, &f, true, false).await;
    let dep = create(&p, &f, dv, reference(source, v, t)).await;
    let svc =
        svc_workflow::application::workflow_instance::query_service::WorkflowQueryService::new(
            p.clone(),
        );
    let input=svc.current_task_input(f.agent,dep,svc_workflow::application::workflow_instance::current_task_input::CurrentTaskInputRequest::Input{expected:None,submission_ids:vec![]}).await.unwrap();
    assert_eq!(input["prerequisiteReadiness"]["status"], "WAITING");
    assert!(input["prerequisiteReadiness"].get("submissionId").is_none());
    sqlx::query("UPDATE workflow_instances SET cancelled=true WHERE workflow_instance_id=$1")
        .bind(source)
        .execute(&p)
        .await
        .unwrap();
    let closed = svc_workflow::store::postgres::workflow_instance_repository::prerequisite::read(
        &p, f.agent, dep,
    )
    .await
    .unwrap();
    assert_eq!(
        closed["prerequisiteReadiness"]["reason"],
        "SOURCE_CANCELLED"
    );
    let other_source = create(&p, &f, v, json!({})).await;
    let dep2 = create(&p, &f, dv, reference(other_source, v, t)).await;
    let visit: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(other_source)
    .fetch_one(&p)
    .await
    .unwrap();
    let sf = Fixture {
        instance: other_source,
        visit,
        transition: t,
        ..f
    };
    let mut c = transition_command(&sf, 1);
    c.submission_payload = Some(json!({"value":"canonical"}));
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &p,
        crate::common::admitting_gate(),
        c,
    )
    .await
    .unwrap();
    // Deliberate corruption in this fresh test's own fixture; trigger bypass is
    // transaction-local and never affects another database or old run.
    let mut tx = p.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE workflow_submissions SET payload_digest=$2 WHERE workflow_instance_id=$1")
        .bind(other_source)
        .bind("0".repeat(64))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let unknown = svc_workflow::store::postgres::workflow_instance_repository::prerequisite::read(
        &p, f.agent, dep2,
    )
    .await
    .unwrap();
    assert_eq!(unknown["prerequisiteReadiness"]["status"], "UNKNOWN");
    assert_eq!(
        unknown["prerequisiteReadiness"]["reason"],
        "LINEAGE_INCONSISTENT"
    );
    assert!(unknown["prerequisiteReadiness"]
        .get("submissionId")
        .is_none());
}
