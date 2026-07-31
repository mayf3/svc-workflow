//! Axum adapter for the internal workflow API.
//!
//! Auth V1 is the only authentication path.  A write gate middleware
//! blocks write endpoints when `AUTH_V1_CANARY_WRITE_ENABLED` is false.

pub mod canary_guard;
pub mod dto;
pub mod error;
mod handlers;
mod state;

use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, StatusCode};
use axum::middleware;
use axum::routing::{delete, get, post, put};
use axum::{BoxError, Router};
use tower::ServiceBuilder;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

pub use state::{AppState, HttpConfig};

pub const API_CONTRACT_VERSION: &str = "internal-v0";
pub const SERVICE_VERSION: &str = "0.3.1";
pub const SCHEMA_VERSION: &str = "0014";
pub const EXPECTED_MIGRATION_VERSION: i64 = 14;

pub fn router(state: AppState, config: &HttpConfig) -> Router {
    let request_id = HeaderName::from_static("x-request-id");
    Router::new()
        .route("/healthz", get(handlers::health::healthz))
        .route("/readyz", get(handlers::health::readyz))
        .route("/version", get(handlers::health::version))
        .route(
            "/internal/v1/workflow-instances",
            post(handlers::instances::create).layer(middleware::from_fn_with_state(
                state.clone(),
                canary_guard::canary_write_guard,
            )),
        )
        .route(
            "/internal/v1/workflow-instances/{workflowInstanceId}",
            get(handlers::instances::detail),
        )
        .route(
            "/internal/v1/workflow-instances/{workflowInstanceId}/transitions",
            post(handlers::transitions::execute).layer(middleware::from_fn_with_state(
                state.clone(),
                canary_guard::canary_write_guard,
            )),
        )
        .route(
            "/internal/v1/workflow-instances/{workflowInstanceId}/timeline",
            get(handlers::timeline::list),
        )
        .route(
            "/internal/v1/workflow-instances/{workflowInstanceId}/submissions",
            get(handlers::submissions::list),
        )
        .route(
            "/internal/v1/workflow-instances/domain",
            get(handlers::instances::domain_list),
        )
        .route(
            "/internal/v1/worklists/assigned-to-me",
            get(handlers::worklists::assigned_to_me),
        )
        .route(
            "/internal/v1/worklists/creator-owned-drafts",
            get(handlers::worklists::creator_owned_drafts),
        )
        // Self-projection
        .route(
            "/internal/v1/principals/me",
            put(handlers::self_projection::self_project_handler),
        )
        // Domain member management
        .route(
            "/internal/v1/domains/{domainId}/members",
            get(handlers::domain_members::list_members),
        )
        .route(
            "/internal/v1/domains/{domainId}/members/{principalId}",
            put(handlers::domain_members::add_member),
        )
        .route(
            "/internal/v1/domains/{domainId}/members/{principalId}",
            delete(handlers::domain_members::remove_member),
        )
        // Domain Owner Definition management
        .route(
            "/internal/v1/domains/{domainId}/definitions",
            get(handlers::definitions::list_definitions),
        )
        .route(
            "/internal/v1/domains/{domainId}/definitions/{definitionId}",
            get(handlers::definitions::get_definition_detail),
        )
        .route(
            "/internal/v1/domains/{domainId}/definitions",
            post(handlers::definitions::create_definition).layer(middleware::from_fn_with_state(
                state.clone(),
                canary_guard::canary_write_guard,
            )),
        )
        .route(
            "/internal/v1/domains/{domainId}/definitions/{definitionId}/versions",
            post(handlers::definitions::create_draft_version).layer(
                middleware::from_fn_with_state(state.clone(), canary_guard::canary_write_guard),
            ),
        )
        .route(
            "/internal/v1/domains/{domainId}/definitions/{definitionId}/draft",
            put(handlers::definitions::replace_draft_graph).layer(middleware::from_fn_with_state(
                state.clone(),
                canary_guard::canary_write_guard,
            )),
        )
        .route(
            "/internal/v1/domains/{domainId}/definitions/{definitionId}/publish",
            post(handlers::definitions::publish_version).layer(middleware::from_fn_with_state(
                state.clone(),
                canary_guard::canary_write_guard,
            )),
        )
        .route(
            "/internal/v1/domains/{domainId}/definitions/{definitionId}/archive",
            post(handlers::definitions::archive_definition).layer(middleware::from_fn_with_state(
                state.clone(),
                canary_guard::canary_write_guard,
            )),
        )
        // Provisioning endpoints
        .route(
            "/internal/v1/admin/principals",
            post(handlers::provisioning::principals::create),
        )
        .route(
            "/internal/v1/admin/principals/{principalId}",
            get(handlers::provisioning::principals::get),
        )
        .route(
            "/internal/v1/admin/domains",
            post(handlers::provisioning::domains::create),
        )
        .route(
            "/internal/v1/admin/domains/{domainId}",
            get(handlers::provisioning::domains::get),
        )
        .route(
            "/internal/v1/admin/domains/{domainId}/role-bindings/{principalId}",
            put(handlers::provisioning::role_bindings::create),
        )
        .route(
            "/internal/v1/admin/domains/{domainId}/role-bindings/{principalId}",
            delete(handlers::provisioning::role_bindings::delete),
        )
        .route(
            "/internal/v1/admin/domains/{domainId}/owner",
            put(handlers::provisioning::role_bindings::replace_domain_owner),
        )
        .route(
            "/internal/v1/admin/definition-versions/{definitionVersionId}",
            get(handlers::provisioning::definitions::get),
        )
        .fallback(|| async {
            error::ApiError::new(StatusCode::NOT_FOUND, "route_not_found", "route not found")
        })
        .method_not_allowed_fallback(|| async {
            error::ApiError::new(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "method not allowed",
            )
        })
        .layer(PropagateRequestIdLayer::new(request_id.clone()))
        .layer(SetRequestIdLayer::new(request_id, MakeRequestUuid))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_service_error))
                .timeout(Duration::from_secs(config.request_timeout_seconds)),
        )
        .layer(DefaultBodyLimit::max(config.request_body_max_bytes))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn handle_service_error(error: BoxError) -> error::ApiError {
    if error.is::<tower::timeout::error::Elapsed>() {
        error::ApiError::new(
            StatusCode::REQUEST_TIMEOUT,
            "request_timeout",
            "request timed out",
        )
    } else {
        tracing::error!(error = %error, "unhandled HTTP service error");
        error::ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "internal service error",
        )
    }
}
