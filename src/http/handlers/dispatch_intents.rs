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
#[serde(rename_all = "camelCase")]
pub struct DueDispatchIntentsQuery {
    limit: Option<i64>,
    /// Keyset continuation cursor (SVC_WORKFLOW_DISPATCH_INTENT_KEYSET_
    /// CONTINUATION_V1 CTR-DKC-001): taken verbatim from a previously
    /// returned record. Both params are strings parsed HERE — typed
    /// Option<DateTime> fields would surface extraction rejections as 400,
    /// not the mandated 422.
    after_next_eligible_at: Option<String>,
    after_dispatch_intent_id: Option<String>,
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

    // CTR-DKC-001: both-or-neither; malformed values land in the same 422
    // invalid_pagination family. Validation is handler-side and precedes the
    // in-snapshot role check (the cursor never reaches the query when it is
    // malformed).
    let cursor = match (
        query.after_next_eligible_at.as_deref(),
        query.after_dispatch_intent_id.as_deref(),
    ) {
        (None, None) => None,
        (Some(ts), Some(id)) => {
            let parsed_ts = chrono::DateTime::parse_from_rfc3339(ts)
                .ok()
                .map(|t| t.with_timezone(&chrono::Utc));
            let parsed_id = id.parse::<uuid::Uuid>().ok();
            match (parsed_ts, parsed_id) {
                (Some(ts), Some(id)) => Some((ts, id)),
                _ => {
                    return Err(ApiError::unprocessable(
                        "invalid_pagination",
                        "afterNextEligibleAt must be RFC3339 and afterDispatchIntentId must be a UUID",
                    ));
                }
            }
        }
        _ => {
            return Err(ApiError::unprocessable(
                "invalid_pagination",
                "afterNextEligibleAt and afterDispatchIntentId must be provided together",
            ));
        }
    };

    let intents = state
        .query_service
        .list_due_dispatch_intents(principal.principal_id.into_uuid(), limit, cursor)
        .await
        .map_err(ApiError::from_query)?;

    Ok(Json(serde_json::json!({ "items": intents })))
}
