use super::*;

#[test]
fn transition_context_presence_is_not_collapsed_by_the_http_dto() {
    let base = json!({"transitionDefinitionId":Uuid::new_v4(),"expectedWorkflowStateVersion":1});
    assert!(
        serde_json::from_value::<svc_workflow::http::dto::ExecuteWorkflowTransitionRequest>(
            base.clone()
        )
        .is_ok()
    );
    let mut present = base;
    present["contextPayload"] = Value::Null;
    assert!(serde_json::from_value::<svc_workflow::http::dto::ExecuteWorkflowTransitionRequest>(present).is_ok(), "the adapter must preserve explicit null for receipted invalid_input, rather than reject an unknown field");
}

#[tokio::test]
async fn context_real_two_gate_binding_wait_ready_return_and_acb_revision_lineage() {
    use super::context_support as c;
    let p = pool().await;
    let f = c::fixture(&p).await;
    let jwk = common::MockJwksServer::start().await;
    let app = http_support::app(&p, &jwk, f.pm).await;
    let old = c::current(&p, f.managed).await;
    let effect1 = c::effect(&p, &f).await;
    let leaf1: Uuid =
        serde_json::from_value(effect1["result"]["workflowInstanceId"].clone()).unwrap();
    let first = c::primary(&p, f.entry).await;
    let base = json!({"transitionDefinitionId":first,"expectedWorkflowStateVersion":old.0,"submissionPayload":c::report(old.1)});
    let mut bound = c::initial();
    bound["workflowPrerequisite"] = c::reference(&f, leaf1);
    let mut cases = vec![(None, "context_validation_failed")];
    for invalid in [
        Value::Null,
        json!([]),
        json!("text"),
        json!(3),
        json!(false),
    ] {
        cases.push((Some(invalid), "invalid_input"));
    }
    let mut changed = bound.clone();
    changed["businessInput"]["immutable"] = json!(2);
    cases.push((Some(changed), "context_validation_failed"));
    let mut catalog = bound.clone();
    catalog["acceptanceCriteria"][0]["id"] = json!("different");
    cases.push((Some(catalog), "context_validation_failed"));
    let mut forged = bound.clone();
    forged["workflowPrerequisite"]["workflowInstanceId"] = json!(Uuid::new_v4());
    cases.push((Some(forged), "context_validation_failed"));
    let mut partial = bound.clone();
    partial["workflowPrerequisite"]
        .as_object_mut()
        .unwrap()
        .remove("transitionId");
    cases.push((Some(partial), "context_validation_failed"));
    for (field, value) in [
        ("definitionVersionId", json!(Uuid::new_v4())),
        ("transitionId", json!(Uuid::new_v4())),
        ("kind", json!("OTHER")),
        ("extra", json!(true)),
    ] {
        let mut wrong = bound.clone();
        wrong["workflowPrerequisite"][field] = value;
        cases.push((Some(wrong), "context_validation_failed"));
    }
    let before = history(&p).await;
    for (i, (context, code)) in cases.into_iter().enumerate() {
        let mut body = base.clone();
        if let Some(v) = context {
            body["contextPayload"] = v;
        }
        let key = format!("negative-{i}");
        let (s, r) = c::post(app.clone(), &jwk, f.owner, f.managed, &key, body.clone()).await;
        assert_eq!(s, 422, "{r}");
        assert_eq!(r["error"]["code"], code, "{r}");
        let (s, replay) = c::post(app.clone(), &jwk, f.owner, f.managed, &key, body).await;
        assert_eq!(s, 422, "{replay}");
        assert_eq!(replay, r);
        assert_eq!(c::current(&p, f.managed).await, old);
    }
    let after = history(&p).await;
    for key in ["instances", "contexts", "activations", "events"] {
        assert_eq!(before[key], after[key], "failure changed {key}");
    }
    let mut valid = base;
    valid["contextPayload"] = bound.clone();
    let (s, r) = c::post(
        app.clone(),
        &jwk,
        f.owner,
        f.managed,
        "actual-bind-one",
        valid.clone(),
    )
    .await;
    assert_eq!(s, 200, "{r}");
    let one = c::current(&p, f.managed).await;
    assert_eq!(one.0, old.0 + 1);
    assert_ne!(one.1, old.1);
    assert_eq!(one.3, f.gate1);
    assert_eq!(r["currentContextRevisionId"], json!(one.1));
    let event:(Uuid,Uuid,Value)=sqlx::query_as("SELECT context_revision_id,submission_id,event_data FROM workflow_events WHERE workflow_instance_id=$1 AND new_workflow_state_version=$2").bind(f.managed).bind(one.0).fetch_one(&p).await.unwrap();
    assert_eq!(event.0, old.1);
    assert_eq!(event.2["previousContextRevisionId"], json!(old.1));
    assert_eq!(event.2["newContextRevisionId"], json!(one.1));
    let sub: Uuid = sqlx::query_scalar(
        "SELECT context_revision_id FROM workflow_submissions WHERE submission_id=$1",
    )
    .bind(event.1)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(sub, old.1);
    let (s, replay) = c::post(
        app.clone(),
        &jwk,
        f.owner,
        f.managed,
        "actual-bind-one",
        valid.clone(),
    )
    .await;
    assert_eq!(s, 200);
    assert_eq!(r, replay);
    valid["contextPayload"]["businessInput"]["immutable"] = json!(2);
    let (s, r) = c::post(
        app.clone(),
        &jwk,
        f.owner,
        f.managed,
        "actual-bind-one",
        valid,
    )
    .await;
    assert_eq!(s, 409, "{r}");
    sqlx::query("INSERT INTO global_role_bindings(binding_id,principal_id,role_key) VALUES($1,$2,'GLOBAL_SCHEDULER_READ')").bind(Uuid::new_v4()).bind(f.pm).execute(&p).await.unwrap();
    let ready =
        svc_workflow::store::postgres::workflow_instance_repository::query_ready_intents::list(
            &p, f.pm, 100, None,
        )
        .await
        .unwrap();
    assert!(!ready
        .iter()
        .any(|r| r.due.workflow_instance_id == f.managed));
    let supplier_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_node_visits WHERE workflow_instance_id=$1",
    )
    .bind(f.managed)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(supplier_before, 2);
    transition(&p, f.owner, leaf1, 1, f.terminal, Some(json!({})))
        .await
        .unwrap();
    let ready =
        svc_workflow::store::postgres::workflow_instance_repository::query_ready_intents::list(
            &p, f.pm, 100, None,
        )
        .await
        .unwrap();
    assert!(ready
        .iter()
        .any(|r| r.due.workflow_instance_id == f.managed));
    let next = c::primary(&p, f.gate1).await;
    let(s,r)=c::post(app.clone(),&jwk,f.owner,f.managed,"old-acb",json!({"transitionDefinitionId":next,"expectedWorkflowStateVersion":one.0,"submissionPayload":c::report(old.1)})).await;
    assert_eq!(s, 422, "{r}");
    assert_eq!(r["error"]["code"], "submission_validation_failed");
    let omitted = json!({"transitionDefinitionId":next,"expectedWorkflowStateVersion":one.0,"submissionPayload":c::report(one.1)});
    let (s, r) = c::post(
        app.clone(),
        &jwk,
        f.owner,
        f.managed,
        "omit-unchanged",
        omitted,
    )
    .await;
    assert_eq!(s, 200, "{r}");
    let two = c::current(&p, f.managed).await;
    assert_eq!(two.1, one.1);
    assert_eq!(two.3, f.producer2);
    let no_pair:Value=sqlx::query_scalar("SELECT event_data FROM workflow_events WHERE workflow_instance_id=$1 AND new_workflow_state_version=$2").bind(f.managed).bind(two.0).fetch_one(&p).await.unwrap();
    assert!(no_pair.get("newContextRevisionId").is_none());
    let effect2 = c::effect(&p, &f).await;
    let leaf2: Uuid =
        serde_json::from_value(effect2["result"]["workflowInstanceId"].clone()).unwrap();
    assert_ne!(leaf1, leaf2);
    let mut bound2 = bound.clone();
    bound2["workflowPrerequisite"] = c::reference(&f, leaf2);
    let(s,r)=c::post(app.clone(),&jwk,f.owner,f.managed,"actual-bind-two",json!({"transitionDefinitionId":c::primary(&p,f.producer2).await,"expectedWorkflowStateVersion":two.0,"submissionPayload":c::report(two.1),"contextPayload":bound2})).await;
    assert_eq!(s, 200, "{r}");
    let three = c::current(&p, f.managed).await;
    assert_eq!(three.3, f.gate2);
    assert_ne!(three.1, two.1);
    let ret = json!({"rootCauseNodeVisitId":three.2,"reasonCode":"QA","reason":"revisit earlier real supplier"});
    let(s,r)=c::post(app.clone(),&jwk,f.owner,f.managed,"return-rebind",json!({"transitionDefinitionId":f.back1,"expectedWorkflowStateVersion":three.0,"submissionPayload":ret,"contextPayload":bound})).await;
    assert_eq!(s, 200, "{r}");
    let four = c::current(&p, f.managed).await;
    assert_eq!(four.3, f.gate1);
    assert_ne!(four.1, three.1);
    let(s,r)=c::post(app.clone(),&jwk,f.owner,f.managed,"again-first",json!({"transitionDefinitionId":next,"expectedWorkflowStateVersion":four.0,"submissionPayload":c::report(four.1)})).await;
    assert_eq!(s, 200, "{r}");
    let replay = c::effect(&p, &f).await;
    assert_eq!(replay["effectId"], effect2["effectId"]);
    assert_eq!(replay["result"], effect2["result"]);
    assert_eq!(replay["replayed"], true);
    let five = c::current(&p, f.managed).await;
    let(s,r)=c::post(app.clone(),&jwk,f.owner,f.managed,"reenter-second",json!({"transitionDefinitionId":c::primary(&p,f.producer2).await,"expectedWorkflowStateVersion":five.0,"submissionPayload":c::report(five.1),"contextPayload":bound2})).await;
    assert_eq!(s, 200, "{r}");
    let six = c::current(&p, f.managed).await;
    transition(&p, f.owner, leaf2, 1, f.terminal, Some(json!({})))
        .await
        .unwrap();
    let(s,r)=c::post(app.clone(),&jwk,f.owner,f.managed,"after-second",json!({"transitionDefinitionId":c::primary(&p,f.gate2).await,"expectedWorkflowStateVersion":six.0,"submissionPayload":c::report(six.1)})).await;
    assert_eq!(s, 200, "{r}");
    let seven = c::current(&p, f.managed).await;
    assert_eq!(seven.3, f.qa);
    let qa: Uuid = sqlx::query_scalar(
        "SELECT assignee_principal_id FROM workflow_node_visits WHERE node_visit_id=$1",
    )
    .bind(seven.2)
    .fetch_one(&p)
    .await
    .unwrap();
    let(s,r)=c::post(app.clone(),&jwk,qa,f.managed,"identical-return",json!({"transitionDefinitionId":f.back2,"expectedWorkflowStateVersion":seven.0,"submissionPayload":{"rootCauseNodeVisitId":seven.2,"reasonCode":"QA","reason":"revisit exact binding"},"contextPayload":bound2})).await;
    assert_eq!(s, 200, "{r}");
    let eight = c::current(&p, f.managed).await;
    assert_eq!(eight.1, seven.1);
    let last:Value=sqlx::query_scalar("SELECT event_data FROM workflow_events WHERE workflow_instance_id=$1 AND new_workflow_state_version=$2").bind(f.managed).bind(eight.0).fetch_one(&p).await.unwrap();
    assert!(last.get("newContextRevisionId").is_none());
    assert!(last.get("previousContextRevisionId").is_none());
    let effects: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_operation_effects WHERE managed_instance_id=$1",
    )
    .bind(f.managed)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(effects, 2);
    println!("actual late binding EventData: {}; currentContextRevisionId={}; exactly two real effects survive multi-gate RETURN",event.2,eight.1);
}

#[tokio::test]
async fn context_absent_ordinary_hash_wire_compatibility_and_present_outside_scope() {
    let p = pool().await;
    let (owner, domain) = seed(&p).await;
    let pm = actor(&p).await;
    let (v, t) = version(&p, domain, owner, json!({"type":"object"})).await;
    publish(&p, owner, v, None, "publish").await.unwrap();
    let s = create(&p, owner, domain, v, json!({"business":1}))
        .await
        .unwrap();
    let jwk = common::MockJwksServer::start().await;
    let app = http_support::app(&p, &jwk, pm).await;
    let base = json!({"transitionDefinitionId":t,"expectedWorkflowStateVersion":1});
    for (i, present) in [
        json!({"business":1}),
        Value::Null,
        json!([]),
        json!("x"),
        json!(1),
        json!(true),
    ]
    .into_iter()
    .enumerate()
    {
        let mut body = base.clone();
        body["contextPayload"] = present;
        let (s, r) = super::context_support::post(
            app.clone(),
            &jwk,
            owner,
            s.workflow_instance_id,
            &format!("out-of-scope-{i}"),
            body.clone(),
        )
        .await;
        assert_eq!(s, 422, "{r}");
        assert_eq!(r["error"]["code"], "invalid_input");
    }
    let (status, r) = super::context_support::post(
        app.clone(),
        &jwk,
        owner,
        s.workflow_instance_id,
        "original-absent",
        base.clone(),
    )
    .await;
    assert_eq!(status, 200, "{r}");
    assert_eq!(
        r["currentContextRevisionId"],
        json!(s.current_context_revision_id)
    );
    let stored_hash:String=sqlx::query_scalar("SELECT request_hash FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key='original-absent'").bind(owner).fetch_one(&p).await.unwrap();
    use svc_workflow::domain::ids::*;
    let old_hash =
        svc_workflow::application::workflow_instance::idempotency::compute_transition_request_hash(
            "v1",
            "original-absent",
            &PrincipalId::from_uuid(owner),
            &WorkflowInstanceId::from_uuid(s.workflow_instance_id),
            1,
            &TransitionId::from_uuid(t),
            &None,
        )
        .unwrap();
    assert_eq!(stored_hash, old_hash);
    let (_, replay) = super::context_support::post(
        app.clone(),
        &jwk,
        owner,
        s.workflow_instance_id,
        "original-absent",
        base.clone(),
    )
    .await;
    assert_eq!(replay, r);
    let mut explicit_null = base;
    explicit_null["contextPayload"] = Value::Null;
    let (status, r) = super::context_support::post(
        app,
        &jwk,
        owner,
        s.workflow_instance_id,
        "original-absent",
        explicit_null,
    )
    .await;
    assert_eq!(status, 409, "{r}");
    let data:Value=sqlx::query_scalar("SELECT event_data FROM workflow_events WHERE workflow_instance_id=$1 AND event_type='WORKFLOW_TRANSITION_COMMITTED'").bind(s.workflow_instance_id).fetch_one(&p).await.unwrap();
    assert!(data.get("previousContextRevisionId").is_none());
    assert!(data.get("newContextRevisionId").is_none());
}

#[tokio::test]
async fn context_fault_after_append_rolls_back_receipt_context_visit_and_event() {
    use super::context_support as c;
    let p = pool().await;
    let f = c::fixture(&p).await;
    let e = c::effect(&p, &f).await;
    let leaf = serde_json::from_value(e["result"]["workflowInstanceId"].clone()).unwrap();
    let old = c::current(&p, f.managed).await;
    let mut payload = c::initial();
    payload["workflowPrerequisite"] = c::reference(&f, leaf);
    let jwk = common::MockJwksServer::start().await;
    let app = http_support::app(&p, &jwk, f.pm).await;
    let body = json!({"transitionDefinitionId":c::primary(&p,f.entry).await,"expectedWorkflowStateVersion":old.0,"submissionPayload":c::report(old.1),"contextPayload":payload});
    let mut stale = body.clone();
    stale["expectedWorkflowStateVersion"] = json!(0);
    let (s, r) = c::post(
        app.clone(),
        &jwk,
        f.owner,
        f.managed,
        "stale-context",
        stale,
    )
    .await;
    assert_eq!(s, 409, "{r}");
    assert_eq!(c::current(&p, f.managed).await, old);
    sqlx::query("CREATE FUNCTION isolated_context_event_fault() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'isolated post-context fault'; END $$").execute(&p).await.unwrap();
    sqlx::query("CREATE TRIGGER isolated_context_event_fault BEFORE INSERT ON workflow_events FOR EACH ROW WHEN (NEW.event_type='WORKFLOW_TRANSITION_COMMITTED') EXECUTE FUNCTION isolated_context_event_fault()").execute(&p).await.unwrap();
    let before = history(&p).await;
    let (s, r) = c::post(app, &jwk, f.owner, f.managed, "faulted-context", body).await;
    assert_eq!(s, 503, "{r}");
    assert_eq!(before, history(&p).await);
    assert_eq!(c::current(&p, f.managed).await, old);
}
