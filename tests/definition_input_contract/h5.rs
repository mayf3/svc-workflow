//! Reading an input contract must not grant any of the four H-5 reads.

use super::*;
use svc_workflow::application::definition::queries::{
    GetCompleteVersionGraph, GetDefinition, GetDefinitionVersion, ListDefinitionVersions,
};

async fn h5_reads(
    pool: &sqlx::PgPool,
    seeded: &PublishedVersion,
    actor: uuid::Uuid,
) -> [Result<(), DefinitionError>; 4] {
    let service = DefinitionService::new(PgDefinitionRepository::new(pool.clone()));
    [
        service
            .get_definition(GetDefinition {
                actor_principal_id: actor,
                workflow_definition_id: seeded.definition_id,
            })
            .await
            .map(|_| ()),
        service
            .get_definition_version(GetDefinitionVersion {
                actor_principal_id: actor,
                definition_version_id: seeded.version_id,
            })
            .await
            .map(|_| ()),
        service
            .list_definition_versions(ListDefinitionVersions {
                actor_principal_id: actor,
                workflow_definition_id: seeded.definition_id,
            })
            .await
            .map(|_| ()),
        service
            .get_complete_version_graph(GetCompleteVersionGraph {
                actor_principal_id: actor,
                definition_version_id: seeded.version_id,
            })
            .await
            .map(|_| ()),
    ]
}

#[tokio::test]
async fn p2_member_still_denied_on_all_four_h5_owner_reads() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member = seed_domain_member(&pool, seeded.domain_id, true).await;
    member_read(&pool, member, seeded.version_id)
        .await
        .expect("member may read only the input contract");

    for (index, result) in h5_reads(&pool, &seeded, member)
        .await
        .into_iter()
        .enumerate()
    {
        assert!(
            matches!(result, Err(DefinitionError::PermissionDenied)),
            "H-5 read {index} must deny a same-domain member: {result:?}"
        );
    }
}

#[tokio::test]
async fn p3_owner_reads_new_surface_and_all_four_h5_reads_still_work() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    member_read(&pool, seeded.owner_id, seeded.version_id)
        .await
        .expect("owner input-contract read unchanged");

    for (index, result) in h5_reads(&pool, &seeded, seeded.owner_id)
        .await
        .into_iter()
        .enumerate()
    {
        assert!(result.is_ok(), "owner H-5 read {index}: {result:?}");
    }
}
