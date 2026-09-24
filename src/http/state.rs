//! HTTP service configuration and shared state.

use std::net::{IpAddr, SocketAddr};

use sqlx::PgPool;

use crate::application::provisioning::ProvisioningConfig;
use crate::application::workflow_instance::query_service::WorkflowQueryService;
use crate::auth::admission::{AdmissionClient, AdmissionConfig};
use crate::auth::{AuthV1CanaryConfig, JwksConfig, JwksVerifier};

/// WORKFLOW_EXECUTION_CONTROL_V1 policy configuration.
#[derive(Debug, Clone)]
pub struct ExecutionControlConfig {
    /// CTR-SWEC-005: the per-(instance, transitionDefinitionId) RETURN limit.
    /// Reaching the limit escalates the target visit to HUMAN_REQUIRED
    /// in-transaction; further RETURNs fail closed (409
    /// return_policy_exhausted). Env WORKFLOW_POLICY_MAX_RETURNS_PER_EDGE,
    /// default 3, minimum 1.
    pub max_returns_per_edge: u32,
}

impl ExecutionControlConfig {
    pub fn from_env() -> Result<Self, String> {
        let max_returns_per_edge = std::env::var("WORKFLOW_POLICY_MAX_RETURNS_PER_EDGE")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u32>()
            .map_err(|_| {
                "WORKFLOW_POLICY_MAX_RETURNS_PER_EDGE must be a positive integer".to_string()
            })?;
        if max_returns_per_edge < 1 {
            return Err("WORKFLOW_POLICY_MAX_RETURNS_PER_EDGE must be >= 1".to_string());
        }
        Ok(Self {
            max_returns_per_edge,
        })
    }
}

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub bind_addr: SocketAddr,
    pub request_body_max_bytes: usize,
    pub request_timeout_seconds: u64,
    pub jwks_config: JwksConfig,
    pub provisioning_config: ProvisioningConfig,
    /// Auth V1 feature flags and allow-list.
    pub auth_v1_canary_config: AuthV1CanaryConfig,
    /// Canonical identity admission configuration (CTR-CIR-003,
    /// SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2).
    pub admission: AdmissionConfig,
    /// WORKFLOW_EXECUTION_CONTROL_V1 policy configuration.
    pub execution_control: ExecutionControlConfig,
}

impl HttpConfig {
    pub fn from_env() -> Result<Self, String> {
        let ip = std::env::var("WORKFLOW_BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1".to_string())
            .parse::<IpAddr>()
            .map_err(|_| "WORKFLOW_BIND_ADDR must be an IP address".to_string())?;

        crate::auth::validate_env()?;

        let port = parse_env("WORKFLOW_PORT", 8989u16)?;
        let request_body_max_bytes = parse_env("WORKFLOW_REQUEST_BODY_MAX_BYTES", 2_097_152usize)?;
        if request_body_max_bytes == 0 {
            return Err("WORKFLOW_REQUEST_BODY_MAX_BYTES must be positive".to_string());
        }
        let request_timeout_seconds = parse_env("WORKFLOW_REQUEST_TIMEOUT_SECS", 30u64)?;
        if request_timeout_seconds == 0 {
            return Err("WORKFLOW_REQUEST_TIMEOUT_SECS must be positive".to_string());
        }

        let jwks_config = JwksConfig::from_env()?;
        let provisioning_config = ProvisioningConfig::from_env()?;
        let auth_v1_canary_config = AuthV1CanaryConfig::from_env();
        let admission = AdmissionConfig::from_env().map_err(|error| error.to_string())?;
        let execution_control = ExecutionControlConfig::from_env()?;

        Ok(Self {
            bind_addr: SocketAddr::new(ip, port),
            request_body_max_bytes,
            request_timeout_seconds,
            jwks_config,
            provisioning_config,
            auth_v1_canary_config,
            admission,
            execution_control,
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
    pub auth_verifier: JwksVerifier,
    pub provisioning_config: ProvisioningConfig,
    /// Auth V1 feature flags and allow-list (used by write guard).
    pub auth_v1_canary_config: AuthV1CanaryConfig,
    /// Canonical identity admission client (CTR-CIR-003). `Some` only when
    /// admission is enabled and construction succeeded; disabled mode keeps
    /// `None`. A failed construction while enabled is a fail-closed boot
    /// panic with a sanitized message.
    pub admission_client: Option<AdmissionClient>,
    /// WORKFLOW_EXECUTION_CONTROL_V1 policy configuration.
    pub execution_control: ExecutionControlConfig,
}

impl AppState {
    pub fn new(pool: PgPool, config: &HttpConfig) -> Self {
        let auth_verifier = JwksVerifier::new(&config.jwks_config, &config.auth_v1_canary_config);
        let admission_client = if config.admission.enabled {
            match AdmissionClient::new(config.admission.clone()) {
                Ok(client) => Some(client),
                Err(error) => panic!("admission client construction failed: {error}"),
            }
        } else {
            None
        };

        Self {
            query_service: WorkflowQueryService::new(pool.clone()),
            auth_verifier,
            provisioning_config: config.provisioning_config.clone(),
            pool,
            auth_v1_canary_config: config.auth_v1_canary_config.clone(),
            admission_client,
            execution_control: config.execution_control.clone(),
        }
    }
}
