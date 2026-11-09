use super::{path_uuid, require_scope};
use crate::{
    application::workflow_instance::current_task_input::{
        CurrentTaskInputError, CurrentTaskInputRequest,
    },
    auth::AuthenticatedPrincipal,
    http::{error::ApiError, AppState},
};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde_json::Value;

pub(crate) async fn read(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<String>,
    query: Result<Query<Vec<(String, String)>>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;
    let id = path_uuid(&id)?;
    let Query(pairs) = query.map_err(|_| error(CurrentTaskInputError::Invalid))?;
    let request = CurrentTaskInputRequest::parse(pairs).map_err(error)?;
    state
        .query_service
        .current_task_input(principal.principal_id.into_uuid(), id, request)
        .await
        .map(Json)
        .map_err(error)
}
fn error(error: CurrentTaskInputError) -> ApiError {
    match error {
        CurrentTaskInputError::Query(e) => ApiError::from_query(e),
        CurrentTaskInputError::Invalid => ApiError::unprocessable(
            "invalid_current_task_input",
            "current task input parameters are invalid",
        ),
        CurrentTaskInputError::Stale => ApiError::new(
            StatusCode::CONFLICT,
            "current_task_input_stale",
            "current task coordinates have changed",
        ),
        CurrentTaskInputError::BodyChanged => ApiError::new(
            StatusCode::CONFLICT,
            "current_task_body_changed",
            "current task body has changed",
        ),
        CurrentTaskInputError::BodyNotFound => ApiError::new(
            StatusCode::NOT_FOUND,
            "current_task_body_not_found_or_not_visible",
            "current task body not found or not visible",
        ),
    }
}
