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
    /// Semantic model: 1 = Legacy (default), 2 = Minimal, 3 = Visit Activation.
    /// Omitted or 1 -> Legacy; 2/3 keep their explicit model; others are rejected.
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

pub(crate) fn require_direct_token(principal: &AuthenticatedPrincipal) -> Result<(), ApiError> {
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
// GET /internal/v1/domains/{domainId}/definitions/{definitionId}/versions/{versionId}
// ---------------------------------------------------------------------------

/// Closed internal result categories for `get_definition_version_detail`,
/// classified 1:1 from `DefinitionError`. The diagnostic event records WHICH
/// cause folded into the public opaque error; the public response keeps being
/// mapped by `map_definition_error` exactly as before. Exhaustive by
/// construction — no wildcard arm — so a new `DefinitionError` variant fails
/// compilation until it is classified here.
fn definition_version_detail_error_category(e: &DefinitionError) -> &'static str {
    use DefinitionError as E;
    match e {
        E::PrincipalNotFound => "principal_not_found",
        E::PrincipalDisabled => "principal_disabled",
        E::DomainNotFound => "domain_not_found",
        E::DomainDisabled => "domain_disabled",
        E::PermissionDenied => "permission_denied",
        E::DefinitionNotFound => "definition_not_found",
        E::DefinitionVersionNotFound => "definition_version_not_found",
        E::DefinitionKeyConflict => "definition_key_conflict",
        E::VersionNotDraft => "version_not_draft",
        E::InvalidLifecycleTransition => "invalid_lifecycle_transition",
        E::GraphValidationFailed(_) => "graph_validation_failed",
        E::SchemaValidationFailed(_) => "schema_validation_failed",
        E::DefinitionArchived => "definition_archived",
        E::FixedPrincipalInvalid(_) => "fixed_principal_invalid",
        E::DigestFailure(_) => "digest_failure",
        E::ConcurrentModification(_) => "concurrent_modification",
        E::AdmissionFailed(_) => "admission_failed",
        E::StorageError(_) => "storage_error",
    }
}

/// Emit the internal structured diagnostic event for the precise version
/// read. Field list is a closed whitelist: fixed operation, diagnostic
/// version, the correlated request id (the same `x-request-id` value the
/// authentication audit sees; honestly absent when none was set — never
/// fabricated, never used for authorization), the three resolved path UUIDs,
/// the verified principal id, and the closed result category. Tokens,
/// headers, queries, payloads and graph content are never recorded. The
/// event is internal only — status, body and error codes of the HTTP
/// response are unchanged by it.
fn emit_definition_version_detail_diagnostic(
    request_id: Option<&str>,
    domain_id: Uuid,
    definition_id: Uuid,
    version_id: Uuid,
    actor_principal_id: Uuid,
    result_category: &str,
) {
    tracing::info!(
        target: "definition_version_detail",
        operation = "get_definition_version_detail",
        diagnostic_version = 1u32,
        request_id = request_id,
        domain_id = %domain_id,
        definition_id = %definition_id,
        version_id = %version_id,
        actor_principal_id = %actor_principal_id,
        result_category = result_category,
        "definition_version_detail_diagnostic"
    );
}

/// Precise single-version read for the same-domain owner: full graph
/// (nodes/transitions) plus the exact version row. Reuses the existing
/// DefinitionService authorization (H-5 domain owner + enabled principal).
/// Path ownership must be consistent — a version that does not belong to the
/// path definition/domain is reported as intentionally not visible (opaque
/// 404, no existence leak). DRAFT and PUBLISHED versions are both readable
/// by the owner, matching the definition-detail read policy.
pub(crate) async fn get_definition_version_detail(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path((domain_id, definition_id, version_id)): Path<(Uuid, Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_scope(&principal, "workflow.read")?;

    // Correlation only, from the same source the authentication audit reads
    // (the x-request-id header set by SetRequestIdLayer). Absent when the
    // layer set none — never fabricated.
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    let actor_principal_id = principal.principal_id.into_uuid();

    let repo = PgDefinitionRepository::new(state.pool.clone());
    let service = DefinitionService::new(repo);

    let result = match service
        .get_definition_version(GetDefinitionVersion {
            actor_principal_id,
            definition_version_id: version_id,
        })
        .await
    {
        Ok(result) => result,
        Err(e) => {
            emit_definition_version_detail_diagnostic(
                request_id,
                domain_id,
                definition_id,
                version_id,
                actor_principal_id,
                definition_version_detail_error_category(&e),
            );
            return Err(map_definition_error(e, Some(domain_id)));
        }
    };

    let def = &result.version.definition;
    if def.id.into_uuid() != definition_id || def.domain_id.into_uuid() != domain_id {
        emit_definition_version_detail_diagnostic(
            request_id,
            domain_id,
            definition_id,
            version_id,
            actor_principal_id,
            "path_binding_mismatch",
        );
        return Err(ApiError::new(
            axum::http::StatusCode::NOT_FOUND,
            "definition_not_found",
            "workflow definition not found",
        ));
    }

    emit_definition_version_detail_diagnostic(
        request_id,
        domain_id,
        definition_id,
        version_id,
        actor_principal_id,
        "success",
    );

    Ok(Json(serde_json::json!({
        "definition": result.version.definition,
        "version": result.version.version,
        "nodes": result.version.nodes,
        "transitions": result.version.transitions,
        "nodes_count": result.nodes_count,
        "transitions_count": result.transitions_count,
    })))
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
        Some(3) => 3,
        Some(other) => {
            return Err(ApiError::unprocessable(
                "invalid_semantic_model_version",
                "semanticModelVersion must be 1 (Legacy), 2 (Minimal), or 3 (Visit Activation)",
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

    // Canonical identity admission gate (CTR-CIR-003): `Some` only when the
    // deployment enabled admission; dormant mode keeps existing behavior.
    // The monotonic admission start is captured here, before any transaction.
    let admission = crate::store::postgres::admission_gate::AdmissionGate::new(
        state.admission_client.as_ref(),
    )
    .with_pool(Some(&state.pool));

    let result = governance_publish_version(
        &state.pool,
        principal.principal_id.into_uuid(),
        &key,
        request_id,
        payload.version_id,
        payload.expected_revision,
        admission,
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
        E::AdmissionFailed(error) => {
            // Sanitized `admission_*` family (CTR-CIR-003); never surfaces
            // internal endpoint/body details.
            let status = axum::http::StatusCode::from_u16(error.sanitized_status() as u16)
                .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
            ApiError::new(status, error.sanitized_code(), "definition publish was not admitted")
        }
        E::StorageError(detail) => {
            tracing::error!(error = %detail, "definition storage error");
            ApiError::service_unavailable("service_unavailable", "storage is unavailable")
        }
    }
}

// ---------------------------------------------------------------------------
// Tests: internal diagnostics for the precise version read
// ---------------------------------------------------------------------------

#[cfg(test)]
mod definition_version_detail_diagnostics_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Minimal in-memory writer so emitted events can be asserted without any
    /// real log destination. tracing-subscriber is already a dependency;
    /// nothing new is installed and nothing is written outside the test.
    #[derive(Clone, Default)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("shared buffer lock")
                .extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn capture<F: FnOnce()>(emit: F) -> String {
        let shared = SharedBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer({
                let shared = shared.clone();
                move || shared.clone()
            })
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, emit);
        let captured = shared.0.lock().expect("shared buffer lock").clone();
        String::from_utf8(captured).expect("captured diagnostics are utf-8")
    }

    // Synthetic UUIDs only — never real business coordinates.
    fn synthetic() -> (Uuid, Uuid, Uuid, Uuid) {
        (
            Uuid::from_u128(0x0001_0000_0000_0000_0000_0000_0000_0001),
            Uuid::from_u128(0x0002_0000_0000_0000_0000_0000_0000_0002),
            Uuid::from_u128(0x0003_0000_0000_0000_0000_0000_0000_0003),
            Uuid::from_u128(0x0004_0000_0000_0000_0000_0000_0000_0004),
        )
    }

    #[test]
    fn error_category_mapping_is_exhaustive_and_closed() {
        use crate::auth::admission::AdmissionError;
        use DefinitionError as E;
        let cases: Vec<(E, &str)> = vec![
            (E::PrincipalNotFound, "principal_not_found"),
            (E::PrincipalDisabled, "principal_disabled"),
            (E::DomainNotFound, "domain_not_found"),
            (E::DomainDisabled, "domain_disabled"),
            (E::PermissionDenied, "permission_denied"),
            (E::DefinitionNotFound, "definition_not_found"),
            (E::DefinitionVersionNotFound, "definition_version_not_found"),
            (E::DefinitionKeyConflict, "definition_key_conflict"),
            (E::VersionNotDraft, "version_not_draft"),
            (
                E::InvalidLifecycleTransition,
                "invalid_lifecycle_transition",
            ),
            (E::GraphValidationFailed(vec![]), "graph_validation_failed"),
            (
                E::SchemaValidationFailed("s".into()),
                "schema_validation_failed",
            ),
            (E::DefinitionArchived, "definition_archived"),
            (
                E::FixedPrincipalInvalid("f".into()),
                "fixed_principal_invalid",
            ),
            (E::DigestFailure("d".into()), "digest_failure"),
            (
                E::ConcurrentModification("c".into()),
                "concurrent_modification",
            ),
            (
                E::AdmissionFailed(AdmissionError::Timeout),
                "admission_failed",
            ),
            (E::StorageError("x".into()), "storage_error"),
        ];
        for (error, expected) in cases {
            assert_eq!(definition_version_detail_error_category(&error), expected);
            assert!(!expected.is_empty());
            assert_eq!(expected.to_lowercase(), expected);
        }
    }

    #[test]
    fn event_fields_are_the_closed_whitelist_and_no_secret_channel_leaks() {
        let (dom, def, ver, actor) = synthetic();
        let line = capture(|| {
            emit_definition_version_detail_diagnostic(
                Some("req-whitelist-1"),
                dom,
                def,
                ver,
                actor,
                "path_binding_mismatch",
            )
        });
        for expected in [
            "definition_version_detail_diagnostic",
            "get_definition_version_detail",
            "diagnostic_version=1",
            "req-whitelist-1",
            &dom.to_string(),
            &def.to_string(),
            &ver.to_string(),
            &actor.to_string(),
            "path_binding_mismatch",
        ] {
            assert!(
                line.contains(expected),
                "missing field/value {expected:?} in: {line}"
            );
        }
        // No token, header, query, payload or graph channel may leak.
        for forbidden in [
            "authorization",
            "bearer",
            "cookie",
            "jti",
            "token",
            "payload",
            "query",
            "nodes",
            "transitions",
            "context_schema",
            "submission_schema",
            "instructions",
            "assignee",
            "metadata",
        ] {
            assert!(
                !line.to_lowercase().contains(forbidden),
                "forbidden token {forbidden:?} leaked into: {line}"
            );
        }
    }

    #[test]
    fn missing_request_id_is_omitted_not_fabricated() {
        let (dom, def, ver, actor) = synthetic();
        let line = capture(|| {
            emit_definition_version_detail_diagnostic(None, dom, def, ver, actor, "success")
        });
        // tracing renders an Option::None field as fully absent — the event
        // carries no request id at all, never a fabricated placeholder.
        assert!(!line.contains("request_id"), "request id leaked in: {line}");
        assert!(line.contains("result_category=\"success\""));
    }

    #[test]
    fn success_and_failure_are_correlatable_by_request_id() {
        let (dom, def, ver, actor) = synthetic();
        let lines = capture(|| {
            emit_definition_version_detail_diagnostic(
                Some("req-corr-9"),
                dom,
                def,
                ver,
                actor,
                "success",
            );
            emit_definition_version_detail_diagnostic(
                Some("req-corr-9"),
                dom,
                def,
                ver,
                actor,
                "definition_version_not_found",
            );
        });
        assert_eq!(lines.matches("req-corr-9").count(), 2);
        assert!(lines.contains("result_category=\"success\""));
        assert!(lines.contains("result_category=\"definition_version_not_found\""));
    }
}
