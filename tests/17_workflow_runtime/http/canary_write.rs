//! Auth V1 integration tests — write gate, positive flows, negatives.
//!
//! Covers fail-closed guard behavior, write enable/disable gating,
//! positive write flows, and auth negatives.
//!
//! All write tests exercise the full HTTP path through `canary_write_guard`
//! and the `AuthenticatedPrincipal` extractor.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};

use super::*;

// ---------------------------------------------------------------------------
// Canary config helpers
// ---------------------------------------------------------------------------

fn canary_config(
    enabled: bool,
    write_enabled: bool,
    allowed_client_id: &str,
    allowed_sub: &str,
) -> AuthV1CanaryConfig {
    AuthV1CanaryConfig {
        enabled,
        write_enabled,
        allowed_client_id: allowed_client_id.to_string(),
        allowed_sub: allowed_sub.to_string(),
        jwks_url: String::new(), // populated before use
        issuer: "auth-service".to_string(),
        audience: "svc-workflow".to_string(),
        cache_ttl_secs: 30,
        http_timeout_secs: 5,
        max_stale_secs: 60,
        clock_skew_seconds: 0,
    }
}

fn build_app(pool: sqlx::PgPool, jwks_url: &str, canary: AuthV1CanaryConfig) -> axum::Router {
    let mut canary_config = canary;
    canary_config.jwks_url = jwks_url.to_string();
    let http_config = HttpConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        request_body_max_bytes: 2_097_152,
        request_timeout_seconds: 30,
        jwks_config: JwksConfig {
            jwks_url: jwks_url.to_string(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 30,
            http_timeout_secs: 5,
            max_stale_secs: 60,
            clock_skew_seconds: 0,
        },
        provisioning_config: ProvisioningConfig::new(Vec::new()),
        auth_v1_canary_config: canary_config,
    };
    let state = AppState::new(pool, &http_config);
    http::router(state, &http_config)
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

fn request(method: &str, uri: &str, token: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap()
}

/// Create a POST request with an auto-generated Idempotency-Key header.
fn write_request(uri: &str, token: &str, body: Value) -> Request<Body> {
    let idem_key = uuid::Uuid::new_v4().to_string();
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("idempotency-key", &idem_key)
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_domain_membership(pool: &sqlx::PgPool, domain_id: Uuid, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1, $2, $3, 'MEMBER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("add domain membership");
}

/// Seed a published definition where DRAFT uses WORKFLOW_CREATOR.
async fn seed_published_definition(pool: &sqlx::PgPool, domain_id: Uuid) -> (Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("canary-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query("INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Canary Test')")
        .bind(def_id).bind(domain_id).bind(&def_key)
        .execute(pool).await.expect("insert def");

    sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1, $2, 1, 'DRAFT', '{\"type\":\"object\"}'::jsonb)")
        .bind(ver_id).bind(def_id)
        .execute(pool).await.expect("insert version");

    let draft_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();

    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')")
        .bind(draft_id).bind(ver_id)
        .execute(pool).await.expect("insert draft node");

    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', NULL)")
        .bind(term_id).bind(ver_id)
        .execute(pool).await.expect("insert terminal node");

    let advance_id = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance-done', 'Complete', $3, $4, 'ADVANCE')")
        .bind(advance_id).bind(ver_id).bind(draft_id).bind(term_id)
        .execute(pool).await.expect("insert transition");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(advance_id).bind(draft_id)
        .execute(pool).await.expect("set primary");

    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(pool).await.expect("publish version");

    (domain_id, ver_id, advance_id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// 1. Read: V1 token with workflow.read scope → assigned-to-me returns 200.
#[tokio::test]
async fn v1_read_assigned_to_me() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let principal = owner;
    let allowed_client = "canary-client";
    let canary = canary_config(true, false, allowed_client, &principal.to_string());
    let app = build_app(pool.clone(), &mock.url, canary);

    // Wait for eager JWKS fetch
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token(
        principal,
        "workflow.read",
        allowed_client,
        300,
        &mock.key_pair,
    );
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

/// 2. Write enabled + workflow.execute scope → create instance returns 201.
#[tokio::test]
async fn v1_write_create_instance() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _) = seed_published_definition(&pool, domain_id).await;
    let allowed_client = "canary-client";

    seed_domain_membership(&pool, domain_id, principal).await;

    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token(
        principal,
        "workflow.execute workflow.read",
        allowed_client,
        300,
        &mock.key_pair,
    );
    let body = json!({
        "domainId": domain_id,
        "definitionVersionId": ver_id,
        "metadata": {"source": "canary-write"},
        "contextPayload": {"hello": "world"}
    });
    let resp = app
        .clone()
        .oneshot(write_request(
            "/internal/v1/workflow-instances",
            &token,
            body,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

/// 3. Write enabled → valid transition (advance) returns 200.
#[tokio::test]
async fn v1_write_transition() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, advance_id) = seed_published_definition(&pool, domain_id).await;
    let allowed_client = "canary-client";

    seed_domain_membership(&pool, domain_id, principal).await;

    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool.clone(), &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Create an instance first
    let create_token = common::v1_token(
        principal,
        "workflow.execute workflow.read",
        allowed_client,
        300,
        &mock.key_pair,
    );
    let create_body = json!({
        "domainId": domain_id,
        "definitionVersionId": ver_id,
        "metadata": {"source": "canary-transition"},
        "contextPayload": {"hello": "world"}
    });
    let create_resp = app
        .clone()
        .oneshot(write_request(
            "/internal/v1/workflow-instances",
            &create_token,
            create_body,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let create_json = json_body(create_resp).await;
    let instance_id = create_json["workflowInstanceId"]
        .as_str()
        .unwrap()
        .to_string();

    // Transition
    let transition_token = common::v1_token(
        principal,
        "workflow.execute workflow.read",
        allowed_client,
        300,
        &mock.key_pair,
    );
    let transition_body = json!({
        "transitionId": advance_id,
        "submissionPayload": null
    });
    let trans_resp = app
        .clone()
        .oneshot(write_request(
            &format!("/internal/v1/workflow-instances/{instance_id}/transitions"),
            &transition_token,
            transition_body,
        ))
        .await
        .unwrap();
    assert_eq!(trans_resp.status(), StatusCode::OK);
}

/// 4. V1 token write disabled → POST create returns 403.
#[tokio::test]
async fn v1_write_disabled_rejected_403() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _) = seed_published_definition(&pool, domain_id).await;
    let allowed_client = "canary-client";

    let canary = canary_config(true, false, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token(
        principal,
        "workflow.execute workflow.read",
        allowed_client,
        300,
        &mock.key_pair,
    );
    let body = json!({
        "domainId": domain_id,
        "definitionVersionId": ver_id,
        "metadata": {},
        "contextPayload": {}
    });
    let resp = app
        .clone()
        .oneshot(write_request(
            "/internal/v1/workflow-instances",
            &token,
            body,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "canary_read_only");
}

/// 5. Write disabled → GET assigned-to-me still works.
#[tokio::test]
async fn v1_write_disabled_read_still_works() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";

    let canary = canary_config(true, false, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token(
        principal,
        "workflow.read",
        allowed_client,
        300,
        &mock.key_pair,
    );
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

/// 6. V1 disabled → all requests rejected 401.
#[tokio::test]
async fn v1_disabled_rejected_401() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(false, false, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token(
        principal,
        "workflow.read",
        allowed_client,
        300,
        &mock.key_pair,
    );
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 7. Wrong client_id → 401.
#[tokio::test]
async fn v1_wrong_client_id_rejected_401() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let canary = canary_config(true, false, "allowed-client", &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token(
        principal,
        "workflow.read",
        "wrong-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 8. Wrong sub → 401.
#[tokio::test]
async fn v1_wrong_sub_rejected_401() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let other_principal = Uuid::new_v4();
    let canary = canary_config(true, false, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token(
        other_principal,
        "workflow.read",
        allowed_client,
        300,
        &mock.key_pair,
    );
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 9. Missing workflow.execute scope → POST create returns 403.
#[tokio::test]
async fn v1_missing_execute_scope_rejected_403() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _) = seed_published_definition(&pool, domain_id).await;
    let allowed_client = "canary-client";

    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token(
        principal,
        "workflow.read",
        allowed_client,
        300,
        &mock.key_pair,
    );
    let body = json!({
        "domainId": domain_id,
        "definitionVersionId": ver_id,
        "metadata": {},
        "contextPayload": {}
    });
    let resp = app
        .clone()
        .oneshot(write_request(
            "/internal/v1/workflow-instances",
            &token,
            body,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// 10. Wrong issuer → 401.
#[tokio::test]
async fn v1_wrong_issuer_rejected_401() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "wrong-issuer",
        "sub": principal.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": allowed_client,
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 11. Wrong audience → 401.
#[tokio::test]
async fn v1_wrong_audience_rejected_401() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": principal.to_string(),
        "aud": "wrong-audience",
        "principal_type": "agent",
        "client_id": allowed_client,
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 12. Wrong signature → 401.
#[tokio::test]
async fn v1_wrong_signature_rejected_401() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let mut token = common::v1_token(
        principal,
        "workflow.read",
        allowed_client,
        300,
        &mock.key_pair,
    );
    let len = token.len();
    token.replace_range(len - 1..len, "X");

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 13. Expired token → 401.
#[tokio::test]
async fn v1_expired_rejected_401() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token(
        principal,
        "workflow.read",
        allowed_client,
        -3600,
        &mock.key_pair,
    );

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "token_expired");
}

/// 14. Wrong principal_type → 401.
#[tokio::test]
async fn v1_wrong_principal_type_rejected_401() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": principal.to_string(),
        "aud": "svc-workflow",
        "principal_type": "human",
        "client_id": allowed_client,
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 15. Wrong token_use → 401.
#[tokio::test]
async fn v1_wrong_token_use_rejected_401() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": principal.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": allowed_client,
        "token_use": "invalid_use",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 16. V1 token with extra field (deny_unknown_fields) → rejected.
#[tokio::test]
async fn v1_token_with_extra_field_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token_with_extra_field(
        principal,
        "workflow.read",
        allowed_client,
        300,
        "extra_field",
        "rejected",
        &mock.key_pair,
    );

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    // deny_unknown_fields means this token is malformed → 401
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 17. HS256 token rejected in V1-only mode.
#[tokio::test]
async fn hs256_token_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": principal.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read"
    });
    let hs256_token = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(b"dummy-secret"),
    )
    .unwrap();

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&hs256_token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 18. Missing kid → rejected.
#[tokio::test]
async fn v1_missing_kid_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": principal.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": allowed_client,
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300
    });
    // No kid in header
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
