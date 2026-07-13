//! svc-workflow — Serial governed workflow kernel (Rust + PostgreSQL).
//!
//! 当前状态：`IMPLEMENTATION_IN_PROGRESS` — PR 2 完成 Definition 与不可变版本发布服务。
//!
//! 本程序目前只提供 PostgreSQL 持久化基础类型、Schema 迁移入口、数据库约束测试
//! 以及 Workflow Definition / Version 管理服务。

use svc_workflow::store::postgres::migrations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!("svc-workflow: definition version service");

    let pool = svc_workflow::store::postgres::pool::create_pool().await;
    migrations::run(&pool).await;

    tracing::info!("migrations applied successfully");
    println!("svc-workflow: Definition & Version service ready");

    Ok(())
}
