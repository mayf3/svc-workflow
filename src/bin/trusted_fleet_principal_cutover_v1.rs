//! Exact-plan-bound trusted fleet Principal successor cutover.
//! This is intentionally an offline one-shot operator, not a reassignment API.
#![allow(unexpected_cfgs)]
use chrono::{SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use std::{collections::HashSet, env, fs, process};
use svc_workflow::domain::workflow_instance::recovery::{BeforeSnapshotV1, WorkflowProjection};
use svc_workflow::store::postgres::admin_recovery_repository::migration_manifest::{
    TRUSTED_FLEET_PAIRS as PAIRS, TRUSTED_FLEET_PLAN_SHA256 as PLAN_SHA,
};
use uuid::Uuid;

const PLAN_SIZE: usize = 540472;
const PLAN_PATH: &str =
    "/Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json";
const EVENT_TYPE: &str = "PRINCIPAL_SUCCESSOR_MIGRATION_COMMITTED";
const COMMAND_TYPE: &str = "PRINCIPAL_SUCCESSOR_MIGRATION_V1";
const AUDIT_ACTION: &str = "PRINCIPAL_SUCCESSOR_MIGRATION_V1_COMMITTED";
const EXCLUDED_AGENT: &str = "efficiency-agent";
const EXCLUDED_PRINCIPAL: &str = "d09f8849-073c-484a-978c-f375113c28b2";

#[derive(Debug)]
struct Error(String);
type Result<T> = std::result::Result<T, Error>;
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self(format!("database conflict: {e}"))
    }
}
fn conflict(s: impl Into<String>) -> Error {
    Error(format!("CONFLICT: {}", s.into()))
}
fn outcome_error(outcome: &str, s: impl Into<String>) -> Error {
    Error(format!("{outcome}: {}", s.into()))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Plan,
    Apply(Scope),
    Verify,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Scope {
    Build,
    Efficiency,
    Remaining,
}
#[derive(Debug)]
struct Cli {
    mode: Mode,
}

#[derive(Debug, Deserialize)]
struct Plan {
    schema: String,
    artifact_revision: u32,
    mode: String,
    production_change: String,
    summary: Summary,
    fleet_rows: Vec<Pair>,
    domain_tuples: Vec<DomainTuple>,
    current_responsibility_tuples: Vec<Responsibility>,
    creator_owned_draft_tuples: Vec<Value>,
    explicit_exclusions: Vec<Value>,
    excluded_identities: Value,
}
#[derive(Debug, Deserialize)]
struct Summary {
    total_new_agents: usize,
    exact_successor_pair_count: usize,
    ambiguous_count: usize,
    conflict_count: usize,
    new_workflow_projection_missing_count: usize,
    new_workflow_projection_present_count: usize,
    total_domain_owner_transfers: usize,
    total_domain_member_transfers: usize,
    total_active_responsibility_transfers: usize,
    total_draft_ownership_candidates: usize,
}
#[derive(Clone, Debug, Deserialize)]
struct Pair {
    old_agent_id: String,
    new_agent_id: String,
    old_principal_id: Uuid,
    new_principal_id: Uuid,
    old_principal_external_ref: String,
    new_principal_external_ref: String,
    old_principal_status: String,
    new_principal_status: String,
    classification: String,
    evidence: PairEvidence,
}
#[derive(Clone, Debug, Deserialize)]
struct PairEvidence {
    new_workflow_principal_present: bool,
}
#[derive(Clone, Debug, Deserialize)]
struct DomainTuple {
    domain_id: Uuid,
    domain_key: String,
    role: String,
    old_principal_id: Uuid,
    new_principal_id: Uuid,
    enabled: bool,
    migration_candidate: bool,
    before: DomainBefore,
    after: DomainAfter,
}
#[derive(Clone, Debug, Deserialize)]
struct DomainBefore {
    old_enabled: bool,
    new_enabled: bool,
}
#[derive(Clone, Debug, Deserialize)]
struct DomainAfter {
    old_enabled: bool,
    new_enabled: bool,
}
#[derive(Clone, Debug, Deserialize)]
struct Responsibility {
    workflow_instance_id: Uuid,
    domain_id: Uuid,
    current_visit_id: Uuid,
    node_id: Uuid,
    old_principal_id: Uuid,
    new_principal_id: Uuid,
    expected_state_version: i32,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        let message = e.to_string();
        let outcome = message
            .split_once(':')
            .map(|(prefix, _)| prefix)
            .filter(|value| matches!(*value, "CONFLICT" | "ROLLED_BACK" | "OUTCOME_UNKNOWN"))
            .unwrap_or("CONFLICT");
        println!("{}", json!({"outcome":outcome,"writes":0,"error":message}));
        process::exit(1);
    }
}
async fn run() -> Result<()> {
    let cli = parse_args()?;
    let (plan, raw) = load_plan()?;
    let database_url =
        env::var("DATABASE_URL").map_err(|_| conflict("DATABASE_URL is required"))?;
    let auth_url =
        env::var("AUTH_DATABASE_URL").map_err(|_| conflict("AUTH_DATABASE_URL is required"))?;
    let workflow = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await?;
    let auth = PgPoolOptions::new()
        .max_connections(2)
        .connect(&auth_url)
        .await?;
    verify_database_identity(&workflow, &auth).await?;
    let actor = env::var("MIGRATION_ACTOR_PRINCIPAL_ID")
        .ok()
        .map(|s| {
            s.parse::<Uuid>()
                .map_err(|_| conflict("MIGRATION_ACTOR_PRINCIPAL_ID must be UUID"))
        })
        .transpose()?;
    match cli.mode {
        Mode::Plan => observe(&workflow, &auth, &plan, None, false).await,
        Mode::Verify => observe(&workflow, &auth, &plan, None, true).await,
        Mode::Apply(scope) => {
            apply_scope(
                &workflow,
                &auth,
                &plan,
                scope,
                actor.ok_or_else(|| {
                    conflict("MIGRATION_ACTOR_PRINCIPAL_ID is required for apply")
                })?,
                &raw,
            )
            .await
        }
    }
}
#[cfg(not(trusted_fleet_cutover_conformance))]
async fn verify_database_identity(workflow: &PgPool, auth: &PgPool) -> Result<()> {
    let workflow_name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(workflow)
        .await?;
    let auth_name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(auth)
        .await?;
    if workflow_name != "svc_workflow_dogfood_clean" || auth_name != "agent_dev_center" {
        return Err(conflict(format!(
            "database identity mismatch: workflow={workflow_name}, auth={auth_name}"
        )));
    }
    Ok(())
}
#[cfg(trusted_fleet_cutover_conformance)]
async fn verify_database_identity(_workflow: &PgPool, _auth: &PgPool) -> Result<()> {
    Ok(())
}

fn parse_args() -> Result<Cli> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = match args.as_slice() {
        [flag] if flag == "--plan" => Mode::Plan,
        [flag] if flag == "--verify" => Mode::Verify,
        [apply, scope_flag, scope] if apply == "--apply" && scope_flag == "--scope" => {
            Mode::Apply(match scope.as_str() {
                "build-in-public-canary" => Scope::Build,
                "efficiency-canary" => Scope::Efficiency,
                "remaining-fleet" => Scope::Remaining,
                _ => return Err(conflict("invalid closed --scope")),
            })
        }
        _ => return Err(conflict("use only --plan, --verify, or --apply --scope <closed-scope>; arbitrary IDs, paths, and subsets are forbidden")),
    };
    Ok(Cli { mode })
}
fn load_plan() -> Result<(Plan, Vec<u8>)> {
    let raw = fs::read(PLAN_PATH)?;
    if raw.len() != PLAN_SIZE || hex::encode(Sha256::digest(&raw)) != PLAN_SHA {
        return Err(conflict("plan bytes do not match frozen size/SHA-256"));
    }
    let root: Value =
        serde_json::from_slice(&raw).map_err(|e| conflict(format!("invalid plan JSON: {e}")))?;
    let required = [
        "artifact_revision",
        "authority_recommendation",
        "broker_4xx_diagnosis",
        "classification_rules",
        "creator_owned_draft_tuples",
        "current_responsibility_tuples",
        "domain_tuples",
        "excluded_identities",
        "explicit_exclusions",
        "fleet_rows",
        "history_visibility",
        "mode",
        "production_change",
        "schema",
        "snapshot_observed_at_utc",
        "sources",
        "summary",
        "supersession",
    ];
    let obj = root
        .as_object()
        .ok_or_else(|| conflict("plan root must be object"))?;
    if obj.len() != required.len() || required.iter().any(|k| !obj.contains_key(*k)) {
        return Err(conflict("plan root has non-exact shape"));
    }
    let p: Plan = serde_json::from_value(root).map_err(|e| conflict(format!("plan shape: {e}")))?;
    validate_plan(&p)?;
    Ok((p, raw))
}
fn validate_plan(p: &Plan) -> Result<()> {
    if p.schema != "workflow_trusted_fleet_successor_plan_v2"
        || p.artifact_revision != 2
        || p.mode != "READ_ONLY_CANONICAL_PLAN"
        || p.production_change != "NONE"
    {
        return Err(conflict("plan identity fields drifted"));
    }
    let s = &p.summary;
    if (
        s.total_new_agents,
        s.exact_successor_pair_count,
        s.ambiguous_count,
        s.conflict_count,
        s.new_workflow_projection_missing_count,
        s.new_workflow_projection_present_count,
        s.total_domain_owner_transfers,
        s.total_domain_member_transfers,
        s.total_active_responsibility_transfers,
        s.total_draft_ownership_candidates,
    ) != (86, 86, 0, 0, 85, 1, 8, 752, 80, 99)
        || p.fleet_rows.len() != 86
        || p.domain_tuples.len() != 760
        || p.current_responsibility_tuples.len() != 80
        || p.creator_owned_draft_tuples.len() != 99
    {
        return Err(conflict("plan counts drifted"));
    }
    let mut ids = HashSet::new();
    for (i, (f, c)) in p.fleet_rows.iter().zip(PAIRS).enumerate() {
        if f.classification != "EXACT_SUCCESSOR_PAIR"
            || f.old_agent_id != c.0
            || f.new_agent_id != c.1
            || f.old_principal_id.to_string() != c.2
            || f.new_principal_id.to_string() != c.3
            || !ids.insert((f.old_principal_id, f.new_principal_id))
        {
            return Err(conflict(format!(
                "pair {} differs from compiled allowlist",
                i + 1
            )));
        }
    }
    if p.domain_tuples
        .iter()
        .filter(|x| x.role == "DOMAIN_OWNER")
        .count()
        != 8
        || p.domain_tuples
            .iter()
            .filter(|x| x.role == "DOMAIN_MEMBER")
            .count()
            != 752
        || p.domain_tuples.iter().any(|x| {
            !x.enabled
                || !x.migration_candidate
                || !x.before.old_enabled
                || x.before.new_enabled
                || x.after.old_enabled
                || !x.after.new_enabled
        })
    {
        return Err(conflict("domain tuple shape/state invalid"));
    }
    let pair_ids: HashSet<(Uuid, Uuid)> = p
        .fleet_rows
        .iter()
        .map(|pair| (pair.old_principal_id, pair.new_principal_id))
        .collect();
    if p.domain_tuples
        .iter()
        .any(|tuple| !pair_ids.contains(&(tuple.old_principal_id, tuple.new_principal_id)))
    {
        return Err(conflict("domain tuple is outside exact pair allowlist"));
    }
    let mut instances = HashSet::new();
    if p.current_responsibility_tuples.iter().any(|tuple| {
        !pair_ids.contains(&(tuple.old_principal_id, tuple.new_principal_id))
            || tuple.expected_state_version < 1
            || !instances.insert(tuple.workflow_instance_id)
    }) {
        return Err(conflict(
            "responsibility tuple is invalid, duplicate, or outside allowlist",
        ));
    }
    if p.creator_owned_draft_tuples
        .iter()
        .any(|tuple| tuple.get("migration_candidate").and_then(Value::as_bool) != Some(false))
    {
        return Err(conflict("draft creator tuple attempted migration"));
    }
    if p.explicit_exclusions.len() != 8 {
        return Err(conflict("explicit exclusions count drifted"));
    }
    let excluded_records = p
        .excluded_identities
        .get("records")
        .and_then(Value::as_array)
        .ok_or_else(|| conflict("excluded identity records missing"))?;
    if excluded_records.len() != 1
        || excluded_records[0].get("agent_id").and_then(Value::as_str) != Some(EXCLUDED_AGENT)
        || excluded_records[0]
            .get("principal_id")
            .and_then(Value::as_str)
            != Some(EXCLUDED_PRINCIPAL)
        || excluded_records[0]
            .get("future_operator_writes")
            .and_then(Value::as_i64)
            != Some(0)
    {
        return Err(conflict("excluded identity is not exact zero-write record"));
    }
    Ok(())
}
fn selected(scope: Scope, p: &Pair) -> bool {
    match scope {
        Scope::Build => p.new_agent_id == "agt_build-in-public-agent",
        Scope::Efficiency => p.new_agent_id == "agt_efficiency-agent",
        Scope::Remaining => {
            p.new_agent_id != "agt_build-in-public-agent"
                && p.new_agent_id != "agt_efficiency-agent"
        }
    }
}
async fn auth_exact(auth: &PgPool, p: &Pair) -> Result<String> {
    let rows=sqlx::query("SELECT id, principal_type::text AS principal_type, agent_id, external_ref, status::text AS status, display_name FROM machine_principals WHERE id = ANY($1) ORDER BY id").bind(&[p.old_principal_id,p.new_principal_id][..]).fetch_all(auth).await?;
    if rows.len() != 2 {
        return Err(conflict(format!(
            "Auth pair missing for {}",
            p.new_agent_id
        )));
    }
    let mut new_name = None;
    for r in rows {
        let id: Uuid = r.get("id");
        let principal_type: String = r.get("principal_type");
        let agent: Option<String> = r.try_get("agent_id").ok();
        let ext: Option<String> = r.try_get("external_ref").ok();
        let status: String = r.get("status");
        let name: Option<String> = r.try_get("display_name").ok();
        let (ea, ee, es) = if id == p.old_principal_id {
            (
                &p.old_agent_id,
                &p.old_principal_external_ref,
                &p.old_principal_status,
            )
        } else if id == p.new_principal_id {
            new_name = name.clone();
            (
                &p.new_agent_id,
                &p.new_principal_external_ref,
                &p.new_principal_status,
            )
        } else {
            return Err(conflict("Auth returned unplanned UUID"));
        };
        if principal_type != "agent"
            || agent.as_deref() != Some(ea)
            || ext.as_deref().unwrap_or("") != ee
            || status != *es
        {
            return Err(conflict(format!("Auth identity drift for {id}")));
        }
    }
    let n = new_name
        .ok_or_else(|| conflict(format!("Auth display_name missing for {}", p.new_agent_id)))?;
    if n.is_empty() || n.chars().count() > 256 {
        return Err(conflict("Auth display_name invalid"));
    }
    Ok(n)
}
async fn projection_exact(w: &PgPool, pair: &Pair, display: &str) -> Result<bool> {
    let row = sqlx::query("SELECT principal_type::text AS t,display_name,enabled FROM principals WHERE principal_id=$1")
        .bind(pair.new_principal_id)
        .fetch_optional(w)
        .await?;
    Ok(row.as_ref().is_some_and(|r| {
        r.get::<String, _>("t") == "AGENT"
            && r.get::<String, _>("display_name") == display
            && r.get::<bool, _>("enabled")
    }))
}

async fn domain_tuple_state(w: &PgPool, tuple: &DomainTuple) -> Result<&'static str> {
    let row = sqlx::query(
        "SELECT d.domain_key, d.enabled AS domain_enabled,
         (SELECT enabled FROM domain_role_bindings WHERE domain_id=d.domain_id AND principal_id=$2 AND role_key=$4) AS old_enabled,
         (SELECT enabled FROM domain_role_bindings WHERE domain_id=d.domain_id AND principal_id=$3 AND role_key=$4) AS new_enabled
         FROM domains d WHERE d.domain_id=$1",
    )
    .bind(tuple.domain_id)
    .bind(tuple.old_principal_id)
    .bind(tuple.new_principal_id)
    .bind(&tuple.role)
    .fetch_optional(w)
    .await?
    .ok_or_else(|| conflict(format!("domain {} is missing", tuple.domain_id)))?;
    if row.get::<String, _>("domain_key") != tuple.domain_key
        || !row.get::<bool, _>("domain_enabled")
    {
        return Ok("CONFLICT");
    }
    let old_enabled: Option<bool> = row.try_get("old_enabled").ok();
    let new_enabled: Option<bool> = row.try_get("new_enabled").ok();
    Ok(match (old_enabled, new_enabled) {
        (Some(true), None | Some(false)) => "PRE",
        (Some(false), Some(true)) => "READY",
        _ => "CONFLICT",
    })
}

async fn responsibility_tuple_state(w: &PgPool, tuple: &Responsibility) -> Result<&'static str> {
    let row = sqlx::query(
        "SELECT wi.domain_id, wi.current_node_visit_id, wi.workflow_state_version, wi.cancelled, wi.archived_at,
         v.node_id, v.assignee_principal_id, n.node_type::text AS node_type
         FROM workflow_instances wi
         JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id
         JOIN workflow_node_definitions n ON n.node_id=v.node_id AND n.definition_version_id=wi.definition_version_id
         WHERE wi.workflow_instance_id=$1",
    )
    .bind(tuple.workflow_instance_id)
    .fetch_optional(w)
    .await?
    .ok_or_else(|| conflict(format!("instance {} is missing", tuple.workflow_instance_id)))?;
    if row.get::<Uuid, _>("domain_id") != tuple.domain_id {
        return Ok("CONFLICT");
    }
    let current_visit: Uuid = row.get("current_node_visit_id");
    let version: i32 = row.get("workflow_state_version");
    let node: Uuid = row.get("node_id");
    let assignee: Option<Uuid> = row.try_get("assignee_principal_id").ok();
    let terminal = row.get::<bool, _>("cancelled")
        || row
            .try_get::<Option<chrono::DateTime<Utc>>, _>("archived_at")
            .ok()
            .flatten()
            .is_some()
        || row.get::<String, _>("node_type") == "TERMINAL";
    if terminal {
        let source_ok: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_node_visits WHERE node_visit_id=$1 AND workflow_instance_id=$2 AND node_id=$3 AND assignee_principal_id=$4",
        )
        .bind(tuple.current_visit_id)
        .bind(tuple.workflow_instance_id)
        .bind(tuple.node_id)
        .bind(tuple.old_principal_id)
        .fetch_one(w)
        .await?;
        return Ok(if source_ok == 1 {
            "TERMINAL"
        } else {
            "CONFLICT"
        });
    }
    if current_visit == tuple.current_visit_id
        && version == tuple.expected_state_version
        && node == tuple.node_id
        && assignee == Some(tuple.old_principal_id)
    {
        return Ok("PRE");
    }
    if version == tuple.expected_state_version + 1
        && node == tuple.node_id
        && assignee == Some(tuple.new_principal_id)
    {
        let event_ok: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1 AND event_type=$2 AND source_node_visit_id=$3 AND target_node_visit_id=$4 AND old_workflow_state_version=$5 AND new_workflow_state_version=$6",
        )
        .bind(tuple.workflow_instance_id)
        .bind(EVENT_TYPE)
        .bind(tuple.current_visit_id)
        .bind(current_visit)
        .bind(tuple.expected_state_version)
        .bind(tuple.expected_state_version + 1)
        .fetch_one(w)
        .await?;
        return Ok(if event_ok == 1 { "READY" } else { "CONFLICT" });
    }
    Ok("CONFLICT")
}

async fn observe(
    w: &PgPool,
    a: &PgPool,
    p: &Plan,
    _scope: Option<Scope>,
    terminal: bool,
) -> Result<()> {
    let mut pairs = Vec::new();
    let mut projection_ready = 0usize;
    let mut owner_ready = 0usize;
    let mut member_ready = 0usize;
    let mut responsibility_ready = 0usize;
    let mut terminal_skipped = 0usize;
    let mut conflicts = 0usize;
    for pair in &p.fleet_rows {
        let mut pair_conflicts = Vec::new();
        let display = match auth_exact(a, pair).await {
            Ok(value) => value,
            Err(error) => {
                pair_conflicts.push(error.to_string());
                String::new()
            }
        };
        let projection = !display.is_empty() && projection_exact(w, pair, &display).await?;
        let projection_present: i64 =
            sqlx::query_scalar("SELECT count(*) FROM principals WHERE principal_id=$1")
                .bind(pair.new_principal_id)
                .fetch_one(w)
                .await?;
        if projection {
            projection_ready += 1;
        } else if projection_present != 0 {
            pair_conflicts
                .push("projection exists but does not exactly match Auth formal fields".into());
        } else if pair.evidence.new_workflow_principal_present {
            pair_conflicts.push("plan-required existing projection is now missing".into());
        } else if terminal {
            pair_conflicts.push("projection is not exact terminal state".into());
        }
        for tuple in p.domain_tuples.iter().filter(|tuple| {
            tuple.old_principal_id == pair.old_principal_id
                && tuple.new_principal_id == pair.new_principal_id
        }) {
            let state = domain_tuple_state(w, tuple).await?;
            if state == "READY" {
                if tuple.role == "DOMAIN_OWNER" {
                    owner_ready += 1;
                } else {
                    member_ready += 1;
                }
            } else if state == "CONFLICT" || terminal {
                if state != "READY" {
                    pair_conflicts.push(format!(
                        "domain tuple {} {} is {state}",
                        tuple.domain_id, tuple.role
                    ));
                }
            }
        }
        for tuple in p.current_responsibility_tuples.iter().filter(|tuple| {
            tuple.old_principal_id == pair.old_principal_id
                && tuple.new_principal_id == pair.new_principal_id
        }) {
            let state = responsibility_tuple_state(w, tuple).await?;
            match state {
                "READY" => responsibility_ready += 1,
                "TERMINAL" => {
                    terminal_skipped += 1;
                    responsibility_ready += 1;
                }
                "CONFLICT" => pair_conflicts.push(format!(
                    "responsibility {} drifted",
                    tuple.workflow_instance_id
                )),
                "PRE" if terminal => pair_conflicts.push(format!(
                    "responsibility {} remains in pre-state",
                    tuple.workflow_instance_id
                )),
                _ => {}
            }
        }
        if !pair_conflicts.is_empty() {
            conflicts += 1;
        }
        pairs.push(json!({"newAgentId":pair.new_agent_id,"outcome":if pair_conflicts.is_empty(){if terminal{"NOOP"}else{"PLANNED"}}else{"CONFLICT"},"projectionExact":projection,"drift":pair_conflicts}));
    }
    let excluded: Uuid = EXCLUDED_PRINCIPAL.parse().expect("compiled excluded UUID");
    let excluded_domain_writes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM domain_role_bindings WHERE principal_id=$1 AND enabled",
    )
    .bind(excluded)
    .fetch_one(w)
    .await?;
    let excluded_visits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_node_visits WHERE assignee_principal_id=$1",
    )
    .bind(excluded)
    .fetch_one(w)
    .await?;
    if excluded_domain_writes != 0 || excluded_visits != 0 {
        conflicts += 1;
    }
    println!(
        "{}",
        json!({
            "planSha256": PLAN_SHA,
            "outcome": if conflicts == 0 { if terminal {"NOOP"} else {"PLANNED"} } else {"CONFLICT"},
            "writes": 0,
            "newAudits": 0,
            "workflowProjectionReadyCount": projection_ready,
            "domainOwnerReadyCount": owner_ready,
            "domainMemberReadyCount": member_ready,
            "activeResponsibilityReadyCount": responsibility_ready,
            "terminalResponsibilitySkippedCount": terminal_skipped,
            "conflictCount": conflicts,
            "errorCount": 0,
            "outcomeUnknownCount": 0,
            "historyRewriteCount": 0,
            "excludedIdentityDomainCount": excluded_domain_writes,
            "excludedIdentityVisitCount": excluded_visits,
            "pairs": pairs
        })
    );
    if conflicts == 0 {
        Ok(())
    } else {
        Err(conflict(format!("{conflicts} pair(s) have tuple drift")))
    }
}
async fn gate(w: &PgPool, a: &PgPool, plan: &Plan, agent: &str, actor: Uuid) -> Result<()> {
    let pair = plan
        .fleet_rows
        .iter()
        .find(|pair| pair.new_agent_id == agent)
        .ok_or_else(|| conflict("compiled canary missing"))?;
    let display = auth_exact(a, pair).await?;
    let migration_id = pair_migration_id(pair)?;
    if !pair_terminal_exact(w, plan, pair, &display, actor, &migration_id).await? {
        return Err(conflict(format!(
            "canary {agent} is not exact terminal PASS"
        )));
    }
    Ok(())
}
async fn apply_scope(
    w: &PgPool,
    a: &PgPool,
    p: &Plan,
    scope: Scope,
    actor: Uuid,
    _raw: &[u8],
) -> Result<()> {
    let actor_ok: i64 =
        sqlx::query_scalar("SELECT count(*) FROM principals WHERE principal_id=$1 AND enabled")
            .bind(actor)
            .fetch_one(w)
            .await?;
    if actor_ok != 1
        || actor.to_string() == EXCLUDED_PRINCIPAL
        || p.fleet_rows
            .iter()
            .any(|pair| actor == pair.old_principal_id || actor == pair.new_principal_id)
    {
        return Err(conflict(
            "operator Principal is absent, disabled, excluded, or part of the fleet",
        ));
    }
    if scope != Scope::Build {
        gate(w, a, p, "agt_build-in-public-agent", actor).await?;
    }
    if scope == Scope::Remaining {
        gate(w, a, p, "agt_efficiency-agent", actor).await?;
    }
    let mut outcomes = Vec::new();
    for (idx, pair) in p
        .fleet_rows
        .iter()
        .enumerate()
        .filter(|(_, x)| selected(scope, x))
    {
        let display = auth_exact(a, pair).await?;
        let outcome = apply_pair(w, p, idx + 1, pair, &display, actor).await?;
        outcomes.push(json!({"pairIndex":idx+1,"newAgentId":pair.new_agent_id,"outcome":outcome}));
    }
    let committed = outcomes
        .iter()
        .filter(|item| item.get("outcome").and_then(Value::as_str) == Some("COMMITTED"))
        .count();
    let noops = outcomes
        .iter()
        .filter(|item| item.get("outcome").and_then(Value::as_str) == Some("NOOP"))
        .count();
    println!(
        "{}",
        json!({"planSha256":PLAN_SHA,"scope":format!("{:?}",scope),"outcome":if committed==0{"NOOP"}else{"COMMITTED"},"writes":if committed==0{Some(0)}else{None},"newAudits":committed,"noopCount":noops,"pairs":outcomes})
    );
    Ok(())
}
fn pair_migration_id(pair: &Pair) -> Result<String> {
    Ok(format!(
        "trusted-fleet-v1:{}",
        digest(&json!({
            "planFileSha256": PLAN_SHA,
            "oldPrincipal": pair.old_principal_id,
            "newPrincipal": pair.new_principal_id,
        }))?
    ))
}
fn deterministic(label: &str) -> Uuid {
    let mut b: [u8; 16] = Sha256::digest(format!("{PLAN_SHA}:{label}").as_bytes())[..16]
        .try_into()
        .unwrap();
    b[6] = (b[6] & 0x0f) | 0x50;
    b[8] = (b[8] & 0x3f) | 0x80;
    Uuid::from_bytes(b)
}
fn digest(v: &Value) -> Result<String> {
    let s = jcs_canonicalize::canonicalize(
        &serde_json::to_string(v).map_err(|e| conflict(e.to_string()))?,
    )
    .map_err(|e| conflict(e.to_string()))?;
    Ok(hex::encode(Sha256::digest(s.as_bytes())))
}
async fn pair_pre_exact(
    pool: &PgPool,
    plan: &Plan,
    pair: &Pair,
    display: &str,
    actor: Uuid,
    migration_id: &str,
) -> Result<bool> {
    let projected = projection_exact(pool, pair, display).await?;
    let projection_absent: i64 =
        sqlx::query_scalar("SELECT count(*) FROM principals WHERE principal_id=$1")
            .bind(pair.new_principal_id)
            .fetch_one(pool)
            .await?;
    if !projected && projection_absent != 0 {
        return Ok(false);
    }
    for tuple in plan.domain_tuples.iter().filter(|tuple| {
        tuple.old_principal_id == pair.old_principal_id
            && tuple.new_principal_id == pair.new_principal_id
    }) {
        if domain_tuple_state(pool, tuple).await? != "PRE" {
            return Ok(false);
        }
    }
    for tuple in plan.current_responsibility_tuples.iter().filter(|tuple| {
        tuple.old_principal_id == pair.old_principal_id
            && tuple.new_principal_id == pair.new_principal_id
    }) {
        if !matches!(
            responsibility_tuple_state(pool, tuple).await?,
            "PRE" | "TERMINAL"
        ) {
            return Ok(false);
        }
    }
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2 AND command_type=$3")
        .bind(actor).bind(format!("{migration_id}:summary")).bind(COMMAND_TYPE).fetch_one(pool).await?;
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_security_audits WHERE principal_id=$1 AND action=$2 AND resource_type='TRUSTED_FLEET_PRINCIPAL_CUTOVER' AND resource_id=$3")
        .bind(actor).bind(AUDIT_ACTION).bind(migration_id).fetch_one(pool).await?;
    Ok(receipts == 0 && audits == 0)
}

async fn pair_terminal_exact(
    pool: &PgPool,
    plan: &Plan,
    pair: &Pair,
    display: &str,
    actor: Uuid,
    migration_id: &str,
) -> Result<bool> {
    if !projection_exact(pool, pair, display).await? {
        return Ok(false);
    }
    for tuple in plan.domain_tuples.iter().filter(|tuple| {
        tuple.old_principal_id == pair.old_principal_id
            && tuple.new_principal_id == pair.new_principal_id
    }) {
        if domain_tuple_state(pool, tuple).await? != "READY" {
            return Ok(false);
        }
    }
    for tuple in plan.current_responsibility_tuples.iter().filter(|tuple| {
        tuple.old_principal_id == pair.old_principal_id
            && tuple.new_principal_id == pair.new_principal_id
    }) {
        if !matches!(
            responsibility_tuple_state(pool, tuple).await?,
            "READY" | "TERMINAL"
        ) {
            return Ok(false);
        }
    }
    let summary: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2 AND command_type=$3 AND receipt_status='COMPLETED'",
    )
    .bind(actor)
    .bind(format!("{migration_id}:summary"))
    .bind(COMMAND_TYPE)
    .fetch_one(pool)
    .await?;
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_security_audits WHERE principal_id=$1 AND action=$2 AND resource_type='TRUSTED_FLEET_PRINCIPAL_CUTOVER' AND resource_id=$3",
    )
    .bind(actor)
    .bind(AUDIT_ACTION)
    .bind(migration_id)
    .fetch_one(pool)
    .await?;
    Ok(summary == 1 && audits == 1)
}

async fn apply_pair(
    pool: &PgPool,
    plan: &Plan,
    index: usize,
    pair: &Pair,
    display: &str,
    actor: Uuid,
) -> Result<&'static str> {
    let migration_id = pair_migration_id(pair)?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(&migration_id)
        .execute(&mut *tx)
        .await?;
    let summary_key = format!("{migration_id}:summary");
    let done:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2 AND command_type=$3 AND receipt_status='COMPLETED'").bind(actor).bind(&summary_key).bind(COMMAND_TYPE).fetch_one(&mut *tx).await?;
    if done > 1 {
        return Err(conflict(format!(
            "duplicate summary receipts for pair {index}"
        )));
    }
    if done == 1 {
        tx.rollback().await?;
        return if pair_terminal_exact(pool, plan, pair, display, actor, &migration_id).await? {
            Ok("NOOP")
        } else {
            Err(conflict(format!(
                "receipt exists but terminal state drifted for pair {index}"
            )))
        };
    }
    let old:i64=sqlx::query_scalar("SELECT count(*) FROM principals WHERE principal_id=$1 AND principal_type='AGENT' AND enabled").bind(pair.old_principal_id).fetch_one(&mut *tx).await?;
    if old != 1 {
        return Err(conflict(format!(
            "OLD Workflow projection drift pair {index}"
        )));
    }
    match sqlx::query("SELECT principal_type::text AS t,display_name,enabled FROM principals WHERE principal_id=$1 FOR UPDATE").bind(pair.new_principal_id).fetch_optional(&mut *tx).await?{None=>{sqlx::query("INSERT INTO principals(principal_id,principal_type,display_name,email,enabled,metadata) VALUES($1,'AGENT',$2,NULL,TRUE,NULL)").bind(pair.new_principal_id).bind(display).execute(&mut *tx).await?;},Some(r)=>if r.get::<String,_>("t")!="AGENT"||r.get::<String,_>("display_name")!=display||!r.get::<bool,_>("enabled"){return Err(conflict(format!("NEW Workflow projection drift pair {index}")));}}
    let domains: Vec<_> = plan
        .domain_tuples
        .iter()
        .filter(|x| {
            x.old_principal_id == pair.old_principal_id
                && x.new_principal_id == pair.new_principal_id
        })
        .collect();
    for d in &domains {
        let key = if d.role == "DOMAIN_OWNER" {
            "DOMAIN_OWNER"
        } else if d.role == "DOMAIN_MEMBER" {
            "DOMAIN_MEMBER"
        } else {
            return Err(conflict("unplanned role"));
        };
        let dk: String = sqlx::query_scalar(
            "SELECT domain_key FROM domains WHERE domain_id=$1 AND enabled FOR UPDATE",
        )
        .bind(d.domain_id)
        .fetch_one(&mut *tx)
        .await?;
        if dk != d.domain_key {
            return Err(conflict("Domain key drift"));
        }
        let old_enabled:Option<bool>=sqlx::query_scalar("SELECT enabled FROM domain_role_bindings WHERE domain_id=$1 AND principal_id=$2 AND role_key=$3 FOR UPDATE").bind(d.domain_id).bind(pair.old_principal_id).bind(key).fetch_optional(&mut *tx).await?;
        let new_enabled:Option<bool>=sqlx::query_scalar("SELECT enabled FROM domain_role_bindings WHERE domain_id=$1 AND principal_id=$2 AND role_key=$3 FOR UPDATE").bind(d.domain_id).bind(pair.new_principal_id).bind(key).fetch_optional(&mut *tx).await?;
        if old_enabled != Some(true) || new_enabled == Some(true) {
            return Err(conflict(format!("Domain binding drift pair {index}")));
        }
        let disabled = sqlx::query("UPDATE domain_role_bindings SET enabled=FALSE,disabled_at=now() WHERE domain_id=$1 AND principal_id=$2 AND role_key=$3 AND enabled").bind(d.domain_id).bind(pair.old_principal_id).bind(key).execute(&mut *tx).await?;
        if disabled.rows_affected() != 1 {
            return Err(conflict(format!("domain disable CAS failed pair {index}")));
        }
        sqlx::query("INSERT INTO domain_role_bindings(binding_id,domain_id,principal_id,role_key,enabled,disabled_at) VALUES($1,$2,$3,$4,TRUE,NULL) ON CONFLICT(domain_id,principal_id,role_key) DO UPDATE SET enabled=TRUE,disabled_at=NULL").bind(deterministic(&format!("domain:{index}:{}:{key}",d.domain_id))).bind(d.domain_id).bind(pair.new_principal_id).bind(key).execute(&mut *tx).await?;
    }
    let rs: Vec<_> = plan
        .current_responsibility_tuples
        .iter()
        .filter(|x| {
            x.old_principal_id == pair.old_principal_id
                && x.new_principal_id == pair.new_principal_id
        })
        .collect();
    let mut migrated = 0;
    let mut skipped = 0;
    for r in rs {
        match migrate_responsibility(&mut tx, index, pair, &migration_id, r, actor).await? {
            true => migrated += 1,
            false => skipped += 1,
        }
    }
    let command = deterministic(&format!("summary-command:{index}"));
    let audit = deterministic(&format!("summary-audit:{index}"));
    let request = json!({"migrationId":migration_id,"planFileSha256":PLAN_SHA,"pairIndex":index,"oldPrincipal":pair.old_principal_id,"newPrincipal":pair.new_principal_id,"migrationActor":actor});
    let response = json!({"commandId":command,"auditId":audit,"outcome":"COMMITTED","pairIndex":index,"domains":domains.len(),"responsibilitiesMigrated":migrated,"responsibilitiesSkipped":skipped});
    insert_completed_receipt(&mut tx, command, actor, &summary_key, &request, &response).await?;
    sqlx::query("INSERT INTO workflow_security_audits(audit_id,principal_id,action,resource_type,resource_id,details) VALUES($1,$2,$3,'TRUSTED_FLEET_PRINCIPAL_CUTOVER',$4,$5)").bind(audit).bind(actor).bind(AUDIT_ACTION).bind(&migration_id).bind(&response).execute(&mut *tx).await?;
    let remaining:i64=sqlx::query_scalar("SELECT count(*) FROM domain_role_bindings WHERE principal_id=$1 AND enabled AND domain_id=ANY($2)").bind(pair.old_principal_id).bind(&domains.iter().map(|x|x.domain_id).collect::<Vec<_>>()).fetch_one(&mut *tx).await?;
    if remaining != 0 {
        return Err(conflict("post-commit Domain reobserve failed"));
    }
    if let Err(commit_error) = tx.commit().await {
        return match pair_terminal_exact(pool, plan, pair, display, actor, &migration_id).await {
            Ok(true) => Ok("COMMITTED"),
            Ok(false) => match pair_pre_exact(pool, plan, pair, display, actor, &migration_id).await {
                Ok(true) => Err(outcome_error("ROLLED_BACK", format!("pair {index} commit was not applied: {commit_error}"))),
                Ok(false) => Err(conflict(format!("pair {index} has partial state after commit error: {commit_error}"))),
                Err(read_error) => Err(outcome_error("OUTCOME_UNKNOWN", format!("pair {index} commit error {commit_error}; preread failed: {read_error}"))),
            },
            Err(read_error) => Err(outcome_error("OUTCOME_UNKNOWN", format!("pair {index} commit error {commit_error}; terminal reread failed: {read_error}"))),
        };
    }
    if pair_terminal_exact(pool, plan, pair, display, actor, &migration_id).await? {
        Ok("COMMITTED")
    } else {
        Err(conflict(format!(
            "pair {index} committed but terminal reread disagrees"
        )))
    }
}
async fn insert_completed_receipt(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    actor: Uuid,
    key: &str,
    request: &Value,
    response: &Value,
) -> Result<()> {
    let rh = digest(request)?;
    let rd = digest(response)?;
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status) VALUES($1,$2,$3,$4,$5,'PROCESSING')")
        .bind(id).bind(actor).bind(key).bind(COMMAND_TYPE).bind(rh)
        .execute(&mut **tx).await?;
    let updated = sqlx::query("UPDATE workflow_command_receipts SET receipt_status='COMPLETED',response_status=200,response_body=$2,response_digest=$3,completed_at=now() WHERE command_id=$1 AND receipt_status='PROCESSING'")
        .bind(id).bind(response).bind(rd).execute(&mut **tx).await?;
    if updated.rows_affected() != 1 {
        return Err(conflict(
            "receipt completion did not affect exactly one row",
        ));
    }
    Ok(())
}
async fn migrate_responsibility(
    tx: &mut Transaction<'_, Postgres>,
    index: usize,
    pair: &Pair,
    migration_id: &str,
    r: &Responsibility,
    actor: Uuid,
) -> Result<bool> {
    let row=sqlx::query("SELECT wi.domain_id,wi.definition_version_id,wi.created_by_principal_id,wi.current_context_revision_id,wi.current_node_visit_id,wi.workflow_state_version,wi.cancelled,wi.archived_at,v.node_id,v.assignee_principal_id,n.node_type::text AS node_type FROM workflow_instances wi JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id JOIN workflow_node_definitions n ON n.node_id=v.node_id WHERE wi.workflow_instance_id=$1 FOR UPDATE OF wi,v").bind(r.workflow_instance_id).fetch_one(&mut **tx).await?;
    let cancelled: bool = row.try_get("cancelled").unwrap_or(false);
    let archived: Option<chrono::DateTime<Utc>> = row.try_get("archived_at").ok();
    let terminal = row.get::<String, _>("node_type") == "TERMINAL";
    if cancelled || archived.is_some() || terminal {
        if row.get::<Uuid, _>("domain_id") != r.domain_id {
            return Err(conflict(format!(
                "terminal responsibility domain drift {}",
                r.workflow_instance_id
            )));
        }
        let source_ok: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_node_visits WHERE node_visit_id=$1 AND workflow_instance_id=$2 AND node_id=$3 AND assignee_principal_id=$4")
            .bind(r.current_visit_id).bind(r.workflow_instance_id).bind(r.node_id).bind(pair.old_principal_id)
            .fetch_one(&mut **tx).await?;
        if source_ok != 1 {
            return Err(conflict(format!(
                "terminal responsibility source drift {}",
                r.workflow_instance_id
            )));
        }
        return Ok(false);
    }
    if row.get::<Uuid, _>("domain_id") != r.domain_id
        || row.get::<Uuid, _>("current_node_visit_id") != r.current_visit_id
        || row.get::<Uuid, _>("node_id") != r.node_id
        || row.get::<Uuid, _>("assignee_principal_id") != pair.old_principal_id
        || row.get::<i32, _>("workflow_state_version") != r.expected_state_version
    {
        return Err(conflict(format!(
            "active responsibility drift {}",
            r.workflow_instance_id
        )));
    }
    let open_assistance: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_assistance_cases WHERE node_visit_id=$1 AND status IN ('OWNER_PENDING','HUMAN_REQUIRED')")
        .bind(r.current_visit_id).fetch_one(&mut **tx).await?;
    if open_assistance != 0 {
        return Err(conflict(format!(
            "open assistance blocks responsibility {}",
            r.workflow_instance_id
        )));
    }
    let context: Option<Uuid> = row.try_get("current_context_revision_id").ok();
    let before = BeforeSnapshotV1::new(
        r.workflow_instance_id,
        r.domain_id,
        row.get("definition_version_id"),
        row.get("created_by_principal_id"),
        &WorkflowProjection {
            current_context_revision_id: context,
            current_node_visit_id: Some(r.current_visit_id),
            workflow_state_version: r.expected_state_version,
        },
    )
    .digest()
    .map_err(|e| conflict(e.to_string()))?;
    let target = deterministic(&format!("visit:{index}:{}", r.workflow_instance_id));
    let next:i32=sqlx::query_scalar("SELECT COALESCE(MAX(visit_number),0)+1 FROM workflow_node_visits WHERE workflow_instance_id=$1 AND node_id=$2").bind(r.workflow_instance_id).bind(r.node_id).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,$4,$5,NULL)").bind(target).bind(r.workflow_instance_id).bind(r.node_id).bind(next).bind(pair.new_principal_id).execute(&mut **tx).await?;
    let new_version = r.expected_state_version + 1;
    let cas = sqlx::query("UPDATE workflow_instances SET current_node_visit_id=$1,workflow_state_version=$2,updated_at=now() WHERE workflow_instance_id=$3 AND current_node_visit_id=$4 AND workflow_state_version=$5").bind(target).bind(new_version).bind(r.workflow_instance_id).bind(r.current_visit_id).bind(r.expected_state_version).execute(&mut **tx).await?;
    if cas.rows_affected() != 1 {
        return Err(conflict(format!(
            "responsibility CAS failed for {}",
            r.workflow_instance_id
        )));
    }
    let after = BeforeSnapshotV1::new(
        r.workflow_instance_id,
        r.domain_id,
        row.get("definition_version_id"),
        row.get("created_by_principal_id"),
        &WorkflowProjection {
            current_context_revision_id: context,
            current_node_visit_id: Some(target),
            workflow_state_version: new_version,
        },
    )
    .digest()
    .map_err(|e| conflict(e.to_string()))?;
    let command = deterministic(&format!("command:{index}:{}", r.workflow_instance_id));
    let event = deterministic(&format!("event:{index}:{}", r.workflow_instance_id));
    let occurred = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true);
    let data = json!({"migration_id":migration_id,"plan_sha256":PLAN_SHA,"pair_index":index,"old_agent_id":pair.old_agent_id,"new_agent_id":pair.new_agent_id,"old_principal_id":pair.old_principal_id,"new_principal_id":pair.new_principal_id,"workflow_instance_id":r.workflow_instance_id,"old_visit_id":r.current_visit_id,"new_visit_id":target,"node_id":r.node_id,"expected_state_version":r.expected_state_version,"resulting_state_version":new_version,"before_projection_digest":before,"after_projection_digest":after,"causation_id":Value::Null,"correlation_id":Value::Null,"occurred_at":occurred});
    let request = json!({"migrationId":migration_id,"planFileSha256":PLAN_SHA,"pairIndex":index,"workflowInstanceId":r.workflow_instance_id,"migrationActor":actor});
    let response = json!({"commandId":command,"eventId":event,"outcome":"COMMITTED","beforeProjectionDigest":before,"afterProjectionDigest":after});
    insert_completed_receipt(
        tx,
        command,
        actor,
        &format!("{migration_id}:{}", r.workflow_instance_id),
        &request,
        &response,
    )
    .await?;
    let ed = digest(&data)?;
    sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,command_id,event_type,source_node_visit_id,target_node_visit_id,context_revision_id,event_data,event_data_digest,actor_principal_id,from_node_id,to_node_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,$3,'v1',$4,$5,$6,$7,$8,$9,$10,$11,$12,$12,$13,$3)").bind(event).bind(r.workflow_instance_id).bind(new_version).bind(command).bind(EVENT_TYPE).bind(r.current_visit_id).bind(target).bind(context).bind(&data).bind(ed).bind(actor).bind(r.node_id).bind(r.expected_state_version).execute(&mut **tx).await?;
    Ok(true)
}
