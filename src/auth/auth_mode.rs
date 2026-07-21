//! JWKS-mode authentication configuration.
//!
//! The service now supports only RS256 JWKS verification (Auth V1).
//! Legacy HS256 (`test_hs256`) mode has been removed.

use std::net::IpAddr;

/// JWKS-mode configuration.
#[derive(Debug, Clone)]
pub struct JwksConfig {
    pub jwks_url: String,
    pub issuer: String,
    pub audience: String,
    pub cache_ttl_secs: u64,
    pub http_timeout_secs: u64,
    pub max_stale_secs: u64,
    pub clock_skew_seconds: u64,
}

impl JwksConfig {
    pub fn from_env() -> Result<Self, String> {
        let jwks_url = std::env::var("WORKFLOW_JWKS_URL")
            .map_err(|_| "WORKFLOW_JWKS_URL is required".to_string())?;
        if jwks_url.is_empty() {
            return Err("WORKFLOW_JWKS_URL must not be empty".to_string());
        }
        // Validate URL scheme — only http and https are accepted.
        let scheme = jwks_url.split(':').next().unwrap_or("");
        if scheme != "http" && scheme != "https" {
            return Err(format!(
                "WORKFLOW_JWKS_URL scheme '{scheme}' is not allowed: only http and https are supported"
            ));
        }
        // Reject URL userinfo (embedded credentials).
        if jwks_url.contains('@') {
            return Err(
                "WORKFLOW_JWKS_URL must not contain userinfo (embedded credentials)".to_string(),
            );
        }
        let issuer = std::env::var("WORKFLOW_JWT_ISSUER")
            .map_err(|_| "WORKFLOW_JWT_ISSUER is required".to_string())?;
        if issuer.is_empty() {
            return Err("WORKFLOW_JWT_ISSUER must not be empty".to_string());
        }
        let audience = std::env::var("WORKFLOW_JWT_AUDIENCE")
            .map_err(|_| "WORKFLOW_JWT_AUDIENCE is required".to_string())?;
        if audience.is_empty() {
            return Err("WORKFLOW_JWT_AUDIENCE must not be empty".to_string());
        }
        let cache_ttl_secs = std::env::var("WORKFLOW_JWKS_CACHE_TTL")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .map_err(|_| "WORKFLOW_JWKS_CACHE_TTL must be an unsigned integer".to_string())?;
        let http_timeout_secs = std::env::var("WORKFLOW_JWKS_HTTP_TIMEOUT")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u64>()
            .map_err(|_| "WORKFLOW_JWKS_HTTP_TIMEOUT must be an unsigned integer".to_string())?;
        let max_stale_secs = std::env::var("WORKFLOW_JWKS_MAX_STALE")
            .unwrap_or_else(|_| "600".to_string())
            .parse::<u64>()
            .map_err(|_| "WORKFLOW_JWKS_MAX_STALE must be an unsigned integer".to_string())?;
        let clock_skew_seconds = std::env::var("WORKFLOW_JWT_CLOCK_SKEW")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u64>()
            .map_err(|_| "WORKFLOW_JWT_CLOCK_SKEW must be an unsigned integer".to_string())?;
        Ok(Self {
            jwks_url,
            issuer,
            audience,
            cache_ttl_secs,
            http_timeout_secs,
            max_stale_secs,
            clock_skew_seconds,
        })
    }
}

/// Validate that the environment is consistent with jwks mode.
///
/// In jwks mode, `WORKFLOW_JWT_SECRET` (the legacy HS256 secret) must not be set.
pub fn validate_env() -> Result<(), String> {
    if std::env::var("WORKFLOW_JWT_SECRET").is_ok() {
        return Err("WORKFLOW_JWT_SECRET must not be set (use JWKS keys instead)".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialises env-sensitive tests to prevent race conditions on global env vars.
    static ENV_MTX: Mutex<()> = Mutex::new(());

    fn with_env<F>(vars: &[(&str, &str)], f: F)
    where
        F: FnOnce(),
    {
        let _guard = ENV_MTX.lock().unwrap();
        let originals: Vec<_> = vars
            .iter()
            .map(|(k, v)| {
                let original = std::env::var(k).ok();
                unsafe { std::env::set_var(k, v) };
                (k, original)
            })
            .collect();
        f();
        for (k, original) in originals {
            match original {
                Some(v) => unsafe { std::env::set_var(k, &v) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }

    #[test]
    fn jwks_config_valid_http() {
        with_env(
            &[
                ("WORKFLOW_JWKS_URL", "http://localhost:8080/keys"),
                ("WORKFLOW_JWT_ISSUER", "auth-service"),
                ("WORKFLOW_JWT_AUDIENCE", "svc-workflow"),
            ],
            || {
                assert!(JwksConfig::from_env().is_ok());
            },
        );
    }

    #[test]
    fn jwks_config_valid_https() {
        with_env(
            &[
                (
                    "WORKFLOW_JWKS_URL",
                    "https://auth.example.com/.well-known/jwks.json",
                ),
                ("WORKFLOW_JWT_ISSUER", "auth-service"),
                ("WORKFLOW_JWT_AUDIENCE", "svc-workflow"),
            ],
            || {
                assert!(JwksConfig::from_env().is_ok());
            },
        );
    }

    #[test]
    fn jwks_config_rejects_file_scheme() {
        with_env(
            &[
                ("WORKFLOW_JWKS_URL", "file:///etc/keys.json"),
                ("WORKFLOW_JWT_ISSUER", "issuer"),
                ("WORKFLOW_JWT_AUDIENCE", "audience"),
            ],
            || {
                let err = JwksConfig::from_env().unwrap_err();
                assert!(err.contains("scheme"), "{err}");
            },
        );
    }

    #[test]
    fn jwks_config_rejects_userinfo() {
        with_env(
            &[
                (
                    "WORKFLOW_JWKS_URL",
                    "https://user:pass@auth.example.com/keys.json",
                ),
                ("WORKFLOW_JWT_ISSUER", "issuer"),
                ("WORKFLOW_JWT_AUDIENCE", "audience"),
            ],
            || {
                let err = JwksConfig::from_env().unwrap_err();
                assert!(err.contains("userinfo"), "{err}");
            },
        );
    }

    #[test]
    fn validate_env_rejects_jwt_secret() {
        with_env(&[("WORKFLOW_JWT_SECRET", "some-secret")], || {
            let err = validate_env().unwrap_err();
            assert!(err.contains("WORKFLOW_JWT_SECRET"), "{err}");
        });
    }

    #[test]
    fn validate_env_accepts_clean() {
        let _guard = ENV_MTX.lock().unwrap();
        let original = std::env::var("WORKFLOW_JWT_SECRET").ok();
        unsafe { std::env::remove_var("WORKFLOW_JWT_SECRET") };
        assert!(validate_env().is_ok());
        if let Some(v) = original {
            unsafe { std::env::set_var("WORKFLOW_JWT_SECRET", &v) };
        }
    }
}
