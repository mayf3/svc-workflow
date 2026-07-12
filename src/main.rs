//! svc-workflow
//!
//! Serial governed workflow kernel (Rust + PostgreSQL).
//!
//! 当前状态：`IMPLEMENTATION_IN_PROGRESS` — PR 1 完成数据库骨架与不可变事实表。
//!
//! 本程序目前只提供 PostgreSQL 持久化基础类型、Schema 迁移入口和数据库约束测试。
//! 不启动 HTTP 服务，不实现 Command Service、Transition 引擎或 Legacy 导入。

mod domain;
mod store;

use store::postgres::migrations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!("svc-workflow: storage foundation");

    let pool = store::postgres::pool::create_pool().await;
    migrations::run(&pool).await;

    tracing::info!("migrations applied successfully");
    println!("svc-workflow: PostgreSQL storage foundation ready");

    Ok(())
}
