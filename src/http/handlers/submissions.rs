//! Submission history query endpoint.
//!
//! `GET /internal/v1/workflow-instances/{workflowInstanceId}/submissions` is a
//! thin HTTP adapter over the existing
//! [`WorkflowQueryService::list_submission_history`](crate::application::workflow_instance::query_service::WorkflowQueryService::list_submission_history)
//! application query. It performs no authorization of its own — visibility is
//! enforced entirely by the reused query, whose row-level rules allow a full
//! viewer to read every submission, the actor to read submissions it authored,
//! and a historical participant to read `RETURN` submissions whose
//! `relatedSubmissionIds` reference one of the actor's own submissions.
//!
//! Pagination reuses the application query's existing `TimeUuidCursor`
//! keyset — the same `(created_at, id)` tuple the service compares against —
//! with no second set of filtering or ordering rules.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::Json;

use crate::application::workflow_instance::query_types::{
    ListSubmissionHistory, Page, SubmissionHistoryItem, TimeUuidCursor,
};
use crate::auth::AuthenticatedPrincipal;
use crate::http::dto::SubmissionHistoryQuery;
use crate::http::error::ApiError;
use crate::http::AppState;

use super::{path_uuid, require_scope};

/// GET /internal/v1/workflow-instances/{workflowInstanceId}/submissions
///
/// Returns the submission history for a workflow instance. Visibility and
/// pagination semantics are owned entirely by `list_submission_history`.
pub(crate) async fn list(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(workflow_instance_id): Path<String>,
    query: Result<Query<SubmissionHistoryQuery>, QueryRejection>,
) -> Result<Json<Page<SubmissionHistoryItem>>, ApiError> {
    require_scope(&principal, "workflow.read")?;
    let workflow_instance_id = path_uuid(&workflow_instance_id)?;
    let Query(query) = query.map_err(ApiError::from_query_rejection)?;
    let after = parse_after_cursor(query.after_created_at, query.after_id)?;
    let page = state
        .query_service
        .list_submission_history(ListSubmissionHistory {
            actor_principal_id: principal.principal_id.into_uuid(),
            workflow_instance_id,
            after,
            limit: query.limit,
        })
        .await
        .map_err(ApiError::from_query)?;
    Ok(Json(page))
}

/// Parse the composite forward cursor from two optional query string parameters.
///
/// Both `afterCreatedAt` and `afterId` must be present together, or both
/// absent — mirroring [`parse_worklist_cursor`](super::worklists) but for the
/// `after` direction. Invalid or half-present values produce a 422.
fn parse_after_cursor(
    after_created_at: Option<String>,
    after_id: Option<String>,
) -> Result<Option<TimeUuidCursor>, ApiError> {
    match (after_created_at, after_id) {
        (None, None) => Ok(None),
        (Some(created_at), Some(id)) => {
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
                .map_err(|_| {
                    ApiError::unprocessable(
                        "invalid_cursor",
                        "afterCreatedAt must be an RFC 3339 timestamp",
                    )
                })?
                .with_timezone(&chrono::Utc);
            let id = uuid::Uuid::parse_str(&id).map_err(|_| {
                ApiError::unprocessable("invalid_cursor", "afterId must be a valid UUID")
            })?;
            Ok(Some(TimeUuidCursor { created_at, id }))
        }
        (Some(_), None) => Err(ApiError::unprocessable(
            "invalid_cursor",
            "afterCreatedAt requires afterId",
        )),
        (None, Some(_)) => Err(ApiError::unprocessable(
            "invalid_cursor",
            "afterId requires afterCreatedAt",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn after_cursor_none_when_both_absent() {
        assert!(parse_after_cursor(None, None).unwrap().is_none());
    }

    #[test]
    fn after_cursor_valid_pair() {
        let result = parse_after_cursor(
            Some("2024-01-15T10:30:00Z".to_string()),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
        )
        .unwrap();
        let cursor = result.unwrap();
        assert_eq!(
            cursor.id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn after_cursor_rejects_missing_after_id() {
        let err =
            parse_after_cursor(Some("2024-01-15T10:30:00Z".to_string()), None).unwrap_err();
        assert_eq!(err.code(), "invalid_cursor");
    }

    #[test]
    fn after_cursor_rejects_missing_after_created_at() {
        let err = parse_after_cursor(
            None,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
        )
        .unwrap_err();
        assert_eq!(err.code(), "invalid_cursor");
    }

    #[test]
    fn after_cursor_rejects_invalid_timestamp() {
        let err = parse_after_cursor(
            Some("not-a-timestamp".to_string()),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
        )
        .unwrap_err();
        assert_eq!(err.code(), "invalid_cursor");
    }

    #[test]
    fn after_cursor_rejects_invalid_uuid() {
        let err = parse_after_cursor(
            Some("2024-01-15T10:30:00Z".to_string()),
            Some("not-a-uuid".to_string()),
        )
        .unwrap_err();
        assert_eq!(err.code(), "invalid_cursor");
    }
}
