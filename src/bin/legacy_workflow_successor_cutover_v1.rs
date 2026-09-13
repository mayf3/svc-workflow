//! Exact seven-row Wiki successor-Instance cutover candidate.
//! Governing authority: SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use std::{env, fs, process};
use uuid::Uuid;

const SCHEMA: &str = "wiki_legacy_principal_batch_unblock_plan_v0";
const RECEIPT_SCHEMA: &str = "wiki_legacy_principal_batch_unblock_receipt_v0";
const FAMILY: &str = "WIKI_LEGACY_PRINCIPAL_BATCH_UNBLOCK_V0";
const LOCK: &str = "wiki_legacy_principal_batch_unblock_v0";
const SOURCE_NODE_KEY: &str = "publish";
const NEXT_NODE_KEY: &str = "publish_review";
const TRANSITION_KEY: &str = "publish-done";
const SOURCES: [&str; 7] = [
    "20a509da-3e36-4321-b61d-a7c32525080c",
    "3995267b-9c57-46ac-b075-04c9b8fd9f32",
    "750d1fc8-d9cf-41ea-8b62-2e97f88f7594",
    "8030d089-02aa-4b67-b4f1-8d787e8f7bbc",
    "904480be-e45d-4e02-bb44-ae318a454e38",
    "bfdff759-b74b-45cb-b861-9f076d945feb",
    "c6474674-c71a-406a-ac1e-2fd4d1c43de2",
];

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
fn reject(s: impl Into<String>) -> Error {
    Error(format!("CONFLICT_ZERO_WRITE: {}", s.into()))
}
fn id(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap()
}
fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Plan {
    schema: String,
    command_family: String,
    database_name: String,
    source_definition_version_id: Uuid,
    successor_definition_version_id: Uuid,
    successor_version_number: i32,
    successor_definition_digest: String,
    semantic_model_version: i16,
    source_node_id: Uuid,
    successor_source_node_id: Uuid,
    successor_next_node_id: Uuid,
    successor_transition_id: Uuid,
    source_creator_principal: Uuid,
    canonical_creator_principal: Uuid,
    source_current_assignee: Uuid,
    legacy_next_principal: Uuid,
    canonical_next_principal: Uuid,
    operator_principal: Uuid,
    rows: Vec<PlanRow>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanRow {
    source_instance_id: Uuid,
    source_expected_state_version: i32,
    source_expected_current_visit: Uuid,
    source_expected_context_revision: Uuid,
    source_expected_context_digest: String,
    source_expected_visit_count: i64,
    source_expected_event_count: i64,
    source_expected_context_count: i64,
    successor_instance_id: Uuid,
    successor_initial_visit_id: Uuid,
    successor_context_revision_id: Uuid,
    successor_event_id: Uuid,
    command_id: Uuid,
    source_closure_fact: String,
}

fn load(path: &str) -> Result<(Plan, String)> {
    let bytes = fs::read(path)?;
    let plan: Plan =
        serde_json::from_slice(&bytes).map_err(|e| reject(format!("invalid plan JSON: {e}")))?;
    validate_closed_plan(&plan)?;
    Ok((plan, sha(&bytes)))
}

fn validate_closed_plan(p: &Plan) -> Result<()> {
    let exact = p.schema == SCHEMA
        && p.command_family == FAMILY
        && p.database_name == "svc_workflow_dogfood_clean"
        && p.source_definition_version_id == id("1bf0d466-91aa-4cd0-8b51-b550523f4408")
        && p.successor_definition_version_id == id("df59e327-43d3-462c-9cf3-f75271b8ca75")
        && p.successor_version_number == 3
        && p.successor_definition_digest
            == "a48d6ac093098c99f17a9290ad2d3945bf214e0d432fad46047bfc8bb82a73e3"
        && p.semantic_model_version == 1
        && p.source_node_id == id("9d27c951-1cf3-4b13-a2ac-f21d790eb7b1")
        && p.successor_source_node_id == id("7acf20dd-222d-4aa9-b59c-88ec33bbc4e9")
        && p.successor_next_node_id == id("67df10a2-f216-48b5-b69e-e06befa4e972")
        && p.successor_transition_id == id("9558fc40-5e4e-424a-9a7b-6bffe3e211f0")
        && p.source_creator_principal == id("87047adb-2931-400b-b5f0-c384cba37b8d")
        && p.canonical_creator_principal == id("8402d851-5bae-475c-bdb2-6fa1522803cf")
        && p.source_current_assignee == id("8402d851-5bae-475c-bdb2-6fa1522803cf")
        && p.legacy_next_principal == id("6ccdae57-0a2c-4a2e-b3e4-e3f4ec1edf2a")
        && p.canonical_next_principal == id("40daa67b-af5a-462d-b4ca-d40213147afb")
        && p.operator_principal == id("8402d851-5bae-475c-bdb2-6fa1522803cf")
        && p.rows.len() == SOURCES.len();
    if !exact {
        return Err(reject(
            "plan constants are outside the exact seven-row Wiki scope",
        ));
    }
    let mut ids = std::collections::HashSet::new();
    for (i, row) in p.rows.iter().enumerate() {
        if row.source_instance_id.to_string() != SOURCES[i]
            || row.source_expected_state_version != 4
            || row.source_expected_context_digest.len() != 64
            || row.source_expected_visit_count != 4
            || row.source_expected_event_count != 4
            || row.source_expected_context_count != 1
            || row.source_closure_fact != "CIR_CANONICAL_SUCCESSOR"
        {
            return Err(reject(format!(
                "row {} is outside the frozen preimage",
                i + 1
            )));
        }
        for id in [
            row.successor_instance_id,
            row.successor_initial_visit_id,
            row.successor_context_revision_id,
            row.successor_event_id,
            row.command_id,
        ] {
            if !ids.insert(id) {
                return Err(reject("duplicate preassigned UUID"));
            }
        }
    }
    Ok(())
}

async fn assert_db_identity(pool: &PgPool, p: &Plan) -> Result<()> {
    let db: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(pool)
        .await?;
    if db != p.database_name {
        return Err(reject(format!("database identity mismatch: {db}")));
    }
    Ok(())
}

async fn assert_enabled_agent(tx: &mut Transaction<'_, Postgres>, principal: Uuid) -> Result<()> {
    let row: Option<(String, bool)> =
        sqlx::query_as("SELECT principal_type::text,enabled FROM principals WHERE principal_id=$1")
            .bind(principal)
            .fetch_optional(&mut **tx)
            .await?;
    if row.as_ref().map(|x| (x.0.as_str(), x.1)) != Some(("AGENT", true)) {
        return Err(reject(format!(
            "principal {principal} is not an enabled AGENT"
        )));
    }
    Ok(())
}

async fn assert_shared(tx: &mut Transaction<'_, Postgres>, p: &Plan) -> Result<()> {
    for principal in [
        p.canonical_creator_principal,
        p.canonical_next_principal,
        p.operator_principal,
    ] {
        assert_enabled_agent(tx, principal).await?;
    }
    let target = sqlx::query("SELECT version_number,version_status::text,definition_digest,semantic_model_version FROM workflow_definition_versions WHERE definition_version_id=$1")
        .bind(p.successor_definition_version_id).fetch_optional(&mut **tx).await?.ok_or_else(|| reject("corrected DefinitionVersion missing"))?;
    if target.get::<i32, _>(0) != p.successor_version_number
        || target.get::<String, _>(1) != "PUBLISHED"
        || target.get::<Option<String>, _>(2).as_deref() != Some(&p.successor_definition_digest)
        || target.get::<i16, _>(3) != p.semantic_model_version
    {
        return Err(reject("corrected DefinitionVersion drift"));
    }
    let source_edge: Option<(Uuid,Uuid,Uuid)> = sqlx::query_as("SELECT s.node_id,t.transition_id,n.fixed_principal_id FROM workflow_transition_definitions t JOIN workflow_node_definitions s ON s.node_id=t.source_node_id JOIN workflow_node_definitions n ON n.node_id=t.target_node_id WHERE t.definition_version_id=$1 AND s.node_key=$2 AND t.transition_key=$3 AND n.node_key=$4")
        .bind(p.source_definition_version_id).bind(SOURCE_NODE_KEY).bind(TRANSITION_KEY).bind(NEXT_NODE_KEY).fetch_optional(&mut **tx).await?;
    if source_edge
        != Some((
            p.source_node_id,
            id("f001901b-32eb-41d7-98ac-a8aa8da03ff8"),
            p.legacy_next_principal,
        ))
    {
        return Err(reject(
            "source DefinitionVersion no longer has the exact stale edge",
        ));
    }
    let target_edge: Option<(Uuid,Uuid,Uuid,Uuid)> = sqlx::query_as("SELECT s.node_id,t.transition_id,n.node_id,n.fixed_principal_id FROM workflow_transition_definitions t JOIN workflow_node_definitions s ON s.node_id=t.source_node_id JOIN workflow_node_definitions n ON n.node_id=t.target_node_id WHERE t.definition_version_id=$1 AND s.node_key=$2 AND t.transition_key=$3 AND n.node_key=$4")
        .bind(p.successor_definition_version_id).bind(SOURCE_NODE_KEY).bind(TRANSITION_KEY).bind(NEXT_NODE_KEY).fetch_optional(&mut **tx).await?;
    if target_edge
        != Some((
            p.successor_source_node_id,
            p.successor_transition_id,
            p.successor_next_node_id,
            p.canonical_next_principal,
        ))
    {
        return Err(reject(
            "corrected DefinitionVersion does not bind the next visit to canonical principal",
        ));
    }
    let domain_id: Uuid = sqlx::query_scalar("SELECT d.domain_id FROM workflow_definitions d JOIN workflow_definition_versions v ON v.workflow_definition_id=d.workflow_definition_id WHERE v.definition_version_id=$1")
        .bind(p.successor_definition_version_id).fetch_one(&mut **tx).await?;
    let owner: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM domain_role_bindings WHERE domain_id=$1 AND principal_id=$2 AND role_key='DOMAIN_OWNER' AND enabled)").bind(domain_id).bind(p.operator_principal).fetch_one(&mut **tx).await?;
    let successor_member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM domain_role_bindings WHERE domain_id=$1 AND principal_id=$2 AND enabled)").bind(domain_id).bind(p.canonical_next_principal).fetch_one(&mut **tx).await?;
    let legacy_enabled: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM domain_role_bindings WHERE domain_id=$1 AND principal_id=$2 AND enabled)").bind(domain_id).bind(p.legacy_next_principal).fetch_one(&mut **tx).await?;
    if !owner || !successor_member || legacy_enabled {
        return Err(reject(
            "exact Domain authority/successor binding preimage is not satisfied",
        ));
    }
    Ok(())
}

async fn assert_source(tx: &mut Transaction<'_, Postgres>, p: &Plan, r: &PlanRow) -> Result<Value> {
    let q = sqlx::query("SELECT wi.workflow_state_version,wi.current_node_visit_id,wi.current_context_revision_id,wi.definition_version_id,wi.semantic_model_version,wi.cancelled,wi.archived_at,wi.created_by_principal_id,cr.payload,cr.payload_digest,n.node_key,nv.assignee_principal_id FROM workflow_instances wi JOIN workflow_context_revisions cr ON cr.context_revision_id=wi.current_context_revision_id JOIN workflow_node_visits nv ON nv.node_visit_id=wi.current_node_visit_id JOIN workflow_node_definitions n ON n.node_id=nv.node_id WHERE wi.workflow_instance_id=$1")
        .bind(r.source_instance_id).fetch_optional(&mut **tx).await?.ok_or_else(|| reject("source missing"))?;
    if q.get::<i32, _>(0) != r.source_expected_state_version
        || q.get::<Uuid, _>(1) != r.source_expected_current_visit
        || q.get::<Uuid, _>(2) != r.source_expected_context_revision
        || q.get::<Uuid, _>(3) != p.source_definition_version_id
        || q.get::<i16, _>(4) != p.semantic_model_version
        || q.get::<bool, _>(5)
        || q.get::<Option<chrono::DateTime<chrono::Utc>>, _>(6)
            .is_some()
        || q.get::<Uuid, _>(7) != p.source_creator_principal
        || q.get::<String, _>(9) != r.source_expected_context_digest
        || q.get::<String, _>(10) != SOURCE_NODE_KEY
        || q.get::<Uuid, _>(11) != p.source_current_assignee
    {
        return Err(reject("source exact preimage drift"));
    }
    let counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM workflow_node_visits WHERE workflow_instance_id=$1),(SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1),(SELECT count(*) FROM workflow_context_revisions WHERE workflow_instance_id=$1)")
        .bind(r.source_instance_id).fetch_one(&mut **tx).await?;
    if counts
        != (
            r.source_expected_visit_count,
            r.source_expected_event_count,
            r.source_expected_context_count,
        )
    {
        return Err(reject("source historical fact count drift"));
    }
    Ok(q.get(8))
}

async fn apply(pool: &PgPool, p: &Plan, plan_sha: &str, idx: usize, inject: bool) -> Result<Value> {
    let r = p
        .rows
        .get(idx)
        .ok_or_else(|| reject("--row must be 1..7"))?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(LOCK)
        .execute(&mut *tx)
        .await?;
    if let Some((stored_sha, receipt)) = sqlx::query_as::<_,(String,Value)>("SELECT plan_sha256,receipt FROM workflow_instance_successor_cutovers WHERE source_instance_id=$1").bind(r.source_instance_id).fetch_optional(&mut *tx).await? {
        if stored_sha != plan_sha { return Err(reject("source already closed by a different plan")); }
        tx.rollback().await?; return Ok(receipt);
    }
    sqlx::query("SELECT workflow_instance_id FROM workflow_instances WHERE workflow_instance_id=$1 FOR UPDATE").bind(r.source_instance_id).fetch_optional(&mut *tx).await?.ok_or_else(|| reject("source missing"))?;
    assert_shared(&mut tx, p).await?;
    let context = assert_source(&mut tx, p, r).await?;
    let collision: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM workflow_instances WHERE workflow_instance_id=$1)+(SELECT count(*) FROM workflow_node_visits WHERE node_visit_id=$2)+(SELECT count(*) FROM workflow_context_revisions WHERE context_revision_id=$3)+(SELECT count(*) FROM workflow_events WHERE event_id=$4)+(SELECT count(*) FROM workflow_command_receipts WHERE command_id=$5)")
        .bind(r.successor_instance_id).bind(r.successor_initial_visit_id).bind(r.successor_context_revision_id).bind(r.successor_event_id).bind(r.command_id).fetch_one(&mut *tx).await?;
    if collision != 0 {
        return Err(reject("successor UUID collision"));
    }
    let request_hash = sha(format!("{plan_sha}:{}", r.source_instance_id).as_bytes());
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash) VALUES($1,$2,$3,$4,$5)").bind(r.command_id).bind(p.operator_principal).bind(format!("{FAMILY}:{}",r.source_instance_id)).bind(FAMILY).bind(request_hash).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO workflow_instances(workflow_instance_id,domain_id,definition_version_id,created_by_principal_id,current_context_revision_id,current_node_visit_id,workflow_state_version,external_url,metadata,external_reference,require_explicit_node_assignees,subject_id,artifact_id,artifact_version,artifact_digest,require_artifact_binding,semantic_model_version,execution_class) SELECT $1,domain_id,$2,$3,$4,$5,1,external_url,metadata,NULL,require_explicit_node_assignees,subject_id,artifact_id,artifact_version,artifact_digest,require_artifact_binding,semantic_model_version,execution_class FROM workflow_instances WHERE workflow_instance_id=$6")
        .bind(r.successor_instance_id).bind(p.successor_definition_version_id).bind(p.canonical_creator_principal).bind(r.successor_context_revision_id).bind(r.successor_initial_visit_id).bind(r.source_instance_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO workflow_context_revisions(context_revision_id,workflow_instance_id,revision_number,previous_revision_id,payload,payload_digest,created_by_principal_id) VALUES($1,$2,1,NULL,$3,$4,$5)").bind(r.successor_context_revision_id).bind(r.successor_instance_id).bind(&context).bind(&r.source_expected_context_digest).bind(p.operator_principal).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,1,$4,NULL)").bind(r.successor_initial_visit_id).bind(r.successor_instance_id).bind(p.successor_source_node_id).bind(p.source_current_assignee).execute(&mut *tx).await?;
    let event_data = json!({"cirSourceInstanceId":r.source_instance_id,"definitionVersionId":p.successor_definition_version_id,"initialNodeId":p.successor_source_node_id,"closureReason":r.source_closure_fact,"originContextRevisionId":r.source_expected_context_revision});
    let event_digest =
        jcs_canonicalize::sha256_jcs_hex(&event_data).map_err(|e| reject(e.to_string()))?;
    sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,command_id,event_type,target_node_visit_id,context_revision_id,event_data,event_data_digest,actor_principal_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,1,'v1',$3,'INSTANCE_CREATED',$4,$5,$6,$7,$8,0,1)").bind(r.successor_event_id).bind(r.successor_instance_id).bind(r.command_id).bind(r.successor_initial_visit_id).bind(r.successor_context_revision_id).bind(&event_data).bind(event_digest).bind(p.operator_principal).execute(&mut *tx).await?;
    if inject {
        return Err(reject("injected failure after successor creation"));
    }
    let receipt = json!({"schema":RECEIPT_SCHEMA,"planSha256":plan_sha,"sourceInstanceId":r.source_instance_id,"successorInstanceId":r.successor_instance_id,"sourceClosure":"CIR_CANONICAL_SUCCESSOR","nextAssigneePrincipalId":p.canonical_next_principal});
    let response_digest =
        jcs_canonicalize::sha256_jcs_hex(&receipt).map_err(|e| reject(e.to_string()))?;
    sqlx::query("UPDATE workflow_command_receipts SET receipt_status='COMPLETED',response_status=200,response_body=$1,response_digest=$2,completed_at=now() WHERE command_id=$3").bind(&receipt).bind(response_digest).bind(r.command_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO workflow_instance_successor_cutovers(source_instance_id,successor_instance_id,source_definition_version_id,successor_definition_version_id,source_current_visit_id,successor_initial_visit_id,successor_context_revision_id,source_expected_state_version,model,closure_reason,plan_sha256,operator_principal_id,receipt) VALUES($1,$2,$3,$4,$5,$6,$7,$8,1,'CIR_CANONICAL_SUCCESSOR',$9,$10,$11)").bind(r.source_instance_id).bind(r.successor_instance_id).bind(p.source_definition_version_id).bind(p.successor_definition_version_id).bind(r.source_expected_current_visit).bind(r.successor_initial_visit_id).bind(r.successor_context_revision_id).bind(r.source_expected_state_version).bind(plan_sha).bind(p.operator_principal).bind(&receipt).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(receipt)
}

async fn verify(pool: &PgPool, p: &Plan, plan_sha: &str, idx: usize) -> Result<Value> {
    let r = p
        .rows
        .get(idx)
        .ok_or_else(|| reject("--row must be 1..7"))?;
    let row=sqlx::query("SELECT c.receipt,wi.definition_version_id,wi.created_by_principal_id,wi.current_context_revision_id,wi.current_node_visit_id,cr.payload_digest,nv.assignee_principal_id,n.node_key,tn.fixed_principal_id FROM workflow_instance_successor_cutovers c JOIN workflow_instances wi ON wi.workflow_instance_id=c.successor_instance_id JOIN workflow_context_revisions cr ON cr.context_revision_id=wi.current_context_revision_id JOIN workflow_node_visits nv ON nv.node_visit_id=wi.current_node_visit_id JOIN workflow_node_definitions n ON n.node_id=nv.node_id JOIN workflow_transition_definitions t ON t.transition_id=n.primary_advance_transition_id JOIN workflow_node_definitions tn ON tn.node_id=t.target_node_id WHERE c.source_instance_id=$1 AND c.successor_instance_id=$2 AND c.plan_sha256=$3").bind(r.source_instance_id).bind(r.successor_instance_id).bind(plan_sha).fetch_optional(pool).await?.ok_or_else(||reject("closure/receipt missing"))?;
    if row.get::<Uuid, _>(1) != p.successor_definition_version_id
        || row.get::<Uuid, _>(2) != p.canonical_creator_principal
        || row.get::<Uuid, _>(3) != r.successor_context_revision_id
        || row.get::<Uuid, _>(4) != r.successor_initial_visit_id
        || row.get::<String, _>(5) != r.source_expected_context_digest
        || row.get::<Uuid, _>(6) != p.source_current_assignee
        || row.get::<String, _>(7) != SOURCE_NODE_KEY
        || row.get::<Uuid, _>(8) != p.canonical_next_principal
    {
        return Err(reject("successor verification mismatch"));
    }
    Ok(row.get(0))
}

fn arg(args: &[String], name: &str) -> Result<String> {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].clone())
        .ok_or_else(|| reject(format!("missing {name}")))
}
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{e}");
        process::exit(2)
    }
}
async fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| reject("mode required"))?;
    let (p, actual) = load(&arg(&args, "--plan")?)?;
    if mode != "plan" && actual != arg(&args, "--plan-sha256")? {
        return Err(reject("plan SHA-256 mismatch"));
    }
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&env::var("DATABASE_URL").map_err(|_| reject("DATABASE_URL required"))?)
        .await?;
    assert_db_identity(&pool, &p).await?;
    match mode {
        "plan" => {
            let mut tx = pool.begin().await?;
            sqlx::query("SET TRANSACTION READ ONLY")
                .execute(&mut *tx)
                .await?;
            assert_shared(&mut tx, &p).await?;
            for r in &p.rows {
                assert_source(&mut tx, &p, r).await?;
            }
            tx.rollback().await?;
            println!("PLAN_OK sha256={actual} rows=7 unlockable=7");
        }
        "apply" => {
            let i: usize = arg(&args, "--row")?
                .parse()
                .map_err(|_| reject("bad row"))?;
            let out = apply(
                &pool,
                &p,
                &actual,
                i.checked_sub(1).ok_or_else(|| reject("bad row"))?,
                args.iter().any(|x| x == "--inject-failure-after-create"),
            )
            .await?;
            let receipt = arg(&args, "--receipt-out")?;
            fs::write(&receipt, serde_json::to_vec_pretty(&out).unwrap())?;
            println!("APPLY_OK row={i} receipt={receipt}");
        }
        "verify" => {
            let i: usize = arg(&args, "--row")?
                .parse()
                .map_err(|_| reject("bad row"))?;
            let out = verify(
                &pool,
                &p,
                &actual,
                i.checked_sub(1).ok_or_else(|| reject("bad row"))?,
            )
            .await?;
            if let Ok(path) = arg(&args, "--receipt") {
                let disk: Value =
                    serde_json::from_slice(&fs::read(path)?).map_err(|e| reject(e.to_string()))?;
                if disk != out {
                    return Err(reject("receipt mismatch"));
                }
            }
            println!("VERIFY_OK row={i}");
        }
        _ => {
            return Err(reject(
                "use plan|apply|verify; apply requires --row 1..7 and --receipt-out",
            ))
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frozen() -> Plan {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/evidence/legacy-workflow-successor-cutover-v1/plan.json"
        )))
        .unwrap()
    }
    #[test]
    fn exact_plan_is_accepted() {
        validate_closed_plan(&frozen()).unwrap();
    }
    #[test]
    fn eighth_row_is_rejected() {
        let mut p = frozen();
        p.rows.push(p.rows[0].clone());
        assert!(validate_closed_plan(&p).is_err());
    }
    #[test]
    fn stale_target_is_rejected() {
        let mut p = frozen();
        p.canonical_next_principal = p.legacy_next_principal;
        assert!(validate_closed_plan(&p).is_err());
    }
    #[test]
    fn duplicate_uuid_is_rejected() {
        let mut p = frozen();
        p.rows[1].successor_instance_id = p.rows[0].successor_instance_id;
        assert!(validate_closed_plan(&p).is_err());
    }
}
