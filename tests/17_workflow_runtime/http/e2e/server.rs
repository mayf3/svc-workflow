use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use svc_workflow::auth::JwtConfig;
use svc_workflow::http::{self, AppState, HttpConfig};

const JWT_SECRET: &str = "isolated-e2e-secret-that-is-at-least-32-bytes";

#[derive(Serialize)]
struct Claims {
    sub: String,
    iss: &'static str,
    aud: &'static str,
    exp: usize,
    iat: usize,
    principal_type: &'static str,
    #[serde(rename = "type")]
    token_type: &'static str,
    version: &'static str,
    scope: String,
}

pub(super) fn token(principal_id: uuid::Uuid, scope: &str) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    encode(
        &Header::new(Algorithm::HS256),
        &Claims {
            sub: principal_id.to_string(),
            iss: "auth-service",
            aud: "svc-workflow",
            exp: now + 300,
            iat: now,
            principal_type: "agent",
            token_type: "access",
            version: "v1",
            scope: scope.to_string(),
        },
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("sign local E2E token")
}

pub(super) struct RunningServer {
    pub(super) base_url: String,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), std::io::Error>>,
}

impl RunningServer {
    pub(super) async fn start(pool: sqlx::PgPool, body_limit: usize) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind real E2E TCP listener");
        let address = listener.local_addr().expect("listener address");
        let jwt = JwtConfig {
            secret: JWT_SECRET.to_string(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            clock_skew_seconds: 0,
        };
        let config = HttpConfig {
            bind_addr: address,
            request_body_max_bytes: body_limit,
            request_timeout_seconds: 30,
            jwt: jwt.clone(),
        };
        let app = http::router(AppState::new(pool, &jwt), &config);
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });
        Self {
            base_url: format!("http://{address}"),
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
