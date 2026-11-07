//! Integration tests for the outbox reconciler's three-channel independence
//! (EXECUTION_KICK / FORUM_EVENT / OWNER_ASSISTANCE_WAKE).
//!
//! Coverage map (P0-B task §7):
//!   - kick-only config starts and delivers without any forum dependency
//!   - forum disabled → no forum API calls, bindings and FORUM_EVENT rows
//!     untouched
//!   - owner wake disabled → OWNER_ASSISTANCE_WAKE rows untouched
//!   - all three channels configured → each delivers independently
//!   - kick 401/403 → auth failure class, retried, never delivered
//!   - kick 409 → no contract semantics, retryable
//!   - kick 5xx → bounded retry, dead-letter at the cap, documented requeue
//!     entry recovers the row
//!   - kick 400 → permanent, dead-letters immediately with payload retained
//!   - concurrent workers → FOR UPDATE SKIP LOCKED claim, no double delivery
//!   - crashed worker (lock released via connection death) → row recovers
//!   - transport UNKNOWN → retry scheduled, not blindly marked delivered
//!   - forum API down → does not block the kick channel in the same pass
//!   - kick disabled → kick rows untouched
//!   - restart → no loss of pending rows
//!   - one-connection pool → forum AND kick both delivered (claim transaction
//!     is HTTP-only; no nested pool access), also under concurrent workers

#![allow(clippy::needless_borrow)]
#![allow(unused_imports, unused_variables)]

#[path = "common/mod.rs"]
mod common;

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use sqlx::PgPool;
use uuid::Uuid;

use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use svc_workflow::forum_sync::{
    ExecutionKickConfig, ForumClient, ForumEventConfig, OutboxReconcilerConfig,
    OwnerAssistanceWakeConfig,
};

// ============================================================================
// Isolation: the reconciler drains TABLE-WIDE per kind (production
// semantics), so every test runs against its OWN freshly created database
// (per-binary TEST_DATABASE_URL server, unique db per test, migrations
// applied). No cross-binary or cross-test interference is possible; no
// quarantine or suite lock is needed.
// ============================================================================

/// Create a unique database on the TEST_DATABASE_URL server, apply all
/// migrations, and return its pool. Mirrors tests/prerequisites/support.rs.
async fn isolated_db() -> PgPool {
    isolated_db_with_url().await.0
}

/// Same as `isolated_db`, additionally returning the database URL so a test
/// can derive constrained pools (e.g. `max_connections(1)`) for regression
/// coverage of pool-starvation scenarios.
async fn isolated_db_with_url() -> (PgPool, String) {
    let url = common::test_database_url();
    let base = url
        .rsplit_once('/')
        .expect("TEST_DATABASE_URL must include a database segment")
        .0
        .to_string();
    let name = format!("svc_outbox_p0b_{}", Uuid::new_v4().simple());
    let admin = sqlx::PgPool::connect(&format!("{base}/postgres"))
        .await
        .expect("connect admin database of the isolated test server");
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .expect("create per-test database");
    admin.close().await;

    let database_url = format!("{base}/{name}");
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("connect per-test database");
    let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
        .await
        .expect("load migrations");
    migrator
        .run(&pool)
        .await
        .expect("apply migrations on per-test database");
    (pool, database_url)
}

// ============================================================================
// Harness (mirrors tests/36)
// ============================================================================

async fn seed_agent(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'Test Agent', NULL, TRUE)",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("insert agent");
    id
}

async fn seed_domain_member(pool: &PgPool, domain_id: Uuid, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1, $2, $3, 'AGENT', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("seed domain member");
}

async fn seed_v1_definition(
    pool: &PgPool,
    domain_id: Uuid,
    agent_id: Uuid,
) -> (Uuid, Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("p0b-test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'P0B Test Def')",
    )
    .bind(def_id).bind(domain_id).bind(&def_key)
    .execute(pool).await.expect("insert def");

    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, semantic_model_version) VALUES ($1, $2, 1, 'DRAFT', 3)",
    )
    .bind(ver_id).bind(def_id)
    .execute(pool).await.expect("insert version");

    let start_id = Uuid::new_v4();
    let work_id = Uuid::new_v4();
    let done_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'start', 'Start', 0, 'TASK', 'WORKFLOW_CREATOR')",
    ).bind(start_id).bind(ver_id).execute(pool).await.expect("insert start");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) VALUES ($1, $2, 'work', 'Work', 1, 'TASK', 'FIXED_PRINCIPAL', $3)",
    ).bind(work_id).bind(ver_id).bind(agent_id).execute(pool).await.expect("insert work");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)",
    ).bind(done_id).bind(ver_id).execute(pool).await.expect("insert done");

    let t1 = Uuid::new_v4();
    let t3 = Uuid::new_v4();
    for (tid, key, src, dst, effect) in [
        (t1, "advance-1", start_id, work_id, "ADVANCE"),
        (t3, "return", work_id, start_id, "RETURN"),
    ] {
        sqlx::query(
            "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, $3, $3, $4, $5, $6::transition_effect)",
        )
        .bind(tid).bind(ver_id).bind(key).bind(src).bind(dst).bind(effect)
        .execute(pool).await.expect("insert transition");
    }
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t1).bind(start_id).execute(pool).await.expect("primary start");

    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(pool).await.expect("publish");

    (ver_id, start_id, work_id, t3)
}

/// An AGENT creator makes the entry visit a DISPATCH_INTENT activation, which
/// queues the canonical EXECUTION_KICK row in the create transaction.
async fn create_instance_with_kick(pool: &PgPool) -> (Uuid, Uuid) {
    let (_owner, domain_id) = common::seed_principal_and_domain(pool).await;
    let creator = seed_agent(pool).await;
    seed_domain_member(pool, domain_id, creator).await;
    let agent = seed_agent(pool).await;
    let (ver_id, _start, _work, _ret) = seed_v1_definition(pool, domain_id, agent).await;
    let result = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        pool,
        common::admitting_gate(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(creator),
            idempotency_key: format!("create-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(ver_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({}),
            context_payload: serde_json::json!({ "title": "p0b" }),
        },
    )
    .await
    .expect("create instance should succeed");
    (result.workflow_instance_id, result.current_node_visit_id)
}

/// Queue one extra outbox row for direct consumer testing.
async fn queue_row(
    pool: &PgPool,
    instance_id: Uuid,
    kind: &str,
    event_key: &str,
    payload: serde_json::Value,
) -> Uuid {
    let outbox_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_outbox
         (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(outbox_id)
    .bind(instance_id)
    .bind(kind)
    .bind(event_key)
    .bind(payload)
    .execute(pool)
    .await
    .expect("queue outbox row");
    outbox_id
}

struct RowState {
    delivered_at: Option<Timestamp>,
    dead_lettered_at: Option<Timestamp>,
    attempt_count: i32,
    last_error: Option<String>,
    next_attempt_future: bool,
}

type Timestamp = sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>;

async fn row_state(pool: &PgPool, outbox_id: Uuid) -> RowState {
    let (delivered_at, dead_lettered_at, attempt_count, last_error, next_attempt_at): (
        Option<Timestamp>,
        Option<Timestamp>,
        i32,
        Option<String>,
        Timestamp,
    ) = sqlx::query_as(
        "SELECT delivered_at, dead_lettered_at, attempt_count, last_error, next_attempt_at
           FROM workflow_outbox WHERE outbox_id = $1",
    )
    .bind(outbox_id)
    .fetch_one(pool)
    .await
    .expect("row state");
    RowState {
        delivered_at,
        dead_lettered_at,
        attempt_count,
        last_error,
        next_attempt_future: next_attempt_at > sqlx::types::chrono::Utc::now(),
    }
}

async fn force_due(pool: &PgPool, outbox_id: Uuid) {
    sqlx::query("UPDATE workflow_outbox SET next_attempt_at = now() WHERE outbox_id = $1")
        .bind(outbox_id)
        .execute(pool)
        .await
        .expect("force due");
}

// #1359: exercise the actual background loop with twenty real 10s Forum
// timeouts. On the serial baseline this takes about 200s; on the isolated
// loop the healthy receivers settle immediately and release the fake's wait.
// The endpoint timings and durable-row timings are both printed for RED/GREEN.
#[tokio::test]
async fn forum_timeouts_do_not_delay_healthy_background_channels() {
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;

    let pool = isolated_db().await;
    let mut first_instance = None;
    for _ in 0..20 {
        let (instance, _) = create_instance_with_kick(&pool).await;
        first_instance.get_or_insert(instance);
    }
    let instance = first_instance.unwrap();
    // Preserve fixture history but isolate the twenty binding timeouts from
    // the separate event-post phase; these rows are simply not due yet.
    sqlx::query("UPDATE workflow_outbox SET next_attempt_at = now() + interval '1 hour' WHERE outbox_kind = 'FORUM_EVENT'")
        .execute(&pool).await.unwrap();
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;
    let wake_row = queue_row(
        &pool, instance, "OWNER_ASSISTANCE_WAKE", "owner-assistance:1359-timing",
        serde_json::json!({
            "workflowInstanceId": instance, "nodeVisitId": Uuid::new_v4(),
            "assistanceCaseId": Uuid::new_v4(), "ownerPrincipalId": Uuid::new_v4(),
            "reason": "RETURN_POLICY_EXHAUSTED"
        }),
    ).await;

    let released = Arc::new(AtomicBool::new(false));
    let release = Arc::new(tokio::sync::Notify::new());
    let lookups = Arc::new(AtomicUsize::new(0));
    let lookup_released = released.clone();
    let lookup_release = release.clone();
    let lookup_calls = lookups.clone();
    let forum_app = Router::new()
        .route("/oauth/token", post(|| async {
            Json(serde_json::json!({"access_token": "forum-token", "expires_in": 600}))
        }))
        .route("/api/threads", get(move || {
            let released = lookup_released.clone();
            let release = lookup_release.clone();
            let calls = lookup_calls.clone();
            async move {
                // Register before checking the flag so teardown cannot miss
                // a waiter between checking and waiting.
                let notified = release.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                calls.fetch_add(1, Ordering::SeqCst);
                if !released.load(Ordering::SeqCst) {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(11)) => {},
                        _ = notified => {},
                    }
                }
                StatusCode::SERVICE_UNAVAILABLE
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let forum_base = format!("http://{}", listener.local_addr().unwrap());
    let forum_server = tokio::spawn(async move { axum::serve(listener, forum_app).await.unwrap() });

    let started = Instant::now();
    let kick_received = Arc::new(Mutex::new(None));
    let wake_received = Arc::new(Mutex::new(None));
    let kick_record = kick_received.clone();
    let wake_record = wake_received.clone();
    let healthy_app = Router::new()
        .route("/oauth/token", post(|| async {
            Json(serde_json::json!({"access_token": "wake-token", "token_type": "Bearer",
                "expires_in": 600, "scope": "workflow.execute"}))
        }))
        .route("/workflow-execution/kicks", post(move || {
            let record = kick_record.clone();
            async move {
                record.lock().unwrap().get_or_insert(started.elapsed());
                StatusCode::OK
            }
        }))
        .route("/workflow-execution/owner-assistance-wakes", post(move || {
            let record = wake_record.clone();
            async move {
                record.lock().unwrap().get_or_insert(started.elapsed());
                StatusCode::OK
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let healthy_base = format!("http://{}", listener.local_addr().unwrap());
    let healthy_server = tokio::spawn(async move { axum::serve(listener, healthy_app).await.unwrap() });
    let mut config = forum_and_kick_config(&forum_base, &healthy_base);
    // Only one pass: the immediate first tick, then ten minutes. This keeps
    // test teardown from racing a second baseline pass.
    config.poll_interval_ms = 600_000;
    config.owner_assistance_wake = Ok(Some(OwnerAssistanceWakeConfig {
        url: format!("{healthy_base}/workflow-execution/owner-assistance-wakes"),
        auth_base_url: healthy_base, client_id: "wake-client".into(),
        client_secret: "wake-secret".into(), audience: "svc-workflow".into(),
    }));
    let background_pool = pool.clone();
    let background = tokio::spawn(async move {
        svc_workflow::forum_sync::run_loop(background_pool, config).await;
    });

    let mut kick_settled = None;
    let mut wake_settled = None;
    let observation = tokio::time::timeout(Duration::from_secs(230), async {
        loop {
            if kick_settled.is_none() && row_state(&pool, kick_row).await.delivered_at.is_some() {
                kick_settled = Some(started.elapsed());
            }
            if wake_settled.is_none() && row_state(&pool, wake_row).await.delivered_at.is_some() {
                wake_settled = Some(started.elapsed());
            }
            if kick_settled.is_some() && wake_settled.is_some() { break; }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await;
    let lookup_count_at_settlement = lookups.load(Ordering::SeqCst);
    let kick_at = *kick_received.lock().unwrap();
    let wake_at = *wake_received.lock().unwrap();
    println!("H1359_TIMING pending_bindings=20 forum_http_timeout_ms=10000 lookup_calls={lookup_count_at_settlement} kick_received_ms={:?} wake_received_ms={:?} kick_settled_ms={:?} wake_settled_ms={:?}",
        kick_at.map(|d| d.as_millis()), wake_at.map(|d| d.as_millis()),
        kick_settled.map(|d| d.as_millis()), wake_settled.map(|d| d.as_millis()));

    // Release only the test receiver. No product timeout, claim or retry
    // semantics are altered to make the timing assertion pass.
    released.store(true, Ordering::SeqCst);
    release.notify_waiters();
    let cleanup = tokio::time::timeout(Duration::from_secs(5), async {
        while lookups.load(Ordering::SeqCst) < 20 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }).await;
    background.abort();
    let _ = background.await;
    forum_server.abort();
    healthy_server.abort();
    pool.close().await;
    assert!(cleanup.is_ok(), "fixture binding preparation must finish before teardown");
    assert!(observation.is_ok(), "healthy channels did not settle within the observation window");
    assert!(kick_settled.unwrap() < Duration::from_secs(2)
        && wake_settled.unwrap() < Duration::from_secs(2),
        "healthy channels waited for Forum timeouts: kick={kick_settled:?}, wake={wake_settled:?}");
}

// ── fake HTTP servers ───────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct GatewayCapture {
    calls: Arc<AtomicUsize>,
    per_key: Arc<Mutex<HashMap<String, usize>>>,
    bodies: Arc<Mutex<Vec<serde_json::Value>>>,
    bearers: Arc<Mutex<Vec<String>>>,
    idempotency_keys: Arc<Mutex<Vec<String>>>,
    delay_ms: u64,
    status_by_attempt: Arc<Mutex<Vec<u16>>>,
    /// When set, every request fails with this status (takes precedence over
    /// the per-attempt script).
    always_status: Arc<Mutex<Option<u16>>>,
}

impl GatewayCapture {
    fn new(delay_ms: u64) -> Self {
        Self {
            delay_ms,
            ..Default::default()
        }
    }
}

async fn gateway_handler(
    State(capture): State<GatewayCapture>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    capture.calls.fetch_add(1, Ordering::SeqCst);
    if capture.delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(capture.delay_ms)).await;
    }
    let event_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    *capture
        .per_key
        .lock()
        .unwrap()
        .entry(event_key.clone())
        .or_insert(0) += 1;
    capture.idempotency_keys.lock().unwrap().push(event_key);
    capture.bodies.lock().unwrap().push(body);
    capture.bearers.lock().unwrap().push(
        headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string(),
    );
    // Scripted per-attempt statuses; beyond the script the gateway answers 200.
    if let Some(code) = *capture.always_status.lock().unwrap() {
        return StatusCode::from_u16(code).unwrap();
    }
    let scripted = {
        let mut statuses = capture.status_by_attempt.lock().unwrap();
        if statuses.is_empty() {
            None
        } else {
            Some(statuses.remove(0))
        }
    };
    scripted
        .map(|code| StatusCode::from_u16(code).unwrap())
        .unwrap_or(StatusCode::OK)
}

/// Spawn a fake Agent Core kicks endpoint. Returns the base URL and its join
/// handle (abort it at the end of the test).
async fn spawn_kick_gateway(capture: GatewayCapture) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route(
            "/workflow-execution/kicks",
            post(gateway_handler).with_state(capture),
        )
        .fallback(|| async { StatusCode::NOT_FOUND });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), server)
}

/// Kick-only reconciler config (no forum, no wake).
fn kick_only_config(base_url: &str) -> OutboxReconcilerConfig {
    OutboxReconcilerConfig {
        forum: None,
        poll_interval_ms: 250,
        kick: Some(ExecutionKickConfig {
            url: format!("{base_url}/workflow-execution/kicks"),
            token: "kick-static-token".to_string(),
        }),
        owner_assistance_wake: Ok(None),
    }
}

// ============================================================================
// Channel isolation
// ============================================================================

#[tokio::test]
async fn kick_only_delivers_without_any_forum_dependency() {
    let pool = isolated_db().await;
    let (instance, visit) = create_instance_with_kick(&pool).await;
    let kicks = outbox_event_keys(&pool, instance, "EXECUTION_KICK").await;
    assert_eq!(kicks.len(), 1, "creation queues exactly one kick row");

    let capture = GatewayCapture::default();
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let forum = ForumClient::new(kick_only_config(&base_url));
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 1, "the kick row is delivered");
    assert_eq!(capture.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        capture.bearers.lock().unwrap().last().map(String::as_str),
        Some("Bearer kick-static-token")
    );
    let body = capture.bodies.lock().unwrap().last().unwrap().clone();
    assert_eq!(body["nodeVisitId"], serde_json::json!(visit));
    assert_eq!(body["workflowInstanceId"], serde_json::json!(instance));
    let kick_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT outbox_id FROM workflow_outbox WHERE workflow_instance_id = $1 AND outbox_kind = 'EXECUTION_KICK'",
    )
    .bind(instance)
    .fetch_one(&pool)
    .await
    .unwrap();
    let state = row_state(&pool, kick_id).await;
    assert!(state.delivered_at.is_some());

    // Forum facts are untouched: no forum channel configured means no forum
    // API call, no binding update, and no attempt on the FORUM_EVENT row.
    let binding: (String,) = sqlx::query_as(
        "SELECT binding_state::text FROM workflow_forum_bindings WHERE workflow_instance_id = $1",
    )
    .bind(instance)
    .fetch_one(&pool)
    .await
    .expect("binding row exists");
    assert_eq!(binding.0, "PENDING", "forum binding untouched");
    let forum_rows = outbox_event_keys(&pool, instance, "FORUM_EVENT").await;
    assert!(!forum_rows.is_empty());
    for (key, _) in &forum_rows {
        let id: Uuid = sqlx::query_scalar(
            "SELECT outbox_id FROM workflow_outbox WHERE workflow_instance_id = $1 AND outbox_kind = 'FORUM_EVENT' AND event_key = $2",
        )
        .bind(instance)
        .bind(key)
        .fetch_one(&pool)
        .await
        .unwrap();
        let state = row_state(&pool, id).await;
        assert_eq!(state.attempt_count, 0, "forum row never attempted");
        assert!(state.delivered_at.is_none());
    }
    server.abort();
}

#[tokio::test]
async fn forum_api_outage_does_not_block_kick_channel() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let forum_row = outbox_first_row(&pool, instance, "FORUM_EVENT").await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;

    // Forum channel points at a closed port (API down); kick channel is
    // healthy. Both enabled in one pass.
    let capture = GatewayCapture::default();
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let config = OutboxReconcilerConfig {
        forum: Some(ForumEventConfig {
            auth_base_url: "http://127.0.0.1:1".to_string(),
            client_id: "forum-client".to_string(),
            client_secret: "forum-secret".to_string(),
            forum_origin: "http://127.0.0.1:1".to_string(),
            audience: "svc-forum".to_string(),
        }),
        poll_interval_ms: 250,
        kick: Some(ExecutionKickConfig {
            url: format!("{base_url}/workflow-execution/kicks"),
            token: "kick-static-token".to_string(),
        }),
        owner_assistance_wake: Ok(None),
    };
    let forum = ForumClient::new(config);
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 1, "kick delivered despite forum outage");
    assert_eq!(capture.calls.load(Ordering::SeqCst), 1);

    let kick_state = row_state(&pool, kick_row).await;
    assert!(kick_state.delivered_at.is_some());
    let forum_state = row_state(&pool, forum_row).await;
    assert!(forum_state.delivered_at.is_none());
    assert!(forum_state.attempt_count >= 1, "forum row backed off");
    server.abort();
}

#[tokio::test]
async fn kick_disabled_leaves_kick_rows_untouched() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;

    // Only the forum channel is enabled (dormant credentials pointing nowhere;
    // bindings are PENDING so the ensure pass warns and moves on).
    let config = OutboxReconcilerConfig {
        forum: Some(ForumEventConfig {
            auth_base_url: "http://127.0.0.1:1".to_string(),
            client_id: "forum-client".to_string(),
            client_secret: "forum-secret".to_string(),
            forum_origin: "http://127.0.0.1:1".to_string(),
            audience: "svc-forum".to_string(),
        }),
        poll_interval_ms: 250,
        kick: None,
        owner_assistance_wake: Ok(None),
    };
    let forum = ForumClient::new(config);
    svc_workflow::forum_sync::run_once(&pool, &forum).await;

    let state = row_state(&pool, kick_row).await;
    assert_eq!(state.attempt_count, 0, "kick row never attempted");
    assert!(state.delivered_at.is_none());
    assert!(state.dead_lettered_at.is_none());
}

#[tokio::test]
async fn owner_wake_disabled_leaves_wake_rows_untouched() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let wake_row = queue_row(
        &pool,
        instance,
        "OWNER_ASSISTANCE_WAKE",
        &format!("owner-assistance:{}", Uuid::new_v4()),
        serde_json::json!({
            "workflowInstanceId": instance,
            "nodeVisitId": Uuid::new_v4(),
            "assistanceCaseId": Uuid::new_v4(),
            "ownerPrincipalId": Uuid::new_v4(),
            "reason": "RETURN_POLICY_EXHAUSTED",
        }),
    )
    .await;

    let capture = GatewayCapture::default();
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let forum = ForumClient::new(kick_only_config(&base_url));
    svc_workflow::forum_sync::run_once(&pool, &forum).await;

    let state = row_state(&pool, wake_row).await;
    assert_eq!(state.attempt_count, 0, "wake row never attempted");
    assert!(state.delivered_at.is_none());
    server.abort();
}

// ============================================================================
// Kick delivery reliability
// ============================================================================

#[tokio::test]
async fn kick_5xx_retries_with_backoff_then_delivers() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;

    let capture = GatewayCapture::default();
    capture.status_by_attempt.lock().unwrap().push(502);
    capture.status_by_attempt.lock().unwrap().push(200);
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let forum = ForumClient::new(kick_only_config(&base_url));

    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 0, "502 is retryable, not delivered");
    let state = row_state(&pool, kick_row).await;
    assert!(state.delivered_at.is_none());
    assert_eq!(state.attempt_count, 1);
    assert!(state.next_attempt_future, "backoff scheduled");
    assert!(state.last_error.unwrap_or_default().contains("5xx"));

    force_due(&pool, kick_row).await;
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 1);
    let state = row_state(&pool, kick_row).await;
    assert!(state.delivered_at.is_some());
    assert_eq!(capture.calls.load(Ordering::SeqCst), 2);
    server.abort();
}

#[tokio::test]
async fn kick_401_and_403_are_auth_failures_never_delivered() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;

    // 401 on the first attempt, then 403: both are auth-class.
    let capture = GatewayCapture::default();
    capture.status_by_attempt.lock().unwrap().push(401);
    capture.status_by_attempt.lock().unwrap().push(403);
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let forum = ForumClient::new(kick_only_config(&base_url));

    for attempt in 1..=2 {
        force_due(&pool, kick_row).await;
        let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
        assert_eq!(delivered, 0, "auth failures are never marked delivered");
        let state = row_state(&pool, kick_row).await;
        assert_eq!(state.attempt_count, attempt);
        assert!(state.delivered_at.is_none());
        assert!(
            state
                .last_error
                .unwrap_or_default()
                .contains("authentication/authorization"),
            "the error names the auth failure class"
        );
    }
    // Still recoverable, not dead-lettered.
    assert!(row_state(&pool, kick_row).await.dead_lettered_at.is_none());
    server.abort();
}

#[tokio::test]
async fn kick_409_is_retryable_per_contract() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;

    let capture = GatewayCapture::default();
    capture.status_by_attempt.lock().unwrap().push(409);
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let forum = ForumClient::new(kick_only_config(&base_url));

    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 0);
    let state = row_state(&pool, kick_row).await;
    assert_eq!(state.attempt_count, 1);
    assert!(state.delivered_at.is_none());
    assert!(state.dead_lettered_at.is_none());
    server.abort();
}

#[tokio::test]
async fn kick_dead_letters_at_the_retry_cap_and_recovers_via_requeue() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;

    let capture = GatewayCapture::default();
    *capture.always_status.lock().unwrap() = Some(502);
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let forum = ForumClient::new(kick_only_config(&base_url));

    let payload_before: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM workflow_outbox WHERE outbox_id = $1")
            .bind(kick_row)
            .fetch_one(&pool)
            .await
            .unwrap();

    for attempt in 1..=12 {
        force_due(&pool, kick_row).await;
        let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
        assert_eq!(delivered, 0);
        let state = row_state(&pool, kick_row).await;
        assert_eq!(state.attempt_count, attempt);
        if attempt < 12 {
            assert!(state.dead_lettered_at.is_none());
        } else {
            assert!(state.dead_lettered_at.is_some(), "the cap dead-letters");
        }
    }

    // Dead-lettered rows are invisible to the drain (no more attempts).
    let calls_after_cap = capture.calls.load(Ordering::SeqCst);
    assert_eq!(calls_after_cap, 12);
    force_due(&pool, kick_row).await;
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 0);
    assert_eq!(capture.calls.load(Ordering::SeqCst), 12);

    // The documented recovery entry: repair the cause, then requeue.
    let (stored_payload, last_error): (serde_json::Value, Option<String>) =
        sqlx::query_as("SELECT payload, last_error FROM workflow_outbox WHERE outbox_id = $1")
            .bind(kick_row)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored_payload, payload_before, "original request retained");
    assert!(last_error.unwrap_or_default().contains("5xx"));
    *capture.always_status.lock().unwrap() = None; // the cause is repaired
    sqlx::query(
        "UPDATE workflow_outbox
            SET dead_lettered_at = NULL, attempt_count = 0, next_attempt_at = now()
          WHERE outbox_id = $1 AND delivered_at IS NULL",
    )
    .bind(kick_row)
    .execute(&pool)
    .await
    .unwrap();
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 1, "requeued row delivers after repair");
    let state = row_state(&pool, kick_row).await;
    assert!(state.delivered_at.is_some());
    server.abort();
}

#[tokio::test]
async fn kick_400_dead_letters_immediately_with_payload_retained() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;
    let payload_before: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM workflow_outbox WHERE outbox_id = $1")
            .bind(kick_row)
            .fetch_one(&pool)
            .await
            .unwrap();

    let capture = GatewayCapture::default();
    capture.status_by_attempt.lock().unwrap().push(400);
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let forum = ForumClient::new(kick_only_config(&base_url));

    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 0, "a permanent rejection is not a delivery");
    let state = row_state(&pool, kick_row).await;
    assert!(state.dead_lettered_at.is_some(), "400 dead-letters at once");
    assert!(state.delivered_at.is_none());
    let (stored_payload, last_error): (serde_json::Value, Option<String>) =
        sqlx::query_as("SELECT payload, last_error FROM workflow_outbox WHERE outbox_id = $1")
            .bind(kick_row)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored_payload, payload_before);
    assert!(last_error
        .unwrap_or_default()
        .contains("permanent rejection"));
    server.abort();
}

#[tokio::test]
async fn concurrent_workers_never_double_deliver() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let mut rows = vec![outbox_first_row(&pool, instance, "EXECUTION_KICK").await];
    for i in 0..5 {
        rows.push(
            queue_row(
                &pool,
                instance,
                "EXECUTION_KICK",
                &format!("kick:extra-{i}:{}", Uuid::new_v4()),
                serde_json::json!({
                    "dispatchIntentId": Uuid::new_v4(),
                    "nodeVisitId": Uuid::new_v4(),
                    "workflowInstanceId": instance,
                }),
            )
            .await,
        );
    }

    // A slow gateway widens the claim window so workers genuinely overlap.
    let capture = GatewayCapture::new(150);
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let forum = Arc::new(ForumClient::new(kick_only_config(&base_url)));

    let mut workers = Vec::new();
    for _ in 0..4 {
        let pool = pool.clone();
        let forum = forum.clone();
        workers.push(tokio::spawn(async move {
            svc_workflow::forum_sync::run_once(&pool, &forum).await
        }));
    }
    let mut total = 0usize;
    for worker in workers {
        total += worker.await.unwrap();
    }
    assert_eq!(total, 6, "every row delivered exactly once in total");
    assert_eq!(capture.calls.load(Ordering::SeqCst), 6);
    for (key, count) in capture.per_key.lock().unwrap().iter() {
        assert_eq!(*count, 1, "event {key} was delivered more than once");
    }
    for row in rows {
        let state = row_state(&pool, row).await;
        assert!(state.delivered_at.is_some());
    }
    server.abort();
}

#[tokio::test]
async fn claim_lock_released_by_crashed_worker_recovers() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;

    let capture = GatewayCapture::default();
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let forum = ForumClient::new(kick_only_config(&base_url));

    // A "crashed" worker holds the claim: its transaction is still open with
    // the row lock taken (the lock IS the lease — it dies with the
    // connection, not with a timeout).
    let mut crashed_tx = pool.begin().await.unwrap();
    let claimed = sqlx::query_as::<_, (Uuid,)>(
        "SELECT outbox_id FROM workflow_outbox WHERE outbox_id = $1 AND outbox_kind = 'EXECUTION_KICK'
          AND delivered_at IS NULL AND dead_lettered_at IS NULL AND next_attempt_at <= now()
         FOR UPDATE SKIP LOCKED",
    )
    .bind(kick_row)
    .fetch_optional(&mut *crashed_tx)
    .await
    .unwrap();
    assert!(claimed.is_some(), "the crashed worker held the claim");
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 0, "a locked row is not double-claimed");
    assert_eq!(capture.calls.load(Ordering::SeqCst), 0);

    // The process dies: its connection drops and the transaction aborts.
    crashed_tx.rollback().await.unwrap();
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 1, "the row recovers on the next pass");
    server.abort();
}

#[tokio::test]
async fn transport_unknown_result_schedules_retry_and_carries_stable_request_id() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;

    // Closed port: the delivery outcome is UNKNOWN from this side.
    let config = OutboxReconcilerConfig {
        forum: None,
        poll_interval_ms: 250,
        kick: Some(ExecutionKickConfig {
            url: "http://127.0.0.1:1/workflow-execution/kicks".to_string(),
            token: "kick-static-token".to_string(),
        }),
        owner_assistance_wake: Ok(None),
    };
    let forum = ForumClient::new(config);
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 0, "UNKNOWN is not a delivery");
    let state = row_state(&pool, kick_row).await;
    assert_eq!(state.attempt_count, 1);
    assert!(state.delivered_at.is_none());
    assert!(
        state.next_attempt_future,
        "retried with backoff, not replayed"
    );
    assert!(state.dead_lettered_at.is_none());

    // The recovery pass carries the stable Idempotency-Key (event key).
    let event_key: String =
        sqlx::query_scalar("SELECT event_key FROM workflow_outbox WHERE outbox_id = $1")
            .bind(kick_row)
            .fetch_one(&pool)
            .await
            .unwrap();
    let capture = GatewayCapture::default();
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let forum = ForumClient::new(kick_only_config(&base_url));
    force_due(&pool, kick_row).await;
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 1);
    assert_eq!(
        capture
            .idempotency_keys
            .lock()
            .unwrap()
            .last()
            .map(String::as_str),
        Some(event_key.as_str()),
        "stable request identity across retries"
    );
    server.abort();
}

#[tokio::test]
async fn restart_does_not_lose_pending_rows() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;

    // Pass 1 (worker "A"): the gateway is down; rows back off.
    let config = OutboxReconcilerConfig {
        forum: None,
        poll_interval_ms: 250,
        kick: Some(ExecutionKickConfig {
            url: "http://127.0.0.1:1/workflow-execution/kicks".to_string(),
            token: "kick-static-token".to_string(),
        }),
        owner_assistance_wake: Ok(None),
    };
    let worker_a = ForumClient::new(config);
    let delivered = svc_workflow::forum_sync::run_once(&pool, &worker_a).await;
    assert_eq!(delivered, 0);

    // Restart (worker "B": a fresh client, e.g. after a process restart).
    let capture = GatewayCapture::default();
    let (base_url, server) = spawn_kick_gateway(capture.clone()).await;
    let worker_b = ForumClient::new(kick_only_config(&base_url));
    force_due(&pool, kick_row).await;
    let delivered = svc_workflow::forum_sync::run_once(&pool, &worker_b).await;
    assert_eq!(delivered, 1, "no loss across the restart");
    let state = row_state(&pool, kick_row).await;
    assert!(state.delivered_at.is_some());
    server.abort();
}

// ============================================================================
// One-connection pool regression (Codex P2): the claim transaction must never
// wait on a second pool connection. Forum bindings/threads are resolved
// BEFORE the claim, so even `max_connections(1)` delivers forum and kick rows.
// ============================================================================

/// Minimal forum fake: token + thread lookup (always empty) + thread create
/// (unique id) + message post (counted per event key).
async fn spawn_forum_fake(
    message_keys: Arc<Mutex<HashMap<String, usize>>>,
) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route(
            "/oauth/token",
            post(|| async {
                Json(serde_json::json!({"access_token": "forum-token", "expires_in": 300}))
            }),
        )
        .route(
            "/api/threads",
            get(|| async { Json(serde_json::json!({"items": []})) }).post(|| async {
                Json(serde_json::json!({"thread": {"id": format!("thread-{}", Uuid::new_v4())}}))
            }),
        )
        .route(
            "/api/threads/{thread_id}/messages",
            post(move |Json(body): Json<serde_json::Value>| {
                let message_keys = message_keys.clone();
                async move {
                    let key = body["metadata"]["eventKey"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    *message_keys.lock().unwrap().entry(key).or_insert(0) += 1;
                    StatusCode::CREATED
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), server)
}

fn forum_and_kick_config(forum_base: &str, kick_base: &str) -> OutboxReconcilerConfig {
    OutboxReconcilerConfig {
        forum: Some(ForumEventConfig {
            auth_base_url: forum_base.to_string(),
            client_id: "forum-client".to_string(),
            client_secret: "forum-secret".to_string(),
            forum_origin: forum_base.to_string(),
            audience: "svc-forum".to_string(),
        }),
        poll_interval_ms: 250,
        kick: Some(ExecutionKickConfig {
            url: format!("{kick_base}/workflow-execution/kicks"),
            token: "kick-static-token".to_string(),
        }),
        owner_assistance_wake: Ok(None),
    }
}

#[tokio::test]
async fn forum_delivery_survives_a_one_connection_pool() {
    let (pool, database_url) = isolated_db_with_url().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let kick_row = outbox_first_row(&pool, instance, "EXECUTION_KICK").await;
    let forum_row = outbox_first_row(&pool, instance, "FORUM_EVENT").await;

    let message_keys = Arc::new(Mutex::new(HashMap::new()));
    let (forum_base, forum_server) = spawn_forum_fake(message_keys.clone()).await;
    let kick_capture = GatewayCapture::default();
    let (kick_base, kick_server) = spawn_kick_gateway(kick_capture.clone()).await;

    // The constrained deployment: one connection total. Before the
    // prepare-before-claim fix this pass deadlocked on itself (the claim
    // transaction held the only connection while ensure_thread/get_binding
    // waited for a second one) and the forum row timed out forever.
    let one_conn = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("connect one-connection pool");
    let forum = ForumClient::new(forum_and_kick_config(&forum_base, &kick_base));
    let delivered = svc_workflow::forum_sync::run_once(&one_conn, &forum).await;
    assert_eq!(delivered, 2, "forum event AND kick delivered on one connection");

    let kick_state = row_state(&pool, kick_row).await;
    assert!(kick_state.delivered_at.is_some());
    let forum_state = row_state(&pool, forum_row).await;
    assert!(forum_state.delivered_at.is_some());
    assert_eq!(forum_state.attempt_count, 0, "no hidden retry");
    assert_eq!(kick_capture.calls.load(Ordering::SeqCst), 1);
    let keys = message_keys.lock().unwrap();
    assert_eq!(keys.len(), 1, "exactly one forum message");
    assert_eq!(keys.values().sum::<usize>(), 1);

    forum_server.abort();
    kick_server.abort();
}

#[tokio::test]
async fn concurrent_workers_on_a_one_connection_pool_deliver_both_kinds() {
    let (pool, database_url) = isolated_db_with_url().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let mut kick_rows = vec![outbox_first_row(&pool, instance, "EXECUTION_KICK").await];
    for i in 0..2 {
        kick_rows.push(
            queue_row(
                &pool,
                instance,
                "EXECUTION_KICK",
                &format!("kick:pool1-{i}:{}", Uuid::new_v4()),
                serde_json::json!({
                    "dispatchIntentId": Uuid::new_v4(),
                    "nodeVisitId": Uuid::new_v4(),
                    "workflowInstanceId": instance,
                }),
            )
            .await,
        );
    }

    let message_keys = Arc::new(Mutex::new(HashMap::new()));
    let (forum_base, forum_server) = spawn_forum_fake(message_keys.clone()).await;
    let kick_capture = GatewayCapture::default();
    let (kick_base, kick_server) = spawn_kick_gateway(kick_capture.clone()).await;

    let one_conn = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("connect one-connection pool");
    let forum = Arc::new(ForumClient::new(forum_and_kick_config(
        &forum_base,
        &kick_base,
    )));

    // Two workers racing on ONE connection: no deadlock, no starvation, and
    // every event delivered exactly once.
    let mut workers = Vec::new();
    for _ in 0..2 {
        let pool = one_conn.clone();
        let forum = forum.clone();
        workers.push(tokio::spawn(async move {
            svc_workflow::forum_sync::run_once(&pool, &forum).await
        }));
    }
    let mut total = 0usize;
    for worker in workers {
        total += worker.await.unwrap();
    }
    assert_eq!(total, 4, "3 kicks + 1 forum event, each delivered once");
    assert_eq!(kick_capture.calls.load(Ordering::SeqCst), 3);
    for (key, count) in kick_capture.per_key.lock().unwrap().iter() {
        assert_eq!(*count, 1, "kick event {key} delivered more than once");
    }
    let forum_posts = {
        let keys = message_keys.lock().unwrap();
        (keys.len(), keys.values().sum::<usize>())
    };
    assert_eq!(forum_posts, (1, 1), "exactly one forum event, posted once");
    for row in kick_rows {
        let state = row_state(&pool, row).await;
        assert!(state.delivered_at.is_some());
    }
    forum_server.abort();
    kick_server.abort();
}

// ============================================================================
// All three channels together (forum + wake regression through the new loop)
// ============================================================================

#[tokio::test]
async fn all_three_channels_deliver_independently_in_one_pass() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    let wake_row = queue_row(
        &pool,
        instance,
        "OWNER_ASSISTANCE_WAKE",
        &format!("owner-assistance:{}", Uuid::new_v4()),
        serde_json::json!({
            "workflowInstanceId": instance,
            "nodeVisitId": Uuid::new_v4(),
            "assistanceCaseId": Uuid::new_v4(),
            "ownerPrincipalId": Uuid::new_v4(),
            "reason": "RETURN_POLICY_EXHAUSTED",
        }),
    )
    .await;

    let kick_capture = GatewayCapture::default();
    let (kick_base, kick_server) = spawn_kick_gateway(kick_capture.clone()).await;

    // Forum fake: oauth + thread lookup (none) + thread create + message post.
    let forum_calls = Arc::new(AtomicUsize::new(0));
    let forum_calls_for_get = forum_calls.clone();
    let forum_calls_for_post = forum_calls.clone();
    let forum_calls_for_message = forum_calls.clone();
    let forum_app = Router::new()
        .route(
            "/oauth/token",
            post(|| async {
                Json(serde_json::json!({"access_token": "forum-token", "expires_in": 300}))
            }),
        )
        .route(
            "/api/threads",
            get(move || {
                let calls = forum_calls_for_get.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Json(serde_json::json!({"items": []}))
                }
            })
            .post(move || {
                let calls = forum_calls_for_post.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    // Unique per creation: the canonical binding table enforces
                    // one thread per instance across the whole database.
                    Json(serde_json::json!({"thread": {"id": format!("thread-{}", Uuid::new_v4())}}))
                }
            }),
        )
        .route(
            "/api/threads/{thread_id}/messages",
            post(move || {
                let calls = forum_calls_for_message.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    StatusCode::CREATED
                }
            }),
        );
    let forum_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let forum_addr = forum_listener.local_addr().unwrap();
    let forum_server =
        tokio::spawn(async move { axum::serve(forum_listener, forum_app).await.unwrap() });

    // Wake fake: oauth + wake endpoint.
    let wake_calls = Arc::new(AtomicUsize::new(0));
    let wake_calls_clone = wake_calls.clone();
    let wake_app = Router::new()
        .route(
            "/oauth/token",
            post(|| async {
                Json(serde_json::json!({
                    "access_token": "wake-token", "token_type": "Bearer",
                    "expires_in": 600, "scope": "workflow.execute"
                }))
            }),
        )
        .route(
            "/workflow-execution/owner-assistance-wakes",
            post(move || {
                let wake_calls = wake_calls_clone.clone();
                async move {
                    wake_calls.fetch_add(1, Ordering::SeqCst);
                    StatusCode::OK
                }
            }),
        );
    let wake_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let wake_addr = wake_listener.local_addr().unwrap();
    let wake_server =
        tokio::spawn(async move { axum::serve(wake_listener, wake_app).await.unwrap() });

    let config = OutboxReconcilerConfig {
        forum: Some(ForumEventConfig {
            auth_base_url: format!("http://{forum_addr}"),
            client_id: "forum-client".to_string(),
            client_secret: "forum-secret".to_string(),
            forum_origin: format!("http://{forum_addr}"),
            audience: "svc-forum".to_string(),
        }),
        poll_interval_ms: 250,
        kick: Some(ExecutionKickConfig {
            url: format!("{kick_base}/workflow-execution/kicks"),
            token: "kick-static-token".to_string(),
        }),
        owner_assistance_wake: Ok(Some(OwnerAssistanceWakeConfig {
            url: format!("http://{wake_addr}/workflow-execution/owner-assistance-wakes"),
            auth_base_url: format!("http://{wake_addr}"),
            client_id: "wake-client".to_string(),
            client_secret: "wake-secret".to_string(),
            audience: "svc-workflow".to_string(),
        })),
    };
    let forum = ForumClient::new(config);
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    // 1 kick + at least 1 forum event + 1 wake.
    assert!(
        delivered >= 3,
        "each channel delivered its own rows (delivered={delivered})"
    );

    assert_eq!(kick_capture.calls.load(Ordering::SeqCst), 1);
    assert!(forum_calls.load(Ordering::SeqCst) >= 2, "thread + message");
    assert_eq!(wake_calls.load(Ordering::SeqCst), 1);
    let binding: (String,) = sqlx::query_as(
        "SELECT binding_state::text FROM workflow_forum_bindings WHERE workflow_instance_id = $1",
    )
    .bind(instance)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(binding.0, "BOUND");
    let wake_state = row_state(&pool, wake_row).await;
    assert!(wake_state.delivered_at.is_some());

    forum_server.abort();
    wake_server.abort();
    kick_server.abort();
}

// ============================================================================
// Per-instance forum ordering under concurrency (V3 CTR-SWEC-002 retention;
// Codex P2 on PR #124). The candidate query's anti-overtake predicate (a
// FORUM_EVENT row is selectable only while no earlier undelivered FORUM_EVENT
// row exists for the same instance) must hold under two racing workers,
// delayed deliveries, and backoff windows.
// ============================================================================

/// Queue a FORUM_EVENT row with an explicit created_at offset (minutes after
/// now), so ordering is deterministic relative to the creation-time
/// `wf_created:*` row.
async fn queue_forum_event_at(pool: &PgPool, instance_id: Uuid, key: &str, offset_min: i32) -> Uuid {
    let outbox_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_outbox
         (outbox_id, workflow_instance_id, outbox_kind, event_key, payload, created_at)
         VALUES ($1, $2, 'FORUM_EVENT', $3, $4, now() + make_interval(mins => $5))",
    )
    .bind(outbox_id)
    .bind(instance_id)
    .bind(key)
    .bind(serde_json::json!({"eventType": "transition_committed"}))
    .bind(offset_min)
    .execute(pool)
    .await
    .expect("queue forum event at");
    outbox_id
}

/// Forum fake that records the arrival order of event keys. The FIRST `e1`
/// attempt is delayed 200ms and returns 503 (widening the concurrency window
/// AND starting a backoff); every other post succeeds immediately.
async fn spawn_ordering_forum_fake(
    arrivals: Arc<Mutex<Vec<String>>>,
) -> (String, tokio::task::JoinHandle<()>) {
    let e1_first_posted = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let app = Router::new()
        .route(
            "/oauth/token",
            post(|| async {
                Json(serde_json::json!({"access_token": "forum-token", "expires_in": 300}))
            }),
        )
        .route(
            "/api/threads",
            get(|| async { Json(serde_json::json!({"items": []})) }).post(|| async {
                Json(serde_json::json!({"thread": {"id": format!("thread-{}", Uuid::new_v4())}}))
            }),
        )
        .route(
            "/api/threads/{thread_id}/messages",
            post(move |Json(body): Json<serde_json::Value>| {
                let arrivals = arrivals.clone();
                let e1_first_posted = e1_first_posted.clone();
                async move {
                    let key = body["metadata"]["eventKey"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    let is_e1_first =
                        key == "e1" && e1_first_posted.swap(false, Ordering::SeqCst);
                    if is_e1_first {
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                    arrivals.lock().unwrap().push(key);
                    if is_e1_first {
                        StatusCode::SERVICE_UNAVAILABLE
                    } else {
                        StatusCode::CREATED
                    }
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), server)
}

/// Drive `passes` reconciler passes, returning the total delivered count.
async fn drive_passes(
    pool: &PgPool,
    forum: &ForumClient,
    passes: usize,
) -> usize {
    let mut delivered = 0usize;
    for _ in 0..passes {
        delivered += svc_workflow::forum_sync::run_once(pool, forum).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    delivered
}

#[tokio::test]
async fn forum_events_deliver_oldest_first_under_two_concurrent_workers() {
    let (pool, _url) = isolated_db_with_url().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    // created_at ascending after the creation-time `wf_created:*` row, so the
    // expected per-instance order is [wf_created, e1, e2, e3].
    for (key, offset) in [("e1", 1), ("e2", 2), ("e3", 3)] {
        queue_forum_event_at(&pool, instance, key, offset).await;
    }

    let arrivals = Arc::new(Mutex::new(Vec::new()));
    let (forum_base, forum_server) = spawn_ordering_forum_fake(arrivals.clone()).await;
    let kick_capture = GatewayCapture::default();
    let (kick_base, kick_server) = spawn_kick_gateway(kick_capture.clone()).await;
    let forum = Arc::new(ForumClient::new(forum_and_kick_config(
        &forum_base,
        &kick_base,
    )));

    // Phase 1: two workers race while e1's first delivery is delayed 200ms
    // and then fails into backoff. Neither worker may post e2/e3 at any point
    // in this window.
    let mut workers = Vec::new();
    for _ in 0..2 {
        let pool = pool.clone();
        let forum = forum.clone();
        workers.push(tokio::spawn(async move {
            drive_passes(&pool, &forum, 5).await
        }));
    }
    let mut total = 0usize;
    for worker in workers {
        total += worker.await.unwrap();
    }
    // wf_created + the kick delivered; the e1 503 attempt is not a delivery.
    assert_eq!(
        total, 2,
        "only wf_created and the kick delivered while e1 is pending/backed off"
    );
    {
        let order = arrivals.lock().unwrap().clone();
        assert_eq!(
            order.len(),
            2,
            "e2/e3 must never be posted while e1 is pending: {order:?}"
        );
        assert!(order[0].starts_with("wf_created:"), "{order:?}");
        assert_eq!(order[1], "e1", "{order:?}");
    }
    let e1_state = row_state(&pool, outbox_row_by_key(&pool, instance, "e1").await).await;
    assert_eq!(e1_state.attempt_count, 1);
    assert!(e1_state.delivered_at.is_none());
    assert!(e1_state.next_attempt_future, "backoff window open");
    for key in ["e2", "e3"] {
        let state = row_state(&pool, outbox_row_by_key(&pool, instance, key).await).await;
        assert_eq!(state.attempt_count, 0, "{key} never attempted");
        assert!(state.delivered_at.is_none());
    }

    // Phase 2: the backoff elapses; e1 delivers; e2 still cannot overtake in
    // the same pass (its row was held back when the batch was taken).
    force_due(&pool, outbox_row_by_key(&pool, instance, "e1").await).await;
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 1, "only e1 in this pass");
    assert!(arrivals.lock().unwrap().last().unwrap() == "e1");
    let e2_state = row_state(&pool, outbox_row_by_key(&pool, instance, "e2").await).await;
    assert_eq!(e2_state.attempt_count, 0, "e2 untouched in e1's pass");

    // Phase 3: with e1 committed-delivered, e2 and then e3 proceed in order.
    let delivered = drive_passes(&pool, &forum, 4).await;
    assert_eq!(delivered, 2, "e2 then e3");
    let order = arrivals.lock().unwrap().clone();
    assert_eq!(
        order,
        vec![
            order[0].clone(), // wf_created:*
            "e1".to_string(),
            "e1".to_string(),
            "e2".to_string(),
            "e3".to_string(),
        ],
        "strict oldest-first per instance across concurrency and backoff"
    );
    forum_server.abort();
    kick_server.abort();
}

#[tokio::test]
async fn backed_off_first_forum_event_blocks_later_events_across_passes() {
    let pool = isolated_db().await;
    let (instance, _visit) = create_instance_with_kick(&pool).await;
    for (key, offset) in [("e1", 1), ("e2", 2)] {
        queue_forum_event_at(&pool, instance, key, offset).await;
    }

    let arrivals = Arc::new(Mutex::new(Vec::new()));
    let (forum_base, forum_server) = spawn_ordering_forum_fake(arrivals.clone()).await;
    let kick_capture = GatewayCapture::default();
    let (kick_base, kick_server) = spawn_kick_gateway(kick_capture.clone()).await;
    let forum = ForumClient::new(forum_and_kick_config(&forum_base, &kick_base));

    // Pass 1: wf_created delivers; e1 is held back behind it.
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 2, "kick + wf_created");
    // Pass 2: e1's first attempt fails into backoff; e2 untouched.
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 0, "e1 failed, nothing else deliverable");
    let e2_state = row_state(&pool, outbox_row_by_key(&pool, instance, "e2").await).await;
    assert_eq!(e2_state.attempt_count, 0, "e2 not attempted behind backed-off e1");
    assert!(row_state(&pool, outbox_row_by_key(&pool, instance, "e1").await)
        .await
        .next_attempt_future);

    // Pass 3 (after the backoff): e1 delivers; e2 stays untouched in the same
    // pass — the batch was taken while e1 was still undelivered.
    force_due(&pool, outbox_row_by_key(&pool, instance, "e1").await).await;
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 1, "e1 only");
    assert_eq!(
        row_state(&pool, outbox_row_by_key(&pool, instance, "e2").await)
            .await
            .attempt_count,
        0,
        "e2 not overtaken in e1's delivery pass"
    );

    // Pass 4: with e1 committed-delivered, e2 proceeds.
    let delivered = svc_workflow::forum_sync::run_once(&pool, &forum).await;
    assert_eq!(delivered, 1, "e2 follows");
    let order = arrivals.lock().unwrap().clone();
    assert_eq!(
        order,
        vec![
            order[0].clone(), // wf_created:*
            "e1".to_string(),
            "e1".to_string(),
            "e2".to_string(),
        ],
        "strict oldest-first across a backoff window"
    );
    forum_server.abort();
    kick_server.abort();
}

// ============================================================================
// helpers
// ============================================================================

async fn outbox_row_by_key(pool: &PgPool, instance_id: Uuid, event_key: &str) -> Uuid {
    sqlx::query_scalar(
        "SELECT outbox_id FROM workflow_outbox WHERE workflow_instance_id = $1 AND event_key = $2",
    )
    .bind(instance_id)
    .bind(event_key)
    .fetch_one(pool)
    .await
    .expect("outbox row by key")
}

async fn outbox_event_keys(
    pool: &PgPool,
    instance_id: Uuid,
    kind: &str,
) -> Vec<(String, serde_json::Value)> {
    sqlx::query_as(
        "SELECT event_key, payload FROM workflow_outbox WHERE workflow_instance_id = $1 AND outbox_kind = $2 ORDER BY created_at",
    )
    .bind(instance_id)
    .bind(kind)
    .fetch_all(pool)
    .await
    .expect("outbox rows")
}

async fn outbox_first_row(pool: &PgPool, instance_id: Uuid, kind: &str) -> Uuid {
    sqlx::query_scalar(
        "SELECT outbox_id FROM workflow_outbox WHERE workflow_instance_id = $1 AND outbox_kind = $2 ORDER BY created_at LIMIT 1",
    )
    .bind(instance_id)
    .bind(kind)
    .fetch_one(pool)
    .await
    .expect("outbox row")
}
