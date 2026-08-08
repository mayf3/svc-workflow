//! svc-workflow internal HTTP service + maintenance CLI.

use std::path::PathBuf;

use svc_workflow::application::workflow_instance::admin_repair::{
    apply_repair_context, plan_repair_context, RepairContextRequest,
};
use svc_workflow::http::{self, AppState, HttpConfig};
use svc_workflow::store::postgres::migrations;
use uuid::Uuid;

type PgPool = sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str());
    let migrate_only = command == Some("--migrate");
    let repair_mode = command == Some("repair-context");

    let config =
        if migrate_only || repair_mode {
            None
        } else {
            Some(HttpConfig::from_env().map_err(|message| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
            })?)
        };
    let pool = svc_workflow::store::postgres::pool::create_pool().await;
    migrations::run(&pool).await;
    tracing::info!("migrations applied successfully");
    if migrate_only {
        return Ok(());
    }
    if repair_mode {
        return run_repair_command(&pool, &args[2..]).await;
    }

    let config = config.expect("server configuration loaded above");
    let state = AppState::new(pool, &config);
    let app = http::router(state, &config);
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!(address = %config.bind_addr, "svc-workflow HTTP server listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// ===========================================================================
// repair-context maintenance CLI
//
// `svc-workflow repair-context --instance-id <uuid> --payload-file <path>
//   --operator <uuid> --reason <str> --repair-source <str> [--apply]`
//
// Defaults to DRY-RUN: prints the full repair plan and writes nothing.
// `--apply` is required to append the context revision. The `--operator` is
// an AUDIT ATTRIBUTION, not caller authentication — this CLI may only run in
// a trusted host operations environment; the DOMAIN_OWNER/WORKFLOW_ADMIN
// role check guards against misoperation.
// ===========================================================================

struct RepairCliArgs {
    instance_id: Uuid,
    payload_file: PathBuf,
    operator: Uuid,
    reason: String,
    repair_source: String,
    apply: bool,
}

const REPAIR_USAGE: &str = "\
usage: svc-workflow repair-context \\
  --instance-id <uuid> \\
  --payload-file <path-to-json> \\
  --operator <principal-uuid> \\
  --reason <string> \\
  --repair-source <string> \\
  [--apply]

Defaults to DRY-RUN (prints the plan, writes nothing).
--apply commits the repair (append context revision + audit).
--operator is AUDIT ATTRIBUTION, not caller authentication.";

fn parse_repair_args(args: &[String]) -> Result<RepairCliArgs, String> {
    let mut instance_id: Option<String> = None;
    let mut payload_file: Option<String> = None;
    let mut operator: Option<String> = None;
    let mut reason: Option<String> = None;
    let mut repair_source: Option<String> = None;
    let mut apply = false;

    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        let mut value = || -> Result<String, String> {
            i += 1;
            args.get(i)
                .cloned()
                .ok_or_else(|| format!("{flag} requires a value"))
        };
        match flag {
            "--instance-id" => instance_id = Some(value()?),
            "--payload-file" => payload_file = Some(value()?),
            "--operator" => operator = Some(value()?),
            "--reason" => reason = Some(value()?),
            "--repair-source" => repair_source = Some(value()?),
            "--apply" => apply = true,
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    let instance_id = instance_id
        .ok_or_else(|| "--instance-id is required".to_string())?
        .parse::<Uuid>()
        .map_err(|e| format!("--instance-id must be a UUID: {e}"))?;
    let operator = operator
        .ok_or_else(|| "--operator is required".to_string())?
        .parse::<Uuid>()
        .map_err(|e| format!("--operator must be a UUID: {e}"))?;
    let payload_file = payload_file.ok_or_else(|| "--payload-file is required".to_string())?;
    let reason = reason.ok_or_else(|| "--reason is required".to_string())?;
    let repair_source = repair_source.ok_or_else(|| "--repair-source is required".to_string())?;

    Ok(RepairCliArgs {
        instance_id,
        payload_file: PathBuf::from(payload_file),
        operator,
        reason,
        repair_source,
        apply,
    })
}

async fn run_repair_command(
    pool: &PgPool,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let cli = match parse_repair_args(args) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("repair-context: {message}");
            eprintln!("{REPAIR_USAGE}");
            std::process::exit(2);
        }
    };

    let raw = std::fs::read_to_string(&cli.payload_file).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "cannot read payload file {}: {e}",
                cli.payload_file.display()
            ),
        )
    })?;
    let context_payload: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("payload file is not valid JSON: {e}"),
        )
    })?;

    let request = RepairContextRequest {
        operator_principal_id: cli.operator,
        workflow_instance_id: cli.instance_id,
        context_payload,
        reason: cli.reason,
        repair_source: cli.repair_source,
    };

    let outcome = if cli.apply {
        apply_repair_context(pool, request).await
    } else {
        plan_repair_context(pool, request).await
    };

    match outcome {
        Ok(outcome) => {
            println!("{}", serde_json::to_string_pretty(&outcome)?);
            if outcome.applied {
                eprintln!("REPAIR APPLIED: revision appended and audit recorded");
            } else {
                eprintln!("DRY-RUN: no data written (re-run with --apply to commit)");
            }
            Ok(())
        }
        Err(error) => {
            eprintln!("REPAIR FAILED: {error}");
            std::process::exit(3);
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
