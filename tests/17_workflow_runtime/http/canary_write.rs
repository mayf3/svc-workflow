//! Auth V1 single-agent read-write canary integration tests.
//!
//! Covers fail-closed guard behavior, write enable/disable gating,
//! positive write flows, auth negatives, and switch regression checks.
//!
//! All write tests exercise the full HTTP path through `canary_write_guard`
//! and the `AuthenticatedPrincipal` extractor.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthMode, AuthV1CanaryConfig, JwksConfig};
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use svc_workflow::http::{self, AppState, HttpConfig};

use super::*;

// ---------------------------------------------------------------------------
// Test RSA key material (same as jwks_auth.rs)
// ---------------------------------------------------------------------------

const TEST_RSA_N: &str = "zLFR5xYtoavfN3HKTpix5__zi4MXpiWYQAqa__FHkONKDj14yFnk9DV2QMcc6v_jCYqWD8arZ39oNPNz9mtEthOScwv-ORQQh3JxcCltZsgDTdzPsXpN61wkcWVU9fgaWjdQBssL3D1cd3vBLyYYb0qVkXFtwmf2r_s9PjrbtViQPuG9Xhh-L5pGfLsptN3C2-K8vy9I6A-R4YdD3pLdue-X5P3gQObbxLiLzekdR_ZTNsNCukqksj_JxcdVIxwuatg6OYuOPhyGEZb6kedoaJMqLxmCF5lEse_pNaDFOuIIt01hflru9ibhnZ0KK1-7351Flef6xf7JzatGIWmreQ";
const TEST_RSA_E: &str = "AQAB";
const TEST_RSA_PRIVATE_KEY_PEM: &str = "-----BEGIN RSA PRIVATE KEY-----\nMIIEogIBAAKCAQEAzLFR5xYtoavfN3HKTpix5//zi4MXpiWYQAqa//FHkONKDj14\nyFnk9DV2QMcc6v/jCYqWD8arZ39oNPNz9mtEthOScwv+ORQQh3JxcCltZsgDTdzP\nsXpN61wkcWVU9fgaWjdQBssL3D1cd3vBLyYYb0qVkXFtwmf2r/s9PjrbtViQPuG9\nXhh+L5pGfLsptN3C2+K8vy9I6A+R4YdD3pLdue+X5P3gQObbxLiLzekdR/ZTNsNC\nukqksj/JxcdVIxwuatg6OYuOPhyGEZb6kedoaJMqLxmCF5lEse/pNaDFOuIIt01h\nflru9ibhnZ0KK1+7351Flef6xf7JzatGIWmreQIDAQABAoIBAAkSvxeoMwOck7to\nbthHCnPHM6t2dyDlP7dvAOnhbxOsD4dMEEOJQI3WpNRAPzbnes/cdcRjQQvIaP0X\n4YcFwDj16yLwYCd1jToDx6V6IKBSs1rLM+WhDz0ki3T/UeHJSpm/I+v5KiBsE+Iz\n+R826BRe0Pxuc7gPVa79SvysLTr/iq1dE545W0UEC1bAqXc2sJfaIFa10xIG3Gmk\nV46FW+8rZIzAmuR7OA1lWSG4f45m4x78/LgF/gb4xoXOG/NAB9d+hgq/NI0M+JxU\nAackLa9V2T4ECs8lUSuUek8XFgEiSAXQDr9dH3cbrCUR69AjHsVtJQlkli69GXKG\nmWjk9AECgYEA7tfZtZ73LfAcAkG7EWMzbI1yXKkRtzdiKT1EgrbfsPU7GwpwRqxO\nTW9P8ZmKvh5Npi5t0+QpMgQGGTbuI1LLO9EDP/oiOXI9DZtNEYeSa4zNoiKWKkMl\noPs2i4/kUNNPqMBW/JnRmoapM/9GWAv7xYjhw+tYVUrf6S2jnWHOGfkCgYEA22V3\ngjZdMblt2B7M9sE3cMixCp7elG9iM0hH77JThTK+NMFslbIE/VDKdifjPPq85fi5\n64fm7eGH5nBNRn2+6xBqH8PAdaTgSyPWpVkhL6kkNrjyTnjhPOHZAxgWEYKZw3LE\n/s7ej4vazYrE8voIJSwtDrSNZIFDsmShWlzgfYECgYBPJE8Lk4UsP6fIR6eI92oO\nyj/e3Fb2cu+f4qFU/uvYYyoWp7rUcDvyBLRkxg/nN3tbWX8i+zN7U0ICEOWP5ttZ\nEsUU6fl1N5lrbM54xIeMA7gPxY4kquNJGHTWgfORpLN8o18vjHibz4s5o5jXjAD9\nT4IfvVgjyw+u4GSavdHhYQKBgBTxaqcTaXIFsWagChDEAPbTMZNB9x1URJuAmt1W\nuIJOhbmjfSoNBEzqGWmOBTMc/Es3owfIwVKT5NUqgzXnawIlXvwJQ6X3RzHlCehe\nybwy+TIAFaFICLg3FvAkrHafcO4nVoa8WKJ7Rze3t3U6SOzDesmckqK1dDDjSkPF\n+egBAoGAV9k+JQZzLc5+XJgsm8htUS2b0MOipCaABLf8P6IISyiE3ccvEECuwjfS\nBHgT+w1o5NF/c1zANedBtHmfk5XIvrf/OWzXhEGSWXhBrn2LLPCuh1OOHDQlKvff\nqIPymQBoF0zFpZdyAbKy7b8/fji7yG0vXceAa3jO4xSn6eYhGPQ=\n-----END RSA PRIVATE KEY-----\n";

const JWKS_KID: &str = "canary-test-key-v1";

// ---------------------------------------------------------------------------
// Test claims struct matching V1DirectMachineAccess profile
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct V1TestClaims {
    iss: String,
    sub: String,
    aud: String,
    principal_type: String,
    client_id: String,
    token_use: String,
    #[serde(rename = "type")]
    token_type: String,
    version: String,
    scope: String,
    jti: String,
    iat: usize,
    nbf: usize,
    exp: usize,
}

/// Create an RS256 JWT matching the Auth V1 DirectMachineAccess profile.
fn v1_token(subject: Uuid, scope: &str, client_id: &str, exp_offset: i64) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = V1TestClaims {
        iss: "auth-service".to_string(),
        sub: subject.to_string(),
        aud: "svc-workflow".to_string(),
        principal_type: "agent".to_string(),
        client_id: client_id.to_string(),
        token_use: "access".to_string(),
        token_type: "access".to_string(),
        version: "v1".to_string(),
        scope: scope.to_string(),
        jti: format!("canary-test-{}", Uuid::new_v4()),
        iat: now,
        nbf: now,
        exp: (now as i64 + exp_offset) as usize,
    };
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(JWKS_KID.to_string());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();
    encode(&header, &claims, &key).unwrap()
}

/// Create a V1 token with an extra field that violates deny_unknown_fields.
fn v1_token_with_extra_field(
    subject: Uuid,
    scope: &str,
    client_id: &str,
    exp_offset: i64,
    extra_key: &str,
    extra_value: &str,
) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = serde_json::json!({
        "iss": "auth-service",
        "sub": subject.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": client_id,
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": scope,
        "jti": format!("canary-test-{}", Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": (now as i64 + exp_offset) as usize,
        extra_key: extra_value,
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(JWKS_KID.to_string());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();
    encode(&header, &claims, &key).unwrap()
}

// ---------------------------------------------------------------------------
// Mock JWKS server
// ---------------------------------------------------------------------------

struct MockJwksServer {
    url: String,
    #[allow(dead_code)]
    shutdown: tokio::sync::oneshot::Sender<()>,
}

impl MockJwksServer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{addr}/.well-known/jwks.json");
        let (shutdown, mut rx) = tokio::sync::oneshot::channel::<()>();

        let body = serde_json::json!({
            "keys": [{
                "kty": "RSA",
                "use": "sig",
                "alg": "RS256",
                "kid": JWKS_KID,
                "n": TEST_RSA_N,
                "e": TEST_RSA_E,
            }]
        })
        .to_string();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        if let Ok((stream, _)) = result {
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                body.len(), body
                            );
                            let mut writer = tokio::io::BufWriter::new(stream);
                            let _ = writer.write_all(resp.as_bytes()).await;
                            let _ = writer.flush().await;
                        }
                    }
                    _ = &mut rx => break,
                }
            }
        });

        Self { url, shutdown }
    }
}

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
        auth_mode: AuthMode::Jwks,
        hs256_config: None,
        jwks_config: Some(JwksConfig {
            jwks_url: jwks_url.to_string(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 30,
            http_timeout_secs: 5,
            max_stale_secs: 60,
            clock_skew_seconds: 0,
        }),
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

/// 1. Read canary preserved: workflow.read V1 token → assigned-to-me returns 200.
#[tokio::test]
async fn canary_read_preserved() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (_owner, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let principal = _owner;
    let allowed_client = "canary-client";
    let canary = canary_config(true, false, allowed_client, &principal.to_string());
    let app = build_app(pool.clone(), &mock.url, canary);

    // Wait for eager JWKS fetch
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(principal, "workflow.read", allowed_client, 300);
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

/// 2. Write enabled + workflow.execute → create instance returns 201.
#[tokio::test]
async fn canary_write_create_instance() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _) = seed_published_definition(&pool, domain_id).await;
    let allowed_client = "canary-client";

    seed_domain_membership(&pool, domain_id, principal).await;

    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(
        principal,
        "workflow.execute workflow.read",
        allowed_client,
        300,
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
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "expected 201, got {:?}",
        json_body(resp).await
    );
    let created = json_body(resp).await;
    assert!(created["workflowInstanceId"].as_str().is_some());
}

/// 3. Write enabled + workflow.execute → transition returns 200.
#[tokio::test]
async fn canary_write_transition() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, advance_id) = seed_published_definition(&pool, domain_id).await;
    let allowed_client = "canary-client";

    seed_domain_membership(&pool, domain_id, principal).await;

    // Create instance via app layer (not HTTP) to avoid the canary guard
    let create_cmd = CreateWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(principal),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        domain_id: DomainId::from_uuid(domain_id),
        definition_version_id: DefinitionVersionId::from_uuid(ver_id),
        external_reference: None,
        external_url: None,
        metadata: json!({"source": "canary-transition-test"}),
        context_payload: json!({"hello": "world"}),
    };
    let created = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool, create_cmd,
    )
    .await
    .expect("create instance");
    let instance_id = created.workflow_instance_id;

    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(
        principal,
        "workflow.execute workflow.read",
        allowed_client,
        300,
    );
    let trans_body = json!({
        "transitionDefinitionId": advance_id,
        "expectedWorkflowStateVersion": 1,
        "submissionPayload": {"evidence": "canary-test"}
    });
    let uri = format!("/internal/v1/workflow-instances/{instance_id}/transitions");
    let resp = app
        .clone()
        .oneshot(write_request(&uri, &token, trans_body))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "expected 200, got {:?}",
        json_body(resp).await
    );
}

/// 4. Idempotency-Key: replay returns same result.
#[tokio::test]
async fn canary_write_idempotency() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _) = seed_published_definition(&pool, domain_id).await;
    let allowed_client = "canary-client";

    seed_domain_membership(&pool, domain_id, principal).await;

    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(
        principal,
        "workflow.execute workflow.read",
        allowed_client,
        300,
    );
    let idem_key = format!("canary-idem-{}", Uuid::new_v4());
    let body = json!({
        "domainId": domain_id,
        "definitionVersionId": ver_id,
        "metadata": {"source": "canary-idempotency"},
        "contextPayload": {"key": "value"}
    });

    let req_builder = Request::builder()
        .method("POST")
        .uri("/internal/v1/workflow-instances")
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", &idem_key)
        .header("content-type", "application/json");
    let req = req_builder.body(Body::from(body.to_string())).unwrap();

    // First call
    let resp1 = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::CREATED);

    // Second call (replay)
    let req_builder2 = Request::builder()
        .method("POST")
        .uri("/internal/v1/workflow-instances")
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", &idem_key)
        .header("content-type", "application/json");
    let req2 = req_builder2.body(Body::from(body.to_string())).unwrap();
    let resp2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::CREATED);

    let r1 = json_body(resp1).await;
    let r2 = json_body(resp2).await;
    assert_eq!(r1["workflowInstanceId"], r2["workflowInstanceId"]);
}

// ---------------------------------------------------------------------------
// Auth negatives — fail-closed
// ---------------------------------------------------------------------------

/// Recognised V1 token with Canary disabled → 401 (no fallback to legacy).
#[tokio::test]
async fn v1_token_canary_disabled_rejected_401() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(false, false, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(principal, "workflow.read", allowed_client, 300);
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
    assert_eq!(body["error"]["code"], "auth_v1_disabled");
}

/// Recognised V1 token with wrong client_id → 401 (no fallback).
#[tokio::test]
async fn v1_token_wrong_client_id_rejected_401() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let canary = canary_config(true, false, "allowed-client", &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(principal, "workflow.read", "wrong-client", 300);
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
    assert!(body["error"]["code"]
        .as_str()
        .unwrap()
        .contains("unauthorized"));
}

/// Recognised V1 token with wrong sub → 401 (no fallback).
#[tokio::test]
async fn v1_token_wrong_sub_rejected_401() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let other_principal = Uuid::new_v4();
    let canary = canary_config(true, false, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Token has other_principal as sub, but canary allows only `principal`
    let token = v1_token(other_principal, "workflow.read", allowed_client, 300);
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
    assert!(body["error"]["code"]
        .as_str()
        .unwrap()
        .contains("unauthorized"));
}

/// V1 token with write disabled → POST create returns 403.
#[tokio::test]
async fn v1_token_write_disabled_rejected_403() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _) = seed_published_definition(&pool, domain_id).await;
    let allowed_client = "canary-client";

    let canary = canary_config(true, false, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(
        principal,
        "workflow.execute workflow.read",
        allowed_client,
        300,
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

/// V1 token with write disabled → GET assigned-to-me still works.
#[tokio::test]
async fn v1_token_write_disabled_read_still_works() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";

    let canary = canary_config(true, false, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(principal, "workflow.read", allowed_client, 300);
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

/// V1 token missing workflow.execute scope → POST create returns 403.
#[tokio::test]
async fn v1_token_missing_execute_scope_rejected_403() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _) = seed_published_definition(&pool, domain_id).await;
    let allowed_client = "canary-client";

    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Token has only workflow.read, not workflow.execute
    let token = v1_token(principal, "workflow.read", allowed_client, 300);
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

/// V1 token with wrong issuer → 401.
#[tokio::test]
async fn v1_token_wrong_issuer_rejected_401() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
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
        "jti": "wrong-issuer-test-jti",
        "iat": now,
        "nbf": now,
        "exp": now + 300
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(JWKS_KID.to_string());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();
    let token = encode(&header, &claims, &key).unwrap();

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

/// V1 token with wrong audience → 401.
#[tokio::test]
async fn v1_token_wrong_audience_rejected_401() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
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
        "jti": "wrong-audience-test-jti",
        "iat": now,
        "nbf": now,
        "exp": now + 300
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(JWKS_KID.to_string());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();
    let token = encode(&header, &claims, &key).unwrap();

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

/// Wrong signature → 401.
#[tokio::test]
async fn v1_token_wrong_signature_rejected_401() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let mut token = v1_token(principal, "workflow.read", allowed_client, 300);
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

/// Expired token → 401.
#[tokio::test]
async fn v1_token_expired_rejected_401() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(principal, "workflow.read", allowed_client, -3600);

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

/// Wrong principal_type → 401.
#[tokio::test]
async fn v1_token_wrong_principal_type_rejected_401() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
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
        "principal_type": "human",  // wrong — must be "agent"
        "client_id": allowed_client,
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read",
        "jti": "wrong-ptype-test-jti",
        "iat": now,
        "nbf": now,
        "exp": now + 300
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(JWKS_KID.to_string());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();
    let token = encode(&header, &claims, &key).unwrap();

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

/// Wrong token_use → 401.
#[tokio::test]
async fn v1_token_wrong_token_use_rejected_401() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
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
        "jti": "wrong-tuse-test-jti",
        "iat": now,
        "nbf": now,
        "exp": now + 300
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(JWKS_KID.to_string());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM.as_bytes()).unwrap();
    let token = encode(&header, &claims, &key).unwrap();

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

/// HS256 token with canary active → not recognised as V1 token → falls through
/// to legacy JWKS verifier → rejected because HS256 in JWKS mode.
#[tokio::test]
async fn hs256_token_not_v1_shape_falls_through() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let _now = chrono::Utc::now().timestamp() as usize;
    let claims = json!({
        "iss": "auth-service",
        "sub": principal.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "type": "access",
        "version": "v1",
        "scope": "workflow.read"
    });
    let hs256_token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(b"dummy-secret"),
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
    // In JWKS mode, HS256 should be rejected by the legacy verifier
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// V1 token with extra field (agent_id) → not recognised as V1 shape →
/// falls through to legacy JWKS verifier.
#[tokio::test]
async fn v1_token_with_extra_field_falls_through() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Token has RS256 + kid + valid claims but also has "agent_id" → not V1 shape
    let token = v1_token_with_extra_field(
        principal,
        "workflow.read",
        allowed_client,
        300,
        "agent_id",
        "some-agent",
    );

    // Since the token matches legacy JWKS verifier's WorkflowClaims (agent_id is accepted there),
    // it should pass through to legacy and be accepted by the JWKS verifier.
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
    // Legacy JWKS mode should accept the token
    assert_eq!(resp.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Switch regression
// ---------------------------------------------------------------------------

/// Both flags default-off → Legacy HS256 mode still works (tested in other files).
/// Here we verify that when both flags are off, a V1 token is NOT accepted
/// (it gets identified and rejected by the fail-closed guard).
#[tokio::test]
async fn both_flags_default_off_v1_token_rejected() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let allowed_client = "canary-client";
    // Both flags default to false
    let canary = canary_config(false, false, allowed_client, &principal.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(principal, "workflow.read", allowed_client, 300);
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

/// Domain-level authorization preserved: Agent cannot create instance in domain they don't own.
#[tokio::test]
async fn domain_authorization_preserved() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    // Create two domains with their owners
    let (owner_a, domain_a) = seed_principal_domain_with_owner(&pool).await;
    let (_owner_b, domain_b) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_a, _) = seed_published_definition(&pool, domain_a).await;
    let allowed_client = "canary-client";

    // Owner A has domain membership in domain_a but NOT domain_b
    seed_domain_membership(&pool, domain_a, owner_a).await;

    let canary = canary_config(true, true, allowed_client, &owner_a.to_string());
    let app = build_app(pool, &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(
        owner_a,
        "workflow.execute workflow.read",
        allowed_client,
        300,
    );
    // Try to create in domain_b where owner_a has no domain permission
    let body = json!({
        "domainId": domain_b,
        "definitionVersionId": ver_a, // ver_a belongs to domain_a, not domain_b
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
    // Should be rejected by domain authorization check
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// Write canary: principal from token.sub, not from request body.
#[tokio::test]
async fn actor_comes_from_token_sub() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let (principal, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _) = seed_published_definition(&pool, domain_id).await;
    let allowed_client = "canary-client";

    seed_domain_membership(&pool, domain_id, principal).await;

    let canary = canary_config(true, true, allowed_client, &principal.to_string());
    let app = build_app(pool.clone(), &mock.url, canary);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let token = v1_token(
        principal,
        "workflow.execute workflow.read",
        allowed_client,
        300,
    );
    let body = json!({
        "domainId": domain_id,
        "definitionVersionId": ver_id,
        "metadata": {"source": "actor-test"},
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

    // Verify the creator in the database was set to token.sub
    let created = json_body(resp).await;
    let instance_id_str = created["workflowInstanceId"].as_str().unwrap().to_string();
    let instance_uuid = Uuid::parse_str(&instance_id_str).unwrap();
    let creator_row: (Uuid,) = sqlx::query_as(
        "SELECT created_by_principal_id FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_uuid)
    .fetch_one(&pool)
    .await
    .expect("find instance");
    assert_eq!(creator_row.0, principal, "creator must be token.sub");
}
