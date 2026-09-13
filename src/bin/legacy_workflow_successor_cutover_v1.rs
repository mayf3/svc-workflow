//! Bounded offline CIR successor-Instance cutover operator.
//! Exactly four frozen source Instances are accepted; arbitrary IDs are impossible.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use std::{env, fs, process};
use uuid::Uuid;

const SCHEMA: &str = "legacy_workflow_successor_cutover_plan_v1";
const RECEIPT_SCHEMA: &str = "legacy_workflow_successor_cutover_receipt_v1";
const FAMILY: &str = "LEGACY_WORKFLOW_SUCCESSOR_CUTOVER_V1";
const LOCK: &str = "legacy_workflow_successor_cutover_v1";
const SOURCES: [&str; 4] = [
    "92af02be-e727-46e8-8fab-bddb7a45b3da",
    "6ca15ad1-3e1d-4773-88a7-206b8c6602e9",
    "9bac385a-fed2-42be-ac58-50cff7b42c4a",
    "ff244e92-e2cd-4da8-a51e-1aa87e52dafd",
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
    successor_model: i16,
    target_node_ids: TargetNodes,
    target_transition_ids: TargetTransitions,
    publisher_principal: Uuid,
    operator_principal: Uuid,
    successor_creator_principal: Uuid,
    rows: Vec<PlanRow>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetNodes {
    dispatch: Uuid,
    publishing: Uuid,
    verify: Uuid,
    done: Uuid,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetTransitions {
    advance_to_publishing: Uuid,
    advance_to_verify: Uuid,
    verified: Uuid,
    verify_failed: Uuid,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanRow {
    source_instance_id: Uuid,
    source_expected_state_version: i32,
    source_expected_current_visit: Uuid,
    source_expected_context_revision: Uuid,
    source_expected_context: Value,
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

fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn load(path: &str) -> Result<(Plan, Vec<u8>, String)> {
    let bytes = fs::read(path)?;
    let plan: Plan =
        serde_json::from_slice(&bytes).map_err(|e| reject(format!("invalid plan JSON: {e}")))?;
    validate_closed_plan(&plan)?;
    let digest = sha(&bytes);
    Ok((plan, bytes, digest))
}
fn validate_closed_plan(p: &Plan) -> Result<()> {
    if p.schema != SCHEMA
        || p.command_family != FAMILY
        || p.rows.len() != 4
        || p.successor_model != 1
    {
        return Err(reject("wrong schema/family/row count/model"));
    }
    if p.successor_definition_digest.len() != 64 {
        return Err(reject("bad definition digest"));
    }
    if p.publisher_principal != Uuid::parse_str("8b44befd-934e-48cd-afac-31ed981fb732").unwrap()
        || p.operator_principal != Uuid::parse_str("271876c1-5bc9-4902-855d-68bf5c99eccb").unwrap()
        || p.successor_creator_principal
            != Uuid::parse_str("d5b3aeb2-e754-49a9-9914-b963521c0985").unwrap()
    {
        return Err(reject(
            "target principals are not the frozen canonical principals",
        ));
    }
    for (i, row) in p.rows.iter().enumerate() {
        if row.source_instance_id.to_string() != SOURCES[i] {
            return Err(reject(format!("row {} is not frozen source", i + 1)));
        }
        if row.source_closure_fact != "CIR_CANONICAL_SUCCESSOR" {
            return Err(reject("wrong closure fact"));
        }
        let op = row
            .source_expected_context
            .get("operator_principal")
            .and_then(Value::as_str);
        let publisher = row
            .source_expected_context
            .get("publisher_principal")
            .and_then(Value::as_str);
        if op != Some("aed2c679-71c0-4e23-8d73-c448c5ea6dd2")
            || publisher != Some("d2bec623-56ba-46cb-a217-933cfe63a243")
        {
            return Err(reject("source context legacy preimage mismatch"));
        }
    }
    let mut ids = std::collections::HashSet::new();
    for id in [
        p.successor_definition_version_id,
        p.target_node_ids.dispatch,
        p.target_node_ids.publishing,
        p.target_node_ids.verify,
        p.target_node_ids.done,
        p.target_transition_ids.advance_to_publishing,
        p.target_transition_ids.advance_to_verify,
        p.target_transition_ids.verified,
        p.target_transition_ids.verify_failed,
    ]
    .into_iter()
    .chain(p.rows.iter().flat_map(|r| {
        [
            r.successor_instance_id,
            r.successor_initial_visit_id,
            r.successor_context_revision_id,
            r.successor_event_id,
            r.command_id,
        ]
    })) {
        if !ids.insert(id) {
            return Err(reject("duplicate preassigned UUID in plan"));
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
async fn assert_principal(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<()> {
    let v: Option<(String, bool)> = sqlx::query_as(
        "SELECT principal_type::text, enabled FROM principals WHERE principal_id=$1",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    if v.as_ref().map(|x| (x.0.as_str(), x.1)) != Some(("AGENT", true)) {
        return Err(reject(format!("principal {id} is not enabled AGENT")));
    }
    Ok(())
}
async fn assert_source<'a>(
    tx: &mut Transaction<'a, Postgres>,
    p: &Plan,
    r: &PlanRow,
) -> Result<()> {
    let q = sqlx::query("SELECT wi.workflow_state_version,wi.current_node_visit_id,wi.current_context_revision_id,wi.definition_version_id,wi.semantic_model_version,wi.cancelled,wi.archived_at,cr.payload,n.node_key,nv.assignee_principal_id FROM workflow_instances wi JOIN workflow_context_revisions cr ON cr.context_revision_id=wi.current_context_revision_id JOIN workflow_node_visits nv ON nv.node_visit_id=wi.current_node_visit_id JOIN workflow_node_definitions n ON n.node_id=nv.node_id WHERE wi.workflow_instance_id=$1")
        .bind(r.source_instance_id).fetch_optional(&mut **tx).await?.ok_or_else(|| reject("source missing"))?;
    if q.get::<i32, _>(0) != r.source_expected_state_version
        || q.get::<Uuid, _>(1) != r.source_expected_current_visit
        || q.get::<Uuid, _>(2) != r.source_expected_context_revision
        || q.get::<Uuid, _>(3) != p.source_definition_version_id
        || q.get::<i16, _>(4) != 1
        || q.get::<bool, _>(5)
        || q.get::<Option<chrono::DateTime<chrono::Utc>>, _>(6)
            .is_some()
        || q.get::<Value, _>(7) != r.source_expected_context
        || q.get::<String, _>(8) != "publishing"
        || q.get::<Uuid, _>(9) != p.publisher_principal
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
    let lineage: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_identity_successor_lines WHERE (source_principal_id,successor_principal_id) IN (($1,$2),($3,$4))")
        .bind(Uuid::parse_str("d2bec623-56ba-46cb-a217-933cfe63a243").unwrap()).bind(p.publisher_principal)
        .bind(Uuid::parse_str("aed2c679-71c0-4e23-8d73-c448c5ea6dd2").unwrap()).bind(p.operator_principal)
        .fetch_one(&mut **tx).await?;
    if lineage != 2 {
        return Err(reject("exact identity successor lineage absent"));
    }
    Ok(())
}

async fn ensure_target_definition(tx: &mut Transaction<'_, Postgres>, p: &Plan) -> Result<()> {
    if let Some(v) = sqlx::query("SELECT version_number,version_status::text,definition_digest,semantic_model_version FROM workflow_definition_versions WHERE definition_version_id=$1")
        .bind(p.successor_definition_version_id).fetch_optional(&mut **tx).await? {
        if v.get::<i32,_>(0)!=p.successor_version_number || v.get::<String,_>(1)!="PUBLISHED" || v.get::<Option<String>,_>(2).as_deref()!=Some(&p.successor_definition_digest) || v.get::<i16,_>(3)!=1 { return Err(reject("target definition collision/drift")); }
        return Ok(());
    }
    let collisions: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM workflow_node_definitions WHERE node_id=ANY($1))+(SELECT count(*) FROM workflow_transition_definitions WHERE transition_id=ANY($2))")
        .bind(vec![p.target_node_ids.dispatch,p.target_node_ids.publishing,p.target_node_ids.verify,p.target_node_ids.done])
        .bind(vec![p.target_transition_ids.advance_to_publishing,p.target_transition_ids.advance_to_verify,p.target_transition_ids.verified,p.target_transition_ids.verify_failed])
        .fetch_one(&mut **tx).await?;
    if collisions != 0 {
        return Err(reject("target graph UUID collision"));
    }
    sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id,workflow_definition_id,version_number,version_status,definition_digest,json_schema_dialect,validator_version,context_schema,submission_schema,metadata,published_at,published_by_principal_id,semantic_model_version) SELECT $1,workflow_definition_id,$2,'DRAFT',NULL,json_schema_dialect,validator_version,context_schema,submission_schema,jsonb_build_object('cirSourceDefinitionVersionId',definition_version_id,'cirReason','CANONICAL_IDENTITY_SUCCESSOR'),NULL,NULL,semantic_model_version FROM workflow_definition_versions WHERE definition_version_id=$3 AND semantic_model_version=1")
        .bind(p.successor_definition_version_id).bind(p.successor_version_number).bind(p.source_definition_version_id).execute(&mut **tx).await?;
    let src_nodes: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT node_id,node_key FROM workflow_node_definitions WHERE definition_version_id=$1",
    )
    .bind(p.source_definition_version_id)
    .fetch_all(&mut **tx)
    .await?;
    for (old, key) in src_nodes {
        let new = match key.as_str() {
            "dispatch" => p.target_node_ids.dispatch,
            "publishing" => p.target_node_ids.publishing,
            "verify" => p.target_node_ids.verify,
            "done" => p.target_node_ids.done,
            _ => return Err(reject("unexpected source node")),
        };
        sqlx::query("INSERT INTO workflow_node_definitions (node_id,definition_version_id,node_key,display_name,order_index,node_type,assignee_ref_type,fixed_principal_id,instructions,primary_advance_transition_id,metadata,assignee_input_key) SELECT $1,$2,node_key,display_name,order_index,node_type,assignee_ref_type,CASE fixed_principal_id WHEN 'd2bec623-56ba-46cb-a217-933cfe63a243'::uuid THEN $3 WHEN 'aed2c679-71c0-4e23-8d73-c448c5ea6dd2'::uuid THEN $4 ELSE fixed_principal_id END,instructions,NULL,metadata,assignee_input_key FROM workflow_node_definitions WHERE node_id=$5")
            .bind(new).bind(p.successor_definition_version_id).bind(p.publisher_principal).bind(p.operator_principal).bind(old).execute(&mut **tx).await?;
    }
    let transitions = [
        (
            "advance-to-publishing",
            p.target_transition_ids.advance_to_publishing,
        ),
        (
            "advance-to-verify",
            p.target_transition_ids.advance_to_verify,
        ),
        ("verified", p.target_transition_ids.verified),
        ("verify-failed", p.target_transition_ids.verify_failed),
    ];
    for (key, new) in transitions {
        sqlx::query("INSERT INTO workflow_transition_definitions (transition_id,definition_version_id,transition_key,display_name,source_node_id,target_node_id,transition_effect,submission_schema,metadata) SELECT $1,$2,t.transition_key,t.display_name,CASE sn.node_key WHEN 'dispatch' THEN $3 WHEN 'publishing' THEN $4 WHEN 'verify' THEN $5 WHEN 'done' THEN $6 END,CASE tn.node_key WHEN 'dispatch' THEN $3 WHEN 'publishing' THEN $4 WHEN 'verify' THEN $5 WHEN 'done' THEN $6 END,t.transition_effect,t.submission_schema,t.metadata FROM workflow_transition_definitions t JOIN workflow_node_definitions sn ON sn.node_id=t.source_node_id JOIN workflow_node_definitions tn ON tn.node_id=t.target_node_id WHERE t.definition_version_id=$7 AND t.transition_key=$8")
            .bind(new).bind(p.successor_definition_version_id).bind(p.target_node_ids.dispatch).bind(p.target_node_ids.publishing).bind(p.target_node_ids.verify).bind(p.target_node_ids.done).bind(p.source_definition_version_id).bind(key).execute(&mut **tx).await?;
    }
    for (node, transition) in [
        (
            p.target_node_ids.dispatch,
            Some(p.target_transition_ids.advance_to_publishing),
        ),
        (
            p.target_node_ids.publishing,
            Some(p.target_transition_ids.advance_to_verify),
        ),
        (
            p.target_node_ids.verify,
            Some(p.target_transition_ids.verified),
        ),
        (p.target_node_ids.done, None),
    ] {
        sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2").bind(transition).bind(node).execute(&mut **tx).await?;
    }
    sqlx::query("UPDATE workflow_definition_versions SET version_status='PUBLISHED',definition_digest=$1,published_at=now(),published_by_principal_id=$2 WHERE definition_version_id=$3")
        .bind(&p.successor_definition_digest).bind(p.operator_principal).bind(p.successor_definition_version_id).execute(&mut **tx).await?;
    sqlx::query("UPDATE workflow_definition_versions SET version_status='DEPRECATED',deprecated_at=now(),deprecated_by_principal_id=$1,deprecation_reason='CIR_CANONICAL_SUCCESSOR' WHERE definition_version_id=$2 AND version_status='PUBLISHED'")
        .bind(p.operator_principal).bind(p.source_definition_version_id).execute(&mut **tx).await?;
    Ok(())
}

async fn apply(pool: &PgPool, p: &Plan, plan_sha: &str, idx: usize, inject: bool) -> Result<Value> {
    let r = p
        .rows
        .get(idx)
        .ok_or_else(|| reject("--row must be 1..4"))?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(LOCK)
        .execute(&mut *tx)
        .await?;
    if let Some((stored_sha, receipt)) = sqlx::query_as::<_, (String, Value)>(
        "SELECT plan_sha256,receipt FROM workflow_instance_successor_cutovers WHERE source_instance_id=$1",
    )
    .bind(r.source_instance_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        if stored_sha != plan_sha {
            return Err(reject("source already closed by a different plan"));
        }
        tx.rollback().await?;
        return Ok(receipt);
    }
    sqlx::query("SELECT workflow_instance_id FROM workflow_instances WHERE workflow_instance_id=$1 FOR UPDATE")
        .bind(r.source_instance_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| reject("source missing"))?;
    assert_source(&mut tx, p, r).await?;
    for id in [
        p.publisher_principal,
        p.operator_principal,
        p.successor_creator_principal,
    ] {
        assert_principal(&mut tx, id).await?;
    }
    let collision: i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM workflow_instances WHERE workflow_instance_id=$1)+(SELECT count(*) FROM workflow_node_visits WHERE node_visit_id=$2)+(SELECT count(*) FROM workflow_context_revisions WHERE context_revision_id=$3)+(SELECT count(*) FROM workflow_events WHERE event_id=$4)+(SELECT count(*) FROM workflow_command_receipts WHERE command_id=$5)")
        .bind(r.successor_instance_id).bind(r.successor_initial_visit_id).bind(r.successor_context_revision_id).bind(r.successor_event_id).bind(r.command_id).fetch_one(&mut *tx).await?;
    if collision != 0 {
        return Err(reject("successor UUID collision"));
    }
    ensure_target_definition(&mut tx, p).await?;
    let mut context = r.source_expected_context.clone();
    context["operator_principal"] = json!(p.operator_principal);
    context["publisher_principal"] = json!(p.publisher_principal);
    let context_digest =
        jcs_canonicalize::sha256_jcs_hex(&context).map_err(|e| reject(e.to_string()))?;
    let request_hash = sha(format!("{plan_sha}:{}", r.source_instance_id).as_bytes());
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash) VALUES($1,$2,$3,$4,$5)")
        .bind(r.command_id).bind(p.operator_principal).bind(format!("{FAMILY}:{}",r.source_instance_id)).bind(FAMILY).bind(&request_hash).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO workflow_instances(workflow_instance_id,domain_id,definition_version_id,created_by_principal_id,current_context_revision_id,current_node_visit_id,workflow_state_version,external_url,metadata,external_reference,require_explicit_node_assignees,subject_id,artifact_id,artifact_version,artifact_digest,require_artifact_binding,semantic_model_version,execution_class) SELECT $1,domain_id,$2,$3,$4,$5,1,external_url,metadata,NULL,require_explicit_node_assignees,subject_id,artifact_id,artifact_version,artifact_digest,require_artifact_binding,1,execution_class FROM workflow_instances WHERE workflow_instance_id=$6")
        .bind(r.successor_instance_id).bind(p.successor_definition_version_id).bind(p.successor_creator_principal).bind(r.successor_context_revision_id).bind(r.successor_initial_visit_id).bind(r.source_instance_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO workflow_context_revisions(context_revision_id,workflow_instance_id,revision_number,previous_revision_id,payload,payload_digest,created_by_principal_id) VALUES($1,$2,1,NULL,$3,$4,$5)")
        .bind(r.successor_context_revision_id).bind(r.successor_instance_id).bind(&context).bind(&context_digest).bind(p.operator_principal).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,1,$4,NULL)")
        .bind(r.successor_initial_visit_id).bind(r.successor_instance_id).bind(p.target_node_ids.publishing).bind(p.publisher_principal).execute(&mut *tx).await?;
    let event_data = json!({"cirSourceInstanceId":r.source_instance_id,"definitionVersionId":p.successor_definition_version_id,"initialNodeId":p.target_node_ids.publishing,"closureReason":r.source_closure_fact});
    let event_digest =
        jcs_canonicalize::sha256_jcs_hex(&event_data).map_err(|e| reject(e.to_string()))?;
    sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,command_id,event_type,target_node_visit_id,context_revision_id,event_data,event_data_digest,actor_principal_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,1,'v1',$3,'INSTANCE_CREATED',$4,$5,$6,$7,$8,0,1)")
        .bind(r.successor_event_id).bind(r.successor_instance_id).bind(r.command_id).bind(r.successor_initial_visit_id).bind(r.successor_context_revision_id).bind(&event_data).bind(&event_digest).bind(p.operator_principal).execute(&mut *tx).await?;
    if inject {
        return Err(reject("injected failure after successor creation"));
    }
    let receipt = json!({"schema":RECEIPT_SCHEMA,"planSha256":plan_sha,"sourceInstanceId":r.source_instance_id,"successorInstanceId":r.successor_instance_id,"sourceClosure":"CIR_CANONICAL_SUCCESSOR"});
    let response_digest =
        jcs_canonicalize::sha256_jcs_hex(&receipt).map_err(|e| reject(e.to_string()))?;
    sqlx::query("UPDATE workflow_command_receipts SET receipt_status='COMPLETED',response_status=200,response_body=$1,response_digest=$2,completed_at=now() WHERE command_id=$3")
        .bind(&receipt).bind(response_digest).bind(r.command_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO workflow_instance_successor_cutovers(source_instance_id,successor_instance_id,source_definition_version_id,successor_definition_version_id,source_current_visit_id,successor_initial_visit_id,successor_context_revision_id,source_expected_state_version,model,closure_reason,plan_sha256,operator_principal_id,receipt) VALUES($1,$2,$3,$4,$5,$6,$7,$8,1,'CIR_CANONICAL_SUCCESSOR',$9,$10,$11)")
        .bind(r.source_instance_id).bind(r.successor_instance_id).bind(p.source_definition_version_id).bind(p.successor_definition_version_id).bind(r.source_expected_current_visit).bind(r.successor_initial_visit_id).bind(r.successor_context_revision_id).bind(r.source_expected_state_version).bind(plan_sha).bind(p.operator_principal).bind(&receipt).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(receipt)
}

async fn verify(pool: &PgPool, p: &Plan, plan_sha: &str, idx: usize) -> Result<Value> {
    let r = p
        .rows
        .get(idx)
        .ok_or_else(|| reject("--row must be 1..4"))?;
    let row=sqlx::query("SELECT c.receipt,wi.definition_version_id,wi.semantic_model_version,wi.current_context_revision_id,wi.current_node_visit_id,cr.payload,nv.assignee_principal_id,n.node_key FROM workflow_instance_successor_cutovers c JOIN workflow_instances wi ON wi.workflow_instance_id=c.successor_instance_id JOIN workflow_context_revisions cr ON cr.context_revision_id=wi.current_context_revision_id JOIN workflow_node_visits nv ON nv.node_visit_id=wi.current_node_visit_id JOIN workflow_node_definitions n ON n.node_id=nv.node_id WHERE c.source_instance_id=$1 AND c.successor_instance_id=$2 AND c.plan_sha256=$3")
        .bind(r.source_instance_id).bind(r.successor_instance_id).bind(plan_sha).fetch_optional(pool).await?.ok_or_else(||reject("closure/receipt missing"))?;
    let context: Value = row.get(5);
    if row.get::<Uuid, _>(1) != p.successor_definition_version_id
        || row.get::<i16, _>(2) != 1
        || row.get::<Uuid, _>(3) != r.successor_context_revision_id
        || row.get::<Uuid, _>(4) != r.successor_initial_visit_id
        || row.get::<Uuid, _>(6) != p.publisher_principal
        || row.get::<String, _>(7) != "publishing"
        || context["publisher_principal"] != json!(p.publisher_principal)
        || context["operator_principal"] != json!(p.operator_principal)
    {
        return Err(reject("successor verification mismatch"));
    }
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM workflow_node_visits WHERE workflow_instance_id=$1),(SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1),(SELECT count(*) FROM workflow_context_revisions WHERE workflow_instance_id=$1)").bind(r.source_instance_id).fetch_one(pool).await?;
    if counts
        != (
            r.source_expected_visit_count,
            r.source_expected_event_count,
            r.source_expected_context_count,
        )
    {
        return Err(reject("historical source facts changed"));
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
    let path = arg(&args, "--plan")?;
    let (p, _, actual) = load(&path)?;
    if mode != "plan" {
        let expected = arg(&args, "--plan-sha256")?;
        if actual != expected {
            return Err(reject("plan SHA-256 mismatch"));
        }
    }
    let db = env::var("DATABASE_URL").map_err(|_| reject("DATABASE_URL required"))?;
    let pool = PgPoolOptions::new().max_connections(2).connect(&db).await?;
    assert_db_identity(&pool, &p).await?;
    match mode {
        "plan" => {
            let mut tx = pool.begin().await?;
            sqlx::query("SET TRANSACTION READ ONLY")
                .execute(&mut *tx)
                .await?;
            for r in &p.rows {
                assert_source(&mut tx, &p, r).await?;
            }
            tx.rollback().await?;
            println!("PLAN_OK sha256={actual} rows=4");
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
                "use plan|apply|verify; apply requires --row 1..4 and --receipt-out",
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
    fn t1_exact_frozen_plan_is_accepted() {
        validate_closed_plan(&frozen()).unwrap();
    }

    #[test]
    fn t4_wrong_model_is_rejected() {
        let mut p = frozen();
        p.successor_model = 2;
        assert!(validate_closed_plan(&p).is_err());
    }

    #[test]
    fn t5_noncanonical_successor_principal_is_rejected() {
        let mut p = frozen();
        p.publisher_principal = Uuid::parse_str("d2bec623-56ba-46cb-a217-933cfe63a243").unwrap();
        assert!(validate_closed_plan(&p).is_err());
    }

    #[test]
    fn t6_duplicate_preassigned_uuid_is_rejected() {
        let mut p = frozen();
        p.rows[1].successor_instance_id = p.rows[0].successor_instance_id;
        assert!(validate_closed_plan(&p).is_err());
    }
}
