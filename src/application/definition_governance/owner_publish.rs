//! Publication's Version, optional policy approval and original receipt share one commit.
use super::*;
use sqlx::Acquire;
fn map(e: impl std::fmt::Display) -> DefinitionGovernanceError {
    DefinitionGovernanceError::StorageError(e.to_string())
}
#[allow(clippy::too_many_arguments)]
pub async fn governance_publish_version_with_actor(
    pool: &PgPool,
    actor_id: Uuid,
    key: &str,
    _request: &str,
    version: Uuid,
    expected_revision: Option<String>,
    admission: AdmissionGate<'_>,
    policy_actor: Option<Uuid>,
) -> Result<WorkflowDefinitionVersion, DefinitionGovernanceError> {
    let hash = compute_receipt_hash(
        &serde_json::json!({"commandType":COMMAND_TYPE_PUBLISH,"command":{"definitionVersionId":version,"expectedRevision":expected_revision}}),
    );
    let mut tx = pool.begin().await.map_err(map)?;
    let receipt = acquire_receipt(&mut tx, actor_id, key, COMMAND_TYPE_PUBLISH, &hash)
        .await
        .map_err(map)?;
    if !receipt.is_owned() {
        tx.commit().await.map_err(map)?;
        return handle_receipt_result(receipt, &hash);
    }
    let command = receipt.command_id();
    let service = DefinitionService::new(PgDefinitionRepository::new(pool.clone()));
    let cmd = PublishVersion {
        actor_principal_id: actor_id,
        definition_version_id: version,
        expected_revision: expected_revision.clone(),
    };
    let mut facts = tx.begin().await.map_err(map)?;
    let result = async {
        let digest = service.prepare_publish(&cmd, admission).await?;
        let result = service
            .repo
            .publish_on_tx(
                &mut facts,
                version,
                actor_id,
                &digest,
                expected_revision.as_deref(),
                admission,
                Some(command),
                policy_actor,
            )
            .await?;
        admission
            .check_commit_budget()
            .map_err(DefinitionError::AdmissionFailed)?;
        Ok::<_, DefinitionError>(result)
    }
    .await;
    match result {
        Ok(value) => {
            facts.commit().await.map_err(map)?;
            let body = serde_json::to_value(&value)
                .map_err(|e| DefinitionGovernanceError::InternalConsistency(e.to_string()))?;
            write_security_audit(&mut tx,actor_id,COMMAND_TYPE_PUBLISH,actor_id,Uuid::nil(),&serde_json::json!({"operation":COMMAND_TYPE_PUBLISH,"actorPrincipalId":actor_id,"result":"success"})).await.map_err(map)?;
            complete_receipt(&mut tx, command, 200, &body)
                .await
                .map_err(map)?;
            tx.commit().await.map_err(map)?;
            Ok(value)
        }
        Err(e) => {
            facts.rollback().await.map_err(map)?;
            let e: DefinitionGovernanceError = e.into();
            if matches!(
                e,
                DefinitionGovernanceError::AdmissionFailed(_)
                    | DefinitionGovernanceError::StorageError(_)
                    | DefinitionGovernanceError::InternalConsistency(_)
            ) {
                return Err(e);
            }
            let body = match &e {
                DefinitionGovernanceError::GraphValidationFailed(details) => {
                    serde_json::json!({"error":e.label(),"details":details,"graphInputHash":hash})
                }
                _ => serde_json::json!({"error":e.label()}),
            };
            complete_receipt(&mut tx, command, e.status_code() as i32, &body)
                .await
                .map_err(map)?;
            tx.commit().await.map_err(map)?;
            Err(e)
        }
    }
}
