//! Atomicity and fault injection tests for ReviseWorkflowContext.

#![allow(dead_code)]

use super::*;
use sqlx::Connection;

/// RAII trigger guard for context revision tests (reused pattern from instance_create).
const TEST_DB_URL: &str = "postgres://postgres:postgres@localhost:5432/svc_workflow";

struct ReviseTriggerGuard {
    suffix: String,
    table_or_kind: String,
    is_receipt: bool,
}

impl ReviseTriggerGuard {
    async fn install_revision_blocker(pool: &PgPool) -> Self {
        let suffix = Uuid::new_v4().to_string().replace('-', "");
        let fn_name = format!("fn_test_fail_{suffix}");
        let trg_name = format!("trg_test_fail_{suffix}");

        let _ = sqlx::query(&format!(
            "DROP TRIGGER IF EXISTS {trg_name} ON workflow_context_revisions"
        ))
        .execute(pool)
        .await;
        let _ = sqlx::query(&format!("DROP FUNCTION IF EXISTS {fn_name}()"))
            .execute(pool)
            .await;

        sqlx::query(&format!(
            "CREATE FUNCTION {fn_name}() RETURNS TRIGGER AS $$
             BEGIN RAISE EXCEPTION 'test_injected_failure: revision blocked' USING ERRCODE = '23000'; END;
             $$ LANGUAGE plpgsql"
        )).execute(pool).await.expect("create function");
        sqlx::query(&format!(
            "CREATE TRIGGER {trg_name} BEFORE INSERT ON workflow_context_revisions FOR EACH ROW EXECUTE FUNCTION {fn_name}()"
        )).execute(pool).await.expect("create trigger");

        Self {
            suffix,
            table_or_kind: "workflow_context_revisions".to_string(),
            is_receipt: false,
        }
    }

    async fn install_instance_update_blocker(pool: &PgPool) -> Self {
        let suffix = Uuid::new_v4().to_string().replace('-', "");
        let fn_name = format!("fn_test_fail_{suffix}");
        let trg_name = format!("trg_test_fail_{suffix}");

        let _ = sqlx::query(&format!(
            "DROP TRIGGER IF EXISTS {trg_name} ON workflow_instances"
        ))
        .execute(pool)
        .await;
        let _ = sqlx::query(&format!("DROP FUNCTION IF EXISTS {fn_name}()"))
            .execute(pool)
            .await;

        sqlx::query(&format!(
            "CREATE OR REPLACE FUNCTION {fn_name}() RETURNS TRIGGER AS $$
             BEGIN
                 IF TG_OP = 'UPDATE' THEN
                     RAISE EXCEPTION 'test_injected_failure: instance update blocked' USING ERRCODE = '23000';
                 END IF;
                 RETURN NEW;
             END;
             $$ LANGUAGE plpgsql"
        )).execute(pool).await.expect("create function");
        sqlx::query(&format!(
            "CREATE TRIGGER {trg_name} BEFORE UPDATE ON workflow_instances FOR EACH ROW EXECUTE FUNCTION {fn_name}()"
        )).execute(pool).await.expect("create trigger");

        Self {
            suffix,
            table_or_kind: "workflow_instances".to_string(),
            is_receipt: false,
        }
    }

    async fn install_event_blocker(pool: &PgPool) -> Self {
        let suffix = Uuid::new_v4().to_string().replace('-', "");
        let fn_name = format!("fn_test_fail_{suffix}");
        let trg_name = format!("trg_test_fail_{suffix}");

        let _ = sqlx::query(&format!(
            "DROP TRIGGER IF EXISTS {trg_name} ON workflow_events"
        ))
        .execute(pool)
        .await;
        let _ = sqlx::query(&format!("DROP FUNCTION IF EXISTS {fn_name}()"))
            .execute(pool)
            .await;

        sqlx::query(&format!(
            "CREATE FUNCTION {fn_name}() RETURNS TRIGGER AS $$
             BEGIN RAISE EXCEPTION 'test_injected_failure: event blocked' USING ERRCODE = '23000'; END;
             $$ LANGUAGE plpgsql"
        )).execute(pool).await.expect("create function");
        sqlx::query(&format!(
            "CREATE TRIGGER {trg_name} BEFORE INSERT ON workflow_events FOR EACH ROW EXECUTE FUNCTION {fn_name}()"
        )).execute(pool).await.expect("create trigger");

        Self {
            suffix,
            table_or_kind: "workflow_events".to_string(),
            is_receipt: false,
        }
    }
}

impl Drop for ReviseTriggerGuard {
    fn drop(&mut self) {
        let suffix = self.suffix.clone();
        let on_table = self.table_or_kind.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .build()
                .expect("build rt");
            rt.block_on(async move {
                let Ok(mut conn) = sqlx::PgConnection::connect(TEST_DB_URL).await else {
                    return;
                };
                let trg_name = format!("trg_test_fail_{suffix}");
                let fn_name = format!("fn_test_fail_{suffix}");
                let _ = sqlx::query(&format!("DROP TRIGGER IF EXISTS {trg_name} ON {on_table}"))
                    .execute(&mut conn)
                    .await;
                let _ = sqlx::query(&format!("DROP FUNCTION IF EXISTS {fn_name}()"))
                    .execute(&mut conn)
                    .await;
            });
        })
        .join()
        .ok();
    }
}

async fn seeded_instance(pool: &PgPool) -> (Uuid, Uuid) {
    let (principal_id, domain_id) = seed_principal_domain_with_owner(pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(pool, domain_id).await;
    let r = create_workflow_instance(pool, make_command(principal_id, domain_id, ver_id))
        .await
        .expect("create");
    (principal_id, r.workflow_instance_id)
}

#[tokio::test]
async fn test_revise_revision_insert_failure_rolls_back() {
    let pool = create_pool().await;
    let (principal_id, instance_id) = seeded_instance(&pool).await;
    let _guard = ReviseTriggerGuard::install_revision_blocker(&pool).await;
    let err = revise_workflow_context(
        &pool,
        make_revise_command(principal_id, instance_id, 1, serde_json::json!({"v": 2})),
    )
    .await;
    assert!(err.is_err(), "revision insert failure must fail");
    let rev_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_context_revisions WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(rev_count, 1, "only original revision");
}

#[tokio::test]
async fn test_revise_event_insert_failure_rolls_back() {
    let pool = create_pool().await;
    let (principal_id, instance_id) = seeded_instance(&pool).await;
    let _guard = ReviseTriggerGuard::install_event_blocker(&pool).await;
    let err = revise_workflow_context(
        &pool,
        make_revise_command(principal_id, instance_id, 1, serde_json::json!({"v": 2})),
    )
    .await;
    assert!(err.is_err(), "event insert failure must fail");
    let rev_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_context_revisions WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(rev_count, 1, "revision rolled back after event failure");
    let sv: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("sv");
    assert_eq!(sv, 1, "state version rolled back");
}
