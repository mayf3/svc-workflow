//! Auth V1 feature flags and allow-list configuration.
//!
//! These flags control whether Auth V1 is open.  When off or allow-list
//! does not match, requests are rejected directly with no fallback.

/// Feature flags and allow-list for Auth V1.
///
/// All flags default-off — V1 auth is locked down until explicitly configured.
#[derive(Debug, Clone)]
pub struct AuthV1CanaryConfig {
    /// Master switch.  When `false` (the default) all V1-authenticated
    /// requests are rejected.
    pub enabled: bool,
    /// Separate write gate.  When `false` (the default), write requests
    /// (create / transition) are rejected with a definitive 403.
    pub write_enabled: bool,
    /// If non-empty, only this `client_id` is accepted.
    pub allowed_client_id: String,
    /// If non-empty, only this `sub` is accepted.
    pub allowed_sub: String,
    /// If non-empty, OBO tokens must have `act.sub` matching this value.
    pub allowed_delegating_sub: String,
    /// JWKS URL for RS256 key resolution.
    pub jwks_url: String,
    /// Expected exact issuer (contract: `"auth-service"`).
    pub issuer: String,
    /// Expected exact audience (contract: `"svc-workflow"`).
    pub audience: String,
    /// JWKS cache TTL (default 300 s).
    pub cache_ttl_secs: u64,
    /// JWKS fetch HTTP timeout (default 5 s).
    pub http_timeout_secs: u64,
    /// Max stale time for cache (default 600 s).
    pub max_stale_secs: u64,
    /// Clock skew tolerance (default 60 s).
    pub clock_skew_seconds: u64,
}

impl Default for AuthV1CanaryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            write_enabled: false,
            allowed_client_id: String::new(),
            allowed_sub: String::new(),
            allowed_delegating_sub: String::new(),
            jwks_url: String::new(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        }
    }
}

impl AuthV1CanaryConfig {
    /// Build config from environment variables.
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("AUTH_V1_CANARY_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            write_enabled: std::env::var("AUTH_V1_CANARY_WRITE_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            allowed_client_id: std::env::var("AUTH_V1_CANARY_ALLOWED_CLIENT_ID")
                .unwrap_or_default(),
            allowed_sub: std::env::var("AUTH_V1_CANARY_ALLOWED_SUB").unwrap_or_default(),
            allowed_delegating_sub: std::env::var("AUTH_V1_CANARY_ALLOWED_DELEGATING_SUB")
                .unwrap_or_default(),
            jwks_url: std::env::var("WORKFLOW_JWKS_URL").unwrap_or_default(),
            issuer: std::env::var("WORKFLOW_JWT_ISSUER")
                .unwrap_or_else(|_| "auth-service".to_string()),
            audience: std::env::var("WORKFLOW_JWT_AUDIENCE")
                .unwrap_or_else(|_| "svc-workflow".to_string()),
            cache_ttl_secs: std::env::var("WORKFLOW_JWKS_CACHE_TTL")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            http_timeout_secs: std::env::var("WORKFLOW_JWKS_HTTP_TIMEOUT")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            max_stale_secs: std::env::var("WORKFLOW_JWKS_MAX_STALE")
                .unwrap_or_else(|_| "600".to_string())
                .parse()
                .unwrap_or(600),
            clock_skew_seconds: std::env::var("WORKFLOW_JWT_CLOCK_SKEW")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
        }
    }

    /// Quick check: canary is enabled AND both allow-list values are present.
    pub fn is_active(&self) -> bool {
        self.enabled && !self.allowed_client_id.is_empty() && !self.allowed_sub.is_empty()
    }

    /// Quick check: canary is active AND write operations are permitted.
    pub fn write_active(&self) -> bool {
        self.is_active() && self.write_enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_active_requires_all_fields() {
        let mut config = AuthV1CanaryConfig {
            enabled: true,
            write_enabled: false,
            allowed_client_id: "client".to_string(),
            allowed_sub: "sub".to_string(),
            ..Default::default()
        };
        assert!(config.is_active());

        config.enabled = false;
        assert!(!config.is_active());
        config.enabled = true;

        config.allowed_client_id.clear();
        assert!(!config.is_active());
        config.allowed_client_id = "client".to_string();

        config.allowed_sub.clear();
        assert!(!config.is_active());
    }

    #[test]
    fn config_default_is_disabled() {
        let config = AuthV1CanaryConfig::default();
        assert!(!config.enabled);
        assert!(!config.is_active());
    }

    #[test]
    fn write_active_requires_write_enabled() {
        let config = AuthV1CanaryConfig {
            enabled: true,
            write_enabled: false,
            allowed_client_id: "client".to_string(),
            allowed_sub: "sub".to_string(),
            ..Default::default()
        };
        assert!(config.is_active());
        assert!(!config.write_active());

        let config = AuthV1CanaryConfig {
            enabled: true,
            write_enabled: true,
            allowed_client_id: "client".to_string(),
            allowed_sub: "sub".to_string(),
            ..Default::default()
        };
        assert!(config.write_active());
    }
}
