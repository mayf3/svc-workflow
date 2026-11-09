use super::*;
async fn fresh() -> PgPool {
    let database = format!("svc_admission_obs_{}", Uuid::new_v4().simple());
    let admin = PgPool::connect(&common::admin_database_url())
        .await
        .unwrap();
    sqlx::query(&format!("CREATE DATABASE {database}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
    println!("fresh observation database: {database}");
    let pool = PgPool::connect(&format!("{}/{database}", common::test_database_base()))
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    pool
}
#[tokio::test]
async fn ordinary_http_preserves_actual_waiting_admission() {
    let p = fresh().await;
    let f = setup(&p).await;
    let (sv, st) = version(&p, &f, false, false).await;
    let source = create(&p, &f, sv, json!({})).await;
    let (dv, _) = version(&p, &f, true, false).await;
    let mock = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &mock).await;
    let body = json!({"domainId":f.domain,"definitionVersionId":dv,"executionClass":"BUSINESS","metadata":{},"contextPayload":reference(source,sv,st)});
    let (status, response) = http(
        router,
        f.owner,
        &mock,
        "POST",
        "/internal/v1/workflow-instances",
        Some(&Uuid::new_v4().to_string()),
        body,
    )
    .await;
    assert_eq!(status, 201, "{response}");
    assert_eq!(
        response["admissionObservation"]["status"], "WAITING",
        "actual create-time observation required: {response}"
    );
}
async fn finish_source(p: &PgPool, f: &Fixture, source: Uuid, t: Uuid) {
    let visit: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(p)
    .await
    .unwrap();
    let sf = Fixture {
        instance: source,
        visit,
        transition: t,
        ..*f
    };
    let mut c = transition_command(&sf, 1);
    c.submission_payload = Some(json!({"z":2,"a":"实际 terminal"}));
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        p,
        crate::common::admitting_gate(),
        c,
    )
    .await
    .unwrap();
}
fn create_body(f: &Fixture, v: Uuid, c: Value) -> Value {
    json!({"domainId":f.domain,"definitionVersionId":v,"executionClass":"BUSINESS","metadata":{},"contextPayload":c})
}
async fn assert_binding(
    p: &PgPool,
    body: &Value,
    actor: Uuid,
    key: &str,
    source: Uuid,
    sv: Uuid,
    t: Uuid,
    status: &str,
) {
    let id: Uuid = body["workflowInstanceId"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let facts:(Uuid,Uuid,Value,String,Uuid,Uuid)=sqlx::query_as("SELECT wi.definition_version_id,c.context_revision_id,c.payload,c.payload_digest,v.node_visit_id,e.event_id FROM workflow_instances wi JOIN workflow_context_revisions c ON c.workflow_instance_id=wi.workflow_instance_id AND c.revision_number=1 JOIN workflow_node_visits v ON v.workflow_instance_id=wi.workflow_instance_id AND v.visit_number=1 JOIN workflow_events e ON e.workflow_instance_id=wi.workflow_instance_id AND e.event_type='INSTANCE_CREATED' WHERE wi.workflow_instance_id=$1").bind(id).fetch_one(p).await.unwrap();
    assert_eq!(
        facts.3,
        svc_workflow::domain::definition::digest::compute_json_digest(&facts.2).unwrap()
    );
    let mut expected = json!({"version":1,"kind":"TERMINAL_SUBMISSION","status":status,"dependentInstanceId":id,"dependentDefinitionVersionId":facts.0,"initialContextRevisionId":facts.1,"initialContextSha256":facts.3,"initialVisitId":facts.4,"birthEventId":facts.5,"sourceInstanceId":source,"sourceDefinitionVersionId":sv,"transitionId":t});
    if status == "READY" {
        let proof:(Uuid,Uuid,String)=sqlx::query_as("SELECT e.event_id,s.submission_id,s.payload_digest FROM workflow_events e JOIN workflow_submissions s USING(submission_id) WHERE e.workflow_instance_id=$1 AND s.workflow_instance_id=$1").bind(source).fetch_one(p).await.unwrap();
        expected["eventId"] = json!(proof.0);
        expected["submissionId"] = json!(proof.1);
        expected["submissionSha256"] = json!(proof.2);
    }
    assert_eq!(
        body["admissionObservation"], expected,
        "closed exact full binding"
    );
    let stored:(Value,String)=sqlx::query_as("SELECT response_body,response_digest FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2").bind(actor).bind(key).fetch_one(p).await.unwrap();
    println!(
        "creation admission evidence: {}",
        json!({"httpBody":body,"storedResponse":stored.0,"initialContextPayload":facts.2,"initialContextSha256":facts.3,"expectedObservationFromActualFacts":expected})
    );
    assert_eq!(stored.0, *body);
    assert_eq!(
        stored.1,
        svc_workflow::domain::definition::digest::compute_json_digest(body).unwrap()
    );
    assert_eq!(body["currentContextRevisionId"], json!(facts.1));
    assert_eq!(body["currentNodeVisitId"], json!(facts.4));
    assert!(!sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM workflow_events WHERE workflow_instance_id=$1 AND event_data::text LIKE '%admissionObservation%')").bind(id).fetch_one(p).await.unwrap());
}
#[tokio::test]
async fn waiting_and_ready_full_binding_http_receipts_and_live_reader_privacy() {
    let p = fresh().await;
    let f = setup(&p).await;
    let (sv, st) = version(&p, &f, false, false).await;
    let source = create(&p, &f, sv, json!({})).await;
    let (dv, _) = version(&p, &f, true, false).await;
    let mock = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &mock).await;
    for status in ["WAITING", "READY"] {
        if status == "READY" {
            finish_source(&p, &f, source, st).await;
        }
        let key = Uuid::new_v4().to_string();
        let mut context = reference(source, sv, st);
        context["unicode"] = json!({"z":1,"a":"字符"});
        let (code, b) = http(
            router.clone(),
            f.owner,
            &mock,
            "POST",
            "/internal/v1/workflow-instances",
            Some(&key),
            create_body(&f, dv, context),
        )
        .await;
        assert_eq!(code, 201, "{b}");
        assert_binding(&p, &b, f.owner, &key, source, sv, st, status).await;
        let id = b["workflowInstanceId"].as_str().unwrap();
        for suffix in ["", "/readiness", "/timeline"] {
            let (code, read) = http(
                router.clone(),
                f.agent,
                &mock,
                "GET",
                &format!("/internal/v1/workflow-instances/{id}{suffix}"),
                None,
                json!({}),
            )
            .await;
            assert_eq!(code, 200, "{read}");
            assert!(!read.to_string().contains("admissionObservation"));
        }
        assert_eq!(
            http(
                router.clone(),
                f.outsider,
                &mock,
                "GET",
                &format!("/internal/v1/workflow-instances/{id}/readiness"),
                None,
                json!({})
            )
            .await
            .0,
            404
        );
    }
}
#[tokio::test]
async fn replay_is_historical_after_source_progress_and_owner_loss_and_legacy_stays_absent() {
    let p = fresh().await;
    let f = setup(&p).await;
    let (sv, st) = version(&p, &f, false, false).await;
    let source = create(&p, &f, sv, json!({})).await;
    let (dv, _) = version(&p, &f, true, false).await;
    let mock = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &mock).await;
    let key = Uuid::new_v4().to_string();
    let body = create_body(&f, dv, reference(source, sv, st));
    let (code, first) = http(
        router.clone(),
        f.owner,
        &mock,
        "POST",
        "/internal/v1/workflow-instances",
        Some(&key),
        body.clone(),
    )
    .await;
    assert_eq!(code, 201);
    assert_binding(&p, &first, f.owner, &key, source, sv, st, "WAITING").await;
    finish_source(&p, &f, source, st).await;
    let id: Uuid = first["workflowInstanceId"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let live = svc_workflow::store::postgres::workflow_instance_repository::prerequisite::read(
        &p, f.agent, id,
    )
    .await
    .unwrap();
    assert_eq!(live["prerequisiteReadiness"]["status"], "READY");
    sqlx::query("UPDATE domain_role_bindings SET enabled=false WHERE domain_id=$1 AND principal_id=$2 AND role_key='DOMAIN_OWNER'").bind(f.domain).bind(f.owner).execute(&p).await.unwrap();
    let (code, replay) = http(
        router.clone(),
        f.owner,
        &mock,
        "POST",
        "/internal/v1/workflow-instances",
        Some(&key),
        body.clone(),
    )
    .await;
    assert_eq!(code, 201);
    assert_eq!(replay, first);
    assert_eq!(
        serde_json::to_vec(&replay["admissionObservation"]).unwrap(),
        serde_json::to_vec(&first["admissionObservation"]).unwrap()
    );
    let mut changed = body.clone();
    changed["metadata"] = json!({"changed":true});
    assert_eq!(
        http(
            router.clone(),
            f.owner,
            &mock,
            "POST",
            "/internal/v1/workflow-instances",
            Some(&key),
            changed
        )
        .await
        .0,
        409
    );
    assert_ne!(
        http(
            router.clone(),
            f.outsider,
            &mock,
            "POST",
            "/internal/v1/workflow-instances",
            Some(&key),
            body.clone()
        )
        .await
        .0,
        201
    );
    // A real receipt with all original facts, seeded to the historical pre-V3
    // response shape only inside this fresh DB. Keep its JCS digest coherent.
    let mut legacy = first.clone();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("admissionObservation");
    let digest = svc_workflow::domain::definition::digest::compute_json_digest(&legacy).unwrap();
    // Historical-shape seeding bypass is transaction-local in this test-owned
    // fresh database. Production COMPLETED receipt immutability is unchanged.
    let mut tx = p.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE workflow_command_receipts SET response_body=$1,response_digest=$2 WHERE principal_id=$3 AND idempotency_key=$4").bind(&legacy).bind(digest).bind(f.owner).bind(&key).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let (code, replay) = http(
        router,
        f.owner,
        &mock,
        "POST",
        "/internal/v1/workflow-instances",
        Some(&key),
        body,
    )
    .await;
    assert_eq!(code, 201);
    assert_eq!(replay, legacy);
    assert!(replay.get("admissionObservation").is_none());
}
#[tokio::test]
async fn dependency_free_http_response_and_receipt_omit_observation() {
    let p = fresh().await;
    let f = setup(&p).await;
    let (v, _) = version(&p, &f, false, false).await;
    let mock = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &mock).await;
    let key = Uuid::new_v4().to_string();
    let (code, b) = http(
        router,
        f.owner,
        &mock,
        "POST",
        "/internal/v1/workflow-instances",
        Some(&key),
        create_body(&f, v, json!({})),
    )
    .await;
    assert_eq!(code, 201);
    assert_eq!(b.as_object().unwrap().len(), 5);
    assert!(b.get("admissionObservation").is_none());
    let receipt:Value=sqlx::query_scalar("SELECT response_body FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2").bind(f.owner).bind(key).fetch_one(&p).await.unwrap();
    assert_eq!(receipt, b);
}
#[tokio::test]
async fn failures_keep_deterministic_receipts_without_observations_or_facts() {
    let p = fresh().await;
    let f = setup(&p).await;
    let (sv, st) = version(&p, &f, false, false).await;
    let (dv, _) = version(&p, &f, true, false).await;
    let closed = create(&p, &f, sv, json!({})).await;
    sqlx::query("UPDATE workflow_instances SET cancelled=true WHERE workflow_instance_id=$1")
        .bind(closed)
        .execute(&p)
        .await
        .unwrap();
    let unknown = create(&p, &f, sv, json!({})).await;
    finish_source(&p, &f, unknown, st).await;
    let mut tx = p.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE workflow_submissions SET payload_digest=$2 WHERE workflow_instance_id=$1")
        .bind(unknown)
        .bind("0".repeat(64))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let foreign = setup(&p).await;
    let (fv, ft) = version(&p, &foreign, false, false).await;
    let fi = create(&p, &foreign, fv, json!({})).await;
    let mock = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &mock).await;
    for (actor, context) in [
        (f.owner, reference(closed, sv, st)),
        (f.owner, reference(unknown, sv, st)),
        (f.owner, reference(fi, fv, ft)),
        (f.owner, json!({})),
        (f.agent, reference(closed, sv, st)),
    ] {
        let key = Uuid::new_v4().to_string();
        let body = create_body(&f, dv, context);
        let (code, b) = http(
            router.clone(),
            actor,
            &mock,
            "POST",
            "/internal/v1/workflow-instances",
            Some(&key),
            body.clone(),
        )
        .await;
        assert!(code >= 400, "{b}");
        assert!(b.get("admissionObservation").is_none());
        assert_eq!(
            http(
                router.clone(),
                actor,
                &mock,
                "POST",
                "/internal/v1/workflow-instances",
                Some(&key),
                body
            )
            .await,
            (code, b)
        );
        let row:(Uuid,i32,Value)=sqlx::query_as("SELECT command_id,response_status,response_body FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2 AND receipt_status='COMPLETED'").bind(actor).bind(key).fetch_one(&p).await.unwrap();
        assert!(row.1 >= 400);
        assert!(row.2.get("admissionObservation").is_none());
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM workflow_events WHERE command_id=$1"
            )
            .bind(row.0)
            .fetch_one(&p)
            .await
            .unwrap(),
            0
        );
    }
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
}
#[tokio::test]
async fn known_rollback_at_each_fact_receipt_and_deferred_commit_boundary_leaves_no_success() {
    let p = fresh().await;
    let f = setup(&p).await;
    let (sv, st) = version(&p, &f, false, false).await;
    let source = create(&p, &f, sv, json!({})).await;
    let (dv, _) = version(&p, &f, true, false).await;
    for table in [
        "workflow_instances",
        "workflow_context_revisions",
        "workflow_node_visits",
        "workflow_events",
        "workflow_outbox",
        "workflow_command_receipts",
        "deferred_commit",
    ] {
        let actual = if table == "deferred_commit" {
            "workflow_command_receipts"
        } else {
            table
        };
        sqlx::query("CREATE FUNCTION fail_observation_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'isolated observation boundary fault'; END $$").execute(&p).await.unwrap();
        let receipt = actual == "workflow_command_receipts";
        let ddl = if table == "deferred_commit" {
            format!("CREATE CONSTRAINT TRIGGER observation_fault AFTER UPDATE ON {actual} DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.receipt_status='COMPLETED') EXECUTE FUNCTION fail_observation_test()")
        } else {
            format!("CREATE TRIGGER observation_fault BEFORE {} ON {actual} FOR EACH ROW {} EXECUTE FUNCTION fail_observation_test()",if receipt{"UPDATE"}else{"INSERT"},if receipt{"WHEN (NEW.receipt_status='COMPLETED')"}else{""})
        };
        sqlx::query(&ddl).execute(&p).await.unwrap();
        let before:Value=sqlx::query_scalar("SELECT jsonb_build_array((SELECT count(*) FROM workflow_instances),(SELECT count(*) FROM workflow_context_revisions),(SELECT count(*) FROM workflow_node_visits),(SELECT count(*) FROM workflow_events),(SELECT count(*) FROM workflow_outbox),(SELECT count(*) FROM workflow_command_receipts))").fetch_one(&p).await.unwrap();
        let cmd = create_cmd(&f, dv, reference(source, sv, st));
        let key = cmd.idempotency_key.clone();
        let outcome = create_workflow_instance(&p, crate::common::admitting_gate(), cmd).await;
        assert!(
            outcome.is_err(),
            "{table} must not claim successful observation"
        );
        let after:Value=sqlx::query_scalar("SELECT jsonb_build_array((SELECT count(*) FROM workflow_instances),(SELECT count(*) FROM workflow_context_revisions),(SELECT count(*) FROM workflow_node_visits),(SELECT count(*) FROM workflow_events),(SELECT count(*) FROM workflow_outbox),(SELECT count(*) FROM workflow_command_receipts))").fetch_one(&p).await.unwrap();
        assert_eq!(before, after, "rollback {table}");
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM workflow_command_receipts WHERE idempotency_key=$1"
            )
            .bind(key)
            .fetch_one(&p)
            .await
            .unwrap(),
            0
        );
        sqlx::query(&format!("DROP TRIGGER observation_fault ON {actual}"))
            .execute(&p)
            .await
            .unwrap();
        sqlx::query("DROP FUNCTION fail_observation_test()")
            .execute(&p)
            .await
            .unwrap();
    }
}
#[tokio::test]
async fn gcr_reuses_admitted_payload_without_full_observation_or_extra_event_and_late_failure_rolls_back(
) {
    let p = fresh().await;
    let f = setup(&p).await;
    let (sv, st) = version(&p, &f, false, false).await;
    let source = create(&p, &f, sv, json!({})).await;
    let mock = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &mock).await;
    for status in ["WAITING", "READY", "LATE_REJECT"] {
        if status == "READY" {
            finish_source(&p, &f, source, st).await;
        }
        // Same-Domain source reference: this extra source/case must use original
        // fixture domain; request a fresh source visit case on that Domain.
        let source_case = create(&p, &f, sv, json!({})).await;
        let visit: Uuid = sqlx::query_scalar(
            "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
        )
        .bind(source_case)
        .fetch_one(&p)
        .await
        .unwrap();
        let cf = Fixture {
            instance: source_case,
            visit,
            ..f
        };
        let case = request_case(&p, &cf).await;
        let (cv, ct) = version(&p, &f, true, true).await;
        let key = Uuid::new_v4().to_string();
        let path = format!(
            "/internal/v1/assistance-cases/{}/governance-child:create-and-bind",
            case.assistance_case_id
        );
        let body = json!({"definitionVersionId":cv,"contextPayload":reference(source,sv,st),"acceptanceTransitionId":if status=="LATE_REJECT"{Uuid::new_v4()}else{ct},"expectedWorkflowStateVersion":2});
        let (code, b) = http(
            router.clone(),
            f.owner,
            &mock,
            "POST",
            &path,
            Some(&key),
            body.clone(),
        )
        .await;
        if status == "LATE_REJECT" {
            assert!(code >= 400, "{b}");
            assert!(b.get("prerequisiteReadiness").is_none());
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM workflow_instances WHERE definition_version_id=$1"
                )
                .bind(cv)
                .fetch_one(&p)
                .await
                .unwrap(),
                0
            );
        } else {
            assert_eq!(code, 201, "{b}");
            assert_eq!(b["prerequisiteReadiness"]["status"], status);
            assert!(b.get("admissionObservation").is_none());
            assert_eq!(
                b["prerequisiteReadiness"]["sourceInstanceId"],
                json!(source)
            );
            assert_eq!(
                b["prerequisiteReadiness"].as_object().unwrap().len(),
                if status == "READY" { 7 } else { 5 }
            );
        }
        let receipt:(Uuid,Value)=sqlx::query_as("SELECT command_id,response_body FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2").bind(f.owner).bind(&key).fetch_one(&p).await.unwrap();
        assert!(receipt.1.get("admissionObservation").is_none());
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM workflow_events WHERE command_id=$1"
            )
            .bind(receipt.0)
            .fetch_one(&p)
            .await
            .unwrap(),
            if status == "LATE_REJECT" { 0 } else { 2 }
        );
        assert_eq!(
            http(
                router.clone(),
                f.owner,
                &mock,
                "POST",
                &path,
                Some(&key),
                body
            )
            .await,
            (code, b)
        );
    }
}
#[tokio::test]
async fn concurrent_source_commit_and_create_observation_has_one_real_serialization_point() {
    let p = fresh().await;
    let f = setup(&p).await;
    let (sv, st) = version(&p, &f, false, false).await;
    let (dv, _) = version(&p, &f, true, false).await;
    for _ in 0..3 {
        let source = create(&p, &f, sv, json!({})).await;
        let cmd = create_cmd(&f, dv, reference(source, sv, st));
        let key = cmd.idempotency_key.clone();
        let (created, ()) = tokio::join!(
            create_workflow_instance(&p, crate::common::admitting_gate(), cmd),
            finish_source(&p, &f, source, st)
        );
        let created = created.unwrap();
        let wire = svc_workflow::http::dto::CreateWorkflowInstanceResponse::from(created);
        let body = serde_json::to_value(wire).unwrap();
        let status = body["admissionObservation"]["status"].as_str().unwrap();
        assert!(["WAITING", "READY"].contains(&status));
        assert_binding(&p, &body, f.owner, &key, source, sv, st, status).await;
        let live = svc_workflow::store::postgres::workflow_instance_repository::prerequisite::read(
            &p,
            f.agent,
            body["workflowInstanceId"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(live["prerequisiteReadiness"]["status"], "READY");
    }
}
