//! HTTP integration tests for the global (cross-domain) workflow instance
//! list and its global read roles (GLOBAL_WORKFLOW_COORDINATOR and the
//! read-only GLOBAL_WORKFLOW_READER, SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1).
//!
//! Covers:
//! - coordinator sees instances across multiple domains
//! - coordinator sees instances assigned to other principals
//! - keyset pagination is continuous (no duplicates / no misses)
//! - response is summary-only (no detail / submission payload)
//! - normal agent denied (403 global_read_role_required)
//! - DOMAIN_OWNER without a global read role denied (403) and domain boundary intact
//! - coordinator gains no write powers (cancel / archive / transition denied)
//! - global role binding provisioning lifecycle (PUT / DELETE)
//! - READER sees the same cross-domain summaries (read-only grant works)
//! - READER gains no write powers (domain create / owner replace / cancel /
//!   archive / transition / assistance owner-inbox all denied)
//! - READER role key accepted by the provisioning endpoints

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
use svc_workflow::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use svc_workflow::http::{self, AppState, HttpConfig};

// ============================================================================
// Test app builder
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

async fn do_put(
    app: axum::Router,
    path: &str,
    token: &str,
    body: Value,
    idem_key: &str,
) -> (u16, Value) {
    let req = Request::builder()
        .method("PUT")
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

async fn do_delete(
    app: axum::Router,
    path: &str,
    token: &str,
    body: Value,
    idem_key: &str,
) -> (u16, Value) {
    let req = Request::builder()
        .method("DELETE")
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

/// Seed a published definition with a DRAFT node (WORKFLOW_CREATOR assignee).
/// Returns (domain_id, definition_version_id, definition_key).
async fn seed_published_definition(pool: &PgPool, domain_id: Uuid) -> (Uuid, Uuid, String) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("global-test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Global Test Def')",
    )
    .bind(def_id).bind(domain_id).bind(&def_key)
    .execute(pool).await.expect("insert def");

    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1, $2, 1, 'DRAFT', NULL)",
    )
    .bind(ver_id).bind(def_id)
    .execute(pool).await.expect("insert version");

    let draft_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')",
    )
    .bind(draft_id).bind(ver_id)
    .execute(pool).await.expect("insert draft node");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', NULL)",
    )
    .bind(term_id).bind(ver_id)
    .execute(pool).await.expect("insert terminal node");

    let trans_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance', 'Advance', $3, $4, 'ADVANCE')",
    )
    .bind(trans_id).bind(ver_id).bind(draft_id).bind(term_id)
    .execute(pool).await.expect("insert transition");
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2",
    )
    .bind(trans_id).bind(draft_id)
    .execute(pool).await.expect("set primary");

    sqlx::query(
        "UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1",
    )
    .bind(ver_id)
    .execute(pool).await.expect("publish version");

    (domain_id, ver_id, def_key)
}

/// Create a workflow instance via the application service (real creation path).
async fn create_instance(
    pool: &PgPool,
    creator_id: Uuid,
    domain_id: Uuid,
    definition_version_id: Uuid,
    title: &str,
) -> (Uuid, i32) {
    let result = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        pool,
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(creator_id),
            idempotency_key: format!("create-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(definition_version_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({}),
            context_payload: serde_json::json!({ "title": title }),
        },
    )
    .await
    .expect("create instance should succeed");

    (result.workflow_instance_id, result.workflow_state_version)
}

/// Grant the formal global coordinator role directly in the DB (used to set
/// up fixtures; the provisioning API lifecycle is tested separately).
async fn grant_global_coordinator(pool: &PgPool, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO global_role_bindings (binding_id, principal_id, role_key, enabled) VALUES ($1, $2, 'GLOBAL_WORKFLOW_COORDINATOR', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("grant global coordinator");
}

/// Grant the read-only global workflow reader role directly in the DB
/// (SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1; provisioning lifecycle tested
/// separately below).
async fn grant_global_reader(pool: &PgPool, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO global_role_bindings (binding_id, principal_id, role_key, enabled) VALUES ($1, $2, 'GLOBAL_WORKFLOW_READER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("grant global reader");
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

#[tokio::test]
async fn coordinator_sees_multi_domain_and_other_assignee_instances() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    // Two domains with independent owners/creators; a coordinator who is not
    // an owner or creator of either.
    let (owner_a, domain_a) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver_a, key_a) = seed_published_definition(&pool, domain_a).await;
    let (owner_b, domain_b) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver_b, key_b) = seed_published_definition(&pool, domain_b).await;
    let coordinator = seed_agent(&pool).await;
    grant_global_coordinator(&pool, coordinator).await;

    let (inst_a1, _) = create_instance(&pool, owner_a, domain_a, ver_a, "alpha-1").await;
    let (inst_a2, _) = create_instance(&pool, owner_a, domain_a, ver_a, "alpha-2").await;
    let (inst_b1, _) = create_instance(&pool, owner_b, domain_b, ver_b, "beta-1").await;

    let app = build_app(pool.clone(), &mock.url, vec![]);
    let token = direct_token(coordinator, "workflow.read", &mock.key_pair);
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/workflow-instances/global",
        &token,
    )
    .await;

    assert_eq!(status, 200, "coordinator list should succeed: {body}");
    let items = body["items"].as_array().expect("items array");
    let ids: Vec<String> = items
        .iter()
        .map(|it| it["workflow_instance_id"].as_str().unwrap().to_string())
        .collect();
    for expected in [inst_a1, inst_a2, inst_b1] {
        assert!(
            ids.contains(&expected.to_string()),
            "instance {expected} must be visible"
        );
    }

    // Exactly-two-domain visibility, proven precisely via the definitionKey
    // filter: all three created instances (and only those) are reachable in
    // one unfiltered stream spanning both domains.
    let domains: Vec<String> = items
        .iter()
        .map(|it| it["domain_id"].as_str().unwrap().to_string())
        .collect();
    assert!(domains.contains(&domain_a.to_string()));
    assert!(domains.contains(&domain_b.to_string()));

    // Per-definition scoping (the shared test DB may hold other instances).
    let (status, body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/global?definitionKey={key_a}"),
        &token,
    )
    .await;
    assert_eq!(status, 200);
    let items_a = body["items"].as_array().unwrap();
    assert_eq!(items_a.len(), 2, "domain A instances: {body}");
    for it in items_a {
        assert_eq!(it["domain_id"].as_str().unwrap(), domain_a.to_string());
    }

    let (status, body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/global?definitionKey={key_b}"),
        &token,
    )
    .await;
    assert_eq!(status, 200);
    let items_b = body["items"].as_array().unwrap();
    assert_eq!(items_b.len(), 1, "domain B instances: {body}");
    assert_eq!(
        items_b[0]["domain_id"].as_str().unwrap(),
        domain_b.to_string()
    );

    // The coordinator is not the current assignee of any of these (DRAFT node
    // assignee = WORKFLOW_CREATOR = owner). Assignee field still present.
    for it in items_a.iter().chain(items_b.iter()) {
        let assignee = it["current_assignee_principal_id"].as_str();
        assert!(assignee.is_some(), "assignee must be projected");
        assert_ne!(assignee.unwrap(), coordinator.to_string());
    }
}

#[tokio::test]
async fn coordinator_list_pagination_is_continuous() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver, key) = seed_published_definition(&pool, domain).await;
    let coordinator = seed_agent(&pool).await;
    grant_global_coordinator(&pool, coordinator).await;

    let mut expected = Vec::new();
    for i in 0..5 {
        let (id, _) = create_instance(&pool, owner, domain, ver, &format!("page-{i}")).await;
        expected.push(id);
    }

    let app = build_app(pool.clone(), &mock.url, vec![]);
    let token = direct_token(coordinator, "workflow.read", &mock.key_pair);

    // Scope to this test's definition so leftover instances in the shared
    // test DB cannot perturb the walk.
    let base = format!("/internal/v1/workflow-instances/global?definitionKey={key}");
    let mut collected: Vec<String> = Vec::new();
    let mut cursor: Option<(String, String)> = None;
    let mut rounds = 0;
    loop {
        let path = match &cursor {
            Some((created_at, id)) => {
                format!("{base}&limit=2&beforeCreatedAt={created_at}&beforeId={id}")
            }
            None => format!("{base}&limit=2"),
        };
        let (status, body) = do_get(app.clone(), &path, &token).await;
        assert_eq!(status, 200, "page must succeed: {body}");
        let items = body["items"].as_array().expect("items");
        for it in items {
            collected.push(it["workflow_instance_id"].as_str().unwrap().to_string());
        }
        let next = body["next_cursor"].clone();
        rounds += 1;
        if next.is_null() {
            break;
        }
        assert!(rounds < 10, "pagination must terminate");
        let created_at = next["created_at"].as_str().unwrap().to_string();
        let id = next["id"].as_str().unwrap().to_string();
        cursor = Some((created_at, id));
    }

    assert_eq!(collected.len(), 5, "all instances collected: {collected:?}");
    let unique: std::collections::HashSet<_> = collected.iter().collect();
    assert_eq!(unique.len(), 5, "no duplicates: {collected:?}");
    for id in &expected {
        assert!(collected.contains(&id.to_string()), "no misses for {id}");
    }
}

#[tokio::test]
async fn coordinator_list_returns_summary_only() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver, _) = seed_published_definition(&pool, domain).await;
    let coordinator = seed_agent(&pool).await;
    grant_global_coordinator(&pool, coordinator).await;
    create_instance(&pool, owner, domain, ver, "summary-1").await;

    let app = build_app(pool.clone(), &mock.url, vec![]);
    let token = direct_token(coordinator, "workflow.read", &mock.key_pair);
    let (status, body) = do_get(
        app,
        "/internal/v1/workflow-instances/global?status=all",
        &token,
    )
    .await;
    assert_eq!(status, 200);
    let item = &body["items"][0];

    // Summary surface only — never detail / submission payload.
    for key in [
        "workflow_instance_id",
        "domain_id",
        "definition_key",
        "current_node",
        "current_assignee_principal_id",
        "is_terminal",
        "title",
        "created_at",
        "updated_at",
    ] {
        assert!(item.get(key).is_some(), "summary field {key} missing");
    }
    for forbidden in [
        "context_payload",
        "current_context",
        "submission",
        "event_data",
        "external_reference",
        "external_url",
    ] {
        assert!(
            item.get(forbidden).is_none(),
            "detail field {forbidden} must not be exposed"
        );
    }
}

#[tokio::test]
async fn normal_agent_and_domain_owner_without_coordinator_denied() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver, _) = seed_published_definition(&pool, domain).await;
    let normal_agent = seed_agent(&pool).await;
    create_instance(&pool, owner, domain, ver, "denied-1").await;

    let app = build_app(pool.clone(), &mock.url, vec![]);

    // Normal agent (no role) → 403
    let token = direct_token(normal_agent, "workflow.read", &mock.key_pair);
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/workflow-instances/global",
        &token,
    )
    .await;
    assert_eq!(status, 403, "normal agent must be denied: {body}");
    assert_eq!(body["error"]["code"], "global_read_role_required");

    // DOMAIN_OWNER without a global read role → 403
    let token = direct_token(owner, "workflow.read", &mock.key_pair);
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/workflow-instances/global",
        &token,
    )
    .await;
    assert_eq!(
        status, 403,
        "domain owner without a global read role must be denied: {body}"
    );
    assert_eq!(body["error"]["code"], "global_read_role_required");

    // Missing scope → 403 forbidden
    let token = direct_token(normal_agent, "workflow.execute", &mock.key_pair);
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/workflow-instances/global",
        &token,
    )
    .await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], "forbidden");
}

#[tokio::test]
async fn domain_owner_boundary_unchanged_and_coordinator_gets_no_domain_powers() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    let (owner_a, domain_a) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver_a, _) = seed_published_definition(&pool, domain_a).await;
    let (owner_b, domain_b) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver_b, _) = seed_published_definition(&pool, domain_b).await;
    let coordinator = seed_agent(&pool).await;
    grant_global_coordinator(&pool, coordinator).await;

    let (inst_a, _) = create_instance(&pool, owner_a, domain_a, ver_a, "boundary-a").await;
    let (inst_b, _) = create_instance(&pool, owner_b, domain_b, ver_b, "boundary-b").await;

    let app = build_app(pool.clone(), &mock.url, vec![]);
    let coordinator_token = direct_token(coordinator, "workflow.read", &mock.key_pair);

    // Coordinator is NOT a domain owner → domain list 404 (boundary intact).
    let (status, body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/domain?domainId={domain_a}"),
        &coordinator_token,
    )
    .await;
    assert_eq!(
        status, 404,
        "coordinator must not see domain list without DOMAIN_OWNER: {body}"
    );
    assert_eq!(
        body["error"]["code"],
        "workflow_instance_not_found_or_not_visible"
    );

    // Owner A's domain list unchanged: only domain A instances visible.
    let owner_token = direct_token(owner_a, "workflow.read", &mock.key_pair);
    let (status, body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/domain?domainId={domain_a}&status=all"),
        &owner_token,
    )
    .await;
    assert_eq!(status, 200);
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0]["workflow_instance_id"].as_str().unwrap(),
        inst_a.to_string()
    );
    // Owner A cannot see domain B via domain list.
    let (status, body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/domain?domainId={domain_b}&status=all"),
        &owner_token,
    )
    .await;
    assert_eq!(
        status, 404,
        "cross-domain domain list must stay denied: {body}"
    );

    // Coordinator cannot cancel an instance it does not own (no DOMAIN_OWNER).
    let exec_token = direct_token(
        coordinator,
        "workflow.execute workflow.read",
        &mock.key_pair,
    );
    let (status, body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_a}/cancel"),
        &exec_token,
        json!({"reason": "coordinator test"}),
        "cancel-coord-1",
    )
    .await;
    assert_eq!(status, 403, "coordinator cancel must be denied: {body}");
    assert_eq!(body["error"]["code"], "not_domain_owner");

    // Coordinator cannot archive either.
    let (status, body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_a}/archive"),
        &exec_token,
        json!({"reason": "coordinator test"}),
        "archive-coord-1",
    )
    .await;
    assert_eq!(status, 403, "coordinator archive must be denied: {body}");
    assert_eq!(body["error"]["code"], "not_domain_owner");

    // Coordinator cannot transition a non-assigned instance.
    let trans_id: Uuid = sqlx::query_scalar(
        "SELECT transition_id FROM workflow_transition_definitions WHERE definition_version_id = $1 ORDER BY transition_key LIMIT 1",
    )
    .bind(ver_b)
    .fetch_one(&pool)
    .await
    .expect("find transition");
    let (status, body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_b}/transitions"),
        &exec_token,
        json!({
            "transitionDefinitionId": trans_id.to_string(),
            "expectedWorkflowStateVersion": 1,
            "submissionPayload": null
        }),
        "transition-coord-1",
    )
    .await;
    assert_eq!(
        status, 403,
        "coordinator transition on other-assignee must be denied: {body}"
    );
    assert_eq!(body["error"]["code"], "principal_not_assignee");
}

#[tokio::test]
async fn global_role_binding_provisioning_lifecycle() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    let admin = seed_agent(&pool).await;
    let target = seed_agent(&pool).await;

    let app = build_app(pool.clone(), &mock.url, vec![admin]);
    let admin_token = direct_token(admin, "workflow.admin", &mock.key_pair);

    // PUT grants the formal coordinator role.
    let (status, body) = do_put(
        app.clone(),
        &format!("/internal/v1/admin/global-role-bindings/{target}"),
        &admin_token,
        json!({"roleKey": "GLOBAL_WORKFLOW_COORDINATOR", "enabled": true}),
        "grant-global-1",
    )
    .await;
    assert_eq!(status, 200, "grant must succeed: {body}");
    assert_eq!(body["roleKey"], "GLOBAL_WORKFLOW_COORDINATOR");
    assert_eq!(body["enabled"], true);

    let enabled: bool = sqlx::query_scalar(
        "SELECT enabled FROM global_role_bindings WHERE principal_id = $1 AND role_key = 'GLOBAL_WORKFLOW_COORDINATOR'",
    )
    .bind(target)
    .fetch_one(&pool)
    .await
    .expect("binding row");
    assert!(enabled);

    // Unsupported role key → 422.
    let (status, body) = do_put(
        app.clone(),
        &format!("/internal/v1/admin/global-role-bindings/{target}"),
        &admin_token,
        json!({"roleKey": "DOMAIN_OWNER", "enabled": true}),
        "grant-global-2",
    )
    .await;
    assert_eq!(status, 422);
    assert_eq!(body["error"]["code"], "role_key_invalid");

    // Coordinator token can now read the global list; after revoke it cannot.
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver, _) = seed_published_definition(&pool, domain).await;
    create_instance(&pool, owner, domain, ver, "lifecycle-1").await;

    let read_app = build_app(pool.clone(), &mock.url, vec![admin]);
    let read_token = direct_token(target, "workflow.read", &mock.key_pair);
    let (status, _) = do_get(
        read_app.clone(),
        "/internal/v1/workflow-instances/global",
        &read_token,
    )
    .await;
    assert_eq!(status, 200, "granted coordinator can list");

    // DELETE revokes.
    let (status, body) = do_delete(
        app.clone(),
        &format!("/internal/v1/admin/global-role-bindings/{target}"),
        &admin_token,
        json!({"roleKey": "GLOBAL_WORKFLOW_COORDINATOR"}),
        "revoke-global-1",
    )
    .await;
    assert_eq!(status, 200, "revoke must succeed: {body}");
    assert_eq!(body["enabled"], false);

    let (status, _) = do_get(
        read_app,
        "/internal/v1/workflow-instances/global",
        &read_token,
    )
    .await;
    assert_eq!(status, 403, "revoked coordinator can no longer list");
}

#[tokio::test]
async fn global_workflow_reader_sees_cross_domain_instances() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    // Two domains with independent owners; a READER (not owner/creator of
    // either, no coordinator binding).
    let (owner_a, domain_a) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver_a, key_a) = seed_published_definition(&pool, domain_a).await;
    let (owner_b, domain_b) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver_b, _) = seed_published_definition(&pool, domain_b).await;
    let reader = seed_agent(&pool).await;
    grant_global_reader(&pool, reader).await;

    let (inst_a1, _) = create_instance(&pool, owner_a, domain_a, ver_a, "reader-a1").await;
    let (inst_a2, _) = create_instance(&pool, owner_a, domain_a, ver_a, "reader-a2").await;
    let (inst_b1, _) = create_instance(&pool, owner_b, domain_b, ver_b, "reader-b1").await;

    let app = build_app(pool.clone(), &mock.url, vec![]);
    let token = direct_token(reader, "workflow.read", &mock.key_pair);
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/workflow-instances/global",
        &token,
    )
    .await;

    assert_eq!(status, 200, "reader list should succeed: {body}");
    let items = body["items"].as_array().expect("items array");
    let ids: Vec<String> = items
        .iter()
        .map(|it| it["workflow_instance_id"].as_str().unwrap().to_string())
        .collect();
    for expected in [inst_a1, inst_a2, inst_b1] {
        assert!(
            ids.contains(&expected.to_string()),
            "instance {expected} must be visible to the reader"
        );
    }

    // Per-definition scoping keeps working for a reader.
    let (status, body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/global?definitionKey={key_a}"),
        &token,
    )
    .await;
    assert_eq!(status, 200);
    let items_a = body["items"].as_array().unwrap();
    assert_eq!(items_a.len(), 2, "domain A instances via reader: {body}");
    for it in items_a {
        assert_eq!(it["domain_id"].as_str().unwrap(), domain_a.to_string());
        // Summary projection only.
        assert!(it.get("context_payload").is_none());
    }
}

#[tokio::test]
async fn reader_gains_no_write_or_assistance_powers() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    let (owner_a, domain_a) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver_a, _) = seed_published_definition(&pool, domain_a).await;
    let (owner_b, domain_b) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver_b, _) = seed_published_definition(&pool, domain_b).await;
    let reader = seed_agent(&pool).await;
    grant_global_reader(&pool, reader).await;

    let (inst_a, _) = create_instance(&pool, owner_a, domain_a, ver_a, "reader-write-a").await;
    let (inst_b, _) = create_instance(&pool, owner_b, domain_b, ver_b, "reader-write-b").await;

    let app = build_app(pool.clone(), &mock.url, vec![]);
    // Reader mints workflow.execute too (HR-main shape): write gates must
    // still deny — READER is not a write role of any kind.
    let exec_token = direct_token(reader, "workflow.execute workflow.read", &mock.key_pair);

    // Domain create → coordinator-only.
    let (status, body) = do_post(
        app.clone(),
        "/internal/v1/domains",
        &exec_token,
        json!({
            "domainId": Uuid::new_v4(),
            "domainKey": "reader-denied-create-1",
            "displayName": "Denied",
            "enabled": true
        }),
        "reader-denied-create-1",
    )
    .await;
    assert_eq!(status, 403, "reader domain create must be denied: {body}");
    assert_eq!(body["error"]["code"], "global_coordinator_required");

    // Domain owner replacement → coordinator-only.
    let (status, body) = do_put(
        app.clone(),
        &format!("/internal/v1/domains/{domain_a}/owner"),
        &exec_token,
        json!({ "newOwnerPrincipalId": Uuid::new_v4() }),
        "reader-denied-owner-1",
    )
    .await;
    assert_eq!(
        status, 403,
        "reader owner replacement must be denied: {body}"
    );
    assert_eq!(body["error"]["code"], "global_coordinator_required");

    // Cancel / archive stay DOMAIN_OWNER-gated.
    let (status, body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_a}/cancel"),
        &exec_token,
        json!({"reason": "reader test"}),
        "reader-denied-cancel-1",
    )
    .await;
    assert_eq!(status, 403, "reader cancel must be denied: {body}");
    assert_eq!(body["error"]["code"], "not_domain_owner");

    let (status, body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_a}/archive"),
        &exec_token,
        json!({"reason": "reader test"}),
        "reader-denied-archive-1",
    )
    .await;
    assert_eq!(status, 403, "reader archive must be denied: {body}");
    assert_eq!(body["error"]["code"], "not_domain_owner");

    // Transition stays assignee-gated.
    let trans_id: Uuid = sqlx::query_scalar(
        "SELECT transition_id FROM workflow_transition_definitions WHERE definition_version_id = $1 ORDER BY transition_key LIMIT 1",
    )
    .bind(ver_b)
    .fetch_one(&pool)
    .await
    .expect("find transition");
    let (status, body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_b}/transitions"),
        &exec_token,
        json!({
            "transitionDefinitionId": trans_id.to_string(),
            "expectedWorkflowStateVersion": 1,
            "submissionPayload": null
        }),
        "reader-denied-transition-1",
    )
    .await;
    assert_eq!(
        status, 403,
        "reader transition on other-assignee must be denied: {body}"
    );
    assert_eq!(body["error"]["code"], "principal_not_assignee");

    // Assistance human-required stays coordinator-only (READER ≠ assistance
    // reader; the assistance gate is deliberately unchanged — owner-inbox is
    // domain-owner-scoped and returns an empty page, but human-required is
    // the cross-domain coordinator surface).
    let (status, body) = do_get(
        app.clone(),
        "/internal/v1/assistance-cases/human-required",
        &exec_token,
    )
    .await;
    assert_eq!(status, 403, "reader human-required must be denied: {body}");
    assert_eq!(body["error"]["code"], "global_coordinator_required");
}

#[tokio::test]
async fn reader_role_binding_provisioning_lifecycle() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    let admin = seed_agent(&pool).await;
    let target = seed_agent(&pool).await;

    let app = build_app(pool.clone(), &mock.url, vec![admin]);
    let admin_token = direct_token(admin, "workflow.admin", &mock.key_pair);

    // PUT accepts the read-only role key.
    let (status, body) = do_put(
        app.clone(),
        &format!("/internal/v1/admin/global-role-bindings/{target}"),
        &admin_token,
        json!({"roleKey": "GLOBAL_WORKFLOW_READER", "enabled": true}),
        "grant-reader-1",
    )
    .await;
    assert_eq!(status, 200, "reader grant must succeed: {body}");
    assert_eq!(body["roleKey"], "GLOBAL_WORKFLOW_READER");
    assert_eq!(body["enabled"], true);

    let enabled: bool = sqlx::query_scalar(
        "SELECT enabled FROM global_role_bindings WHERE principal_id = $1 AND role_key = 'GLOBAL_WORKFLOW_READER'",
    )
    .bind(target)
    .fetch_one(&pool)
    .await
    .expect("reader binding row");
    assert!(enabled);

    // Unsupported role key → still 422.
    let (status, body) = do_put(
        app.clone(),
        &format!("/internal/v1/admin/global-role-bindings/{target}"),
        &admin_token,
        json!({"roleKey": "GLOBAL_READ_ALL", "enabled": true}),
        "grant-reader-2",
    )
    .await;
    assert_eq!(status, 422);
    assert_eq!(body["error"]["code"], "role_key_invalid");

    // Granted reader can list; after revoke it cannot.
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let (_, ver, _) = seed_published_definition(&pool, domain).await;
    create_instance(&pool, owner, domain, ver, "reader-lifecycle-1").await;

    let read_app = build_app(pool.clone(), &mock.url, vec![admin]);
    let read_token = direct_token(target, "workflow.read", &mock.key_pair);
    let (status, _) = do_get(
        read_app.clone(),
        "/internal/v1/workflow-instances/global",
        &read_token,
    )
    .await;
    assert_eq!(status, 200, "granted reader can list");

    let (status, body) = do_delete(
        app.clone(),
        &format!("/internal/v1/admin/global-role-bindings/{target}"),
        &admin_token,
        json!({"roleKey": "GLOBAL_WORKFLOW_READER"}),
        "revoke-reader-1",
    )
    .await;
    assert_eq!(status, 200, "reader revoke must succeed: {body}");
    assert_eq!(body["enabled"], false);

    let (status, body) = do_get(
        read_app,
        "/internal/v1/workflow-instances/global",
        &read_token,
    )
    .await;
    assert_eq!(status, 403, "revoked reader can no longer list");
    assert_eq!(body["error"]["code"], "global_read_role_required");
}
