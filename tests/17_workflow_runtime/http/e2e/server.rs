use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};

use super::*;

pub(super) struct RunningServer {
    pub(super) base_url: String,
    pub(super) key_pair: common::RsaTestKeyPair,
    mock: common::MockJwksServer,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), std::io::Error>>,
}

impl RunningServer {
    pub(super) async fn start(pool: sqlx::PgPool, body_limit: usize, allowed_sub: &str) -> Self {
        let mock = common::MockJwksServer::start().await;
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind real E2E TCP listener");
        let address = listener.local_addr().expect("listener address");

        let jwks_url = mock.url.clone();

        let config = HttpConfig {
            bind_addr: address,
            request_body_max_bytes: body_limit,
            request_timeout_seconds: 30,
            jwks_config: JwksConfig {
                jwks_url,
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
                allowed_client_id: "e2e-client".to_string(),
                allowed_sub: allowed_sub.to_string(),
                jwks_url: mock.url.clone(),
                issuer: "auth-service".to_string(),
                audience: "svc-workflow".to_string(),
                cache_ttl_secs: 300,
                http_timeout_secs: 5,
                max_stale_secs: 600,
                clock_skew_seconds: 60,
            },
        };
        let state = AppState::new(pool, &config);
        let app = http::router(state, &config);
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        // Wait for eager JWKS fetch
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        Self {
            base_url: format!("http://{address}"),
            key_pair: mock.key_pair.clone(),
            mock,
            shutdown,
            task,
        }
    }

    pub(super) async fn stop(self) -> Result<(), String> {
        let _ = self.shutdown.send(());
        self.task
            .await
            .map_err(|error| format!("join E2E HTTP server: {error}"))?
            .map_err(|error| format!("serve E2E HTTP server: {error}"))
    }
}
