use super::*;
#[tokio::test]
async fn lost_reply_case_readback_recovers_exact_child_prerequisite_and_receipt() {
    let p = pool().await;
    let f = setup(&p).await;
    let case = request_case(&p, &f).await;
    let (sv, st) = version(&p, &f, false, false).await;
    let supplier = create(&p, &f, sv, json!({})).await;
    let (cv, ct) = version(&p, &f, true, true).await;
    let command=svc_workflow::domain::workflow_instance::governance_child::CreateAndBindGovernanceChildCommand{principal_id:PrincipalId::from_uuid(f.owner),idempotency_key:Uuid::new_v4().to_string(),command_schema_version:"v1".into(),assistance_case_id:AssistanceCaseId::from_uuid(case.assistance_case_id),definition_version_id:DefinitionVersionId::from_uuid(cv),context_payload:reference(supplier,sv,st),acceptance_transition_id:TransitionId::from_uuid(ct),expected_workflow_state_version:2};
    // Drop the successful response on purpose, then recover only by linked detail.
    drop(
        svc_workflow::application::workflow_instance::assistance::create_and_bind_governance_child(
            &p,
            AdmissionGate::disabled(),
            command.clone(),
        )
        .await
        .unwrap(),
    );
    let detail = svc_workflow::application::workflow_instance::assistance::get_assistance_case(
        &p,
        f.owner,
        case.assistance_case_id,
    )
    .await
    .unwrap();
    let binding = detail.governance_binding.unwrap();
    let readiness = detail.prerequisite_readiness.unwrap();
    assert_eq!(readiness["status"], "WAITING");
    assert_eq!(readiness["sourceInstanceId"], json!(supplier));
    assert_eq!(readiness["sourceDefinitionVersionId"], json!(sv));
    assert_eq!(readiness["transitionId"], json!(st));
    assert_eq!(binding["child"]["definitionVersionId"], json!(cv));
    let replay =
        svc_workflow::application::workflow_instance::assistance::create_and_bind_governance_child(
            &p,
            AdmissionGate::disabled(),
            command.clone(),
        )
        .await
        .unwrap();
    let mut core = replay.clone();
    core.as_object_mut()
        .unwrap()
        .remove("prerequisiteReadiness");
    assert_eq!(core, binding);
    assert_eq!(replay["prerequisiteReadiness"], readiness);
    let alice = svc_workflow::application::workflow_instance::assistance::get_assistance_case(
        &p,
        f.agent,
        case.assistance_case_id,
    )
    .await
    .unwrap();
    assert_eq!(alice.prerequisite_readiness, Some(readiness));
    assert!(
        svc_workflow::application::workflow_instance::assistance::get_assistance_case(
            &p,
            f.outsider,
            case.assistance_case_id
        )
        .await
        .is_err()
    );
    let visit: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(supplier)
    .fetch_one(&p)
    .await
    .unwrap();
    let sf = Fixture {
        instance: supplier,
        visit,
        transition: st,
        ..f
    };
    let mut finish = transition_command(&sf, 1);
    finish.submission_payload = Some(json!({"actual":"supplier terminal artifact"}));
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &p,
        AdmissionGate::disabled(),
        finish,
    )
    .await
    .unwrap();
    let fresh = svc_workflow::application::workflow_instance::assistance::get_assistance_case(
        &p,
        f.owner,
        case.assistance_case_id,
    )
    .await
    .unwrap();
    assert_eq!(fresh.prerequisite_readiness.unwrap()["status"], "READY");
    assert_eq!(
        svc_workflow::application::workflow_instance::assistance::create_and_bind_governance_child(
            &p,
            AdmissionGate::disabled(),
            command
        )
        .await
        .unwrap(),
        replay,
        "receipt replay is immutable even as readiness derives READY"
    );
}
