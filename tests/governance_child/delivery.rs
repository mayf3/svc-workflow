use super::*;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
#[derive(Clone, Default)]
struct Capture {
    calls: Arc<Mutex<Vec<Value>>>,
    fail_first: bool,
}
async fn wake(State(c): State<Capture>, Json(p): Json<Value>) -> StatusCode {
    let mut v = c.calls.lock().unwrap();
    v.push(p);
    if c.fail_first && v.len() == 1 {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::NO_CONTENT
    }
}
async fn token() -> Json<Value> {
    Json(
        json!({"access_token":"isolated-test-token","token_type":"Bearer","expires_in":300,"scope":"workflow.execute"}),
    )
}
async fn drain(
    pool: &PgPool,
    capture: Capture,
) -> (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>) {
    // Test setup acknowledges unrelated initial/forum rows; all review rows remain real.
    sqlx::query("UPDATE workflow_outbox SET delivered_at=now() WHERE payload->>'phase' IS DISTINCT FROM 'GOVERNANCE_CHILD_REVIEW'").execute(pool).await.unwrap();
    sqlx::query("UPDATE workflow_forum_bindings SET binding_state='BOUND',forum_thread_id=workflow_instance_id::text").execute(pool).await.unwrap();
    let router = Router::new()
        .route("/oauth/token", post(token))
        .route("/wake", post(wake))
        .with_state(capture);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let config = svc_workflow::forum_sync::ForumSyncConfig {
        auth_base_url: url.clone(),
        client_id: "unused".into(),
        client_secret: "unused".into(),
        forum_origin: url.clone(),
        audience: "svc-forum".into(),
        poll_interval_ms: 10,
        kick: None,
        owner_assistance_wake: Ok(Some(svc_workflow::forum_sync::OwnerAssistanceWakeConfig {
            url: format!("{url}/wake"),
            auth_base_url: url,
            client_id: "isolated".into(),
            client_secret: "synthetic".into(),
            audience: "svc-workflow".into(),
        })),
    };
    let pool = pool.clone();
    let worker = tokio::spawn(svc_workflow::forum_sync::run_loop(pool, config));
    (worker, server)
}
async fn wait_calls(c: &Capture, n: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if c.calls.lock().unwrap().len() >= n {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "timed out waiting for {n} calls; received {:?}",
            c.calls.lock().unwrap()
        )
    });
}
async fn bound_terminal(pool: &PgPool) -> (Fixture, Fixture, Uuid, Value) {
    let f = setup_with_schema(pool, schema()).await;
    let c = child(pool, &f).await;
    let case = request_case(pool, &f).await;
    let b = bind_governance_child(pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
        .await
        .unwrap();
    finish(pool, &c, &b, "ACCEPTED").await;
    (f, c, case.assistance_case_id, b)
}
#[sqlx::test]
async fn real_drain_retries_same_wake_then_delivers_to_each_replacement(pool: PgPool) {
    let (f, _, case, _) = bound_terminal(&pool).await;
    replace(&pool, &f, f.replacement_owner, "before-delivery").await;
    let capture = Capture {
        fail_first: true,
        ..Default::default()
    };
    let (worker, server) = drain(&pool, capture.clone()).await;
    wait_calls(&capture, 1).await;
    sqlx::query("UPDATE workflow_outbox SET next_attempt_at=now() WHERE delivered_at IS NULL")
        .execute(&pool)
        .await
        .unwrap();
    wait_calls(&capture, 2).await;
    {
        let calls = capture.calls.lock().unwrap();
        assert_eq!(calls[0], calls[1]);
        assert_eq!(calls[0]["ownerPrincipalId"], json!(f.replacement_owner));
    }
    // Wait for the real 2xx acknowledgement before changing Owner again.
    tokio::time::timeout(Duration::from_secs(5),async {loop {let n:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_outbox WHERE delivered_at IS NULL AND payload->>'phase'='GOVERNANCE_CHILD_REVIEW'").fetch_one(&pool).await.unwrap();if n==0 {break}tokio::time::sleep(Duration::from_millis(10)).await;}}).await.unwrap();
    replace(&pool, &f, f.owner, "after-delivery").await;
    wait_calls(&capture, 3).await;
    replace(&pool, &f, f.replacement_owner, "switch-back").await;
    wait_calls(&capture, 4).await;
    {
        let calls = capture.calls.lock().unwrap();
        assert_eq!(calls[2]["ownerPrincipalId"], json!(f.owner));
        assert_eq!(calls[3]["ownerPrincipalId"], json!(f.replacement_owner));
        assert_ne!(calls[1]["reviewWakeId"], calls[3]["reviewWakeId"]);
    }
    let d = serde_json::to_value(
        get_assistance_case(&pool, f.replacement_owner, case)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        d["governanceBinding"]["currentReview"]["reviewStatus"],
        "OUTSTANDING"
    );
    if let Ok(output) = std::env::var("GCR_REVIEW_PAYLOAD_OUTPUT") {
        let captured = capture.calls.lock().unwrap()[0].clone();
        // Exact JSON observed by the local HTTP receiver, produced by the real
        // bind -> terminal -> outbox -> authenticated drain path above.
        std::fs::write(&output, serde_json::to_vec_pretty(&captured).unwrap()).unwrap();
        std::fs::write(format!("{output}.receipt.json"),serde_json::to_vec_pretty(&json!({"assistanceCaseId":case,"governanceBinding":d["governanceBinding"],"governanceTerminalProof":d["governanceTerminalProof"],"capturedHttpPayload":captured,"validationEnvironment":"task-owned PostgreSQL and local HTTP receiver; not production"})).unwrap()).unwrap();
    }
    worker.abort();
    server.abort();
}
#[sqlx::test]
async fn drain_suppresses_stale_source_without_http_delivery(pool: PgPool) {
    let (f, _, _, _) = bound_terminal(&pool).await;
    // Deliberate stale projection fixture: source is no longer executable.
    sqlx::query("UPDATE workflow_instances SET cancelled=TRUE WHERE workflow_instance_id=$1")
        .bind(f.instance)
        .execute(&pool)
        .await
        .unwrap();
    let capture = Capture::default();
    let (worker, server) = drain(&pool, capture.clone()).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let r: Option<String> =
                sqlx::query_scalar("SELECT retired_reason FROM workflow_governance_child_reviews")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            if r.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(capture.calls.lock().unwrap().is_empty());
    let reason: String = sqlx::query_scalar(
        "SELECT last_error FROM workflow_outbox WHERE payload->>'phase'='GOVERNANCE_CHILD_REVIEW'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(reason.contains("STALE_SOURCE_OR_CASE"));
    worker.abort();
    server.abort();
}
#[sqlx::test]
async fn closed_review_dto_rejects_unknown_members_before_transport(pool: PgPool) {
    bound_terminal(&pool).await;
    sqlx::query("UPDATE workflow_outbox SET payload=payload || '{\"reason\":\"ASSISTANCE_REQUESTED\"}'::jsonb WHERE payload->>'phase'='GOVERNANCE_CHILD_REVIEW'").execute(&pool).await.unwrap();
    let capture = Capture::default();
    let (worker, server) = drain(&pool, capture.clone()).await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        capture.calls.lock().unwrap().is_empty(),
        "unknown phase members must fail before HTTP"
    );
    let n:i32=sqlx::query_scalar("SELECT attempt_count FROM workflow_outbox WHERE payload->>'phase'='GOVERNANCE_CHILD_REVIEW'").fetch_one(&pool).await.unwrap();
    assert!(n > 0);
    worker.abort();
    server.abort();
}
