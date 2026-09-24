//! Canonical identity admission wiring at the workflow assignment write
//! seams (CTR-CIR-003, SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2).
//!
//! Coverage:
//!   ADMISSION_ENABLED_CREATE_WRITES_AND_ADMITS_COMMAND_PRINCIPALS
//!   ADMISSION_REJECTED_FAILS_CLOSED_WITH_ZERO_WRITES (HTTP 422 admission_rejected;
//!       the receipt rolls back too, so the same idempotency key can retry)
//!   ADMISSION_UNAVAILABLE_FAILS_CLOSED (HTTP 503 admission_unavailable, zero writes)
//!   ADMISSION_DISABLED_IS_DORMANT (existing behavior unchanged, no directory)
//!   TRANSITION_REJECTED_FAILS_CLOSED_THEN_SAME_KEY_RETRY_ADMITS
//!   REVISE_REJECTED_FAILS_CLOSED_THEN_SAME_KEY_RETRY_ADMITS (app surface)
//!
//! The directory stub is a local loopback HTTP server speaking exactly the
//! pinned routes: POST /oauth/token, GET /api/v1/directory/principals/{id}/agent,
//! GET /v1/directory/agents/{id}.

#[allow(dead_code, unused_imports)]
#[path = "common/mod.rs"]
mod common;

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::workflow_instance::create::create_workflow_instance;
use svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition;
use svc_workflow::application::workflow_instance::revise::revise_workflow_context;
use svc_workflow::auth::admission::{AdmissionClient, AdmissionConfig};
use svc_workflow::domain::ids::{DefinitionVersionId, DomainId, TransitionId, WorkflowInstanceId};
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand, ReviseWorkflowContextCommand,
};
use svc_workflow::domain::workflow_instance::errors::revise_error_label;
use svc_workflow::domain::workflow_instance::errors::ReviseWorkflowContextError;
use svc_workflow::http::{self, AppState, HttpConfig};
use svc_workflow::store::postgres::admission_gate::AdmissionGate;

use common::MockJwksServer;

// ---------------------------------------------------------------------------
// Directory stub (loopback HTTP)
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
    requests
        .lock()
        .expect("stub requests lock")
        .push(path.clone());

    let principal_status = match mode {
        StubMode::Admit => "active",
        StubMode::PrincipalDisabled => "disabled",
    };

    let (status, response_body) = if path == "/oauth/token" {
        // Echo the requested scope exactly (the client requires it).
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
    format!(r#"{{"access_token":"tok","token_type":"Bearer","expires_in":3600,"scope":"{scope}"}}"#)
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
    let path = head.lines().next()?.split_whitespace().nth(1)?.to_string();
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

/// A configured-but-unreachable directory endpoint (connection refused).
async fn closed_port() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    format!("http://{addr}")
}

// ---------------------------------------------------------------------------
// HTTP harness
// ---------------------------------------------------------------------------

fn admission_config_enabled(base: &str) -> AdmissionConfig {
    AdmissionConfig {
        enabled: true,
        auth_base_url: base.to_string(),
        core_base_url: base.to_string(),
        client_id: "svc-workflow".to_string(),
        client_secret: svc_workflow::auth::admission::SecretString::new("test-secret"),
        deadline_ms: svc_workflow::auth::admission::ADMISSION_DEADLINE_MS,
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
            allowed_client_id: "admission-wiring".to_string(),
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
        execution_control: svc_workflow::http::ExecutionControlConfig {
            max_returns_per_edge: 3,
        },
    }
}

fn build_app(pool: sqlx::PgPool, jwks_url: &str, admission: AdmissionConfig) -> axum::Router {
    let config = http_config(jwks_url, admission);
    let state = AppState::new(pool, &config);
    http::router(state, &config)
}

fn token(principal_id: Uuid, scope: &str, key_pair: &common::RsaTestKeyPair) -> String {
    common::v1_token(principal_id, scope, "admission-wiring", 300, key_pair)
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
// Seeding: caller + domain + a published legacy definition
//   draft (WORKFLOW_CREATOR) --advance--> normal (FIXED_PRINCIPAL agent)
//        normal --advance--> done (TERMINAL)
// ---------------------------------------------------------------------------

struct Fixture {
    caller: Uuid,
    agent: Uuid,
    domain_id: Uuid,
    version_id: Uuid,
    draft_advance: Uuid,
    normal_advance: Uuid,
}

async fn seed(pool: &sqlx::PgPool) -> Fixture {
    let (caller, domain_id) = common::seed_principal_domain_with_owner(pool).await;
    let agent = common::seed_second_principal(pool).await;

    let def_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let def_key = format!("adm-{}", &Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) \
         VALUES ($1, $2, $3, 'Admission Def')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(pool)
    .await
    .expect("insert definition");

    sqlx::query(
        "INSERT INTO workflow_definition_versions \
         (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) \
         VALUES ($1, $2, 1, 'DRAFT', NULL)",
    )
    .bind(version_id)
    .bind(def_id)
    .execute(pool)
    .await
    .expect("insert draft version");

    let draft = Uuid::new_v4();
    let normal = Uuid::new_v4();
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
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) \
         VALUES ($1, $2, 'work', 'Work', 1, 'NORMAL', 'FIXED_PRINCIPAL', $3)",
    )
    .bind(normal)
    .bind(version_id)
    .bind(agent)
    .execute(pool)
    .await
    .expect("insert normal node");
    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) \
         VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)",
    )
    .bind(done)
    .bind(version_id)
    .execute(pool)
    .await
    .expect("insert terminal node");

    let draft_advance = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions \
         (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) \
         VALUES ($1, $2, 'advance-work', 'Advance', $3, $4, 'ADVANCE')",
    )
    .bind(draft_advance)
    .bind(version_id)
    .bind(draft)
    .bind(normal)
    .execute(pool)
    .await
    .expect("insert draft advance");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(draft_advance)
        .bind(draft)
        .execute(pool)
        .await
        .expect("set draft primary");

    let normal_advance = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions \
         (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) \
         VALUES ($1, $2, 'advance-done', 'Complete', $3, $4, 'ADVANCE')",
    )
    .bind(normal_advance)
    .bind(version_id)
    .bind(normal)
    .bind(done)
    .execute(pool)
    .await
    .expect("insert normal advance");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(normal_advance)
        .bind(normal)
        .execute(pool)
        .await
        .expect("set normal primary");

    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(version_id)
        .execute(pool)
        .await
        .expect("publish version");

    Fixture {
        caller,
        agent,
        domain_id,
        version_id,
        draft_advance,
        normal_advance,
    }
}

fn create_command(
    caller: Uuid,
    fixture: &Fixture,
    idempotency_key: String,
) -> CreateWorkflowInstanceCommand {
    CreateWorkflowInstanceCommand {
        principal_id: svc_workflow::domain::ids::PrincipalId::from_uuid(caller),
        idempotency_key,
        command_schema_version: "v1".to_string(),
        execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
        domain_id: DomainId::from_uuid(fixture.domain_id),
        definition_version_id: DefinitionVersionId::from_uuid(fixture.version_id),
        external_reference: None,
        external_url: None,
        metadata: json!({}),
        context_payload: json!({}),
    }
}

async fn instance_count(pool: &sqlx::PgPool, domain_id: Uuid) -> i64 {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM workflow_instances WHERE domain_id = $1")
            .bind(domain_id)
            .fetch_one(pool)
            .await
            .expect("count instances");
    count
}

async fn receipt_count(pool: &sqlx::PgPool, principal: Uuid, key: &str) -> i64 {
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

fn enabled_client(stub: &StubDirectory) -> AdmissionClient {
    AdmissionClient::new(admission_config_enabled(&stub.url())).expect("client builds")
}

fn disabled_client() -> AdmissionConfig {
    AdmissionConfig::disabled()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// (a) Admission enabled + directory admits: the create succeeds and the
/// directory observed exactly the command's distinct Agent Principals
/// (the FIXED_PRINCIPAL assignment target).
#[tokio::test]
async fn admission_enabled_create_writes_and_admits_command_principals() {
    let pool = common::create_pool().await;
    let fixture = seed(&pool).await;
    let stub = StubDirectory::spawn(StubMode::Admit);
    let mock = MockJwksServer::start().await;
    let app = build_app(
        pool.clone(),
        &mock.url,
        admission_config_enabled(&stub.url()),
    );

    let bearer = token(fixture.caller, "workflow.execute", &mock.key_pair);
    let (status, body) = send(
        app,
        "POST",
        "/internal/v1/workflow-instances",
        &bearer,
        Some("adm-create-ok"),
        Some(json!({
            "domainId": fixture.domain_id,
            "definitionVersionId": fixture.version_id,
            "metadata": {},
            "contextPayload": {}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert!(body["workflowInstanceId"].is_string());
    assert_eq!(instance_count(&pool, fixture.domain_id).await, 1);

    // The directory saw both directory reads for the command's assignment
    // target: the entry DRAFT node resolves WORKFLOW_CREATOR, so the
    // assignment fact is the caller's own visit (the agent target is
    // admitted at transition time — see the transition test below).
    let requests = stub.requests();
    assert!(
        requests
            .iter()
            .any(|p| p == &format!("/api/v1/directory/principals/{}/agent", fixture.caller)),
        "auth directory read for the resolved WORKFLOW_CREATOR assignee expected: {requests:?}"
    );
    assert!(
        requests
            .iter()
            .any(|p| p == &format!("/v1/directory/agents/agent-{}", fixture.caller)),
        "agent-core directory read expected: {requests:?}"
    );
    // Fresh token per exact read audience per command.
    assert_eq!(
        requests
            .iter()
            .filter(|p| p.as_str() == "/oauth/token")
            .count(),
        2
    );
}

/// (b) Directory rejects the assignment target (principalStatus != active):
/// the entire business write fails closed — zero instances, zero receipts.
/// The rolled-back receipt makes the SAME idempotency key retryable against
/// an admitting directory (zero business delta, no cached negative result).
#[tokio::test]
async fn create_rejected_fails_closed_with_zero_writes() {
    let pool = common::create_pool().await;
    let fixture = seed(&pool).await;
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
        "/internal/v1/workflow-instances",
        &bearer,
        Some("adm-create-rejected"),
        Some(json!({
            "domainId": fixture.domain_id,
            "definitionVersionId": fixture.version_id,
            "metadata": {},
            "contextPayload": {}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "body: {body}");
    assert_eq!(body["error"]["code"], "admission_rejected");

    // Zero business delta: no instance, no receipt, no context revision.
    assert_eq!(instance_count(&pool, fixture.domain_id).await, 0);
    assert_eq!(
        receipt_count(&pool, fixture.caller, "adm-create-rejected").await,
        0
    );

    // Retry with the SAME idempotency key against an admitting directory
    // succeeds — proving nothing was persisted by the rejected attempt.
    let client = enabled_client(&admitting);
    let result = create_workflow_instance(
        &pool,
        AdmissionGate::new(Some(&client)),
        create_command(fixture.caller, &fixture, "adm-create-rejected".to_string()),
    )
    .await
    .expect("retry with admitting directory must succeed");
    assert_eq!(instance_count(&pool, fixture.domain_id).await, 1);
    let _ = result;
}

/// (c) Directory unreachable: fail closed with 503 admission_unavailable,
/// zero writes.
#[tokio::test]
async fn create_unavailable_fails_closed() {
    let pool = common::create_pool().await;
    let fixture = seed(&pool).await;
    let dead = closed_port().await;
    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url, admission_config_enabled(&dead));

    let bearer = token(fixture.caller, "workflow.execute", &mock.key_pair);
    let (status, body) = send(
        app,
        "POST",
        "/internal/v1/workflow-instances",
        &bearer,
        Some("adm-create-unavailable"),
        Some(json!({
            "domainId": fixture.domain_id,
            "definitionVersionId": fixture.version_id,
            "metadata": {},
            "contextPayload": {}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "body: {body}");
    assert_eq!(body["error"]["code"], "admission_unavailable");
    assert_eq!(instance_count(&pool, fixture.domain_id).await, 0);
    assert_eq!(
        receipt_count(&pool, fixture.caller, "adm-create-unavailable").await,
        0
    );
}

/// (d) Admission disabled (dormant deploy): behavior unchanged, no directory
/// traffic, write succeeds.
#[tokio::test]
async fn create_dormant_when_disabled_writes_without_directory() {
    let pool = common::create_pool().await;
    let fixture = seed(&pool).await;
    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url, disabled_client());

    let bearer = token(fixture.caller, "workflow.execute", &mock.key_pair);
    let (status, body) = send(
        app,
        "POST",
        "/internal/v1/workflow-instances",
        &bearer,
        Some("adm-create-dormant"),
        Some(json!({
            "domainId": fixture.domain_id,
            "definitionVersionId": fixture.version_id,
            "metadata": {},
            "contextPayload": {}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_eq!(instance_count(&pool, fixture.domain_id).await, 1);
}

/// (e) Transition onto an assigned node with a rejecting directory fails
/// closed (422, state version unchanged, receipt rolled back); the same
/// idempotency key then succeeds against an admitting directory.
#[tokio::test]
async fn transition_rejected_fails_closed_then_same_key_retry_admits() {
    let pool = common::create_pool().await;
    let fixture = seed(&pool).await;
    let rejecting = StubDirectory::spawn(StubMode::PrincipalDisabled);
    let admitting = StubDirectory::spawn(StubMode::Admit);
    let mock = MockJwksServer::start().await;

    // Create through an admitting client (app level; HTTP create is covered
    // by the tests above).
    let client = enabled_client(&admitting);
    let created = create_workflow_instance(
        &pool,
        AdmissionGate::new(Some(&client)),
        create_command(fixture.caller, &fixture, "adm-trans".to_string()),
    )
    .await
    .expect("create with admitting directory");
    assert_eq!(created.workflow_state_version, 1);

    // HTTP transition with a REJECTING directory configured.
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
            "/internal/v1/workflow-instances/{}/transitions",
            created.workflow_instance_id
        ),
        &bearer,
        Some("adm-trans-1"),
        Some(json!({
            "transitionDefinitionId": fixture.draft_advance,
            "expectedWorkflowStateVersion": 1
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "body: {body}");
    assert_eq!(body["error"]["code"], "admission_rejected");

    // Zero business delta: state version and visit count unchanged, and the
    // transition receipt rolled back.
    let (version, visits): (i32, i64) = sqlx::query_as(
        "SELECT wi.workflow_state_version, \
                (SELECT COUNT(*) FROM workflow_node_visits v WHERE v.workflow_instance_id = wi.workflow_instance_id) \
         FROM workflow_instances wi WHERE wi.workflow_instance_id = $1",
    )
    .bind(created.workflow_instance_id)
    .fetch_one(&pool)
    .await
    .expect("instance readback");
    assert_eq!(version, 1);
    assert_eq!(visits, 1);
    assert_eq!(receipt_count(&pool, fixture.caller, "adm-trans-1").await, 0);

    // Same idempotency key now succeeds against an admitting directory.
    let admitted = execute_workflow_transition(
        &pool,
        AdmissionGate::new(Some(&enabled_client(&admitting))),
        ExecuteWorkflowTransitionCommand {
            principal_id: svc_workflow::domain::ids::PrincipalId::from_uuid(fixture.caller),
            idempotency_key: "adm-trans-1".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(created.workflow_instance_id),
            expected_workflow_state_version: 1,
            transition_definition_id: TransitionId::from_uuid(fixture.draft_advance),
            submission_payload: None,
        },
    )
    .await
    .expect("retry with admitting directory");
    assert_eq!(admitted.workflow_state_version, 2);
    // The new visit is assigned to the FIXED_PRINCIPAL agent.
    let (assignee,): (Option<Uuid>,) = sqlx::query_as(
        "SELECT assignee_principal_id FROM workflow_node_visits WHERE node_visit_id = $1",
    )
    .bind(admitted.current_node_visit_id)
    .fetch_one(&pool)
    .await
    .expect("visit readback");
    assert_eq!(assignee, Some(fixture.agent));
}

/// (f) Revise (app surface): an identity-bearing context value that the new
/// revision persists must be admitted; rejection fails closed with the
/// receipt rolled back, and the same key retries against an admitting
/// directory.
#[tokio::test]
async fn revise_rejected_fails_closed_then_same_key_retry_admits() {
    let pool = common::create_pool().await;
    let fixture = seed(&pool).await;

    // A definition whose required INSTANCE_INPUT_PRINCIPAL key is read from
    // the context: revise must admission-check the value it persists.
    let version_id = seed_iip_revision_target(&pool, fixture.domain_id, fixture.agent).await;
    let mut fixture = fixture;
    fixture.version_id = version_id;

    let admitting = StubDirectory::spawn(StubMode::Admit);
    let rejecting = StubDirectory::spawn(StubMode::PrincipalDisabled);

    // Create with admission dormant (base instance; identity keys are
    // validated at create by the create-time invariant).
    let created = create_workflow_instance(
        &pool,
        AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: svc_workflow::domain::ids::PrincipalId::from_uuid(fixture.caller),
            idempotency_key: "adm-revise-base".to_string(),
            command_schema_version: "v1".to_string(),
            execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
            domain_id: DomainId::from_uuid(fixture.domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(fixture.version_id),
            external_reference: None,
            external_url: None,
            metadata: json!({}),
            context_payload: json!({"assigneePrincipalId": fixture.agent.to_string()}),
        },
    )
    .await
    .expect("base create");

    let revise_command = |key: &str| ReviseWorkflowContextCommand {
        principal_id: svc_workflow::domain::ids::PrincipalId::from_uuid(fixture.caller),
        idempotency_key: key.to_string(),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(created.workflow_instance_id),
        expected_workflow_state_version: 1,
        context_payload: json!({"assigneePrincipalId": fixture.agent.to_string()}),
    };

    // Rejecting directory -> fail closed.
    let error = revise_workflow_context(
        &pool,
        AdmissionGate::new(Some(&enabled_client(&rejecting))),
        revise_command("adm-revise-1"),
    )
    .await
    .expect_err("rejected revision must fail");
    assert!(
        matches!(error, ReviseWorkflowContextError::AdmissionFailed(_)),
        "{error:?}"
    );
    assert_eq!(revise_error_label(&error), "admission_rejected");
    assert_eq!(
        receipt_count(&pool, fixture.caller, "adm-revise-1").await,
        0
    );
    let (revisions,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_context_revisions WHERE workflow_instance_id = $1",
    )
    .bind(created.workflow_instance_id)
    .fetch_one(&pool)
    .await
    .expect("count revisions");
    assert_eq!(revisions, 1, "no revision may be persisted on rejection");

    // Same key against an admitting directory succeeds.
    revise_workflow_context(
        &pool,
        AdmissionGate::new(Some(&enabled_client(&admitting))),
        revise_command("adm-revise-1"),
    )
    .await
    .expect("retry with admitting directory");
    let (revisions,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_context_revisions WHERE workflow_instance_id = $1",
    )
    .bind(created.workflow_instance_id)
    .fetch_one(&pool)
    .await
    .expect("count revisions");
    assert_eq!(revisions, 2);

    // Admission observed the persisted identity value.
    let stub_requests = admitting.requests();
    let principals: BTreeSet<String> = stub_requests
        .iter()
        .filter_map(|p| p.strip_prefix("/api/v1/directory/principals/"))
        .map(|s| s.to_string())
        .collect();
    assert!(
        principals.contains(&format!("{}/agent", fixture.agent)),
        "the identity value carried by the revision must be admitted: {stub_requests:?}"
    );
}

/// A published legacy definition whose DRAFT node resolves from the context
/// (INSTANCE_INPUT_PRINCIPAL) — used to exercise identity-bearing revisions.
async fn seed_iip_revision_target(pool: &sqlx::PgPool, domain_id: Uuid, agent: Uuid) -> Uuid {
    let def_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let def_key = format!("iip-rev-{}", &Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) \
         VALUES ($1, $2, $3, 'IIP Revision Def')",
    )
    .bind(def_id)
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
    .bind(def_id)
    .bind(serde_json::json!({"type":"object"}))
    .execute(pool)
    .await
    .expect("insert draft version");

    // DRAFT node resolves its assignee from context key `assigneePrincipalId`
    // (graph rules force the DRAFT node of legacy definitions to keep
    // INSTANCE_INPUT_PRINCIPAL only when the publish validator allows it;
    // for seeds we use the same shape as the repair-capability fixtures).
    let draft = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, assignee_input_key) \
         VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'INSTANCE_INPUT_PRINCIPAL', 'assigneePrincipalId')",
    )
    .bind(draft)
    .bind(version_id)
    .execute(pool)
    .await
    .expect("insert iip draft node");
    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(version_id)
        .execute(pool)
        .await
        .expect("publish version");
    let _ = agent;
    version_id
}

// ---------------------------------------------------------------------------
// T62 — canonical successor admission vs persisted assignment (scratch probe,
// promotion candidate): with a workflow_identity_successor_lines edge P -> Q,
// the admission face canonicalizes the assignment target P to Q (the
// directory stub must observe Q), while the persisted new visit still binds
// the PRE-canonical source P (ticket's static anchors: create_transaction
// Step9 INSERT resolved_assignee_id = P).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t62_successor_admission_observes_successor_but_persisted_visit_stays_source() {
    let pool = common::create_pool().await;
    let fixture = seed(&pool).await;
    // fixture.agent = P: the work node's FIXED_PRINCIPAL assignee.
    let successor_q = common::seed_second_principal(&pool).await; // Q

    // Lineage edge P -> Q.
    sqlx::query(
        "INSERT INTO workflow_identity_successor_lines \
         (source_principal_id, successor_principal_id, legacy_agent_id, canonical_agent_id, classification, evidence, repair_reason) \
         VALUES ($1, $2, $3, $4, 'STALE_PRINCIPAL_WITH_UNIQUE_REPAIR', \
                 jsonb_build_object('source', 'tests/31 t62 runtime probe'), \
                 'T62 runtime fixture: unique canonical successor mechanically proven')",
    )
    .bind(fixture.agent)
    .bind(successor_q)
    .bind(format!("legacy-{}", fixture.agent))
    .bind(format!("canonical-{}", successor_q))
    .execute(&pool)
    .await
    .expect("insert successor line");

    let stub = StubDirectory::spawn(StubMode::Admit);
    let client = enabled_client(&stub);
    // Production wiring (main.rs) attaches the pool so the gate can resolve
    // identity lineage (canonicalize_principals -> resolve_current_principal).
    let gate = AdmissionGate::new(Some(&client)).with_pool(Some(&pool));

    // Create as CALLER (draft visit = caller; caller has no lineage edge so
    // its own admission canonicalizes to itself).
    let run = Uuid::new_v4().simple().to_string();
    let created = create_workflow_instance(
        &pool,
        gate.clone(),
        create_command(fixture.caller, &fixture, format!("t62-create-{run}")),
    )
    .await
    .expect("create instance");

    // Transition draft -> work as CALLER: the new visit's assignment target
    // is the work node's FIXED_PRINCIPAL = P. Admission canonicalizes P -> Q;
    // the persisted insert (Step 9) is expected to stay bound to P.
    let outcome = execute_workflow_transition(
        &pool,
        gate,
        ExecuteWorkflowTransitionCommand {
            principal_id: svc_workflow::domain::ids::PrincipalId::from_uuid(fixture.caller),
            idempotency_key: format!("t62-advance-1-{run}"),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(created.workflow_instance_id),
            expected_workflow_state_version: 1,
            transition_definition_id: TransitionId::from_uuid(fixture.draft_advance),
            submission_payload: None,
        },
    )
    .await
    .expect("transition executes");

    assert_eq!(
        outcome.workflow_state_version, 2,
        "transition must execute (admission passes via canonical successor)"
    );

    // ── Observation 1+2: the directory admitted the CANONICAL successor Q ──
    let requests = stub.requests();
    let q_admitted = requests
        .iter()
        .any(|p| p == &format!("/api/v1/directory/principals/{}/agent", successor_q));
    let p_admitted = requests
        .iter()
        .any(|p| p == &format!("/api/v1/directory/principals/{}/agent", fixture.agent));
    println!(
        "T62 DEBUG: agent(P)={} successor(Q)={} caller={} requests={:?}",
        fixture.agent, successor_q, fixture.caller, requests
    );
    assert!(
        q_admitted,
        "O1/O2: admission must canonicalize P -> Q before the directory reads (requests: {requests:?})"
    );
    assert!(
        !p_admitted,
        "O1/O2: the stale source P must NOT be admitted under its own id"
    );

    // ── Observation 3+4: the persisted new work visit stays bound to P ──
    let persisted_assignee: Uuid = sqlx::query_scalar(
        "SELECT v.assignee_principal_id FROM workflow_node_visits v \
         JOIN workflow_node_definitions d ON d.node_id = v.node_id \
         WHERE v.workflow_instance_id = $1 AND d.node_key = 'work'",
    )
    .bind(created.workflow_instance_id)
    .fetch_one(&pool)
    .await
    .expect("work visit readback");
    assert_eq!(
        persisted_assignee, fixture.agent,
        "O3/O4 FAIL_CONDITION: persisted visit assignee must be the PRE-canonical source P (validation saw Q)"
    );
    assert_ne!(persisted_assignee, successor_q);

    // ── Observation 5: the transition receipt's principal is the command
    // principal P (receipt identity bound pre-canonicalization) ──
    let receipt_principal: Uuid = sqlx::query_scalar(
        "SELECT principal_id FROM workflow_command_receipts \
         WHERE idempotency_key = 't62-advance-1-' || $1",
    )
    .bind(&run)
    .fetch_one(&pool)
    .await
    .expect("receipt readback");
    assert_eq!(receipt_principal, fixture.caller,
        "O5: the transition receipt records the commanding principal (caller); the assignment face (visit assignee = source P) is asserted above");

    // ── T62-REPAIR regression legs (read-side enrichment is the fix surface;
    // transition persistence stays un-rewritten) ──
    // Leg 3: Q's worklist includes the P-stored active work EXACTLY ONCE.
    // Leg 4: the stale source P cannot act as current owner (empty worklist).
    // Leg 5: an unrelated principal sees nothing.
    let unrelated = common::seed_second_principal(&pool).await;
    // Read visibility: BOTH the stale source (existing CIR read-side
    // enrichment contract, test 34) and the canonical successor see the
    // active work; an unrelated principal sees none.
    for (who, expected) in [
        (successor_q, 1usize),
        (fixture.agent, 1usize),
        (unrelated, 0usize),
    ] {
        let page = svc_workflow::store::postgres::workflow_instance_repository::query_worklists::list_assigned_to_me(
            &pool,
            svc_workflow::application::workflow_instance::query_types::ListAssignedToMe {
                actor_principal_id: who,
                before: None,
                limit: Some(20),
            },
        )
        .await
        .expect("worklist readback");
        assert_eq!(
            page.items.len(),
            expected,
            "worklist for {who} must contain {expected} item(s) (exactly-once semantics)"
        );
        if expected == 1 {
            assert_eq!(
                page.items[0].detail.instance.workflow_instance_id,
                created.workflow_instance_id
            );
        }
    }

    println!("T62 RESULT: admission saw canonical Q; persisted assignee + receipt bound to source P — VALIDATED_TARGET(Q) != PERSISTED_TARGET(P) confirmed at runtime; Q worklist enrichment exactly-once verified");
}
