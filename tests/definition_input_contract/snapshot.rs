//! Real PostgreSQL interleavings through the production service/repository.
//! Run suite 40 with --test-threads=1: table locks are database-wide barriers.

use super::*;
use std::time::Duration;

/// Match the backend's actual blocked SELECT, rather than assuming a sleep
/// means that the earlier reads finished. Mirrors the query snapshot harness.
async fn wait_for_read(pool: &sqlx::PgPool, table: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                 WHERE datname = current_database() AND wait_event_type = 'Lock'
                   AND state = 'active' AND query LIKE 'SELECT%'
                   AND query LIKE $1)",
            )
            .bind(format!("%FROM {table} WHERE%"))
            .fetch_one(pool)
            .await
            .unwrap();
            if waiting {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("production SELECT did not reach the database barrier");
}

fn start_read(
    pool: &sqlx::PgPool,
    member: uuid::Uuid,
    version: uuid::Uuid,
) -> tokio::task::JoinHandle<
    Result<
        svc_workflow::application::definition::queries::PublishedVersionInputContract,
        DefinitionError,
    >,
> {
    let pool = pool.clone();
    tokio::spawn(async move { member_read(&pool, member, version).await })
}

async fn finish_read(
    task: tokio::task::JoinHandle<
        Result<
            svc_workflow::application::definition::queries::PublishedVersionInputContract,
            DefinitionError,
        >,
    >,
) -> Result<
    svc_workflow::application::definition::queries::PublishedVersionInputContract,
    DefinitionError,
> {
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("read did not finish after releasing barrier")
        .unwrap()
}

async fn change_binding(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    domain: uuid::Uuid,
    member: uuid::Uuid,
    enabled: bool,
) {
    sqlx::query(
        "UPDATE domain_role_bindings SET enabled = $3 WHERE domain_id = $1 AND principal_id = $2",
    )
    .bind(domain)
    .bind(member)
    .bind(enabled)
    .execute(&mut **tx)
    .await
    .unwrap();
}

async fn deprecate(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, seed: &PublishedVersion) {
    // Legal lifecycle edge and actor; normal immutability/FK triggers remain active.
    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'DEPRECATED', deprecated_at = now(), deprecated_by_principal_id = $2 WHERE definition_version_id = $1")
        .bind(seed.version_id).bind(seed.owner_id)
        .execute(&mut **tx).await.unwrap();
}

#[tokio::test]
async fn snapshot_deprecation_before_membership_grant_never_authorizes() {
    let pool = create_pool().await;
    let seed = seed_published_version(&pool).await;
    let member = seed_domain_member(&pool, seed.domain_id, false).await;
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE domain_role_bindings IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await
        .unwrap();
    let read = start_read(&pool, member, seed.version_id);
    wait_for_read(&pool, "domain_role_bindings").await;
    deprecate(&mut writer, &seed).await;
    change_binding(&mut writer, seed.domain_id, member, true).await;
    writer.commit().await.unwrap();

    // Before commit: PUBLISHED but not a member. After: member but DEPRECATED.
    // There is no legal linearization point. The initial snapshot denies binding.
    let error = finish_read(read)
        .await
        .expect_err("never-authorized schema leaked");
    assert!(
        matches!(error, DefinitionError::PermissionDenied),
        "{error:?}"
    );
    assert!(matches!(
        member_read(&pool, member, seed.version_id).await,
        Err(DefinitionError::VersionNotPublished)
    ));
}

#[tokio::test]
async fn snapshot_membership_revocation_before_domain_enable_never_authorizes() {
    let pool = create_pool().await;
    let seed = seed_published_version(&pool).await;
    let member = seed_domain_member(&pool, seed.domain_id, true).await;
    set_domain_enabled(&pool, seed.domain_id, false).await;
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE domains IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await
        .unwrap();
    let read = start_read(&pool, member, seed.version_id);
    wait_for_read(&pool, "domains").await;
    change_binding(&mut writer, seed.domain_id, member, false).await;
    sqlx::query("UPDATE domains SET enabled = TRUE WHERE domain_id = $1")
        .bind(seed.domain_id)
        .execute(&mut *writer)
        .await
        .unwrap();
    writer.commit().await.unwrap();

    let error = finish_read(read)
        .await
        .expect_err("never-authorized schema leaked");
    assert!(
        matches!(error, DefinitionError::DomainDisabled),
        "{error:?}"
    );
    assert!(matches!(
        member_read(&pool, member, seed.version_id).await,
        Err(DefinitionError::PermissionDenied)
    ));
}

#[tokio::test]
async fn snapshot_principal_disable_before_membership_grant_never_authorizes() {
    let pool = create_pool().await;
    let seed = seed_published_version(&pool).await;
    let member = seed_domain_member(&pool, seed.domain_id, false).await;
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE domain_role_bindings IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await
        .unwrap();
    let read = start_read(&pool, member, seed.version_id);
    wait_for_read(&pool, "domain_role_bindings").await;
    sqlx::query("UPDATE principals SET enabled = FALSE WHERE principal_id = $1")
        .bind(member)
        .execute(&mut *writer)
        .await
        .unwrap();
    change_binding(&mut writer, seed.domain_id, member, true).await;
    writer.commit().await.unwrap();

    let error = finish_read(read)
        .await
        .expect_err("never-authorized schema leaked");
    assert!(
        matches!(error, DefinitionError::PermissionDenied),
        "{error:?}"
    );
    assert!(matches!(
        member_read(&pool, member, seed.version_id).await,
        Err(DefinitionError::PrincipalDisabled)
    ));
}

#[tokio::test]
async fn snapshot_authorized_before_deprecation_can_return_old_contract() {
    let pool = create_pool().await;
    let seed = seed_published_version(&pool).await;
    let member = seed_domain_member(&pool, seed.domain_id, true).await;
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE domain_role_bindings IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await
        .unwrap();
    let read = start_read(&pool, member, seed.version_id);
    wait_for_read(&pool, "domain_role_bindings").await;
    deprecate(&mut writer, &seed).await;
    writer.commit().await.unwrap();

    let contract = finish_read(read)
        .await
        .expect("initial snapshot was authorized");
    assert_eq!(contract.version_status, DefinitionVersionStatus::PUBLISHED);
    assert_eq!(contract.context_schema, Some(input_schema()));
    assert!(matches!(
        member_read(&pool, member, seed.version_id).await,
        Err(DefinitionError::VersionNotPublished)
    ));
}

#[tokio::test]
async fn snapshot_authorized_before_binding_revocation_can_return_old_contract() {
    let pool = create_pool().await;
    let seed = seed_published_version(&pool).await;
    let member = seed_domain_member(&pool, seed.domain_id, true).await;
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE domains IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await
        .unwrap();
    let read = start_read(&pool, member, seed.version_id);
    wait_for_read(&pool, "domains").await;
    change_binding(&mut writer, seed.domain_id, member, false).await;
    writer.commit().await.unwrap();

    let contract = finish_read(read)
        .await
        .expect("initial snapshot was authorized");
    assert_eq!(contract.version_status, DefinitionVersionStatus::PUBLISHED);
    assert_eq!(contract.context_schema, Some(input_schema()));
    assert!(matches!(
        member_read(&pool, member, seed.version_id).await,
        Err(DefinitionError::PermissionDenied)
    ));
}

#[tokio::test]
async fn snapshot_denial_precedence_and_missing_principal_are_preserved() {
    let pool = create_pool().await;
    let seed = seed_published_version(&pool).await;
    let member = seed_domain_member(&pool, seed.domain_id, false).await;
    let mut tx = pool.begin().await.unwrap();
    deprecate(&mut tx, &seed).await;
    tx.commit().await.unwrap();
    set_domain_enabled(&pool, seed.domain_id, false).await;

    assert!(matches!(
        member_read(&pool, uuid::Uuid::new_v4(), uuid::Uuid::new_v4()).await,
        Err(DefinitionError::PrincipalNotFound)
    ));
    assert!(matches!(
        member_read(&pool, member, uuid::Uuid::new_v4()).await,
        Err(DefinitionError::DefinitionVersionNotFound)
    ));
    assert!(matches!(
        member_read(&pool, member, seed.version_id).await,
        Err(DefinitionError::PermissionDenied)
    ));
    sqlx::query(
        "UPDATE domain_role_bindings SET enabled = TRUE WHERE domain_id = $1 AND principal_id = $2",
    )
    .bind(seed.domain_id)
    .bind(member)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        member_read(&pool, member, seed.version_id).await,
        Err(DefinitionError::DomainDisabled)
    ));
}
