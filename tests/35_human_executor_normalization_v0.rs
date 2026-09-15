//! Exact-18 HUMAN executor normalization V1 conformance against disposable PostgreSQL.

#![allow(unused_imports)]

#[path = "common/mod.rs"]
mod common;

use serde_json::Value;
use sha2::Digest;
use sqlx::{Connection, Executor, PgConnection, PgPool, Row};
use std::process::{Command, Output};
use uuid::Uuid;

const HUMAN: &str = "8902db0d-429a-4e37-985c-f8b92d4b78fb";
const ACTOR: &str = "bc970ced-710f-4479-9ff0-e295a1c59424";
const EXCLUDED_TERMINAL_WORKFLOWS: [&str; 2] = [
    "2edf5b53-1dd9-4c93-b356-4029d3fe1adb",
    "f0ebdef1-8cab-4b97-82ac-af92b8ed3e12",
];
const PLAN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/evidence/human-executor-normalization-v1/exact-18-plan.tsv"
));

#[derive(Clone)]
struct PlanRow {
    workflow: Uuid,
    source: Uuid,
    version: i32,
    old_owner: Uuid,
    human: Uuid,
    target: Uuid,
    definition_version: Uuid,
    node: Uuid,
    visit_number: i32,
    context: Uuid,
    context_digest: String,
    definition_key: String,
    node_key: String,
    node_type: String,
}

fn rows() -> Vec<PlanRow> {
    PLAN.lines()
        .skip(1)
        .map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            PlanRow {
                workflow: c[0].parse().unwrap(),
                source: c[1].parse().unwrap(),
                version: c[2].parse().unwrap(),
                old_owner: c[3].parse().unwrap(),
                human: c[4].parse().unwrap(),
                target: c[5].parse().unwrap(),
                definition_version: c[6].parse().unwrap(),
                node: c[7].parse().unwrap(),
                visit_number: c[8].parse().unwrap(),
                context: c[9].parse().unwrap(),
                context_digest: c[10].into(),
                definition_key: c[11].into(),
                node_key: c[12].into(),
                node_type: c[13].into(),
            }
        })
        .collect()
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_human_executor_normalization_v0")
}

fn run(url: &str, database: &str, args: &[&str]) -> Output {
    run_with_env(url, database, args, None)
}

fn run_with_env(
    url: &str,
    database: &str,
    args: &[&str],
    extra_env: Option<(&str, &str)>,
) -> Output {
    let mut command = Command::new(binary());
    command
        .args(args)
        .env("DATABASE_URL", url)
        .env("NORMALIZATION_DATABASE_NAME", database)
        .env("NORMALIZATION_ACTOR_PRINCIPAL_ID", ACTOR);
    if let Some((key, value)) = extra_env {
        command.env(key, value);
    }
    command.output().expect("run operator")
}

fn last_json(output: &Output) -> Value {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .last()
        .unwrap_or_else(|| panic!("missing JSON: {}", String::from_utf8_lossy(&output.stdout)))
}

async fn disposable() -> (String, String, PgPool) {
    let name = format!("human_normalization_{}", Uuid::new_v4().simple());
    let mut admin = PgConnection::connect(&common::admin_database_url())
        .await
        .unwrap();
    admin
        .execute(format!("CREATE DATABASE {name}").as_str())
        .await
        .unwrap();
    let url = format!("{}/{}", common::test_database_base(), name);
    let pool = PgPool::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    seed(&pool).await;
    (name, url, pool)
}

async fn drop_db(name: &str, pool: PgPool) {
    pool.close().await;
    let mut admin = PgConnection::connect(&common::admin_database_url())
        .await
        .unwrap();
    admin
        .execute(format!("DROP DATABASE {name} WITH (FORCE)").as_str())
        .await
        .unwrap();
}

async fn seed(pool: &PgPool) {
    let plan = rows();
    let actor: Uuid = ACTOR.parse().unwrap();
    let human: Uuid = HUMAN.parse().unwrap();
    let old = plan[0].old_owner;
    let domain = Uuid::new_v4();
    let definition = Uuid::new_v4();
    let version = plan[0].definition_version;
    let node = plan[0].node;
    let terminal_node = Uuid::new_v4();
    for (id, kind, name) in [
        (actor, "AGENT", "Administrative Actor"),
        (human, "HUMAN", "Owner Human"),
        (old, "AGENT", "Legacy Efficiency Agent"),
    ] {
        sqlx::query("INSERT INTO principals(principal_id,principal_type,display_name,enabled) VALUES($1,$2::principal_type,$3,TRUE)")
            .bind(id).bind(kind).bind(name).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO domains(domain_id,domain_key,display_name) VALUES($1,$2,'Human normalization test')")
        .bind(domain).bind(format!("human-normalization-{}", Uuid::new_v4().simple())).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO workflow_definitions(workflow_definition_id,domain_id,definition_key,display_name) VALUES($1,$2,$3,'Human work')")
        .bind(definition).bind(domain).bind(&plan[0].definition_key).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO workflow_definition_versions(definition_version_id,workflow_definition_id,version_number,version_status,semantic_model_version) VALUES($1,$2,1,'DRAFT',1)")
        .bind(version).bind(definition).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO workflow_node_definitions(node_id,definition_version_id,node_key,display_name,order_index,node_type,assignee_ref_type,fixed_principal_id) VALUES($1,$2,$3,'Open',0,$4::node_type,'FIXED_PRINCIPAL',$5)")
        .bind(node).bind(version).bind(&plan[0].node_key).bind(&plan[0].node_type).bind(old).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO workflow_node_definitions(node_id,definition_version_id,node_key,display_name,order_index,node_type,assignee_ref_type) VALUES($1,$2,'completed','Completed',1,'TERMINAL',NULL)")
        .bind(terminal_node).bind(version).execute(pool).await.unwrap();
    for (index, row) in plan.iter().enumerate() {
        assert_eq!(
            (row.definition_version, row.node, row.human),
            (version, node, human)
        );
        sqlx::query("INSERT INTO workflow_instances(workflow_instance_id,domain_id,definition_version_id,created_by_principal_id,workflow_state_version,semantic_model_version,metadata) VALUES($1,$2,$3,$4,$5,1,jsonb_build_object('business','unchanged'))")
            .bind(row.workflow).bind(domain).bind(version).bind(actor).bind(row.version).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO workflow_context_revisions(context_revision_id,workflow_instance_id,revision_number,payload,payload_digest,created_by_principal_id) VALUES($1,$2,1,jsonb_build_object('stillPending',true),$3,$4)")
            .bind(row.context).bind(row.workflow).bind(&row.context_digest).bind(actor).execute(pool).await.unwrap();
        let source_transition = (index == 0).then(Uuid::new_v4);
        sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(row.source).bind(row.workflow).bind(node).bind(row.visit_number).bind(old).bind(source_transition).execute(pool).await.unwrap();
        sqlx::query("UPDATE workflow_instances SET current_context_revision_id=$1,current_node_visit_id=$2 WHERE workflow_instance_id=$3")
            .bind(row.context).bind(row.source).bind(row.workflow).execute(pool).await.unwrap();
    }
    let unrelated = Uuid::new_v4();
    let unrelated_context = Uuid::new_v4();
    let unrelated_visit = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_instances(workflow_instance_id,domain_id,definition_version_id,created_by_principal_id,workflow_state_version,semantic_model_version,metadata) VALUES($1,$2,$3,$4,1,1,jsonb_build_object('unrelated',true))")
        .bind(unrelated).bind(domain).bind(version).bind(actor).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO workflow_context_revisions(context_revision_id,workflow_instance_id,revision_number,payload,payload_digest,created_by_principal_id) VALUES($1,$2,1,'{}'::jsonb,$3,$4)")
        .bind(unrelated_context).bind(unrelated).bind("f".repeat(64)).bind(actor).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,1,$4,NULL)")
        .bind(unrelated_visit).bind(unrelated).bind(node).bind(old).execute(pool).await.unwrap();
    sqlx::query("UPDATE workflow_instances SET current_context_revision_id=$1,current_node_visit_id=$2 WHERE workflow_instance_id=$3")
        .bind(unrelated_context).bind(unrelated_visit).bind(unrelated).execute(pool).await.unwrap();

    for workflow in EXCLUDED_TERMINAL_WORKFLOWS {
        let workflow: Uuid = workflow.parse().unwrap();
        let context = Uuid::new_v4();
        let visit = Uuid::new_v4();
        sqlx::query("INSERT INTO workflow_instances(workflow_instance_id,domain_id,definition_version_id,created_by_principal_id,current_context_revision_id,current_node_visit_id,workflow_state_version,semantic_model_version,metadata) VALUES($1,$2,$3,$4,NULL,NULL,2,1,jsonb_build_object('excludedTerminal',true))")
            .bind(workflow).bind(domain).bind(version).bind(actor).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO workflow_context_revisions(context_revision_id,workflow_instance_id,revision_number,payload,payload_digest,created_by_principal_id) VALUES($1,$2,1,jsonb_build_object('completed',true),$3,$4)")
            .bind(context).bind(workflow).bind("e".repeat(64)).bind(actor).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,2,NULL,$4)")
            .bind(visit).bind(workflow).bind(terminal_node).bind(Uuid::new_v4()).execute(pool).await.unwrap();
        sqlx::query("UPDATE workflow_instances SET current_context_revision_id=$1,current_node_visit_id=$2 WHERE workflow_instance_id=$3")
            .bind(context).bind(visit).bind(workflow).execute(pool).await.unwrap();
    }
}

async fn excluded_terminal_snapshot(pool: &PgPool) -> Value {
    let ids: Vec<Uuid> = EXCLUDED_TERMINAL_WORKFLOWS
        .iter()
        .map(|id| id.parse().unwrap())
        .collect();
    sqlx::query_scalar(
        "SELECT jsonb_build_object(
            'instances', (SELECT jsonb_agg(to_jsonb(wi) ORDER BY workflow_instance_id) FROM workflow_instances wi WHERE workflow_instance_id=ANY($1)),
            'visits', (SELECT jsonb_agg(to_jsonb(v) ORDER BY node_visit_id) FROM workflow_node_visits v WHERE workflow_instance_id=ANY($1)),
            'contexts', (SELECT jsonb_agg(to_jsonb(c) ORDER BY context_revision_id) FROM workflow_context_revisions c WHERE workflow_instance_id=ANY($1)),
            'events', (SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY event_id),'[]'::jsonb) FROM workflow_events e WHERE workflow_instance_id=ANY($1)))",
    )
    .bind(ids)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn artifact_counts(pool: &PgPool) -> (i64, i64, i64, i64) {
    let targets: Vec<Uuid> = rows().iter().map(|row| row.target).collect();
    let visits =
        sqlx::query_scalar("SELECT count(*) FROM workflow_node_visits WHERE node_visit_id=ANY($1)")
            .bind(&targets)
            .fetch_one(pool)
            .await
            .unwrap();
    let receipts = sqlx::query_scalar("SELECT count(*) FROM workflow_command_receipts WHERE command_type='HUMAN_EXECUTOR_NORMALIZATION_V1'").fetch_one(pool).await.unwrap();
    let events = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_events WHERE event_type='HUMAN_EXECUTOR_NORMALIZED'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let audits = sqlx::query_scalar("SELECT count(*) FROM workflow_security_audits WHERE action='HUMAN_EXECUTOR_NORMALIZATION_V1_COMMITTED'").fetch_one(pool).await.unwrap();
    (visits, receipts, events, audits)
}

#[test]
fn cli_is_closed_to_exact_embedded_plan() {
    for args in [
        vec!["--plan-path", "/tmp/other.tsv"],
        vec![
            "--apply",
            "--workflow-id",
            "012e72de-4851-454e-b5f2-05b0db15707d",
        ],
        vec![
            "--apply",
            "--target-principal",
            "00000000-0000-0000-0000-000000000000",
        ],
    ] {
        let output = Command::new(binary()).args(args).output().unwrap();
        assert!(!output.status.success());
        assert_eq!(last_json(&output)["outcome"], "CONFLICT");
    }
}

#[tokio::test]
async fn exact_group_is_atomic_append_only_and_replay_safe() {
    let (name, url, pool) = disposable().await;
    let plan = rows();
    assert_eq!(plan.len(), 18);
    let excluded_before = excluded_terminal_snapshot(&pool).await;
    let human: Uuid = HUMAN.parse().unwrap();
    sqlx::query("UPDATE principals SET enabled=FALSE WHERE principal_id=$1")
        .bind(human)
        .execute(&pool)
        .await
        .unwrap();
    assert!(!run(&url, &name, &["--apply"]).status.success());
    assert_eq!(artifact_counts(&pool).await, (0, 0, 0, 0));
    sqlx::query("UPDATE principals SET enabled=TRUE,principal_type='AGENT' WHERE principal_id=$1")
        .bind(human)
        .execute(&pool)
        .await
        .unwrap();
    assert!(!run(&url, &name, &["--apply"]).status.success());
    assert_eq!(artifact_counts(&pool).await, (0, 0, 0, 0));
    sqlx::query("UPDATE principals SET principal_type='HUMAN' WHERE principal_id=$1")
        .bind(human)
        .execute(&pool)
        .await
        .unwrap();
    let ready = run(&url, &name, &["--plan"]);
    assert!(
        ready.status.success(),
        "{}",
        String::from_utf8_lossy(&ready.stdout)
    );
    assert_eq!(last_json(&ready)["outcome"], "READY");

    sqlx::query("UPDATE workflow_instances SET workflow_state_version=workflow_state_version+1 WHERE workflow_instance_id=$1").bind(plan[7].workflow).execute(&pool).await.unwrap();
    assert!(!run(&url, &name, &["--apply"]).status.success());
    assert_eq!(artifact_counts(&pool).await, (0, 0, 0, 0));
    sqlx::query("UPDATE workflow_instances SET workflow_state_version=workflow_state_version-1 WHERE workflow_instance_id=$1").bind(plan[7].workflow).execute(&pool).await.unwrap();

    sqlx::query(&format!("CREATE FUNCTION fail_human_normalization_test() RETURNS trigger AS $$ BEGIN IF NEW.workflow_instance_id='{}'::uuid THEN RAISE EXCEPTION 'injected test failure'; END IF; RETURN NEW; END; $$ LANGUAGE plpgsql", plan[8].workflow)).execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_human_normalization_test BEFORE INSERT ON workflow_events FOR EACH ROW EXECUTE FUNCTION fail_human_normalization_test()")
        .execute(&pool).await.unwrap();
    let injected = run(&url, &name, &["--apply"]);
    assert!(!injected.status.success());
    assert_eq!(artifact_counts(&pool).await, (0, 0, 0, 0));
    sqlx::query("DROP TRIGGER fail_human_normalization_test ON workflow_events")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION fail_human_normalization_test()")
        .execute(&pool)
        .await
        .unwrap();

    let source_before: Value =
        sqlx::query("SELECT to_jsonb(v) AS row FROM workflow_node_visits v WHERE node_visit_id=$1")
            .bind(plan[0].source)
            .fetch_one(&pool)
            .await
            .unwrap()
            .get("row");
    assert!(!source_before["entered_by_transition_id"].is_null());
    let business_sql = "SELECT jsonb_build_object('domain',domain_id,'definition',definition_version_id,'creator',created_by_principal_id,'context',current_context_revision_id,'metadata',metadata,'cancelled',cancelled,'archived',archived_at,'semantic',semantic_model_version,'executionClass',execution_class) AS row FROM workflow_instances WHERE workflow_instance_id=$1";
    let business_before: Value = sqlx::query(business_sql)
        .bind(plan[0].workflow)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("row");
    let unrelated_before: Value = sqlx::query("SELECT to_jsonb(wi) AS row FROM workflow_instances wi WHERE metadata @> '{\"unrelated\":true}'::jsonb")
        .fetch_one(&pool).await.unwrap().get("row");
    let applied = run(&url, &name, &["--apply"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stdout)
    );
    assert_eq!(last_json(&applied)["outcome"], "APPLIED");
    assert_eq!(artifact_counts(&pool).await, (18, 18, 18, 1));
    let ids: Vec<Uuid> = plan.iter().map(|row| row.workflow).collect();
    let active_human: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_instances wi JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id JOIN principals p ON p.principal_id=v.assignee_principal_id WHERE wi.workflow_instance_id=ANY($1) AND NOT wi.cancelled AND wi.archived_at IS NULL AND p.principal_type='HUMAN'").bind(&ids).fetch_one(&pool).await.unwrap();
    let active_agent: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_instances wi JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id JOIN principals p ON p.principal_id=v.assignee_principal_id WHERE wi.workflow_instance_id=ANY($1) AND NOT wi.cancelled AND wi.archived_at IS NULL AND p.principal_type='AGENT'").bind(&ids).fetch_one(&pool).await.unwrap();
    assert_eq!((active_human, active_agent), (18, 0));
    let source_after: Value =
        sqlx::query("SELECT to_jsonb(v) AS row FROM workflow_node_visits v WHERE node_visit_id=$1")
            .bind(plan[0].source)
            .fetch_one(&pool)
            .await
            .unwrap()
            .get("row");
    let business_after: Value = sqlx::query(business_sql)
        .bind(plan[0].workflow)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("row");
    let unrelated_after: Value = sqlx::query("SELECT to_jsonb(wi) AS row FROM workflow_instances wi WHERE metadata @> '{\"unrelated\":true}'::jsonb")
        .fetch_one(&pool).await.unwrap().get("row");
    assert_eq!(source_before, source_after);
    assert_eq!(business_before, business_after);
    assert_eq!(unrelated_before, unrelated_after);
    assert_eq!(excluded_before, excluded_terminal_snapshot(&pool).await);
    let submissions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_submissions WHERE workflow_instance_id=ANY($1)",
    )
    .bind(&ids)
    .fetch_one(&pool)
    .await
    .unwrap();
    let activations: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_activations WHERE workflow_instance_id=ANY($1)",
    )
    .bind(&ids)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((submissions, activations), (0, 0));

    let replay = run(&url, &name, &["--apply"]);
    assert!(replay.status.success());
    assert_eq!(last_json(&replay)["outcome"], "NOOP");
    assert_eq!(artifact_counts(&pool).await, (18, 18, 18, 1));
    assert_eq!(
        last_json(&run(&url, &name, &["--verify"]))["outcome"],
        "VERIFIED"
    );
    sqlx::query("UPDATE workflow_instances SET current_node_visit_id=$1,workflow_state_version=$2 WHERE workflow_instance_id=$3").bind(plan[0].source).bind(plan[0].version).bind(plan[0].workflow).execute(&pool).await.unwrap();
    assert!(!run(&url, &name, &["--apply"]).status.success());
    assert_eq!(artifact_counts(&pool).await, (18, 18, 18, 1));
    drop_db(&name, pool).await;
}

#[tokio::test]
async fn committed_but_acknowledgement_lost_is_reconciled_without_retry() {
    let (name, url, pool) = disposable().await;
    let applied = run_with_env(
        &url,
        &name,
        &["--apply"],
        Some(("NORMALIZATION_TEST_SIMULATE_COMMIT_ACK_LOSS", "1")),
    );
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stdout)
    );
    assert_eq!(last_json(&applied)["outcome"], "APPLIED");
    assert_eq!(last_json(&applied)["writes"], 0);
    assert_eq!(artifact_counts(&pool).await, (18, 18, 18, 1));
    assert_eq!(
        last_json(&run(&url, &name, &["--verify"]))["outcome"],
        "VERIFIED"
    );
    drop_db(&name, pool).await;
}

#[tokio::test]
async fn collision_and_open_assistance_abort_before_operator_writes() {
    let plan = rows();
    let (name, url, pool) = disposable().await;
    sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,$4,$5,NULL)")
        .bind(plan[3].target).bind(plan[3].workflow).bind(plan[3].node).bind(plan[3].visit_number+1).bind(plan[3].old_owner).execute(&pool).await.unwrap();
    assert!(!run(&url, &name, &["--apply"]).status.success());
    assert_eq!(artifact_counts(&pool).await, (1, 0, 0, 0));
    drop_db(&name, pool).await;

    let (name, url, pool) = disposable().await;
    let group = format!(
        "SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V1:bba710b9790fed4c0136b9a0f33186f87f11be9e5da3f76e08784bfbce8dd871:{}:{name}:{}:{HUMAN}",
        env!("GIT_SHA"),
        ACTOR
    );
    let key = format!(
        "human-normalization-v1:{}:{}",
        &hex::encode(sha2::Sha256::digest(group.as_bytes()))[..24],
        plan[2].workflow
    );
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status) VALUES($1,$2,$3,'CONFLICTING_COMMAND',$4,'PROCESSING')")
        .bind(Uuid::new_v4()).bind(ACTOR.parse::<Uuid>().unwrap()).bind(key).bind("0".repeat(64)).execute(&pool).await.unwrap();
    assert!(!run(&url, &name, &["--apply"]).status.success());
    assert_eq!(artifact_counts(&pool).await, (0, 0, 0, 0));
    let conflicting_receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_command_receipts WHERE command_type='CONFLICTING_COMMAND'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(conflicting_receipts, 1);
    drop_db(&name, pool).await;

    let (name, url, pool) = disposable().await;
    let command = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status,response_status,response_body,response_digest,completed_at) VALUES($1,$2,$3,'REQUEST_WORKFLOW_ASSISTANCE',$4,'COMPLETED',200,'{}'::jsonb,$4,now())")
        .bind(command).bind(ACTOR.parse::<Uuid>().unwrap()).bind(format!("assist-{}",Uuid::new_v4())).bind("0".repeat(64)).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO workflow_assistance_cases(assistance_case_id,workflow_instance_id,node_visit_id,status,requested_by_principal_id,request_payload,request_payload_digest,request_command_id) VALUES($1,$2,$3,'OWNER_PENDING',$4,'{}'::jsonb,$5,$6)")
        .bind(Uuid::new_v4()).bind(plan[5].workflow).bind(plan[5].source).bind(ACTOR.parse::<Uuid>().unwrap()).bind("0".repeat(64)).bind(command).execute(&pool).await.unwrap();
    assert!(!run(&url, &name, &["--apply"]).status.success());
    assert_eq!(artifact_counts(&pool).await, (0, 0, 0, 0));
    drop_db(&name, pool).await;
}
