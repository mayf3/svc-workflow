//! Exercise the actual route, token scope and opaque error mapping with a
//! disposable database and freshly generated mock-JWKS keys.

use super::*;
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{admission::AdmissionConfig, AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self as http_api, AppState, ExecutionControlConfig, HttpConfig};
use tower::ServiceExt;
use uuid::Uuid;

struct Harness {
    app: axum::Router,
    jwks: common::MockJwksServer,
}

impl Harness {
    async fn new(pool: sqlx::PgPool) -> Self {
        let jwks = common::MockJwksServer::start().await;
        let config = HttpConfig {
            admission: AdmissionConfig::disabled(),
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            request_body_max_bytes: 2_097_152,
            request_timeout_seconds: 30,
            jwks_config: JwksConfig {
                jwks_url: jwks.url.clone(),
                issuer: "auth-service".into(),
                audience: "svc-workflow".into(),
                cache_ttl_secs: 300,
                http_timeout_secs: 5,
                max_stale_secs: 600,
                clock_skew_seconds: 60,
            },
            // A member's read must not rely on provisioning/admin authority.
            provisioning_config: ProvisioningConfig::new(vec![]),
            execution_control: ExecutionControlConfig {
                max_returns_per_edge: 3,
            },
            auth_v1_canary_config: AuthV1CanaryConfig {
                enabled: true,
                write_enabled: true,
                allowed_client_id: "test-client".into(),
                jwks_url: jwks.url.clone(),
                ..Default::default()
            },
        };
        Self {
            app: http_api::router(AppState::new(pool, &config), &config),
            jwks,
        }
    }

    async fn request(
        &self,
        actor: Uuid,
        scope: &str,
        method: Method,
        path: &str,
        body: Value,
    ) -> (StatusCode, Value) {
        let token = common::v1_token(actor, scope, "test-client", 300, &self.jwks.key_pair);
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .header("idempotency-key", Uuid::new_v4().to_string())
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 2_097_152).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    async fn read(&self, actor: Uuid, version: Uuid) -> (StatusCode, Value) {
        self.request(
            actor,
            "workflow.read",
            Method::GET,
            &format!("/internal/v1/definition-versions/{version}/input-contract"),
            Value::Null,
        )
        .await
    }
}

// Catches a route accidentally using H-5, returning management fields or the
// latest version, or accepting a different ordinary creator audience.
#[tokio::test]
async fn same_domain_agent_reads_exact_schema_then_creates_without_owner_or_admin() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member = seed_domain_member(&pool, seeded.domain_id, true).await;
    let service = DefinitionService::new(PgDefinitionRepository::new(pool.clone()));
    let newer = service
        .create_draft_version(CreateDraftVersion {
            actor_principal_id: seeded.owner_id,
            workflow_definition_id: seeded.definition_id,
            context_schema: Some(json!({"type": "object", "required": ["different"]})),
            json_schema_dialect: None,
            validator_version: None,
            metadata: None,
            semantic_model_version: 1,
        })
        .await
        .unwrap();
    let (nodes, transitions) = minimal_graph();
    service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: seeded.owner_id,
            definition_version_id: newer.id.into_uuid(),
            context_schema: None,
            nodes,
            transitions,
        })
        .await
        .unwrap();
    service
        .publish_version(
            PublishVersion {
                actor_principal_id: seeded.owner_id,
                definition_version_id: newer.id.into_uuid(),
                expected_revision: None,
            },
            AdmissionGate::disabled(),
        )
        .await
        .unwrap();

    let h = Harness::new(pool.clone()).await;
    let (status, contract) = h.read(member, seeded.version_id).await;
    assert_eq!(status, StatusCode::OK, "{contract}");
    assert_eq!(
        contract,
        json!({
            "definitionVersionId": seeded.version_id,
            "definitionId": seeded.definition_id,
            "versionNumber": 1,
            "versionStatus": "PUBLISHED",
            "contextSchema": input_schema()
        })
    );

    let (status, created) = h
        .request(
            member,
            "workflow.execute",
            Method::POST,
            "/internal/v1/workflow-instances",
            json!({
                "domainId": seeded.domain_id,
                "definitionVersionId": contract["definitionVersionId"],
                "contextPayload": {"title": "schema supplied by the member read"},
                "metadata": {}
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let instance_id = Uuid::parse_str(created["workflowInstanceId"].as_str().unwrap()).unwrap();
    let actual: (Uuid, Uuid) = sqlx::query_as(
        "SELECT created_by_principal_id, definition_version_id FROM workflow_instances \
         WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(actual, (member, seeded.version_id));
}

// Catches existence/status leaks before the active same-domain binding gate.
#[tokio::test]
async fn unauthorized_agents_get_the_same_opaque_404_for_existing_and_missing_versions() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let stranger = seed_second_principal(&pool).await;
    let revoked = seed_domain_member(&pool, seeded.domain_id, false).await;
    let (_, other_domain) = seed_principal_and_domain(&pool).await;
    let cross_domain = seed_domain_member(&pool, other_domain, true).await;
    let h = Harness::new(pool.clone()).await;

    for actor in [stranger, revoked, cross_domain] {
        let (status, existing) = h.read(actor, seeded.version_id).await;
        let (missing_status, missing) = h.read(actor, Uuid::new_v4()).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{existing}");
        assert_eq!(missing_status, StatusCode::NOT_FOUND, "{missing}");
        assert_eq!(existing["error"]["code"], "definition_not_found");
        assert_eq!(existing["error"], missing["error"]);
        assert!(!existing.to_string().contains("contextSchema"));

        let (create_status, denied) = h
            .request(
                actor,
                "workflow.execute",
                Method::POST,
                "/internal/v1/workflow-instances",
                json!({
                    "domainId": seeded.domain_id,
                    "definitionVersionId": seeded.version_id,
                    "contextPayload": {"title": "unauthorized creator"},
                    "metadata": {}
                }),
            )
            .await;
        assert_eq!(create_status, StatusCode::FORBIDDEN, "{denied}");
        assert_eq!(denied["error"]["code"], "domain_membership_required");
    }
}

// Catches status/scope guards being removed from the member-facing route.
#[tokio::test]
async fn only_published_versions_with_read_scope_return_schema() {
    use svc_workflow::application::definition::commands::RevokeVersion;

    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member = seed_domain_member(&pool, seeded.domain_id, true).await;
    let h = Harness::new(pool.clone()).await;
    let service = DefinitionService::new(PgDefinitionRepository::new(pool.clone()));
    let draft = service
        .create_draft_version(CreateDraftVersion {
            actor_principal_id: seeded.owner_id,
            workflow_definition_id: seeded.definition_id,
            context_schema: Some(json!({"type": "object", "x-secret": "draft-only"})),
            json_schema_dialect: None,
            validator_version: None,
            metadata: None,
            semantic_model_version: 1,
        })
        .await
        .unwrap();
    let (status, body) = h.read(member, draft.id.into_uuid()).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["error"]["code"], "version_not_published");
    assert!(!body.to_string().contains("draft-only"));

    let (status, body) = h
        .request(
            member,
            "workflow.execute",
            Method::GET,
            &format!(
                "/internal/v1/definition-versions/{}/input-contract",
                seeded.version_id
            ),
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(!body.to_string().contains("contextSchema"));

    service
        .deprecate_version(DeprecateVersion {
            actor_principal_id: seeded.owner_id,
            definition_version_id: seeded.version_id,
            deprecation_reason: None,
        })
        .await
        .unwrap();
    let (status, body) = h.read(member, seeded.version_id).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["error"]["code"], "version_not_published");
    assert!(!body.to_string().contains("contextSchema"));

    service
        .revoke_version(RevokeVersion {
            actor_principal_id: seeded.owner_id,
            definition_version_id: seeded.version_id,
        })
        .await
        .unwrap();
    let (status, body) = h.read(member, seeded.version_id).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["error"]["code"], "version_not_published");
    assert!(!body.to_string().contains("contextSchema"));
}
