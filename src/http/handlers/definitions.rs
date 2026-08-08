//! Domain Owner definition management handlers.
//!
//! These handlers let a verified DOMAIN_OWNER manage workflow definitions
//! within their domain.  Write endpoints require a Direct Machine Token
//! (`token_use=access`, no delegation).  Read endpoints accept both Direct
//! and OBO tokens (matching existing definition read contract).

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::application::definition::commands::{RawNodeDefinition, RawTransitionDefinition};
use crate::application::definition::queries::{
    GetCompleteVersionGraph, GetDefinition, GetDefinitionVersion, ListDefinitionVersions,
    ListDomainDefinitions,
};
use crate::application::definition::DefinitionService;
use crate::application::definition_governance::{
    governance_archive_definition, governance_create_definition, governance_create_draft_version,
    governance_publish_version, governance_replace_draft_graph,
};
use crate::auth::AuthenticatedPrincipal;
use crate::domain::definition::error::DefinitionError;
use crate::http::error::ApiError;
use crate::http::AppState;
use crate::store::postgres::definition_repository::PgDefinitionRepository;

use super::{idempotency_key, require_scope};

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateDefinitionBody {
    pub definition_key: String,
    pub display_name: String,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateDraftVersionBody {
    pub context_schema: Option<serde_json::Value>,
    pub json_schema_dialect: Option<String>,
    pub validator_version: Option<String>,
    pub metadata: Option<serde_json::Value>,
    /// Semantic model version: 1 = Legacy (default), 2 = Minimal.
    /// Omitted or 1 -> Legacy; 2 -> Minimal; any other value is rejected.
    pub semantic_model_version: Option<i16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReplaceDraftGraphBody {
    pub definition_version_id: Uuid,
    pub context_schema: Option<serde_json::Value>,
    pub nodes: Vec<RawNodeDefinition>,
    pub transitions: Vec<RawTransitionDefinition>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PublishVersionBody {
    pub version_id: Uuid,
    pub expected_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DefinitionListQuery {
    pub before_created_at: Option<String>,
    pub before_id: Option<Uuid>,
    pub limit: Option<u32>,
    pub include_archived: Option<bool>,
}

// ---------------------------------------------------------------------------
// Direct-token gate (writes only)
// ---------------------------------------------------------------------------

fn require_direct_token(principal: &AuthenticatedPrincipal) -> Result<(), ApiError> {
    if principal.auth_context.token_use != "access"
        || principal.auth_context.delegating_principal_id.is_some()
    {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "direct_token_required",
            "only direct access tokens may manage workflow definitions",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// GET /internal/v1/domains/{domainId}/definitions
// ---------------------------------------------------------------------------

pub(crate) async fn list_definitions(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(domain_id): Path<Uuid>,
    query: Result<Query<DefinitionListQuery>, QueryRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;

    let Query(query) = query.map_err(ApiError::from_query_rejection)?;
    let limit = query.limit.unwrap_or(20).min(100);
    let include_archived = query.include_archived.unwrap_or(false);

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

    let repo = PgDefinitionRepository::new(state.pool.clone());
    let service = DefinitionService::new(repo);

    let result = service
        .list_domain_definitions(ListDomainDefinitions {
            actor_principal_id: principal.principal_id.into_uuid(),
            domain_id,
            before_created_at,
            before_id: query.before_id,
            limit,
            include_archived,
        })
        .await
        .map_err(|e| map_definition_error(e, Some(domain_id)))?;

    let response = serde_json::json!({
        "items": result.definitions,
        "next_cursor": result.next_cursor.map(|(ts, id)| {
            serde_json::json!({
                "created_at": ts.to_rfc3339(),
                "id": id,
            })
        }),
    });

    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// GET /internal/v1/domains/{domainId}/definitions/{definitionId}
// ---------------------------------------------------------------------------

pub(crate) async fn get_definition_detail(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path((domain_id, definition_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;

    let repo = PgDefinitionRepository::new(state.pool.clone());
    let service = DefinitionService::new(repo);

    let def_result = service
        .get_definition(GetDefinition {
            actor_principal_id: principal.principal_id.into_uuid(),
            workflow_definition_id: definition_id,
        })
        .await
        .map_err(|e| map_definition_error(e, Some(domain_id)))?;

    let version_result = service
        .list_definition_versions(ListDefinitionVersions {
            actor_principal_id: principal.principal_id.into_uuid(),
            workflow_definition_id: definition_id,
        })
        .await
        .map_err(|e| map_definition_error(e, Some(domain_id)))?;

    let response = serde_json::json!({
        "definition": def_result.definition.definition,
        "versions": version_result.versions,
    });

    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// POST /internal/v1/domains/{domainId}/definitions
// ---------------------------------------------------------------------------

pub(crate) async fn create_definition(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: HeaderMap,
    Path(domain_id): Path<Uuid>,
    payload: Result<Json<CreateDefinitionBody>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    let key = idempotency_key(&headers)?;
    let Json(payload) = payload.map_err(ApiError::from_json_rejection)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let result = governance_create_definition(
        &state.pool,
        principal.principal_id.into_uuid(),
        &key,
        request_id,
        domain_id,
        &payload.definition_key,
        &payload.display_name,
        payload.description.as_deref(),
        payload.metadata.as_ref(),
    )
    .await
    .map_err(ApiError::from_definition_governance)?;

    Ok(Json(serde_json::json!({
        "workflowDefinitionId": result.id,
        "domainId": result.domain_id,
        "definitionKey": result.definition_key,
        "displayName": result.display_name,
        "createdAt": result.created_at,
    })))
}

// ---------------------------------------------------------------------------
// POST /internal/v1/domains/{domainId}/definitions/{definitionId}/versions
// ---------------------------------------------------------------------------

pub(crate) async fn create_draft_version(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: HeaderMap,
    Path((_domain_id, definition_id)): Path<(Uuid, Uuid)>,
    payload: Result<Json<CreateDraftVersionBody>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    let key = idempotency_key(&headers)?;
    let Json(payload) = payload.map_err(ApiError::from_json_rejection)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let semantic_model_version = match payload.semantic_model_version {
        None | Some(1) => 1,
        Some(2) => 2,
        Some(other) => {
            return Err(ApiError::unprocessable(
                "invalid_semantic_model_version",
                "semanticModelVersion must be 1 (Legacy) or 2 (Minimal)",
            )
            .with_details(serde_json::json!({ "provided": other })));
        }
    };

    let result = governance_create_draft_version(
        &state.pool,
        principal.principal_id.into_uuid(),
        &key,
        request_id,
        definition_id,
        payload.context_schema,
        payload.json_schema_dialect,
        payload.validator_version,
        payload.metadata,
        semantic_model_version,
    )
    .await
    .map_err(ApiError::from_definition_governance)?;

    Ok(Json(serde_json::json!({
        "definitionVersionId": result.id,
        "workflowDefinitionId": result.workflow_definition_id,
        "versionNumber": result.version_number,
        "versionStatus": result.version_status,
        "createdAt": result.created_at,
    })))
}

// ---------------------------------------------------------------------------
// PUT /internal/v1/domains/{domainId}/definitions/{definitionId}/draft
// ---------------------------------------------------------------------------

pub(crate) async fn replace_draft_graph(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: HeaderMap,
    Path((_domain_id, _definition_id)): Path<(Uuid, Uuid)>,
    payload: Result<Json<ReplaceDraftGraphBody>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    let key = idempotency_key(&headers)?;
    let Json(payload) = payload.map_err(ApiError::from_json_rejection)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    governance_replace_draft_graph(
        &state.pool,
        principal.principal_id.into_uuid(),
        &key,
        request_id,
        payload.definition_version_id,
        payload.context_schema,
        payload.nodes,
        payload.transitions,
    )
    .await
    .map_err(ApiError::from_definition_governance)?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

// ---------------------------------------------------------------------------
// POST /internal/v1/domains/{domainId}/definitions/{definitionId}/publish
// ---------------------------------------------------------------------------

pub(crate) async fn publish_version(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: HeaderMap,
    Path((_domain_id, _definition_id)): Path<(Uuid, Uuid)>,
    payload: Result<Json<PublishVersionBody>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    let key = idempotency_key(&headers)?;
    let Json(payload) = payload.map_err(ApiError::from_json_rejection)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let result = governance_publish_version(
        &state.pool,
        principal.principal_id.into_uuid(),
        &key,
        request_id,
        payload.version_id,
        payload.expected_revision,
    )
    .await
    .map_err(ApiError::from_definition_governance)?;

    Ok(Json(serde_json::json!({
        "definitionVersionId": result.id,
        "versionNumber": result.version_number,
        "versionStatus": result.version_status,
        "digest": result.definition_digest,
        "publishedAt": result.published_at,
    })))
}

// ---------------------------------------------------------------------------
// POST /internal/v1/domains/{domainId}/definitions/{definitionId}/archive
// ---------------------------------------------------------------------------

pub(crate) async fn archive_definition(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: HeaderMap,
    Path((_domain_id, definition_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.execute")?;
    require_direct_token(&principal)?;
    let key = idempotency_key(&headers)?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let result = governance_archive_definition(
        &state.pool,
        principal.principal_id.into_uuid(),
        &key,
        request_id,
        definition_id,
    )
    .await
    .map_err(ApiError::from_definition_governance)?;

    Ok(Json(serde_json::json!({
        "workflowDefinitionId": result.id,
        "definitionKey": result.definition_key,
        "archived": result.archived,
        "archivedAt": result.archived_at,
    })))
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

fn map_definition_error(e: DefinitionError, _domain_id: Option<Uuid>) -> ApiError {
    use DefinitionError as E;
    match e {
        // Cross-domain existence leak prevention:
        // Any permission or existence error is opaque 404 definition_not_found
        // so callers cannot distinguish "definition exists but not yours"
        // from "definition does not exist".
        E::PermissionDenied
        | E::PrincipalNotFound
        | E::PrincipalDisabled
        | E::DomainNotFound
        | E::DefinitionNotFound
        | E::DefinitionVersionNotFound => ApiError::new(
            axum::http::StatusCode::NOT_FOUND,
            "definition_not_found",
            "workflow definition not found",
        ),
        E::DomainDisabled => ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "domain_disabled",
            "domain is disabled",
        ),
        E::DefinitionArchived => ApiError::new(
            axum::http::StatusCode::CONFLICT,
            "definition_not_editable",
            "workflow definition is archived",
        ),
        E::VersionNotDraft => ApiError::new(
            axum::http::StatusCode::CONFLICT,
            "definition_version_immutable",
            "definition version is not in DRAFT status",
        ),
        E::DefinitionKeyConflict => ApiError::new(
            axum::http::StatusCode::CONFLICT,
            "definition_key_conflict",
            "definition key already exists in this domain",
        ),
        E::ConcurrentModification(detail) => ApiError::new(
            axum::http::StatusCode::CONFLICT,
            "revision_conflict",
            "concurrent modification detected",
        )
        .with_details(serde_json::json!({"detail": detail})),
        E::GraphValidationFailed(errors) => {
            ApiError::unprocessable("graph_validation_failed", "graph validation failed")
                .with_details(serde_json::json!({"errors": errors}))
        }
        E::SchemaValidationFailed(detail) => {
            ApiError::unprocessable("schema_validation_failed", "schema validation failed")
                .with_details(serde_json::json!({"detail": detail}))
        }
        E::FixedPrincipalInvalid(detail) => ApiError::unprocessable(
            "fixed_principal_invalid",
            "fixed principal reference is invalid",
        )
        .with_details(serde_json::json!({"detail": detail})),
        E::DigestFailure(detail) => {
            ApiError::unprocessable("digest_failure", "digest computation failed")
                .with_details(serde_json::json!({"detail": detail}))
        }
        E::InvalidLifecycleTransition => ApiError::new(
            axum::http::StatusCode::CONFLICT,
            "invalid_lifecycle_transition",
            "invalid lifecycle status transition",
        ),
        E::StorageError(detail) => {
            tracing::error!(error = %detail, "definition storage error");
            ApiError::service_unavailable("service_unavailable", "storage is unavailable")
        }
    }
}
