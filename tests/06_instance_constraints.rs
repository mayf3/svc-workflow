#![allow(clippy::needless_borrow)]
//! Test: Workflow Instance immutable fields.

mod common;

/// Create a minimal instance and return its ID.
async fn create_instance(pool: &sqlx::PgPool) -> uuid::Uuid {
    let (creator_id, domain_id) = common::seed_principal_and_domain(&pool).await;
    let (_, def_ver_id, node_id, _) = common::seed_workflow_definition(&pool, domain_id).await;

    let instance_id = uuid::Uuid::new_v4();
    let ctx_id = uuid::Uuid::new_v4();
    let visit_id = uuid::Uuid::new_v4();
    let digest = sha256_hex(b"{}");

    let mut tx = pool.begin().await.expect("begin tx");

    sqlx::query(
        r#"INSERT INTO workflow_instances (workflow_instance_id, domain_id, definition_version_id, created_by_principal_id, current_context_revision_id, current_node_visit_id, workflow_state_version) VALUES ($1,$2,$3,$4,$5,$6,1)"#
    )
    .bind(instance_id).bind(domain_id).bind(def_ver_id).bind(creator_id).bind(ctx_id).bind(visit_id)
    .execute(&mut *tx).await.expect("insert instance");

    sqlx::query(
        r#"INSERT INTO workflow_context_revisions (context_revision_id, workflow_instance_id, revision_number, previous_revision_id, payload, payload_digest, created_by_principal_id) VALUES ($1,$2,1,NULL,'{}'::jsonb,$3,$4)"#
    )
    .bind(ctx_id).bind(instance_id).bind(&digest).bind(creator_id)
    .execute(&mut *tx).await.expect("insert ctx");

    sqlx::query(
        r#"INSERT INTO workflow_node_visits (node_visit_id, workflow_instance_id, node_id, visit_number, assignee_principal_id) VALUES ($1,$2,$3,1,$4)"#
    )
    .bind(visit_id).bind(instance_id).bind(node_id).bind(creator_id)
    .execute(&mut *tx).await.expect("insert visit");

    tx.commit().await.expect("commit tx");
    instance_id
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(data))
}

#[tokio::test]
async fn test_instance_domain_id_immutable() {
    let pool = common::create_pool().await;
    let instance_id = create_instance(&pool).await;

    // Create another domain
    let other_domain = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO domains (domain_id, domain_key, display_name, enabled) VALUES ($1, 'other-domain', 'Other', TRUE)"
    )
    .bind(other_domain)
    .execute(&pool)
    .await
    .expect("insert other domain");

    // Try to change domain_id
    let result =
        sqlx::query("UPDATE workflow_instances SET domain_id = $1 WHERE workflow_instance_id = $2")
            .bind(other_domain)
            .bind(instance_id)
            .execute(&pool)
            .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("trg_instance_immutable_fields")
                    || err_str.contains("immutable field"),
                "expected trigger rejection of domain_id change, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected trigger rejection of domain_id change"),
    }
}

#[tokio::test]
async fn test_instance_definition_version_id_immutable() {
    let pool = common::create_pool().await;
    let instance_id = create_instance(&pool).await;

    // Changing workflow_state_version should be allowed (it's a projection field)
    let result = sqlx::query(
        "UPDATE workflow_instances SET workflow_state_version = 2 WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .execute(&pool)
    .await;

    assert!(
        result.is_ok(),
        "changing workflow_state_version should succeed"
    );
}

#[tokio::test]
async fn test_instance_created_by_principal_id_immutable() {
    let pool = common::create_pool().await;
    let instance_id = create_instance(&pool).await;

    let other_principal = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, enabled) VALUES ($1, 'HUMAN', 'Other', TRUE)"
    )
    .bind(other_principal)
    .execute(&pool)
    .await
    .expect("insert other principal");

    // Try to change created_by_principal_id
    let result = sqlx::query(
        "UPDATE workflow_instances SET created_by_principal_id = $1 WHERE workflow_instance_id = $2"
    )
    .bind(other_principal)
    .bind(instance_id)
    .execute(&pool)
    .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("trg_instance_immutable_fields")
                    || err_str.contains("immutable field"),
                "expected trigger rejection of created_by_principal_id change, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected trigger rejection of created_by_principal_id change"),
    }
}

#[tokio::test]
async fn test_workflow_state_version_minimum() {
    let pool = common::create_pool().await;
    let (creator_id, domain_id) = common::seed_principal_and_domain(&pool).await;
    let (_, def_ver_id, _node_id, _) = common::seed_workflow_definition(&pool, domain_id).await;

    let instance_id = uuid::Uuid::new_v4();

    // Try to insert with workflow_state_version = 0
    let result = sqlx::query(
        r#"
        INSERT INTO workflow_instances
            (workflow_instance_id, domain_id, definition_version_id,
             created_by_principal_id, workflow_state_version)
        VALUES ($1, $2, $3, $4, 0)
        "#,
    )
    .bind(instance_id)
    .bind(domain_id)
    .bind(def_ver_id)
    .bind(creator_id)
    .execute(&pool)
    .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("check constraint") || err_str.contains("CHECK"),
                "expected CHECK constraint violation for version=0, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected CHECK constraint failure for workflow_state_version=0"),
    }
}
