//! Accepted CTR-VAI-001/011: formal model 3 authoring must retain its model.
#[path = "common/mod.rs"]
mod common;
use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::{json, Value};
use svc_workflow::application::definition::repository::DefinitionRepository;
use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::domain::definition::model::SemanticModelVersion;
use svc_workflow::http::{self, AppState, HttpConfig};
use svc_workflow::store::postgres::definition_repository::PgDefinitionRepository;
use tower::ServiceExt;
use uuid::Uuid;

fn build_app(pool: sqlx::PgPool, jwks_url: &str, admin_ids: Vec<Uuid>) -> axum::Router {
    let config = HttpConfig {
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

async fn request(
    app: &axum::Router,
    method: &str,
    path: &str,
    token: &str,
    body: Value,
) -> (u16, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", Uuid::new_v4().to_string())
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn model3_http_authoring_roundtrips_through_repository_and_publish() {
    let pool = common::create_pool().await;
    let owner = common::seed_second_principal(&pool).await;
    let (_, domain) = common::seed_principal_and_domain(&pool).await;
    common::seed_domain_owner(&pool, domain, owner).await;
    let jwks = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &jwks.url, vec![]);
    let token = common::v1_token(
        owner,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &jwks.key_pair,
    );
    let root = format!("/internal/v1/domains/{domain}/definitions");
    let (status, def) = request(&app, "POST", &root, &token,
        json!({"definitionKey":format!("model3-{}",Uuid::new_v4()),"displayName":"Model 3 conformance"})).await;
    assert_eq!(status, 200, "{def}");
    let def_id = def["workflowDefinitionId"].as_str().unwrap();
    let base = format!("{root}/{def_id}");
    let repo = PgDefinitionRepository::new(pool.clone());
    // The old model mappings and omitted default must retain their meaning.
    for (body, expected) in [
        (json!({}), SemanticModelVersion::Legacy),
        (
            json!({"semanticModelVersion":1}),
            SemanticModelVersion::Legacy,
        ),
        (
            json!({"semanticModelVersion":2}),
            SemanticModelVersion::Minimal,
        ),
    ] {
        let (status, version) =
            request(&app, "POST", &format!("{base}/versions"), &token, body).await;
        assert_eq!(status, 200, "{version}");
        let id = Uuid::parse_str(version["definitionVersionId"].as_str().unwrap()).unwrap();
        assert_eq!(
            repo.get_version(id).await.unwrap().semantic_model_version,
            expected
        );
    }
    let before: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM workflow_definition_versions WHERE workflow_definition_id=$1",
    )
    .bind(Uuid::parse_str(def_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    for bad in [-1, 0, 4, 32767] {
        let (status, result) = request(
            &app,
            "POST",
            &format!("{base}/versions"),
            &token,
            json!({"semanticModelVersion":bad}),
        )
        .await;
        assert_eq!(status, 422, "{result}");
        assert!(
            result
                .to_string()
                .contains("invalid_semantic_model_version"),
            "{result}"
        );
    }
    let after: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM workflow_definition_versions WHERE workflow_definition_id=$1",
    )
    .bind(Uuid::parse_str(def_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        before, after,
        "rejected model values must not create versions"
    );
    let (status, version) = request(
        &app,
        "POST",
        &format!("{base}/versions"),
        &token,
        json!({"semanticModelVersion":3}),
    )
    .await;
    assert_eq!(status, 200, "{version}");
    let id = Uuid::parse_str(version["definitionVersionId"].as_str().unwrap()).unwrap();
    assert_eq!(
        repo.get_version(id).await.unwrap().semantic_model_version,
        SemanticModelVersion::VisitActivation
    );
    assert_eq!(
        repo.lock_version(id).await.unwrap().semantic_model_version,
        SemanticModelVersion::VisitActivation
    );
    let mut graph = json!({"definitionVersionId":id,"nodes":[
        {"node_key":"work","display_name":"Work","order_index":0,"node_type":"TASK","assignee_ref_type":"WORKFLOW_CREATOR","primary_advance_transition_key":"finish"},
        {"node_key":"done","display_name":"Done","order_index":1,"node_type":"TERMINAL"}],
        "transitions":[{"transition_key":"finish","display_name":"Finish","source_node_key":"work","target_node_key":"done","transition_effect":"ADVANCE"}]});
    // A model 3 graph must reject Legacy node kinds, then accept TASK.
    graph["nodes"][0]["node_type"] = json!("NORMAL");
    let (status, bad_graph) =
        request(&app, "PUT", &format!("{base}/draft"), &token, graph.clone()).await;
    assert!(status >= 400, "{bad_graph}");
    assert!(
        bad_graph.to_string().contains("graph validation failed"),
        "{bad_graph}"
    );
    let untouched: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM workflow_node_definitions WHERE definition_version_id=$1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(untouched.0, 0);
    graph["nodes"][0]["node_type"] = json!("TASK");
    let (status, replaced) = request(&app, "PUT", &format!("{base}/draft"), &token, graph).await;
    assert_eq!(status, 200, "{replaced}");
    let (status, published) = request(
        &app,
        "POST",
        &format!("{base}/publish"),
        &token,
        json!({"versionId":id}),
    )
    .await;
    assert_eq!(status, 200, "{published}");
    assert_eq!(published["versionStatus"], "PUBLISHED");
    let readback = repo.get_version(id).await.unwrap();
    assert_eq!(
        readback.semantic_model_version,
        SemanticModelVersion::VisitActivation
    );
    assert_eq!(
        serde_json::to_value(&readback).unwrap()["semantic_model_version"],
        3
    );
    let persisted: (i16,String) = sqlx::query_as("SELECT semantic_model_version,version_status::text FROM workflow_definition_versions WHERE definition_version_id=$1")
        .bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(persisted, (3, "PUBLISHED".into()));
    // DB's unchanged closed model set remains the final backstop.
    let rejected = sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id,workflow_definition_id,version_number,version_status,semantic_model_version) VALUES ($1,$2,99,'DRAFT',4)")
        .bind(Uuid::new_v4()).bind(Uuid::parse_str(def_id).unwrap()).execute(&pool).await;
    assert!(rejected.is_err());
}
