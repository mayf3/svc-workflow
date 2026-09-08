// One-time P7 executor: SOURCE_ONLY_STOP_NEW for exactly two frozen bad
// definition versions (V2 CTR-CIR-003 final paragraph + runbook
// SOURCE_ONLY_STOP_NEW_V1). Governed lifecycle API only: atomic deprecation
// with recorded reason; no direct SQL; no delete/cancel/assignee rewrite.
use svc_workflow::application::definition::{commands::DeprecateVersion, DefinitionService, SOURCE_IDENTITY_UNRESOLVED};
use svc_workflow::store::postgres::definition_repository::PgDefinitionRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let actor: uuid::Uuid = std::env::var("ACTOR_PRINCIPAL_ID").expect("ACTOR_PRINCIPAL_ID").parse()?;
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(&url).await?;
    let service = DefinitionService::new(PgDefinitionRepository::new(pool.clone()));
    for vid in [
        "9b07afc4-d3a2-456d-8b96-13fdffbaf995",
        "e01d1f3a-661b-468f-9eda-0506abaa5c0b",
    ] {
        let out = service
            .deprecate_version(DeprecateVersion {
                actor_principal_id: actor,
                definition_version_id: vid.parse()?,
                deprecation_reason: Some(SOURCE_IDENTITY_UNRESOLVED.into()),
            })
            .await?;
        println!("DEPRECATED {} -> status={:?}", vid, out.version_status);
    }
    Ok(())
}
