//! HTTP integration tests for the agent-facing GLOBAL_WORKFLOW_COORDINATOR
//! domain management endpoints:
//!
//!   POST /internal/v1/domains              (create domain)
//!   PUT  /internal/v1/domains/{domainId}/owner (set domain owner)
//!
//! Authorization model under test:
//!   - coarse scope `workflow.execute` (auth layer)
//!   - business role GLOBAL_WORKFLOW_COORDINATOR verified server-side from
//!     `global_role_bindings` (never in the JWT)
//!   - DOMAIN_OWNER / plain agents without the role are denied (403)
//!   - OBO (delegated) tokens are denied (direct token required)
//!   - idempotency: same key replays the stored response

#![allow(clippy::needless_borrow)]
#![allow(unused_imports, unused_variables)]

mod common;

use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};

// ============================================================================
// Test app builder
// ============================================================================

fn build_app(pool: sqlx::PgPool, jwks_url: &str) -> axum::Router {
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

fn obo_token(
    subject: Uuid,
    delegating: Uuid,
    scope: &str,
    key_pair: &common::RsaTestKeyPair,
) -> String {
    common::v1_obo_token(subject, delegating, scope, None, 300, key_pair)
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

// ============================================================================
// Seeds
// ============================================================================

async fn seed_agent(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'Test Agent', NULL, TRUE)",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("seed agent");
    id
}

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

async fn seed_domain(pool: &PgPool, domain_id: Uuid) {
    sqlx::query(
        "INSERT INTO domains (domain_id, domain_key, display_name, enabled) VALUES ($1, $2, 'Coord Test Domain', TRUE)",
    )
    .bind(domain_id)
    .bind(format!("coord-test-{}", &Uuid::new_v4().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed domain");
}

async fn domain_enabled(pool: &PgPool, domain_id: Uuid) -> bool {
    sqlx::query_scalar("SELECT enabled FROM domains WHERE domain_id = $1")
        .bind(domain_id)
        .fetch_one(pool)
        .await
        .expect("read domain")
}

async fn owner_binding_enabled(pool: &PgPool, domain_id: Uuid, owner_id: Uuid) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM domain_role_bindings WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE)",
    )
    .bind(domain_id)
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .expect("read owner binding")
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn coordinator_create_domain_succeeds_and_replays() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let coordinator = seed_agent(&pool).await;
    grant_global_coordinator(&pool, coordinator).await;

    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(coordinator, "workflow.execute", &mock.key_pair);
    let domain_id = Uuid::new_v4();

    let (status, body) = do_post(
        app.clone(),
        "/internal/v1/domains",
        &token,
        json!({
            "domainId": domain_id,
            "domainKey": "coord-create-1",
            "displayName": "Coord Created Domain",
            "enabled": true
        }),
        "coord-create-domain-1",
    )
    .await;
    assert_eq!(status, 200, "create domain must succeed: {body}");
    assert_eq!(body["domainId"], domain_id.to_string());
    assert_eq!(body["domainKey"], "coord-create-1");
    assert!(domain_enabled(&pool, domain_id).await);

    // Same idempotency key replays the stored response (no second domain row).
    let (status2, body2) = do_post(
        app.clone(),
        "/internal/v1/domains",
        &token,
        json!({
            "domainId": domain_id,
            "domainKey": "coord-create-1",
            "displayName": "Coord Created Domain",
            "enabled": true
        }),
        "coord-create-domain-1",
    )
    .await;
    assert_eq!(status2, 200, "replay must succeed: {body2}");
    assert_eq!(body2["domainId"], domain_id.to_string());
}

#[tokio::test]
async fn coordinator_set_domain_owner_succeeds() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let coordinator = seed_agent(&pool).await;
    grant_global_coordinator(&pool, coordinator).await;
    let new_owner = seed_agent(&pool).await;
    let domain_id = Uuid::new_v4();
    seed_domain(&pool, domain_id).await;

    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(coordinator, "workflow.execute", &mock.key_pair);

    let (status, body) = do_put(
        app.clone(),
        &format!("/internal/v1/domains/{domain_id}/owner"),
        &token,
        json!({ "newOwnerPrincipalId": new_owner }),
        "coord-set-owner-1",
    )
    .await;
    assert_eq!(status, 200, "set owner must succeed: {body}");
    assert_eq!(body["domainId"], domain_id.to_string());
    assert_eq!(body["newOwnerId"], new_owner.to_string());
    assert!(owner_binding_enabled(&pool, domain_id, new_owner).await);

    // Same key replays.
    let (status2, body2) = do_put(
        app.clone(),
        &format!("/internal/v1/domains/{domain_id}/owner"),
        &token,
        json!({ "newOwnerPrincipalId": new_owner }),
        "coord-set-owner-1",
    )
    .await;
    assert_eq!(status2, 200, "replay must succeed: {body2}");
}

#[tokio::test]
async fn non_coordinator_agent_denied_for_create_domain() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let plain_agent = seed_agent(&pool).await; // no coordinator binding

    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(plain_agent, "workflow.execute", &mock.key_pair);

    let (status, body) = do_post(
        app,
        "/internal/v1/domains",
        &token,
        json!({
            "domainId": Uuid::new_v4(),
            "domainKey": "denied-create-1",
            "displayName": "Denied",
            "enabled": true
        }),
        "denied-create-1",
    )
    .await;
    assert_eq!(status, 403, "must be denied: {body}");
    assert_eq!(body["error"]["code"], "global_coordinator_required");
}

#[tokio::test]
async fn non_coordinator_agent_denied_for_set_domain_owner() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let plain_agent = seed_agent(&pool).await;
    let domain_id = Uuid::new_v4();
    seed_domain(&pool, domain_id).await;

    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(plain_agent, "workflow.execute", &mock.key_pair);

    let (status, body) = do_put(
        app,
        &format!("/internal/v1/domains/{domain_id}/owner"),
        &token,
        json!({ "newOwnerPrincipalId": Uuid::new_v4() }),
        "denied-owner-1",
    )
    .await;
    assert_eq!(status, 403, "must be denied: {body}");
    assert_eq!(body["error"]["code"], "global_coordinator_required");
}

#[tokio::test]
async fn domain_owner_without_coordinator_role_denied() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    // A DOMAIN_OWNER (of their own domain) without the global coordinator
    // role must not be able to create domains or set owners.
    let (owner_id, _domain_id) = common::seed_principal_domain_with_owner(&pool).await;

    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(owner_id, "workflow.execute", &mock.key_pair);

    let (status, body) = do_post(
        app.clone(),
        "/internal/v1/domains",
        &token,
        json!({
            "domainId": Uuid::new_v4(),
            "domainKey": "owner-denied-1",
            "displayName": "Denied",
            "enabled": true
        }),
        "owner-denied-1",
    )
    .await;
    assert_eq!(status, 403, "must be denied: {body}");
    assert_eq!(body["error"]["code"], "global_coordinator_required");
}

#[tokio::test]
async fn read_scope_denied() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let coordinator = seed_agent(&pool).await;
    grant_global_coordinator(&pool, coordinator).await;

    let app = build_app(pool.clone(), &mock.url);
    // workflow.read is not enough — writes require workflow.execute.
    let token = direct_token(coordinator, "workflow.read", &mock.key_pair);

    let (status, body) = do_post(
        app,
        "/internal/v1/domains",
        &token,
        json!({
            "domainId": Uuid::new_v4(),
            "domainKey": "read-scope-denied",
            "displayName": "Denied",
            "enabled": true
        }),
        "read-scope-denied-1",
    )
    .await;
    assert_eq!(status, 403, "must be denied: {body}");
}

#[tokio::test]
async fn obo_token_denied() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let coordinator = seed_agent(&pool).await;
    grant_global_coordinator(&pool, coordinator).await;
    let delegating = Uuid::new_v4();

    let app = build_app(pool.clone(), &mock.url);
    // Delegated (OBO) tokens are rejected — writes need a direct access token.
    let token = obo_token(coordinator, delegating, "workflow.execute", &mock.key_pair);

    let (status, body) = do_post(
        app,
        "/internal/v1/domains",
        &token,
        json!({
            "domainId": Uuid::new_v4(),
            "domainKey": "obo-denied",
            "displayName": "Denied",
            "enabled": true
        }),
        "obo-denied-1",
    )
    .await;
    assert_eq!(status, 403, "must be denied: {body}");
    assert_eq!(body["error"]["code"], "direct_token_required");
}
