//! Integration tests for the work execution class
//! (SVC_WORKFLOW_WORK_EXECUTION_CLASS_V1, acceptance matrix T5a–m).
//!
//! Covers the Owner-frozen hard checks:
//! - T5a: marked (NON_BUSINESS_TEST) instance with a due DISPATCH_INTENT is
//!   absent from the business due feed while a BUSINESS control is present
//! - T5b: wake on a marked activation is receipted and the row STAYS absent
//! - T5c: unmarked creation is byte-identical legacy behavior (BUSINESS, in feed)
//! - T5d: post-migration rows read BUSINESS (column default IS the backfill)
//! - T5e: unknown executionClass value → 422 invalid_input, zero persistence
//! - T5f: ordinary member marking → 403 not_domain_owner, zero runtime facts;
//!   same-IK identical rejected body → deterministic replay (no new facts)
//! - T5g: DOMAIN_OWNER marking → persisted NON_BUSINESS_TEST
//! - T5h: the SAME marked instance stays visible/executable via its assignee
//!   worklist (targeted path intact — no worklist filtering)
//! - T5i: domain/global safe summaries expose `execution_class`
//! - T5j: class-mixed due window returns only BUSINESS rows in unchanged order
//! - T5k: a DOMAIN_OWNER of another domain cannot mark (binding scope is the
//!   instance's domain)
//! - idempotency: same IK + BUSINESS vs NON_BUSINESS_TEST → 409
//!   idempotency_conflict (never a replay under the old class)
//!
//! Serial on a shared run-scoped DB; fixtures use the same staggered past
//! window scheme as tests/29 so concurrent windows never interleave.

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
use svc_workflow::domain::enums::WorkflowExecutionClass;
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use svc_workflow::http::{self, AppState, HttpConfig};

// ============================================================================
// Harness (mirrors tests/29_dispatch_intent_keyset.rs)
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

async fn do_post(app: axum::Router, path: &str, token: &str, body: Value) -> (u16, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("idempotency-key", format!("ik-{}", Uuid::new_v4()))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
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

async fn seed_global_role(pool: &PgPool, principal_id: Uuid, role_key: &str) {
    sqlx::query(
        "INSERT INTO global_role_bindings (binding_id, principal_id, role_key, enabled) VALUES ($1, $2, $3, TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .bind(role_key)
    .execute(pool)
    .await
    .expect("seed global role");
}

/// VISIT_ACTIVATION_V1 published model-3 definition: WORKFLOW_CREATOR entry
/// TASK (creator resolves to an AGENT ⇒ DISPATCH_INTENT), FIXED_PRINCIPAL
/// work node, TERMINAL done. Identical shape to tests/29's seed.
async fn seed_v1_definition(pool: &PgPool, domain_id: Uuid, agent_id: Uuid) -> Uuid {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("wec-test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'WEC Test Def')",
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
    let t2 = Uuid::new_v4();
    for (tid, key, src, dst, effect) in [
        (t1, "advance-1", start_id, work_id, "ADVANCE"),
        (t2, "advance-2", work_id, done_id, "ADVANCE"),
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

async fn create_instance_command(
    creator_id: Uuid,
    domain_id: Uuid,
    ver_id: Uuid,
    execution_class: WorkflowExecutionClass,
    idempotency_key: String,
) -> CreateWorkflowInstanceCommand {
    CreateWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(creator_id),
        idempotency_key,
        command_schema_version: "v1".to_string(),
        execution_class,
        domain_id: DomainId::from_uuid(domain_id),
        definition_version_id: DefinitionVersionId::from_uuid(ver_id),
        external_reference: None,
        external_url: None,
        metadata: serde_json::json!({}),
        context_payload: serde_json::json!({ "title": "wec" }),
    }
}

/// Pin an activation's effective nextEligibleAt to an exact (past) instant
/// via the append-only eligibility-event write path (the WAKE write path).
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

/// Per-test exclusive past windows (same scheme as tests/29): different
/// tests never interleave and everything is strictly in the past (due).
static NEXT_WINDOW: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(100);

async fn due_instant() -> DateTime<Utc> {
    let idx = NEXT_WINDOW.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Utc::now() - chrono::Duration::hours(3) + chrono::Duration::minutes(idx)
}

/// Full fixture: an enabled AGENT creator (domain member), a DOMAIN_OWNER
/// principal, and a published model-3 definition with an AGENT-owned entry.
async fn seed_fixture(
    pool: &PgPool,
) -> (Uuid, Uuid, Uuid, Uuid) {
    // A domain with NO other owner: the creator AGENT holds BOTH the member
    // binding and the single DOMAIN_OWNER governance binding (the
    // idx_drb_single_owner invariant permits exactly one enabled owner).
    // Marked creates must ride an AGENT owner so the activation is a
    // DISPATCH_INTENT (the feed-exclusion subject).
    let (_owner, domain_id) = common::seed_principal_and_domain(&pool).await;
    let creator = seed_agent(pool).await;
    seed_domain_member(pool, domain_id, creator).await;
    seed_domain_owner_binding(pool, domain_id, creator).await;
    let ver_id = seed_v1_definition(pool, domain_id, creator).await;
    (creator, domain_id, creator, ver_id)
}

async fn seed_domain_owner_binding(pool: &PgPool, domain_id: Uuid, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1, $2, $3, 'DOMAIN_OWNER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("seed domain owner binding");
}

// ============================================================================
// T5d — migration default
// ============================================================================

#[tokio::test]
async fn t5d_migration_default_is_business() {
    let pool = common::create_pool().await;
    // Raw insert that omits the class column entirely (the historical form).
    let (creator, domain_id) = common::seed_principal_and_domain(&pool).await;
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'T5D Def')",
    )
    .bind(def_id).bind(domain_id).bind(format!("t5d-{}", &Uuid::new_v4().to_string()[..8]))
    .execute(&pool).await.expect("insert def");
    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, semantic_model_version) VALUES ($1, $2, 1, 'PUBLISHED', 1)",
    )
    .bind(ver_id).bind(def_id)
    .execute(&pool).await.expect("insert version");
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_instances (workflow_instance_id, domain_id, definition_version_id, created_by_principal_id, workflow_state_version) VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(id)
    .bind(domain_id)
    .bind(ver_id)
    .bind(creator)
    .execute(&pool)
    .await
    .expect("raw insert");
    let class: String = sqlx::query_scalar("SELECT execution_class::text FROM workflow_instances WHERE workflow_instance_id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("row exists");
    assert_eq!(class, "BUSINESS", "column default IS the compatibility story");
}

// ============================================================================
// T5g / T5a / T5b / T5h / T5i / T5j — owner marking + structural exclusion
// ============================================================================

#[tokio::test]
async fn t5g_owner_marking_persists_and_t5a_excluded_from_due_feed() {
    let pool = common::create_pool().await;
    let (owner, domain_id, creator, ver_id) = seed_fixture(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);

    // Owner marks the instance NON_BUSINESS_TEST over HTTP.
    let (status, body) = do_post(
        app.clone(),
        "/internal/v1/workflow-instances",
        &direct_token(creator, "workflow.execute", &mock.key_pair),
        json!({
            "domainId": domain_id,
            "definitionVersionId": ver_id,
            "executionClass": "NON_BUSINESS_TEST",
            "metadata": {},
            "contextPayload": {"title": "marked"}
        }),
    )
    .await;
    assert_eq!(status, 201, "owner marking must succeed: {body}");
    let marked_instance = body["workflowInstanceId"].as_str().unwrap().to_string();
    let marked_visit = body["currentNodeVisitId"].as_str().unwrap().to_string();
    let marked_state_version = body["workflowStateVersion"].as_i64().unwrap() as i32;

    let class: String = sqlx::query_scalar(
        "SELECT execution_class::text FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(Uuid::parse_str(&marked_instance).unwrap())
    .fetch_one(&pool)
    .await
    .expect("row exists");
    assert_eq!(class, "NON_BUSINESS_TEST");

    // T5a: force the marked intent due; it must NOT appear in the feed.
    // A BUSINESS control instance seeded in the same window MUST appear.
    let marked_at = due_instant().await;
    let marked_activation = force_next_eligible_at(&pool, Uuid::parse_str(&marked_visit).unwrap(), marked_at).await;

    let (cstatus, cbody) = do_post(
        app.clone(),
        "/internal/v1/workflow-instances",
        &direct_token(creator, "workflow.execute", &mock.key_pair),
        json!({
            "domainId": domain_id,
            "definitionVersionId": ver_id,
            "metadata": {},
            "contextPayload": {"title": "control"}
        }),
    )
    .await;
    assert_eq!(cstatus, 201, "unmarked create is normal: {cbody}");
    let control_visit = cbody["currentNodeVisitId"].as_str().unwrap().to_string();
    let control_at = due_instant().await;
    let control_activation = force_next_eligible_at(&pool, Uuid::parse_str(&control_visit).unwrap(), control_at).await;

    // Scheduler-role feed read.
    let scheduler = seed_agent(&pool).await;
    seed_global_role(&pool, scheduler, "GLOBAL_SCHEDULER_READ").await;
    let (fstatus, fbody) = do_get(
        app.clone(),
        "/internal/v1/dispatch-intents?limit=100",
        &direct_token(scheduler, "workflow.read", &mock.key_pair),
    )
    .await;
    assert_eq!(fstatus, 200, "{fbody}");
    let records = fbody["items"].as_array().cloned().unwrap_or_default();
    let ids: Vec<&str> = records
        .iter()
        .map(|r| r["dispatchIntentId"].as_str().unwrap())
        .collect();
    assert!(
        !ids.contains(&marked_activation.to_string().as_str()),
        "T5a: marked intent must be structurally absent from the business feed"
    );
    assert!(
        ids.contains(&control_activation.to_string().as_str()),
        "T5c: unmarked control must be present (row shape/order unchanged)"
    );

    // T5b: wake on the marked activation — receipted, and it STAYS absent.
    let (wstatus, wbody) = do_post(
        app.clone(),
        &format!(
            "/internal/v1/workflow-instances/{marked_instance}/node-visits/{marked_visit}/wake"
        ),
        &direct_token(scheduler, "workflow.execute", &mock.key_pair),
        json!({
            "expectedWorkflowStateVersion": marked_state_version,
            "cause": "t5b wake"
        }),
    )
    .await;
    assert_eq!(wstatus, 200, "wake is receipted: {wbody}");
    let (_, fbody2) = do_get(
        app.clone(),
        "/internal/v1/dispatch-intents?limit=100",
        &direct_token(scheduler, "workflow.read", &mock.key_pair),
    )
    .await;
    let records2 = fbody2["items"].as_array().cloned().unwrap_or_default();
    let ids2: Vec<String> = records2
        .iter()
        .map(|r| r["dispatchIntentId"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !ids2.contains(&marked_activation.to_string()),
        "T5b: wake must not re-admit a marked row into the business feed"
    );
}

#[tokio::test]
async fn t5h_marked_instance_still_on_assignee_worklist() {
    let pool = common::create_pool().await;
    let (owner, domain_id, creator, ver_id) = seed_fixture(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);

    let (status, body) = do_post(
        app.clone(),
        "/internal/v1/workflow-instances",
        &direct_token(creator, "workflow.execute", &mock.key_pair),
        json!({
            "domainId": domain_id,
            "definitionVersionId": ver_id,
            "executionClass": "NON_BUSINESS_TEST",
            "metadata": {},
            "contextPayload": {"title": "worklist-visible"}
        }),
    )
    .await;
    assert_eq!(status, 201, "{body}");

    // The entry node is WORKFLOW_CREATOR → the creator's own worklist still
    // shows the marked instance (targeted path intact).
    let (wstatus, wbody) = do_get(
        app,
        "/internal/v1/worklists/assigned-to-me",
        &direct_token(creator, "workflow.read", &mock.key_pair),
    )
    .await;
    assert_eq!(wstatus, 200, "{wbody}");
    let items = wbody["items"].as_array().cloned().unwrap_or_default();
    assert!(
        items.iter().any(|i| i["detail"]["instance"]["workflow_instance_id"].as_str() == body["workflowInstanceId"].as_str()),
        "T5h: marked instance must remain fully visible/executable via its assignee worklist"
    );
}

#[tokio::test]
async fn t5i_global_summary_exposes_execution_class() {
    let pool = common::create_pool().await;
    let (owner, domain_id, creator, ver_id) = seed_fixture(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);

    let (_, body) = do_post(
        app.clone(),
        "/internal/v1/workflow-instances",
        &direct_token(creator, "workflow.execute", &mock.key_pair),
        json!({
            "domainId": domain_id,
            "definitionVersionId": ver_id,
            "executionClass": "NON_BUSINESS_TEST",
            "metadata": {},
            "contextPayload": {"title": "summary-visible"}
        }),
    )
    .await;
    let marked = body["workflowInstanceId"].as_str().unwrap().to_string();

    let coordinator = seed_agent(&pool).await;
    seed_global_role(&pool, coordinator, "GLOBAL_WORKFLOW_COORDINATOR").await;
    let (gstatus, gbody) = do_get(
        app,
        "/internal/v1/workflow-instances/global?limit=100",
        &direct_token(coordinator, "workflow.read", &mock.key_pair),
    )
    .await;
    assert_eq!(gstatus, 200, "{gbody}");
    let records = gbody["items"].as_array().cloned().unwrap_or_default();
    let mine = records
        .iter()
        .find(|r| r["workflow_instance_id"].as_str() == Some(marked.as_str()))
        .expect("marked instance in safe summary");
    assert_eq!(
        mine["execution_class"].as_str(),
        Some("NON_BUSINESS_TEST"),
        "T5i: class-only positive visibility on the safe summary (snake_case)"
    );
}

#[tokio::test]
async fn t5j_class_mixed_window_returns_only_business_in_order() {
    let pool = common::create_pool().await;
    let (owner, domain_id, creator, ver_id) = seed_fixture(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);

    // Two BUSINESS rows then one marked row, seeded at strictly ordered
    // distinct past instants within this test's window.
    let mut activations = Vec::new();
    for marked in [false, false, true] {
        let body_builder = if marked {
            json!({
                "domainId": domain_id,
                "definitionVersionId": ver_id,
                "executionClass": "NON_BUSINESS_TEST",
                "metadata": {},
                "contextPayload": {"title": "mixed-marked"}
            })
        } else {
            json!({
                "domainId": domain_id,
                "definitionVersionId": ver_id,
                "metadata": {},
                "contextPayload": {"title": "mixed-business"}
            })
        };
        let token = if marked {
            direct_token(owner, "workflow.execute", &mock.key_pair)
        } else {
            direct_token(creator, "workflow.execute", &mock.key_pair)
        };
        let (status, body) = do_post(app.clone(), "/internal/v1/workflow-instances", &token, body_builder).await;
        assert_eq!(status, 201, "{body}");
        let at = due_instant().await;
        let activation = force_next_eligible_at(
            &pool,
            Uuid::parse_str(body["currentNodeVisitId"].as_str().unwrap()).unwrap(),
            at,
        )
        .await;
        activations.push((activation, marked));
    }

    let scheduler = seed_agent(&pool).await;
    seed_global_role(&pool, scheduler, "GLOBAL_SCHEDULER_READ").await;
    let (_, fbody) = do_get(
        app,
        "/internal/v1/dispatch-intents?limit=100",
        &direct_token(scheduler, "workflow.read", &mock.key_pair),
    )
    .await;
    let records = fbody["items"].as_array().cloned().unwrap_or_default();
    let ids: Vec<String> = records
        .iter()
        .map(|r| r["dispatchIntentId"].as_str().unwrap().to_string())
        .collect();
    let business_ids: Vec<String> = activations
        .iter()
        .filter(|(_, m)| !m)
        .map(|(a, _)| a.to_string())
        .collect();
    // Both BUSINESS rows present, in eligibility order, marked row absent.
    let pos: Vec<usize> = business_ids
        .iter()
        .map(|id| ids.iter().position(|x| x == id).expect("business row in feed"))
        .collect();
    assert!(
        pos.len() == 2 && pos[0] < pos[1],
        "T5j: BUSINESS ordering preserved in a class-mixed window (pos={pos:?})"
    );
    assert!(
        !ids.contains(&activations[2].0.to_string()),
        "T5j: marked row absent while BUSINESS rows flow normally"
    );
}

// ============================================================================
// T5e / T5f / T5k — denial family, replay, scope; idempotency conflicts
// ============================================================================

#[tokio::test]
async fn t5f_member_marking_denied_zero_facts_and_deterministic_replay() {
    let pool = common::create_pool().await;
    let (_owner, domain_id, _creator, ver_id) = seed_fixture(&pool).await;
    // A second agent with ONLY the member binding: the suppression path the
    // marking authority must close (create work for another principal, then
    // mark it out of the dispatcher).
    let member = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, member).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);

    let body = json!({
        "domainId": domain_id,
        "definitionVersionId": ver_id,
        "executionClass": "NON_BUSINESS_TEST",
        "metadata": {},
        "contextPayload": {"title": "member-mark"}
    });
    // Fixed idempotency key so the retry below exercises receipt replay.
    let token = direct_token(member, "workflow.execute", &mock.key_pair);
    let req = Request::builder()
        .method("POST")
        .uri("/internal/v1/workflow-instances")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("idempotency-key", "wec-member-mark-fixed-ik")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status().as_u16(), 403, "member marking denied");
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let denied: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(denied["error"]["code"].as_str(), Some("not_domain_owner"));

    let count_check: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_command_receipts WHERE principal_id = $1",
    )
    .bind(member)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);
    assert!(count_check >= 1, "failure receipt persisted for replay");

    // Same IK + identical rejected body → deterministic replay (still 403,
    // no second receipt, no runtime facts).
    let req2 = Request::builder()
        .method("POST")
        .uri("/internal/v1/workflow-instances")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("idempotency-key", "wec-member-mark-fixed-ik")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let resp2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(resp2.status().as_u16(), 403, "replayed deterministic failure");
    let bytes2 = to_bytes(resp2.into_body(), usize::MAX).await.unwrap();
    let replayed: Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(replayed["error"]["code"].as_str(), Some("not_domain_owner"));

    // Zero instances were created by either attempt.
    let instances: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_instances WHERE created_by_principal_id = $1",
    )
    .bind(member)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(instances, 0, "T5f: zero runtime facts on rejection");
}

#[tokio::test]
async fn t5e_unknown_class_value_422_invalid_input() {
    let pool = common::create_pool().await;
    let (_owner, domain_id, creator, ver_id) = seed_fixture(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);

    let (status, body) = do_post(
        app,
        "/internal/v1/workflow-instances",
        &direct_token(creator, "workflow.execute", &mock.key_pair),
        json!({
            "domainId": domain_id,
            "definitionVersionId": ver_id,
            "executionClass": "SOMETHING_ELSE",
            "metadata": {},
            "contextPayload": {}
        }),
    )
    .await;
    assert_eq!(status, 422, "handler-side closed enum check: {body}");
    assert_eq!(body["error"]["code"].as_str(), Some("invalid_input"));
}

#[tokio::test]
async fn t5k_other_domain_owner_cannot_mark() {
    let pool = common::create_pool().await;
    // The actor is a member of domain A but holds its DOMAIN_OWNER binding
    // NOWHERE; their single owner binding is on domain B. Membership on A
    // passes; the marking authority (owner of THE TARGET domain) must deny.
    let (_human_b, domain_b) = common::seed_principal_and_domain(&pool).await;
    let (_owner_a, domain_a) = common::seed_principal_domain_with_owner(&pool).await;
    let actor = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_a, actor).await;
    // The actor's ONLY owner binding is on domain B (single-owner invariant
    // respected: domain B has exactly this one enabled owner).
    seed_domain_owner_binding(&pool, domain_b, actor).await;
    let creator_a = seed_agent(&pool).await;
    let ver_a = seed_v1_definition(&pool, domain_a, creator_a).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);

    let (status, body) = do_post(
        app,
        "/internal/v1/workflow-instances",
        &direct_token(actor, "workflow.execute", &mock.key_pair),
        json!({
            "domainId": domain_a,
            "definitionVersionId": ver_a,
            "executionClass": "NON_BUSINESS_TEST",
            "metadata": {},
            "contextPayload": {}
        }),
    )
    .await;
    assert_eq!(status, 403, "T5k: domain binding is the authority scope: {body}");
    assert_eq!(body["error"]["code"].as_str(), Some("not_domain_owner"));
}

#[tokio::test]
async fn idempotency_same_ik_different_class_is_conflict_not_replay() {
    let pool = common::create_pool().await;
    let (_owner, domain_id, creator, ver_id) = seed_fixture(&pool).await;
    let admission = svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled();
    let fixed_ik = format!("wec-conflict-{}", Uuid::new_v4());

    let first = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool,
        admission.clone(),
        create_instance_command(
            creator,
            domain_id,
            ver_id,
            WorkflowExecutionClass::Business,
            fixed_ik.clone(),
        )
        .await,
    )
    .await
    .expect("business create succeeds");

    let second = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool,
        admission,
        create_instance_command(
            creator,
            domain_id,
            ver_id,
            WorkflowExecutionClass::NonBusinessTest,
            fixed_ik.clone(),
        )
        .await,
    )
    .await
    .expect_err("same IK + different executionClass must conflict");

    match second {
        svc_workflow::domain::workflow_instance::errors::CreateWorkflowInstanceError::IdempotencyConflict {
            original_command_id,
            ..
        } => {
            assert_ne!(original_command_id, Uuid::nil());
        }
        other => panic!(
            "expected IdempotencyConflict, got {other:?} — class must participate in request identity"
        ),
    }
}

#[tokio::test]
async fn t5c_unmarked_business_default_unchanged() {
    let pool = common::create_pool().await;
    let (_owner, domain_id, creator, ver_id) = seed_fixture(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);

    let (status, body) = do_post(
        app.clone(),
        "/internal/v1/workflow-instances",
        &direct_token(creator, "workflow.execute", &mock.key_pair),
        json!({
            "domainId": domain_id,
            "definitionVersionId": ver_id,
            "metadata": {},
            "contextPayload": {"title": "default"}
        }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    let class: String = sqlx::query_scalar(
        "SELECT execution_class::text FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(Uuid::parse_str(body["workflowInstanceId"].as_str().unwrap()).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(class, "BUSINESS", "T5c: absent class = BUSINESS, byte-identical legacy path");
}
