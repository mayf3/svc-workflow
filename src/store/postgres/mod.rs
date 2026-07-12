//! PostgreSQL storage layer.

pub mod migrations;
pub mod pool;

/// Default database name used in development / CI.
#[allow(dead_code)]
pub const DEFAULT_DB_NAME: &str = "svc_workflow";
