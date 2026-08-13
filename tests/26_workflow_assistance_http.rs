//! Black-box router coverage for the frozen Assistance V1 HTTP contract.

#[allow(dead_code, unused_imports)]
#[path = "common/mod.rs"]
mod common;
#[allow(dead_code)]
#[path = "25_workflow_assistance/helpers.rs"]
mod helpers;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};

fn token(principal_id: Uuid, scope: &str, key_pair: &common::RsaTestKeyPair) -> String {
    common::v1_token(principal_id, scope, "assistance-http", 300, key_pair)
}

fn request(
    method: &str,
    uri: &str,
    token: &str,
    key: Option<&str>,
    body: Option<Value>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"));
    if let Some(key) = key {
        builder = builder.header("idempotency-key", key);
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn assistance_http_agent_owner_human_query_and_resume_contract() {
    let pool = common::create_pool().await;
    let fixture = helpers::setup(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let config = HttpConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        request_body_max_bytes: 2_097_152,
        request_timeout_seconds: 30,
        jwks_config: JwksConfig {
            jwks_url: mock.url.clone(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
        provisioning_config: ProvisioningConfig::new(Vec::new()),
        auth_v1_canary_config: AuthV1CanaryConfig {
            enabled: true,
            write_enabled: true,
            allowed_client_id: "assistance-http".to_string(),
            allowed_sub: String::new(),
            allowed_delegating_sub: String::new(),
            jwks_url: mock.url.clone(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
    };
    let state = AppState::new(pool.clone(), &config);
    let app = http::router(state, &config);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let agent = token(
        fixture.agent,
        "workflow.execute workflow.read",
        &mock.key_pair,
    );
    let owner = token(
        fixture.owner,
        "workflow.execute workflow.read",
        &mock.key_pair,
    );
    let coordinator = token(fixture.coordinator, "workflow.read", &mock.key_pair);
    let requested = app
        .clone()
        .oneshot(request(
            "POST",
            &format!(
                "/internal/v1/workflow-instances/{}/assistance-cases",
                fixture.instance
            ),
            &agent,
            Some("http-assistance-request"),
            Some(json!({
                "currentNodeVisitId": fixture.visit,
                "expectedWorkflowStateVersion": 1,
                "request": {"message": "Need help through HTTP"}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(requested.status(), StatusCode::CREATED);
    let requested = json_body(requested).await;
    let case_id = requested["assistanceCaseId"].as_str().unwrap();
    assert_eq!(requested["workflowStateVersion"], 2);

    let owner_inbox = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/assistance-cases/owner-inbox",
            &owner,
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(owner_inbox.status(), StatusCode::OK);
    assert_eq!(
        json_body(owner_inbox).await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let escalated = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/assistance-cases/{case_id}/escalate-to-human"),
            &owner,
            Some("http-assistance-escalate"),
            Some(json!({
                "expectedWorkflowStateVersion": 2,
                "escalation": {"message": "Human approval required"}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(escalated.status(), StatusCode::OK);
    assert_eq!(json_body(escalated).await["workflowStateVersion"], 3);

    let human_required = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/assistance-cases/human-required",
            &coordinator,
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(human_required.status(), StatusCode::OK);
    let human_required = json_body(human_required).await;
    let projected = human_required["items"][0].clone();
    assert!(projected.get("contextPayload").is_none());
    assert!(projected.get("submissions").is_none());
    assert!(projected.get("nodeVisitId").is_none());
    assert!(projected.get("resolvedByPrincipalId").is_none());
    assert!(projected.get("resolution").is_none());
    assert!(projected.get("workflowStateVersion").is_none());
    assert!(projected.get("currentNodeVisitId").is_none());
    assert!(projected.get("voidedAt").is_none());
    assert_eq!(projected.as_object().unwrap().len(), 11);

    let coordinator_detail = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/internal/v1/assistance-cases/{case_id}"),
            &coordinator,
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(coordinator_detail.status(), StatusCode::OK);
    assert_eq!(json_body(coordinator_detail).await, projected);

    let resolved = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/assistance-cases/{case_id}/resolve"),
            &owner,
            Some("http-assistance-resolve"),
            Some(json!({
                "expectedWorkflowStateVersion": 3,
                "resolution": {
                    "message": "Owner accepts Human approval",
                    "supportingPayload": {"approvalReference": "HUMAN-HTTP-1"}
                }
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resolved.status(), StatusCode::OK);
    assert_eq!(json_body(resolved).await["workflowStateVersion"], 4);

    let coordinator_after_resolution = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/internal/v1/assistance-cases/{case_id}"),
            &coordinator,
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(coordinator_after_resolution.status(), StatusCode::NOT_FOUND);

    let requested_by_me = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/assistance-cases/requested-by-me?status=RESOLVED",
            &agent,
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(requested_by_me.status(), StatusCode::OK);
    assert_eq!(
        json_body(requested_by_me).await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let detail = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/internal/v1/assistance-cases/{case_id}"),
            &agent,
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail = json_body(detail).await;
    assert_eq!(detail["workflowStateVersion"], 4);
    assert_eq!(detail["currentNodeVisitId"], fixture.visit.to_string());

    let transitioned = app
        .clone()
        .oneshot(request(
            "POST",
            &format!(
                "/internal/v1/workflow-instances/{}/transitions",
                fixture.instance
            ),
            &agent,
            Some("http-transition-after-resolution"),
            Some(json!({
                "expectedWorkflowStateVersion": 4,
                "transitionDefinitionId": fixture.transition,
                "submissionPayload": {}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(transitioned.status(), StatusCode::OK);
    assert_eq!(json_body(transitioned).await["workflowStateVersion"], 5);
}
