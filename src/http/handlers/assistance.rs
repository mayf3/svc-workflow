//! Workflow Assistance V1 HTTP adapter.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::application::workflow_instance::assistance::{
    escalate_assistance_to_human, get_assistance_case, get_human_required_assistance_case,
    list_assistance, list_human_required_assistance, request_assistance, resolve_assistance,
    AssistanceCursor, AssistanceListView,
};
use crate::auth::AuthenticatedPrincipal;
use crate::domain::ids::{AssistanceCaseId, NodeVisitId, WorkflowInstanceId};
use crate::domain::workflow_instance::assistance::{
    AssistanceCaseStatus, AssistanceError, AssistancePayload, EscalateAssistanceCommand,
    RequestAssistanceCommand, ResolveAssistanceCommand,
};
use crate::http::error::ApiError;
use crate::http::AppState;

use super::{idempotency_key, require_scope};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RequestAssistanceBody {
    current_node_visit_id: Uuid,
    expected_workflow_state_version: i32,
    request: AssistancePayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EscalateAssistanceBody {
    expected_workflow_state_version: i32,
    escalation: AssistancePayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolveAssistanceBody {
    expected_workflow_state_version: i32,
    resolution: AssistancePayload,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssistanceQuery {
    before_created_at: Option<String>,
    before_escalated_at: Option<String>,
    before_id: Option<Uuid>,
    limit: Option<u32>,
    status: Option<AssistanceCaseStatus>,
}

fn path_id(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| {
        ApiError::bad_request("invalid_path_parameter", "path identifier must be a UUID")
    })
}

fn cursor(
    query: &AssistanceQuery,
    human_required: bool,
) -> Result<Option<AssistanceCursor>, ApiError> {
    let raw_at = if human_required {
        if query.before_created_at.is_some() {
            return Err(ApiError::unprocessable(
                "invalid_cursor",
                "human-required uses beforeEscalatedAt",
            ));
        }
        query.before_escalated_at.as_ref()
    } else {
        if query.before_escalated_at.is_some() {
            return Err(ApiError::unprocessable(
                "invalid_cursor",
                "this view uses beforeCreatedAt",
            ));
        }
        query.before_created_at.as_ref()
    };
    match (raw_at, query.before_id) {
        (None, None) => Ok(None),
        (Some(at), Some(id)) => {
            let at = chrono::DateTime::parse_from_rfc3339(at)
                .map_err(|_| {
                    ApiError::unprocessable("invalid_cursor", "cursor time must be RFC 3339")
                })?
                .with_timezone(&chrono::Utc);
            Ok(Some(AssistanceCursor { at, id }))
        }
        _ => Err(ApiError::unprocessable(
            "invalid_cursor",
            "cursor time and beforeId must be supplied together",
        )),
    }
}

pub(crate) async fn request(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<RequestAssistanceBody>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    let idempotency_key = idempotency_key(&headers)?;
    let instance_id = path_id(&instance_id)?;
    let Json(payload) = payload.map_err(ApiError::from_json_rejection)?;
    let result = request_assistance(
        &state.pool,
        RequestAssistanceCommand {
            principal_id: principal.principal_id,
            idempotency_key,
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            current_node_visit_id: NodeVisitId::from_uuid(payload.current_node_visit_id),
            expected_workflow_state_version: payload.expected_workflow_state_version,
            request: payload.request,
        },
    )
    .await
    .map_err(ApiError::from_assistance)?;
    let status = if result.replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((status, Json(result)))
}

pub(crate) async fn escalate(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(case_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<EscalateAssistanceBody>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    let idempotency_key = idempotency_key(&headers)?;
    let case_id = path_id(&case_id)?;
    let Json(payload) = payload.map_err(ApiError::from_json_rejection)?;
    let result = escalate_assistance_to_human(
        &state.pool,
        EscalateAssistanceCommand {
            principal_id: principal.principal_id,
            idempotency_key,
            command_schema_version: "v1".to_string(),
            assistance_case_id: AssistanceCaseId::from_uuid(case_id),
            expected_workflow_state_version: payload.expected_workflow_state_version,
            escalation: payload.escalation,
        },
    )
    .await
    .map_err(ApiError::from_assistance)?;
    Ok((StatusCode::OK, Json(result)))
}

pub(crate) async fn resolve(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(case_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<ResolveAssistanceBody>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    let idempotency_key = idempotency_key(&headers)?;
    let case_id = path_id(&case_id)?;
    let Json(payload) = payload.map_err(ApiError::from_json_rejection)?;
    let result = resolve_assistance(
        &state.pool,
        ResolveAssistanceCommand {
            principal_id: principal.principal_id,
            idempotency_key,
            command_schema_version: "v1".to_string(),
            assistance_case_id: AssistanceCaseId::from_uuid(case_id),
            expected_workflow_state_version: payload.expected_workflow_state_version,
            resolution: payload.resolution,
        },
    )
    .await
    .map_err(ApiError::from_assistance)?;
    Ok((StatusCode::OK, Json(result)))
}

async fn list(
    state: AppState,
    principal: AuthenticatedPrincipal,
    query: Result<Query<AssistanceQuery>, QueryRejection>,
    view: AssistanceListView,
) -> Result<impl IntoResponse, ApiError> {
    require_scope(&principal, "workflow.read")?;
    let Query(query) = query.map_err(ApiError::from_query_rejection)?;
    if !matches!(view, AssistanceListView::RequestedByMe) && query.status.is_some() {
        return Err(ApiError::unprocessable(
            "invalid_filter",
            "status is only supported by requested-by-me",
        ));
    }
    let before = cursor(&query, false)?;
    let page = list_assistance(
        &state.pool,
        principal.principal_id.into_uuid(),
        view,
        query.status,
        before,
        query.limit.unwrap_or(50),
    )
    .await
    .map_err(ApiError::from_assistance)?;
    Ok(Json(page))
}

pub(crate) async fn owner_inbox(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    query: Result<Query<AssistanceQuery>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    list(state, principal, query, AssistanceListView::OwnerInbox).await
}

pub(crate) async fn human_required(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    query: Result<Query<AssistanceQuery>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    require_scope(&principal, "workflow.read")?;
    let Query(query) = query.map_err(ApiError::from_query_rejection)?;
    if query.status.is_some() {
        return Err(ApiError::unprocessable(
            "invalid_filter",
            "status is not supported by human-required",
        ));
    }
    let before = cursor(&query, true)?;
    let page = list_human_required_assistance(
        &state.pool,
        principal.principal_id.into_uuid(),
        before,
        query.limit.unwrap_or(50),
    )
    .await
    .map_err(ApiError::from_assistance)?;
    Ok(Json(page))
}

pub(crate) async fn requested_by_me(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    query: Result<Query<AssistanceQuery>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    list(state, principal, query, AssistanceListView::RequestedByMe).await
}

pub(crate) async fn detail(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(case_id): Path<String>,
) -> Result<Response, ApiError> {
    require_scope(&principal, "workflow.read")?;
    let actor = principal.principal_id.into_uuid();
    let case_id = path_id(&case_id)?;
    match get_assistance_case(&state.pool, actor, case_id).await {
        Ok(detail) => Ok(Json(detail).into_response()),
        Err(AssistanceError::AssistanceCaseNotFoundOrNotVisible) => {
            let minimal = get_human_required_assistance_case(&state.pool, actor, case_id)
                .await
                .map_err(ApiError::from_assistance)?;
            Ok(Json(minimal).into_response())
        }
        Err(error) => Err(ApiError::from_assistance(error)),
    }
}
