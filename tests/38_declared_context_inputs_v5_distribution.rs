//! Workflow v5 distribution `targetPlatforms` — creation-time fail-closed
//! contract (Product #449, svc-workflow slice).
//!
//! Defect (production census 2026-10-04): instances of
//! `bip_article_pipeline_v5_blog_author_202609` were created with
//! `targetPlatforms` missing or `[]` because the published definition's
//! context schema does not require the field; the visits later stranded at
//! `distribution` as forever-ACTIONABLE_NOW work (61 identical
//! DISPATCH_INTENT activations, zero authoritative progress). The broker
//! assistance surface (dsh-agent-core PR #450) stops the loop for existing
//! visits; THIS slice closes the creation-time half for future instances.
//!
//! Mechanism (existing only — no new framework/table/role/state machine):
//! 1. Publish-time definition self-consistency, in the SAME model-agnostic
//!    pass that already compiles the context/submission schemas
//!    (`validate_json_schemas`): a node may DECLARE the context inputs it
//!    consumes via authoring metadata `requiredContextInputs`; every
//!    declared key must be required-by-schema, and an array-typed declared
//!    key must be non-empty (`minItems >= 1`) and registry-backed
//!    (closed `items.enum`). Half-declared contracts can no longer publish.
//! 2. Creation-time refusal then comes from the existing context-schema
//!    gate, which already enforces required/minItems/enum at every context
//!    write (create/revise/repair/revise-and-transition).
//!
//! The corrected Workflow v5 distribution context contract at the bottom of
//! this file is the definition-lane artifact for the later, separately
//! authorized production version publish (NOT done here — no production
//! definition mutation in this lane).
//!
//! RED-first: the publish-rule assertions below were RED on prior head
//! (5d479d83, where declared-but-uncovered inputs published freely); the
//! creation-refusal and no-projection assertions pin the frozen context
//! gate over the real engine.

#[path = "common/mod.rs"]
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
    root: String,
    client: reqwest::Client,
    token: String,
    domain: Uuid,
    owner: Uuid,
    publisher: Uuid,
    pool: sqlx::PgPool,
    _jwks: common::MockJwksServer,
    _server: tokio::task::JoinHandle<()>,
}

async fn harness() -> Harness {
    let pool = common::create_pool().await;
    let owner = common::seed_second_principal(&pool).await;
    let (_, domain) = common::seed_principal_and_domain(&pool).await;
    common::seed_domain_owner(&pool, domain, owner).await;
    let publisher = common::seed_second_principal(&pool).await;
    let jwks = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &jwks.url, vec![owner, publisher]);
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
        root,
        client,
        token,
        domain,
        owner,
        publisher,
        pool,
        _jwks: jwks,
        _server: server,
    }
}

/// The platform registry of the production build-in-public distribution
/// lane (article platforms from the authoring instruction; 小宇宙 for the
/// podcast lane). Business data, never hardcoded in the engine — it lives
/// only in the definition's own context schema enum.
const V5_PLATFORM_REGISTRY: [&str; 4] = ["掘金", "CSDN", "知乎", "小宇宙"];

fn corrected_v5_context_schema() -> Value {
    json!({
        "type": "object",
        "required": ["targetPlatforms", "publisher_principal"],
        "properties": {
            "targetPlatforms": {
                "type": "array",
                "minItems": 1,
                "items": {"enum": V5_PLATFORM_REGISTRY}
            },
            "publisher_principal": {"type": "string", "format": "uuid"}
        }
    })
}

impl Harness {
    async fn request(&self, method: &str, path: &str, body: Value, token: &str) -> (u16, Value) {
        let response = self
            .client
            .request(method.parse().unwrap(), format!("{}{path}", self.root))
            .bearer_auth(token)
            .header("idempotency-key", Uuid::new_v4().to_string())
            .json(&body)
            .send()
            .await
            .unwrap();
        (
            response.status().as_u16(),
            response.json().await.unwrap_or(Value::Null),
        )
    }

    async fn post(&self, path: &str, body: Value) -> (u16, Value) {
        self.request("POST", path, body, &self.token).await
    }

    async fn put(&self, path: &str, body: Value) -> (u16, Value) {
        self.request("PUT", path, body, &self.token).await
    }

    async fn get(&self, path: &str, token: &str) -> (u16, Value) {
        let response = self
            .client
            .get(format!("{}{path}", self.root))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        (
            response.status().as_u16(),
            response.json().await.unwrap_or(Value::Null),
        )
    }

    /// Author a fresh definition + DRAFT version. Returns
    /// (definitionId, versionId, definitionBasePath).
    async fn author_version(&self, context_schema: Value) -> (Uuid, Uuid, String) {
        let path = format!("/internal/v1/domains/{}/definitions", self.domain);
        let (status, def) = self
            .post(
                &path,
                json!({
                    "definitionKey": format!("v5-distribution-{}", Uuid::new_v4()),
                    "displayName": "Workflow v5 distribution targetPlatforms contract"
                }),
            )
            .await;
        assert_eq!(status, 200, "{def}");
        let definition_id = Uuid::parse_str(def["workflowDefinitionId"].as_str().unwrap()).unwrap();
        let base = format!("{path}/{definition_id}");

        let (status, version) = self
            .post(
                &format!("{base}/versions"),
                json!({"contextSchema": context_schema}),
            )
            .await;
        assert_eq!(status, 200, "{version}");
        let version_id = Uuid::parse_str(version["definitionVersionId"].as_str().unwrap()).unwrap();
        (definition_id, version_id, base)
    }

    /// Replace the draft graph, returning the raw outcome — negative tests
    /// assert the authoring-time rejection itself.
    async fn replace_draft(
        &self,
        base: &str,
        version_id: Uuid,
        context_schema: Value,
        nodes: Value,
        transitions: Value,
    ) -> (u16, Value) {
        self.put(
            &format!("{base}/draft"),
            json!({
                "definitionVersionId": version_id,
                "contextSchema": context_schema,
                "nodes": nodes,
                "transitions": transitions
            }),
        )
        .await
    }

    /// Author a fully valid draft (200 expected). Returns (definitionId,
    /// versionId).
    async fn author_draft(
        &self,
        context_schema: Value,
        nodes: Value,
        transitions: Value,
    ) -> (Uuid, Uuid) {
        let (definition_id, version_id, base) = self.author_version(context_schema.clone()).await;
        let (status, replaced) = self
            .replace_draft(&base, version_id, context_schema, nodes, transitions)
            .await;
        assert_eq!(status, 200, "{replaced}");
        (definition_id, version_id)
    }

    /// The v5-faithful Legacy-model minimal graph: draft -> distribution
    /// (owner resolved from the context's publisher_principal) -> done,
    /// with the distribution ADVANCE requiring `distributionInstances` in
    /// its submission — the exact production defect shape's structure.
    /// `distribution_metadata` carries the node's declared context inputs.
    fn v5_legacy_nodes(&self, distribution_metadata: Value) -> Value {
        json!([
            {"node_key": "draft", "display_name": "Draft", "order_index": 0,
             "node_type": "DRAFT", "assignee_ref_type": "WORKFLOW_CREATOR",
             "primary_advance_transition_key": "start"},
            {"node_key": "distribution", "display_name": "Distribution", "order_index": 1,
             "node_type": "NORMAL", "assignee_ref_type": "INSTANCE_INPUT_PRINCIPAL",
             "assignee_input_key": "publisher_principal",
             "primary_advance_transition_key": "complete-distribution",
             "metadata": distribution_metadata},
            {"node_key": "done", "display_name": "Done", "order_index": 2,
             "node_type": "TERMINAL"}
        ])
    }

    fn v5_legacy_transitions(&self) -> Value {
        json!([
            {"transition_key": "start", "display_name": "Enter distribution",
             "source_node_key": "draft", "target_node_key": "distribution",
             "transition_effect": "ADVANCE"},
            {"transition_key": "complete-distribution", "display_name": "Finish distribution",
             "source_node_key": "distribution", "target_node_key": "done",
             "transition_effect": "ADVANCE",
             "submission_schema": {"type": "object",
                 "required": ["distributionInstances"],
                 "properties": {"distributionInstances":
                     {"type": "array", "minItems": 1}}}}
        ])
    }

    async fn publish(&self, definition_id: Uuid, version_id: Uuid) -> (u16, Value) {
        self.post(
            &format!(
                "/internal/v1/domains/{}/definitions/{definition_id}/publish",
                self.domain
            ),
            json!({"versionId": version_id}),
        )
        .await
    }

    async fn create_instance(
        &self,
        version_id: Uuid,
        context: Value,
        external_reference: &str,
        idempotency_key: &str,
    ) -> (u16, Value) {
        let response = self
            .client
            .post(format!("{}/internal/v1/workflow-instances", self.root))
            .bearer_auth(&self.token)
            .header("idempotency-key", idempotency_key)
            .json(&json!({
                "domainId": self.domain,
                "definitionVersionId": version_id,
                "executionClass": "BUSINESS",
                "externalReference": external_reference,
                "metadata": {},
                "contextPayload": context
            }))
            .send()
            .await
            .unwrap();
        (
            response.status().as_u16(),
            response.json().await.unwrap_or(Value::Null),
        )
    }

    /// Seed GLOBAL_SCHEDULER_READ for the owner so the dispatch-intent feed
    /// (the due/dispatchable projection) is assertable — disposable DB.
    async fn seed_global_scheduler_read(&self) {
        sqlx::query(
            "INSERT INTO global_role_bindings (binding_id, principal_id, role_key, enabled) \
             VALUES ($1, $2, 'GLOBAL_SCHEDULER_READ', TRUE)",
        )
        .bind(Uuid::new_v4())
        .bind(self.owner)
        .execute(&self.pool)
        .await
        .unwrap();
    }
}

fn graph_error_codes(body: &Value) -> Vec<String> {
    body["error"]["details"]["errors"]
        .as_array()
        .map(|errors| {
            errors
                .iter()
                .filter_map(|e| e["code"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Publish-time self-consistency: declared inputs must be schema-covered.
// The rule runs in the same model-agnostic pass that compiles the schemas,
// so a violating contract is rejected at draft-authoring time already and
// can never reach publication. These were RED on prior head (declared-but-
// uncovered inputs authored and published freely — the exact gap that let
// the production v5 contract strand).
// ---------------------------------------------------------------------------

/// Assert the contract is refused at authoring time with the exact rule
/// code, and that the version can still not be published afterwards.
async fn assert_rejected_at_draft_and_unpublishable(
    h: &Harness,
    context_schema: Value,
    distribution_metadata: Value,
    expected_code: &str,
) {
    let (definition_id, version_id, base) = h.author_version(context_schema.clone()).await;
    let (status, replaced) = h
        .replace_draft(
            &base,
            version_id,
            context_schema,
            h.v5_legacy_nodes(distribution_metadata),
            h.v5_legacy_transitions(),
        )
        .await;
    assert_eq!(
        status, 422,
        "half-declared contract must fail at authoring time: {replaced}"
    );
    assert_eq!(
        replaced["error"]["code"], "graph_validation_failed",
        "{replaced}"
    );
    let codes = graph_error_codes(&replaced);
    assert!(
        codes.iter().any(|c| c == expected_code),
        "expected {expected_code}, got: {codes:?}"
    );

    // The version was never graphed: publication stays impossible.
    let (status, published) = h.publish(definition_id, version_id).await;
    assert_eq!(
        status, 422,
        "uncovered contract must not publish: {published}"
    );
    assert_eq!(
        published["error"]["code"], "graph_validation_failed",
        "{published}"
    );
}

#[tokio::test]
async fn declared_input_not_required_by_schema_fails_publish() {
    let h = harness().await;
    // Schema types the field but does not require it.
    let schema = json!({
        "type": "object",
        "required": ["publisher_principal"],
        "properties": {
            "targetPlatforms": {"type": "array", "minItems": 1,
                                 "items": {"enum": V5_PLATFORM_REGISTRY}},
            "publisher_principal": {"type": "string"}
        }
    });
    assert_rejected_at_draft_and_unpublishable(
        &h,
        schema,
        json!({"requiredContextInputs": ["targetPlatforms"]}),
        "DECLARED_CONTEXT_INPUT_NOT_REQUIRED",
    )
    .await;
}

#[tokio::test]
async fn declared_input_without_schema_type_fails_publish() {
    let h = harness().await;
    // Required but UNTYPED: any present value (including []) satisfies
    // creation, so a required-only schema still strands the declaring node —
    // the 44dee9da defect through an apparently-covered contract.
    let schema = json!({
        "type": "object",
        "required": ["targetPlatforms", "publisher_principal"],
        "properties": {
            "publisher_principal": {"type": "string"}
        }
    });
    assert_rejected_at_draft_and_unpublishable(
        &h,
        schema,
        json!({"requiredContextInputs": ["targetPlatforms"]}),
        "DECLARED_CONTEXT_INPUT_UNTYPED",
    )
    .await;
}

#[tokio::test]
async fn declared_array_input_allowing_empty_fails_publish() {
    let h = harness().await;
    // Required + typed, but no minItems: [] would satisfy creation — the
    // exact production 44dee9da defect shape.
    let schema = json!({
        "type": "object",
        "required": ["targetPlatforms", "publisher_principal"],
        "properties": {
            "targetPlatforms": {"type": "array", "items": {"enum": V5_PLATFORM_REGISTRY}},
            "publisher_principal": {"type": "string"}
        }
    });
    assert_rejected_at_draft_and_unpublishable(
        &h,
        schema,
        json!({"requiredContextInputs": ["targetPlatforms"]}),
        "DECLARED_CONTEXT_INPUT_ALLOWS_EMPTY",
    )
    .await;
}

#[tokio::test]
async fn declared_array_input_with_unbounded_values_fails_publish() {
    let h = harness().await;
    // Required + non-empty but no closed value set: unknown platforms would
    // satisfy creation.
    let schema = json!({
        "type": "object",
        "required": ["targetPlatforms", "publisher_principal"],
        "properties": {
            "targetPlatforms": {"type": "array", "minItems": 1,
                                 "items": {"type": "string"}},
            "publisher_principal": {"type": "string"}
        }
    });
    assert_rejected_at_draft_and_unpublishable(
        &h,
        schema,
        json!({"requiredContextInputs": ["targetPlatforms"]}),
        "DECLARED_CONTEXT_INPUT_UNBOUNDED_VALUES",
    )
    .await;
}

#[tokio::test]
async fn malformed_declared_inputs_fail_publish() {
    let h = harness().await;
    assert_rejected_at_draft_and_unpublishable(
        &h,
        corrected_v5_context_schema(),
        // Declaration is a bare string instead of an array of keys.
        json!({"requiredContextInputs": "targetPlatforms"}),
        "DECLARED_CONTEXT_INPUTS_MALFORMED",
    )
    .await;
}

#[tokio::test]
async fn declared_input_rule_covers_visit_activation_model() {
    // Semantic model 3 (the model class the production v5 definition runs
    // on): the rule lives in the model-agnostic authoring/publish pass, so
    // a TASK/TERMINAL graph declaring an uncovered input must also fail.
    let h = harness().await;
    let smv3_schema = json!({
        "type": "object",
        "properties": {
            "targetPlatforms": {"type": "array", "minItems": 1,
                                 "items": {"enum": V5_PLATFORM_REGISTRY}}
        }
    });
    let path = format!("/internal/v1/domains/{}/definitions", h.domain);
    let (status, def) = h
        .post(
            &path,
            json!({
                "definitionKey": format!("v5-smv3-{}", Uuid::new_v4()),
                "displayName": "Workflow v5 SMV3 declared inputs"
            }),
        )
        .await;
    assert_eq!(status, 200, "{def}");
    let definition_id = Uuid::parse_str(def["workflowDefinitionId"].as_str().unwrap()).unwrap();
    let base = format!("{path}/{definition_id}");
    let (status, version) = h
        .post(
            &format!("{base}/versions"),
            json!({"semanticModelVersion": 3, "contextSchema": smv3_schema}),
        )
        .await;
    assert_eq!(status, 200, "{version}");
    let version_id = Uuid::parse_str(version["definitionVersionId"].as_str().unwrap()).unwrap();
    let (status, replaced) = h
        .put(
            &format!("{base}/draft"),
            json!({
                "definitionVersionId": version_id,
                "contextSchema": smv3_schema,
                "nodes": [
                    {"node_key": "authoring", "display_name": "Authoring", "order_index": 0,
                     "node_type": "TASK", "assignee_ref_type": "WORKFLOW_CREATOR",
                     "primary_advance_transition_key": "hand-off"},
                    {"node_key": "distribution", "display_name": "Distribution", "order_index": 1,
                     "node_type": "TASK", "assignee_ref_type": "FIXED_PRINCIPAL",
                     "fixed_principal_id": h.owner,
                     "primary_advance_transition_key": "finish",
                     "metadata": {"requiredContextInputs": ["targetPlatforms"]}},
                    {"node_key": "done", "display_name": "Done", "order_index": 2,
                     "node_type": "TERMINAL"}
                ],
                "transitions": [
                    {"transition_key": "hand-off", "display_name": "Hand off",
                     "source_node_key": "authoring", "target_node_key": "distribution",
                     "transition_effect": "ADVANCE"},
                    {"transition_key": "finish", "display_name": "Finish",
                     "source_node_key": "distribution", "target_node_key": "done",
                     "transition_effect": "ADVANCE"}
                ]
            }),
        )
        .await;
    assert_eq!(
        status, 422,
        "SMV3 declared input outside context_schema.required must fail authoring: {replaced}"
    );
    let codes = graph_error_codes(&replaced);
    assert!(
        codes
            .iter()
            .any(|c| c == "DECLARED_CONTEXT_INPUT_NOT_REQUIRED"),
        "expected DECLARED_CONTEXT_INPUT_NOT_REQUIRED, got: {codes:?}"
    );

    // Publication stays impossible.
    let (status, published) = h.publish(definition_id, version_id).await;
    assert_eq!(
        status, 422,
        "SMV3 uncovered contract must not publish: {published}"
    );
    assert_eq!(
        published["error"]["code"], "graph_validation_failed",
        "{published}"
    );
}

// ---------------------------------------------------------------------------
// Corrected contract end-to-end: publish succeeds, creation refuses every
// invalid platform context before any visit exists, accepts the registered
// list, and no actionable/due projection exists for refused contexts.
// Creation refusals were enforced on prior head too (frozen context gate) —
// these assertions pin that behavior over the real engine.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn corrected_v5_contract_refuses_invalid_context_and_projects_nothing() {
    let h = harness().await;
    let (definition_id, version_id) = h
        .author_draft(
            corrected_v5_context_schema(),
            h.v5_legacy_nodes(json!({"requiredContextInputs": ["targetPlatforms"]})),
            h.v5_legacy_transitions(),
        )
        .await;

    // The fully-covered contract publishes (rule is satisfied, not bypassed).
    let (status, published) = h.publish(definition_id, version_id).await;
    assert_eq!(status, 200, "{published}");
    assert_eq!(published["versionStatus"], "PUBLISHED", "{published}");

    h.seed_global_scheduler_read().await;

    let publisher = h.publisher.to_string();
    let refusals = [
        ("missing", json!({"publisher_principal": publisher})),
        (
            "empty-array",
            json!({"targetPlatforms": [], "publisher_principal": publisher}),
        ),
        (
            "unknown-platform",
            json!({"targetPlatforms": ["不存在的平台"], "publisher_principal": publisher}),
        ),
    ];

    for (shape, context) in &refusals {
        let key = Uuid::new_v4().to_string();
        let external = format!("v5-refusal-{shape}");
        let (status, body) = h
            .create_instance(version_id, context.clone(), &external, &key)
            .await;
        assert_eq!(
            status, 422,
            "invalid platform context ({shape}) must be refused at instance creation: {body}"
        );
        assert_eq!(
            body["error"]["code"], "context_validation_failed",
            "{shape}: {body}"
        );

        // Idempotency preserved: replaying the SAME refused request with the
        // same idempotency key replays the deterministic failure — no
        // instance is created, no new error shape appears.
        let (replay_status, replay_body) = h
            .create_instance(version_id, context.clone(), &external, &key)
            .await;
        assert_eq!(replay_status, 422, "{shape} replay: {replay_body}");
        assert_eq!(
            replay_body["error"]["code"], "context_validation_failed",
            "{shape} replay: {replay_body}"
        );
    }

    // Positive control: the valid registered list creates the instance.
    let (status, created) = h
        .create_instance(
            version_id,
            json!({"targetPlatforms": ["掘金", "知乎"],
                    "publisher_principal": h.publisher.to_string()}),
            "v5-valid-platforms",
            &Uuid::new_v4().to_string(),
        )
        .await;
    assert_eq!(
        status, 201,
        "valid registered platform list must create the instance: {created}"
    );
    let created_id = created["workflowInstanceId"].as_str().unwrap().to_string();

    // No actionable/due projection for refused contexts: the domain owns
    // exactly ONE instance (the valid one) — refused contexts never became
    // instances, so no distribution visit, no worklist entry, and no
    // dispatchable feed record can exist for them. Assertions are scoped to
    // this test's own domain so the shared test database cannot bleed other
    // suites' artifacts into the verdict.
    let (status, list) = h
        .get(
            &format!(
                "/internal/v1/workflow-instances/domain?domainId={}&lifecycle=all&limit=100",
                h.domain
            ),
            &h.token,
        )
        .await;
    assert_eq!(status, 200, "{list}");
    let items = list["items"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        items.len(),
        1,
        "exactly the valid instance may exist in the test domain: {list}"
    );
    assert_eq!(
        items[0]["workflow_instance_id"],
        json!(created_id),
        "{list}"
    );
    for (shape, _) in &refusals {
        let rendered = serde_json::to_string(&list).unwrap();
        assert!(
            !rendered.contains(&format!("v5-refusal-{shape}")),
            "refused context {shape} must not appear in the domain projection: {rendered}"
        );
    }

    // Worklist (the assignee's actionable view) names only real work.
    let (status, worklist) = h
        .get("/internal/v1/worklists/assigned-to-me", &h.token)
        .await;
    assert_eq!(status, 200, "{worklist}");
    let rendered = serde_json::to_string(&worklist).unwrap();
    for (shape, _) in &refusals {
        assert!(
            !rendered.contains(&format!("v5-refusal-{shape}")),
            "refused context {shape} must not be actionable in the worklist: {rendered}"
        );
    }

    // Due/dispatchable projection (dispatch intents): no feed record may
    // reference any instance of this domain — invalid platform context
    // never minted a dispatchable distribution visit.
    let (status, feed) = h
        .get("/internal/v1/dispatch-intents?limit=100", &h.token)
        .await;
    assert_eq!(status, 200, "{feed}");
    let feed_items = feed["items"].as_array().cloned().unwrap_or_default();
    let offending: Vec<&Value> = feed_items
        .iter()
        .filter(|item| {
            item["workflowInstanceId"]
                .as_str()
                .map(|id| id == created_id)
                .unwrap_or(false)
        })
        .collect();
    assert!(
        offending.is_empty(),
        "no dispatchable visit may exist for refused platform contexts: {offending:?}"
    );
}
