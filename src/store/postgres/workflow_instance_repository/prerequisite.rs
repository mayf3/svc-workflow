//! Read-time prerequisite derivation; no wait ledger, timer or wake writer.
use super::query_visibility;
use crate::application::workflow_instance::query_types::WorkflowQueryError;
use crate::domain::definition::{
    digest,
    prerequisite::{self as contract, Reference},
};
use crate::domain::workflow_instance::errors::CreateWorkflowInstanceError as CreateError;
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
mod reads;
mod source;
pub use reads::read;
pub(crate) use reads::{project, reader_allowed};
fn storage(e: sqlx::Error) -> WorkflowQueryError {
    WorkflowQueryError::StorageError(e.to_string())
}
fn invalid() -> CreateError {
    CreateError::ContextValidationFailed(
        "prerequisite source unavailable, not visible, or invalid".into(),
    )
}
fn shape(r: &Reference, status: &str, reason: Option<&str>) -> Value {
    let mut v = json!({"status":status,"sourceInstanceId":r.workflow_instance_id,"sourceDefinitionVersionId":r.definition_version_id,"transitionId":r.transition_id});
    if let Some(reason) = reason {
        v["reason"] = json!(reason)
    }
    v
}
/// Revalidated while the source/Domain/version locks are held through commit.
pub(crate) async fn admit(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    domain: Uuid,
    schema: Option<&Value>,
    context: &Value,
) -> Result<Option<Value>, CreateError> {
    let reference =
        contract::parse(schema, context).map_err(CreateError::ContextValidationFailed)?;
    let Some(r) = reference else { return Ok(None) };
    // Same domain guard as Owner replacement. A foreign/missing source is never distinguished.
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM domains WHERE domain_id=$1 FOR SHARE")
            .bind(domain)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| CreateError::StorageError(e.to_string()))?;
    if enabled != Some(true) {
        return Err(invalid());
    }
    if !query_visibility::check_domain_owner(tx, actor, domain)
        .await
        .map_err(|e| CreateError::StorageError(e.to_string()))?
    {
        return Err(CreateError::NotDomainOwner);
    }
    let found:Option<Uuid>=sqlx::query_scalar("SELECT workflow_instance_id FROM workflow_instances WHERE workflow_instance_id=$1 AND domain_id=$2 FOR UPDATE").bind(r.workflow_instance_id).bind(domain).fetch_optional(&mut **tx).await.map_err(|e|CreateError::StorageError(e.to_string()))?;
    if found.is_none() {
        return Err(invalid());
    }
    sqlx::query("SELECT dv.definition_version_id FROM workflow_definition_versions dv JOIN workflow_transition_definitions t USING(definition_version_id) WHERE dv.definition_version_id=$1 AND t.transition_id=$2 FOR SHARE OF dv,t").bind(r.definition_version_id).bind(r.transition_id).fetch_all(&mut **tx).await.map_err(|e|CreateError::StorageError(e.to_string()))?;
    let v = source::evaluate(tx, domain, &r)
        .await
        .map_err(|e| CreateError::StorageError(e.to_string()))?;
    if matches!(v["status"].as_str(), Some("READY" | "WAITING")) {
        Ok(Some(v))
    } else {
        Err(invalid())
    }
}
