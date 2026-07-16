//! Provisioning API integration tests.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthMode, Hs256Config};
use svc_workflow::domain::ids::PrincipalId;
use svc_workflow::http::{self, AppState, HttpConfig};

use super::*;

const JWT_SECRET: &str = "provisioning-test-secret-at-least-32-bytes";

// Provisioning allow-list principal
const PROVISIONING_PRINCIPAL_ID: &str = "11111111-1111-1111-1111-111111111111";

#[derive(Serialize)]
struct Claims {
    sub: String,
    iss: &'static str,
    aud: &'static str,
    exp: usize,
    iat: usize,
    principal_type: &'static str,
    #[serde(rename = "type")]
    token_type: &'static str,
    version: &'static str,
    scope: String,
}

fn token(principal_id: Uuid, scope: &str) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    encode(
        &Header::new(Algorithm::HS256),
        &Claims {
            sub: principal_id.to_string(),
            iss: "auth-service",
            aud: "svc-workflow",
            exp: now + 300,
            iat: now,
            principal_type: "agent",
            token_type: "access",
            version: "v1",
            scope: scope.to_string(),
        },
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .unwrap()
}

fn provisioning_token() -> String {
    token(
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
        "workflow.admin workflow.read workflow.execute",
    )
}

fn regular_token() -> String {
    token(Uuid::new_v4(), "workflow.read workflow.execute")
}

/// Seed the provisioning actor as a real principal in the database.
/// Must be called before `app(pool)` since the app uses the pool.
async fn seed_provisioning_actor(pool: &sqlx::PgPool) {
    let id = Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
         VALUES ($1, 'AGENT', 'Provisioning Actor', NULL, TRUE)
         ON CONFLICT (principal_id) DO NOTHING",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("seed provisioning actor");

    // Clean up stale PROCESSING receipts from interrupted test runs
    sqlx::query(
        "DELETE FROM workflow_command_receipts
         WHERE principal_id = $1 AND receipt_status = 'PROCESSING'",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("clean stale processing receipts");
}

fn app(pool: sqlx::PgPool) -> axum::Router {
    app_for_actor(pool, Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap())
}

fn app_for_actor(pool: sqlx::PgPool, actor_id: Uuid) -> axum::Router {
    let hs256 = Hs256Config {
        secret: JWT_SECRET.to_string(),
        issuer: "auth-service".to_string(),
        audience: "svc-workflow".to_string(),
        clock_skew_seconds: 0,
    };
    let config = HttpConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        request_body_max_bytes: 2_097_152,
        request_timeout_seconds: 30,
        auth_mode: AuthMode::TestHs256,
        hs256_config: Some(hs256),
        jwks_config: None,
        provisioning_config: ProvisioningConfig::new(vec![PrincipalId::from_uuid(actor_id)]),
    };
    http::router(AppState::new(pool, &config), &config)
}

fn request(
    method: &str,
    uri: &str,
    token: Option<&str>,
    key: Option<&str>,
    body: Option<Value>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if let Some(key) = key {
        builder = builder.header("idempotency-key", key);
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap()
}

fn unique_key(prefix: &str) -> String {
    format!("{}-{}", prefix, Uuid::new_v4())
}

fn unique_domain_key(prefix: &str) -> String {
    format!("{}-{}", prefix, Uuid::new_v4())
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// 1. No token → 401
#[tokio::test]
async fn no_token_rejected() {
    let pool = create_pool().await;
    seed_provisioning_actor(&pool).await;
    let app = app(pool);
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            None,
            Some(&unique_key("key-1")),
            Some(json!({
                "principalId": Uuid::new_v4(),
                "principalType": "human",
                "enabled": true,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 2. Missing workflow.admin scope → 403
#[tokio::test]
async fn missing_scope_rejected() {
    let pool = create_pool().await;
    seed_provisioning_actor(&pool).await;
    let app = app(pool);
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&regular_token()),
            Some(&unique_key("key-2")),
            Some(json!({
                "principalId": Uuid::new_v4(),
                "principalType": "human",
                "enabled": true,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// 3+5. Valid provisioning token can create principal
#[tokio::test]
async fn create_principal_success() {
    let pool = create_pool().await;
    seed_provisioning_actor(&pool).await;
    let app = app(pool);
    let principal_id = Uuid::new_v4();
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token()),
            Some(&unique_key("key-3")),
            Some(json!({
                "principalId": principal_id,
                "principalType": "agent",
                "enabled": true,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    let status = resp.status();
    let body = json_body(resp).await;
    assert_eq!(status, StatusCode::OK, "create principal should succeed");
    assert_eq!(body["principalId"], principal_id.to_string());
    assert_eq!(body["enabled"], true);
}

/// 7-8. Create HUMAN + AGENT
#[tokio::test]
async fn create_human_and_agent() {
    let pool = create_pool().await;
    seed_provisioning_actor(&pool).await;
    let app = app(pool);

    // HUMAN
    let human_id = Uuid::new_v4();
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token()),
            Some(&unique_key("key-human")),
            Some(json!({
                "principalId": human_id,
                "principalType": "human",
                "enabled": true,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // AGENT
    let agent_id = Uuid::new_v4();
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token()),
            Some(&unique_key("key-agent")),
            Some(json!({
                "principalId": agent_id,
                "principalType": "agent",
                "enabled": true,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

/// 9. Idempotent replay
#[tokio::test]
async fn idempotent_replay() {
    let pool = create_pool().await;
    seed_provisioning_actor(&pool).await;
    let app = app(pool);
    let principal_id = Uuid::new_v4();
    let body = json!({
        "principalId": principal_id,
        "principalType": "agent",
        "enabled": true,
        "source": "auth-service"
    });
    let key = unique_key("replay-key");

    let resp1 = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token()),
            Some(&key),
            Some(body.clone()),
        ))
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    let resp2 = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token()),
            Some(&key),
            Some(body),
        ))
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    assert_eq!(json_body(resp1).await, json_body(resp2).await);
}

/// 12. Modify enabled
#[tokio::test]
async fn modify_principal_enabled() {
    let pool = create_pool().await;
    seed_provisioning_actor(&pool).await;
    let app = app(pool);
    let principal_id = Uuid::new_v4();

    // Create disabled
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token()),
            Some(&unique_key("key-enable-1")),
            Some(json!({
                "principalId": principal_id,
                "principalType": "agent",
                "enabled": false,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["enabled"], false);

    // Enable
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token()),
            Some(&unique_key("key-enable-2")),
            Some(json!({
                "principalId": principal_id,
                "principalType": "agent",
                "enabled": true,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["enabled"], true);
}

/// 10. principal_type conflict
#[tokio::test]
async fn principal_type_conflict() {
    let pool = create_pool().await;
    seed_provisioning_actor(&pool).await;
    let app = app(pool);
    let principal_id = Uuid::new_v4();

    // Create as agent
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token()),
            Some(&unique_key("key-type-1")),
            Some(json!({
                "principalId": principal_id,
                "principalType": "agent",
                "enabled": true,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Try to change type
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token()),
            Some(&unique_key("key-type-2")),
            Some(json!({
                "principalId": principal_id,
                "principalType": "human",
                "enabled": true,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    assert_eq!(
        json_body(resp).await["error"]["code"],
        "principal_type_conflict"
    );
}

/// 16-17. Create Domain
#[tokio::test]
async fn create_domain_success() {
    let pool = create_pool().await;
    seed_provisioning_actor(&pool).await;
    let app = app(pool);
    let domain_id = Uuid::new_v4();
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/domains",
            Some(&provisioning_token()),
            Some(&unique_key("key-domain")),
            Some(json!({
                "domainId": domain_id,
                "domainKey": &unique_domain_key("test-domain"),
                "displayName": "Test Domain",
                "enabled": true
            })),
        ))
        .await
        .unwrap();
    let status = resp.status();
    let body = json_body(resp).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["domainId"], domain_id.to_string());
    assert!(
        !body["domainKey"].as_str().unwrap_or("").is_empty(),
        "domainKey should be non-empty"
    );
}

#[path = "provisioning_regressions.rs"]
mod regressions;
