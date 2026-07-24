//! Upgrade path verification: apply only 0001→0012, then upgrade with 0013→0014.
//!
//! This test verifies that the split migration can upgrade from an existing
//! 0001→0012 baseline without the PostgreSQL 55P04 error.
//!
//! It creates a temporary database and copies only base migrations into a
//! scratch directory so sqlx applies exactly 0001→0012 in phase 1, then
//! applies the full set (including 0013→0014) in phase 2.

use sqlx::migrate::Migrator;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::path::Path;

const ADMIN_URL: &str = "postgres://postgres:postgres@localhost:5432/postgres";
const MIGRATIONS_DIR: &str = "migrations";
const BASE_MIGRATIONS: &[&str] = &[
    "0001", "0002", "0003", "0004", "0005", "0006", "0007", "0008", "0009", "0010", "0011", "0012",
];

fn base_migration_name(prefix: &str) -> String {
    let entries = std::fs::read_dir(MIGRATIONS_DIR)
        .expect("read migrations directory")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .find(|f| f.starts_with(prefix))
        .unwrap_or_else(|| panic!("migration file starting with {prefix}"));
    entries
}

fn setup_base_migrations(tmp_dir: &Path) {
    std::fs::create_dir_all(tmp_dir).expect("create temp migration dir");
    for prefix in BASE_MIGRATIONS {
        let name = base_migration_name(prefix);
        let src = Path::new(MIGRATIONS_DIR).join(&name);
        let dst = tmp_dir.join(&name);
        std::fs::copy(&src, &dst).unwrap_or_else(|_| panic!("copy {name}"));
    }
}

async fn create_temporary_database() -> (String, PgPool) {
    let name = format!("svc_workflow_upgrade_{}", uuid::Uuid::new_v4().simple());
    let mut admin = PgConnection::connect(ADMIN_URL)
        .await
        .expect("connect to PostgreSQL administration database");
    admin
        .execute(format!("CREATE DATABASE {name}").as_str())
        .await
        .expect("create upgrade-test database");
    let url = format!("postgres://postgres:postgres@localhost:5432/{name}");
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to upgrade-test database");
    (name, pool)
}

async fn drop_database(name: &str) {
    let mut admin = PgConnection::connect(ADMIN_URL)
        .await
        .expect("connect for drop");
    admin
        .execute(format!("DROP DATABASE {name} WITH (FORCE)").as_str())
        .await
        .expect("drop database");
}

#[tokio::test]
async fn upgrade_0012_to_0014_succeeds() {
    let tmp_dir = std::env::temp_dir().join(format!("mig_upgrade_{}", uuid::Uuid::new_v4().simple()));
    setup_base_migrations(&tmp_dir);

    let (db_name, pool) = create_temporary_database().await;

    // Phase 1: Apply only base migrations (0001→0012)
    let base_migrator: Migrator = Migrator::new(tmp_dir.clone())
        .await
        .expect("load base migrations");
    base_migrator
        .run(&pool)
        .await
        .expect("base migrations (0001→0012) must apply cleanly");

    // Verify base state: migration 12 is applied, 13 is not
    let count_12: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM _sqlx_migrations WHERE version = 12 AND success",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");
    assert_eq!(
        count_12.0, 1,
        "migration 0012 must be applied after base phase"
    );

    let count_13: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM _sqlx_migrations WHERE version = 13",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");
    assert_eq!(count_13.0, 0, "migration 0013 must NOT yet be applied");

    let count_14: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM _sqlx_migrations WHERE version = 14",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");
    assert_eq!(count_14.0, 0, "migration 0014 must NOT yet be applied");

    // Phase 2: Apply full migrations (0013→0014 will run)
    let full_migrator: Migrator = Migrator::new(Path::new(MIGRATIONS_DIR))
        .await
        .expect("load full migrations");
    full_migrator
        .run(&pool)
        .await
        .expect("upgrade migrations (0013→0014) must apply cleanly without 55P04 error");

    // Verify final state
    let count_13: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM _sqlx_migrations WHERE version = 13 AND success",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");
    assert_eq!(
        count_13.0, 1,
        "migration 0013 must be applied after upgrade"
    );

    let count_14: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM _sqlx_migrations WHERE version = 14 AND success",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");
    assert_eq!(
        count_14.0, 1,
        "migration 0014 must be applied after upgrade"
    );

    // Verify the INSTANCE_INPUT_PRINCIPAL enum value is present
    let enum_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM pg_enum e JOIN pg_type t ON e.enumtypid = t.oid \
         WHERE t.typname = 'assignee_ref_type' AND e.enumlabel = 'INSTANCE_INPUT_PRINCIPAL'",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");
    assert_eq!(
        enum_count.0, 1,
        "INSTANCE_INPUT_PRINCIPAL enum value must exist after upgrade"
    );

    // Verify the assignee_input_key column exists
    let col_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::int8 FROM information_schema.columns \
         WHERE table_name = 'workflow_node_definitions' AND column_name = 'assignee_input_key'",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");
    assert_eq!(
        col_count.0, 1,
        "assignee_input_key column must exist after upgrade"
    );

    pool.close().await;
    drop_database(&db_name).await;

    // Cleanup temp dir
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
