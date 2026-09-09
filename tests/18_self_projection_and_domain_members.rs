//! HTTP integration tests for Agent self-projection and domain member
//! management.  Uses in-process axum with MockJwksServer.

#![allow(clippy::needless_borrow)]

mod common;

use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};

// ---------------------------------------------------------------------------
// Test app builder
// ---------------------------------------------------------------------------

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
        provisioning_config: ProvisioningConfig::new(Vec::new()),
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
    act_sub: Uuid,
    scope: &str,
    key_pair: &common::RsaTestKeyPair,
) -> String {
    common::v1_obo_token(subject, act_sub, scope, Some("test-client"), 300, key_pair)
}

async fn do_put(app: axum::Router, path: &str, token: &str) -> (u16, Value) {
    let req = Request::builder()
        .method("PUT")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

async fn do_put_idem(app: axum::Router, path: &str, token: &str, key: &str) -> (u16, Value) {
    let req = Request::builder()
        .method("PUT")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", key)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
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

async fn do_del_idem(app: axum::Router, path: &str, token: &str, key: &str) -> (u16, Value) {
    let req = Request::builder()
        .method("DELETE")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", key)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

// ===========================================================================
// Self-Projection Tests
// ===========================================================================

#[tokio::test]
async fn agent_self_projection_success() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let agent_id = Uuid::new_v4();
    let token = direct_token(agent_id, "workflow.read", &mock.key_pair);

    let (status, body) = do_put(app, "/internal/v1/principals/me", &token).await;
    assert_eq!(status, 200, "self-projection: {body:?}");
    assert_eq!(body["principalId"].as_str().unwrap(), &agent_id.to_string());
    assert!(body["created"].as_bool().unwrap());
}

#[tokio::test]
async fn self_projection_idempotent() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let agent_id = Uuid::new_v4();
    let token = direct_token(agent_id, "workflow.read", &mock.key_pair);

    let app = build_app(pool.clone(), &mock.url);
    let (s1, _) = do_put(app, "/internal/v1/principals/me", &token).await;
    assert_eq!(s1, 200);
    let app = build_app(pool.clone(), &mock.url);
    let (s2, b2) = do_put(app, "/internal/v1/principals/me", &token).await;
    assert_eq!(s2, 200);
    assert!(!b2["created"].as_bool().unwrap());
}

#[tokio::test]
async fn obo_token_rejected_for_self_projection() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let agent_id = Uuid::new_v4();
    let delegating = Uuid::new_v4();
    let token = obo_token(agent_id, delegating, "workflow.read", &mock.key_pair);

    let (status, body) = do_put(app, "/internal/v1/principals/me", &token).await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], "direct_token_required");
}

#[tokio::test]
async fn disabled_principal_cannot_self_project() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let agent_id = Uuid::new_v4();
    sqlx::query("INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'disabled', NULL, FALSE)")
        .bind(agent_id).execute(&pool).await.unwrap();

    let token = direct_token(agent_id, "workflow.read", &mock.key_pair);
    let (status, body) = do_put(app, "/internal/v1/principals/me", &token).await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], "principal_disabled");
}

#[tokio::test]
async fn type_conflict_self_projection_fails() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let agent_id = Uuid::new_v4();
    sqlx::query("INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'HUMAN', 'human', NULL, TRUE)")
        .bind(agent_id).execute(&pool).await.unwrap();

    let token = direct_token(agent_id, "workflow.read", &mock.key_pair);
    let (status, body) = do_put(app, "/internal/v1/principals/me", &token).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "principal_projection_conflict");
}

// ===========================================================================
// Domain Member Tests
// ===========================================================================

async fn seed_owner_member_scenario(pool: &sqlx::PgPool) -> (Uuid, Uuid, Uuid) {
    let (owner_id, domain_id) = common::seed_principal_domain_with_owner(pool).await;
    let member_id = Uuid::new_v4();
    sqlx::query("INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'test-agent', NULL, TRUE)")
        .bind(member_id).execute(pool).await.unwrap();
    (owner_id, domain_id, member_id)
}

#[tokio::test]
async fn owner_lists_members() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute workflow.read", &mock.key_pair);

    // Add member
    let app = build_app(pool.clone(), &mock.url);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");
    let _ = do_put_idem(app, &path, &owner_token, "add-key").await;

    // List
    let app = build_app(pool.clone(), &mock.url);
    let (status, body) = do_get(
        app,
        &format!("/internal/v1/domains/{domain_id}/members"),
        &owner_token,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["items"][0]["role"].as_str().unwrap(), "DOMAIN_MEMBER");
}

#[tokio::test]
async fn non_owner_cannot_list_members() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (_, domain_id, _) = seed_owner_member_scenario(&pool).await;
    let stranger = Uuid::new_v4();
    sqlx::query("INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'stranger', NULL, TRUE)")
        .bind(stranger).execute(&pool).await.unwrap();
    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(stranger, "workflow.read", &mock.key_pair);

    let (status, body) = do_get(
        app,
        &format!("/internal/v1/domains/{domain_id}/members"),
        &token,
    )
    .await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], "not_domain_owner");
}

#[tokio::test]
async fn owner_adds_existing_principal_as_member() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let app = build_app(pool.clone(), &mock.url);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let (status, body) = do_put_idem(app, &path, &owner_token, "add-key-1").await;
    assert_eq!(status, 200, "add member: {body:?}");
    assert_eq!(
        body["principalId"].as_str().unwrap(),
        &member_id.to_string()
    );
}

#[tokio::test]
async fn add_member_idempotent() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let app = build_app(pool.clone(), &mock.url);
    let (s1, _) = do_put_idem(app, &path, &owner_token, "idem-key").await;
    assert_eq!(s1, 200);
    let app = build_app(pool.clone(), &mock.url);
    let (s2, _) = do_put_idem(app, &path, &owner_token, "idem-key").await;
    assert_eq!(s2, 200);
}

#[tokio::test]
async fn add_unregistered_principal_fails() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, _) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let app = build_app(pool.clone(), &mock.url);
    let unknown = Uuid::new_v4();
    let path = format!("/internal/v1/domains/{domain_id}/members/{unknown}");

    let (status, body) = do_put_idem(app, &path, &owner_token, "add-key-np").await;
    assert_eq!(status, 404);
    assert_eq!(body["error"]["code"], "principal_not_registered");
}

#[tokio::test]
async fn add_owner_as_member_fails() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, _) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let app = build_app(pool.clone(), &mock.url);
    let path = format!("/internal/v1/domains/{domain_id}/members/{owner_id}");

    let (status, body) = do_put_idem(app, &path, &owner_token, "add-owner-key").await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "principal_is_owner");
}

#[tokio::test]
async fn owner_removes_member() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let app = build_app(pool.clone(), &mock.url);
    let _ = do_put_idem(app, &path, &owner_token, "add-rm").await;
    let app = build_app(pool.clone(), &mock.url);
    let (status, body) = do_del_idem(app, &path, &owner_token, "del-key").await;
    assert_eq!(status, 200);
    assert!(!body["enabled"].as_bool().unwrap_or(true));
}

#[tokio::test]
async fn remove_nonexistent_member_fails() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let app = build_app(pool.clone(), &mock.url);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let (status, body) = do_del_idem(app, &path, &owner_token, "del-none").await;
    assert_eq!(status, 404);
    assert_eq!(body["error"]["code"], "member_not_found");
}

#[tokio::test]
async fn cross_domain_operation_fails() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, _domain_a, _) = seed_owner_member_scenario(&pool).await;
    let (_, domain_b) = common::seed_principal_domain_with_owner(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let target = Uuid::new_v4();
    sqlx::query("INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'cross-target', NULL, TRUE)")
        .bind(target).execute(&pool).await.unwrap();
    let app = build_app(pool.clone(), &mock.url);
    let path = format!("/internal/v1/domains/{domain_b}/members/{target}");

    let (status, body) = do_put_idem(app, &path, &owner_token, "cross-key").await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], "not_domain_owner");
}

#[tokio::test]
async fn obo_token_rejected_for_member_ops() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let delegating = Uuid::new_v4();
    let token = obo_token(owner_id, delegating, "workflow.execute", &mock.key_pair);
    let app = build_app(pool.clone(), &mock.url);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let (status, body) = do_put_idem(app, &path, &token, "obo-key").await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], "direct_token_required");
}

#[tokio::test]
async fn multi_domain_membership_independent() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let member_id = Uuid::new_v4();
    sqlx::query("INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'multi', NULL, TRUE)")
        .bind(member_id).execute(&pool).await.unwrap();
    let (owner_a, domain_a) = common::seed_principal_domain_with_owner(&pool).await;
    let (owner_b, domain_b) = common::seed_principal_domain_with_owner(&pool).await;
    let token_a = direct_token(owner_a, "workflow.execute workflow.read", &mock.key_pair);
    let token_b = direct_token(owner_b, "workflow.execute workflow.read", &mock.key_pair);

    // Add to domain A
    let app = build_app(pool.clone(), &mock.url);
    let pa = format!("/internal/v1/domains/{domain_a}/members/{member_id}");
    let (s1, _) = do_put_idem(app, &pa, &token_a, "ma-key").await;
    assert_eq!(s1, 200);

    // Add to domain B
    let app = build_app(pool.clone(), &mock.url);
    let pb = format!("/internal/v1/domains/{domain_b}/members/{member_id}");
    let (s2, _) = do_put_idem(app, &pb, &token_b, "mb-key").await;
    assert_eq!(s2, 200);

    // Verify both domains have the member
    let app = build_app(pool.clone(), &mock.url);
    let (s3, b3) = do_get(
        app,
        &format!("/internal/v1/domains/{domain_a}/members"),
        &token_a,
    )
    .await;
    assert_eq!(s3, 200);
    assert_eq!(b3["items"].as_array().unwrap().len(), 1);

    let app = build_app(pool.clone(), &mock.url);
    let (s4, b4) = do_get(
        app,
        &format!("/internal/v1/domains/{domain_b}/members"),
        &token_b,
    )
    .await;
    assert_eq!(s4, 200);
    assert_eq!(b4["items"].as_array().unwrap().len(), 1);

    // Remove from domain A only
    let app = build_app(pool.clone(), &mock.url);
    let (s5, _) = do_del_idem(app, &pa, &token_a, "del-ma").await;
    assert_eq!(s5, 200);

    // Domain B unaffected
    let app = build_app(pool.clone(), &mock.url);
    let (s6, b6) = do_get(
        app,
        &format!("/internal/v1/domains/{domain_b}/members"),
        &token_b,
    )
    .await;
    assert_eq!(s6, 200);
    assert_eq!(b6["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn delete_does_not_affect_owner() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, _) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let app = build_app(pool.clone(), &mock.url);
    let path = format!("/internal/v1/domains/{domain_id}/members/{owner_id}");

    let (status, body) = do_del_idem(app, &path, &owner_token, "del-owner").await;
    assert_eq!(status, 404);
    assert_eq!(body["error"]["code"], "member_not_found");

    let owner_ok: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM domain_role_bindings WHERE domain_id=$1 AND principal_id=$2 AND role_key='DOMAIN_OWNER' AND enabled=TRUE)")
        .bind(domain_id).bind(owner_id)
        .fetch_one(&pool).await.unwrap();
    assert!(owner_ok, "owner binding must survive");
}

// ===========================================================================
// Caller-scoped domain discovery (GET /internal/v1/principals/me/domains)
// ===========================================================================

#[tokio::test]
async fn caller_lists_own_owner_domain() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    let token = direct_token(owner_id, "workflow.read", &mock.key_pair);

    let app = build_app(pool.clone(), &mock.url);
    let (status, body) = do_get(app, "/internal/v1/principals/me/domains", &token).await;
    assert_eq!(status, 200, "my domains: {body:?}");
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0]["domain_id"].as_str().unwrap(),
        &domain_id.to_string()
    );
    assert_eq!(items[0]["caller_role"].as_str().unwrap(), "DOMAIN_OWNER");
    assert!(!items[0]["domain_key"].as_str().unwrap().is_empty());
    assert!(!items[0]["display_name"].as_str().unwrap().is_empty());
    assert!(items[0]["binding_created_at"].as_str().is_some());
}

#[tokio::test]
async fn caller_lists_owner_and_member_domains() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_a) = common::seed_principal_domain_with_owner(&pool).await;

    // Caller is DOMAIN_MEMBER of a second domain.
    let (_, domain_b) = common::seed_principal_and_domain(&pool).await;
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1, $2, $3, 'DOMAIN_MEMBER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_b)
    .bind(owner_id)
    .execute(&pool)
    .await
    .unwrap();

    let token = direct_token(owner_id, "workflow.read", &mock.key_pair);
    let app = build_app(pool.clone(), &mock.url);
    let (status, body) = do_get(app, "/internal/v1/principals/me/domains", &token).await;
    assert_eq!(status, 200, "my domains: {body:?}");
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);

    let by_domain: std::collections::HashMap<&str, &serde_json::Value> = items
        .iter()
        .map(|item| (item["domain_id"].as_str().unwrap(), item))
        .collect();
    assert_eq!(
        by_domain[domain_a.to_string().as_str()]["caller_role"],
        "DOMAIN_OWNER"
    );
    assert_eq!(
        by_domain[domain_b.to_string().as_str()]["caller_role"],
        "DOMAIN_MEMBER"
    );
}

#[tokio::test]
async fn my_domains_is_caller_scoped() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_a, _) = common::seed_principal_domain_with_owner(&pool).await;
    let (owner_b, _) = common::seed_principal_domain_with_owner(&pool).await;
    assert_ne!(owner_a, owner_b);

    // Caller A sees only their own domain, never B's.
    let token_a = direct_token(owner_a, "workflow.read", &mock.key_pair);
    let app = build_app(pool.clone(), &mock.url);
    let (status, body) = do_get(app, "/internal/v1/principals/me/domains", &token_a).await;
    assert_eq!(status, 200);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn caller_with_no_bindings_gets_empty_list() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let stranger = common::seed_second_principal(&pool).await;
    let token = direct_token(stranger, "workflow.read", &mock.key_pair);

    let app = build_app(pool.clone(), &mock.url);
    let (status, body) = do_get(app, "/internal/v1/principals/me/domains", &token).await;
    assert_eq!(status, 200, "my domains: {body:?}");
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn disabled_binding_excluded_from_my_domains() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_a) = common::seed_principal_domain_with_owner(&pool).await;

    // A second, disabled binding must not appear.
    let (_, domain_b) = common::seed_principal_and_domain(&pool).await;
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1, $2, $3, 'DOMAIN_OWNER', FALSE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_b)
    .bind(owner_id)
    .execute(&pool)
    .await
    .unwrap();

    let token = direct_token(owner_id, "workflow.read", &mock.key_pair);
    let app = build_app(pool.clone(), &mock.url);
    let (status, body) = do_get(app, "/internal/v1/principals/me/domains", &token).await;
    assert_eq!(status, 200);
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0]["domain_id"].as_str().unwrap(),
        &domain_a.to_string()
    );
}

#[tokio::test]
async fn disabled_domain_excluded_from_my_domains() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_a) = common::seed_principal_domain_with_owner(&pool).await;

    // A second domain that is disabled must not appear even with an active binding.
    let (_, domain_b) = common::seed_principal_and_domain(&pool).await;
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1, $2, $3, 'DOMAIN_OWNER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_b)
    .bind(owner_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE domains SET enabled = FALSE WHERE domain_id = $1")
        .bind(domain_b)
        .execute(&pool)
        .await
        .unwrap();

    let token = direct_token(owner_id, "workflow.read", &mock.key_pair);
    let app = build_app(pool.clone(), &mock.url);
    let (status, body) = do_get(app, "/internal/v1/principals/me/domains", &token).await;
    assert_eq!(status, 200);
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0]["domain_id"].as_str().unwrap(),
        &domain_a.to_string()
    );
}

#[tokio::test]
async fn my_domains_requires_workflow_read_scope() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, _) = common::seed_principal_domain_with_owner(&pool).await;
    let token = direct_token(owner_id, "workflow.execute", &mock.key_pair);

    let app = build_app(pool.clone(), &mock.url);
    let (status, body) = do_get(app, "/internal/v1/principals/me/domains", &token).await;
    assert_eq!(
        status, 403,
        "my domains must require workflow.read: {body:?}"
    );
}

#[tokio::test]
async fn my_domains_accepts_obo_token_for_self_scoped_read() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id) = common::seed_principal_domain_with_owner(&pool).await;
    let delegating = Uuid::new_v4();
    let token = obo_token(owner_id, delegating, "workflow.read", &mock.key_pair);

    // Read-only self-scoped discovery works with OBO (principal = act.sub).
    let app = build_app(pool.clone(), &mock.url);
    let (status, body) = do_get(app, "/internal/v1/principals/me/domains", &token).await;
    assert_eq!(status, 200, "my domains via OBO: {body:?}");
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0]["domain_id"].as_str().unwrap(),
        &domain_id.to_string()
    );
}

// ---------------------------------------------------------------------------
// SVC_WORKFLOW_DOMAIN_MEMBERSHIP_CONTROL_PLANE_V1 (role grammar +
// already_member + domain_owner_delegation_forbidden)
// ---------------------------------------------------------------------------

async fn do_put_idem_body(
    app: axum::Router,
    path: &str,
    token: &str,
    key: &str,
    body: &str,
) -> (u16, Value) {
    let req = Request::builder()
        .method("PUT")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", key)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

async fn count_member_added_audits(
    pool: &sqlx::PgPool,
    owner_id: Uuid,
    domain_id: Uuid,
    member_id: Uuid,
) -> i64 {
    let resource = format!("{domain_id}/{member_id}");
    sqlx::query_scalar(
        "SELECT count(*) FROM workflow_security_audits \
         WHERE principal_id = $1 AND action = 'member_added' AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(resource)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// ACC-DMC-001: explicit role=DOMAIN_MEMBER grants member; response carries
/// the granted role. Also covers the truly absent body (no content-type).
#[tokio::test]
async fn add_member_explicit_role_domain_member() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let app = build_app(pool.clone(), &mock.url);
    let (status, body) =
        do_put_idem_body(app, &path, &owner_token, "role-key-1", r#"{"role":"DOMAIN_MEMBER"}"#)
            .await;
    assert_eq!(status, 200, "explicit DOMAIN_MEMBER: {body:?}");
    assert_eq!(body["role"].as_str().unwrap(), "DOMAIN_MEMBER");
    assert_eq!(body["principalId"].as_str().unwrap(), &member_id.to_string());

    // Absent body entirely (no content-type, no bytes) keeps the default.
    let other = Uuid::new_v4();
    sqlx::query("INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'other-agent', NULL, TRUE)")
        .bind(other).execute(&pool).await.unwrap();
    let app = build_app(pool.clone(), &mock.url);
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/internal/v1/domains/{domain_id}/members/{other}"))
        .header("authorization", format!("Bearer {owner_token}"))
        .header("idempotency-key", "role-key-2")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status().as_u16(), 200);
}

/// ACC-DMC-002: logical duplicate with a NEW key is 409 already_member —
/// no duplicate binding, no second success audit row.
#[tokio::test]
async fn add_member_logical_duplicate_returns_already_member() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let app = build_app(pool.clone(), &mock.url);
    let (s1, _) = do_put_idem_body(app, &path, &owner_token, "dup-key-1", "{}").await;
    assert_eq!(s1, 200);

    let app = build_app(pool.clone(), &mock.url);
    let (s2, b2) = do_put_idem_body(app, &path, &owner_token, "dup-key-2", "{}").await;
    assert_eq!(s2, 409, "logical duplicate: {b2:?}");
    assert_eq!(b2["error"]["code"], "already_member");

    // exactly one enabled binding
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM domain_role_bindings \
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_MEMBER' AND enabled = TRUE",
    )
    .bind(domain_id)
    .bind(member_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 1);

    // exactly one member_added audit row for the real mutation only
    let audits = count_member_added_audits(&pool, owner_id, domain_id, member_id).await;
    assert_eq!(audits, 1, "duplicate must not fabricate a second audit row");
}

/// ACC-DMC-003: replay returns the STORED outcome of each key — the grant
/// key keeps returning 200, the duplicate key keeps returning 409.
#[tokio::test]
async fn already_member_outcome_is_replay_stable() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let app = build_app(pool.clone(), &mock.url);
    let (s1, _) = do_put_idem_body(app, &path, &owner_token, "replay-a", "{}").await;
    assert_eq!(s1, 200);
    let app = build_app(pool.clone(), &mock.url);
    let (s2, _) = do_put_idem_body(app, &path, &owner_token, "replay-b", "{}").await;
    assert_eq!(s2, 409);

    let app = build_app(pool.clone(), &mock.url);
    let (s1r, b1r) = do_put_idem_body(app, &path, &owner_token, "replay-a", "{}").await;
    assert_eq!(s1r, 200, "grant-key replay must return the stored 200: {b1r:?}");
    assert_eq!(b1r["principalId"].as_str().unwrap(), &member_id.to_string());

    let app = build_app(pool.clone(), &mock.url);
    let (s2r, b2r) = do_put_idem_body(app, &path, &owner_token, "replay-b", "{}").await;
    assert_eq!(s2r, 409, "duplicate-key replay must return the stored 409: {b2r:?}");
    assert_eq!(b2r["error"]["code"], "already_member");

    // still exactly one binding + one audit row after all replays
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM domain_role_bindings \
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_MEMBER' AND enabled = TRUE",
    )
    .bind(domain_id)
    .bind(member_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 1);
    let audits = count_member_added_audits(&pool, owner_id, domain_id, member_id).await;
    assert_eq!(audits, 1);
}

/// ACC-DMC-004: role=DOMAIN_OWNER is 403 domain_owner_delegation_forbidden,
/// creates no binding, and is replay-stable through the receipt.
#[tokio::test]
async fn add_member_domain_owner_role_forbidden() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let app = build_app(pool.clone(), &mock.url);
    let (status, body) =
        do_put_idem_body(app, &path, &owner_token, "owner-role-1", r#"{"role":"DOMAIN_OWNER"}"#)
            .await;
    assert_eq!(status, 403, "DOMAIN_OWNER delegation: {body:?}");
    assert_eq!(body["error"]["code"], "domain_owner_delegation_forbidden");

    // no member binding was created and no audit row written
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM domain_role_bindings \
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_MEMBER' AND enabled = TRUE",
    )
    .bind(domain_id)
    .bind(member_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 0);
    let audits = count_member_added_audits(&pool, owner_id, domain_id, member_id).await;
    assert_eq!(audits, 0);

    // replay of the same key returns the stored 403 (receipt-completed)
    let app = build_app(pool.clone(), &mock.url);
    let (status2, body2) =
        do_put_idem_body(app, &path, &owner_token, "owner-role-1", r#"{"role":"DOMAIN_OWNER"}"#)
            .await;
    assert_eq!(status2, 403);
    assert_eq!(body2["error"]["code"], "domain_owner_delegation_forbidden");
}

/// ACC-DMC-005: malformed grammar (unknown role, unknown field) is 400
/// invalid_input and never silently reinterpreted.
#[tokio::test]
async fn add_member_unknown_role_rejected() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let app = build_app(pool.clone(), &mock.url);
    let (status, body) =
        do_put_idem_body(app, &path, &owner_token, "bad-role-1", r#"{"role":"ADMIN"}"#).await;
    assert_eq!(status, 400, "unknown role: {body:?}");
    assert_eq!(body["error"]["code"], "invalid_input");

    let app = build_app(pool.clone(), &mock.url);
    let (status2, body2) = do_put_idem_body(
        app,
        &path,
        &owner_token,
        "bad-role-2",
        r#"{"role":"DOMAIN_MEMBER","extra":1}"#,
    )
    .await;
    assert_eq!(status2, 400, "unknown field: {body2:?}");
    assert_eq!(body2["error"]["code"], "invalid_input");

    // nothing was granted
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM domain_role_bindings \
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_MEMBER' AND enabled = TRUE",
    )
    .bind(domain_id)
    .bind(member_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 0);
}

/// ACC-DMC-003 (hash dimension): the same key with a different role is a
/// different logical command ⇒ 409 idempotency_conflict.
#[tokio::test]
async fn add_member_same_key_different_role_conflict() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let app = build_app(pool.clone(), &mock.url);
    let (s1, _) = do_put_idem_body(app, &path, &owner_token, "same-key-x", "{}").await;
    assert_eq!(s1, 200);

    let app = build_app(pool.clone(), &mock.url);
    let (s2, b2) = do_put_idem_body(
        app,
        &path,
        &owner_token,
        "same-key-x",
        r#"{"role":"DOMAIN_OWNER"}"#,
    )
    .await;
    assert_eq!(s2, 409, "same key different role: {b2:?}");
    assert_eq!(b2["error"]["code"], "idempotency_conflict");
}

/// Regression guard: remove-then-re-add across separate commands is legal
/// (already_member only fires on a concurrent enabled binding).
#[tokio::test]
async fn remove_then_re_add_is_allowed() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id, member_id) = seed_owner_member_scenario(&pool).await;
    let owner_token = direct_token(owner_id, "workflow.execute", &mock.key_pair);
    let path = format!("/internal/v1/domains/{domain_id}/members/{member_id}");

    let app = build_app(pool.clone(), &mock.url);
    let (s1, _) = do_put_idem_body(app, &path, &owner_token, "cycle-1", "{}").await;
    assert_eq!(s1, 200);
    let app = build_app(pool.clone(), &mock.url);
    let (sd, _) = do_del_idem(app, &path, &owner_token, "cycle-del").await;
    assert_eq!(sd, 200);
    let app = build_app(pool.clone(), &mock.url);
    let (s2, b2) = do_put_idem_body(app, &path, &owner_token, "cycle-2", "{}").await;
    assert_eq!(s2, 200, "re-add after remove: {b2:?}");
}
