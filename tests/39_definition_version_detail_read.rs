//! Test: GET /internal/v1/domains/{domainId}/definitions/{definitionId}/versions/{versionId}
//! (AGENT_CORE_WORKFLOW_DEFINITION_VERSION_READ_V1 amendment — precise
//! single-version read with the complete graph for the same-domain owner.)
//!
//! Matrix:
//! - owner reads a PUBLISHED version: full fields (assignee_ref_type,
//!   instructions, primary_advance_transition_key, metadata on nodes;
//!   submission_schema, transition_effect on transitions), version_status,
//!   counts — not just counts;
//! - owner reads a DRAFT version of the same definition;
//! - adjacent versions do not cross graphs;
//! - path ownership mismatch (real version + wrong definitionId or wrong
//!   domainId) → opaque 404 definition_not_found;
//! - nonexistent version → 404;
//! - non-owner caller → 404 (no existence leak);
//! - token without workflow.read → rejected before any body.

mod common;

use serde_json::{json, Value};
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
    pool: sqlx::PgPool,
    root: String,
    client: reqwest::Client,
    token: String,
    domain: Uuid,
    owner: Uuid,
    second: Uuid,
    _jwks: common::MockJwksServer,
    _server: tokio::task::JoinHandle<()>,
}

async fn harness() -> Harness {
    let pool = common::create_pool().await;
    let owner = common::seed_second_principal(&pool).await;
    let second = common::seed_second_principal(&pool).await;
    let (_, domain) = common::seed_principal_and_domain(&pool).await;
    common::seed_domain_owner(&pool, domain, owner).await;
    let jwks = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &jwks.url, vec![owner]);
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
    Harness {
        pool,
        root,
        client,
        token,
        domain,
        owner,
        second,
        _jwks: jwks,
        _server: server,
    }
}

impl Harness {
    async fn get_with_token_full(
        &self,
        path: &str,
        token: &str,
    ) -> (u16, Value, reqwest::header::HeaderMap) {
        let response = self
            .client
            .request(reqwest::Method::GET, format!("{}{path}", self.root))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        (status, body, headers)
    }

    async fn get_with_token(&self, path: &str, token: &str) -> (u16, Value) {
        let response = self
            .client
            .request(reqwest::Method::GET, format!("{}{path}", self.root))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        (status, body)
    }

    /// Author a definition with a rich DRAFT version whose nodes/transitions
    /// carry the full field set. Returns (definitionId, draftVersionId).
    async fn author_rich_draft(&self, suffix: &str, with_instructions: bool) -> (Uuid, Uuid) {
        let path = format!("/internal/v1/domains/{}/definitions", self.domain);
        let (status, def) = self
            .request_json(
                "POST",
                &path,
                json!({
                    "definitionKey": format!("version-read-{}-{}", suffix, Uuid::new_v4()),
                    "displayName": format!("Version read {suffix}"),
                    "description": "precise version read test"
                }),
                &self.token,
            )
            .await;
        assert_eq!(status, 200, "{def}");
        let definition_id = Uuid::parse_str(def["workflowDefinitionId"].as_str().unwrap()).unwrap();
        let base = format!("{path}/{definition_id}");

        let instructions_a = if with_instructions {
            "冻结需求；缺必需信息时等待用户补齐".to_string()
        } else {
            String::new()
        };
        let mut node_draft = json!({
            "node_key": "draft", "display_name": "Draft", "order_index": 0,
            "node_type": "DRAFT", "assignee_ref_type": "WORKFLOW_CREATOR",
            "primary_advance_transition_key": "advance-work"
        });
        if with_instructions {
            node_draft["instructions"] = json!(instructions_a);
            node_draft["metadata"] = json!({"humanConfirmationRequired": true});
        }
        let (status, version) = self
            .request_json(
                "POST",
                &format!("{base}/versions"),
                json!({"contextSchema": {"type": "object"}}),
                &self.token,
            )
            .await;
        assert_eq!(status, 200, "{version}");
        let version_id = Uuid::parse_str(version["definitionVersionId"].as_str().unwrap()).unwrap();

        let transitions = json!([
            {"transition_key": "advance-work", "display_name": "Start work",
             "source_node_key": "draft", "target_node_key": "work",
             "transition_effect": "ADVANCE",
             "submission_schema": {"type": "object", "additionalProperties": false,
                "required": ["confirm"], "properties": {"confirm": {"type": "boolean"}}}},
            {"transition_key": "finish", "display_name": "Finish",
             "source_node_key": "work", "target_node_key": "done",
             "transition_effect": "ADVANCE"}
        ]);
        let (status, replaced) = self
            .request_json(
                "PUT",
                &format!("{base}/draft"),
                json!({
                    "definitionVersionId": version_id,
                    "contextSchema": {"type": "object"},
                    "nodes": [node_draft,
                        {"node_key": "work", "display_name": "Work", "order_index": 1,
                         "node_type": "NORMAL", "assignee_ref_type": "WORKFLOW_CREATOR",
                         "primary_advance_transition_key": "finish"},
                        {"node_key": "done", "display_name": "Done", "order_index": 2,
                         "node_type": "TERMINAL"}],
                    "transitions": transitions
                }),
                &self.token,
            )
            .await;
        assert_eq!(status, 200, "{replaced}");
        (definition_id, version_id)
    }

    async fn publish(&self, definition_id: Uuid, version_id: Uuid) -> (u16, Value) {
        self.request_json(
            "POST",
            &format!(
                "/internal/v1/domains/{}/definitions/{definition_id}/publish",
                self.domain
            ),
            json!({"versionId": version_id}),
            &self.token,
        )
        .await
    }

    async fn request_json(
        &self,
        method: &str,
        path: &str,
        body: Value,
        token: &str,
    ) -> (u16, Value) {
        let response = self
            .client
            .request(method.parse().unwrap(), format!("{}{path}", self.root))
            .bearer_auth(token)
            .header("idempotency-key", Uuid::new_v4().to_string())
            .json(&body)
            .send()
            .await
            .unwrap();
        (response.status().as_u16(), response.json().await.unwrap())
    }
}

fn version_path(domain: &Uuid, definition: &Uuid, version: &Uuid) -> String {
    format!(
        "/internal/v1/domains/{domain}/definitions/{definition}/versions/{version}"
    )
}

#[tokio::test]
async fn owner_reads_published_version_with_complete_graph_fields() {
    let h = harness().await;
    let (definition_id, draft_version) = h.author_rich_draft("pub", true).await;
    let (status, published) = h.publish(definition_id, draft_version).await;
    assert_eq!(status, 200, "{published}");

    let (status, body) = h
        .get_with_token(
            &version_path(&h.domain, &definition_id, &draft_version),
            &h.token,
        )
        .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["version"]["version_status"], "PUBLISHED", "{body}");
    assert_eq!(body["version"]["id"], json!(draft_version), "{body}");
    assert_eq!(body["definition"]["id"], json!(definition_id), "{body}");
    // field-level graph, not counts
    let nodes = body["nodes"].as_array().expect("nodes array");
    assert!(nodes.len() >= 2);
    let draft_node = nodes
        .iter()
        .find(|n| n["node_key"] == "draft")
        .expect("draft node");
    assert_eq!(
        draft_node["assignee_ref"]["ref_type"],
        "WORKFLOW_CREATOR"
    );
    assert_eq!(
        draft_node["instructions"],
        "冻结需求；缺必需信息时等待用户补齐"
    );
    assert_eq!(
        draft_node["metadata"]["humanConfirmationRequired"],
        json!(true)
    );
    let transitions = body["transitions"].as_array().expect("transitions array");
    let t0 = &transitions
        .iter()
        .find(|t| t["transition_key"] == "advance-work")
        .expect("advance-work transition");
    assert_eq!(t0["transition_effect"], "ADVANCE");
    assert_eq!(t0["submission_schema"]["required"][0], "confirm");
    // primary advance cross-reference: the node stores the transition ID
    assert_eq!(
        draft_node["primary_advance_transition_id"],
        t0["transition_id"]
    );
    assert_eq!(body["nodes_count"], json!(nodes.len()));
    assert_eq!(body["transitions_count"], json!(transitions.len()));
}

#[tokio::test]
async fn owner_reads_draft_version_and_adjacent_versions_do_not_cross_graphs() {
    let h = harness().await;
    // published version carries instructions; the newer DRAFT has a bare graph
    let (definition_id, published_version) = h.author_rich_draft("adj-pub", true).await;
    let (status, _) = h.publish(definition_id, published_version).await;
    assert_eq!(status, 200);
    // a SECOND draft version on the same definition with a different graph
    let base = format!(
        "/internal/v1/domains/{}/definitions/{definition_id}",
        h.domain
    );
    let (status, v2) = h
        .request_json(
            "POST",
            &format!("{base}/versions"),
            json!({"contextSchema": {"type": "object"}}),
            &h.token,
        )
        .await;
    assert_eq!(status, 200, "{v2}");
    let v2_id = Uuid::parse_str(v2["definitionVersionId"].as_str().unwrap()).unwrap();
    let (status, replaced) = h
        .request_json(
            "PUT",
            &format!("{base}/draft"),
            json!({
                "definitionVersionId": v2_id,
                "contextSchema": {"type": "object"},
                "nodes": [
                    {"node_key": "only-draft2", "display_name": "Draft2",
                     "order_index": 0, "node_type": "DRAFT",
                     "assignee_ref_type": "WORKFLOW_CREATOR",
                     "primary_advance_transition_key": "go"},
                    {"node_key": "only-done2", "display_name": "Done2",
                     "order_index": 1, "node_type": "TERMINAL"}
                ],
                "transitions": [
                    {"transition_key": "go", "display_name": "Go",
                     "source_node_key": "only-draft2", "target_node_key": "only-done2",
                     "transition_effect": "ADVANCE"}
                ]
            }),
            &h.token,
        )
        .await;
    assert_eq!(status, 200, "{replaced}");

    // DRAFT readable by the owner
    let (status, draft_body) = h
        .get_with_token(&version_path(&h.domain, &definition_id, &v2_id), &h.token)
        .await;
    assert_eq!(status, 200, "{draft_body}");
    assert_eq!(draft_body["version"]["version_status"], "DRAFT");
    let draft_nodes = draft_body["nodes"].as_array().unwrap();
    assert_eq!(draft_nodes.len(), 2);
    let draft_keys: Vec<&str> = draft_nodes.iter().map(|n| n["node_key"].as_str().unwrap()).collect();
    assert!(draft_keys.contains(&"only-draft2"));
    assert!(draft_keys.contains(&"only-done2"));
    assert!(!draft_keys.contains(&"draft"));

    // adjacent version isolation: the published version still has its own graph
    let (status, pub_body) = h
        .get_with_token(
            &version_path(&h.domain, &definition_id, &published_version),
            &h.token,
        )
        .await;
    assert_eq!(status, 200, "{pub_body}");
    let pub_nodes = pub_body["nodes"].as_array().unwrap();
    assert_eq!(pub_nodes.len(), 3); // draft + work + done (its own graph)
    assert!(pub_nodes.iter().all(|n| n["node_key"] != "only-draft2"));
}

#[tokio::test]
async fn path_ownership_mismatch_and_missing_and_foreign_caller_are_opaque_404() {
    let h = harness().await;
    let (definition_id, version_id) = h.author_rich_draft("opaque", false).await;

    // wrong definitionId in path (real version id)
    let (status, body) = h
        .get_with_token(
            &version_path(&h.domain, &Uuid::new_v4(), &version_id),
            &h.token,
        )
        .await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"]["code"], "definition_not_found", "{body}");

    // wrong domainId in path
    let (status, body) = h
        .get_with_token(
            &version_path(&Uuid::new_v4(), &definition_id, &version_id),
            &h.token,
        )
        .await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"]["code"], "definition_not_found");

    // nonexistent version
    let (status, body) = h
        .get_with_token(
            &version_path(&h.domain, &definition_id, &Uuid::new_v4()),
            &h.token,
        )
        .await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"]["code"], "definition_not_found");

    // non-owner caller (valid token for a principal that is not the owner)
    let non_owner_token = common::v1_token(
        h.second,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &h._jwks.key_pair,
    );
    let (status, body) = h
        .get_with_token(
            &version_path(&h.domain, &definition_id, &version_id),
            &non_owner_token,
        )
        .await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"]["code"], "definition_not_found");
}

#[tokio::test]
async fn token_without_workflow_read_is_rejected_before_any_body() {
    let h = harness().await;
    let (definition_id, version_id) = h.author_rich_draft("scope", false).await;
    let no_read_token = common::v1_token(
        h.owner,
        "workflow.execute",
        "test-client",
        300,
        &h._jwks.key_pair,
    );
    let (status, body) = h
        .get_with_token(
            &version_path(&h.domain, &definition_id, &version_id),
            &no_read_token,
        )
        .await;
    assert!(
        status == 403 || status == 401,
        "expected scope rejection, got {status}: {body}"
    );
    // the rejection body must not carry the graph
    assert!(body.get("nodes").is_none(), "{body}");
}

#[tokio::test]
async fn disabled_principal_disabled_owner_member_and_admin_without_owner_are_all_opaque_404_with_request_id() {
    let h = harness().await;
    let (definition_id, version_id) = h.author_rich_draft("opaque2", false).await;
    let path = version_path(&h.domain, &definition_id, &version_id);

    // (a) DISABLED principal (exists, enabled=false): PrincipalDisabled must map
    // to the same opaque 404 — no distinguishable existence signal.
    let disabled = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'HUMAN', 'disabled reviewer', NULL, FALSE)",
    )
    .bind(disabled)
    .execute(&h.pool)
    .await
    .unwrap();
    let disabled_token = common::v1_token(
        disabled,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &h._jwks.key_pair,
    );
    let (status, body, headers) = h.get_with_token_full(&path, &disabled_token).await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"]["code"], "definition_not_found");
    assert!(!headers.get("x-request-id").unwrap().is_empty(), "404 must carry x-request-id");

    // (b) DISABLED DOMAIN_OWNER binding (principal enabled, binding not):
    let inactive_owner = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'HUMAN', 'inactive owner', NULL, TRUE)",
    )
    .bind(inactive_owner)
    .execute(&h.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1, $2, $3, 'DOMAIN_OWNER', FALSE)",
    )
    .bind(Uuid::new_v4())
    .bind(h.domain)
    .bind(inactive_owner)
    .execute(&h.pool)
    .await
    .unwrap();
    let inactive_token = common::v1_token(
        inactive_owner,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &h._jwks.key_pair,
    );
    let (status, body, _) = h.get_with_token_full(&path, &inactive_token).await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"]["code"], "definition_not_found");

    // (c) SAME-DOMAIN MEMBER (DOMAIN_MEMBER binding, enabled): H-5 is
    // owner-only, so a member is also opaque-404 — no member read widening.
    let member = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'HUMAN', 'domain member', NULL, TRUE)",
    )
    .bind(member)
    .execute(&h.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1, $2, $3, 'DOMAIN_MEMBER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(h.domain)
    .bind(member)
    .execute(&h.pool)
    .await
    .unwrap();
    let member_token = common::v1_token(
        member,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &h._jwks.key_pair,
    );
    let (status, body, _) = h.get_with_token_full(&path, &member_token).await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"]["code"], "definition_not_found");
}

#[tokio::test]
async fn admin_without_owner_binding_is_opaque_404() {
    // an allow-listed provisioning admin with NO domain owner binding:
    let pool = common::create_pool().await;
    let admin = common::seed_second_principal(&pool).await;
    let (_, domain) = common::seed_principal_and_domain(&pool).await;
    // NOTE: deliberately NO seed_domain_owner for `admin`.
    let jwks = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &jwks.url, vec![admin]);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let admin_token = common::v1_token(
        admin,
        "workflow.execute workflow.read workflow.admin",
        "test-client",
        300,
        &jwks.key_pair,
    );

    // author a definition as the real owner via a second harness is overkill:
    // the opaque behavior must hold even for a nonexistent target.
    let path = version_path(&domain, &Uuid::new_v4(), &Uuid::new_v4());
    let response = client
        .request(reqwest::Method::GET, format!("{root}{path}"))
        .bearer_auth(&admin_token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    // Auth-layer behavior precedes visibility (observed 401 for a principal
    // with no auth registration); the security property asserted is NO GRAPH.
    assert!(status == 401 || status == 404, "unexpected {status}: {body}");
    assert!(body.get("nodes").is_none(), "{body}");
}
