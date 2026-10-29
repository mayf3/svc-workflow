use super::*;
#[tokio::test]
async fn ready_poll_filters_blocked_prefix_before_page_and_restarts() {
    let p = pool().await;
    let f = setup(&p).await;
    let (v, t) = version(&p, &f, false, false).await;
    let source = create(&p, &f, v, json!({})).await;
    let (dv, _) = version(&p, &f, true, false).await;
    let mut waiting = Vec::new();
    for _ in 0..5 {
        waiting.push(create(&p, &f, dv, reference(source, v, t)).await)
    }
    let ready = create(&p, &f, v, json!({})).await;
    sqlx::query("INSERT INTO global_role_bindings(binding_id,principal_id,role_key,enabled) VALUES($1,$2,'GLOBAL_SCHEDULER_READ',true)").bind(Uuid::new_v4()).bind(f.coordinator).execute(&p).await.unwrap();
    let mock = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &mock).await;
    let mut path = "/internal/v1/dispatch-intents/ready?limit=2".to_string();
    let mut found = false;
    let mut seen = std::collections::HashSet::new();
    loop {
        let (s, b) = http(
            router.clone(),
            f.coordinator,
            &mock,
            "GET",
            &path,
            None,
            json!({}),
        )
        .await;
        assert_eq!(s, 200, "{b}");
        let items = b["items"].as_array().unwrap();
        if items.is_empty() {
            break;
        }
        for item in items {
            assert_eq!(item["readinessStatus"], "READY");
            assert_eq!(item.as_object().unwrap().len(), 8);
            assert!(item.get("submissionId").is_none());
            let id: Uuid = item["workflowInstanceId"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            assert!(!waiting.contains(&id));
            found |= id == ready;
            assert!(seen.insert(item["dispatchIntentId"].clone().to_string()));
        }
        let last = items.last().unwrap();
        path=format!("/internal/v1/dispatch-intents/ready?limit=2&afterNextEligibleAt={}&afterDispatchIntentId={}",last["nextEligibleAt"].as_str().unwrap().replace('+',"%2B"),last["dispatchIntentId"].as_str().unwrap());
        assert!(seen.len() < 200);
    }
    assert!(found, "ready item after blocked prefix must be reachable");
    assert_eq!(
        http(
            router.clone(),
            f.owner,
            &mock,
            "GET",
            "/internal/v1/dispatch-intents/ready",
            None,
            json!({})
        )
        .await
        .0,
        403
    );
    assert_ne!(
        http(
            router.clone(),
            f.coordinator,
            &mock,
            "GET",
            "/internal/v1/dispatch-intents/ready?afterDispatchIntentId=bad",
            None,
            json!({})
        )
        .await
        .0,
        200
    );
    let raw = http(
        router,
        f.coordinator,
        &mock,
        "GET",
        "/internal/v1/dispatch-intents?limit=100",
        None,
        json!({}),
    )
    .await;
    assert_eq!(raw.0, 200);
    assert!(raw.1["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v.get("readinessStatus").is_none()));
    let source_visit: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let sf = Fixture {
        instance: source,
        visit: source_visit,
        transition: t,
        ..f
    };
    let mut finish = transition_command(&sf, 1);
    finish.submission_payload = Some(json!({"actual":"supplier artifact"}));
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &p,
        AdmissionGate::disabled(),
        finish,
    )
    .await
    .unwrap();
    let rebuilt = http_support::app(&p, &mock).await;
    let (s, b) = http(
        rebuilt,
        f.coordinator,
        &mock,
        "GET",
        "/internal/v1/dispatch-intents/ready?limit=100",
        None,
        json!({}),
    )
    .await;
    assert_eq!(s, 200);
    let returned: Vec<Uuid> = b["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["workflowInstanceId"].as_str().unwrap().parse().unwrap())
        .collect();
    assert!(
        waiting.iter().all(|id| returned.contains(id)),
        "restart/poll must derive prior WAITING activations as READY"
    );
}
