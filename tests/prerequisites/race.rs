use super::*;
#[tokio::test]
async fn source_terminal_and_dependent_create_have_one_serial_order() {
    let p = pool().await;
    let f = setup(&p).await;
    let (v, t) = version(&p, &f, false, false).await;
    let (dv, _) = version(&p, &f, true, false).await;
    for _ in 0..3 {
        let source = create(&p, &f, v, json!({})).await;
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
        let mut c = transition_command(&sf, 1);
        c.submission_payload = Some(json!({"real":"terminal"}));
        let create = create_workflow_instance(
            &p,
            AdmissionGate::disabled(),
            create_cmd(&f, dv, reference(source, v, t)),
        );
        let finish=svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(&p,AdmissionGate::disabled(),c);
        let (dependent, terminal) = tokio::join!(create, finish);
        terminal.unwrap();
        let dependent = dependent.unwrap();
        let read = svc_workflow::store::postgres::workflow_instance_repository::prerequisite::read(
            &p,
            f.owner,
            dependent.workflow_instance_id,
        )
        .await
        .unwrap();
        assert_eq!(read["prerequisiteReadiness"]["status"], "READY");
    }
}
