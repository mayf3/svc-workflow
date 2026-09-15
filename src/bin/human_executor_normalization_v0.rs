//! Exact-plan-bound HUMAN executor normalization V0.
//!
//! This offline operator is deliberately closed: it embeds one accepted
//! 20-row plan and has no row, owner, workflow, or plan-path inputs.

use chrono::{SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use std::{env, process};
use uuid::Uuid;

const SPEC_ID: &str = "SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V0";
const PLAN_SHA: &str = "b349e203c00ac82e286666a89dbedd6a17f77e0221090a1a9f2db51d8a253199";
const TARGET_HUMAN: &str = "8902db0d-429a-4e37-985c-f8b92d4b78fb";
const TARGET_COUNT: usize = 20;
const COMMAND_TYPE: &str = "HUMAN_EXECUTOR_NORMALIZATION_V0";
const EVENT_TYPE: &str = "HUMAN_EXECUTOR_NORMALIZED";
const AUDIT_ACTION: &str = "HUMAN_EXECUTOR_NORMALIZATION_V0_COMMITTED";
const PLAN_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/evidence/human-executor-normalization-v0/exact-20-plan.tsv"
));

#[derive(Debug)]
struct Error(String);
type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self(format!("database conflict: {value}"))
    }
}

fn conflict(message: impl Into<String>) -> Error {
    Error(format!("CONFLICT: {}", message.into()))
}
fn outcome_unknown(message: impl Into<String>) -> Error {
    Error(format!("OUTCOME_UNKNOWN: {}", message.into()))
}

#[derive(Clone, Copy)]
enum Mode {
    Plan,
    Apply,
    Verify,
}

#[derive(Clone, Debug, Deserialize)]
struct PlanRow {
    workflow_id: Uuid,
    current_visit_id: Uuid,
    expected_state_version: i32,
    expected_current_assignee: Uuid,
    target_human_principal: Uuid,
    target_visit_id: Uuid,
    definition_version_id: Uuid,
    node_id: Uuid,
    visit_number: i32,
    current_context_revision_id: Uuid,
    context_payload_digest: String,
    definition_key: String,
    node_key: String,
    node_type: String,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let message = error.to_string();
        let outcome = if message.starts_with("OUTCOME_UNKNOWN:") {
            "OUTCOME_UNKNOWN"
        } else {
            "CONFLICT"
        };
        println!("{}", json!({"outcome":outcome,"writes":0,"error":message}));
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let mode = parse_args()?;
    let rows = parse_plan()?;
    let database_url =
        env::var("DATABASE_URL").map_err(|_| conflict("DATABASE_URL is required"))?;
    let database_name = env::var("NORMALIZATION_DATABASE_NAME")
        .map_err(|_| conflict("NORMALIZATION_DATABASE_NAME is required"))?;
    let actor = env::var("NORMALIZATION_ACTOR_PRINCIPAL_ID")
        .map_err(|_| conflict("NORMALIZATION_ACTOR_PRINCIPAL_ID is required"))?
        .parse::<Uuid>()
        .map_err(|_| conflict("NORMALIZATION_ACTOR_PRINCIPAL_ID must be UUID"))?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;
    verify_database_name(&pool, &database_name).await?;

    match mode {
        Mode::Plan => plan(&pool, &rows, actor, &database_name).await,
        Mode::Verify => {
            verify_terminal(&pool, &rows, actor, &database_name).await?;
            print_outcome("VERIFIED", 0);
            Ok(())
        }
        Mode::Apply => apply(&pool, &rows, actor, &database_name).await,
    }
}

fn parse_args() -> Result<Mode> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [arg] if arg == "--plan" => Ok(Mode::Plan),
        [arg] if arg == "--apply" => Ok(Mode::Apply),
        [arg] if arg == "--verify" => Ok(Mode::Verify),
        _ => Err(conflict(
            "use only --plan, --apply, or --verify; paths, IDs, and subsets are forbidden",
        )),
    }
}

fn parse_plan() -> Result<Vec<PlanRow>> {
    if hex::encode(Sha256::digest(PLAN_BYTES)) != PLAN_SHA {
        return Err(conflict(
            "embedded plan SHA-256 differs from accepted authority",
        ));
    }
    let text =
        std::str::from_utf8(PLAN_BYTES).map_err(|_| conflict("embedded plan is not UTF-8"))?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| conflict("embedded plan is empty"))?;
    if header != "workflow_id\tcurrent_visit_id\texpected_state_version\texpected_current_assignee\ttarget_human_principal\ttarget_visit_id\tdefinition_version_id\tnode_id\tvisit_number\tcurrent_context_revision_id\tcontext_payload_digest\tdefinition_key\tnode_key\tnode_type" {
        return Err(conflict("embedded plan header differs from exact schema"));
    }
    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        let columns: Vec<&str> = line.split('\t').collect();
        if columns.len() != 14 {
            return Err(conflict(format!(
                "plan row {} has wrong column count",
                index + 1
            )));
        }
        let uuid = |value: &str| {
            value
                .parse::<Uuid>()
                .map_err(|_| conflict(format!("plan row {} has invalid UUID", index + 1)))
        };
        rows.push(PlanRow {
            workflow_id: uuid(columns[0])?,
            current_visit_id: uuid(columns[1])?,
            expected_state_version: columns[2]
                .parse()
                .map_err(|_| conflict("invalid state version"))?,
            expected_current_assignee: uuid(columns[3])?,
            target_human_principal: uuid(columns[4])?,
            target_visit_id: uuid(columns[5])?,
            definition_version_id: uuid(columns[6])?,
            node_id: uuid(columns[7])?,
            visit_number: columns[8]
                .parse()
                .map_err(|_| conflict("invalid visit number"))?,
            current_context_revision_id: uuid(columns[9])?,
            context_payload_digest: columns[10].to_string(),
            definition_key: columns[11].to_string(),
            node_key: columns[12].to_string(),
            node_type: columns[13].to_string(),
        });
    }
    let target: Uuid = TARGET_HUMAN.parse().expect("compiled target UUID");
    let workflows: std::collections::HashSet<_> = rows.iter().map(|row| row.workflow_id).collect();
    let sources: std::collections::HashSet<_> =
        rows.iter().map(|row| row.current_visit_id).collect();
    let targets: std::collections::HashSet<_> =
        rows.iter().map(|row| row.target_visit_id).collect();
    if rows.len() != TARGET_COUNT
        || workflows.len() != TARGET_COUNT
        || sources.len() != TARGET_COUNT
        || targets.len() != TARGET_COUNT
    {
        return Err(conflict(
            "plan is not exactly 20 unique workflow/source/target rows",
        ));
    }
    if rows.iter().any(|row| {
        row.target_human_principal != target
            || row.expected_state_version < 1
            || row.visit_number < 1
            || row.context_payload_digest.len() != 64
            || row.node_type == "TERMINAL"
    }) {
        return Err(conflict("plan target or active-row invariants differ"));
    }
    rows.sort_by_key(|row| row.workflow_id);
    Ok(rows)
}

async fn verify_database_name(pool: &PgPool, expected: &str) -> Result<()> {
    let actual: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(pool)
        .await?;
    if actual != expected || expected.is_empty() {
        return Err(conflict(format!(
            "database identity mismatch: expected={expected} actual={actual}"
        )));
    }
    Ok(())
}

fn group_identity(actor: Uuid, database_name: &str) -> String {
    format!(
        "{SPEC_ID}:{PLAN_SHA}:{}:{database_name}:{actor}:{TARGET_HUMAN}",
        env!("GIT_SHA")
    )
}

fn deterministic(group: &str, label: &str) -> Uuid {
    let mut bytes: [u8; 16] = Sha256::digest(format!("{group}:{label}").as_bytes())[..16]
        .try_into()
        .expect("SHA prefix");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn digest(value: &Value) -> Result<String> {
    let source = serde_json::to_string(value).map_err(|error| conflict(error.to_string()))?;
    let canonical =
        jcs_canonicalize::canonicalize(&source).map_err(|error| conflict(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(canonical.as_bytes())))
}

fn idempotency_key(group: &str, row: &PlanRow) -> String {
    format!(
        "human-normalization-v0:{}:{}",
        &hex::encode(Sha256::digest(group.as_bytes()))[..24],
        row.workflow_id
    )
}

fn row_request(row: &PlanRow, actor: Uuid) -> Value {
    json!({
        "specId":SPEC_ID,"planSha256":PLAN_SHA,"implementationSha":env!("GIT_SHA"),
        "actorPrincipalId":actor,"targetHumanPrincipal":TARGET_HUMAN,"workflowId":row.workflow_id,
        "sourceVisitId":row.current_visit_id,"targetVisitId":row.target_visit_id,
        "expectedStateVersion":row.expected_state_version
    })
}

fn row_response(row: &PlanRow, command: Uuid, event: Uuid) -> Value {
    json!({"outcome":"COMMITTED","commandId":command,"eventId":event,"workflowId":row.workflow_id,
        "targetVisitId":row.target_visit_id,"newStateVersion":row.expected_state_version + 1})
}

fn audit_details(database_name: &str) -> Value {
    json!({
        "specId": SPEC_ID,"planSha256": PLAN_SHA,"implementationSha": env!("GIT_SHA"),
        "database": database_name,"targetHumanPrincipal": TARGET_HUMAN,
        "workflowCount": TARGET_COUNT,"businessStateUnchanged": true,"historyRewritten": false
    })
}

async fn require_principals<'a, E>(executor: E, actor: Uuid) -> Result<()>
where
    E: sqlx::Executor<'a, Database = Postgres>,
{
    let human: Uuid = TARGET_HUMAN.parse().expect("compiled HUMAN UUID");
    let rows = sqlx::query("SELECT principal_id,principal_type::text AS principal_type,enabled FROM principals WHERE principal_id=ANY($1) ORDER BY principal_id FOR UPDATE")
        .bind(&[actor, human][..])
        .fetch_all(executor)
        .await?;
    if rows.len() != 2 || actor == human {
        return Err(conflict("operator or exact HUMAN principal is missing"));
    }
    for row in rows {
        let id: Uuid = row.get("principal_id");
        let principal_type: String = row.get("principal_type");
        let enabled: bool = row.get("enabled");
        if !enabled
            || (id == human && principal_type != "HUMAN")
            || (id == actor && principal_type != "AGENT")
        {
            return Err(conflict(
                "operator/HUMAN principal type or status conflicts",
            ));
        }
    }
    Ok(())
}

async fn plan(pool: &PgPool, rows: &[PlanRow], actor: Uuid, database_name: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    require_principals(&mut *tx, actor).await?;
    let group = group_identity(actor, database_name);
    let artifact_count = artifact_count(&mut *tx, rows, actor, &group).await?;
    if artifact_count == 0 {
        prevalidate(&mut tx, rows).await?;
        tx.rollback().await?;
        print_outcome("READY", 0);
        Ok(())
    } else {
        tx.rollback().await?;
        verify_terminal(pool, rows, actor, database_name).await?;
        print_outcome("NOOP", 0);
        Ok(())
    }
}

async fn artifact_count<'a, E>(
    executor: E,
    rows: &[PlanRow],
    actor: Uuid,
    group: &str,
) -> Result<i64>
where
    E: sqlx::Executor<'a, Database = Postgres>,
{
    let commands: Vec<Uuid> = rows
        .iter()
        .map(|row| deterministic(group, &format!("command:{}", row.workflow_id)))
        .collect();
    let events: Vec<Uuid> = rows
        .iter()
        .map(|row| deterministic(group, &format!("event:{}", row.workflow_id)))
        .collect();
    let keys: Vec<String> = rows.iter().map(|row| idempotency_key(group, row)).collect();
    let workflows: Vec<Uuid> = rows.iter().map(|row| row.workflow_id).collect();
    let sequences: Vec<i32> = rows
        .iter()
        .map(|row| row.expected_state_version + 1)
        .collect();
    let visits: Vec<Uuid> = rows.iter().map(|row| row.target_visit_id).collect();
    let audit = deterministic(group, "group-audit");
    let count: i64 = sqlx::query_scalar(
        "SELECT
          (SELECT count(*) FROM workflow_node_visits WHERE node_visit_id=ANY($1)) +
          (SELECT count(*) FROM workflow_command_receipts
             WHERE command_id=ANY($2) OR (principal_id=$3 AND idempotency_key=ANY($4))) +
          (SELECT count(*) FROM workflow_events
             WHERE event_id=ANY($5) OR command_id=ANY($2)
                OR (workflow_instance_id,event_sequence) IN (SELECT * FROM unnest($6::uuid[],$7::integer[]))) +
          (SELECT count(*) FROM workflow_security_audits WHERE audit_id=$8)",
    )
    .bind(&visits)
    .bind(&commands)
    .bind(actor)
    .bind(&keys)
    .bind(&events)
    .bind(&workflows)
    .bind(&sequences)
    .bind(audit)
    .fetch_one(executor)
    .await?;
    Ok(count)
}

async fn prevalidate(tx: &mut Transaction<'_, Postgres>, rows: &[PlanRow]) -> Result<()> {
    for row in rows {
        let state = sqlx::query(
            "SELECT wi.definition_version_id,wi.current_context_revision_id,wi.current_node_visit_id,
                    wi.workflow_state_version,wi.cancelled,wi.archived_at,wi.semantic_model_version,
                    v.node_id,v.visit_number,v.assignee_principal_id,
                    c.payload_digest,d.definition_key,n.node_key,n.node_type::text AS node_type
             FROM workflow_instances wi
             JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id
             JOIN workflow_context_revisions c ON c.context_revision_id=wi.current_context_revision_id
             JOIN workflow_definition_versions dv ON dv.definition_version_id=wi.definition_version_id
             JOIN workflow_definitions d ON d.workflow_definition_id=dv.workflow_definition_id
             JOIN workflow_node_definitions n ON n.node_id=v.node_id AND n.definition_version_id=wi.definition_version_id
             WHERE wi.workflow_instance_id=$1 FOR UPDATE OF wi,v"
        )
        .bind(row.workflow_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| conflict(format!("workflow {} is missing", row.workflow_id)))?;
        let archived: Option<chrono::DateTime<Utc>> = state.try_get("archived_at").ok();
        if state.get::<Uuid, _>("definition_version_id") != row.definition_version_id
            || state.get::<Uuid, _>("current_context_revision_id")
                != row.current_context_revision_id
            || state.get::<Uuid, _>("current_node_visit_id") != row.current_visit_id
            || state.get::<i32, _>("workflow_state_version") != row.expected_state_version
            || state.get::<bool, _>("cancelled")
            || archived.is_some()
            || state.get::<i16, _>("semantic_model_version") != 1
            || state.get::<Uuid, _>("node_id") != row.node_id
            || state.get::<i32, _>("visit_number") != row.visit_number
            || state.get::<Uuid, _>("assignee_principal_id") != row.expected_current_assignee
            || state.get::<String, _>("payload_digest") != row.context_payload_digest
            || state.get::<String, _>("definition_key") != row.definition_key
            || state.get::<String, _>("node_key") != row.node_key
            || state.get::<String, _>("node_type") != row.node_type
        {
            return Err(conflict(format!(
                "workflow {} preimage drifted",
                row.workflow_id
            )));
        }
        let assistance: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_assistance_cases WHERE node_visit_id=$1 AND status IN ('OWNER_PENDING','HUMAN_REQUIRED')")
            .bind(row.current_visit_id)
            .fetch_one(&mut **tx)
            .await?;
        if assistance != 0 {
            return Err(conflict(format!(
                "workflow {} has open assistance",
                row.workflow_id
            )));
        }
        let collision: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_node_visits WHERE node_visit_id=$1 OR (workflow_instance_id=$2 AND node_id=$3 AND visit_number=$4)")
            .bind(row.target_visit_id)
            .bind(row.workflow_id)
            .bind(row.node_id)
            .bind(row.visit_number + 1)
            .fetch_one(&mut **tx)
            .await?;
        if collision != 0 {
            return Err(conflict(format!(
                "workflow {} target Visit collides",
                row.workflow_id
            )));
        }
    }
    Ok(())
}

async fn apply(pool: &PgPool, rows: &[PlanRow], actor: Uuid, database_name: &str) -> Result<()> {
    let group = group_identity(actor, database_name);
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(&group)
        .execute(&mut *tx)
        .await?;
    require_principals(&mut *tx, actor).await?;
    let artifacts = artifact_count(&mut *tx, rows, actor, &group).await?;
    if artifacts != 0 {
        tx.rollback().await?;
        verify_terminal(pool, rows, actor, database_name).await?;
        print_outcome("NOOP", 0);
        return Ok(());
    }
    prevalidate(&mut tx, rows).await?;

    for (index, row) in rows.iter().enumerate() {
        write_row(&mut tx, row, index + 1, actor, &group).await?;
    }
    let audit_id = deterministic(&group, "group-audit");
    let details = audit_details(database_name);
    sqlx::query("INSERT INTO workflow_security_audits(audit_id,principal_id,action,resource_type,resource_id,details) VALUES($1,$2,$3,'HUMAN_EXECUTOR_NORMALIZATION_PLAN',$4,$5)")
        .bind(audit_id).bind(actor).bind(AUDIT_ACTION).bind(PLAN_SHA).bind(details)
        .execute(&mut *tx).await?;

    let mut commit_error = tx.commit().await.err().map(|error| error.to_string());
    #[cfg(debug_assertions)]
    if commit_error.is_none()
        && database_name.starts_with("human_normalization_")
        && env::var("NORMALIZATION_TEST_SIMULATE_COMMIT_ACK_LOSS").as_deref() == Ok("1")
    {
        commit_error = Some("simulated commit acknowledgement loss".to_string());
    }
    if let Some(error) = commit_error {
        return match verify_terminal(pool, rows, actor, database_name).await {
            Ok(()) => {
                print_outcome("APPLIED", 0);
                Ok(())
            }
            Err(read_error) => Err(outcome_unknown(format!(
                "commit failed: {error}; reconciliation failed: {read_error}"
            ))),
        };
    }
    verify_terminal(pool, rows, actor, database_name).await?;
    print_outcome("APPLIED", TARGET_COUNT * 4 + 1);
    Ok(())
}

async fn write_row(
    tx: &mut Transaction<'_, Postgres>,
    row: &PlanRow,
    index: usize,
    actor: Uuid,
    group: &str,
) -> Result<()> {
    let command = deterministic(group, &format!("command:{}", row.workflow_id));
    let event = deterministic(group, &format!("event:{}", row.workflow_id));
    let key = idempotency_key(group, row);
    let request = row_request(row, actor);
    let request_hash = digest(&request)?;
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status) VALUES($1,$2,$3,$4,$5,'PROCESSING')")
        .bind(command).bind(actor).bind(&key).bind(COMMAND_TYPE).bind(&request_hash)
        .execute(&mut **tx).await?;
    sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,$4,$5,NULL)")
        .bind(row.target_visit_id).bind(row.workflow_id).bind(row.node_id).bind(row.visit_number + 1)
        .bind(row.target_human_principal).execute(&mut **tx).await?;
    let next_version = row.expected_state_version + 1;
    let update = sqlx::query("UPDATE workflow_instances SET current_node_visit_id=$1,workflow_state_version=$2,updated_at=now() WHERE workflow_instance_id=$3 AND current_node_visit_id=$4 AND current_context_revision_id=$5 AND workflow_state_version=$6 AND definition_version_id=$7 AND NOT cancelled AND archived_at IS NULL")
        .bind(row.target_visit_id).bind(next_version).bind(row.workflow_id).bind(row.current_visit_id)
        .bind(row.current_context_revision_id).bind(row.expected_state_version).bind(row.definition_version_id)
        .execute(&mut **tx).await?;
    if update.rows_affected() != 1 {
        return Err(conflict(format!("workflow {} CAS failed", row.workflow_id)));
    }
    let occurred_at = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true);
    let event_data = json!({
        "specId":SPEC_ID,"planSha256":PLAN_SHA,"implementationSha":env!("GIT_SHA"),"rowIndex":index,
        "workflowInstanceId":row.workflow_id,"sourceNodeVisitId":row.current_visit_id,
        "targetNodeVisitId":row.target_visit_id,"oldPrincipalId":row.expected_current_assignee,
        "newPrincipalId":row.target_human_principal,"nodeId":row.node_id,
        "oldWorkflowStateVersion":row.expected_state_version,"newWorkflowStateVersion":next_version,
        "businessStateUnchanged":true,"humanActionFabricated":false,"occurredAt":occurred_at
    });
    let event_digest = digest(&event_data)?;
    sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,command_id,event_type,transition_effect,source_node_visit_id,target_node_visit_id,context_revision_id,submission_id,event_data,event_data_digest,actor_principal_id,from_node_id,to_node_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,$3,'v1',$4,$5,NULL,$6,$7,$8,NULL,$9,$10,$11,$12,$12,$13,$3)")
        .bind(event).bind(row.workflow_id).bind(next_version).bind(command).bind(EVENT_TYPE)
        .bind(row.current_visit_id).bind(row.target_visit_id).bind(row.current_context_revision_id)
        .bind(&event_data).bind(&event_digest).bind(actor).bind(row.node_id).bind(row.expected_state_version)
        .execute(&mut **tx).await?;
    let response = row_response(row, command, event);
    let response_digest = digest(&response)?;
    let completed = sqlx::query("UPDATE workflow_command_receipts SET receipt_status='COMPLETED',response_status=200,response_body=$2,response_digest=$3,completed_at=now() WHERE command_id=$1 AND receipt_status='PROCESSING'")
        .bind(command).bind(response).bind(response_digest).execute(&mut **tx).await?;
    if completed.rows_affected() != 1 {
        return Err(conflict("receipt completion failed"));
    }
    Ok(())
}

async fn verify_terminal(
    pool: &PgPool,
    rows: &[PlanRow],
    actor: Uuid,
    database_name: &str,
) -> Result<()> {
    let group = group_identity(actor, database_name);
    require_principals(pool, actor).await?;
    for row in rows {
        let command = deterministic(&group, &format!("command:{}", row.workflow_id));
        let event = deterministic(&group, &format!("event:{}", row.workflow_id));
        let exact: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_instances wi
             JOIN workflow_node_visits target ON target.node_visit_id=wi.current_node_visit_id
             JOIN workflow_node_visits source ON source.node_visit_id=$2
             JOIN workflow_context_revisions c ON c.context_revision_id=wi.current_context_revision_id
             JOIN workflow_definition_versions dv ON dv.definition_version_id=wi.definition_version_id
             JOIN workflow_definitions d ON d.workflow_definition_id=dv.workflow_definition_id
             JOIN workflow_node_definitions n ON n.node_id=target.node_id AND n.definition_version_id=wi.definition_version_id
             WHERE wi.workflow_instance_id=$1 AND wi.current_node_visit_id=$3 AND wi.workflow_state_version=$4
               AND wi.current_context_revision_id=$5 AND wi.definition_version_id=$6 AND NOT wi.cancelled
               AND wi.archived_at IS NULL AND wi.semantic_model_version=1
               AND target.workflow_instance_id=$1 AND target.node_id=$7 AND target.visit_number=$8
               AND target.assignee_principal_id=$9 AND target.entered_by_transition_id IS NULL
               AND source.workflow_instance_id=$1 AND source.node_id=$7 AND source.visit_number=$10
               AND source.assignee_principal_id=$11 AND c.payload_digest=$12
               AND d.definition_key=$20 AND n.node_key=$21 AND n.node_type::text=$22
               AND EXISTS (SELECT 1 FROM workflow_command_receipts r WHERE r.command_id=$13 AND r.principal_id=$14 AND r.idempotency_key=$15 AND r.command_type=$16 AND r.receipt_status='COMPLETED' AND r.response_status=200)
               AND EXISTS (SELECT 1 FROM workflow_events e WHERE e.event_id=$17 AND e.command_id=$13 AND e.workflow_instance_id=$1 AND e.event_type=$18 AND e.transition_effect IS NULL AND e.source_node_visit_id=$2 AND e.target_node_visit_id=$3 AND e.context_revision_id=$5 AND e.submission_id IS NULL AND e.actor_principal_id=$14 AND e.from_node_id=$7 AND e.to_node_id=$7 AND e.old_workflow_state_version=$19 AND e.new_workflow_state_version=$4)"
        )
        .bind(row.workflow_id).bind(row.current_visit_id).bind(row.target_visit_id)
        .bind(row.expected_state_version + 1).bind(row.current_context_revision_id).bind(row.definition_version_id)
        .bind(row.node_id).bind(row.visit_number + 1).bind(row.target_human_principal).bind(row.visit_number)
        .bind(row.expected_current_assignee).bind(&row.context_payload_digest).bind(command).bind(actor)
        .bind(idempotency_key(&group, row)).bind(COMMAND_TYPE).bind(event).bind(EVENT_TYPE).bind(row.expected_state_version)
        .bind(&row.definition_key).bind(&row.node_key).bind(&row.node_type)
        .fetch_one(pool).await?;
        if exact != 1 {
            return Err(conflict(format!(
                "workflow {} terminal state is not exact",
                row.workflow_id
            )));
        }
        let receipt = sqlx::query("SELECT request_hash,response_body,response_digest FROM workflow_command_receipts WHERE command_id=$1")
            .bind(command).fetch_one(pool).await?;
        let expected_response = row_response(row, command, event);
        if receipt.get::<String, _>("request_hash") != digest(&row_request(row, actor))?
            || receipt.get::<Value, _>("response_body") != expected_response
            || receipt.get::<String, _>("response_digest") != digest(&expected_response)?
        {
            return Err(conflict(format!(
                "workflow {} receipt payload is not exact",
                row.workflow_id
            )));
        }
        let event_row = sqlx::query(
            "SELECT event_data,event_data_digest FROM workflow_events WHERE event_id=$1",
        )
        .bind(event)
        .fetch_one(pool)
        .await?;
        let event_data: Value = event_row.get("event_data");
        let fixed = json!({
            "specId":SPEC_ID,"planSha256":PLAN_SHA,"implementationSha":env!("GIT_SHA"),
            "workflowInstanceId":row.workflow_id,"sourceNodeVisitId":row.current_visit_id,
            "targetNodeVisitId":row.target_visit_id,"oldPrincipalId":row.expected_current_assignee,
            "newPrincipalId":row.target_human_principal,"nodeId":row.node_id,
            "oldWorkflowStateVersion":row.expected_state_version,
            "newWorkflowStateVersion":row.expected_state_version + 1,
            "businessStateUnchanged":true,"humanActionFabricated":false
        });
        let contains_fixed: bool = sqlx::query_scalar("SELECT $1::jsonb @> $2::jsonb")
            .bind(&event_data)
            .bind(&fixed)
            .fetch_one(pool)
            .await?;
        let occurred = event_data
            .get("occurredAt")
            .and_then(Value::as_str)
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok());
        if !contains_fixed
            || occurred.is_none()
            || event_row.get::<String, _>("event_data_digest") != digest(&event_data)?
        {
            return Err(conflict(format!(
                "workflow {} event payload is not exact",
                row.workflow_id
            )));
        }
    }
    let audit = deterministic(&group, "group-audit");
    let audit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_security_audits WHERE audit_id=$1 AND principal_id=$2 AND action=$3 AND resource_type='HUMAN_EXECUTOR_NORMALIZATION_PLAN' AND resource_id=$4 AND details=$5")
        .bind(audit).bind(actor).bind(AUDIT_ACTION).bind(PLAN_SHA).bind(audit_details(database_name)).fetch_one(pool).await?;
    if audit_count != 1 {
        return Err(conflict("group audit is missing or conflicting"));
    }
    Ok(())
}

fn print_outcome(outcome: &str, writes: usize) {
    println!(
        "{}",
        json!({
            "outcome":outcome,"writes":writes,"specId":SPEC_ID,"planSha256":PLAN_SHA,
            "planRowCount":TARGET_COUNT,"targetHumanPrincipal":TARGET_HUMAN,
            "implementationSha":env!("GIT_SHA"),"businessStateUnchanged":true,
            "humanActionFabricated":false,"historyRewrite":false
        })
    );
}
