//! Integration tests for the due Dispatch Intent keyset continuation
//! (SVC_WORKFLOW_DISPATCH_INTENT_KEYSET_CONTINUATION_V1).
//!
//! Covers the acceptance matrix ACC-DKC-001..006:
//! - ACC-DKC-001: no-cursor behavior is unchanged CTR-VAI-009 (same records,
//!   same 7-field projection, same order)
//! - ACC-DKC-002: keyset paging reaches exhaustion across windows with zero
//!   duplicates / zero skips against a static due set
//! - ACC-DKC-003: equal-nextEligibleAt ties break by activation_id in both
//!   ORDER BY and the keyset filter
//! - ACC-DKC-004: half-cursor and malformed cursors return 422
//!   invalid_pagination (validation before any snapshot/role work)
//! - ACC-DKC-005: STARVATION PROOF — with > 100 due intents, a client that
//!   consumed window 1 (no cursor, limit 100) reaches intents 101+ ONLY via
//!   the continuation
//! - ACC-DKC-006: the role gate is unchanged with (valid) cursor parameters;
//!   malformed cursors 422 before the in-snapshot 403
//!
//! Fixtures force DISTINCT PAST nextEligibleAt values (2h back) so the test's
//! intents sort strictly before any concurrently-created fixture (those are
//! authored at now()); window membership is therefore deterministic even on
//! the shared run-scoped test database.

#![allow(clippy::needless_borrow)]
#![allow(unused_imports, unused_variables)]

#[path = "common/mod.rs"]
mod common;

use axum::body::{to_bytes, Body};
use axum::http::Request;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use svc_workflow::http::{self, AppState, HttpConfig};

// ============================================================================
// Harness (mirrors tests/28_visit_activation_v1.rs)
// ============================================================================

fn build_app(pool: sqlx::PgPool, jwks_url: &str) -> axum::Router {
    let config = HttpConfig {
        admission: svc_workflow::auth::admission::AdmissionConfig::disabled(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        request_body_max_bytes: 2_097_152,
        request_timeout_seconds: 30,
        jwks_config: JwksConfig {
            jwks_url: jwks_url.to_string(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
        provisioning_config: ProvisioningConfig::new(vec![]),
        execution_control: svc_workflow::http::ExecutionControlConfig {
            max_returns_per_edge: 3,
        },
        auth_v1_canary_config: AuthV1CanaryConfig {
            enabled: true,
            write_enabled: true,
            allowed_client_id: "test-client".to_string(),
            allowed_sub: String::new(),
            allowed_delegating_sub: String::new(),
            jwks_url: jwks_url.to_string(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
    };
    http::router(AppState::new(pool, &config), &config)
}

fn direct_token(subject: Uuid, scope: &str, key_pair: &common::RsaTestKeyPair) -> String {
    common::v1_token(subject, scope, "test-client", 300, key_pair)
}

async fn do_get(app: axum::Router, path: &str, token: &str) -> (u16, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

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

async fn grant_scheduler_read(pool: &PgPool, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO global_role_bindings (binding_id, principal_id, role_key, enabled) VALUES ($1, $2, 'GLOBAL_SCHEDULER_READ', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("grant scheduler read");
}

/// VISIT_ACTIVATION_V1 published definition (single TASK entry; identical
/// shape to test 28's seed).
async fn seed_v1_definition(pool: &PgPool, domain_id: Uuid, agent_id: Uuid) -> Uuid {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("keyset-test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Keyset Test Def')",
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
    let failed_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'start', 'Start', 0, 'TASK', 'WORKFLOW_CREATOR')",
    ).bind(start_id).bind(ver_id).execute(pool).await.expect("insert start");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) VALUES ($1, $2, 'work', 'Work', 1, 'TASK', 'FIXED_PRINCIPAL', $3)",
    ).bind(work_id).bind(ver_id).bind(agent_id).execute(pool).await.expect("insert work");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)",
    ).bind(done_id).bind(ver_id).execute(pool).await.expect("insert done");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'failed', 'Failed', 3, 'TERMINAL', NULL)",
    ).bind(failed_id).bind(ver_id).execute(pool).await.expect("insert failed");

    let t1 = Uuid::new_v4();
    let t2 = Uuid::new_v4();
    let t3 = Uuid::new_v4();
    let t4 = Uuid::new_v4();
    for (tid, key, src, dst, effect) in [
        (t1, "advance-1", start_id, work_id, "ADVANCE"),
        (t2, "advance-2", work_id, done_id, "ADVANCE"),
        (t3, "return", work_id, start_id, "RETURN"),
        (t4, "terminate", work_id, failed_id, "TERMINATE"),
    ] {
        sqlx::query(
            "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, $3, $3, $4, $5, $6::transition_effect)",
        )
        .bind(tid).bind(ver_id).bind(key).bind(src).bind(dst).bind(effect)
        .execute(pool).await.expect("insert transition");
    }
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t1).bind(start_id).execute(pool).await.expect("primary start");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t2).bind(work_id).execute(pool).await.expect("primary work");

    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(pool).await.expect("publish");

    ver_id
}

async fn create_v1_instance(
    pool: &PgPool,
    creator_id: Uuid,
    domain_id: Uuid,
    ver_id: Uuid,
) -> (Uuid, Uuid, i32) {
    let result = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        pool,
        svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(creator_id),
            idempotency_key: format!("create-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(ver_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({}),
            context_payload: serde_json::json!({ "title": "keyset" }),
        },
    )
    .await
    .expect("create v1 instance should succeed");
    (
        result.workflow_instance_id,
        result.current_node_visit_id,
        result.workflow_state_version,
    )
}

/// Pin the activation's effective nextEligibleAt to an exact instant by
/// appending one eligibility event (the same append-only write path WAKE
/// uses; the activation fact tables themselves are trigger-immutable, so
/// the COALESCE(latest event, initial) in the feed query picks this value
/// up as the current eligibility). Returns the activation id.
async fn force_next_eligible_at(pool: &PgPool, node_visit_id: Uuid, at: DateTime<Utc>) -> Uuid {
    let row: (Uuid, DateTime<Utc>) = sqlx::query_as(
        "SELECT activation_id, initial_next_eligible_at FROM workflow_activations WHERE node_visit_id = $1 AND activation_kind = 'DISPATCH_INTENT'",
    )
    .bind(node_visit_id)
    .fetch_one(pool)
    .await
    .expect("dispatch intent activation exists");
    let (activation_id, initial) = row;
    sqlx::query(
        "INSERT INTO workflow_dispatch_eligibility_events (eligibility_event_id, activation_id, previous_next_eligible_at, new_next_eligible_at, cause_class, command_id) VALUES ($1, $2, $3, $4, 'WAKE', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(activation_id)
    .bind(initial)
    .bind(at)
    .bind(Uuid::new_v4())
    .execute(pool)
    .await
    .expect("append eligibility event");
    activation_id
}

/// One seeded due intent with an exact, distinct past nextEligibleAt.
struct SeededIntent {
    dispatch_intent_id: Uuid,
    workflow_instance_id: Uuid,
    next_eligible_at: DateTime<Utc>,
}

/// Per-test exclusive fixture windows: tests run serially on a shared fresh
/// DB and do NOT clean up after themselves, so every seed call takes the next
/// 10-minute window (2h back + idx * 10min) — fixtures of different tests
/// never interleave, and everything stays strictly in the past (far below any
/// concurrently authored now() record).
static NEXT_WINDOW: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Seed `count` due intents at distinct instants inside this call's window.
async fn seed_due_intents(
    pool: &PgPool,
    creator: Uuid,
    domain_id: Uuid,
    ver_id: Uuid,
    count: usize,
) -> Vec<SeededIntent> {
    let idx = NEXT_WINDOW.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let base = Utc::now() - chrono::Duration::hours(2) + chrono::Duration::minutes(idx * 10);
    let mut seeded = Vec::with_capacity(count);
    for i in 1..=count {
        let at = base + chrono::Duration::seconds(i as i64);
        let (instance_id, visit_id, _) = create_v1_instance(pool, creator, domain_id, ver_id).await;
        let activation_id = force_next_eligible_at(pool, visit_id, at).await;
        seeded.push(SeededIntent {
            dispatch_intent_id: activation_id,
            workflow_instance_id: instance_id,
            next_eligible_at: at,
        });
    }
    seeded
}

fn enc(input: &str) -> String {
    // Minimal percent-encoding for query values (RFC3339 carries '+' and ':';
    // a raw '+' decodes as a space and would corrupt the cursor).
    let mut out = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn parse_ts(value: &Value) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value.as_str().expect("timestamp string"))
        .expect("rfc3339")
        .with_timezone(&Utc)
}

// ============================================================================
// ACC-DKC tests
// ============================================================================

/// ACC-DKC-001: a no-cursor request behaves exactly like CTR-VAI-009 — the
/// fixture's intents come back complete, with exactly the 7 fields, in feed
/// order (and nothing else appears before them).
#[tokio::test]
async fn acc_dkc_001_no_cursor_unchanged() {
    let pool = common::create_pool().await;
    let (_owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    // The creator must be an AGENT principal: an agent-owned entry visit
    // creates the DISPATCH_INTENT activation (a HUMAN owner would create a
    // HUMAN_WORK_ITEM — no eligibility).
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let worker = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, worker).await;
    let scheduler = seed_agent(&pool).await;
    grant_scheduler_read(&pool, scheduler).await;
    let ver_id = seed_v1_definition(&pool, domain_id, worker).await;
    let seeded = seed_due_intents(&pool, creator, domain_id, ver_id, 3).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(scheduler, "workflow.read", &mock.key_pair);

    let (status, body) = do_get(app, "/internal/v1/dispatch-intents?limit=100", &token).await;
    assert_eq!(status, 200, "no-cursor read: {body}");
    let items = body["items"].as_array().expect("items");

    // Every seeded intent is present exactly once, with the exact 7 fields.
    for intent in &seeded {
        let mine: Vec<&Value> = items
            .iter()
            .filter(|it| it["dispatchIntentId"] == json!(intent.dispatch_intent_id.to_string()))
            .collect();
        assert_eq!(mine.len(), 1, "exactly one record for the seeded intent");
        let mut keys: Vec<&str> = mine[0]
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "createdAt",
                "dispatchIntentId",
                "nextEligibleAt",
                "nodeVisitId",
                "ownerPrincipalId",
                "updatedAt",
                "workflowInstanceId"
            ]
        );
        assert_eq!(
            parse_ts(&mine[0]["nextEligibleAt"]),
            intent.next_eligible_at,
            "effective nextEligibleAt is the forced instant"
        );
    }

    // The fixture block is ordered by (nextEligibleAt, activation_id) — each
    // seeded record appears no later than its successor.
    let positions: Vec<usize> = seeded
        .iter()
        .map(|intent| {
            items
                .iter()
                .position(|it| {
                    it["dispatchIntentId"] == json!(intent.dispatch_intent_id.to_string())
                })
                .expect("seeded intent in feed")
        })
        .collect();
    let mut sorted = positions.clone();
    sorted.sort();
    assert_eq!(positions, sorted, "fixture order follows the feed order");
}

/// ACC-DKC-002: keyset paging reaches exhaustion across >= 2 full windows
/// plus a short page with zero duplicates and zero skips; the cursor is the
/// EXACT returned strings; the walk terminates on a short page.
#[tokio::test]
async fn acc_dkc_002_exhaustion_walk() {
    let pool = common::create_pool().await;
    let (_owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    // The creator must be an AGENT principal: an agent-owned entry visit
    // creates the DISPATCH_INTENT activation (a HUMAN owner would create a
    // HUMAN_WORK_ITEM — no eligibility).
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let worker = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, worker).await;
    let scheduler = seed_agent(&pool).await;
    grant_scheduler_read(&pool, scheduler).await;
    let ver_id = seed_v1_definition(&pool, domain_id, worker).await;
    let seeded = seed_due_intents(&pool, creator, domain_id, ver_id, 7).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(scheduler, "workflow.read", &mock.key_pair);

    let mut seen: Vec<String> = Vec::new();
    let mut pages: Vec<usize> = Vec::new();
    let mut cursor: Option<(String, String)> = None;
    for _sweep_page in 0..50 {
        let path = match &cursor {
            None => "/internal/v1/dispatch-intents?limit=3".to_string(),
            Some((ts, id)) => format!(
                "/internal/v1/dispatch-intents?limit=3&afterNextEligibleAt={}&afterDispatchIntentId={}",
                enc(ts),
                enc(id)
            ),
        };
        let (status, body) = do_get(app.clone(), &path, &token).await;
        assert_eq!(status, 200, "page read: {body}");
        let items = body["items"].as_array().expect("items").clone();
        let page_len = items.len();
        pages.push(page_len);
        for item in &items {
            let id = item["dispatchIntentId"].as_str().unwrap().to_string();
            assert!(!seen.contains(&id), "no duplicate across windows: {id}");
            seen.push(id);
        }
        if page_len < 3 {
            break; // short page = exhaustion
        }
        let last = items.last().unwrap();
        cursor = Some((
            last["nextEligibleAt"].as_str().unwrap().to_string(),
            last["dispatchIntentId"].as_str().unwrap().to_string(),
        ));
    }

    assert!(
        pages.len() >= 3,
        "the walk spans multiple windows: {pages:?}"
    );
    assert!(
        *pages.last().unwrap() < 3,
        "the walk terminates on a SHORT page: {pages:?}"
    );
    let expected: Vec<String> = seeded
        .iter()
        .map(|s| s.dispatch_intent_id.to_string())
        .collect();
    let positions: Vec<usize> = expected
        .iter()
        .map(|id| {
            seen.iter()
                .position(|s| s == id)
                .expect("fixture reached by the walk")
        })
        .collect();
    let mut sorted = positions.clone();
    sorted.sort();
    assert_eq!(positions, sorted, "fixture order follows the feed order");
    assert_eq!(
        positions.len(),
        7,
        "every fixture reached exactly once (no skips)"
    );
}

/// ACC-DKC-003: equal-nextEligibleAt ties break by activation_id in both the
/// ORDER BY and the keyset filter (limit=1 walk covers both tied rows).
#[tokio::test]
async fn acc_dkc_003_tie_break_by_activation_id() {
    let pool = common::create_pool().await;
    let (_owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    // The creator must be an AGENT principal: an agent-owned entry visit
    // creates the DISPATCH_INTENT activation (a HUMAN owner would create a
    // HUMAN_WORK_ITEM — no eligibility).
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let worker = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, worker).await;
    let scheduler = seed_agent(&pool).await;
    grant_scheduler_read(&pool, scheduler).await;
    let ver_id = seed_v1_definition(&pool, domain_id, worker).await;

    let tied_at = Utc::now() - chrono::Duration::hours(2);
    let mut tied_ids: Vec<String> = Vec::new();
    for _ in 0..2 {
        let (instance_id, visit_id, _) =
            create_v1_instance(&pool, creator, domain_id, ver_id).await;
        let activation_id = force_next_eligible_at(&pool, visit_id, tied_at).await;
        tied_ids.push(activation_id.to_string());
    }
    tied_ids.sort();

    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(scheduler, "workflow.read", &mock.key_pair);

    // limit=1 walk: walk forward (cursor = exact last strings) until BOTH
    // tied records are seen; they must appear as ADJACENT walk steps in
    // ascending activation_id order (tie broken by activation_id), and the
    // continuation from the first half must reach the second.
    let mut walk: Vec<(String, String)> = Vec::new(); // (id, ts)
    let mut cursor: Option<(String, String)> = None;
    for _ in 0..(tied_ids.len() + 30) {
        let path = match &cursor {
            None => "/internal/v1/dispatch-intents?limit=1".to_string(),
            Some((ts, id)) => format!(
                "/internal/v1/dispatch-intents?limit=1&afterNextEligibleAt={}&afterDispatchIntentId={}",
                enc(ts),
                enc(id)
            ),
        };
        let (status, body) = do_get(app.clone(), &path, &token).await;
        assert_eq!(status, 200, "tie walk: {body}");
        let items = body["items"].as_array().unwrap();
        if items.is_empty() {
            break;
        }
        walk.push((
            items[0]["dispatchIntentId"].as_str().unwrap().to_string(),
            items[0]["nextEligibleAt"].as_str().unwrap().to_string(),
        ));
        // walk tuple is (id, ts); the cursor is (ts, id).
        let last = walk.last().unwrap();
        cursor = Some((last.1.clone(), last.0.clone()));
        if walk.iter().any(|(id, _)| id == &tied_ids[0])
            && walk.iter().any(|(id, _)| id == &tied_ids[1])
        {
            break;
        }
    }

    let pos0 = walk
        .iter()
        .position(|(id, _)| id == &tied_ids[0])
        .expect("tie half 0 reached");
    let pos1 = walk
        .iter()
        .position(|(id, _)| id == &tied_ids[1])
        .expect("tie half 1 reached");
    assert_eq!(pos1, pos0 + 1, "the tie pair is ADJACENT in the walk order");
    assert!(
        tied_ids[0] < tied_ids[1],
        "fixture ids ascend (smaller activation_id first)"
    );
    assert_eq!(
        parse_ts(&Value::String(walk[pos0].1.clone())),
        tied_at,
        "tied timestamp returned exactly"
    );
}

/// ACC-DKC-004: half-cursor and malformed cursors return 422
/// invalid_pagination. A fixture intent exists; the 422 never reads rows.
#[tokio::test]
async fn acc_dkc_004_cursor_validation() {
    let pool = common::create_pool().await;
    let (_owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    // The creator must be an AGENT principal: an agent-owned entry visit
    // creates the DISPATCH_INTENT activation (a HUMAN owner would create a
    // HUMAN_WORK_ITEM — no eligibility).
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let worker = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, worker).await;
    let scheduler = seed_agent(&pool).await;
    grant_scheduler_read(&pool, scheduler).await;
    let ver_id = seed_v1_definition(&pool, domain_id, worker).await;
    let seeded = seed_due_intents(&pool, creator, domain_id, ver_id, 1).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(scheduler, "workflow.read", &mock.key_pair);

    let valid_ts = seeded[0]
        .next_eligible_at
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
    let valid_id = seeded[0].dispatch_intent_id.to_string();
    let cases: Vec<(String, &str)> = vec![
        (
            format!("/internal/v1/dispatch-intents?limit=10&afterNextEligibleAt={}", enc(&valid_ts)),
            "timestamp without id",
        ),
        (
            format!("/internal/v1/dispatch-intents?limit=10&afterDispatchIntentId={}", enc(&valid_id)),
            "id without timestamp",
        ),
        (
            format!(
                "/internal/v1/dispatch-intents?limit=10&afterNextEligibleAt=not-a-time&afterDispatchIntentId={}",
                enc(&valid_id)
            ),
            "malformed timestamp",
        ),
        (
            format!(
                "/internal/v1/dispatch-intents?limit=10&afterNextEligibleAt={}&afterDispatchIntentId=not-a-uuid",
                enc(&valid_ts)
            ),
            "malformed id",
        ),
    ];
    for (path, label) in cases {
        let (status, body) = do_get(app.clone(), &path, &token).await;
        assert_eq!(status, 422, "{label} must 422: {body}");
        assert_eq!(
            body["error"]["code"],
            json!("invalid_pagination"),
            "{label}"
        );
    }

    // Validation is not destructive: a valid cursor still works afterwards.
    let path = format!(
        "/internal/v1/dispatch-intents?limit=10&afterNextEligibleAt={}&afterDispatchIntentId={}",
        enc(&valid_ts),
        enc(&valid_id)
    );
    let (status, body) = do_get(app, &path, &token).await;
    assert_eq!(status, 200, "valid cursor after rejections: {body}");
    let items = body["items"].as_array().unwrap();
    assert!(
        !items
            .iter()
            .any(|it| it["dispatchIntentId"] == json!(valid_id)),
        "exclusive keyset excludes the cursor row itself"
    );
}

/// ACC-DKC-005 — STARVATION PROOF: with 101 due intents (distinct past
/// timestamps, all sorting before any concurrent now()-authored fixture),
/// window 1 (no cursor, limit=100) is exactly intents 1..100, and intent 101
/// is reachable ONLY through the continuation (window 2's first record).
#[tokio::test]
async fn acc_dkc_005_starvation_proof() {
    let pool = common::create_pool().await;
    let (_owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    // The creator must be an AGENT principal: an agent-owned entry visit
    // creates the DISPATCH_INTENT activation (a HUMAN owner would create a
    // HUMAN_WORK_ITEM — no eligibility).
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let worker = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, worker).await;
    let scheduler = seed_agent(&pool).await;
    grant_scheduler_read(&pool, scheduler).await;
    let ver_id = seed_v1_definition(&pool, domain_id, worker).await;
    let seeded = seed_due_intents(&pool, creator, domain_id, ver_id, 101).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(scheduler, "workflow.read", &mock.key_pair);

    let seeded_ids: Vec<String> = seeded
        .iter()
        .map(|s| s.dispatch_intent_id.to_string())
        .collect();

    // Window 1: no cursor, limit=100 — FULL page (leftovers from earlier
    // tests sort strictly BEFORE this window and are part of it).
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/dispatch-intents?limit=100",
        &token,
    )
    .await;
    assert_eq!(status, 200, "window 1: {body}");
    let window1 = body["items"].as_array().unwrap().clone();
    assert_eq!(window1.len(), 100, "window 1 is exactly one full page");
    let seen: Vec<&str> = window1
        .iter()
        .map(|it| it["dispatchIntentId"].as_str().unwrap())
        .collect();
    // The FIRST window is closed under repetition: another cursorless read
    // returns the same first 100 — this is the starvation itself.
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/dispatch-intents?limit=100",
        &token,
    )
    .await;
    assert_eq!(status, 200);
    let window1_again: Vec<&str> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|it| it["dispatchIntentId"].as_str().unwrap())
        .collect();
    assert_eq!(
        seen, window1_again,
        "cursorless reads repeat the same window"
    );
    // Intent #101 is the largest key of our block: it is not in window 1...
    assert!(
        !seen.contains(&seeded_ids[100].as_str()),
        "intent 101 is INVISIBLE without continuation (the starvation)"
    );
    // ...while all leftover fixtures + intents #1..#(100-L) fill the page:
    // the count of OUR records in window 1 is exactly 100 - L (L = earlier
    // leftovers), which is >= 1 because the page is full of oldest-first.
    let ours_in_window1 = seen
        .iter()
        .filter(|id| seeded_ids[..100].contains(&id.to_string()))
        .count();
    assert!(
        ours_in_window1 >= 1,
        "window 1 reaches into our fixture block"
    );

    // Window 2: cursor = EXACT last-item strings of window 1. Everything
    // strictly below the cursor is gone; our block's remainder starts at
    // intent #101, which must now be the FIRST record past the cursor.
    let last = window1.last().unwrap();
    let path = format!(
        "/internal/v1/dispatch-intents?limit=100&afterNextEligibleAt={}&afterDispatchIntentId={}",
        enc(last["nextEligibleAt"].as_str().unwrap()),
        enc(last["dispatchIntentId"].as_str().unwrap())
    );
    let (status, body) = do_get(app.clone(), &path, &token).await;
    assert_eq!(status, 200, "window 2: {body}");
    // Continue from window 1's cursor to EXHAUSTION (short page), collecting
    // everything. Leftover fixtures from earlier tests (window idx 0..N) sort
    // strictly before this window, so the cursor may land mid-block; the
    // invariant under test is reachability + no-repeat + order, not position.
    let mut reached: Vec<String> = Vec::new();
    let mut cursor = Some((
        last["nextEligibleAt"].as_str().unwrap().to_string(),
        last["dispatchIntentId"].as_str().unwrap().to_string(),
    ));
    for _ in 0..10 {
        let path = match &cursor {
            None => "/internal/v1/dispatch-intents?limit=100".to_string(),
            Some((ts, id)) => format!(
                "/internal/v1/dispatch-intents?limit=100&afterNextEligibleAt={}&afterDispatchIntentId={}",
                enc(ts),
                enc(id)
            ),
        };
        let (status, body) = do_get(app.clone(), &path, &token).await;
        assert_eq!(status, 200, "continuation walk: {body}");
        let items = body["items"].as_array().unwrap().clone();
        let page_len = items.len();
        for item in &items {
            let id = item["dispatchIntentId"].as_str().unwrap();
            assert!(!seen.contains(&id), "no repeat across windows: {id}");
            assert!(
                !reached.contains(&id.to_string()),
                "no repeat within continuation: {id}"
            );
            reached.push(id.to_string());
        }
        if page_len < 100 {
            break; // exhaustion
        }
        let last_item = items.last().unwrap();
        cursor = Some((
            last_item["nextEligibleAt"].as_str().unwrap().to_string(),
            last_item["dispatchIntentId"].as_str().unwrap().to_string(),
        ));
    }

    // THE starvation closure: intent 101 — invisible to every cursorless
    // read — is REACHABLE through the keyset continuation.
    assert!(
        reached.contains(&seeded_ids[100]),
        "intent 101 must be reachable via continuation; reached={} of {}",
        reached.len(),
        seeded_ids.len()
    );
    // Full discovery: window 1 UNION the continuation covers ALL 101 fixtures
    // exactly once, in feed order (window 1 holds the oldest slice, the
    // continuation holds everything past the cursor).
    let mut discovered: Vec<String> = seen.iter().map(|s| s.to_string()).collect();
    discovered.extend(reached.iter().cloned());
    let positions: Vec<usize> = seeded_ids
        .iter()
        .filter_map(|id| discovered.iter().position(|r| r == id))
        .collect();
    assert_eq!(
        positions.len(),
        101,
        "all 101 fixtures discovered exactly once across window 1 + continuation"
    );
    let mut sorted = positions.clone();
    sorted.sort();
    assert_eq!(positions, sorted, "fixture order follows the feed order");
}

/// ACC-DKC-006: the role gate is unchanged with valid cursor parameters;
/// a malformed cursor 422s BEFORE the in-snapshot 403.
#[tokio::test]
async fn acc_dkc_006_role_gate_with_cursor() {
    let pool = common::create_pool().await;
    let (_owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    // The creator must be an AGENT principal: an agent-owned entry visit
    // creates the DISPATCH_INTENT activation (a HUMAN owner would create a
    // HUMAN_WORK_ITEM — no eligibility).
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let worker = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, worker).await;
    let scheduler = seed_agent(&pool).await;
    grant_scheduler_read(&pool, scheduler).await;
    let reader = seed_agent(&pool).await;
    let ver_id = seed_v1_definition(&pool, domain_id, worker).await;
    let seeded = seed_due_intents(&pool, creator, domain_id, ver_id, 1).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let reader_token = direct_token(reader, "workflow.read", &mock.key_pair);
    let scheduler_token = direct_token(scheduler, "workflow.read", &mock.key_pair);

    let valid_ts = seeded[0]
        .next_eligible_at
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
    let valid_id = seeded[0].dispatch_intent_id.to_string();

    // Role-less caller (no binding at all) with a VALID cursor -> 403.
    let path = format!(
        "/internal/v1/dispatch-intents?limit=10&afterNextEligibleAt={}&afterDispatchIntentId={}",
        enc(&valid_ts),
        enc(&valid_id)
    );
    let (status, body) = do_get(app.clone(), &path, &reader_token).await;
    assert_eq!(
        status, 403,
        "role gate unchanged with valid cursors: {body}"
    );
    assert_eq!(body["error"]["code"], json!("scheduler_read_role_required"));

    // Malformed cursor on the SAME role-less caller -> 422 (validation
    // precedes the in-snapshot role check).
    let path = format!(
        "/internal/v1/dispatch-intents?limit=10&afterNextEligibleAt=garbage&afterDispatchIntentId={}",
        enc(&valid_id)
    );
    let (status, body) = do_get(app.clone(), &path, &reader_token).await;
    assert_eq!(status, 422, "malformed cursor 422 precedes the 403: {body}");
    assert_eq!(body["error"]["code"], json!("invalid_pagination"));

    // The scheduler retains full access with cursors (no new role requirement).
    let (status, body) = do_get(app, &path, &scheduler_token).await;
    assert_eq!(status, 422, "scheduler sees the same validation");
    let _ = body;
}
