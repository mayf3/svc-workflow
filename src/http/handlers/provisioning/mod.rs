//! Provisioning authorization helpers, extractor, and handler modules.

pub(crate) mod definitions;
pub(crate) mod domains;
pub(crate) mod principals;
pub(crate) mod role_bindings;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method};

use crate::auth::AuthenticatedPrincipal;
use crate::http::error::ApiError;
use crate::http::AppState;

/// Extractor that verifies provisioning authorization.
///
/// Wraps `AuthenticatedPrincipal` and runs the provisioning-specific
/// authorization checks: scope, allow-list, principal type, and delegation gates.
pub(crate) struct ProvisioningAuth {
    pub(crate) principal: AuthenticatedPrincipal,
    pub(crate) actor_provisioned: bool,
}

impl FromRequestParts<AppState> for ProvisioningAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let principal = AuthenticatedPrincipal::from_request_parts(parts, state).await?;
        authorize_provisioning(&principal, state)?;
        let actor: Option<(bool, String)> = sqlx::query_as(
            "SELECT enabled, principal_type::text FROM principals WHERE principal_id = $1",
        )
        .bind(principal.principal_id.into_uuid())
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| {
            tracing::error!(error = %error, "provisioning actor lookup failed");
            ApiError::service_unavailable("service_unavailable", "storage is unavailable")
        })?;
        let bootstrap_route =
            parts.method == Method::POST && parts.uri.path() == "/internal/v1/admin/principals";
        let actor_provisioned = match actor {
            Some((true, principal_type)) if principal_type == "AGENT" => true,
            Some(_) => {
                return Err(ApiError::new(
                    axum::http::StatusCode::FORBIDDEN,
                    "provisioning_not_allowed",
                    "provisioning actor is disabled or is not an agent",
                ))
            }
            None if bootstrap_route => false,
            None => {
                return Err(ApiError::new(
                    axum::http::StatusCode::FORBIDDEN,
                    "provisioning_actor_not_provisioned",
                    "provisioning actor must bootstrap its principal first",
                ))
            }
        };
        Ok(ProvisioningAuth {
            principal,
            actor_provisioned,
        })
    }
}

/// Maximum Idempotency-Key length.
const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;

/// Authorize a provisioning operation.
///
/// Checks:
/// 1. Must have `workflow.admin` scope.
/// 2. `JWT.sub` must be in the configured allow-list.
/// 3. The verified principal must be an agent.
/// 4. Token must be a direct access token (no OBO markers).
pub(crate) fn authorize_provisioning(
    principal: &AuthenticatedPrincipal,
    state: &AppState,
) -> Result<(), ApiError> {
    if !principal.has_scope("workflow.admin") {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "insufficient_scope",
            "workflow.admin scope is required",
        ));
    }

    if !state
        .provisioning_config
        .is_allowed(&principal.principal_id)
    {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "provisioning_not_allowed",
            "principal is not allowed to perform provisioning",
        ));
    }

    if principal.auth_context.principal_type != "agent"
        || principal.auth_context.token_use != "access"
        || principal.auth_context.delegating_principal_id.is_some()
    {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "provisioning_not_allowed",
            "only direct agent access tokens may perform provisioning",
        ));
    }

    Ok(())
}

/// Validate and extract the Idempotency-Key header.
pub(crate) fn idempotency_key(headers: &HeaderMap) -> Result<String, ApiError> {
    let value = headers.get("idempotency-key").ok_or_else(|| {
        ApiError::bad_request(
            "missing_idempotency_key",
            "Idempotency-Key header is required",
        )
    })?;
    let value = value.to_str().map_err(|_| {
        ApiError::bad_request(
            "invalid_idempotency_key",
            "Idempotency-Key header is invalid",
        )
    })?;
    if value.is_empty()
        || value.len() > MAX_IDEMPOTENCY_KEY_LEN
        || !value
            .as_bytes()
            .iter()
            .all(|byte| (0x21..=0x7e).contains(byte))
    {
        return Err(ApiError::bad_request(
            "invalid_idempotency_key",
            "Idempotency-Key must be 1-128 visible ASCII characters",
        ));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    use super::*;
    use crate::application::provisioning::ProvisioningConfig;
    use crate::auth::{AuthContext, AuthMode, Hs256Config};
    use crate::domain::ids::PrincipalId;
    use crate::http::HttpConfig;

    fn fixture(principal_type: &str, delegated: bool) -> (AuthenticatedPrincipal, AppState) {
        let principal_id = PrincipalId::from_uuid(Uuid::new_v4());
        let context = AuthContext {
            subject: principal_id,
            principal_type: principal_type.to_string(),
            token_use: "access".to_string(),
            delegating_principal_id: delegated.then(|| PrincipalId::from_uuid(Uuid::new_v4())),
            authorized_party: delegated.then(|| "adc".to_string()),
            token_id: delegated.then(|| "jti".to_string()),
            audience: "svc-workflow".to_string(),
            scope: "workflow.admin".to_string(),
        };
        let principal = AuthenticatedPrincipal::new_with_context(
            principal_id,
            HashSet::from(["workflow.admin".to_string()]),
            context,
        );
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1/unused")
            .unwrap();
        let config = HttpConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            request_body_max_bytes: 1024,
            request_timeout_seconds: 1,
            auth_mode: AuthMode::TestHs256,
            hs256_config: Some(Hs256Config {
                secret: "test-secret-at-least-32-bytes-long".to_string(),
                issuer: "auth-service".to_string(),
                audience: "svc-workflow".to_string(),
                clock_skew_seconds: 0,
            }),
            jwks_config: None,
            provisioning_config: ProvisioningConfig::new(vec![principal_id]),
        };
        (principal, AppState::new(pool, &config))
    }

    #[test]
    fn only_direct_agents_pass_claim_authorization() {
        let (agent, state) = fixture("agent", false);
        assert!(authorize_provisioning(&agent, &state).is_ok());

        let (human, state) = fixture("human", false);
        assert_eq!(
            authorize_provisioning(&human, &state).unwrap_err().code(),
            "provisioning_not_allowed"
        );

        let (delegated, state) = fixture("agent", true);
        assert_eq!(
            authorize_provisioning(&delegated, &state)
                .unwrap_err()
                .code(),
            "provisioning_not_allowed"
        );
    }
}
