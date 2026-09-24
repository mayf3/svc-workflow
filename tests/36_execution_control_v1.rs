//! Integration tests for WORKFLOW_EXECUTION_CONTROL_V1
//! (SVC_WORKFLOW_EXECUTION_CONTROL_V1, CTR-SWEC-001..006).
//!
//! Goal §12 mapping (svc side):
//! - Case 1: create queues the execution kick + forum binding/event facts
//! - Case 4: system escalation endpoint → HUMAN_REQUIRED case; the escalated
//!   visit leaves the due feed; idempotent replay (escalated=false)
//! - Case 5: RETURN commits are counted from workflow_events and projected
//! - Case 6: the limit-reaching RETURN escalates in-tx; further RETURNs fail
//!   closed (assistance_open while the case is open, return_policy_exhausted
//!   after resolution)
//! - Case 8: nothing here calls forum — rows are only queued (business tx
//!   succeeds with forum sync dormant)
//! - Case 10 support: outbox event_key uniqueness (single canonical row)

#![allow(clippy::needless_borrow)]
#![allow(unused_imports, unused_variables)]

#[path = "common/mod.rs"]
mod common;

use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
};
use svc_workflow::domain::workflow_instance::errors::ExecuteWorkflowTransitionError;
use svc_workflow::http::{self, AppState, ExecutionControlConfig, HttpConfig};
use svc_workflow::store::postgres::admission_gate::AdmissionGate;

// ============================================================================
// Harness (mirrors tests/28/29)
// ============================================================================

fn build_app_with_policy(
    pool: sqlx::PgPool,
    jwks_url: &str,
    max_returns_per_edge: u32,
) -> axum::Router {
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
        execution_control: ExecutionControlConfig {
            max_returns_per_edge,
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

async fn do_post(
    app: axum::Router,
    path: &str,
    token: &str,
    body: Value,
    idem_key: &str,
) -> (u16, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", idem_key)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
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

/// VISIT_ACTIVATION_V1 published definition (test-28 shape):
/// TASK start (WORKFLOW_CREATOR) → TASK work (FIXED_PRINCIPAL=agent) →
/// TERMINAL done, plus RETURN work→start.
async fn seed_v1_definition(
    pool: &PgPool,
    domain_id: Uuid,
    agent_id: Uuid,
) -> (Uuid, Uuid, Uuid, Uuid) {
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

async fn create_instance(
    pool: &PgPool,
    creator_id: Uuid,
    domain_id: Uuid,
    ver_id: Uuid,
) -> (Uuid, Uuid, i32) {
    let result = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        pool,
        AdmissionGate::disabled(),
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
            context_payload: serde_json::json!({ "title": "wec" }),
        },
    )
    .await
    .expect("create instance should succeed");
    (
        result.workflow_instance_id,
        result.current_node_visit_id,
        result.workflow_state_version,
    )
}

async fn transition(
    pool: &PgPool,
    actor_id: Uuid,
    instance_id: Uuid,
    expected_version: i32,
    transition_id: Uuid,
    max_returns: u32,
) -> Result<
    svc_workflow::application::workflow_instance::execute_transition::ExecuteWorkflowTransitionResult,
    ExecuteWorkflowTransitionError,
>{
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition_with_policy(
        pool,
        AdmissionGate::disabled(),
        ExecuteWorkflowTransitionCommand {
            principal_id: PrincipalId::from_uuid(actor_id),
            idempotency_key: format!("transition-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: expected_version,
            transition_definition_id: TransitionId::from_uuid(transition_id),
            submission_payload: None,
        },
        max_returns,
    )
    .await
}

/// Collect the FULL due feed via keyset continuation (the shared test DB
/// holds many stale due intents; a single default page is not enough).
async fn collect_all_due(app: axum::Router, token: &str) -> Value {
    let mut all: Vec<Value> = Vec::new();
    let mut after_ts: Option<String> = None;
    let mut after_id: Option<String> = None;
    loop {
        let mut path = "/internal/v1/dispatch-intents?limit=100".to_string();
        if let (Some(ts), Some(id)) = (&after_ts, &after_id) {
            path.push_str(&format!(
                "&afterNextEligibleAt={ts}&afterDispatchIntentId={id}"
            ));
        }
        let (status, body) = do_get(app.clone(), &path, token).await;
        assert_eq!(status, 200, "due feed page: {body}");
        let items = body["items"].as_array().expect("items").clone();
        let exhausted = items.len() < 100;
        if let Some(last) = items.last() {
            after_ts = Some(last["nextEligibleAt"].as_str().unwrap().to_string());
            after_id = Some(last["dispatchIntentId"].as_str().unwrap().to_string());
        }
        all.extend(items);
        if exhausted {
            return Value::Array(all);
        }
    }
}

async fn outbox_rows(pool: &PgPool, instance_id: Uuid, kind: &str) -> Vec<(String, Value)> {
    sqlx::query_as(
        "SELECT event_key, payload FROM workflow_outbox WHERE workflow_instance_id = $1 AND outbox_kind = $2 ORDER BY created_at",
    )
    .bind(instance_id)
    .bind(kind)
    .fetch_all(pool)
    .await
    .expect("outbox rows")
}

// ============================================================================
// ACC-WEC1-001/002 (Goal Case 1 + Case 8-dormant): creation facts
// ============================================================================

#[tokio::test]
async fn acc01_create_queues_binding_forum_event_and_kick() {
    let pool = common::create_pool().await;
    let (_owner, domain_id) = common::seed_principal_and_domain(&pool).await;
    // The creator is an AGENT principal: an agent-owned entry visit creates a
    // DISPATCH_INTENT activation (kick-eligible). A HUMAN creator would get a
    // HUMAN_WORK_ITEM — deliberately never kicked.
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let agent = seed_agent(&pool).await;
    let (ver_id, _start, _work, _ret) = seed_v1_definition(&pool, domain_id, agent).await;
    let (instance, visit, _version) = create_instance(&pool, creator, domain_id, ver_id).await;

    // Canonical binding fact, PENDING until the reconciler resolves the thread.
    let binding: (String,) = sqlx::query_as(
        "SELECT binding_state::text FROM workflow_forum_bindings WHERE workflow_instance_id = $1",
    )
    .bind(instance)
    .fetch_one(&pool)
    .await
    .expect("binding row");
    assert_eq!(binding.0, "PENDING");

    // workflow_created forum event, exactly one.
    let forum_events = outbox_rows(&pool, instance, "FORUM_EVENT").await;
    let created: Vec<_> = forum_events
        .iter()
        .filter(|(k, _)| k.starts_with("wf_created:"))
        .collect();
    assert_eq!(created.len(), 1, "exactly one wf_created row");
    assert_eq!(created[0].1["eventType"], "workflow_created");

    // The first activation is a DISPATCH_INTENT (AGENT creator) → kick row.
    let activation_id: Uuid = sqlx::query_scalar(
        "SELECT activation_id FROM workflow_activations WHERE node_visit_id = $1",
    )
    .bind(visit)
    .fetch_one(&pool)
    .await
    .expect("activation");
    let kicks = outbox_rows(&pool, instance, "EXECUTION_KICK").await;
    assert_eq!(kicks.len(), 1, "exactly one kick row");
    assert_eq!(kicks[0].0, format!("kick:{activation_id}"));
    assert_eq!(kicks[0].1["nodeVisitId"], json!(visit));
}

// ============================================================================
// ACC-WEC1-003 (Goal Case 5): transition events projected
// ============================================================================

#[tokio::test]
async fn acc03_advance_queues_transition_committed_event() {
    let pool = common::create_pool().await;
    let (creator, domain_id) = common::seed_principal_and_domain(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let agent = seed_agent(&pool).await;
    let (ver_id, _start, _work, _ret) = seed_v1_definition(&pool, domain_id, agent).await;
    let (instance, _visit, version) = create_instance(&pool, creator, domain_id, ver_id).await;

    let t1: Uuid = sqlx::query_scalar(
        "SELECT transition_id FROM workflow_transition_definitions WHERE definition_version_id = $1 AND transition_key = 'advance-1'",
    )
    .bind(ver_id)
    .fetch_one(&pool)
    .await
    .expect("advance-1");

    let result = transition(&pool, creator, instance, version, t1, 3)
        .await
        .expect("advance");
    assert_eq!(result.workflow_state_version, 2);
    assert!(
        result.assistance_case_id.is_none(),
        "ADVANCE never escalates"
    );

    let forum_events = outbox_rows(&pool, instance, "FORUM_EVENT").await;
    let committed: Vec<&(String, Value)> = forum_events
        .iter()
        .filter(|(k, p)| k.starts_with("transition_committed:") && p["effect"] == "ADVANCE")
        .collect();
    assert_eq!(committed.len(), 1, "exactly one transition_committed row");
    assert!(committed[0].1["returnCountPerEdge"].is_null());
}

// ============================================================================
// ACC-WEC1-004 (Goal Case 6): RETURN limit escalates then blocks
// ============================================================================

#[tokio::test]
async fn acc04_return_limit_escalates_then_blocks() {
    let pool = common::create_pool().await;
    let (creator, domain_id) = common::seed_principal_and_domain(&pool).await;
    // The creator must also own the domain so it can resolve the case later.
    common::seed_domain_owner(&pool, domain_id, creator).await;
    let agent = seed_agent(&pool).await;
    let (ver_id, _start, _work, ret) = seed_v1_definition(&pool, domain_id, agent).await;
    let (instance, _visit, version) = create_instance(&pool, creator, domain_id, ver_id).await;

    let t1: Uuid = sqlx::query_scalar(
        "SELECT transition_id FROM workflow_transition_definitions WHERE definition_version_id = $1 AND transition_key = 'advance-1'",
    )
    .bind(ver_id)
    .fetch_one(&pool)
    .await
    .expect("advance-1");

    // start -> work (creator)
    let r1 = transition(&pool, creator, instance, version, t1, 1)
        .await
        .expect("advance");
    assert_eq!(r1.workflow_state_version, 2);

    // RETURN #1 (agent): count+1 == max(1) → commits AND escalates the target
    // visit (start) in the same transaction. Version bumps twice (transition
    // v3 + escalation v4); the receipt reports the FINAL version.
    let r2 = transition(&pool, agent, instance, 2, ret, 1)
        .await
        .expect("return #1");
    assert_eq!(
        r2.workflow_state_version, 4,
        "transition v3 + escalation v4"
    );
    assert_eq!(r2.event_sequence, 4);
    let case_id = r2
        .assistance_case_id
        .expect("limit-reaching RETURN escalates");

    // The escalated visit carries an open HUMAN_REQUIRED case.
    let status: (String,) = sqlx::query_as(
        "SELECT status::text FROM workflow_assistance_cases WHERE assistance_case_id = $1",
    )
    .bind(case_id)
    .fetch_one(&pool)
    .await
    .expect("case");
    assert_eq!(status.0, "HUMAN_REQUIRED");

    // The instance moved back to start (creator's visit); further transitions
    // are fail-closed by the open case (Goal Case 6: the loop STOPS).
    let blocked = transition(&pool, creator, instance, 4, t1, 1).await;
    match blocked {
        Err(ExecuteWorkflowTransitionError::AssistanceOpen) => {}
        other => panic!("expected AssistanceOpen, got {other:?}"),
    }

    // The owner resolves the case; a further RETURN now hits the hard policy
    // refusal (deterministic 409 return_policy_exhausted).
    let resolve = svc_workflow::application::workflow_instance::assistance::resolve_assistance(
        &pool,
        svc_workflow::domain::workflow_instance::assistance::ResolveAssistanceCommand {
            principal_id: PrincipalId::from_uuid(creator),
            idempotency_key: format!("resolve-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            assistance_case_id: AssistanceCaseId::from_uuid(case_id),
            expected_workflow_state_version: 4,
            resolution: svc_workflow::domain::workflow_instance::assistance::AssistancePayload {
                message: "human took over".to_string(),
                supporting_payload: None,
            },
        },
    )
    .await
    .expect("resolve");
    assert_eq!(
        resolve.status,
        svc_workflow::domain::workflow_instance::assistance::AssistanceCaseStatus::Resolved
    );

    // After resolution the loop may proceed one step (advance to work, v6);
    // the NEXT RETURN on the same edge then hits the hard policy refusal
    // (count 1 >= max 1): deterministic 409 semantics, zero side effects.
    let advance2 = transition(&pool, creator, instance, 5, t1, 1)
        .await
        .expect("advance after resolve");
    assert_eq!(advance2.workflow_state_version, 6);
    let blocked2 = transition(&pool, agent, instance, 6, ret, 1).await;
    match blocked2 {
        Err(ExecuteWorkflowTransitionError::ReturnPolicyExhausted { limit }) => {
            assert_eq!(limit, 1)
        }
        other => panic!("expected ReturnPolicyExhausted, got {other:?}"),
    }

    // Forum projection rows for the escalation (Goal Scope G).
    let forum_events = outbox_rows(&pool, instance, "FORUM_EVENT").await;
    assert!(forum_events
        .iter()
        .any(|(k, p)| k.starts_with("return_policy_reached:")
            && p["reason"] == "RETURN_POLICY_EXHAUSTED"));
    assert!(forum_events
        .iter()
        .any(|(_, p)| p["eventType"] == "return_policy_reached"));
}

// ============================================================================
// ACC-WEC1-005 (Goal Case 4): system escalation ingress + due-feed narrowing
// ============================================================================

#[tokio::test]
async fn acc05_escalation_endpoint_gates_idempotency_and_due_feed() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app_with_policy(pool.clone(), &mock.url, 3);

    let (_owner, domain_id) = common::seed_principal_and_domain(&pool).await;
    // AGENT creator ⇒ the entry visit owns a DISPATCH_INTENT (due-feedable).
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let agent = seed_agent(&pool).await;
    let poller = seed_agent(&pool).await;
    let (ver_id, _start, _work, _ret) = seed_v1_definition(&pool, domain_id, agent).await;
    let (instance, visit, _version) = create_instance(&pool, creator, domain_id, ver_id).await;

    let poller_token = direct_token(poller, "workflow.execute", &mock.key_pair);
    let poller_read = direct_token(poller, "workflow.read", &mock.key_pair);

    // Escalation WITHOUT the GLOBAL_SCHEDULER_READ binding → 403.
    let path = format!("/internal/v1/workflow-instances/{instance}/execution-escalations");
    let (status, _body) = do_post(
        app.clone(),
        &path,
        &poller_token,
        json!({ "nodeVisitId": visit, "reason": "ATTEMPTS_EXHAUSTED", "attemptCount": 3 }),
        &format!("esc-denied-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(
        status, 403,
        "the ingress shares the wake gate (fail-closed)"
    );

    // Grant the role; due feed BEFORE escalation offers the visit.
    grant_scheduler_read(&pool, poller).await;
    let body = collect_all_due(app.clone(), &poller_read).await;
    let offered = body
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["nodeVisitId"] == json!(visit));
    assert!(
        offered,
        "the fresh intent is due before escalation; {} items seen",
        body.as_array().unwrap().len()
    );

    // Escalation WITH the role → 200 escalated=true.
    let (status, body) = do_post(
        app.clone(),
        &path,
        &poller_token,
        json!({ "nodeVisitId": visit, "reason": "ATTEMPTS_EXHAUSTED", "attemptCount": 3 }),
        &format!("esc-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(status, 200, "escalation body: {body}");
    assert_eq!(body["escalated"], json!(true));
    let case_id = body["assistanceCaseId"].as_str().unwrap().to_string();

    // The case exists in HUMAN_REQUIRED.
    let status_row: (String,) = sqlx::query_as(
        "SELECT status::text FROM workflow_assistance_cases WHERE assistance_case_id = $1",
    )
    .bind(Uuid::parse_str(&case_id).unwrap())
    .fetch_one(&pool)
    .await
    .expect("case row");
    assert_eq!(status_row.0, "HUMAN_REQUIRED");

    // Due feed AFTER escalation: the visit is GONE (CTR-SWEC-006 narrowing).
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/dispatch-intents?limit=100",
        &poller_read,
    )
    .await;
    assert_eq!(status, 200);
    let offered = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["nodeVisitId"] == json!(visit));
    assert!(!offered, "the escalated visit must leave the due feed");

    // Idempotent replay: a NEW idempotency key on the SAME visit answers
    // escalated=false with the SAME case id (no second case).
    let (status, body2) = do_post(
        app.clone(),
        &path,
        &poller_token,
        json!({ "nodeVisitId": visit, "reason": "ATTEMPTS_EXHAUSTED", "attemptCount": 3 }),
        &format!("esc-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body2["escalated"], json!(false));
    assert_eq!(body2["assistanceCaseId"], json!(case_id));

    // Invalid reason → 422.
    let (status, _body) = do_post(
        app.clone(),
        &path,
        &poller_token,
        json!({ "nodeVisitId": visit, "reason": "JUST_BECAUSE" }),
        &format!("esc-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(status, 422);
}

// ============================================================================
// ACC-WEC1-006 (Case 10 support): outbox event_key uniqueness
// ============================================================================

#[tokio::test]
async fn acc06_outbox_event_key_is_unique() {
    let pool = common::create_pool().await;
    let (creator, domain_id) = common::seed_principal_and_domain(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let agent = seed_agent(&pool).await;
    let (ver_id, _start, _work, _ret) = seed_v1_definition(&pool, domain_id, agent).await;
    let (instance, _visit, _version) = create_instance(&pool, creator, domain_id, ver_id).await;

    let key = format!("wf_created:{instance}");
    let mut tx = pool.begin().await.unwrap();
    svc_workflow::store::postgres::outbox::queue_forum_event(
        &mut tx,
        instance,
        &key,
        json!({ "eventType": "workflow_created" }),
    )
    .await
    .unwrap();
    // A duplicate queue (e.g. a concurrent reconciler) is a no-op.
    svc_workflow::store::postgres::outbox::queue_forum_event(
        &mut tx,
        instance,
        &key,
        json!({ "eventType": "workflow_created" }),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let rows = outbox_rows(&pool, instance, "FORUM_EVENT").await;
    let matches: Vec<_> = rows.iter().filter(|(k, _)| *k == key).collect();
    assert_eq!(matches.len(), 1, "exactly one canonical row per event_key");
}
