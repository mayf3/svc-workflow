use super::*;

#[sqlx::test]
async fn exact_binding_terminal_and_explicit_resolve(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let cmd = bind_cmd(&f, case.assistance_case_id, c.instance, 2);
    let b = bind_governance_child(&pool, cmd.clone()).await.unwrap();
    assert_eq!(b["source"]["nodeVisitId"], json!(f.visit));
    assert_eq!(b["child"]["workflowInstanceId"], json!(c.instance));
    let expected = jcs_canonicalize::sha256_jcs_hex(
        &json!({"source":b["source"],"child":b["child"],"submissionSchema":schema()}),
    )
    .unwrap();
    assert_eq!(b["acceptanceContractDigest"], expected);
    assert_eq!(bind_governance_child(&pool, cmd).await.unwrap(), b);
    let detail = serde_json::to_value(
        get_assistance_case(&pool, f.owner, case.assistance_case_id)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(detail["governanceBinding"]["bindingId"], b["bindingId"]);
    assert!(
        resolve_assistance(&pool, resolve_cmd(&f, case.assistance_case_id, json!({})))
            .await
            .is_err()
    );
    finish(&pool, &c, &b, "ACCEPTED").await;
    let rows = review_rows(&pool).await;
    assert_eq!(rows.len(), 1);
    assert_ne!(rows[0].get("reviewWakeId"), None);
    assert_eq!(rows[0]["bindingId"], b["bindingId"]);
    let detail = serde_json::to_value(
        get_assistance_case(&pool, f.owner, case.assistance_case_id)
            .await
            .unwrap(),
    )
    .unwrap();
    let p = &detail["governanceTerminalProof"];
    let review = json!({"bindingId":b["bindingId"],"terminalEventId":p["terminalEventId"],"submissionId":p["submissionId"],"submissionDigest":p["submissionDigest"],"acceptanceContractDigest":b["acceptanceContractDigest"]});
    resolve_assistance(&pool, resolve_cmd(&f, case.assistance_case_id, review))
        .await
        .unwrap();
    assert_eq!(
        case_status(&pool, case.assistance_case_id).await.0,
        "RESOLVED"
    );
    let event:Value=sqlx::query_scalar("SELECT event_data FROM workflow_events WHERE workflow_instance_id=$1 AND event_type='ASSISTANCE_RESOLVED'").bind(f.instance).fetch_one(&pool).await.unwrap();
    assert_eq!(event["governanceChildReview"]["bindingId"], b["bindingId"]);
}
#[sqlx::test]
async fn closed_admission_owner_cas_self_terminal_and_uniqueness(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let mut wrong = bind_cmd(&f, case.assistance_case_id, c.instance, 2);
    wrong.principal_id = PrincipalId::from_uuid(f.agent);
    assert!(bind_governance_child(&pool, wrong).await.is_err());
    assert!(
        bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 1))
            .await
            .is_err()
    );
    assert!(
        bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, f.instance, 2))
            .await
            .is_err()
    );
    let b = bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
        .await
        .unwrap();
    assert!(
        bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 3))
            .await
            .is_err()
    );
    let mut malformed = transition_command(&c, 1);
    malformed.submission_payload = Some(json!({}));
    assert!(
        execute_workflow_transition(&pool, AdmissionGate::disabled(), malformed)
            .await
            .is_err()
    );
    assert!(review_rows(&pool).await.is_empty());
    finish(&pool, &c, &b, "NOT_ACCEPTED").await;
    assert_eq!(review_rows(&pool).await.len(), 1);
    assert!(
        resolve_assistance(&pool, resolve_cmd(&f, case.assistance_case_id, json!({})))
            .await
            .is_err()
    );
}

pub(super) async fn replace(pool: &PgPool, f: &Fixture, new_owner: Uuid, key: &str) {
    svc_workflow::application::provisioning::replace_owner(
        pool,
        &svc_workflow::domain::provisioning::ReplaceOwnerCommand {
            domain_id: DomainId::from_uuid(f.domain),
            new_owner_id: PrincipalId::from_uuid(new_owner),
        },
        key,
        "gcr-isolated",
        &PrincipalId::from_uuid(f.coordinator),
    )
    .await
    .unwrap();
}
#[sqlx::test]
async fn owner_replacement_before_after_delivery_and_switch_back(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let b = bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
        .await
        .unwrap();
    finish(&pool, &c, &b, "ACCEPTED").await;
    let initial = review_rows(&pool).await;
    assert_eq!(initial.len(), 1);
    replace(&pool, &f, f.replacement_owner, "replace-1").await;
    replace(&pool, &f, f.replacement_owner, "replace-1").await;
    let rows = review_rows(&pool).await;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["ownerPrincipalId"], json!(f.replacement_owner));
    assert_ne!(rows[0]["reviewWakeId"], rows[1]["reviewWakeId"]);
    sqlx::query("UPDATE workflow_outbox SET delivered_at=now() WHERE payload->>'phase'='GOVERNANCE_CHILD_REVIEW'").execute(&pool).await.unwrap();
    replace(&pool, &f, f.owner, "replace-2").await;
    replace(&pool, &f, f.owner, "no-op").await;
    let rows = review_rows(&pool).await;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2]["ownerPrincipalId"], json!(f.owner));
    assert_ne!(rows[0]["reviewWakeId"], rows[2]["reviewWakeId"]);
    let d = serde_json::to_value(
        get_assistance_case(&pool, f.owner, case.assistance_case_id)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        d["governanceBinding"]["currentReview"]["reviewWakeId"],
        rows[2]["reviewWakeId"]
    );
    assert_eq!(
        d["governanceBinding"]["currentReview"]["reviewStatus"],
        "OUTSTANDING"
    );
}
#[sqlx::test]
async fn same_visit_context_repair_preserves_request_and_bound_child_provenance(pool: PgPool) {
    use svc_workflow::{
        application::workflow_instance::{
            revise::revise_workflow_context, revise_and_transition::revise_context_and_transition,
        },
        domain::workflow_instance::commands::{
            ReviseContextAndTransitionCommand, ReviseWorkflowContextCommand,
        },
    };
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let old: Uuid = sqlx::query_scalar(
        "SELECT current_context_revision_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(f.instance)
    .fetch_one(&pool)
    .await
    .unwrap();
    revise_workflow_context(
        &pool,
        AdmissionGate::disabled(),
        ReviseWorkflowContextCommand {
            principal_id: PrincipalId::from_uuid(f.agent),
            idempotency_key: "repair-source".into(),
            command_schema_version: "v1".into(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(f.instance),
            expected_workflow_state_version: 2,
            context_payload: json!({"repaired":true}),
        },
    )
    .await
    .unwrap();
    let b = bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 3))
        .await
        .unwrap();
    assert_eq!(b["source"]["requestContextRevisionId"], json!(old));
    let bound = b["child"]["contextRevisionAtBind"].clone();
    revise_context_and_transition(
        &pool,
        AdmissionGate::disabled(),
        ReviseContextAndTransitionCommand {
            principal_id: PrincipalId::from_uuid(c.agent),
            idempotency_key: "child-terminal".into(),
            command_schema_version: "v1".into(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(c.instance),
            expected_workflow_state_version: 1,
            transition_definition_id: TransitionId::from_uuid(c.transition),
            context_payload: json!({"terminal":true}),
            submission_payload: envelope(&b, "ACCEPTED"),
        },
    )
    .await
    .unwrap();
    let d = serde_json::to_value(
        get_assistance_case(&pool, f.owner, case.assistance_case_id)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_ne!(d["governanceTerminalProof"]["contextRevisionId"], bound);
    let p = &d["governanceTerminalProof"];
    let mut r = resolve_cmd(
        &f,
        case.assistance_case_id,
        json!({"bindingId":b["bindingId"],"terminalEventId":p["terminalEventId"],"submissionId":p["submissionId"],"submissionDigest":p["submissionDigest"],"acceptanceContractDigest":b["acceptanceContractDigest"]}),
    );
    r.expected_workflow_state_version = 4;
    resolve_assistance(&pool, r).await.unwrap();
}
#[sqlx::test]
async fn binding_event_survives_projection_recovery(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
        .await
        .unwrap();
    svc_workflow::application::workflow_instance::admin_recovery::rebuild_projection(
        &pool,
        svc_workflow::domain::workflow_instance::recovery::RebuildProjectionCommand {
            principal_id: PrincipalId::from_uuid(f.admin),
            idempotency_key: "rebuild-bound".into(),
            command_schema_version: "v1".into(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(f.instance),
            expected_before_snapshot_digest: None,
        },
    )
    .await
    .unwrap();
}
#[sqlx::test]
async fn child_input_has_only_service_derived_parent(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let b = bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
        .await
        .unwrap();
    let read=svc_workflow::store::postgres::workflow_instance_repository::current_task_input::read(&pool,f.agent,c.instance,svc_workflow::application::workflow_instance::current_task_input::CurrentTaskInputRequest::parse(vec![]).unwrap()).await.unwrap();
    assert_eq!(read["governanceParent"], b);
    assert!(svc_workflow::store::postgres::workflow_instance_repository::current_task_input::read(&pool,f.outsider,c.instance,svc_workflow::application::workflow_instance::current_task_input::CurrentTaskInputRequest::parse(vec![]).unwrap()).await.is_err());
}
#[sqlx::test]
async fn malformed_or_mismatched_envelope_never_commits_terminal(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let b = bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
        .await
        .unwrap();
    for (field, value) in [
        ("bindingId", json!(Uuid::new_v4())),
        ("acceptanceContractDigest", json!("0".repeat(64))),
        ("sourceRequestPayloadDigest", json!("1".repeat(64))),
        ("verdict", json!("PASS")),
        ("extra", json!(true)),
    ] {
        let mut p = envelope(&b, "ACCEPTED");
        p["governanceAcceptance"][field] = value;
        let mut cmd = transition_command(&c, 1);
        cmd.submission_payload = Some(p);
        assert!(
            execute_workflow_transition(&pool, AdmissionGate::disabled(), cmd)
                .await
                .is_err()
        );
    }
    assert!(review_rows(&pool).await.is_empty());
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_submissions WHERE workflow_instance_id=$1",
    )
    .bind(c.instance)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}
#[sqlx::test]
async fn terminal_before_bind_and_one_layer_edges_are_refused(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let leaf = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let mut cmd = transition_command(&leaf, 1);
    cmd.submission_payload = Some(
        json!({"governanceAcceptance":{"bindingId":Uuid::new_v4(),"acceptanceContractDigest":"0".repeat(64),"sourceRequestPayloadDigest":"0".repeat(64),"verdict":"ACCEPTED"}}),
    );
    execute_workflow_transition(&pool, AdmissionGate::disabled(), cmd)
        .await
        .unwrap();
    assert!(bind_governance_child(
        &pool,
        bind_cmd(&f, case.assistance_case_id, leaf.instance, 2)
    )
    .await
    .is_err());
    assert!(review_rows(&pool).await.is_empty());
    bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
        .await
        .unwrap();
    let childcase = request_case(&pool, &c).await;
    let another = child(&pool, &f).await;
    assert!(bind_governance_child(
        &pool,
        bind_cmd(&c, childcase.assistance_case_id, another.instance, 2)
    )
    .await
    .is_err());
    let anothercase = request_case(&pool, &another).await;
    assert!(bind_governance_child(
        &pool,
        bind_cmd(&another, anothercase.assistance_case_id, f.instance, 2)
    )
    .await
    .is_err());
}
