//! HTTP integration tests for the agent-facing domain management endpoints:
//!
//!   POST /internal/v1/domains              (canonical create domain)
//!   PUT  /internal/v1/domains/{domainId}/owner (set domain owner)
//!
//! Authorization model under test (SVC_WORKFLOW_DOMAIN_CREATE_CANONICAL_CONTRACT_V1):
//!   - create: coarse scope `workflow.execute` + direct token + enabled AGENT
//!     actor; the domainId is server-generated and the caller becomes the
//!     domain's single enabled DOMAIN_OWNER in the same transaction
//!     (creator-becomes-owner). GLOBAL_WORKFLOW_COORDINATOR is NOT a create
//!     prerequisite — the pre-V1 coordinator-only create gate was superseded
//!     by the Owner directive DOMAIN_CREATE_CANONICAL_CONTRACT_ALIGNMENT_V1.
//!   - set_owner: GLOBAL_WORKFLOW_COORDINATOR verified server-side from
//!     `global_role_bindings` (never in the JWT); plain agents denied (403)
//!   - OBO (delegated) tokens are denied (direct token required)
//!   - idempotency: same key replays the stored response (stable domainId);
//!     duplicate domainKey under a new key → 409 domain_identity_conflict

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

async fn seed_agent_with_id(pool: &PgPool, id: Uuid) {
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'Test Agent', NULL, TRUE)",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("seed agent with id");
}

async fn seed_disabled_agent(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'Disabled Agent', NULL, FALSE)",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("seed disabled agent");
    id
}

async fn seed_human(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'HUMAN', 'Test Human', NULL, TRUE)",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("seed human");
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

async fn domain_count_by_key(pool: &PgPool, domain_key: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM domains WHERE domain_key = $1")
        .bind(domain_key)
        .fetch_one(pool)
        .await
        .expect("count domains")
}

// ============================================================================
// Canonical create (SVC_WORKFLOW_DOMAIN_CREATE_CANONICAL_CONTRACT_V1)
// ============================================================================

#[tokio::test]
async fn create_domain_succeeds_server_generated_id_and_replays() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let caller = seed_agent(&pool).await;
    let domain_key = format!("canonical-create-{}", Uuid::new_v4());

    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(caller, "workflow.execute", &mock.key_pair);

    // Business inputs only — no domainId anywhere in the request.
    let (status, body) = do_post(
        app.clone(),
        "/internal/v1/domains",
        &token,
        json!({
            "domainKey": domain_key,
            "displayName": "Canonical Created Domain",
            "enabled": true
        }),
        "canonical-create-1",
    )
    .await;
    assert_eq!(status, 200, "create domain must succeed: {body}");

    let domain_id: Uuid = body["domainId"]
        .as_str()
        .expect("domainId present")
        .parse()
        .expect("domainId is a UUID");
    assert_eq!(body["domainKey"], domain_key);
    assert_eq!(body["ownerPrincipalId"], caller.to_string());
    assert!(domain_enabled(&pool, domain_id).await);
    // Creator became the single enabled DOMAIN_OWNER in the create transaction.
    assert!(owner_binding_enabled(&pool, domain_id, caller).await);

    // Same idempotency key replays the stored response: same server-generated
    // domainId, no second domain row.
    let (status2, body2) = do_post(
        app.clone(),
        "/internal/v1/domains",
        &token,
        json!({
            "domainKey": domain_key,
            "displayName": "Canonical Created Domain",
            "enabled": true
        }),
        "canonical-create-1",
    )
    .await;
    assert_eq!(status2, 200, "replay must succeed: {body2}");
    assert_eq!(body2["domainId"], domain_id.to_string());
    assert_eq!(domain_count_by_key(&pool, &domain_key).await, 1);
}

#[tokio::test]
async fn plain_agent_creates_domain_and_becomes_owner() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    // No GLOBAL_WORKFLOW_COORDINATOR binding — creator-becomes-owner is the
    // canonical create authorization; the blast radius is the caller's own
    // new domain.
    let plain_agent = seed_agent(&pool).await;
    let plain_key = format!("plain-agent-create-{}", Uuid::new_v4());

    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(plain_agent, "workflow.execute", &mock.key_pair);

    let (status, body) = do_post(
        app,
        "/internal/v1/domains",
        &token,
        json!({
            "domainKey": plain_key,
            "enabled": false
        }),
        "plain-agent-create-1",
    )
    .await;
    assert_eq!(status, 200, "plain agent create must succeed: {body}");
    let domain_id: Uuid = body["domainId"]
        .as_str()
        .expect("domainId present")
        .parse()
        .expect("domainId is a UUID");
    assert_eq!(body["ownerPrincipalId"], plain_agent.to_string());
    // displayName absent → defaults to the domainKey.
    assert_eq!(body["displayName"], plain_key);
    assert!(
        !domain_enabled(&pool, domain_id).await,
        "enabled=false is honored"
    );
    // The owner binding is established even for a disabled domain.
    assert!(owner_binding_enabled(&pool, domain_id, plain_agent).await);
}

#[tokio::test]
async fn duplicate_domain_key_conflicts_with_zero_mutation() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let first = seed_agent(&pool).await;
    let second = seed_agent(&pool).await;
    let dup_key = format!("dup-key-{}", Uuid::new_v4());

    let app = build_app(pool.clone(), &mock.url);

    let (status, body) = do_post(
        app.clone(),
        "/internal/v1/domains",
        &direct_token(first, "workflow.execute", &mock.key_pair),
        json!({ "domainKey": dup_key, "enabled": true }),
        "dup-key-first",
    )
    .await;
    assert_eq!(status, 200, "first create must succeed: {body}");

    // A different caller + new key on an occupied domainKey → 409, zero rows.
    let (status2, body2) = do_post(
        app.clone(),
        "/internal/v1/domains",
        &direct_token(second, "workflow.execute", &mock.key_pair),
        json!({ "domainKey": dup_key, "enabled": true }),
        "dup-key-second",
    )
    .await;
    assert_eq!(status2, 409, "duplicate domainKey must conflict: {body2}");
    assert_eq!(body2["error"]["code"], "domain_identity_conflict");
    assert_eq!(domain_count_by_key(&pool, &dup_key).await, 1);

    // Same key, changed business inputs → hash mismatch → idempotency_conflict.
    let (status3, body3) = do_post(
        app,
        "/internal/v1/domains",
        &direct_token(first, "workflow.execute", &mock.key_pair),
        json!({ "domainKey": dup_key, "displayName": "Renamed", "enabled": true }),
        "dup-key-first",
    )
    .await;
    assert_eq!(
        status3, 409,
        "changed inputs under same key must conflict: {body3}"
    );
    assert_eq!(body3["error"]["code"], "idempotency_conflict");
}

#[tokio::test]
async fn caller_supplied_domain_id_is_rejected_unknown_field() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let caller = seed_agent(&pool).await;
    let legacy_key = format!("legacy-domain-id-{}", Uuid::new_v4());

    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(caller, "workflow.execute", &mock.key_pair);

    // The legacy caller-supplied contract exited: domainId is not a field of
    // the canonical create request, and deny_unknown_fields rejects it
    // instead of silently ignoring or requiring it.
    let (status, body) = do_post(
        app,
        "/internal/v1/domains",
        &token,
        json!({
            "domainId": Uuid::new_v4(),
            "domainKey": legacy_key,
            "enabled": true
        }),
        "legacy-domain-id-1",
    )
    .await;
    assert_eq!(
        status, 400,
        "caller-supplied domainId must be rejected: {body}"
    );
    assert_eq!(body["error"]["code"], "unknown_field");
    assert_eq!(domain_count_by_key(&pool, &legacy_key).await, 0);
}

#[tokio::test]
async fn disabled_actor_denied_for_create_domain() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let disabled = seed_disabled_agent(&pool).await;

    let app = build_app(pool.clone(), &mock.url);
    // A still-valid JWT for a disabled principal passes the auth layer
    // (JWT-only validation) and is rejected by the actor validation.
    let token = direct_token(disabled, "workflow.execute", &mock.key_pair);

    let (status, body) = do_post(
        app,
        "/internal/v1/domains",
        &token,
        json!({ "domainKey": format!("disabled-actor-{}", Uuid::new_v4()), "enabled": true }),
        "disabled-actor-1",
    )
    .await;
    assert_eq!(status, 403, "disabled actor must be denied: {body}");
    assert_eq!(body["error"]["code"], "principal_disabled");
}

#[tokio::test]
async fn human_actor_denied_for_create_domain() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let human = seed_human(&pool).await;

    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(human, "workflow.execute", &mock.key_pair);

    let (status, body) = do_post(
        app,
        "/internal/v1/domains",
        &token,
        json!({ "domainKey": format!("human-actor-{}", Uuid::new_v4()), "enabled": true }),
        "human-actor-1",
    )
    .await;
    assert_eq!(status, 403, "HUMAN actor must be denied: {body}");
    assert_eq!(body["error"]["code"], "principal_type_not_allowed");
}

#[tokio::test]
async fn canonical_principal_create_chain() {
    let pool = common::create_pool().await;
    let canonical_principal_id = std::env::var("CANONICAL_CREATE_TEST_PRINCIPAL_ID")
        .ok()
        .map(|v| {
            v.parse::<Uuid>()
                .expect("CANONICAL_CREATE_TEST_PRINCIPAL_ID must be a UUID")
        });
    let Some(principal_id) = canonical_principal_id else {
        eprintln!(
            "skipped: set CANONICAL_CREATE_TEST_PRINCIPAL_ID to a seeded canonical \
             principal UUID to prove the real-identity create chain"
        );
        return;
    };
    let mock = common::MockJwksServer::start().await;
    seed_agent_with_id(&pool, principal_id).await;

    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(principal_id, "workflow.execute", &mock.key_pair);

    let (status, body) = do_post(
        app,
        "/internal/v1/domains",
        &token,
        json!({
            "domainKey": format!("canonical-principal-create-{}", Uuid::new_v4()),
            "displayName": "Canonical Principal Chain",
            "enabled": true
        }),
        "canonical-principal-create-1",
    )
    .await;
    assert_eq!(
        status, 200,
        "canonical principal create must succeed: {body}"
    );
    assert_eq!(body["ownerPrincipalId"], principal_id.to_string());
    let domain_id: Uuid = body["domainId"]
        .as_str()
        .expect("domainId present")
        .parse()
        .expect("domainId is a UUID");
    assert!(owner_binding_enabled(&pool, domain_id, principal_id).await);
}

// ============================================================================
// Coordinator control plane (unchanged surfaces)
// ============================================================================

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
async fn non_coordinator_agent_denied_for_set_domain_owner() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let plain_agent = seed_agent(&pool).await;
    let plain_key = format!("plain-agent-create-{}", Uuid::new_v4());
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
async fn read_scope_denied() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let caller = seed_agent(&pool).await;

    let app = build_app(pool.clone(), &mock.url);
    // workflow.read is not enough — writes require workflow.execute.
    let token = direct_token(caller, "workflow.read", &mock.key_pair);

    let (status, body) = do_post(
        app,
        "/internal/v1/domains",
        &token,
        json!({
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
    let caller = seed_agent(&pool).await;
    let delegating = Uuid::new_v4();

    let app = build_app(pool.clone(), &mock.url);
    // Delegated (OBO) tokens are rejected — writes need a direct access token.
    let token = obo_token(caller, delegating, "workflow.execute", &mock.key_pair);

    let (status, body) = do_post(
        app,
        "/internal/v1/domains",
        &token,
        json!({
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
