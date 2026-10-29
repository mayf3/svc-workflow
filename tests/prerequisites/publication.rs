use super::*;
use svc_workflow::{
    application::definition::{commands::PublishVersion, DefinitionService},
    store::postgres::definition_repository::PgDefinitionRepository,
};
#[tokio::test]
async fn publication_checks_context_root_marker_and_immutable_digest() {
    let p = pool().await;
    let f = setup(&p).await;
    let svc = DefinitionService::new(PgDefinitionRepository::new(p.clone()));
    let good = json!({"type":"object","x-workflow-prerequisite-v1":true,"required":["workflowPrerequisite"],"properties":{"workflowPrerequisite":svc_workflow::domain::definition::prerequisite::property_schema()}});
    for schema in [
        json!({"type":"object","x-workflow-prerequisite-v1":false}),
        json!({"type":"object","x-workflow-prerequisite-v1":true}),
        json!({"type":"object","properties":{"nested":{"type":"object","x-workflow-prerequisite-v1":true}}}),
    ] {
        let (v, _) = draft_version(&p, &f, schema, false, false).await;
        assert!(svc
            .publish_version(
                PublishVersion {
                    actor_principal_id: f.owner,
                    definition_version_id: v,
                    expected_revision: None
                },
                AdmissionGate::disabled()
            )
            .await
            .is_err());
        assert_eq!(sqlx::query_scalar::<_,String>("SELECT version_status::text FROM workflow_definition_versions WHERE definition_version_id=$1").bind(v).fetch_one(&p).await.unwrap(),"DRAFT");
    }
    let (v, _) = draft_version(&p, &f, good.clone(), false, false).await;
    let published = svc
        .publish_version(
            PublishVersion {
                actor_principal_id: f.owner,
                definition_version_id: v,
                expected_revision: None,
            },
            AdmissionGate::disabled(),
        )
        .await
        .unwrap();
    assert_eq!(published.context_schema, Some(good));
    assert_eq!(published.definition_digest.as_ref().unwrap().len(), 64);
    let changed=sqlx::query("UPDATE workflow_definition_versions SET context_schema='{}'::jsonb WHERE definition_version_id=$1").bind(v).execute(&p).await;
    assert!(changed.is_err(), "published opt-in stays immutable");
}
