//! Atomicity tests (35-38, reduced to 4 focused tests).

use super::*;

#[tokio::test]
async fn test_exactly_one_event_per_creation() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let result = create_workflow_instance(&pool, make_command(principal_id, domain_id, ver_id))
        .await
        .expect("create");
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_events WHERE workflow_instance_id = $1")
            .bind(result.workflow_instance_id)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_command_id_matches_event() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let result = create_workflow_instance(&pool, make_command(principal_id, domain_id, ver_id))
        .await
        .expect("create");
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_events e JOIN workflow_command_receipts r ON e.command_id = r.command_id WHERE e.workflow_instance_id = $1",
    ).bind(result.workflow_instance_id).fetch_one(&pool).await.expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_deferred_fk_committed_successfully() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let result = create_workflow_instance(&pool, make_command(principal_id, domain_id, ver_id))
        .await
        .expect("create");
    let fk_ok: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflow_instances i \
         JOIN workflow_context_revisions cr ON cr.context_revision_id = i.current_context_revision_id AND cr.workflow_instance_id = i.workflow_instance_id \
         JOIN workflow_node_visits nv ON nv.node_visit_id = i.current_node_visit_id AND nv.workflow_instance_id = i.workflow_instance_id \
         WHERE i.workflow_instance_id = $1)",
    ).bind(result.workflow_instance_id).fetch_one(&pool).await.expect("check");
    assert!(fk_ok, "circular FKs must resolve");
}

#[tokio::test]
async fn test_instance_created_exactly_one_event() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let result = create_workflow_instance(&pool, make_command(principal_id, domain_id, ver_id))
        .await
        .expect("create");
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_events WHERE workflow_instance_id = $1")
            .bind(result.workflow_instance_id)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 1, "exactly one event");
}
