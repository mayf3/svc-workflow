use sqlx::PgPool;

use crate::domain::ids::DefinitionVersionId;
use crate::domain::provisioning::ProvisioningError;
use crate::store::postgres::provisioning_repository;

pub async fn get_definition_version(
    pool: &PgPool,
    version_id: DefinitionVersionId,
) -> Result<serde_json::Value, ProvisioningError> {
    let uuid = version_id.into_uuid();
    let summary = provisioning_repository::get_definition_version_summary(pool, uuid).await?;
    match summary {
        Some(summary) => {
            let (nodes, transitions) =
                provisioning_repository::get_definition_graph_counts(pool, uuid).await?;
            Ok(serde_json::json!({
                "definitionVersionId": summary.definition_version_id,
                "workflowDefinitionId": summary.workflow_definition_id,
                "domainId": summary.domain_id,
                "definitionKey": summary.definition_key,
                "versionNumber": summary.version_number,
                "versionStatus": summary.version_status,
                "digest": summary.digest,
                "nodeCount": nodes,
                "transitionCount": transitions,
                "domainEnabled": summary.domain_enabled,
                "canCreateInstances":
                    summary.version_status == "PUBLISHED" && summary.domain_enabled,
            }))
        }
        None => Err(ProvisioningError::DefinitionVersionNotFound),
    }
}
