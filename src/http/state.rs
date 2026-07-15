//! HTTP service configuration and shared state.

use std::net::{IpAddr, SocketAddr};

use sqlx::PgPool;

use crate::application::workflow_instance::query_service::WorkflowQueryService;
use crate::auth::{JwtConfig, JwtVerifier};

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub bind_addr: SocketAddr,
    pub request_body_max_bytes: usize,
    pub request_timeout_seconds: u64,
    pub jwt: JwtConfig,
}

impl HttpConfig {
    pub fn from_env() -> Result<Self, String> {
        let ip = std::env::var("WORKFLOW_BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1".to_string())
            .parse::<IpAddr>()
            .map_err(|_| "WORKFLOW_BIND_ADDR must be an IP address".to_string())?;
        let port = parse_env("WORKFLOW_PORT", 8989u16)?;
        let request_body_max_bytes = parse_env("WORKFLOW_REQUEST_BODY_MAX_BYTES", 2_097_152usize)?;
        if request_body_max_bytes == 0 {
            return Err("WORKFLOW_REQUEST_BODY_MAX_BYTES must be positive".to_string());
        }
        let request_timeout_seconds = parse_env("WORKFLOW_REQUEST_TIMEOUT_SECS", 30u64)?;
        if request_timeout_seconds == 0 {
            return Err("WORKFLOW_REQUEST_TIMEOUT_SECS must be positive".to_string());
        }
        Ok(Self {
            bind_addr: SocketAddr::new(ip, port),
            request_body_max_bytes,
            request_timeout_seconds,
            jwt: JwtConfig::from_env()?,
        })
    }
}

fn parse_env<T>(name: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr + ToString,
{
    std::env::var(name)
        .unwrap_or_else(|_| default.to_string())
        .parse::<T>()
        .map_err(|_| format!("{name} has an invalid value"))
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) query_service: WorkflowQueryService,
    pub(crate) jwt_verifier: JwtVerifier,
}

impl AppState {
    pub fn new(pool: PgPool, jwt: &JwtConfig) -> Self {
        Self {
            query_service: WorkflowQueryService::new(pool.clone()),
            jwt_verifier: JwtVerifier::new(jwt),
            pool,
        }
    }
}
