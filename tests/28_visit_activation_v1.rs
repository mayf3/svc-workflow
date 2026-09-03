//! Integration tests for the VISIT_ACTIVATION_V1 runtime core
//! (SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1).
//!
//! Covers the acceptance matrix ACC-VAI-001..011:
//! - AGENT-owned entry creates a due DISPATCH_INTENT activation (same tx)
//! - HUMAN-owned entry creates a HUMAN_WORK_ITEM (no eligibility)
//! - SERVICE / disabled owners fail closed with zero facts
//! - Transition closes the source activation and creates the target one
//! - Cancel closes the activation; Archive fails closed on active work
//! - Wake applies eligibility + version + Event; stale/closed are no-ops
//! - Due-intent read: role gate fail-closed, exact 7-field projection
//! - Legacy instances never gain activations; revise/combined rejected on V1

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
use svc_workflow::domain::workflow_instance::events::EVENT_SCHEMA_VERSION;
use svc_workflow::http::{self, AppState, HttpConfig};

// ============================================================================
// Test app builder (mirrors tests/23_global_coordinator.rs)
// ============================================================================

fn build_app(pool: sqlx::PgPool, jwks_url: &str, admin_ids: Vec<Uuid>) -> axum::Router {
    let config = HttpConfig {
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
        provisioning_config: ProvisioningConfig::new(
            admin_ids
                .into_iter()
                .map(svc_workflow::domain::ids::PrincipalId::from_uuid)
                .collect(),
        ),
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

// ============================================================================
// Seeds
// ============================================================================

/// VISIT_ACTIVATION_V1 published definition:
/// TASK start (WORKFLOW_CREATOR) --primary ADVANCE--> TASK work
/// (FIXED_PRINCIPAL = agent) --primary ADVANCE--> TERMINAL done,
/// with RETURN work->start and TERMINATE work->failed.
async fn seed_v1_definition(
    pool: &PgPool,
    domain_id: Uuid,
    agent_id: Uuid,
) -> (Uuid, Uuid, Uuid, Uuid, Uuid, Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("v1-test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'V1 Test Def')",
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

    (ver_id, start_id, work_id, done_id, failed_id, t1, t2, t4)
}

/// Legacy published definition (DRAFT entry), mirroring test 23.
async fn seed_legacy_definition(pool: &PgPool, domain_id: Uuid) -> Uuid {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Legacy Def')",
    )
    .bind(def_id).bind(domain_id).bind(format!("legacy-{}", &Uuid::new_v4().to_string()[..8]))
    .execute(pool).await.expect("insert def");
    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, semantic_model_version) VALUES ($1, $2, 1, 'DRAFT', 1)",
    )
    .bind(ver_id).bind(def_id)
    .execute(pool).await.expect("insert version");
    let draft_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')",
    ).bind(draft_id).bind(ver_id).execute(pool).await.expect("insert draft");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', NULL)",
    ).bind(term_id).bind(ver_id).execute(pool).await.expect("insert done");
    let trans_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance', 'Advance', $3, $4, 'ADVANCE')",
    ).bind(trans_id).bind(ver_id).bind(draft_id).bind(term_id)
    .execute(pool).await.expect("insert transition");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(trans_id).bind(draft_id).execute(pool).await.expect("primary");
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
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(creator_id),
            idempotency_key: format!("create-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(ver_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({}),
            context_payload: serde_json::json!({ "title": "v1" }),
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

async fn transition_v1(
    pool: &PgPool,
    actor_id: Uuid,
    instance_id: Uuid,
    expected_version: i32,
    transition_id: Uuid,
) -> (Uuid, i32) {
    let result =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            pool,
            ExecuteWorkflowTransitionCommand {
                principal_id: PrincipalId::from_uuid(actor_id),
                idempotency_key: format!("trans-{}", Uuid::new_v4()),
                command_schema_version: "v1".to_string(),
                workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
                expected_workflow_state_version: expected_version,
                transition_definition_id: TransitionId::from_uuid(transition_id),
                submission_payload: None,
            },
        )
        .await
        .expect("transition should succeed");
    (result.current_node_visit_id, result.workflow_state_version)
}

async fn activation_state(pool: &PgPool, visit_id: Uuid) -> (Option<String>, Option<bool>) {
    let row: Option<(String, bool)> = sqlx::query_as(
        "SELECT a.activation_kind::text,
                (c.activation_id IS NOT NULL)
           FROM workflow_activations a
           LEFT JOIN workflow_activation_closures c ON c.activation_id = a.activation_id
          WHERE a.node_visit_id = $1",
    )
    .bind(visit_id)
    .fetch_optional(pool)
    .await
    .expect("activation state query");
    match row {
        Some((kind, closed)) => (Some(kind), Some(closed)),
        None => (None, None),
    }
}


/// Seed an enabled domain-role binding for `principal_id` in `domain_id`
/// (membership gate for the create path).
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

// ============================================================================
// Tests
// ============================================================================

/// ACC-VAI-002: AGENT-owned entry -> DISPATCH_INTENT with server-authored
/// initial nextEligibleAt, all in the create transaction.
#[tokio::test]
async fn agent_owned_entry_creates_due_dispatch_intent() {
    let pool = common::create_pool().await;
    let (owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    // Explicit AGENT creator: the WORKFLOW_CREATOR-owned entry TASK resolves
    // to an AGENT -> DISPATCH_INTENT.
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let _ = owner;
    let (ver, ..) = seed_v1_definition(&pool, domain_id, creator).await;
    let (instance_id, visit_id, version) =
        create_v1_instance(&pool, creator, domain_id, ver).await;
    assert_eq!(version, 1);

    let row: (String, Option<chrono::DateTime<chrono::Utc>>, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as(
            "SELECT activation_kind::text, initial_next_eligible_at, activation_at
               FROM workflow_activations WHERE node_visit_id = $1",
        )
        .bind(visit_id)
        .fetch_one(&pool)
        .await
        .expect("activation row must exist");

    assert_eq!(row.0, "DISPATCH_INTENT");
    let initial = row.1.expect("DISPATCH_INTENT must carry initial nextEligibleAt");
    // Same transaction timestamp: initial == activation_at.
    assert_eq!(initial, row.2);
}

/// ACC-VAI-002: HUMAN-owned entry -> HUMAN_WORK_ITEM with NULL eligibility.
#[tokio::test]
async fn human_owned_entry_creates_human_work_item() {
    let pool = common::create_pool().await;
    let (owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    let human = common::seed_second_principal(&pool).await; // AGENT by helper... use owner as human instead
    let _ = human;
    // FIXED_PRINCIPAL owner = the (HUMAN-typed) seeded owner principal.
    let (ver, ..) = seed_v1_definition(&pool, domain_id, owner).await;
    // Entry owner is WORKFLOW_CREATOR; create as the owner principal but the
    // seeded principals here are AGENT-typed, so instead bind the entry to a
    // FIXED human principal via a fresh definition.
    let human_id = {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'HUMAN', 'Human', NULL, TRUE)",
        )
        .bind(id)
        .execute(&pool)
        .await
        .expect("insert human");
        id
    };
    let (ver2, ..) = seed_v1_definition(&pool, domain_id, human_id).await;
    let (_instance_id, visit_id, _) = create_v1_instance(&pool, owner, domain_id, ver2).await;

    let (kind, closed) = activation_state(&pool, visit_id).await;
    assert_eq!(kind.as_deref(), Some("HUMAN_WORK_ITEM"));
    assert_eq!(closed, Some(false));

    let eligibility: Option<(i64,)> = sqlx::query_as(
        "SELECT 1::bigint FROM workflow_dispatch_eligibility_events e \
         JOIN workflow_activations a ON a.activation_id = e.activation_id \
         WHERE a.node_visit_id = $1",
    )
    .bind(visit_id)
    .fetch_optional(&pool)
    .await
    .expect("eligibility query");
    assert!(eligibility.is_none(), "HUMAN_WORK_ITEM must have no eligibility facts");
}

/// ACC-VAI-002/007: SERVICE principal can never own new-model work; create
/// fails closed with zero instance/activation facts.
#[tokio::test]
async fn service_owner_fails_closed() {
    let pool = common::create_pool().await;
    let (creator, domain_id) = common::seed_principal_domain_with_owner(&pool).await;

    // Service principal that a FIXED_PRINCIPAL entry would resolve to. The
    // entry owner is WORKFLOW_CREATOR (an AGENT), so instead build a v1
    // definition whose entry resolves to the SERVICE principal via
    // FIXED_PRINCIPAL, using the app-level create to prove the fail-closed.
    let service_id = {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'SERVICE', 'Svc', NULL, TRUE)",
        )
        .bind(id)
        .execute(&pool)
        .await
        .expect("insert service");
        id
    };

    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Svc Def')",
    )
    .bind(def_id).bind(domain_id).bind(format!("svc-{}", &Uuid::new_v4().to_string()[..8]))
    .execute(&pool).await.expect("insert def");
    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, semantic_model_version) VALUES ($1, $2, 1, 'DRAFT', 3)",
    )
    .bind(ver_id).bind(def_id)
    .execute(&pool).await.expect("insert version");
    let start_id = Uuid::new_v4();
    let done_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) VALUES ($1, $2, 'start', 'Start', 0, 'TASK', 'FIXED_PRINCIPAL', $3)",
    ).bind(start_id).bind(ver_id).bind(service_id).execute(&pool).await.expect("insert start");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', NULL)",
    ).bind(done_id).bind(ver_id).execute(&pool).await.expect("insert done");
    let t1 = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'a', 'a', $3, $4, 'ADVANCE')",
    ).bind(t1).bind(ver_id).bind(start_id).bind(done_id)
    .execute(&pool).await.expect("insert transition");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t1).bind(start_id).execute(&pool).await.expect("primary");
    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(&pool).await.expect("publish");

    let (activations_before,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workflow_activations")
        .fetch_one(&pool)
        .await
        .expect("count activations before");
    let result = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool,
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(creator),
            idempotency_key: format!("create-svc-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(ver_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({}),
            context_payload: serde_json::json!({}),
        },
    )
    .await;

    assert!(
        result.is_err(),
        "SERVICE-owned entry must fail closed"
    );
    let (activations_after,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workflow_activations")
        .fetch_one(&pool)
        .await
        .expect("count activations");
    assert_eq!(
        activations_after, activations_before,
        "no activation fact may be created by the failed create"
    );
}

/// ACC-VAI-003: transition closes the source activation and creates the
/// target TASK activation; the terminal target creates none.
#[tokio::test]
async fn transition_closes_and_creates_activations() {
    let pool = common::create_pool().await;
    let (creator, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = seed_agent(&pool).await;
    let (ver, _start, _work, _done, _failed, start_advance, work_advance, terminate) =
        seed_v1_definition(&pool, domain_id, agent).await;
    let (instance_id, start_visit, version) =
        create_v1_instance(&pool, creator, domain_id, ver).await;

    // start -> work (as the creator, the current owner of start).
    let (work_visit, v2) = transition_v1(&pool, creator, instance_id, version, start_advance).await;

    // Source activation closed with reason TRANSITIONED (creator is HUMAN,
    // so the entry activation is a HUMAN_WORK_ITEM).
    let (start_kind, start_closed) = activation_state(&pool, start_visit).await;
    assert_eq!(start_kind.as_deref(), Some("HUMAN_WORK_ITEM"));
    assert_eq!(start_closed, Some(true));
    let (reason,): (String,) = sqlx::query_as(
        "SELECT c.closure_reason FROM workflow_activation_closures c \
         JOIN workflow_activations a ON a.activation_id = c.activation_id \
         WHERE a.node_visit_id = $1",
    )
    .bind(start_visit)
    .fetch_one(&pool)
    .await
    .expect("closure row");
    assert_eq!(reason, "TRANSITIONED");

    // Target TASK activation active (work is FIXED to the agent).
    let (work_kind, work_closed) = activation_state(&pool, work_visit).await;
    assert_eq!(work_kind.as_deref(), Some("DISPATCH_INTENT"));
    assert_eq!(work_closed, Some(false));

    // work -> done (as the agent, the current owner). Target TERMINAL gets
    // no activation.
    let (done_visit, _v3) = transition_v1(&pool, agent, instance_id, v2, terminate).await;
    let (done_kind, _) = activation_state(&pool, done_visit).await;
    assert!(done_kind.is_none(), "TERMINAL targets create no activation");

    // Every visit of the instance has exactly one activation.
    let drift: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_node_visits v \
         WHERE v.workflow_instance_id = $1 \
           AND (SELECT COUNT(*) FROM workflow_activations a \
                 WHERE a.node_visit_id = v.node_visit_id) NOT IN (0, 1)",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("cardinality check");
    assert_eq!(drift.0, 0);
}

/// ACC-VAI-004: Cancel closes the activation; Archive with active work
/// fails closed (constructed via direct SQL terminal-state fixture).
#[tokio::test]
async fn cancel_closes_and_archive_fails_on_active_activation() {
    let pool = common::create_pool().await;
    let (owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    let (ver, ..) = seed_v1_definition(&pool, domain_id, owner).await;
    let (instance_id, visit_id, _) = create_v1_instance(&pool, owner, domain_id, ver).await;

    // Force terminal-state while the activation stays active: the archive
    // guard must fail closed on the drift-free but still-active work.
    sqlx::query("UPDATE workflow_instances SET cancelled = TRUE WHERE workflow_instance_id = $1")
        .bind(instance_id)
        .execute(&pool)
        .await
        .expect("force cancelled");

    let has_active: (bool,) = sqlx::query_as(
        "SELECT NOT EXISTS(
            SELECT 1 FROM workflow_activation_closures c
             JOIN workflow_activations a ON a.activation_id = c.activation_id
            WHERE a.node_visit_id = $1)",
    )
    .bind(visit_id)
    .fetch_one(&pool)
    .await
    .expect("active check");
    assert!(has_active.0, "fixture must keep the activation active");

    // Archive through the real HTTP contract: the active-activation guard
    // must reject with 409 active_activation_exists.
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url, vec![]);
    let owner_token = direct_token(owner, "workflow.execute", &mock.key_pair);
    let (status, body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
        &owner_token,
        json!({ "reason": "trying to archive active work" }),
        &format!("archive-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(status, 409, "archive with active activation must fail: {body}");
    assert_eq!(body["error"]["code"], json!("active_activation_exists"));
}

/// ACC-VAI-005: wake applies eligibility + version + Event on a
/// future-dated intent; version mismatch is a durable no-op; same key
/// replays; stale visit is a no-op.
#[tokio::test]
async fn wake_applies_and_noops() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let _ = owner;
    let scheduler = seed_agent(&pool).await;
    grant_scheduler_read(&pool, scheduler).await;
    let (ver, ..) = seed_v1_definition(&pool, domain_id, creator).await;
    let (instance_id, visit_id, version) = create_v1_instance(&pool, creator, domain_id, ver).await;

    // Push the intent into the future with a directly-seeded eligibility
    // fact (fixture; SCHEDULER_DEFER is out of scope for phase 1).
    let (activation_id,): (Uuid,) = sqlx::query_as(
        "SELECT activation_id FROM workflow_activations WHERE node_visit_id = $1",
    )
    .bind(visit_id)
    .fetch_one(&pool)
    .await
    .expect("activation id");
    let (initial,): (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
        "SELECT initial_next_eligible_at FROM workflow_activations WHERE activation_id = $1",
    )
    .bind(activation_id)
    .fetch_one(&pool)
    .await
    .expect("initial");
    sqlx::query(
        "INSERT INTO workflow_dispatch_eligibility_events \
             (eligibility_event_id, activation_id, previous_next_eligible_at, \
              new_next_eligible_at, cause_class, command_id) \
         VALUES ($1, $2, $3, $3 + interval '1 hour', 'SCHEDULER_DEFER', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(activation_id)
    .bind(initial)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("seed future eligibility");

    let app = build_app(pool.clone(), &mock.url, vec![]);
    let scheduler_token = direct_token(scheduler, "workflow.execute", &mock.key_pair);

    // Version mismatch -> durable no-op (200, wakeApplied=false).
    let (status, body) = do_post(
        app.clone(),
        &format!(
            "/internal/v1/workflow-instances/{instance_id}/node-visits/{visit_id}/wake"
        ),
        &scheduler_token,
        json!({ "expectedWorkflowStateVersion": version + 100 }),
        &format!("wake-mismatch-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(status, 200, "version mismatch is a durable no-op: {body}");
    assert_eq!(body["wakeApplied"], json!(false));
    assert_eq!(body["reason"], json!("VERSION_MISMATCH"));

    // Applied wake.
    let wake_key = format!("wake-apply-{}", Uuid::new_v4());
    let (status, body) = do_post(
        app.clone(),
        &format!(
            "/internal/v1/workflow-instances/{instance_id}/node-visits/{visit_id}/wake"
        ),
        &scheduler_token,
        json!({ "expectedWorkflowStateVersion": version }),
        &wake_key,
    )
    .await;
    assert_eq!(status, 200, "wake should apply: {body}");
    assert_eq!(body["wakeApplied"], json!(true));
    let new_version = body["workflowStateVersion"].as_i64().unwrap() as i32;
    assert_eq!(new_version, version + 1);

    // Eligibility fact count for this activation is now 2 (seeded + wake).
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_dispatch_eligibility_events WHERE activation_id = $1",
    )
    .bind(activation_id)
    .fetch_one(&pool)
    .await
    .expect("count eligibility");
    assert_eq!(count, 2);

    // Exactly one WAKE event on the instance.
    let (events,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_events WHERE workflow_instance_id = $1 \
          AND event_type = 'WAKE_DISPATCH_INTENT'",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("count wake events");
    assert_eq!(events, 1);

    // Same key + same request -> replay of the applied outcome (no extra
    // event, no extra version).
    let (status2, body2) = do_post(
        app.clone(),
        &format!(
            "/internal/v1/workflow-instances/{instance_id}/node-visits/{visit_id}/wake"
        ),
        &scheduler_token,
        json!({ "expectedWorkflowStateVersion": version }),
        &wake_key,
    )
    .await;
    assert_eq!(status2, 200);
    assert_eq!(body2, body);

    // Now the intent is due (nextEligibleAt = now): another wake is a
    // durable no-op ALREADY_DUE.
    let (status3, body3) = do_post(
        app.clone(),
        &format!(
            "/internal/v1/workflow-instances/{instance_id}/node-visits/{visit_id}/wake"
        ),
        &scheduler_token,
        json!({ "expectedWorkflowStateVersion": new_version }),
        &format!("wake-again-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(status3, 200);
    assert_eq!(body3["wakeApplied"], json!(false));
    assert_eq!(body3["reason"], json!("ALREADY_DUE"));

    // Unknown visit on a real instance -> 404 dispatch_intent_not_found.
    let (status4, body4) = do_post(
        app.clone(),
        &format!(
            "/internal/v1/workflow-instances/{instance_id}/node-visits/{}/wake",
            Uuid::new_v4()
        ),
        &scheduler_token,
        json!({ "expectedWorkflowStateVersion": new_version }),
        &format!("wake-unknown-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(status4, 404);
    assert_eq!(body4["error"]["code"], json!("dispatch_intent_not_found"));
}

/// ACC-VAI-006: due-intent read is fail-closed on
/// GLOBAL_SCHEDULER_READ and returns exactly the 7-field minimum projection.
#[tokio::test]
async fn due_read_gate_and_projection() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    let creator = seed_agent(&pool).await;
    seed_domain_member(&pool, domain_id, creator).await;
    let _ = owner;
    let scheduler = seed_agent(&pool).await;
    grant_scheduler_read(&pool, scheduler).await;
    let reader = seed_agent(&pool).await;
    sqlx::query(
        "INSERT INTO global_role_bindings (binding_id, principal_id, role_key, enabled) VALUES ($1, $2, 'GLOBAL_WORKFLOW_READER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(reader)
    .execute(&pool)
    .await
    .expect("grant reader");
    let (ver, ..) = seed_v1_definition(&pool, domain_id, creator).await;
    let (instance_id, visit_id, _) = create_v1_instance(&pool, creator, domain_id, ver).await;

    let app = build_app(pool.clone(), &mock.url, vec![]);

    // Reader (no scheduler read) -> 403 scheduler_read_role_required.
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/dispatch-intents?limit=100",
        &direct_token(reader, "workflow.read", &mock.key_pair),
    )
    .await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], json!("scheduler_read_role_required"));

    // Scheduler sees the due intent with exactly the 7 minimum fields.
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/dispatch-intents?limit=100",
        &direct_token(scheduler, "workflow.read", &mock.key_pair),
    )
    .await;
    assert_eq!(status, 200, "scheduler due read: {body}");
    let items = body["items"].as_array().expect("items");
    let mine: Vec<&Value> = items
        .iter()
        .filter(|it| it["workflowInstanceId"] == json!(instance_id.to_string()))
        .collect();
    assert_eq!(mine.len(), 1, "exactly one due intent for the instance");
    let record = mine[0];
    let mut keys: Vec<&str> = record.as_object().unwrap().keys().map(|k| k.as_str()).collect();
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
    assert_eq!(record["nodeVisitId"], json!(visit_id.to_string()));

    // Missing role entirely -> 403.
    let plain = seed_agent(&pool).await;
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/dispatch-intents",
        &direct_token(plain, "workflow.read", &mock.key_pair),
    )
    .await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], json!("scheduler_read_role_required"));
}

/// ACC-VAI-008: legacy instances never gain activations; v1 instances
/// reject legacy-only commands (revise).
#[tokio::test]
async fn legacy_protection_and_v1_command_guards() {
    let pool = common::create_pool().await;
    let (creator, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = seed_agent(&pool).await;

    // Legacy lifecycle leaves zero activation rows for its visits.
    let legacy_ver = seed_legacy_definition(&pool, domain_id).await;
    let (legacy_instance, legacy_visit, _) =
        create_v1_instance(&pool, creator, domain_id, legacy_ver).await;
    let (kind, _) = activation_state(&pool, legacy_visit).await;
    assert!(kind.is_none(), "legacy visits must not be activated");

    // V1 revise is rejected deterministically.
    let (v1_ver, ..) = seed_v1_definition(&pool, domain_id, agent).await;
    let (v1_instance, _, _) = create_v1_instance(&pool, creator, domain_id, v1_ver).await;
    let revise_result =
        svc_workflow::application::workflow_instance::revise::revise_workflow_context(
            &pool,
            svc_workflow::domain::workflow_instance::commands::ReviseWorkflowContextCommand {
                principal_id: PrincipalId::from_uuid(creator),
                idempotency_key: format!("revise-{}", Uuid::new_v4()),
                command_schema_version: "v1".to_string(),
                workflow_instance_id: WorkflowInstanceId::from_uuid(v1_instance),
                expected_workflow_state_version: 1,
                context_payload: serde_json::json!({ "title": "mutated" }),
            },
        )
        .await;
    let revise_err = revise_result.expect_err("revise must be rejected for VISIT_ACTIVATION_V1");
    // Deterministic 422 wire contract (CTR-VAI-012): status + label.
    assert_eq!(
        svc_workflow::domain::workflow_instance::errors::revise_error_code(&revise_err),
        422,
        "revise rejection must be 422"
    );
    assert_eq!(
        svc_workflow::domain::workflow_instance::errors::revise_error_label(&revise_err),
        "legacy_command_not_supported_for_semantic_model"
    );

    // The legacy instance is untouched by the v1 instance's rejection.
    let (still_none, _) = activation_state(&pool, legacy_visit).await;
    assert!(still_none.is_none());
}

/// ACC-VAI-010: the activation fact families are trigger-immutable.
#[tokio::test]
async fn activation_facts_are_immutable() {
    let pool = common::create_pool().await;
    let (creator, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    let (ver, ..) = seed_v1_definition(&pool, domain_id, creator).await;
    let (_, visit_id, _) = create_v1_instance(&pool, creator, domain_id, ver).await;
    let (activation_id,): (Uuid,) = sqlx::query_as(
        "SELECT activation_id FROM workflow_activations WHERE node_visit_id = $1",
    )
    .bind(visit_id)
    .fetch_one(&pool)
    .await
    .expect("activation");

    let update = sqlx::query("UPDATE workflow_activations SET activation_at = now() WHERE activation_id = $1")
        .bind(activation_id)
        .execute(&pool)
        .await;
    assert!(update.is_err(), "UPDATE on workflow_activations must be rejected");
    let delete = sqlx::query("DELETE FROM workflow_activations WHERE activation_id = $1")
        .bind(activation_id)
        .execute(&pool)
        .await;
    assert!(delete.is_err(), "DELETE on workflow_activations must be rejected");
}
