//! PostgreSQL storage layer.

pub mod definition_repository;
pub mod migrations;
pub mod pool;
pub mod repository_rows;

/// Default database name used in development / CI.
#[allow(dead_code)]
pub const DEFAULT_DB_NAME: &str = "svc_workflow";
