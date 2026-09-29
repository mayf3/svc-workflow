//! Authoring loop end-to-end: natural authoring input -> validate -> create
//! Definition -> draft version -> draft graph -> publish -> create Workflow
//! Instance from the published version.
//!
//! Proves the smallest useful authoring loop over the existing HTTP surface
//! (no new API): a valid definition authors/publishes/instantiates, invalid
//! authoring input returns concrete actionable errors without partial state,
//! published version identity (digest) is stable, and an instance pins the
//! exact published version it was created from.
//!
//! Requires a running PostgreSQL 16 (see tests/common/mod.rs). Real TCP,
//! Auth V1 direct machine tokens, disposable DB.
#[path = "common/mod.rs"]
mod common;

use serde_json::{Value, json};
use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};
use uuid::Uuid;

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

struct Harness {
    root: String,
    client: reqwest::Client,
    token: String,
    admin_token: String,
    domain: Uuid,
    _jwks: common::MockJwksServer,
    _server: tokio::task::JoinHandle<()>,
}

/// One domain, one AGENT principal owning it (DOMAIN_OWNER + provisioning
/// allow-list), a real TCP server, and direct + admin tokens for that owner.
async fn harness(admin_ids: Vec<Uuid>) -> Harness {
    let pool = common::create_pool().await;
    let owner = common::seed_second_principal(&pool).await;
    let (_, domain) = common::seed_principal_and_domain(&pool).await;
    common::seed_domain_owner(&pool, domain, owner).await;
    let jwks = common::MockJwksServer::start().await;
    let app = build_app(
        pool,
        &jwks.url,
        admin_ids.into_iter().chain([owner]).collect(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let token = common::v1_token(
        owner,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &jwks.key_pair,
    );
    let admin_token = common::v1_token(
        owner,
        "workflow.admin workflow.execute workflow.read",
        "test-client",
        300,
        &jwks.key_pair,
    );
    Harness {
        root,
        client,
        token,
        admin_token,
        domain,
        _jwks: jwks,
        _server: server,
    }
}

impl Harness {
    async fn request(&self, method: &str, path: &str, body: Value) -> (u16, Value) {
        self.request_with_token(method, path, body, &self.token, &Uuid::new_v4().to_string())
            .await
    }

    async fn request_with_token(
        &self,
        method: &str,
        path: &str,
        body: Value,
        token: &str,
        idempotency_key: &str,
    ) -> (u16, Value) {
        let response = self
            .client
            .request(method.parse().unwrap(), format!("{}{path}", self.root))
            .bearer_auth(token)
            .header("idempotency-key", idempotency_key)
            .json(&body)
            .send()
            .await
            .unwrap();
        (response.status().as_u16(), response.json().await.unwrap())
    }

    /// Author a fresh definition with a DRAFT version and the minimal linear
    /// graph: draft (WORKFLOW_CREATOR) -> work (TASK, WORKFLOW_CREATOR) ->
    /// done (TERMINAL). Returns (definitionId, versionId).
    async fn author_draft(&self, display_suffix: &str) -> (Uuid, Uuid) {
        let path = format!("/internal/v1/domains/{}/definitions", self.domain);
        let (status, def) = self
            .request(
                "POST",
                &path,
                json!({
                    "definitionKey": format!("authoring-loop-{}", Uuid::new_v4()),
                    "displayName": format!("Authoring loop {display_suffix}"),
                    "description": "smallest useful authoring loop"
                }),
            )
            .await;
        assert_eq!(status, 200, "{def}");
        let definition_id = Uuid::parse_str(def["workflowDefinitionId"].as_str().unwrap()).unwrap();
        let base = format!("{path}/{definition_id}");

        let (status, version) = self
            .request(
                "POST",
                &format!("{base}/versions"),
                json!({"contextSchema": {"type": "object"}}),
            )
            .await;
        assert_eq!(status, 200, "{version}");
        assert_eq!(version["versionStatus"], "DRAFT", "{version}");
        let version_id = Uuid::parse_str(version["definitionVersionId"].as_str().unwrap()).unwrap();

        let (status, replaced) = self
            .request(
                "PUT",
                &format!("{base}/draft"),
                json!({
                    "definitionVersionId": version_id,
                    "contextSchema": {"type": "object"},
                    "nodes": [
                        {"node_key": "draft", "display_name": "Draft", "order_index": 0,
                         "node_type": "DRAFT", "assignee_ref_type": "WORKFLOW_CREATOR",
                         "primary_advance_transition_key": "advance-work"},
                        {"node_key": "work", "display_name": "Work", "order_index": 1,
                         "node_type": "NORMAL", "assignee_ref_type": "WORKFLOW_CREATOR",
                         "primary_advance_transition_key": "finish"},
                        {"node_key": "done", "display_name": "Done", "order_index": 2,
                         "node_type": "TERMINAL"}
                    ],
                    "transitions": [
                        {"transition_key": "advance-work", "display_name": "Start work",
                         "source_node_key": "draft", "target_node_key": "work",
                         "transition_effect": "ADVANCE"},
                        {"transition_key": "finish", "display_name": "Finish",
                         "source_node_key": "work", "target_node_key": "done",
                         "transition_effect": "ADVANCE"}
                    ]
                }),
            )
            .await;
        assert_eq!(status, 200, "{replaced}");
        (definition_id, version_id)
    }

    async fn publish(&self, definition_id: Uuid, version_id: Uuid) -> (u16, Value) {
        let base = format!(
            "/internal/v1/domains/{}/definitions/{definition_id}",
            self.domain
        );
        self.request(
            "POST",
            &format!("{base}/publish"),
            json!({"versionId": version_id}),
        )
        .await
    }

    async fn create_instance(&self, version_id: Uuid) -> (u16, Value) {
        self.request(
            "POST",
            "/internal/v1/workflow-instances",
            json!({
                "domainId": self.domain,
                "definitionVersionId": version_id,
                "executionClass": "BUSINESS",
                "externalReference": "authoring-loop-e2e",
                "metadata": {},
                "contextPayload": {}
            }),
        )
        .await
    }

    async fn instance_detail(&self, instance_id: Uuid) -> Value {
        let (status, body) = self
            .request(
                "GET",
                &format!("/internal/v1/workflow-instances/{instance_id}"),
                json!({}),
            )
            .await;
        assert_eq!(status, 200, "{body}");
        body
    }
}

#[tokio::test]
async fn valid_definition_authors_publishes_and_instantiates() {
    let h = harness(vec![]).await;
    let (definition_id, version_id) = h.author_draft("valid").await;
    let (status, published) = h.publish(definition_id, version_id).await;
    assert_eq!(status, 200, "{published}");
    assert_eq!(published["versionStatus"], "PUBLISHED", "{published}");
    assert_eq!(published["versionNumber"], 1, "{published}");
    assert!(
        published["digest"].as_str().is_some_and(|d| !d.is_empty()),
        "{published}"
    );
    assert!(published["publishedAt"].is_string(), "{published}");

    let (status, instance) = h.create_instance(version_id).await;
    assert_eq!(status, 201, "{instance}");
    let instance_id = Uuid::parse_str(instance["workflowInstanceId"].as_str().unwrap()).unwrap();

    let detail = h.instance_detail(instance_id).await;
    assert_eq!(
        detail["detail"]["instance"]["definition_version_id"],
        json!(version_id),
        "{detail}"
    );
    assert_eq!(
        detail["detail"]["instance"]["definition_version_status"],
        json!("PUBLISHED"),
        "{detail}"
    );
    assert_eq!(
        detail["detail"]["instance"]["current_node"]["node_key"],
        json!("draft"),
        "{detail}"
    );
}

#[tokio::test]
async fn invalid_graph_returns_actionable_errors_without_partial_state() {
    let h = harness(vec![]).await;
    let pool = common::create_pool().await;
    let path = format!("/internal/v1/domains/{}/definitions", h.domain);
    let (status, def) = h
        .request(
            "POST",
            &path,
            json!({"definitionKey": format!("authoring-loop-{}", Uuid::new_v4()), "displayName": "Broken"}),
        )
        .await;
    assert_eq!(status, 200, "{def}");
    let definition_id = Uuid::parse_str(def["workflowDefinitionId"].as_str().unwrap()).unwrap();
    let base = format!("{path}/{definition_id}");
    let (status, version) = h
        .request("POST", &format!("{base}/versions"), json!({}))
        .await;
    assert_eq!(status, 200, "{version}");
    let version_id = Uuid::parse_str(version["definitionVersionId"].as_str().unwrap()).unwrap();

    // Transition targets a node key that does not exist in the graph.
    let (status, rejected) = h
        .request(
            "PUT",
            &format!("{base}/draft"),
            json!({
                "definitionVersionId": version_id,
                "nodes": [
                    {"node_key": "draft", "display_name": "Draft", "order_index": 0,
                     "node_type": "DRAFT", "assignee_ref_type": "WORKFLOW_CREATOR",
                     "primary_advance_transition_key": "advance-work"},
                    {"node_key": "done", "display_name": "Done", "order_index": 1,
                     "node_type": "TERMINAL"}
                ],
                "transitions": [
                    {"transition_key": "advance-work", "display_name": "Start work",
                     "source_node_key": "draft", "target_node_key": "work",
                     "transition_effect": "ADVANCE"}
                ]
            }),
        )
        .await;
    assert_eq!(status, 422, "{rejected}");
    assert_eq!(
        rejected["error"]["code"], "graph_validation_failed",
        "{rejected}"
    );
    let errors = rejected["error"]["details"]["errors"]
        .as_array()
        .unwrap_or_else(|| panic!("concrete error list expected: {rejected}"));
    assert!(
        !errors.is_empty(),
        "at least one actionable error expected: {rejected}"
    );
    // Diagnostics are a closed, sanitizer-safe projection (definition graph
    // diagnostics V1): each rule is a precise stable code plus a static
    // correction. The authoring input named a transition targeting a missing
    // node, so the exact rule must say which rule to fix.
    let rendered = serde_json::to_string(errors).unwrap();
    assert!(
        errors
            .iter()
            .any(|e| e["code"] == json!("TRANSITION_TARGET_MISSING")),
        "expected the exact violated rule TRANSITION_TARGET_MISSING: {rendered}"
    );

    // No partial persistence: the DRAFT version stays empty.
    let nodes: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM workflow_node_definitions WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(nodes.0, 0, "rejected graph must not persist nodes");

    // An unpublished (still DRAFT) version cannot be instantiated.
    let (status, refused) = h.create_instance(version_id).await;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(
        refused["error"]["code"], "version_not_published",
        "{refused}"
    );
}

#[tokio::test]
async fn published_version_identity_is_stable() {
    let h = harness(vec![]).await;
    let (definition_id, version_id) = h.author_draft("identity").await;

    let publish_key = Uuid::new_v4().to_string();
    let base = format!(
        "/internal/v1/domains/{}/definitions/{definition_id}",
        h.domain
    );
    let (status, first) = h
        .request_with_token(
            "POST",
            &format!("{base}/publish"),
            json!({"versionId": version_id}),
            &h.token,
            &publish_key,
        )
        .await;
    assert_eq!(status, 200, "{first}");
    let digest = first["digest"].as_str().unwrap().to_string();
    assert_eq!(first["definitionVersionId"], json!(version_id), "{first}");

    // Replay with the same idempotency key returns the same published identity.
    let (status, replay) = h
        .request_with_token(
            "POST",
            &format!("{base}/publish"),
            json!({"versionId": version_id}),
            &h.token,
            &publish_key,
        )
        .await;
    assert_eq!(status, 200, "{replay}");
    assert_eq!(replay["digest"], json!(digest), "{replay}");
    assert_eq!(replay["versionStatus"], "PUBLISHED", "{replay}");

    // Independent readbacks agree on the same stable identity.
    let (status, readback) = h
        .request_with_token(
            "GET",
            &format!("/internal/v1/admin/definition-versions/{version_id}"),
            json!({}),
            &h.admin_token,
            &Uuid::new_v4().to_string(),
        )
        .await;
    assert_eq!(status, 200, "{readback}");
    assert_eq!(readback["digest"], json!(digest), "{readback}");
    assert_eq!(readback["versionStatus"], "PUBLISHED", "{readback}");
    assert_eq!(readback["canCreateInstances"], json!(true), "{readback}");

    let pool = common::create_pool().await;
    let stored: (String,) = sqlx::query_as(
        "SELECT definition_digest FROM workflow_definition_versions WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        stored.0, digest,
        "DB digest must match the published identity"
    );
}

#[tokio::test]
async fn instance_pins_the_published_version_it_was_created_from() {
    let h = harness(vec![]).await;
    let path = format!("/internal/v1/domains/{}/definitions", h.domain);

    // v1: author + publish + instantiate.
    let (definition_id, v1) = h.author_draft("pin-v1").await;
    let (status, p1) = h.publish(definition_id, v1).await;
    assert_eq!(status, 200, "{p1}");
    let digest_v1 = p1["digest"].as_str().unwrap().to_string();
    let (status, i1) = h.create_instance(v1).await;
    assert_eq!(status, 201, "{i1}");
    let instance_1 = Uuid::parse_str(i1["workflowInstanceId"].as_str().unwrap()).unwrap();

    // v2: same definition, changed graph content, published afterwards.
    let base = format!("{path}/{definition_id}");
    let (status, v2res) = h
        .request(
            "POST",
            &format!("{base}/versions"),
            json!({"contextSchema": {"type": "object"}}),
        )
        .await;
    assert_eq!(status, 200, "{v2res}");
    let v2 = Uuid::parse_str(v2res["definitionVersionId"].as_str().unwrap()).unwrap();
    let (status, replaced) = h
        .request(
            "PUT",
            &format!("{base}/draft"),
            json!({
                "definitionVersionId": v2,
                "contextSchema": {"type": "object"},
                "nodes": [
                    {"node_key": "draft", "display_name": "Draft", "order_index": 0,
                     "node_type": "DRAFT", "assignee_ref_type": "WORKFLOW_CREATOR",
                     "primary_advance_transition_key": "finish"},
                    {"node_key": "done", "display_name": "Done v2", "order_index": 1,
                     "node_type": "TERMINAL"}
                ],
                "transitions": [
                    {"transition_key": "finish", "display_name": "Finish",
                     "source_node_key": "draft", "target_node_key": "done",
                     "transition_effect": "ADVANCE"}
                ]
            }),
        )
        .await;
    assert_eq!(status, 200, "{replaced}");
    let (status, p2) = h.publish(definition_id, v2).await;
    assert_eq!(status, 200, "{p2}");
    assert_eq!(p2["versionNumber"], 2, "{p2}");
    let digest_v2 = p2["digest"].as_str().unwrap().to_string();
    assert_ne!(
        digest_v1, digest_v2,
        "changed graph content must change published identity"
    );

    // The pre-existing instance still references v1; a new instance gets v2.
    let detail_1 = h.instance_detail(instance_1).await;
    assert_eq!(
        detail_1["detail"]["instance"]["definition_version_id"],
        json!(v1),
        "{detail_1}"
    );
    let (status, i2) = h.create_instance(v2).await;
    assert_eq!(status, 201, "{i2}");
    let instance_2 = Uuid::parse_str(i2["workflowInstanceId"].as_str().unwrap()).unwrap();
    let detail_2 = h.instance_detail(instance_2).await;
    assert_eq!(
        detail_2["detail"]["instance"]["definition_version_id"],
        json!(v2),
        "{detail_2}"
    );
}
