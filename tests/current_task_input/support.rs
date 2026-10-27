use super::*;
use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};
fn build_app(pool: sqlx::PgPool, jwks_url: &str, admin_ids: Vec<Uuid>) -> axum::Router {
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
        provisioning_config: ProvisioningConfig::new(
            admin_ids
                .into_iter()
                .map(svc_workflow::domain::ids::PrincipalId::from_uuid)
                .collect(),
        ),
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

/// Use an actual HTTP server for JWKS, so request bytes are consumed before
/// connection closure. The shared raw TCP fixture can reset an unread request.
struct LocalJwks {
    url: String,
    key_pair: RsaTestKeyPair,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for LocalJwks {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl LocalJwks {
    async fn start() -> Self {
        let key_pair = generate_rsa_key_pair();
        let body = json!({"keys":[{"kty":"RSA","use":"sig","alg":"RS256","kid":key_pair.kid,"n":key_pair.n_base64url,"e":key_pair.e_base64url}]});
        let app = axum::Router::new().route(
            "/.well-known/jwks.json",
            axum::routing::get(move || {
                let body = body.clone();
                async move { axum::Json(body) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}/.well-known/jwks.json",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            url,
            key_pair,
            server,
        }
    }
}

pub struct Harness {
    pub pool: PgPool,
    pub seed: QueryFixture,
    pub instance: Uuid,
    pub root: String,
    pub client: reqwest::Client,
    jwks: LocalJwks,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Harness {
    pub async fn new(payload: Value) -> Self {
        let url = common::test_database_url();
        assert!(
            url.contains("127.0.0.1:55432/svc_workflow_test_current_input_20261027_"),
            "isolated CTI test database required"
        );
        let pool = create_pool().await;
        let seed = seed_query_fixture(&pool).await;
        let mut command = make_command(seed.creator, seed.domain, seed.version);
        command.context_payload = payload;
        let instance = create_workflow_instance(&pool, AdmissionGate::disabled(), command)
            .await
            .unwrap()
            .workflow_instance_id;
        let jwks = LocalJwks::start().await;
        let app = build_app(pool.clone(), &jwks.url, vec![seed.owner]);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let root = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            pool,
            seed,
            instance,
            root,
            client: reqwest::Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap(),
            jwks,
            server,
        }
    }
    pub async fn get(&self, actor: Uuid, query: &str) -> (u16, Value, usize) {
        let tok = v1_token(
            actor,
            "workflow.read",
            "test-client",
            300,
            &self.jwks.key_pair,
        );
        let r = self
            .client
            .get(format!(
                "{}/internal/v1/workflow-instances/{}/current-task-input{}",
                self.root,
                self.instance,
                if query.is_empty() {
                    String::new()
                } else {
                    format!("?{query}")
                }
            ))
            .bearer_auth(tok)
            .send()
            .await
            .unwrap();
        let status = r.status().as_u16();
        let bytes = r.bytes().await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        record_http_response(
            &format!(
                "/internal/v1/workflow-instances/{}/current-task-input?{}",
                self.instance, query
            ),
            actor,
            status,
            &bytes,
        );
        (status, body, bytes.len())
    }
    pub fn token(&self, actor: Uuid) -> String {
        v1_token(
            actor,
            "workflow.read",
            "test-client",
            300,
            &self.jwks.key_pair,
        )
    }
    pub async fn input(&self) -> Value {
        let (status, body, size) = self.get(self.seed.owner, "").await;
        assert_eq!(status, 200, "{body}");
        assert!(size <= 65536);
        body
    }
    pub async fn step(&self,actor:Uuid,version:i32,transition:Uuid,payload:Option<Value>)->svc_workflow::application::workflow_instance::execute_transition::ExecuteWorkflowTransitionResult{
        execute_workflow_transition_with_policy(
            &self.pool,
            AdmissionGate::disabled(),
            make_transition_command(actor, self.instance, version, transition, payload),
            100,
        )
        .await
        .unwrap()
    }
    pub async fn bytes(&self, input: &Value, descriptor: &Value) -> Vec<u8> {
        let mut bytes = descriptor["text"].as_str().unwrap().as_bytes().to_vec();
        let mut next = descriptor["next_offset"].as_u64();
        while let Some(offset) = next {
            let query = format!(
                "view=body&{}&bodyKind={}&bodyId={}&expectedBodySha256={}&offset={offset}",
                binding_query(input),
                descriptor["kind"].as_str().unwrap(),
                descriptor["id"].as_str().unwrap(),
                descriptor["sha256"].as_str().unwrap()
            );
            let (status, body, size) = self.get(self.seed.owner, &query).await;
            assert_eq!(status, 200, "{body}");
            assert!(size <= 65536);
            let chunk = &body["body"];
            assert_eq!(chunk["offset"], offset);
            let text = chunk["text"].as_str().unwrap();
            assert!(text.len() <= 8192);
            bytes.extend_from_slice(text.as_bytes());
            next = chunk["next_offset"].as_u64();
            if let Some(n) = next {
                assert!(n > offset);
            }
        }
        use sha2::{Digest, Sha256};
        assert_eq!(
            hex::encode(Sha256::digest(&bytes)),
            descriptor["sha256"].as_str().unwrap()
        );
        assert_eq!(
            bytes.len() as u64,
            descriptor["total_bytes"].as_u64().unwrap()
        );
        bytes
    }
}
pub fn binding_query(input: &Value) -> String {
    let b = &input["binding"];
    format!(
        "expectedNodeVisitId={}&expectedWorkflowStateVersion={}&expectedContextRevisionId={}",
        b["node_visit_id"].as_str().unwrap(),
        b["workflow_state_version"],
        b["context_revision_id"].as_str().unwrap()
    )
}
pub fn make_command(actor: Uuid, domain: Uuid, version: Uuid) -> CreateWorkflowInstanceCommand {
    CreateWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(actor),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".into(),
        execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
        domain_id: DomainId::from_uuid(domain),
        definition_version_id: DefinitionVersionId::from_uuid(version),
        external_reference: None,
        external_url: None,
        metadata: json!({}),
        context_payload: json!({}),
    }
}
pub fn make_transition_command(
    actor: Uuid,
    instance: Uuid,
    version: i32,
    transition: Uuid,
    payload: Option<Value>,
) -> ExecuteWorkflowTransitionCommand {
    ExecuteWorkflowTransitionCommand {
        principal_id: PrincipalId::from_uuid(actor),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".into(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(instance),
        expected_workflow_state_version: version,
        transition_definition_id: TransitionId::from_uuid(transition),
        submission_payload: payload,
    }
}

/// Counts all durable business rows in this test-only database. Tests run serially.
pub async fn business_counts(pool: &PgPool) -> Vec<(String, i64)> {
    let mut counts = Vec::new();
    for table in [
        "workflow_instances",
        "workflow_context_revisions",
        "workflow_node_visits",
        "workflow_submissions",
        "workflow_events",
        "workflow_command_receipts",
        "workflow_command_attempt_audits",
        "workflow_assistance_cases",
        "workflow_activations",
        "workflow_activation_closures",
        "workflow_dispatch_eligibility_events",
        "workflow_outbox",
    ] {
        let count = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(pool)
            .await
            .unwrap();
        counts.push((table.into(), count));
    }
    counts
}
pub async fn seed_v1_definition(
    pool: &PgPool,
    domain_id: Uuid,
    agent_id: Uuid,
    large_detail: bool,
) -> (Uuid, Uuid, Uuid, Uuid, Uuid, Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("v1-test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'V1 Test Def')",
    )
    .bind(def_id).bind(domain_id).bind(&def_key)
    .execute(pool).await.expect("insert def");

    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, semantic_model_version) VALUES ($1, $2, 1, 'DRAFT', 3)",
    )
    .bind(ver_id).bind(def_id)
    .execute(pool).await.expect("insert version");

    let start_id = Uuid::new_v4();
    let work_id = Uuid::new_v4();
    let done_id = Uuid::new_v4();
    let failed_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'start', 'Start', 0, 'TASK', 'WORKFLOW_CREATOR')",
    ).bind(start_id).bind(ver_id).execute(pool).await.expect("insert start");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) VALUES ($1, $2, 'work', 'Work', 1, 'TASK', 'FIXED_PRINCIPAL', $3)",
    ).bind(work_id).bind(ver_id).bind(agent_id).execute(pool).await.expect("insert work");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)",
    ).bind(done_id).bind(ver_id).execute(pool).await.expect("insert done");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'failed', 'Failed', 3, 'TERMINAL', NULL)",
    ).bind(failed_id).bind(ver_id).execute(pool).await.expect("insert failed");

    let t1 = Uuid::new_v4();
    let t2 = Uuid::new_v4();
    let t3 = Uuid::new_v4();
    let t4 = Uuid::new_v4();
    for (tid, key, src, dst, effect) in [
        (t1, "advance-1", start_id, work_id, "ADVANCE"),
        (t2, "advance-2", work_id, done_id, "ADVANCE"),
        (t3, "return", work_id, start_id, "RETURN"),
        (t4, "terminate", work_id, failed_id, "TERMINATE"),
    ] {
        sqlx::query(
            "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, $3, $3, $4, $5, $6::transition_effect)",
        )
        .bind(tid).bind(ver_id).bind(key).bind(src).bind(dst).bind(effect)
        .execute(pool).await.expect("insert transition");
    }
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t1).bind(start_id).execute(pool).await.expect("primary start");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t2).bind(work_id).execute(pool).await.expect("primary work");

    if large_detail {
        sqlx::query("UPDATE workflow_node_definitions SET instructions=$1 WHERE node_id=$2")
            .bind("instructions 😀 ".repeat(500))
            .bind(start_id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("UPDATE workflow_transition_definitions SET submission_schema=$1 WHERE transition_id=$2").bind(json!({"type":"object","description":"schema 😀 ".repeat(3000)})).bind(t1).execute(pool).await.unwrap();
    }
    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(pool).await.expect("publish");

    (ver_id, start_id, work_id, done_id, failed_id, t1, t2, t4)
}

/// Optional test artifact sink; no credentials are recorded. Raw response bytes
/// are retained alongside the parsed copy so wire bounds/digests are reviewable.
pub fn record_http_response(path: &str, actor: Uuid, status: u16, bytes: &[u8]) {
    if let Ok(dir) = std::env::var("CTI_HTTP_EVIDENCE_DIR") {
        use sha2::{Digest, Sha256};
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        let id = Uuid::new_v4();
        let raw_name = format!("{id}.response.raw");
        std::fs::write(dir.join(&raw_name), bytes).unwrap();
        let record = json!({"method":"GET","path":path,"actor":actor,"status":status,"wire_bytes":bytes.len(),"wire_sha256":hex::encode(Sha256::digest(bytes)),"raw_response":raw_name,"response":serde_json::from_slice::<Value>(bytes).unwrap_or(Value::Null)});
        std::fs::write(
            dir.join(format!("{id}.json")),
            serde_json::to_vec_pretty(&record).unwrap(),
        )
        .unwrap();
    }
}
