//! Authenticated workflow principal.

use std::collections::HashSet;

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use crate::domain::ids::PrincipalId;
use crate::http::error::ApiError;
use crate::http::AppState;

#[derive(Debug, Clone)]
pub struct AuthenticatedPrincipal {
    pub principal_id: PrincipalId,
    scopes: HashSet<String>,
}

impl AuthenticatedPrincipal {
    pub(crate) fn new(principal_id: PrincipalId, scopes: HashSet<String>) -> Self {
        Self {
            principal_id,
            scopes,
        }
    }

    pub fn has_scope(&self, required: &str) -> bool {
        self.scopes.contains(required)
    }
}

impl FromRequestParts<AppState> for AuthenticatedPrincipal {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(AUTHORIZATION)
            .ok_or_else(|| ApiError::unauthorized("unauthenticated", "bearer token is required"))?;
        let value = value.to_str().map_err(|_| {
            ApiError::unauthorized("unauthenticated", "authorization header is invalid")
        })?;
        let token = value.strip_prefix("Bearer ").ok_or_else(|| {
            ApiError::unauthorized("unauthenticated", "authorization scheme must be Bearer")
        })?;
        if token.is_empty() {
            return Err(ApiError::unauthorized(
                "unauthenticated",
                "bearer token is required",
            ));
        }
        state.jwt_verifier.verify(token)
    }
}
