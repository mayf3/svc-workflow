//! GET /internal/v1/dispatch-intents — the bounded Scheduler due poll.
//!
//! Fail-closed `GLOBAL_SCHEDULER_READ` gate inside the query snapshot
//! (CTR-VAI-009); the projection is exactly the v0.4.0 §5.7 minimum record.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::auth::AuthenticatedPrincipal;
use crate::http::error::ApiError;
use crate::http::handlers::require_scope;
use crate::http::AppState;

#[derive(Debug, Deserialize)]
pub struct DueDispatchIntentsQuery {
    limit: Option<i64>,
}

pub(crate) async fn list_due(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<DueDispatchIntentsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;

    let limit = query.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(ApiError::unprocessable(
            "invalid_pagination",
            "limit must be 1-100",
        ));
    }

    let intents = state
        .query_service
        .list_due_dispatch_intents(principal.principal_id.into_uuid(), limit)
        .await
        .map_err(ApiError::from_query)?;

    Ok(Json(serde_json::json!({ "items": intents })))
}
