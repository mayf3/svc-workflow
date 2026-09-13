//! Bounded offline CIR successor-Instance cutover operator.
//! Exactly four frozen source Instances are accepted; arbitrary IDs are impossible.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use std::{env, fs, process};
use svc_workflow::auth::admission::{AdmissionClient, AdmissionConfig};
use svc_workflow::store::postgres::admission_gate::AdmissionGate;
use uuid::Uuid;

const SCHEMA: &str = "legacy_workflow_successor_cutover_plan_v1";
const RECEIPT_SCHEMA: &str = "legacy_workflow_successor_cutover_receipt_v1";
const FAMILY: &str = "LEGACY_WORKFLOW_SUCCESSOR_CUTOVER_V1";
const RECEIPT_COMMAND_TYPE: &str = "WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V1";
const AUTHORITY_REFS: [&str; 4] = [
    "SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2/CTR-CIR-003",
    "SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2/CTR-CIR-004",
    "SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2/CTR-CIR-006",
    "SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2/CTR-CIR-007",
];
const LOCK: &str = "legacy_workflow_successor_cutover_v1";
const DOMAIN: &str = "f9b5682c-1c9c-5ace-a7b1-01b9a3be5965";
const LEGACY_PUBLISHER: &str = "d2bec623-56ba-46cb-a217-933cfe63a243";
const LEGACY_OPERATOR: &str = "aed2c679-71c0-4e23-8d73-c448c5ea6dd2";
const LEGACY_CREATOR: &str = "bb9d8f48-7962-4321-8fb1-554bb428c159";
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct DatabaseIdentity {
    database_name: String,
    database_oid: u32,
    system_identifier: String,
    server_address: String,
    server_port: i32,
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

async fn database_identity(pool: &PgPool, p: &Plan) -> Result<DatabaseIdentity> {
    let row = sqlx::query(
        "SELECT current_database() AS database_name, \
                (SELECT oid::bigint FROM pg_database WHERE datname=current_database()) AS database_oid, \
                (pg_control_system()).system_identifier::text AS system_identifier, \
                COALESCE(inet_server_addr()::text,'local') AS server_address, \
                current_setting('port')::int AS server_port",
    )
    .fetch_one(pool)
    .await?;
    let identity = DatabaseIdentity {
        database_name: row.get("database_name"),
        database_oid: row.get::<i64, _>("database_oid") as u32,
        system_identifier: row.get("system_identifier"),
        server_address: row.get("server_address"),
        server_port: row.get("server_port"),
    };
    if identity.database_name != p.database_name {
        return Err(reject(format!(
            "database identity mismatch: {}",
            identity.database_name
        )));
    }
    Ok(identity)
}
async fn assert_source<'a>(
    tx: &mut Transaction<'a, Postgres>,
    p: &Plan,
    r: &PlanRow,
) -> Result<()> {
    let q = sqlx::query("SELECT wi.workflow_state_version,wi.current_node_visit_id,wi.current_context_revision_id,wi.definition_version_id,wi.semantic_model_version,wi.cancelled,wi.archived_at,cr.payload,n.node_key,nv.assignee_principal_id,wi.domain_id,wi.created_by_principal_id FROM workflow_instances wi JOIN workflow_context_revisions cr ON cr.context_revision_id=wi.current_context_revision_id JOIN workflow_node_visits nv ON nv.node_visit_id=wi.current_node_visit_id JOIN workflow_node_definitions n ON n.node_id=nv.node_id WHERE wi.workflow_instance_id=$1")
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
        || q.get::<Uuid, _>(10) != Uuid::parse_str(DOMAIN).unwrap()
        || q.get::<Uuid, _>(11) != Uuid::parse_str(LEGACY_CREATOR).unwrap()
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
    Ok(())
}

async fn lock_group_sources(tx: &mut Transaction<'_, Postgres>, p: &Plan) -> Result<()> {
    let mut source_ids = p
        .rows
        .iter()
        .map(|r| r.source_instance_id)
        .collect::<Vec<_>>();
    source_ids.sort();
    let mut visit_ids = p
        .rows
        .iter()
        .map(|r| r.source_expected_current_visit)
        .collect::<Vec<_>>();
    visit_ids.sort();
    let mut context_ids = p
        .rows
        .iter()
        .map(|r| r.source_expected_context_revision)
        .collect::<Vec<_>>();
    context_ids.sort();
    let locked_sources:Vec<Uuid>=sqlx::query_scalar("SELECT workflow_instance_id FROM workflow_instances WHERE workflow_instance_id=ANY($1) ORDER BY workflow_instance_id FOR UPDATE").bind(&source_ids).fetch_all(&mut **tx).await?;
    if locked_sources != source_ids {
        return Err(reject("one or more frozen sources missing"));
    }
    let locked_visits:Vec<Uuid>=sqlx::query_scalar("SELECT node_visit_id FROM workflow_node_visits WHERE node_visit_id=ANY($1) ORDER BY node_visit_id FOR UPDATE").bind(&visit_ids).fetch_all(&mut **tx).await?;
    if locked_visits != visit_ids {
        return Err(reject("one or more frozen current Visits missing"));
    }
    let locked_contexts:Vec<Uuid>=sqlx::query_scalar("SELECT context_revision_id FROM workflow_context_revisions WHERE context_revision_id=ANY($1) ORDER BY context_revision_id FOR UPDATE").bind(&context_ids).fetch_all(&mut **tx).await?;
    if locked_contexts != context_ids {
        return Err(reject("one or more frozen current Contexts missing"));
    }
    let assistance:Vec<Uuid>=sqlx::query_scalar("SELECT assistance_case_id FROM workflow_assistance_cases WHERE workflow_instance_id=ANY($1) AND status IN ('OWNER_PENDING','HUMAN_REQUIRED') ORDER BY assistance_case_id FOR UPDATE").bind(&source_ids).fetch_all(&mut **tx).await?;
    if !assistance.is_empty() {
        return Err(reject(
            "active Assistance conflicts with connected group cutover",
        ));
    }
    Ok(())
}

async fn lock_and_assert_group_dependencies(
    tx: &mut Transaction<'_, Postgres>,
    p: &Plan,
    source_status: &str,
) -> Result<()> {
    let domain = Uuid::parse_str(DOMAIN).unwrap();
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM domains WHERE domain_id=$1 FOR UPDATE")
            .bind(domain)
            .fetch_optional(&mut **tx)
            .await?;
    if enabled != Some(true) {
        return Err(reject("frozen Domain missing or disabled"));
    }

    let version = sqlx::query(
        "SELECT dv.version_status::text AS status,dv.semantic_model_version,wd.domain_id \
         FROM workflow_definition_versions dv JOIN workflow_definitions wd USING(workflow_definition_id) \
         WHERE dv.definition_version_id=$1 FOR UPDATE OF dv,wd",
    )
    .bind(p.source_definition_version_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| reject("source DefinitionVersion missing"))?;
    if version.get::<String, _>("status") != source_status
        || version.get::<i16, _>("semantic_model_version") != 1
        || version.get::<Uuid, _>("domain_id") != domain
    {
        return Err(reject("source DefinitionVersion/Domain/model drift"));
    }

    let ids = [
        Uuid::parse_str(LEGACY_PUBLISHER).unwrap(),
        Uuid::parse_str(LEGACY_OPERATOR).unwrap(),
        Uuid::parse_str(LEGACY_CREATOR).unwrap(),
        p.publisher_principal,
        p.operator_principal,
        p.successor_creator_principal,
    ];
    let mut sorted = ids.to_vec();
    sorted.sort();
    for id in sorted {
        let row = sqlx::query("SELECT principal_type::text AS kind,enabled FROM principals WHERE principal_id=$1 FOR UPDATE")
            .bind(id).fetch_optional(&mut **tx).await?.ok_or_else(|| reject(format!("principal {id} missing")))?;
        if row.get::<String, _>("kind") != "AGENT" || !row.get::<bool, _>("enabled") {
            return Err(reject(format!("principal {id} is not enabled AGENT")));
        }
    }
    let lineage: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT source_principal_id,successor_principal_id FROM workflow_identity_successor_lines \
         WHERE source_principal_id=ANY($1) ORDER BY source_principal_id FOR UPDATE",
    )
    .bind(vec![
        Uuid::parse_str(LEGACY_CREATOR).unwrap(),
        Uuid::parse_str(LEGACY_OPERATOR).unwrap(),
        Uuid::parse_str(LEGACY_PUBLISHER).unwrap(),
    ])
    .fetch_all(&mut **tx)
    .await?;
    let expected = [
        (
            Uuid::parse_str(LEGACY_CREATOR).unwrap(),
            p.successor_creator_principal,
        ),
        (
            Uuid::parse_str(LEGACY_OPERATOR).unwrap(),
            p.operator_principal,
        ),
        (
            Uuid::parse_str(LEGACY_PUBLISHER).unwrap(),
            p.publisher_principal,
        ),
    ];
    if expected.iter().any(|pair| !lineage.contains(pair)) || lineage.len() != 3 {
        return Err(reject("exact identity successor lineage drift"));
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
    let deprecated=sqlx::query("UPDATE workflow_definition_versions SET version_status='DEPRECATED',deprecated_at=now(),deprecated_by_principal_id=$1,deprecation_reason='CIR_CANONICAL_SUCCESSOR' WHERE definition_version_id=$2 AND version_status='PUBLISHED'")
        .bind(p.operator_principal).bind(p.source_definition_version_id).execute(&mut **tx).await?;
    if deprecated.rows_affected() != 1 {
        return Err(reject("source DefinitionVersion deprecation CAS failed"));
    }
    Ok(())
}

async fn assert_target_definition(tx: &mut Transaction<'_, Postgres>, p: &Plan) -> Result<()> {
    let v=sqlx::query("SELECT version_number,version_status::text,definition_digest,semantic_model_version FROM workflow_definition_versions WHERE definition_version_id=$1")
        .bind(p.successor_definition_version_id).fetch_optional(&mut **tx).await?.ok_or_else(||reject("successor DefinitionVersion missing"))?;
    if v.get::<i32, _>(0) != p.successor_version_number
        || v.get::<String, _>(1) != "PUBLISHED"
        || v.get::<Option<String>, _>(2).as_deref() != Some(&p.successor_definition_digest)
        || v.get::<i16, _>(3) != 1
    {
        return Err(reject(
            "successor DefinitionVersion exact poststate mismatch",
        ));
    }
    let nodes:Vec<(Uuid,String,Option<Uuid>)>=sqlx::query_as("SELECT node_id,node_key,fixed_principal_id FROM workflow_node_definitions WHERE definition_version_id=$1 ORDER BY node_key").bind(p.successor_definition_version_id).fetch_all(&mut **tx).await?;
    let expected_nodes = vec![
        (
            p.target_node_ids.dispatch,
            "dispatch".into(),
            Some(p.operator_principal),
        ),
        (p.target_node_ids.done, "done".into(), None),
        (
            p.target_node_ids.publishing,
            "publishing".into(),
            Some(p.publisher_principal),
        ),
        (
            p.target_node_ids.verify,
            "verify".into(),
            Some(p.operator_principal),
        ),
    ];
    if nodes != expected_nodes {
        return Err(reject("successor Definition node graph mismatch"));
    }
    let transitions:Vec<(Uuid,String)>=sqlx::query_as("SELECT transition_id,transition_key FROM workflow_transition_definitions WHERE definition_version_id=$1 ORDER BY transition_key").bind(p.successor_definition_version_id).fetch_all(&mut **tx).await?;
    let expected_transitions = vec![
        (
            p.target_transition_ids.advance_to_publishing,
            "advance-to-publishing".into(),
        ),
        (
            p.target_transition_ids.advance_to_verify,
            "advance-to-verify".into(),
        ),
        (p.target_transition_ids.verified, "verified".into()),
        (
            p.target_transition_ids.verify_failed,
            "verify-failed".into(),
        ),
    ];
    if transitions != expected_transitions {
        return Err(reject("successor Definition transition graph mismatch"));
    }
    Ok(())
}

fn derived_uuid(seed: Uuid, label: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn canonical_digest(value: &Value) -> Result<String> {
    jcs_canonicalize::sha256_jcs_hex(value).map_err(|e| reject(e.to_string()))
}

fn group_identity(p: &Plan) -> Result<String> {
    canonical_digest(&json!({
        "sourceDefinitionVersionId": p.source_definition_version_id,
        "successorDefinitionVersionId": p.successor_definition_version_id,
        "sources": p.rows.iter().map(|r| r.source_instance_id).collect::<Vec<_>>(),
        "successors": p.rows.iter().map(|r| r.successor_instance_id).collect::<Vec<_>>(),
        "commands": p.rows.iter().map(|r| r.command_id).collect::<Vec<_>>()
    }))
}

fn request_envelope(
    p: &Plan,
    plan_sha: &str,
    db: &DatabaseIdentity,
    actor: Uuid,
    operator_code: &str,
) -> Result<Value> {
    Ok(json!({
        "commandType": RECEIPT_COMMAND_TYPE,
        "authorityRefs": AUTHORITY_REFS,
        "operatorCodeCoordinate": operator_code,
        "planSha256": plan_sha,
        "databaseIdentity": db,
        "actorPrincipalId": actor,
        "groupIdentityDigest": group_identity(p)?,
        "sourceInstanceIds": p.rows.iter().map(|r| r.source_instance_id).collect::<Vec<_>>(),
        "successorInstanceIds": p.rows.iter().map(|r| r.successor_instance_id).collect::<Vec<_>>()
    }))
}

fn group_receipt(
    p: &Plan,
    plan_sha: &str,
    db: &DatabaseIdentity,
    actor: Uuid,
    operator_code: &str,
    request_hash: &str,
) -> Result<Value> {
    Ok(json!({
        "schema": RECEIPT_SCHEMA,
        "commandType": RECEIPT_COMMAND_TYPE,
        "authorityRefs": AUTHORITY_REFS,
        "operatorCodeCoordinate": operator_code,
        "planSha256": plan_sha,
        "databaseIdentity": db,
        "actorPrincipalId": actor,
        "groupIdentityDigest": group_identity(p)?,
        "requestHash": request_hash,
        "rows": p.rows.iter().map(|r| json!({
            "sourceInstanceId": r.source_instance_id,
            "successorInstanceId": r.successor_instance_id,
            "commandId": r.command_id,
            "sourceClosure": r.source_closure_fact
        })).collect::<Vec<_>>()
    }))
}

async fn lock_receipt_and_lineage_identity(
    tx: &mut Transaction<'_, Postgres>,
    p: &Plan,
) -> Result<usize> {
    let mut commands = p
        .rows
        .iter()
        .flat_map(|r| [r.command_id, derived_uuid(r.command_id, "source-command")])
        .collect::<Vec<_>>();
    commands.sort();
    let receipts: Vec<Uuid> = sqlx::query_scalar(
        "SELECT command_id FROM workflow_command_receipts WHERE command_id=ANY($1) ORDER BY command_id FOR UPDATE",
    ).bind(&commands).fetch_all(&mut **tx).await?;
    let mut sources = p
        .rows
        .iter()
        .map(|r| r.source_instance_id)
        .collect::<Vec<_>>();
    sources.sort();
    let closures: Vec<Uuid> = sqlx::query_scalar(
        "SELECT source_instance_id FROM workflow_instance_successor_cutovers WHERE source_instance_id=ANY($1) ORDER BY source_instance_id FOR UPDATE",
    ).bind(&sources).fetch_all(&mut **tx).await?;
    if receipts.len() != closures.len() * 2 || ![0, 4].contains(&closures.len()) {
        return Err(reject(
            "partial/incompatible group receipt or closure state",
        ));
    }
    Ok(closures.len())
}

async fn assert_no_target_collisions(tx: &mut Transaction<'_, Postgres>, p: &Plan) -> Result<()> {
    let graph_collision:i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM workflow_definition_versions WHERE definition_version_id=$1)+(SELECT count(*) FROM workflow_node_definitions WHERE node_id=ANY($2))+(SELECT count(*) FROM workflow_transition_definitions WHERE transition_id=ANY($3))")
        .bind(p.successor_definition_version_id)
        .bind(vec![p.target_node_ids.dispatch,p.target_node_ids.publishing,p.target_node_ids.verify,p.target_node_ids.done])
        .bind(vec![p.target_transition_ids.advance_to_publishing,p.target_transition_ids.advance_to_verify,p.target_transition_ids.verified,p.target_transition_ids.verify_failed])
        .fetch_one(&mut **tx).await?;
    if graph_collision != 0 {
        return Err(reject("preassigned successor Definition graph collision"));
    }
    for r in &p.rows {
        let collision: i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM workflow_instances WHERE workflow_instance_id=$1)+(SELECT count(*) FROM workflow_node_visits WHERE node_visit_id=$2)+(SELECT count(*) FROM workflow_context_revisions WHERE context_revision_id=$3)+(SELECT count(*) FROM workflow_events WHERE event_id IN ($4,$6))+(SELECT count(*) FROM workflow_command_receipts WHERE command_id IN ($5,$8))+(SELECT count(*) FROM workflow_security_audits WHERE audit_id=$7)")
            .bind(r.successor_instance_id).bind(r.successor_initial_visit_id).bind(r.successor_context_revision_id).bind(r.successor_event_id).bind(r.command_id)
            .bind(derived_uuid(r.command_id,"source-event")).bind(derived_uuid(r.command_id,"security-audit"))
            .bind(derived_uuid(r.command_id,"source-command"))
            .fetch_one(&mut **tx).await?;
        if collision != 0 {
            return Err(reject("preassigned successor/group UUID collision"));
        }
    }
    Ok(())
}

async fn apply(
    pool: &PgPool,
    p: &Plan,
    plan_sha: &str,
    db: &DatabaseIdentity,
    actor: Uuid,
    operator_code: &str,
    admission: &AdmissionClient,
    inject: bool,
) -> Result<Value> {
    if actor != p.operator_principal {
        return Err(reject("actual actor is not frozen operator principal"));
    }
    let gate = AdmissionGate::new(Some(admission)).with_pool(Some(pool));
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;
    gate.bind_statement_deadline(&mut tx)
        .await
        .map_err(|e| reject(e.to_string()))?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(LOCK)
        .execute(&mut *tx)
        .await?;
    let existing = lock_receipt_and_lineage_identity(&mut tx, p).await?;
    if existing == 4 {
        lock_group_sources(&mut tx, p).await?;
        lock_and_assert_group_dependencies(&mut tx, p, "DEPRECATED").await?;
        let receipt = verify_group_tx(&mut tx, p, plan_sha, db, actor, operator_code).await?;
        tx.rollback().await?;
        return Ok(receipt);
    }

    lock_group_sources(&mut tx, p).await?;
    let mut rows = p.rows.iter().collect::<Vec<_>>();
    rows.sort_by_key(|r| r.source_instance_id);
    for r in &rows {
        assert_source(&mut tx, p, r).await?;
    }
    lock_and_assert_group_dependencies(&mut tx, p, "PUBLISHED").await?;
    assert_no_target_collisions(&mut tx, p).await?;

    gate.admit([
        p.publisher_principal,
        p.operator_principal,
        p.successor_creator_principal,
    ])
    .await
    .map_err(|e| {
        reject(format!(
            "fresh canonical directory admission failed: {}",
            e.sanitized_code()
        ))
    })?;

    ensure_target_definition(&mut tx, p).await?;
    let request = request_envelope(p, plan_sha, db, actor, operator_code)?;
    let request_hash = canonical_digest(&request)?;
    let receipt = group_receipt(p, plan_sha, db, actor, operator_code, &request_hash)?;
    let response_digest = canonical_digest(&receipt)?;
    let group_digest = group_identity(p)?;

    for r in &p.rows {
        sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash) VALUES($1,$2,$3,$4,$5)")
            .bind(r.command_id).bind(actor).bind(format!("{RECEIPT_COMMAND_TYPE}:{plan_sha}:GROUP:{}",r.command_id)).bind(RECEIPT_COMMAND_TYPE).bind(&request_hash).execute(&mut *tx).await?;
        let source_command = derived_uuid(r.command_id, "source-command");
        sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash) VALUES($1,$2,$3,$4,$5)")
            .bind(source_command).bind(actor).bind(format!("{RECEIPT_COMMAND_TYPE}:{plan_sha}:GROUP_SOURCE:{}",r.command_id)).bind(RECEIPT_COMMAND_TYPE).bind(&request_hash).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO workflow_instances(workflow_instance_id,domain_id,definition_version_id,created_by_principal_id,current_context_revision_id,current_node_visit_id,workflow_state_version,external_url,metadata,external_reference,require_explicit_node_assignees,subject_id,artifact_id,artifact_version,artifact_digest,require_artifact_binding,semantic_model_version,execution_class) SELECT $1,domain_id,$2,$3,$4,$5,1,external_url,metadata,NULL,require_explicit_node_assignees,subject_id,artifact_id,artifact_version,artifact_digest,require_artifact_binding,1,execution_class FROM workflow_instances WHERE workflow_instance_id=$6")
            .bind(r.successor_instance_id).bind(p.successor_definition_version_id).bind(p.successor_creator_principal).bind(r.successor_context_revision_id).bind(r.successor_initial_visit_id).bind(r.source_instance_id).execute(&mut *tx).await?;
        let mut context = r.source_expected_context.clone();
        context["operator_principal"] = json!(p.operator_principal);
        context["publisher_principal"] = json!(p.publisher_principal);
        let context_digest = canonical_digest(&context)?;
        sqlx::query("INSERT INTO workflow_context_revisions(context_revision_id,workflow_instance_id,revision_number,previous_revision_id,payload,payload_digest,created_by_principal_id) VALUES($1,$2,1,NULL,$3,$4,$5)")
            .bind(r.successor_context_revision_id).bind(r.successor_instance_id).bind(&context).bind(&context_digest).bind(actor).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,1,$4,NULL)")
            .bind(r.successor_initial_visit_id).bind(r.successor_instance_id).bind(p.target_node_ids.publishing).bind(p.publisher_principal).execute(&mut *tx).await?;
        let successor_data = json!({"cirSourceInstanceId":r.source_instance_id,"definitionVersionId":p.successor_definition_version_id,"initialNodeId":p.target_node_ids.publishing,"closureReason":r.source_closure_fact,"planSha256":plan_sha,"groupIdentityDigest":group_digest});
        let successor_digest = canonical_digest(&successor_data)?;
        sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,command_id,event_type,target_node_visit_id,context_revision_id,event_data,event_data_digest,actor_principal_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,1,'v1',$3,'INSTANCE_CREATED',$4,$5,$6,$7,$8,0,1)")
            .bind(r.successor_event_id).bind(r.successor_instance_id).bind(r.command_id).bind(r.successor_initial_visit_id).bind(r.successor_context_revision_id).bind(&successor_data).bind(&successor_digest).bind(actor).execute(&mut *tx).await?;
        let source_event = derived_uuid(r.command_id, "source-event");
        let source_node: Uuid =
            sqlx::query_scalar("SELECT node_id FROM workflow_node_visits WHERE node_visit_id=$1")
                .bind(r.source_expected_current_visit)
                .fetch_one(&mut *tx)
                .await?;
        let source_data = json!({"successorInstanceId":r.successor_instance_id,"closureReason":r.source_closure_fact,"planSha256":plan_sha,"groupIdentityDigest":group_digest});
        let source_digest = canonical_digest(&source_data)?;
        let source_update=sqlx::query("UPDATE workflow_instances SET workflow_state_version=$1,updated_at=now() WHERE workflow_instance_id=$2 AND workflow_state_version=$3 AND current_node_visit_id=$4")
            .bind(r.source_expected_state_version+1).bind(r.source_instance_id).bind(r.source_expected_state_version).bind(r.source_expected_current_visit).execute(&mut *tx).await?;
        if source_update.rows_affected() != 1 {
            return Err(reject("source projection CAS failed"));
        }
        sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,command_id,event_type,source_node_visit_id,target_node_visit_id,context_revision_id,event_data,event_data_digest,actor_principal_id,from_node_id,to_node_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,$3,'v1',$4,'INSTANCE_SUPERSEDED_BY_CANONICAL_SUCCESSOR',$5,$5,$6,$7,$8,$9,$10,$10,$11,$3)")
            .bind(source_event).bind(r.source_instance_id).bind(r.source_expected_state_version+1).bind(source_command).bind(r.source_expected_current_visit).bind(r.source_expected_context_revision).bind(&source_data).bind(&source_digest).bind(actor).bind(source_node).bind(r.source_expected_state_version).execute(&mut *tx).await?;
        sqlx::query("UPDATE workflow_command_receipts SET receipt_status='COMPLETED',response_status=200,response_body=$1,response_digest=$2,completed_at=now() WHERE command_id=$3 AND receipt_status='PROCESSING'")
            .bind(&receipt).bind(&response_digest).bind(r.command_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE workflow_command_receipts SET receipt_status='COMPLETED',response_status=200,response_body=$1,response_digest=$2,completed_at=now() WHERE command_id=$3 AND receipt_status='PROCESSING'")
            .bind(&receipt).bind(&response_digest).bind(source_command).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO workflow_security_audits(audit_id,principal_id,action,resource_type,resource_id,details) VALUES($1,$2,'WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V1_COMMITTED','WORKFLOW_INSTANCE_GROUP',$3,$4)")
            .bind(derived_uuid(r.command_id,"security-audit")).bind(actor).bind(&group_digest).bind(&receipt).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO workflow_instance_successor_cutovers(source_instance_id,successor_instance_id,source_definition_version_id,successor_definition_version_id,source_current_visit_id,source_context_revision_id,successor_initial_visit_id,successor_context_revision_id,source_node_id,successor_node_id,source_expected_state_version,model,closure_reason,plan_sha256,group_identity_digest,request_hash,command_id,operator_principal_id,original_creator_principal_id,successor_creator_principal_id,approved_identity_changes,database_identity,authority_refs,operator_code_coordinate,receipt) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,1,'CIR_CANONICAL_SUCCESSOR',$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23)")
            .bind(r.source_instance_id).bind(r.successor_instance_id).bind(p.source_definition_version_id).bind(p.successor_definition_version_id).bind(r.source_expected_current_visit).bind(r.source_expected_context_revision).bind(r.successor_initial_visit_id).bind(r.successor_context_revision_id).bind(source_node).bind(p.target_node_ids.publishing).bind(r.source_expected_state_version).bind(plan_sha).bind(&group_digest).bind(&request_hash).bind(r.command_id).bind(actor).bind(Uuid::parse_str(LEGACY_CREATOR).unwrap()).bind(p.successor_creator_principal).bind(json!({"operator_principal":{"from":LEGACY_OPERATOR,"to":p.operator_principal},"publisher_principal":{"from":LEGACY_PUBLISHER,"to":p.publisher_principal}})).bind(json!(db)).bind(json!(AUTHORITY_REFS)).bind(operator_code).bind(&receipt).execute(&mut *tx).await?;
    }
    if inject {
        return Err(reject("injected failure before group postcondition/commit"));
    }
    gate.check_commit_budget()
        .map_err(|e| reject(e.to_string()))?;
    verify_group_tx(&mut tx, p, plan_sha, db, actor, operator_code).await?;
    tx.commit().await?;
    Ok(receipt)
}

async fn verify_group_tx(
    tx: &mut Transaction<'_, Postgres>,
    p: &Plan,
    plan_sha: &str,
    db: &DatabaseIdentity,
    actor: Uuid,
    operator_code: &str,
) -> Result<Value> {
    let request = request_envelope(p, plan_sha, db, actor, operator_code)?;
    let request_hash = canonical_digest(&request)?;
    let expected_receipt = group_receipt(p, plan_sha, db, actor, operator_code, &request_hash)?;
    let response_digest = canonical_digest(&expected_receipt)?;
    let group_digest = group_identity(p)?;
    let statuses: (String,String) = sqlx::query_as("SELECT (SELECT version_status::text FROM workflow_definition_versions WHERE definition_version_id=$1),(SELECT version_status::text FROM workflow_definition_versions WHERE definition_version_id=$2)").bind(p.source_definition_version_id).bind(p.successor_definition_version_id).fetch_one(&mut **tx).await?;
    if statuses != ("DEPRECATED".into(), "PUBLISHED".into()) {
        return Err(reject("source/target DefinitionVersion poststate mismatch"));
    }
    assert_target_definition(tx, p).await?;
    for r in &p.rows {
        let c = sqlx::query("SELECT successor_instance_id,source_definition_version_id,successor_definition_version_id,source_current_visit_id,source_context_revision_id,successor_initial_visit_id,successor_context_revision_id,source_expected_state_version,model,closure_reason,plan_sha256,group_identity_digest,request_hash,command_id,operator_principal_id,original_creator_principal_id,successor_creator_principal_id,database_identity,authority_refs,operator_code_coordinate,receipt,source_node_id,successor_node_id,approved_identity_changes FROM workflow_instance_successor_cutovers WHERE source_instance_id=$1")
            .bind(r.source_instance_id).fetch_optional(&mut **tx).await?.ok_or_else(||reject("group closure missing"))?;
        let source_node: Uuid =
            sqlx::query_scalar("SELECT node_id FROM workflow_node_visits WHERE node_visit_id=$1")
                .bind(r.source_expected_current_visit)
                .fetch_one(&mut **tx)
                .await?;
        if c.get::<Uuid, _>(0) != r.successor_instance_id
            || c.get::<Uuid, _>(1) != p.source_definition_version_id
            || c.get::<Uuid, _>(2) != p.successor_definition_version_id
            || c.get::<Uuid, _>(3) != r.source_expected_current_visit
            || c.get::<Uuid, _>(4) != r.source_expected_context_revision
            || c.get::<Uuid, _>(5) != r.successor_initial_visit_id
            || c.get::<Uuid, _>(6) != r.successor_context_revision_id
            || c.get::<i32, _>(7) != r.source_expected_state_version
            || c.get::<i16, _>(8) != 1
            || c.get::<String, _>(9) != "CIR_CANONICAL_SUCCESSOR"
            || c.get::<String, _>(10) != plan_sha
            || c.get::<String, _>(11) != group_digest
            || c.get::<String, _>(12) != request_hash
            || c.get::<Uuid, _>(13) != r.command_id
            || c.get::<Uuid, _>(14) != actor
            || c.get::<Uuid, _>(15) != Uuid::parse_str(LEGACY_CREATOR).unwrap()
            || c.get::<Uuid, _>(16) != p.successor_creator_principal
            || c.get::<Value, _>(17) != json!(db)
            || c.get::<Value, _>(18) != json!(AUTHORITY_REFS)
            || c.get::<String, _>(19) != operator_code
            || c.get::<Value, _>(20) != expected_receipt
            || c.get::<Uuid, _>(21) != source_node
            || c.get::<Uuid, _>(22) != p.target_node_ids.publishing
            || c.get::<Value, _>(23)
                != json!({"operator_principal":{"from":LEGACY_OPERATOR,"to":p.operator_principal},"publisher_principal":{"from":LEGACY_PUBLISHER,"to":p.publisher_principal}})
        {
            return Err(reject("exact immutable closure poststate mismatch"));
        }
        let receipt=sqlx::query("SELECT principal_id,command_type,request_hash,receipt_status::text,response_status,response_body,response_digest FROM workflow_command_receipts WHERE command_id=$1").bind(r.command_id).fetch_optional(&mut **tx).await?.ok_or_else(||reject("group command receipt missing"))?;
        if receipt.get::<Uuid, _>(0) != actor
            || receipt.get::<String, _>(1) != RECEIPT_COMMAND_TYPE
            || receipt.get::<String, _>(2) != request_hash
            || receipt.get::<String, _>(3) != "COMPLETED"
            || receipt.get::<Option<i32>, _>(4) != Some(200)
            || receipt.get::<Option<Value>, _>(5).as_ref() != Some(&expected_receipt)
            || receipt.get::<Option<String>, _>(6).as_deref() != Some(&response_digest)
        {
            return Err(reject("exact completed receipt poststate mismatch"));
        }
        let source_command = derived_uuid(r.command_id, "source-command");
        let source_receipt=sqlx::query("SELECT principal_id,command_type,request_hash,receipt_status::text,response_status,response_body,response_digest FROM workflow_command_receipts WHERE command_id=$1").bind(source_command).fetch_optional(&mut **tx).await?.ok_or_else(||reject("source migration receipt missing"))?;
        if source_receipt.get::<Uuid, _>(0) != actor
            || source_receipt.get::<String, _>(1) != RECEIPT_COMMAND_TYPE
            || source_receipt.get::<String, _>(2) != request_hash
            || source_receipt.get::<String, _>(3) != "COMPLETED"
            || source_receipt.get::<Option<i32>, _>(4) != Some(200)
            || source_receipt.get::<Option<Value>, _>(5).as_ref() != Some(&expected_receipt)
            || source_receipt.get::<Option<String>, _>(6).as_deref() != Some(&response_digest)
        {
            return Err(reject("exact source migration receipt poststate mismatch"));
        }
        let successor=sqlx::query("SELECT s.domain_id,s.definition_version_id,s.created_by_principal_id,s.current_context_revision_id,s.current_node_visit_id,s.workflow_state_version,s.external_reference,s.semantic_model_version,s.cancelled,s.archived_at,src.domain_id AS source_domain,src.external_url AS source_url,s.external_url AS successor_url,src.metadata AS source_metadata,s.metadata AS successor_metadata,(s.require_explicit_node_assignees=src.require_explicit_node_assignees AND s.subject_id IS NOT DISTINCT FROM src.subject_id AND s.artifact_id IS NOT DISTINCT FROM src.artifact_id AND s.artifact_version IS NOT DISTINCT FROM src.artifact_version AND s.artifact_digest IS NOT DISTINCT FROM src.artifact_digest AND s.require_artifact_binding=src.require_artifact_binding AND s.execution_class=src.execution_class) AS preserved_runtime_contract FROM workflow_instances s JOIN workflow_instances src ON src.workflow_instance_id=$2 WHERE s.workflow_instance_id=$1").bind(r.successor_instance_id).bind(r.source_instance_id).fetch_optional(&mut **tx).await?.ok_or_else(||reject("successor instance missing"))?;
        if successor.get::<Uuid, _>(0) != Uuid::parse_str(DOMAIN).unwrap()
            || successor.get::<Uuid, _>(1) != p.successor_definition_version_id
            || successor.get::<Uuid, _>(2) != p.successor_creator_principal
            || successor.get::<Uuid, _>(3) != r.successor_context_revision_id
            || successor.get::<Uuid, _>(4) != r.successor_initial_visit_id
            || successor.get::<i32, _>(5) != 1
            || successor.get::<Option<String>, _>(6).is_some()
            || successor.get::<i16, _>(7) != 1
            || successor.get::<bool, _>(8)
            || successor
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>(9)
                .is_some()
            || successor.get::<Uuid, _>(10) != Uuid::parse_str(DOMAIN).unwrap()
            || successor.get::<Option<String>, _>(11) != successor.get::<Option<String>, _>(12)
            || successor.get::<Option<Value>, _>(13) != successor.get::<Option<Value>, _>(14)
            || !successor.get::<bool, _>(15)
        {
            return Err(reject("exact successor instance poststate mismatch"));
        }
        let mut context = r.source_expected_context.clone();
        context["operator_principal"] = json!(p.operator_principal);
        context["publisher_principal"] = json!(p.publisher_principal);
        let ctx=sqlx::query("SELECT revision_number,previous_revision_id,payload,payload_digest,created_by_principal_id FROM workflow_context_revisions WHERE context_revision_id=$1 AND workflow_instance_id=$2").bind(r.successor_context_revision_id).bind(r.successor_instance_id).fetch_optional(&mut **tx).await?.ok_or_else(||reject("successor context missing"))?;
        if ctx.get::<i32, _>(0) != 1
            || ctx.get::<Option<Uuid>, _>(1).is_some()
            || ctx.get::<Value, _>(2) != context
            || ctx.get::<String, _>(3) != canonical_digest(&context)?
            || ctx.get::<Uuid, _>(4) != actor
        {
            return Err(reject("canonical successor Context poststate mismatch"));
        }
        let visit=sqlx::query("SELECT nv.node_id,nv.visit_number,nv.assignee_principal_id,nv.entered_by_transition_id,n.node_key FROM workflow_node_visits nv JOIN workflow_node_definitions n USING(node_id) WHERE nv.node_visit_id=$1 AND nv.workflow_instance_id=$2").bind(r.successor_initial_visit_id).bind(r.successor_instance_id).fetch_optional(&mut **tx).await?.ok_or_else(||reject("successor visit missing"))?;
        if visit.get::<Uuid, _>(0) != p.target_node_ids.publishing
            || visit.get::<i32, _>(1) != 1
            || visit.get::<Uuid, _>(2) != p.publisher_principal
            || visit.get::<Option<Uuid>, _>(3).is_some()
            || visit.get::<String, _>(4) != "publishing"
        {
            return Err(reject("successor initial Visit poststate mismatch"));
        }
        let source=sqlx::query("SELECT workflow_state_version,current_node_visit_id,current_context_revision_id,definition_version_id,semantic_model_version,cancelled,archived_at FROM workflow_instances WHERE workflow_instance_id=$1").bind(r.source_instance_id).fetch_one(&mut **tx).await?;
        if source.get::<i32, _>(0) != r.source_expected_state_version + 1
            || source.get::<Uuid, _>(1) != r.source_expected_current_visit
            || source.get::<Uuid, _>(2) != r.source_expected_context_revision
            || source.get::<Uuid, _>(3) != p.source_definition_version_id
            || source.get::<i16, _>(4) != 1
            || source.get::<bool, _>(5)
            || source
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>(6)
                .is_some()
        {
            return Err(reject("source closure projection poststate mismatch"));
        }
        let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM workflow_node_visits WHERE workflow_instance_id=$1),(SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1),(SELECT count(*) FROM workflow_context_revisions WHERE workflow_instance_id=$1)").bind(r.source_instance_id).fetch_one(&mut **tx).await?;
        if counts
            != (
                r.source_expected_visit_count,
                r.source_expected_event_count + 1,
                r.source_expected_context_count,
            )
        {
            return Err(reject("source historical membership/poststate mismatch"));
        }
        let successor_data = json!({"cirSourceInstanceId":r.source_instance_id,"definitionVersionId":p.successor_definition_version_id,"initialNodeId":p.target_node_ids.publishing,"closureReason":r.source_closure_fact,"planSha256":plan_sha,"groupIdentityDigest":group_digest});
        let successor_event:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_events WHERE event_id=$1 AND workflow_instance_id=$2 AND command_id=$3 AND event_type='INSTANCE_CREATED' AND target_node_visit_id=$4 AND context_revision_id=$5 AND actor_principal_id=$6 AND old_workflow_state_version=0 AND new_workflow_state_version=1 AND event_data=$7 AND event_data_digest=$8").bind(r.successor_event_id).bind(r.successor_instance_id).bind(r.command_id).bind(r.successor_initial_visit_id).bind(r.successor_context_revision_id).bind(actor).bind(&successor_data).bind(canonical_digest(&successor_data)?).fetch_one(&mut **tx).await?;
        let source_data = json!({"successorInstanceId":r.successor_instance_id,"closureReason":r.source_closure_fact,"planSha256":plan_sha,"groupIdentityDigest":group_digest});
        let source_event:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_events WHERE event_id=$1 AND workflow_instance_id=$2 AND command_id=$3 AND event_type='INSTANCE_SUPERSEDED_BY_CANONICAL_SUCCESSOR' AND source_node_visit_id=$4 AND target_node_visit_id=$4 AND context_revision_id=$5 AND actor_principal_id=$6 AND old_workflow_state_version=$7 AND new_workflow_state_version=$8 AND event_data=$9 AND event_data_digest=$10").bind(derived_uuid(r.command_id,"source-event")).bind(r.source_instance_id).bind(source_command).bind(r.source_expected_current_visit).bind(r.source_expected_context_revision).bind(actor).bind(r.source_expected_state_version).bind(r.source_expected_state_version+1).bind(&source_data).bind(canonical_digest(&source_data)?).fetch_one(&mut **tx).await?;
        let audit:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_security_audits WHERE audit_id=$1 AND principal_id=$2 AND action='WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V1_COMMITTED' AND resource_type='WORKFLOW_INSTANCE_GROUP' AND resource_id=$3 AND details=$4").bind(derived_uuid(r.command_id,"security-audit")).bind(actor).bind(&group_digest).bind(&expected_receipt).fetch_one(&mut **tx).await?;
        if successor_event != 1 || source_event != 1 || audit != 1 {
            return Err(reject("required event/audit durable poststate mismatch"));
        }
    }
    let closure_count:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_instance_successor_cutovers WHERE plan_sha256=$1 AND group_identity_digest=$2").bind(plan_sha).bind(&group_digest).fetch_one(&mut **tx).await?;
    if closure_count != 4 {
        return Err(reject("group closure cardinality mismatch"));
    }
    Ok(expected_receipt)
}

async fn verify(
    pool: &PgPool,
    p: &Plan,
    plan_sha: &str,
    db: &DatabaseIdentity,
    actor: Uuid,
    operator_code: &str,
) -> Result<Value> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE READ ONLY")
        .execute(&mut *tx)
        .await?;
    let receipt = verify_group_tx(&mut tx, p, plan_sha, db, actor, operator_code).await?;
    tx.rollback().await?;
    Ok(receipt)
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
    let db_identity = database_identity(&pool, &p).await?;
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
            if args.iter().any(|x| x == "--row") { return Err(reject("per-row apply is forbidden; the four-row connected group is one commit")); }
            let actor=Uuid::parse_str(&arg(&args,"--actor-principal")?).map_err(|_|reject("bad actor principal"))?;
            let operator_code=arg(&args,"--operator-code-coordinate")?;
            let config=AdmissionConfig::from_env().map_err(|e|reject(e.to_string()))?;
            if !config.enabled { return Err(reject("fresh directory admission must be enabled for apply")); }
            let admission=AdmissionClient::new(config).map_err(|e|reject(e.to_string()))?;
            let out = match apply(
                &pool,
                &p,
                &actual,
                &db_identity,
                actor,
                &operator_code,
                &admission,
                args.iter().any(|x| x == "--inject-failure-after-create"),
            )
            .await {
                Ok(out) => out,
                Err(error) => {
                    // `main` exits non-zero on rejection. Await pool closure so PostgreSQL
                    // has durably rolled back the open transaction before the process exits.
                    pool.close().await;
                    return Err(error);
                }
            };
            let receipt = arg(&args, "--receipt-out")?;
            fs::write(&receipt, serde_json::to_vec_pretty(&out).unwrap())?;
            println!("APPLY_OK group_rows=4 receipt={receipt}");
        }
        "verify" => {
            if args.iter().any(|x| x == "--row") { return Err(reject("per-row verify is forbidden; verify covers the connected group")); }
            let actor=Uuid::parse_str(&arg(&args,"--actor-principal")?).map_err(|_|reject("bad actor principal"))?;
            if actor!=p.operator_principal { return Err(reject("actual actor is not frozen operator principal")); }
            let operator_code=arg(&args,"--operator-code-coordinate")?;
            let out = verify(
                &pool,
                &p,
                &actual,
                &db_identity,
                actor,
                &operator_code,
            )
            .await?;
            if let Ok(path) = arg(&args, "--receipt") {
                let disk: Value =
                    serde_json::from_slice(&fs::read(path)?).map_err(|e| reject(e.to_string()))?;
                if disk != out {
                    return Err(reject("receipt mismatch"));
                }
            }
            println!("VERIFY_OK group_rows=4");
        }
        _ => {
            return Err(reject(
                "use plan|apply|verify; apply is one four-row group and requires actor/operator coordinates plus --receipt-out",
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
