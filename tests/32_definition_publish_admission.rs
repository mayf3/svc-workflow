//! Definition publish identity-literal admission (CTR-CIR-003,
//! SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2).
//!
//! Coverage:
//!   PUBLISH_ADMITS_EVERY_IDENTITY_LITERAL (FIXED_PRINCIPAL + context-schema
//!       default literal; directory sees exactly the two distinct principals)
//!   PUBLISH_REJECTED_FAILS_CLOSED_ZERO_DELTA (HTTP 422 admission_rejected;
//!       the version stays DRAFT and the receipt rolls back, so the same
//!       idempotency key can retry)
//!   PUBLISH_INVALID_SCHEMA_LITERAL_FAILS_VALIDATION (422
//!       graph_validation_failed, INSTANCE_INPUT_LITERAL_NOT_UUID; zero
//!       directory traffic — validation precedes admission)
//!   PUBLISH_DORMANT_WHEN_DISABLED (existing behavior unchanged)
//!
//! The directory stub mirrors tests/31_admission_wiring.rs: a local loopback
//! HTTP server speaking exactly the pinned routes.

#[allow(dead_code, unused_imports)]
#[path = "common/mod.rs"]
mod common;

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::auth::admission::{AdmissionConfig, SecretString, ADMISSION_DEADLINE_MS};
use svc_workflow::http::{self, AppState, HttpConfig};

use common::MockJwksServer;

// ---------------------------------------------------------------------------
// Directory stub (loopback HTTP) — same shape as tests/31_admission_wiring.rs
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum StubMode {
    Admit,
    PrincipalDisabled,
}

struct StubDirectory {
    addr: SocketAddr,
    requests: Arc<Mutex<Vec<String>>>,
}

impl StubDirectory {
    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().expect("stub requests lock").clone()
    }

    fn spawn(mode: StubMode) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind stub listener");
        let addr = listener.local_addr().expect("stub addr");
        let requests: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        {
            let requests = requests.clone();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { break };
                    let requests = requests.clone();
                    std::thread::spawn(move || {
                        serve_connection(stream, mode, requests);
                    });
                }
            });
        }
        Self { addr, requests }
    }
}

fn serve_connection(
    mut stream: std::net::TcpStream,
    mode: StubMode,
    requests: Arc<Mutex<Vec<String>>>,
) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let Some((path, body)) = read_request(&mut stream) else {
        return;
    };
    requests.lock().expect("stub requests lock").push(path.clone());

    let principal_status = match mode {
        StubMode::Admit => "active",
        StubMode::PrincipalDisabled => "disabled",
    };

    let (status, response_body) = if path == "/oauth/token" {
        let scope = form_value(&body, "scope");
        (200, token_body(&scope))
    } else if let Some(rest) = path.strip_prefix("/api/v1/directory/principals/") {
        match rest.strip_suffix("/agent") {
            Some(principal) => (
                200,
                format!(
                    r#"{{"principalId":"{principal}","agentId":"agent-{principal}","principalStatus":"{principal_status}"}}"#
                ),
            ),
            None => (404, r#"{"error":"unexpected_path"}"#.to_string()),
        }
    } else if let Some(agent_id) = path.strip_prefix("/v1/directory/agents/") {
        (
            200,
            format!(r#"{{"agentId":"{agent_id}","exists":true,"enabled":true}}"#),
        )
    } else {
        (404, r#"{"error":"unexpected_path"}"#.to_string())
    };

    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        reason(status),
        response_body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(response_body.as_bytes());
    let _ = stream.flush();
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Status",
    }
}

fn token_body(scope: &str) -> String {
    format!(
        r#"{{"access_token":"tok","token_type":"Bearer","expires_in":3600,"scope":"{scope}"}}"#
    )
}

/// The stub keeps the most recent request body for the token endpoint.
static LAST_BODY: Mutex<String> = Mutex::new(String::new());

fn form_value(body: &str, key: &str) -> String {
    body.split('&')
        .find_map(|pair| {
            pair.split_once('=')
                .filter(|(name, _)| *name == key)
                .map(|(_, value)| value.to_string())
        })
        .unwrap_or_default()
}

fn read_request(stream: &mut std::net::TcpStream) -> Option<(String, String)> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 2048];
    let head_end;
    loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            head_end = pos;
            break;
        }
    }
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let path = head
        .lines()
        .next()?
        .split_whitespace()
        .nth(1)?
        .to_string();
    let content_length = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0);
    let body_start = head_end + 4;
    while buf.len() < body_start + content_length {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let body = String::from_utf8_lossy(&buf[body_start..body_start + content_length]).to_string();
    if path == "/oauth/token" {
        *LAST_BODY.lock().expect("body cache") = body.clone();
    }
    Some((path, body))
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

// ---------------------------------------------------------------------------
// HTTP harness (mirrors tests/31_admission_wiring.rs)
// ---------------------------------------------------------------------------

fn admission_config_enabled(base: &str) -> AdmissionConfig {
    AdmissionConfig {
        enabled: true,
        auth_base_url: base.to_string(),
        core_base_url: base.to_string(),
        client_id: "svc-workflow".to_string(),
        client_secret: SecretString::new("test-secret"),
        deadline_ms: ADMISSION_DEADLINE_MS,
        max_in_flight: 8,
    }
}

fn http_config(jwks_url: &str, admission: AdmissionConfig) -> HttpConfig {
    HttpConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        request_body_max_bytes: 2_097_152,
        request_timeout_seconds: 30,
        jwks_config: svc_workflow::auth::JwksConfig {
            jwks_url: jwks_url.to_string(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
        provisioning_config: svc_workflow::application::provisioning::ProvisioningConfig::new(
            Vec::new(),
        ),
        auth_v1_canary_config: svc_workflow::auth::AuthV1CanaryConfig {
            enabled: true,
            write_enabled: true,
            allowed_client_id: "publish-admission".to_string(),
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
        admission,
    }
}

fn build_app(pool: sqlx::PgPool, jwks_url: &str, admission: AdmissionConfig) -> axum::Router {
    let config = http_config(jwks_url, admission);
    let state = AppState::new(pool, &config);
    http::router(state, &config)
}

fn token(principal_id: Uuid, scope: &str, key_pair: &common::RsaTestKeyPair) -> String {
    common::v1_token(principal_id, scope, "publish-admission", 300, key_pair)
}

async fn send(
    app: axum::Router,
    method: &str,
    uri: &str,
    bearer: &str,
    key: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {bearer}"));
    if let Some(key) = key {
        builder = builder.header("idempotency-key", key);
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let response = app
        .oneshot(
            builder
                .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let parsed = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, parsed)
}

// ---------------------------------------------------------------------------
// Seeding: domain owner + two agents + a DRAFT legacy graph
//   draft (WORKFLOW_CREATOR) --advance--> assign (INSTANCE_INPUT_PRINCIPAL
//       key ownerAgent; schema default = agent_two)
//   assign --advance--> review (FIXED_PRINCIPAL agent_one)
//   review --advance--> done (TERMINAL)
// ---------------------------------------------------------------------------

struct Fixture {
    caller: Uuid,
    domain_id: Uuid,
    definition_id: Uuid,
    version_id: Uuid,
}

struct SeedGraph {
    /// Context schema carried by the draft version.
    context_schema: Value,
}

async fn seed(pool: &sqlx::PgPool, graph: SeedGraph) -> Fixture {
    let (caller, domain_id) = common::seed_principal_domain_with_owner(pool).await;
    // The FIXED_PRINCIPAL target must exist locally (publish validates
    // existence); the schema default literal is validated by the directory,
    // not the local principal table.
    let agent_one = common::seed_second_principal(pool).await;

    let definition_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let def_key = format!("pub-adm-{}", &Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) \
         VALUES ($1, $2, $3, 'Publish Admission Def')",
    )
    .bind(definition_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(pool)
    .await
    .expect("insert definition");

    sqlx::query(
        "INSERT INTO workflow_definition_versions \
         (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) \
         VALUES ($1, $2, 1, 'DRAFT', $3)",
    )
    .bind(version_id)
    .bind(definition_id)
    .bind(graph.context_schema.clone())
    .execute(pool)
    .await
    .expect("insert draft version");

    let draft = Uuid::new_v4();
    let assign = Uuid::new_v4();
    let review = Uuid::new_v4();
    let done = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) \
         VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')",
    )
    .bind(draft)
    .bind(version_id)
    .execute(pool)
    .await
    .expect("insert draft node");

    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, assignee_input_key) \
         VALUES ($1, $2, 'assign', 'Assign', 1, 'NORMAL', 'INSTANCE_INPUT_PRINCIPAL', 'ownerAgent')",
    )
    .bind(assign)
    .bind(version_id)
    .execute(pool)
    .await
    .expect("insert assign node");

    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) \
         VALUES ($1, $2, 'review', 'Review', 2, 'NORMAL', 'FIXED_PRINCIPAL', $3)",
    )
    .bind(review)
    .bind(version_id)
    .bind(agent_one)
    .execute(pool)
    .await
    .expect("insert review node");

    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) \
         VALUES ($1, $2, 'done', 'Done', 3, 'TERMINAL', NULL)",
    )
    .bind(done)
    .bind(version_id)
    .execute(pool)
    .await
    .expect("insert done node");

    let edges: [(Uuid, Uuid, &str); 3] = [
        (draft, assign, "advance-assign"),
        (assign, review, "advance-review"),
        (review, done, "advance-done"),
    ];
    for (source, target, key) in edges.iter() {
        let transition_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_transition_definitions \
             (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) \
             VALUES ($1, $2, $3, 'Advance', $4, $5, 'ADVANCE')",
        )
        .bind(transition_id)
        .bind(version_id)
        .bind(key)
        .bind(source)
        .bind(target)
        .execute(pool)
        .await
        .expect("insert transition");
        sqlx::query(
            "UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2",
        )
        .bind(transition_id)
        .bind(source)
        .execute(pool)
        .await
        .expect("set primary advance");
    }

    Fixture {
        caller,
        domain_id,
        definition_id,
        version_id,
    }
}

fn default_literal_schema(agent_two: Uuid) -> Value {
    json!({
        "type": "object",
        "required": ["ownerAgent"],
        "properties": {
            "ownerAgent": {
                "type": "string",
                "default": agent_two.to_string(),
            }
        }
    })
}

async fn version_status(pool: &sqlx::PgPool, version_id: Uuid) -> String {
    let (status,): (String,) = sqlx::query_as(
        "SELECT version_status::TEXT FROM workflow_definition_versions WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .fetch_one(pool)
    .await
    .expect("version readback");
    status
}

async fn receipt_count(pool: &sqlx::PgPool, principal: Uuid, key: &str) -> i64 {
    // Definition governance receipts live in the generic command receipt
    // table keyed by (principal, idempotency key).
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_command_receipts \
         WHERE idempotency_key = $1 AND principal_id = $2",
    )
    .bind(key)
    .bind(principal)
    .fetch_one(pool)
    .await
    .expect("count receipts");
    count
}

fn distinct_auth_principals(requests: &[String]) -> Vec<String> {
    let mut principals: Vec<String> = requests
        .iter()
        .filter_map(|p| p.strip_prefix("/api/v1/directory/principals/"))
        .map(|s| s.to_string())
        .collect();
    principals.sort();
    principals.dedup();
    principals
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// (a) Publish with admission enabled + admitting directory: the version
/// publishes, and the directory observed EXACTLY the distinct identity
/// literals — the FIXED_PRINCIPAL agent plus the context-schema default
/// literal for the INSTANCE_INPUT_PRINCIPAL key.
#[tokio::test]
async fn publish_admits_every_identity_literal_and_publishes() {
    let pool = common::create_pool().await;
    let agent_two_schema = default_literal_schema(Uuid::new_v4());
    // Re-seed with a known agent_two UUID: build the fixture, then read the
    // review node's fixed principal back for assertion symmetry.
    let fixture = seed(&pool, SeedGraph { context_schema: agent_two_schema.clone() }).await;

    let (agent_one,): (Option<Uuid>,) = sqlx::query_as(
        "SELECT fixed_principal_id FROM workflow_node_definitions \
         WHERE definition_version_id = $1 AND node_key = 'review'",
    )
    .bind(fixture.version_id)
    .fetch_one(&pool)
    .await
    .expect("review node readback");
    let agent_one = agent_one.expect("fixed principal set");

    let stub = StubDirectory::spawn(StubMode::Admit);
    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url, admission_config_enabled(&stub.url()));

    let bearer = token(fixture.caller, "workflow.execute", &mock.key_pair);
    let (status, body) = send(
        app,
        "POST",
        &format!(
            "/internal/v1/domains/{}/definitions/{}/publish",
            fixture.domain_id, fixture.definition_id
        ),
        &bearer,
        Some("pub-adm-ok"),
        Some(json!({"versionId": fixture.version_id})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["versionStatus"], "PUBLISHED", "body: {body}");
    assert_eq!(version_status(&pool, fixture.version_id).await, "PUBLISHED");

    // The directory must have observed BOTH literals: the FIXED_PRINCIPAL
    // agent and the schema default identity value.
    let principals = distinct_auth_principals(&stub.requests());
    assert_eq!(
        principals,
        vec![
            format!("{}/agent", agent_one),
            format!("{}/agent", agent_two_id(&agent_two_schema)),
        ],
        "exactly the two distinct identity literals admitted: {principals:?}"
    );
    // Fresh token per exact read audience per command.
    assert_eq!(
        stub.requests()
            .iter()
            .filter(|p| p.as_str() == "/oauth/token")
            .count(),
        2
    );
}

/// Extract the default literal UUID from the seeded schema (test helper).
fn agent_two_id(schema: &Value) -> String {
    schema["properties"]["ownerAgent"]["default"]
        .as_str()
        .expect("default literal present")
        .to_string()
}

/// (b) Directory rejects an identity literal: the publish fails closed with
/// 422 admission_rejected, the version stays DRAFT, and the receipt rolls
/// back so the SAME idempotency key can retry against an admitting directory
/// (no cached negative result — CTR-CIR-003).
#[tokio::test]
async fn publish_rejected_fails_closed_zero_delta() {
    let pool = common::create_pool().await;
    let agent_two = Uuid::new_v4();
    let fixture = seed(
        &pool,
        SeedGraph {
            context_schema: default_literal_schema(agent_two),
        },
    )
    .await;

    let rejecting = StubDirectory::spawn(StubMode::PrincipalDisabled);
    let admitting = StubDirectory::spawn(StubMode::Admit);
    let mock = MockJwksServer::start().await;
    let app = build_app(
        pool.clone(),
        &mock.url,
        admission_config_enabled(&rejecting.url()),
    );

    let bearer = token(fixture.caller, "workflow.execute", &mock.key_pair);
    let (status, body) = send(
        app,
        "POST",
        &format!(
            "/internal/v1/domains/{}/definitions/{}/publish",
            fixture.domain_id, fixture.definition_id
        ),
        &bearer,
        Some("pub-adm-rejected"),
        Some(json!({"versionId": fixture.version_id})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "body: {body}");
    assert_eq!(body["error"]["code"], "admission_rejected");

    // Zero business delta: the version stays DRAFT and no receipt persists.
    assert_eq!(version_status(&pool, fixture.version_id).await, "DRAFT");
    assert_eq!(receipt_count(&pool, fixture.caller, "pub-adm-rejected").await, 0);
    assert_eq!(distinct_auth_principals(&rejecting.requests()).len(), 2);

    // Retry with the SAME idempotency key against an admitting directory.
    let mock2 = MockJwksServer::start().await;
    let app2 = build_app(
        pool.clone(),
        &mock2.url,
        admission_config_enabled(&admitting.url()),
    );
    let bearer2 = token(fixture.caller, "workflow.execute", &mock2.key_pair);
    let (status, body) = send(
        app2,
        "POST",
        &format!(
            "/internal/v1/domains/{}/definitions/{}/publish",
            fixture.domain_id, fixture.definition_id
        ),
        &bearer2,
        Some("pub-adm-rejected"),
        Some(json!({"versionId": fixture.version_id})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(version_status(&pool, fixture.version_id).await, "PUBLISHED");
}

/// (c) A present-but-invalid schema identity literal is rejected by
/// VALIDATION (422 graph_validation_failed, INSTANCE_INPUT family code)
/// BEFORE any directory traffic; the version stays DRAFT.
#[tokio::test]
async fn publish_invalid_schema_literal_fails_validation_without_directory() {
    let pool = common::create_pool().await;
    let mut schema = default_literal_schema(Uuid::new_v4());
    schema["properties"]["ownerAgent"]["default"] = json!("not-a-uuid");
    let fixture = seed(&pool, SeedGraph { context_schema: schema }).await;

    let stub = StubDirectory::spawn(StubMode::Admit);
    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url, admission_config_enabled(&stub.url()));

    let bearer = token(fixture.caller, "workflow.execute", &mock.key_pair);
    let (status, body) = send(
        app,
        "POST",
        &format!(
            "/internal/v1/domains/{}/definitions/{}/publish",
            fixture.domain_id, fixture.definition_id
        ),
        &bearer,
        Some("pub-adm-bad-literal"),
        Some(json!({"versionId": fixture.version_id})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "body: {body}");
    assert_eq!(body["error"]["code"], "graph_validation_failed");
    let rendered = serde_json::to_string(&body["error"]).unwrap_or_default();
    assert!(
        rendered.contains("INSTANCE_INPUT_LITERAL_NOT_UUID"),
        "INSTANCE_INPUT grammar error expected: {rendered}"
    );

    assert_eq!(version_status(&pool, fixture.version_id).await, "DRAFT");
    // Validation precedes admission: zero directory traffic.
    assert!(stub.requests().is_empty(), "{:?}", stub.requests());
}

/// (d) Admission disabled (dormant deploy): publish behavior unchanged, no
/// directory configured at all.
#[tokio::test]
async fn publish_dormant_when_disabled() {
    let pool = common::create_pool().await;
    let fixture = seed(
        &pool,
        SeedGraph {
            context_schema: default_literal_schema(Uuid::new_v4()),
        },
    )
    .await;

    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url, AdmissionConfig::disabled());

    let bearer = token(fixture.caller, "workflow.execute", &mock.key_pair);
    let (status, body) = send(
        app,
        "POST",
        &format!(
            "/internal/v1/domains/{}/definitions/{}/publish",
            fixture.domain_id, fixture.definition_id
        ),
        &bearer,
        Some("pub-adm-dormant"),
        Some(json!({"versionId": fixture.version_id})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["versionStatus"], "PUBLISHED");
    assert_eq!(version_status(&pool, fixture.version_id).await, "PUBLISHED");
}
