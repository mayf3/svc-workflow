//! Narrow authenticated Owner-obligation wire; business receipts stay unwrapped.
use crate::{
    auth::AuthenticatedPrincipal,
    http::{error::ApiError, AppState},
    store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::{self as owner, OwnerError},
    },
};
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;
fn error(e: OwnerError) -> Response {
    (StatusCode::from_u16(e.status).unwrap_or(StatusCode::SERVICE_UNAVAILABLE),Json(serde_json::json!({"error":{"code":e.code.to_ascii_lowercase(),"message":"Owner obligation request refused"}}))).into_response()
}
fn outcome(result: Result<serde_json::Value, OwnerError>) -> Response {
    match result {
        Ok(body) => Json(body).into_response(),
        Err(e) => error(e),
    }
}
pub(crate) async fn read(
    State(s): State<AppState>,
    p: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Response {
    if let Err(e) = super::require_scope(&p, "workflow.read") {
        return e.into_response();
    }
    outcome(
        owner::reads::managed(
            &s.pool,
            p.principal_id.into_uuid(),
            id,
            s.workflow_policy_actor_principal_id,
        )
        .await,
    )
}
pub(crate) async fn read_effect(
    State(s): State<AppState>,
    p: AuthenticatedPrincipal,
    Path((id, effect)): Path<(Uuid, Uuid)>,
) -> Response {
    if let Err(e) = super::require_scope(&p, "workflow.read") {
        return e.into_response();
    }
    outcome(owner::reads::effect(&s.pool, p.principal_id.into_uuid(), id, effect).await)
}
pub(crate) async fn execute(
    State(s): State<AppState>,
    p: AuthenticatedPrincipal,
    Path((id, visit, slot)): Path<(Uuid, Uuid, String)>,
    payload: Result<Json<owner::effects::Execute>, JsonRejection>,
) -> Response {
    if let Err(e) = super::require_scope(&p, "workflow.execute") {
        return e.into_response();
    }
    let Some(slot) = slot.strip_suffix(":execute").filter(|s| !s.is_empty()) else {
        return error(OwnerError::new(404, "not_found"));
    };
    let body = match payload {
        Ok(Json(body)) => body,
        Err(e) => return ApiError::from_json_rejection(e).into_response(),
    };
    outcome(
        owner::effects::execute(
            &s.pool,
            p.principal_id.into_uuid(),
            s.workflow_policy_actor_principal_id,
            AdmissionGate::new(s.admission_client.as_ref()).with_pool(Some(&s.pool)),
            id,
            visit,
            slot,
            body,
        )
        .await,
    )
}
