//! Worklist query endpoints.
//!
//! Both endpoints require `workflow.read` scope and derive the actor
//! exclusively from `JWT.sub`. No query parameter can override the actor.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::Json;

use crate::application::workflow_instance::query_types::{
    AssignedWorkItem, CreatorDraftItem, ListAssignedToMe, ListCreatorOwnedDrafts, Page,
    TimeUuidCursor,
};
use crate::auth::AuthenticatedPrincipal;
use crate::http::dto::WorklistQuery;
use crate::http::error::ApiError;
use crate::http::AppState;

use super::require_scope;

/// GET /internal/v1/worklists/assigned-to-me
///
/// Returns worklist items currently assigned to the authenticated actor.
/// Pagination uses `before` cursor (descending by created_at).
pub(crate) async fn assigned_to_me(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    query: Result<Query<WorklistQuery>, QueryRejection>,
) -> Result<Json<Page<AssignedWorkItem>>, ApiError> {
    require_scope(&principal, "workflow.read")?;
    let Query(query) = query.map_err(ApiError::from_query_rejection)?;
    let before = parse_worklist_cursor(query.before_created_at, query.before_id)?;
    let page = state
        .query_service
        .list_assigned_to_me(ListAssignedToMe {
            actor_principal_id: principal.principal_id.into_uuid(),
            before,
            limit: query.limit,
        })
        .await
        .map_err(ApiError::from_query)?;
    Ok(Json(page))
}

/// GET /internal/v1/worklists/creator-owned-drafts
///
/// Returns draft instances created by the authenticated actor.
/// Pagination uses `before` cursor (descending by created_at).
pub(crate) async fn creator_owned_drafts(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    query: Result<Query<WorklistQuery>, QueryRejection>,
) -> Result<Json<Page<CreatorDraftItem>>, ApiError> {
    require_scope(&principal, "workflow.read")?;
    let Query(query) = query.map_err(ApiError::from_query_rejection)?;
    let before = parse_worklist_cursor(query.before_created_at, query.before_id)?;
    let page = state
        .query_service
        .list_creator_owned_drafts(ListCreatorOwnedDrafts {
            actor_principal_id: principal.principal_id.into_uuid(),
            before,
            limit: query.limit,
        })
        .await
        .map_err(ApiError::from_query)?;
    Ok(Json(page))
}

/// Parse the composite cursor from two optional query string parameters.
///
/// Both `beforeCreatedAt` and `beforeId` must be present together, or both
/// absent. Invalid or missing values produce a 422 Unprocessable Entity.
fn parse_worklist_cursor(
    before_created_at: Option<String>,
    before_id: Option<String>,
) -> Result<Option<TimeUuidCursor>, ApiError> {
    match (before_created_at, before_id) {
        (None, None) => Ok(None),
        (Some(created_at), Some(id)) => {
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
                .map_err(|_| {
                    ApiError::unprocessable(
                        "invalid_cursor",
                        "beforeCreatedAt must be an RFC 3339 timestamp",
                    )
                })?
                .with_timezone(&chrono::Utc);
            let id = uuid::Uuid::parse_str(&id).map_err(|_| {
                ApiError::unprocessable("invalid_cursor", "beforeId must be a valid UUID")
            })?;
            Ok(Some(TimeUuidCursor { created_at, id }))
        }
        (Some(_), None) => Err(ApiError::unprocessable(
            "invalid_cursor",
            "beforeCreatedAt requires beforeId",
        )),
        (None, Some(_)) => Err(ApiError::unprocessable(
            "invalid_cursor",
            "beforeId requires beforeCreatedAt",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_parsing_none_when_both_absent() {
        assert!(parse_worklist_cursor(None, None).unwrap().is_none());
    }

    #[test]
    fn cursor_parsing_valid_pair() {
        let result = parse_worklist_cursor(
            Some("2024-01-15T10:30:00Z".to_string()),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
        )
        .unwrap();
        assert!(result.is_some());
        let cursor = result.unwrap();
        assert_eq!(
            cursor.id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn cursor_parsing_rejects_missing_before_id() {
        let err =
            parse_worklist_cursor(Some("2024-01-15T10:30:00Z".to_string()), None).unwrap_err();
        assert_eq!(err.code(), "invalid_cursor");
    }

    #[test]
    fn cursor_parsing_rejects_missing_before_created_at() {
        let err = parse_worklist_cursor(
            None,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
        )
        .unwrap_err();
        assert_eq!(err.code(), "invalid_cursor");
    }

    #[test]
    fn cursor_parsing_rejects_invalid_timestamp() {
        let err = parse_worklist_cursor(
            Some("not-a-timestamp".to_string()),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
        )
        .unwrap_err();
        assert_eq!(err.code(), "invalid_cursor");
    }

    #[test]
    fn cursor_parsing_rejects_invalid_uuid() {
        let err = parse_worklist_cursor(
            Some("2024-01-15T10:30:00Z".to_string()),
            Some("not-a-uuid".to_string()),
        )
        .unwrap_err();
        assert_eq!(err.code(), "invalid_cursor");
    }
}
