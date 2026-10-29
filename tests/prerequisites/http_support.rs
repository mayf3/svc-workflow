use super::*;
use svc_workflow::{
    application::provisioning::ProvisioningConfig,
    auth::{AuthV1CanaryConfig, JwksConfig},
    http::{self, AppState, HttpConfig},
};
pub(super) async fn app(pool: &PgPool, m: &common::MockJwksServer) -> axum::Router {
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
