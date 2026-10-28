use super::common;
use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use svc_workflow::{
    application::provisioning::ProvisioningConfig,
    auth::{AuthV1CanaryConfig, JwksConfig},
    http::{self, AppState, HttpConfig},
};
use tower::ServiceExt;
async fn app(pool: &PgPool, m: &common::MockJwksServer) -> axum::Router {
    app_with_write(pool, m, true).await
}
async fn app_with_write(
    pool: &PgPool,
    m: &common::MockJwksServer,
    write_enabled: bool,
) -> axum::Router {
    let c = HttpConfig {
        admission: svc_workflow::auth::admission::AdmissionConfig::disabled(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        request_body_max_bytes: 2_097_152,
        request_timeout_seconds: 30,
        jwks_config: JwksConfig {
            jwks_url: m.url.clone(),
            issuer: "auth-service".into(),
            audience: "svc-workflow".into(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
        provisioning_config: ProvisioningConfig::new(vec![]),
        execution_control: http::ExecutionControlConfig {
            max_returns_per_edge: 3,
        },
        auth_v1_canary_config: AuthV1CanaryConfig {
            enabled: true,
            write_enabled,
            allowed_client_id: "gcr-http".into(),
            allowed_sub: String::new(),
            allowed_delegating_sub: String::new(),
            jwks_url: m.url.clone(),
            issuer: "auth-service".into(),
            audience: "svc-workflow".into(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
    };
    http::router(AppState::new(pool.clone(), &c), &c)
}
fn post(uri: &str, token: &str, body: String) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", Uuid::new_v4().to_string())
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}
#[sqlx::test]
async fn bind_http_scope_owner_and_closed_input(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let mock = common::MockJwksServer::start().await;
    let router = app(&pool, &mock).await;
    let uri = format!(
        "/internal/v1/assistance-cases/{}/governance-child",
        case.assistance_case_id
    );
    let body = json!({"governanceChildInstanceId":c.instance,"acceptanceTransitionId":c.transition,"expectedWorkflowStateVersion":2});
    for (actor, scope, status) in [
        (f.owner, "workflow.read", StatusCode::FORBIDDEN),
        (f.agent, "workflow.execute", StatusCode::FORBIDDEN),
        (f.owner, "workflow.execute", StatusCode::OK),
    ] {
        let t = common::v1_token(actor, scope, "gcr-http", 300, &mock.key_pair);
        let r = router
            .clone()
            .oneshot(post(&uri, &t, body.to_string()))
            .await
            .unwrap();
        assert_eq!(r.status(), status);
    }
    let t = common::v1_token(f.owner, "workflow.execute", "gcr-http", 300, &mock.key_pair);
    let mut forged = body;
    forged["ownerPrincipalId"] = json!(f.owner);
    assert_eq!(
        router
            .oneshot(post(&uri, &t, forged.to_string()))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
}
#[sqlx::test]
async fn linked_http_rejects_duplicate_reserved_review_members(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let b = bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
        .await
        .unwrap();
    finish(&pool, &c, &b, "ACCEPTED").await;
    let detail = serde_json::to_value(
        get_assistance_case(&pool, f.owner, case.assistance_case_id)
            .await
            .unwrap(),
    )
    .unwrap();
    let p = &detail["governanceTerminalProof"];
    let r = json!({"bindingId":b["bindingId"],"terminalEventId":p["terminalEventId"],"submissionId":p["submissionId"],"submissionDigest":p["submissionDigest"],"acceptanceContractDigest":b["acceptanceContractDigest"]});
    let mut body=json!({"expectedWorkflowStateVersion":3,"resolution":{"message":"Reviewed","supportingPayload":{"governanceChildReview":r}}}).to_string();
    body = body.replace(
        "\"governanceChildReview\":{",
        &format!(
            "\"governanceChildReview\":{{\"bindingId\":{},",
            b["bindingId"]
        ),
    );
    let mock = common::MockJwksServer::start().await;
    let router = app(&pool, &mock).await;
    let t = common::v1_token(f.owner, "workflow.execute", "gcr-http", 300, &mock.key_pair);
    let response = router
        .oneshot(post(
            &format!(
                "/internal/v1/assistance-cases/{}/resolve",
                case.assistance_case_id
            ),
            &t,
            body,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        case_status(&pool, case.assistance_case_id).await.0,
        "OWNER_PENDING"
    );
}

#[sqlx::test]
async fn bind_preserves_existing_write_gate(pool: PgPool) {
    let mock = common::MockJwksServer::start().await;
    let router = app_with_write(&pool, &mock, false).await;
    let response = router
        .oneshot(post(
            &format!(
                "/internal/v1/assistance-cases/{}/governance-child",
                Uuid::new_v4()
            ),
            "invalid",
            json!({}).to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
