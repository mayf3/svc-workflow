//! One-off server-side provisioning of a global (domain-independent) role
//! binding.
//!
//! Executes through the SAME audited application path as the HTTP
//! provisioning API (`provision_global_role_binding`: advisory lock,
//! provisioning-actor validation, idempotency receipt) — no direct SQL.
//! Written for the GLOBAL_WORKFLOW_AUDITOR grant (mayf3/svc-workflow#80):
//! after the canonical-identity cutover no HTTP caller holds both a live
//! provisioning credential and the allowlisted subject.
//!
//! Usage:
//!   DATABASE_URL=... \
//!   PROVISIONING_ACTOR_PRINCIPAL_ID=<allowlisted enabled AGENT principal> \
//!   TARGET_PRINCIPAL_ID=<principal to grant> \
//!   ROLE_KEY=<global role key> ENABLED=true \
//!   IDEMPOTENCY_KEY=<stable key> [REQUEST_ID=...] \
//!   cargo run --bin provision-global-role

use std::env;
use std::process;

use uuid::Uuid;

use svc_workflow::application::provisioning::provision_global_role_binding;
use svc_workflow::domain::ids::PrincipalId;
use svc_workflow::domain::provisioning::ProvisionGlobalRoleBindingCommand;

#[tokio::main]
async fn main() {
    let required = [
        "DATABASE_URL",
        "PROVISIONING_ACTOR_PRINCIPAL_ID",
        "TARGET_PRINCIPAL_ID",
        "ROLE_KEY",
        "ENABLED",
        "IDEMPOTENCY_KEY",
    ];
    for key in required {
        if env::var(key).unwrap_or_default().is_empty() {
            eprintln!("missing required env: {key}");
            process::exit(2);
        }
    }

    let database_url = env::var("DATABASE_URL").unwrap();
    let actor = Uuid::parse_str(&env::var("PROVISIONING_ACTOR_PRINCIPAL_ID").unwrap())
        .expect("PROVISIONING_ACTOR_PRINCIPAL_ID must be a UUID");
    let target = Uuid::parse_str(&env::var("TARGET_PRINCIPAL_ID").unwrap())
        .expect("TARGET_PRINCIPAL_ID must be a UUID");
    let role_key = env::var("ROLE_KEY").unwrap();
    let enabled = env::var("ENABLED").unwrap().eq_ignore_ascii_case("true");
    let idempotency_key = env::var("IDEMPOTENCY_KEY").unwrap();
    let request_id = env::var("REQUEST_ID").unwrap_or_else(|_| "-".to_string());

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("connect to database");

    let cmd = ProvisionGlobalRoleBindingCommand {
        principal_id: PrincipalId::from_uuid(target),
        role_key,
        enabled,
    };

    let receipt = provision_global_role_binding(
        &pool,
        &cmd,
        &idempotency_key,
        &request_id,
        &PrincipalId::from_uuid(actor),
    )
    .await
    .expect("provision global role binding");

    println!("{}", serde_json::to_string_pretty(&receipt).unwrap());
}
