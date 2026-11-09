use super::integrity_support::*;
use super::*;
async fn predecessor() -> PgPool {
    let db = format!("svc_prereq_v2_upgrade_{}", Uuid::new_v4().simple());
    let admin = PgPool::connect(&common::admin_database_url())
        .await
        .unwrap();
    sqlx::query(&format!("CREATE DATABASE {db}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
    println!("fresh predecessor database: {db}");
    let p = PgPool::connect(&format!("{}/{}", common::test_database_base(), db))
        .await
        .unwrap();
    let dir = std::env::temp_dir().join(format!("svc-prereq-v29-{}", Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    for e in std::fs::read_dir("migrations").unwrap() {
        let e = e.unwrap();
        let name = e.file_name();
        if name
            .to_string_lossy()
            .split('_')
            .next()
            .unwrap()
            .parse::<u32>()
            .unwrap()
            <= 29
        {
            std::fs::copy(e.path(), dir.join(name)).unwrap();
        }
    }
    sqlx::migrate::Migrator::new(dir.as_path())
        .await
        .unwrap()
        .run(&p)
        .await
        .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    p
}
async fn history(p: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('events',(SELECT jsonb_agg(to_jsonb(e)-'command_event_slot' ORDER BY event_id) FROM workflow_events e),'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY command_id) FROM workflow_command_receipts r))").fetch_one(p).await.unwrap()
}
#[tokio::test]
async fn upgrade_preserves_actual_history_and_old_index_rollback_refuses_valid_pair() {
    let p = predecessor().await;
    setup(&p).await;
    let before = history(&p).await;
    let file_before: i64 =
        sqlx::query_scalar("SELECT pg_relation_filenode('workflow_events')::bigint")
            .fetch_one(&p)
            .await
            .unwrap();
    sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
        .await
        .unwrap()
        .run(&p)
        .await
        .unwrap();
    assert_eq!(history(&p).await, before);
    let file_after: i64 =
        sqlx::query_scalar("SELECT pg_relation_filenode('workflow_events')::bigint")
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(
        file_before, file_after,
        "constant default must not rewrite the historical heap"
    );
    println!(
        "business history sha256 before/after {} heap {}",
        svc_workflow::domain::definition::digest::compute_json_digest(&before).unwrap(),
        file_before
    );
    // Restoring the predecessor unique index succeeds before a pair exists.
    sqlx::query("CREATE UNIQUE INDEX idx_wf_event_unique_command ON workflow_events(command_id) WHERE command_id IS NOT NULL").execute(&p).await.unwrap();
    sqlx::query("DROP INDEX idx_wf_event_unique_command")
        .execute(&p)
        .await
        .unwrap();
    let (_, c, _, _) = pair(&p).await;
    let paired = history(&p).await;
    let e=sqlx::query("CREATE UNIQUE INDEX idx_wf_event_unique_command ON workflow_events(command_id) WHERE command_id IS NOT NULL").execute(&p).await.unwrap_err();
    assert_eq!(
        e.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23505")
    );
    assert_eq!(history(&p).await, paired);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM workflow_events WHERE command_id=$1")
            .bind(c)
            .fetch_one(&p)
            .await
            .unwrap(),
        2
    );
    assert!(sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('idx_wf_event_command_slot') IS NOT NULL"
    )
    .fetch_one(&p)
    .await
    .unwrap());
}
#[tokio::test]
async fn migration_refuses_invalid_preexisting_success_without_changing_history() {
    let p = predecessor().await;
    let f = setup(&p).await;
    let c = receipt(&p, f.owner, "CREATE_AND_BIND_GOVERNANCE_CHILD").await;
    let mut tx = p.begin().await.unwrap();
    complete(&mut tx, c, 201).await.unwrap();
    tx.commit().await.unwrap();
    let before = history(&p).await;
    let e = sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
        .await
        .unwrap()
        .run(&p)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("exactly two events"), "{e}");
    assert_eq!(history(&p).await, before);
    assert!(sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('idx_wf_event_unique_command') IS NOT NULL"
    )
    .fetch_one(&p)
    .await
    .unwrap());
    assert!(!sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_name='workflow_events' AND column_name='command_event_slot')").fetch_one(&p).await.unwrap());
}
