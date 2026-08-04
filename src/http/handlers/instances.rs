//! Create and detail endpoints.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;

use crate::application::workflow_instance::create::create_workflow_instance;
use crate::application::workflow_instance::query_types::{
    GetWorkflowInstanceDetail, ListDomainInstances, StatusFilter, TimeUuidCursor,
};
use crate::auth::AuthenticatedPrincipal;
use crate::domain::ids::{DefinitionVersionId, DomainId, WorkflowInstanceId};
use crate::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use crate::http::dto::{
    detail_response, CreateWorkflowInstanceRequest, CreateWorkflowInstanceResponse,
    DomainInstanceQuery,
};
use crate::http::error::ApiError;
use crate::http::AppState;

use super::{idempotency_key, path_uuid, require_scope};

pub(crate) async fn create(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: HeaderMap,
    payload: Result<Json<CreateWorkflowInstanceRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    let key = idempotency_key(&headers)?;
    let Json(payload) = payload.map_err(ApiError::from_json_rejection)?;
    if payload
        .external_reference
        .as_ref()
        .is_some_and(|value| value.chars().count() > 512)
    {
        return Err(ApiError::unprocessable(
            "invalid_input",
            "externalReference must not exceed 512 characters",
        ));
    }
    let command = CreateWorkflowInstanceCommand {
        principal_id: principal.principal_id,
        idempotency_key: key,
        command_schema_version: "v1".to_string(),
        domain_id: DomainId::from_uuid(payload.domain_id),
        definition_version_id: DefinitionVersionId::from_uuid(payload.definition_version_id),
        external_reference: payload.external_reference,
        external_url: payload.external_url,
        metadata: payload.metadata,
        context_payload: payload.context_payload,
    };
    let result = create_workflow_instance(&state.pool, command)
        .await
        .map_err(ApiError::from_create)?;
    let response = CreateWorkflowInstanceResponse::from(result);
    let location = format!(
        "/internal/v1/workflow-instances/{}",
        response.workflow_instance_id
    );
    let location = HeaderValue::from_str(&location).map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_consistency_error",
            "failed to construct response location",
        )
    })?;
    Ok((
        StatusCode::CREATED,
        [("location", location)],
        Json(response),
    ))
}

pub(crate) async fn detail(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(workflow_instance_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    require_scope(&principal, "workflow.read")?;
    let workflow_instance_id = path_uuid(&workflow_instance_id)?;
    let detail = state
        .query_service
        .get_workflow_instance_detail(GetWorkflowInstanceDetail {
            actor_principal_id: principal.principal_id.into_uuid(),
            workflow_instance_id,
        })
        .await
        .map_err(ApiError::from_query)?;
    Ok(Json(detail_response(detail)))
}

/// GET /internal/v1/workflow-instances/domain
///
/// Returns a paginated, filtered list of all instances in a domain.
/// Only callable by principals with the DOMAIN_OWNER role for the
/// specified domain.
pub(crate) async fn domain_list(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    query: Result<Query<DomainInstanceQuery>, QueryRejection>,
) -> Result<
    Json<
        crate::application::workflow_instance::query_types::Page<
            crate::application::workflow_instance::query_types::DomainInstanceSummary,
        >,
    >,
    ApiError,
> {
    require_scope(&principal, "workflow.read")?;
    let Query(query) = query.map_err(ApiError::from_query_rejection)?;

    // Validate lifecycle parameter (422 for invalid values)
    let lifecycle = query
        .parse_lifecycle()
        .map_err(|(code, msg)| ApiError::unprocessable(code, msg))?;

    // Validate status parameter (422 for invalid values)
    let status_explicit = query
        .parse_status()
        .map_err(|(code, msg)| ApiError::unprocessable(code, msg))?;

    // Resolve default status:
    // - status explicitly provided → use it
    // - status omitted, lifecycle provided → status=all (keep existing
    //   lifecycle callers' results unchanged)
    // - both omitted → status=active (new default: hide cancelled/archived)
    let status = status_explicit.unwrap_or(match lifecycle {
        Some(_) => StatusFilter::All,
        None => StatusFilter::Active,
    });

    // Parse cursor
    let before = parse_domain_cursor(query.before_created_at, query.before_id)?;

    let result = state
        .query_service
        .list_domain_instances(ListDomainInstances {
            actor_principal_id: principal.principal_id.into_uuid(),
            domain_id: query.domain_id,
            before,
            limit: query.limit,
            definition_key: query.definition_key,
            lifecycle,
            current_node_key: query.current_node_key,
            assignee_principal_id: query.assignee_principal_id,
            status,
        })
        .await
        .map_err(ApiError::from_query)?;

    Ok(Json(result))
}

/// Parse the composite cursor for domain instance pagination.
///
/// Both `beforeCreatedAt` and `beforeId` must be present together, or both
/// absent. Invalid or malformed values produce a 422 Unprocessable Entity.
fn parse_domain_cursor(
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
