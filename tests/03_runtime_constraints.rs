#![allow(clippy::needless_borrow)]
//! Test: Runtime entity constraints.
//!
//! Context Revision uniqueness, immutability, cross-instance protection.
//! Node Visit uniqueness and immutability.
//! Submission constraints.

mod common;

/// Helper to create a minimal workflow instance, returning key IDs.
async fn create_minimal_instance(pool: &sqlx::PgPool) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let (creator_id, domain_id) = common::seed_principal_and_domain(&pool).await;
    let (_, def_ver_id, node_id, _) = common::seed_workflow_definition(&pool, domain_id).await;

    let instance_id = uuid::Uuid::new_v4();
    let ctx_id = uuid::Uuid::new_v4();
    let visit_id = uuid::Uuid::new_v4();
    let digest = sha256_hex(b"{}");

    // Start a transaction with deferred constraint checking
    let mut tx = pool.begin().await.expect("begin tx");

    // Insert instance
    sqlx::query(
        r#"
        INSERT INTO workflow_instances
            (workflow_instance_id, domain_id, definition_version_id,
             created_by_principal_id, current_context_revision_id,
             current_node_visit_id, workflow_state_version)
        VALUES ($1, $2, $3, $4, $5, $6, 1)
        "#,
    )
    .bind(instance_id)
    .bind(domain_id)
    .bind(def_ver_id)
    .bind(creator_id)
    .bind(ctx_id)
    .bind(visit_id)
    .execute(&mut *tx)
    .await
    .expect("insert instance");

    // Insert context revision
    sqlx::query(
        r#"
        INSERT INTO workflow_context_revisions
            (context_revision_id, workflow_instance_id, revision_number,
             previous_revision_id, payload, payload_digest, created_by_principal_id)
        VALUES ($1, $2, 1, NULL, '{}'::jsonb, $3, $4)
        "#,
    )
    .bind(ctx_id)
    .bind(instance_id)
    .bind(&digest)
    .bind(creator_id)
    .execute(&mut *tx)
    .await
    .expect("insert context revision");

    // Insert node visit
    sqlx::query(
        r#"
        INSERT INTO workflow_node_visits
            (node_visit_id, workflow_instance_id, node_id, visit_number,
             assignee_principal_id, entered_by_transition_id)
        VALUES ($1, $2, $3, 1, $4, NULL)
        "#,
    )
    .bind(visit_id)
    .bind(instance_id)
    .bind(node_id)
    .bind(creator_id)
    .execute(&mut *tx)
    .await
    .expect("insert node visit");

    tx.commit().await.expect("commit tx");

    (instance_id, ctx_id, visit_id)
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(data))
}

// ============================================================
// Tests
// ============================================================

#[tokio::test]
async fn test_context_revision_number_unique_within_instance() {
    let pool = common::create_pool().await;
    let (instance_id, _ctx_id, _visit_id) = create_minimal_instance(&pool).await;

    let creator_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, enabled) VALUES ($1, 'HUMAN', 'Creator', TRUE)",
    )
    .bind(creator_id)
    .execute(&pool)
    .await
    .expect("insert creator");

    // Attempt to insert a second revision with the same revision_number
    let new_ctx_id = uuid::Uuid::new_v4();
    let result = sqlx::query(
        r#"
        INSERT INTO workflow_context_revisions
            (context_revision_id, workflow_instance_id, revision_number,
             previous_revision_id, payload, payload_digest, created_by_principal_id)
        VALUES ($1, $2, 1, NULL, '{}'::jsonb, $3, $4)
        "#,
    )
    .bind(new_ctx_id)
    .bind(instance_id)
    .bind(sha256_hex(b"{}"))
    .bind(creator_id)
    .execute(&pool)
    .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("unique constraint") || err_str.contains("violates unique"),
                "expected unique constraint violation for duplicate revision_number, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected unique constraint violation but insert succeeded"),
    }
}

#[tokio::test]
async fn test_context_revision_cannot_reference_other_instance() {
    let pool = common::create_pool().await;
    let (instance1, _, _) = create_minimal_instance(&pool).await;
    let (instance2, _, _) = create_minimal_instance(&pool).await;

    let creator_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, enabled) VALUES ($1, 'HUMAN', 'Creator', TRUE)",
    )
    .bind(creator_id)
    .execute(&pool)
    .await
    .expect("insert creator");

    // Get the ctx_id from instance1
    let row: (uuid::Uuid,) = sqlx::query_as(
        "SELECT context_revision_id FROM workflow_context_revisions WHERE workflow_instance_id = $1 LIMIT 1"
    )
    .bind(instance1)
    .fetch_one(&pool)
    .await
    .expect("get ctx from instance1");

    let ctx1 = row.0;

    // Try to insert revision #2 for instance2 with previous = ctx1 (from instance1)
    let ctx_rev2 = uuid::Uuid::new_v4();
    let result = sqlx::query(
        r#"
        INSERT INTO workflow_context_revisions
            (context_revision_id, workflow_instance_id, revision_number,
             previous_revision_id, payload, payload_digest, created_by_principal_id)
        VALUES ($1, $2, 2, $3, '{}'::jsonb, $4, $5)
        "#,
    )
    .bind(ctx_rev2)
    .bind(instance2)
    .bind(ctx1)
    .bind(sha256_hex(b"{}"))
    .bind(creator_id)
    .execute(&pool)
    .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("foreign key constraint")
                    || err_str.contains("fk_previous_revision"),
                "expected FK violation for cross-instance previous_revision, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected FK violation for cross-instance previous_revision"),
    }
}

#[tokio::test]
async fn test_context_revision_immutable() {
    let pool = common::create_pool().await;
    let (_instance_id, ctx_id, _) = create_minimal_instance(&pool).await;

    // Try to UPDATE a context revision
    let result = sqlx::query(
        "UPDATE workflow_context_revisions SET payload = '{\"x\":1}'::jsonb WHERE context_revision_id = $1",
    )
    .bind(ctx_id)
    .execute(&pool)
    .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("trg_context_revisions_immutable")
                    || err_str.contains("immutable record"),
                "expected trigger rejection of UPDATE, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected trigger rejection of UPDATE on context revision"),
    }
}

#[tokio::test]
async fn test_context_revision_cannot_delete() {
    let pool = common::create_pool().await;
    let (_instance_id, ctx_id, _) = create_minimal_instance(&pool).await;

    // Try to DELETE a context revision
    let result =
        sqlx::query("DELETE FROM workflow_context_revisions WHERE context_revision_id = $1")
            .bind(ctx_id)
            .execute(&pool)
            .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("trg_context_revisions_immutable")
                    || err_str.contains("immutable record"),
                "expected trigger rejection of DELETE, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected trigger rejection of DELETE on context revision"),
    }
}

#[tokio::test]
async fn test_node_visit_unique_per_instance_node() {
    let pool = common::create_pool().await;
    let (instance_id, _, visit_id) = create_minimal_instance(&pool).await;

    // Get the node_id from the existing visit
    let row: (uuid::Uuid, uuid::Uuid) = sqlx::query_as(
        "SELECT node_id, assignee_principal_id FROM workflow_node_visits WHERE node_visit_id = $1",
    )
    .bind(visit_id)
    .fetch_one(&pool)
    .await
    .expect("get node_id");

    let node_id = row.0;
    let assignee = row.1;

    // Try to insert another visit with same (instance, node, visit_number)
    let visit_id2 = uuid::Uuid::new_v4();
    let result = sqlx::query(
        r#"
        INSERT INTO workflow_node_visits
            (node_visit_id, workflow_instance_id, node_id, visit_number,
             assignee_principal_id)
        VALUES ($1, $2, $3, 1, $4)
        "#,
    )
    .bind(visit_id2)
    .bind(instance_id)
    .bind(node_id)
    .bind(assignee)
    .execute(&pool)
    .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("unique constraint") || err_str.contains("violates unique"),
                "expected unique constraint violation for duplicate visit, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected unique constraint violation for duplicate visit"),
    }
}

#[tokio::test]
async fn test_node_visit_immutable() {
    let pool = common::create_pool().await;
    let (_instance_id, _, visit_id) = create_minimal_instance(&pool).await;

    // Try to UPDATE a node visit
    let result =
        sqlx::query("UPDATE workflow_node_visits SET visit_number = 99 WHERE node_visit_id = $1")
            .bind(visit_id)
            .execute(&pool)
            .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("trg_node_visits_immutable")
                    || err_str.contains("immutable record"),
                "expected trigger rejection of UPDATE on node visit, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected trigger rejection of UPDATE on node visit"),
    }
}

#[tokio::test]
async fn test_node_visit_cannot_delete() {
    let pool = common::create_pool().await;
    let (_instance_id, _, visit_id) = create_minimal_instance(&pool).await;

    let result = sqlx::query("DELETE FROM workflow_node_visits WHERE node_visit_id = $1")
        .bind(visit_id)
        .execute(&pool)
        .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("trg_node_visits_immutable")
                    || err_str.contains("immutable record"),
                "expected trigger rejection of DELETE on node visit, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected trigger rejection of DELETE on node visit"),
    }
}

#[tokio::test]
async fn test_submission_unique_per_visit() {
    let pool = common::create_pool().await;
    let (instance_id, ctx_id, visit_id) = create_minimal_instance(&pool).await;

    let (_, _, _, trans_id) = {
        // Get domain from the instance
        let row: (uuid::Uuid,) = sqlx::query_as(
            "SELECT domain_id FROM workflow_instances WHERE workflow_instance_id = $1",
        )
        .bind(instance_id)
        .fetch_one(&pool)
        .await
        .expect("get domain");
        common::seed_workflow_definition(&pool, row.0).await
    };

    let author = {
        let row: (uuid::Uuid,) = sqlx::query_as(
            "SELECT assignee_principal_id FROM workflow_node_visits WHERE node_visit_id = $1",
        )
        .bind(visit_id)
        .fetch_one(&pool)
        .await
        .expect("get assignee");
        row.0
    };

    let digest = sha256_hex(b"{}");

    // Insert first submission
    let sub_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO workflow_submissions
            (submission_id, workflow_instance_id, source_node_visit_id,
             context_revision_id, author_principal_id, transition_id,
             payload, payload_digest, schema_version)
        VALUES ($1, $2, $3, $4, $5, $6, '{}'::jsonb, $7, 'v1')
        "#,
    )
    .bind(sub_id)
    .bind(instance_id)
    .bind(visit_id)
    .bind(ctx_id)
    .bind(author)
    .bind(trans_id)
    .bind(&digest)
    .execute(&pool)
    .await
    .expect("first submission should succeed");

    // Try to insert a second submission for the same visit
    let sub_id2 = uuid::Uuid::new_v4();
    let result = sqlx::query(
        r#"
        INSERT INTO workflow_submissions
            (submission_id, workflow_instance_id, source_node_visit_id,
             context_revision_id, author_principal_id, transition_id,
             payload, payload_digest, schema_version)
        VALUES ($1, $2, $3, $4, $5, $6, '{}'::jsonb, $7, 'v1')
        "#,
    )
    .bind(sub_id2)
    .bind(instance_id)
    .bind(visit_id)
    .bind(ctx_id)
    .bind(author)
    .bind(trans_id)
    .bind(sha256_hex(b"{}"))
    .execute(&pool)
    .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("unique constraint") || err_str.contains("violates unique"),
                "expected unique constraint violation for duplicate submission per visit, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected unique constraint violation for duplicate submission per visit"),
    }
}

#[tokio::test]
async fn test_submission_cannot_mix_instances() {
    let pool = common::create_pool().await;
    let (instance1, ctx1, visit1) = create_minimal_instance(&pool).await;
    let (_instance2, _ctx2, visit2) = create_minimal_instance(&pool).await;

    let author = {
        let row: (uuid::Uuid,) = sqlx::query_as(
            "SELECT assignee_principal_id FROM workflow_node_visits WHERE node_visit_id = $1",
        )
        .bind(visit1)
        .fetch_one(&pool)
        .await
        .expect("get assignee");
        row.0
    };

    let (_, _, _, trans_id) = {
        let row: (uuid::Uuid,) = sqlx::query_as(
            "SELECT domain_id FROM workflow_instances WHERE workflow_instance_id = $1",
        )
        .bind(instance1)
        .fetch_one(&pool)
        .await
        .expect("get domain");
        common::seed_workflow_definition(&pool, row.0).await
    };

    // Try to create a submission for instance1 but with instance2's visit
    // The composite FK prevents this since (visit, instance) must match
    let sub_id = uuid::Uuid::new_v4();
    let result = sqlx::query(
        r#"
        INSERT INTO workflow_submissions
            (submission_id, workflow_instance_id, source_node_visit_id,
             context_revision_id, author_principal_id, transition_id,
             payload, payload_digest, schema_version)
        VALUES ($1, $2, $3, $4, $5, $6, '{}'::jsonb, $7, 'v1')
        "#,
    )
    .bind(sub_id)
    .bind(instance1) // instance1
    .bind(visit2) // but visit from instance2
    .bind(ctx1)
    .bind(author)
    .bind(trans_id)
    .bind(sha256_hex(b"{}"))
    .execute(&pool)
    .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("foreign key constraint")
                    || err_str.contains("fk_submission_visit_same_instance"),
                "expected FK violation for cross-instance submission, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected FK violation for cross-instance submission"),
    }
}

#[tokio::test]
async fn test_submission_immutable() {
    let pool = common::create_pool().await;
    let (instance_id, ctx_id, visit_id) = create_minimal_instance(&pool).await;

    let (_, _, _, trans_id) = {
        let row: (uuid::Uuid,) = sqlx::query_as(
            "SELECT domain_id FROM workflow_instances WHERE workflow_instance_id = $1",
        )
        .bind(instance_id)
        .fetch_one(&pool)
        .await
        .expect("get domain");
        common::seed_workflow_definition(&pool, row.0).await
    };

    let author = {
        let row: (uuid::Uuid,) = sqlx::query_as(
            "SELECT assignee_principal_id FROM workflow_node_visits WHERE node_visit_id = $1",
        )
        .bind(visit_id)
        .fetch_one(&pool)
        .await
        .expect("get assignee");
        row.0
    };

    let sub_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO workflow_submissions
            (submission_id, workflow_instance_id, source_node_visit_id,
             context_revision_id, author_principal_id, transition_id,
             payload, payload_digest, schema_version)
        VALUES ($1, $2, $3, $4, $5, $6, '{}'::jsonb, $7, 'v1')
        "#,
    )
    .bind(sub_id)
    .bind(instance_id)
    .bind(visit_id)
    .bind(ctx_id)
    .bind(author)
    .bind(trans_id)
    .bind(sha256_hex(b"{}"))
    .execute(&pool)
    .await
    .expect("insert submission");

    // Try to UPDATE the submission
    let result = sqlx::query(
        "UPDATE workflow_submissions SET payload = '{\"x\":1}'::jsonb WHERE submission_id = $1",
    )
    .bind(sub_id)
    .execute(&pool)
    .await;

    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("trg_submissions_immutable")
                    || err_str.contains("immutable record"),
                "expected trigger rejection of UPDATE on submission, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("expected trigger rejection of UPDATE on submission"),
    }
}
