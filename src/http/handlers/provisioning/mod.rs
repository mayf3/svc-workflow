//! Provisioning authorization helpers, extractor, and handler modules.

pub(crate) mod definitions;
pub(crate) mod domains;
pub(crate) mod principals;
pub(crate) mod role_bindings;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderMap;

use crate::auth::AuthenticatedPrincipal;
use crate::http::error::ApiError;
use crate::http::AppState;

/// Extractor that verifies provisioning authorization.
///
/// Wraps `AuthenticatedPrincipal` and runs the provisioning-specific
/// authorization checks: scope, allow-list, and token_use gate.
pub(crate) struct ProvisioningAuth(pub(crate) AuthenticatedPrincipal);

impl FromRequestParts<AppState> for ProvisioningAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let principal = AuthenticatedPrincipal::from_request_parts(parts, state).await?;
        authorize_provisioning(&principal, state)?;
        Ok(ProvisioningAuth(principal))
    }
}

/// Maximum Idempotency-Key length.
const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;

/// Authorize a provisioning operation.
///
/// Checks:
/// 1. Must have `workflow.provision` scope.
/// 2. `JWT.sub` must be in the configured allow-list.
/// 3. Token must be a direct access token (no OBO).
pub(crate) fn authorize_provisioning(
    principal: &AuthenticatedPrincipal,
    state: &AppState,
) -> Result<(), ApiError> {
    if !principal.has_scope("workflow.provision") {
        return Err(ApiError::forbidden());
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

    if principal.auth_context.token_use == "workflow_obo" {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "provisioning_not_allowed",
            "OBO tokens are not allowed for provisioning",
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
