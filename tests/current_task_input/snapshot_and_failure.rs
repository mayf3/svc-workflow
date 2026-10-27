use super::*;

#[tokio::test]
async fn one_http_snapshot_does_not_mix_a_concurrent_context_revision() {
    let h = Harness::new(json!({"phase":"before"})).await;
    let initial = h.input().await;
    let mut blocker = h.pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE workflow_transition_definitions IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let client = h.client.clone();
    let url = format!(
        "{}/internal/v1/workflow-instances/{}/current-task-input",
        h.root, h.instance
    );
    let token = h.token(h.seed.owner);
    let actor = h.seed.owner;
    let read = tokio::spawn(async move {
        let r = client.get(&url).bearer_auth(token).send().await.unwrap();
        let status = r.status().as_u16();
        let bytes = r.bytes().await.unwrap();
        record_http_response(&url, actor, status, &bytes);
        assert_eq!(status, 200);
        serde_json::from_slice::<Value>(&bytes).unwrap()
    });
    let mut waiting = false;
    for _ in 0..200 {
        waiting=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%workflow_transition_definitions%')").fetch_one(&h.pool).await.unwrap();
        if waiting {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        waiting,
        "must observe real blocked read after snapshot begins"
    );
    svc_workflow::application::workflow_instance::revise::revise_workflow_context(
        &h.pool,
        AdmissionGate::disabled(),
        ReviseWorkflowContextCommand {
            principal_id: PrincipalId::from_uuid(h.seed.creator),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".into(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(h.instance),
            expected_workflow_state_version: 1,
            context_payload: json!({"phase":"after"}),
        },
    )
    .await
    .unwrap();
    blocker.rollback().await.unwrap();
    let before = read.await.unwrap();
    assert_eq!(before["binding"], initial["binding"]);
    let detail: Value =
        serde_json::from_str(before["detail_body"]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        detail["current_context"]["payload"],
        json!({"phase":"before"})
    );
    let after = h.input().await;
    assert_eq!(after["binding"]["workflow_state_version"], 2);
    assert_ne!(
        before["binding"]["context_revision_id"],
        after["binding"]["context_revision_id"]
    );
    assert_eq!(h.get(h.seed.owner, &binding_query(&before)).await.0, 409);
}

#[tokio::test]
async fn interrupted_database_read_is_unavailable_and_recovers_without_business_write() {
    let h = Harness::new(json!({})).await;
    let before = business_counts(&h.pool).await;
    let mut blocker = h.pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE workflow_transition_definitions IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let client = h.client.clone();
    let url = format!(
        "{}/internal/v1/workflow-instances/{}/current-task-input",
        h.root, h.instance
    );
    let token = h.token(h.seed.owner);
    let actor = h.seed.owner;
    let read = tokio::spawn(async move {
        let r = client.get(&url).bearer_auth(token).send().await.unwrap();
        let status = r.status().as_u16();
        let bytes = r.bytes().await.unwrap();
        record_http_response(&url, actor, status, &bytes);
        (status, serde_json::from_slice::<Value>(&bytes).unwrap())
    });
    let mut pid = None;
    for _ in 0..200 {
        pid=sqlx::query_scalar::<_,i32>("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%workflow_transition_definitions%' LIMIT 1").fetch_optional(&h.pool).await.unwrap();
        if pid.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let pid = pid.expect("only this isolated database's blocked CTI read can be terminated");
    let terminated: bool = sqlx::query_scalar("SELECT pg_terminate_backend($1)")
        .bind(pid)
        .fetch_one(&h.pool)
        .await
        .unwrap();
    assert!(terminated);
    blocker.rollback().await.unwrap();
    let (status, b) = read.await.unwrap();
    assert_eq!(status, 503);
    assert_eq!(b["error"]["code"], "service_unavailable");
    assert!(b.get("binding").is_none());
    assert_eq!(before, business_counts(&h.pool).await);
    h.input().await;
}
