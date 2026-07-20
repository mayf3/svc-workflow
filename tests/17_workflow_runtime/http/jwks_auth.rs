//! JWKS-mode integration tests for the V1 DirectMachineAccess profile.
//!
//! Tests the RS256 JWKS verifier using a local mock JWKS endpoint.
//! Only V1 DirectMachineAccess tokens are accepted — OBO tokens and HS256
//! are rejected at the verifier level.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, error::ApiError, AppState, HttpConfig};

use super::*;

// ---------------------------------------------------------------------------
// Helper: build app + config in JWKS mode
// ---------------------------------------------------------------------------

fn jwks_config(bind_addr: std::net::SocketAddr, jwks_url: &str) -> HttpConfig {
    HttpConfig {
        bind_addr,
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
        auth_v1_canary_config: AuthV1CanaryConfig {
            enabled: true,
            write_enabled: true,
            allowed_client_id: String::new(),
            allowed_sub: String::new(),
            allowed_delegating_sub: String::new(),
            jwks_url: String::new(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 30,
            http_timeout_secs: 5,
            max_stale_secs: 60,
            clock_skew_seconds: 0,
        },
    }
}

// ---------------------------------------------------------------------------
// Request/response helpers
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

#[allow(dead_code)]
async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn verify_token(
    state: &AppState,
    token: &str,
) -> Result<svc_workflow::auth::AuthenticatedPrincipal, ApiError> {
    state.auth_verifier.verify(token).await
}

// ---------------------------------------------------------------------------
// Tests — V1 DirectMachineAccess profile
// ---------------------------------------------------------------------------

/// 1. Valid token: JWKS mode is operational (readyz healthcheck).
#[tokio::test]
async fn valid_token_and_readyz() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    // Wait for eager JWKS fetch
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let app = http::router(state, &config);
    let resp = app
        .clone()
        .oneshot(request("GET", "/readyz", None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["status"], "ready");
}

/// 2. Valid RS256 direct Agent token accepted.
#[tokio::test]
async fn valid_direct_agent_token() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token(
        Uuid::new_v4(),
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let result = verify_token(&state, &token).await;
    assert!(result.is_ok());
    let principal = result.unwrap();
    assert_eq!(principal.auth_context.token_use, "access");
    assert_eq!(principal.auth_context.principal_type, "agent");
}

/// 3. Wrong issuer rejected.
#[tokio::test]
async fn wrong_issuer_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "wrong-issuer",
        "sub": Uuid::new_v4().to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": "test-client",
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300,
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 4. Wrong audience rejected.
#[tokio::test]
async fn wrong_audience_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": Uuid::new_v4().to_string(),
        "aud": "wrong-audience",
        "principal_type": "agent",
        "client_id": "test-client",
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300,
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 5. Wrong signature rejected.
#[tokio::test]
async fn wrong_signature_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let mut token = common::v1_token(
        Uuid::new_v4(),
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    // Replace last char to invalidate signature
    let len = token.len();
    token.replace_range(len - 1..len, "X");

    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 6. Expired token rejected.
#[tokio::test]
async fn expired_token_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": Uuid::new_v4().to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": "test-client",
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now - 1200,
        "nbf": now - 1200,
        "exp": now - 600,
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), "token_expired");
}

/// 7. Wrong principal_type rejected.
#[tokio::test]
async fn wrong_principal_type_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": Uuid::new_v4().to_string(),
        "aud": "svc-workflow",
        "principal_type": "human",
        "client_id": "test-client",
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300,
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 8. Wrong token_use rejected.
#[tokio::test]
async fn wrong_token_use_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": Uuid::new_v4().to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": "test-client",
        "token_use": "invalid_use",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300,
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 9. HS256 token rejected in JWKS mode.
#[tokio::test]
async fn hs256_token_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let claims = json!({
        "iss": "auth-service",
        "sub": Uuid::new_v4().to_string(),
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
    let result = verify_token(&state, &hs256_token).await;
    assert!(result.is_err());
}

/// 10. Missing kid header rejected.
#[tokio::test]
async fn missing_kid_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // RS256 token without kid header
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": Uuid::new_v4().to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": "test-client",
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300,
    });
    // No kid in header
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 11. Token with extra field rejected (deny_unknown_fields).
#[tokio::test]
async fn extra_field_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_token_with_extra_field(
        Uuid::new_v4(),
        "workflow.read",
        "test-client",
        300,
        "extra_field",
        "rejected",
        &mock.key_pair,
    );

    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// OBO token verifier tests
// ---------------------------------------------------------------------------

/// 12. Valid OBO token accepted.
#[tokio::test]
async fn valid_obo_token() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let sub = Uuid::new_v4();
    let act_sub = Uuid::new_v4();
    let token = common::v1_obo_token(
        sub,
        act_sub,
        "workflow.read",
        Some("test-client"),
        300,
        &mock.key_pair,
    );
    let result = verify_token(&state, &token).await;
    assert!(result.is_ok());
    let principal = result.unwrap();
    // Domain actor is always token.sub
    assert_eq!(principal.principal_id.into_uuid(), sub);
    assert_eq!(principal.auth_context.token_use, "workflow_obo");
    assert_eq!(
        principal
            .auth_context
            .delegating_principal_id
            .map(|id| id.into_uuid()),
        Some(act_sub)
    );
    assert_eq!(
        principal.auth_context.client_id.as_deref(),
        Some("test-client")
    );
    assert_eq!(principal.auth_context.principal_type, "agent");
}

/// 13. OBO token without `act` claim rejected.
#[tokio::test]
async fn obo_token_missing_act_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_obo_token_missing_act(
        Uuid::new_v4(),
        "workflow.read",
        Some("test-client"),
        300,
        &mock.key_pair,
    );
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 14. OBO token with extra top-level fields rejected.
#[tokio::test]
async fn obo_token_extra_field_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = common::v1_obo_token_with_extra_field(
        Uuid::new_v4(),
        Uuid::new_v4(),
        "workflow.read",
        Some("test-client"),
        300,
        "extra_field",
        "rejected",
        &mock.key_pair,
    );
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 15. Direct token carrying `act` claim rejected (deny_unknown_fields on V1DirectMachineClaims).
#[tokio::test]
async fn direct_token_with_act_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Direct token carrying act claim (should be rejected)
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = serde_json::json!({
        "iss": "auth-service",
        "sub": Uuid::new_v4().to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": "test-client",
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300,
        "act": { "sub": Uuid::new_v4().to_string() }
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 16. OBO token's act.sub must be a valid UUID.
#[tokio::test]
async fn obo_token_invalid_act_sub_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = serde_json::json!({
        "iss": "auth-service",
        "sub": Uuid::new_v4().to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": "test-client",
        "token_use": "workflow_obo",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300,
        "act": { "sub": "not-a-uuid" }
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 17. OBO token with nested act (extra field in act) rejected.
#[tokio::test]
async fn obo_token_nested_act_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = serde_json::json!({
        "iss": "auth-service",
        "sub": Uuid::new_v4().to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": "test-client",
        "token_use": "workflow_obo",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300,
        "act": { "sub": Uuid::new_v4().to_string(), "act": { "sub": "nested" } }
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}

/// 18. Unknown token_use rejected.
#[tokio::test]
async fn unknown_token_use_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let config = jwks_config("127.0.0.1:0".parse().unwrap(), &mock.url);
    let state = AppState::new(pool, &config);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = serde_json::json!({
        "iss": "auth-service",
        "sub": Uuid::new_v4().to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": "test-client",
        "token_use": "bogus_use",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": format!("test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": now + 300,
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(mock.key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(mock.key_pair.private_key_pem.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let result = verify_token(&state, &token).await;
    assert!(result.is_err());
}
