//! Domain owner member management handlers.
//!
//! These handlers let a verified DOMAIN_OWNER manage DOMAIN_MEMBER
//! bindings for their domain.  All endpoints require a Direct Machine
//! Token (`token_use=access`, no delegation).

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::application::domain_membership;
use crate::auth::AuthenticatedPrincipal;
use crate::http::error::ApiError;
use crate::http::AppState;

use super::{idempotency_key, require_scope};

// ---------------------------------------------------------------------------
// Query DTO
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemberListQuery {
    pub before_created_at: Option<String>,
    pub before_id: Option<Uuid>,
    pub limit: Option<u32>,
}

// ---------------------------------------------------------------------------
// Direct-token gate
// ---------------------------------------------------------------------------

/// Reject OBO tokens for all domain member endpoints.
fn require_direct_token(principal: &AuthenticatedPrincipal) -> Result<(), ApiError> {
    if principal.auth_context.token_use != "access"
        || principal.auth_context.delegating_principal_id.is_some()
    {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "direct_token_required",
            "only direct access tokens may manage domain members",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// GET /internal/v1/domains/{domainId}/members
// ---------------------------------------------------------------------------

/// List enabled DOMAIN_MEMBER bindings for a domain.
///
/// Cursor pagination uses `beforeCreatedAt` (RFC 3339) and `beforeId`
/// (UUID), following the same convention as the domain instance list.
pub(crate) async fn list_members(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(domain_id): Path<Uuid>,
    query: Result<Query<MemberListQuery>, QueryRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;
    require_direct_token(&principal)?;

    let Query(query) = query.map_err(ApiError::from_query_rejection)?;
    let limit = query.limit.unwrap_or(20).min(100);

    let before_created_at: Option<DateTime<Utc>> = query
        .before_created_at
        .as_deref()
        .map(|s| {
            DateTime::parse_from_rfc3339(s)
                .map_err(|_| {
                    ApiError::unprocessable(
                        "invalid_cursor",
                        "beforeCreatedAt must be an RFC 3339 timestamp",
                    )
                })
                .map(|dt| dt.with_timezone(&Utc))
        })
        .transpose()?;

    if before_created_at.is_some() != query.before_id.is_some() {
        return Err(ApiError::unprocessable(
            "invalid_cursor",
            "beforeCreatedAt and beforeId must be provided together",
        ));
    }

    let page = domain_membership::list_members(
        &state.pool,
        principal.principal_id.into_uuid(),
        domain_id,
        before_created_at,
        query.before_id,
        limit,
    )
    .await
    .map_err(ApiError::from_domain_membership)?;

    let value = serde_json::to_value(page).map_err(|_| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "internal_consistency_error",
            "failed to serialize member list response",
        )
    })?;
    Ok(Json(value))
}

// ---------------------------------------------------------------------------
// PUT /internal/v1/domains/{domainId}/members/{principalId}
// ---------------------------------------------------------------------------

/// Add a principal as DOMAIN_MEMBER of a domain.
///
/// The target principal must have completed self-projection
/// (`PUT /internal/v1/principals/me`).
///
/// Idempotent: re-adding an existing member returns success.
pub(crate) async fn add_member(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: HeaderMap,
    Path((domain_id, target_principal_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    let key = idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let result = domain_membership::add_member(
        &state.pool,
        principal.principal_id.into_uuid(),
        &key,
        domain_id,
        target_principal_id,
        request_id,
    )
    .await
    .map_err(ApiError::from_domain_membership)?;

    Ok(Json(result))
}

// ---------------------------------------------------------------------------
// DELETE /internal/v1/domains/{domainId}/members/{principalId}
// ---------------------------------------------------------------------------

/// Remove a DOMAIN_MEMBER binding.
///
/// Only removes `DOMAIN_MEMBER` — will not affect `DOMAIN_OWNER`
/// bindings.  Returns 404 if no active member binding exists.
pub(crate) async fn remove_member(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: HeaderMap,
    Path((domain_id, target_principal_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    let key = idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let result = domain_membership::remove_member(
        &state.pool,
        principal.principal_id.into_uuid(),
        &key,
        domain_id,
        target_principal_id,
        request_id,
    )
    .await
    .map_err(ApiError::from_domain_membership)?;

    Ok(Json(result))
}
