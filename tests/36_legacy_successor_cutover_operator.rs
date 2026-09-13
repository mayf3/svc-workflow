//! Executable blocker-union closure for the bounded BIP x4 CIR operator.
//! Run with scripts/run_legacy_successor_cutover_v1_tests.sh.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::{Command, Output},
    sync::{Arc, Mutex},
};
use uuid::Uuid;

const PLAN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/evidence/legacy-workflow-successor-cutover-v1/plan.json"
);
const PLAN_SHA: &str = "dd994cbc1fed7cb8c523eb1cc31e5cd5ca6a27a71a9e7bf3c50337dad7ea6f17";
const ACTOR: &str = "271876c1-5bc9-4902-855d-68bf5c99eccb";
const CODE: &str = "legacy_workflow_successor_cutover_v1/group-atomic-repair";

fn db_url() -> String {
    std::env::var("CUTOVER_TEST_DATABASE_URL")
        .expect("CUTOVER_TEST_DATABASE_URL required; use checked-in runner")
}
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_legacy_workflow_successor_cutover_v1")
}
fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn temp(tag: &str, bytes: &[u8]) -> PathBuf {
    let p = std::env::temp_dir().join(format!("cutover-36-{tag}-{}.json", Uuid::new_v4()));
    std::fs::write(&p, bytes).unwrap();
    p
}

struct Directory {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
}
impl Directory {
    fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = requests.clone();
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let log = log.clone();
                std::thread::spawn(move || {
                    let mut buf = Vec::new();
                    let mut chunk = [0u8; 2048];
                    let mut end = None;
                    while end.is_none() {
                        let n = stream.read(&mut chunk).unwrap_or(0);
                        if n == 0 {
                            return;
                        }
                        buf.extend_from_slice(&chunk[..n]);
                        end = buf.windows(4).position(|w| w == b"\r\n\r\n").map(|x| x + 4)
                    }
                    let end = end.unwrap();
                    let head = String::from_utf8_lossy(&buf[..end]).to_string();
                    let len = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|x| x.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    while buf.len() < end + len {
                        let n = stream.read(&mut chunk).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        buf.extend_from_slice(&chunk[..n]);
                    }
                    let path = head
                        .lines()
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .to_string();
                    log.lock().unwrap().push(path.clone());
                    let body = String::from_utf8_lossy(&buf[end..]).to_string();
                    let response = if path == "/oauth/token" {
                        let scope = body
                            .split('&')
                            .find_map(|p| {
                                p.split_once('=')
                                    .filter(|(k, _)| *k == "scope")
                                    .map(|(_, v)| v)
                            })
                            .unwrap_or("");
                        format!(
                            r#"{{"access_token":"tok","token_type":"Bearer","expires_in":3600,"scope":"{scope}"}}"#
                        )
                    } else if let Some(id) = path
                        .strip_prefix("/api/v1/directory/principals/")
                        .and_then(|x| x.strip_suffix("/agent"))
                    {
                        format!(
                            r#"{{"principalId":"{id}","agentId":"agent-{id}","principalStatus":"active"}}"#
                        )
                    } else if let Some(id) = path.strip_prefix("/v1/directory/agents/") {
                        format!(r#"{{"agentId":"{id}","exists":true,"enabled":true}}"#)
                    } else {
                        r#"{"error":"bad_path"}"#.into()
                    };
                    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response).ok();
                });
            }
        });
        Self { url, requests }
    }
}

fn run(dir: &Directory, args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .env("DATABASE_URL", db_url())
        .env("WORKFLOW_ADMISSION_ENABLED", "1")
        .env("WORKFLOW_ADMISSION_AUTH_BASE_URL", &dir.url)
        .env("WORKFLOW_ADMISSION_CORE_BASE_URL", &dir.url)
        .env("WORKFLOW_ADMISSION_CLIENT_ID", "svc-workflow")
        .env("WORKFLOW_ADMISSION_CLIENT_SECRET", "test-only")
        .output()
        .unwrap()
}
fn apply(dir: &Directory, plan: &str, digest: &str, extra: &[&str]) -> Output {
    let receipt = temp("receipt", b"");
    let mut a = vec![
        "apply",
        "--plan",
        plan,
        "--plan-sha256",
        digest,
        "--actor-principal",
        ACTOR,
        "--operator-code-coordinate",
        CODE,
        "--receipt-out",
        receipt.to_str().unwrap(),
    ];
    a.extend_from_slice(extra);
    run(dir, &a)
}
fn assert_rejected(o: &Output) {
    assert!(
        !o.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&o.stdout)
    );
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("CONFLICT_ZERO_WRITE"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
}

async fn reset(pool: &PgPool) {
    sqlx::query("DROP SCHEMA public CASCADE")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("CREATE SCHEMA public")
        .execute(pool)
        .await
        .unwrap();
    sqlx::migrate!().run(pool).await.unwrap();
}
async fn seed(pool: &PgPool) -> Value {
    let p: Value = serde_json::from_slice(&std::fs::read(PLAN).unwrap()).unwrap();
    let principals = [
        ("d2bec623-56ba-46cb-a217-933cfe63a243", "legacy-publisher"),
        (
            "8b44befd-934e-48cd-afac-31ed981fb732",
            "canonical-publisher",
        ),
        ("aed2c679-71c0-4e23-8d73-c448c5ea6dd2", "legacy-operator"),
        (ACTOR, "canonical-operator"),
        ("bb9d8f48-7962-4321-8fb1-554bb428c159", "legacy-creator"),
        ("d5b3aeb2-e754-49a9-9914-b963521c0985", "canonical-creator"),
    ];
    for (id, name) in principals {
        sqlx::query("INSERT INTO principals(principal_id,principal_type,display_name,enabled) VALUES($1,'AGENT',$2,true)").bind(Uuid::parse_str(id).unwrap()).bind(name).execute(pool).await.unwrap();
    }
    let domain = Uuid::parse_str("f9b5682c-1c9c-5ace-a7b1-01b9a3be5965").unwrap();
    sqlx::query("INSERT INTO domains(domain_id,domain_key,display_name,enabled) VALUES($1,'bip','BIP',true)").bind(domain).execute(pool).await.unwrap();
    let def = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
    let src = Uuid::parse_str(p["sourceDefinitionVersionId"].as_str().unwrap()).unwrap();
    sqlx::query("INSERT INTO workflow_definitions(workflow_definition_id,domain_id,definition_key,display_name) VALUES($1,$2,'bip','BIP')").bind(def).bind(domain).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO workflow_definition_versions(definition_version_id,workflow_definition_id,version_number,version_status,definition_digest,semantic_model_version) VALUES($1,$2,3,'DRAFT',$3,1)").bind(src).bind(def).bind("0".repeat(64)).execute(pool).await.unwrap();
    let nodes = [
        (
            "22222222-2222-4222-8222-222222222221",
            "dispatch",
            Some("aed2c679-71c0-4e23-8d73-c448c5ea6dd2"),
            0,
            "NORMAL",
        ),
        (
            "22222222-2222-4222-8222-222222222222",
            "publishing",
            Some("d2bec623-56ba-46cb-a217-933cfe63a243"),
            1,
            "NORMAL",
        ),
        (
            "22222222-2222-4222-8222-222222222223",
            "verify",
            Some("aed2c679-71c0-4e23-8d73-c448c5ea6dd2"),
            2,
            "NORMAL",
        ),
        (
            "22222222-2222-4222-8222-222222222224",
            "done",
            None,
            3,
            "TERMINAL",
        ),
    ];
    for (id, key, principal, order, kind) in nodes {
        sqlx::query("INSERT INTO workflow_node_definitions(node_id,definition_version_id,node_key,display_name,order_index,node_type,assignee_ref_type,fixed_principal_id) VALUES($1,$2,$3,$3,$4,$5::node_type,CASE WHEN $6::uuid IS NULL THEN NULL ELSE 'FIXED_PRINCIPAL'::assignee_ref_type END,$6)").bind(Uuid::parse_str(id).unwrap()).bind(src).bind(key).bind(order).bind(kind).bind(principal.map(|x|Uuid::parse_str(x).unwrap())).execute(pool).await.unwrap();
    }
    let transitions = [
        (
            "33333333-3333-4333-8333-333333333331",
            "advance-to-publishing",
            0,
            1,
            "ADVANCE",
        ),
        (
            "33333333-3333-4333-8333-333333333332",
            "advance-to-verify",
            1,
            2,
            "ADVANCE",
        ),
        (
            "33333333-3333-4333-8333-333333333333",
            "verified",
            2,
            3,
            "TERMINATE",
        ),
        (
            "33333333-3333-4333-8333-333333333334",
            "verify-failed",
            2,
            1,
            "RETURN",
        ),
    ];
    for (id, key, from, to, effect) in transitions {
        sqlx::query("INSERT INTO workflow_transition_definitions(transition_id,definition_version_id,transition_key,display_name,source_node_id,target_node_id,transition_effect) VALUES($1,$2,$3,$3,$4,$5,$6::transition_effect)").bind(Uuid::parse_str(id).unwrap()).bind(src).bind(key).bind(Uuid::parse_str(nodes[from].0).unwrap()).bind(Uuid::parse_str(nodes[to].0).unwrap()).bind(effect).execute(pool).await.unwrap();
    }
    for (node, transition) in [
        (nodes[0].0, Some(transitions[0].0)),
        (nodes[1].0, Some(transitions[1].0)),
        (nodes[2].0, Some(transitions[2].0)),
        (nodes[3].0, None),
    ] {
        sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2").bind(transition.map(|x|Uuid::parse_str(x).unwrap())).bind(Uuid::parse_str(node).unwrap()).execute(pool).await.unwrap();
    }
    sqlx::query("UPDATE workflow_definition_versions SET version_status='PUBLISHED' WHERE definition_version_id=$1").bind(src).execute(pool).await.unwrap();
    for (old, new) in [
        (
            "d2bec623-56ba-46cb-a217-933cfe63a243",
            "8b44befd-934e-48cd-afac-31ed981fb732",
        ),
        ("aed2c679-71c0-4e23-8d73-c448c5ea6dd2", ACTOR),
        (
            "bb9d8f48-7962-4321-8fb1-554bb428c159",
            "d5b3aeb2-e754-49a9-9914-b963521c0985",
        ),
    ] {
        sqlx::query("INSERT INTO workflow_identity_successor_lines(source_principal_id,successor_principal_id,legacy_agent_id,canonical_agent_id,classification,evidence,repair_reason) VALUES($1,$2,'old','agt_new','MECHANICALLY_PROVEN_SUCCESSOR','{}','fixture')").bind(Uuid::parse_str(old).unwrap()).bind(Uuid::parse_str(new).unwrap()).execute(pool).await.unwrap();
    }
    for row in p["rows"].as_array().unwrap() {
        let iid = Uuid::parse_str(row["sourceInstanceId"].as_str().unwrap()).unwrap();
        let ctx = Uuid::parse_str(row["sourceExpectedContextRevision"].as_str().unwrap()).unwrap();
        let current = Uuid::parse_str(row["sourceExpectedCurrentVisit"].as_str().unwrap()).unwrap();
        let state = row["sourceExpectedStateVersion"].as_i64().unwrap() as i32;
        let visits = row["sourceExpectedVisitCount"].as_i64().unwrap() as i32;
        sqlx::query("INSERT INTO workflow_instances(workflow_instance_id,domain_id,definition_version_id,created_by_principal_id,workflow_state_version,external_url,metadata,external_reference,semantic_model_version) VALUES($1,$2,$3,$4,$5,$6,$7,$8,1)").bind(iid).bind(domain).bind(src).bind(Uuid::parse_str("bb9d8f48-7962-4321-8fb1-554bb428c159").unwrap()).bind(state).bind(format!("https://example/{iid}")).bind(json!({"fixture":true})).bind(iid.to_string()).execute(pool).await.unwrap();
        let payload = row["sourceExpectedContext"].clone();
        let pd = jcs_canonicalize::sha256_jcs_hex(&payload).unwrap();
        sqlx::query("INSERT INTO workflow_context_revisions(context_revision_id,workflow_instance_id,revision_number,payload,payload_digest,created_by_principal_id) VALUES($1,$2,1,$3,$4,$5)").bind(ctx).bind(iid).bind(payload).bind(pd).bind(Uuid::parse_str(LEGACY_OPERATOR).unwrap()).execute(pool).await.unwrap();
        for n in 1..=visits {
            let vid = if n == visits { current } else { Uuid::new_v4() };
            sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id) VALUES($1,$2,$3,$4,$5)").bind(vid).bind(iid).bind(Uuid::parse_str(nodes[1].0).unwrap()).bind(n).bind(Uuid::parse_str("8b44befd-934e-48cd-afac-31ed981fb732").unwrap()).execute(pool).await.unwrap();
        }
        for n in 1..=state {
            sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,event_type,actor_principal_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,$3,'v1','FIXTURE',$4,$5,$3)").bind(Uuid::new_v4()).bind(iid).bind(n).bind(Uuid::parse_str(LEGACY_OPERATOR).unwrap()).bind(n-1).execute(pool).await.unwrap();
        }
        sqlx::query("UPDATE workflow_instances SET current_context_revision_id=$1,current_node_visit_id=$2 WHERE workflow_instance_id=$3").bind(ctx).bind(current).bind(iid).execute(pool).await.unwrap();
    }
    p
}
const LEGACY_OPERATOR: &str = "aed2c679-71c0-4e23-8d73-c448c5ea6dd2";

async fn business_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM workflow_instance_successor_cutovers")
        .fetch_one(pool)
        .await
        .unwrap()
}
async fn history_snapshot(pool: &PgPool, p: &Value) -> Value {
    let ids = p["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| Uuid::parse_str(r["sourceInstanceId"].as_str().unwrap()).unwrap())
        .collect::<Vec<_>>();
    sqlx::query("SELECT jsonb_build_object('visits',(SELECT jsonb_agg(to_jsonb(v) ORDER BY node_visit_id) FROM workflow_node_visits v WHERE workflow_instance_id=ANY($1)),'events',(SELECT jsonb_agg(to_jsonb(e) ORDER BY event_id) FROM workflow_events e WHERE workflow_instance_id=ANY($1)),'contexts',(SELECT jsonb_agg(to_jsonb(c) ORDER BY context_revision_id) FROM workflow_context_revisions c WHERE workflow_instance_id=ANY($1))) AS snapshot").bind(ids).fetch_one(pool).await.unwrap().get("snapshot")
}

#[tokio::test]
async fn blocker_union_group_atomic_fail_closed_and_replay_matrix() {
    let pool = PgPool::connect(&db_url()).await.unwrap();
    reset(&pool).await;
    let p = seed(&pool).await;
    let dir = Directory::spawn();
    let plan_out = run(&dir, &["plan", "--plan", PLAN]);
    assert!(
        plan_out.status.success(),
        "{}",
        String::from_utf8_lossy(&plan_out.stderr)
    );
    let row0 = &p["rows"][0];
    let source = Uuid::parse_str(row0["sourceInstanceId"].as_str().unwrap()).unwrap();
    let expected = row0["sourceExpectedStateVersion"].as_i64().unwrap() as i32;
    sqlx::query(
        "UPDATE workflow_instances SET workflow_state_version=$1 WHERE workflow_instance_id=$2",
    )
    .bind(expected + 1)
    .bind(source)
    .execute(&pool)
    .await
    .unwrap();
    assert_rejected(&apply(&dir, PLAN, PLAN_SHA, &[]));
    assert_eq!(business_count(&pool).await, 0);
    sqlx::query(
        "UPDATE workflow_instances SET workflow_state_version=$1 WHERE workflow_instance_id=$2",
    )
    .bind(expected)
    .bind(source)
    .execute(&pool)
    .await
    .unwrap();
    let alt:Uuid=sqlx::query_scalar("SELECT node_visit_id FROM workflow_node_visits WHERE workflow_instance_id=$1 AND node_visit_id<>$2 LIMIT 1").bind(source).bind(Uuid::parse_str(row0["sourceExpectedCurrentVisit"].as_str().unwrap()).unwrap()).fetch_one(&pool).await.unwrap();
    sqlx::query(
        "UPDATE workflow_instances SET current_node_visit_id=$1 WHERE workflow_instance_id=$2",
    )
    .bind(alt)
    .bind(source)
    .execute(&pool)
    .await
    .unwrap();
    assert_rejected(&apply(&dir, PLAN, PLAN_SHA, &[]));
    assert_eq!(business_count(&pool).await, 0);
    sqlx::query(
        "UPDATE workflow_instances SET current_node_visit_id=$1 WHERE workflow_instance_id=$2",
    )
    .bind(Uuid::parse_str(row0["sourceExpectedCurrentVisit"].as_str().unwrap()).unwrap())
    .bind(source)
    .execute(&pool)
    .await
    .unwrap();
    for (tag, path, value) in [
        ("model", vec!["successorModel"], json!(2)),
        (
            "definition",
            vec!["sourceDefinitionVersionId"],
            json!(Uuid::new_v4()),
        ),
        (
            "context",
            vec!["publisherPrincipal"],
            json!("d2bec623-56ba-46cb-a217-933cfe63a243"),
        ),
    ] {
        let mut bad = p.clone();
        bad[path[0]] = value;
        let bytes = serde_json::to_vec_pretty(&bad).unwrap();
        let file = temp(tag, &bytes);
        assert_rejected(&apply(&dir, file.to_str().unwrap(), &sha(&bytes), &[]));
        assert_eq!(business_count(&pool).await, 0);
    }
    let assistance_command = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status) VALUES($1,$2,$3,'REQUEST_WORKFLOW_ASSISTANCE',$4,'COMPLETED')")
        .bind(assistance_command).bind(Uuid::parse_str("8b44befd-934e-48cd-afac-31ed981fb732").unwrap()).bind(format!("fixture-{assistance_command}")).bind("0".repeat(64)).execute(&pool).await.unwrap();
    let assistance_payload = json!({});
    sqlx::query("INSERT INTO workflow_assistance_cases(assistance_case_id,workflow_instance_id,node_visit_id,status,requested_by_principal_id,request_payload,request_payload_digest,request_command_id) VALUES($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)")
        .bind(Uuid::new_v4()).bind(source).bind(Uuid::parse_str(row0["sourceExpectedCurrentVisit"].as_str().unwrap()).unwrap()).bind(Uuid::parse_str("8b44befd-934e-48cd-afac-31ed981fb732").unwrap()).bind(&assistance_payload).bind(jcs_canonicalize::sha256_jcs_hex(&assistance_payload).unwrap()).bind(assistance_command).execute(&pool).await.unwrap();
    assert_rejected(&apply(&dir, PLAN, PLAN_SHA, &[]));
    assert_eq!(business_count(&pool).await, 0);
    reset(&pool).await;
    seed(&pool).await;
    let collision = Uuid::parse_str(row0["successorInstanceId"].as_str().unwrap()).unwrap();
    let inserted=sqlx::query("INSERT INTO workflow_instances(workflow_instance_id,domain_id,definition_version_id,created_by_principal_id,workflow_state_version,semantic_model_version) SELECT $1,domain_id,definition_version_id,created_by_principal_id,1,1 FROM workflow_instances WHERE workflow_instance_id=$2").bind(collision).bind(source).execute(&pool).await.unwrap();
    assert_eq!(inserted.rows_affected(), 1);
    assert_rejected(&apply(&dir, PLAN, PLAN_SHA, &[]));
    assert_eq!(business_count(&pool).await, 0);
    // Reset the disposable fixture after the deliberate collision.
    reset(&pool).await;
    let p = seed(&pool).await;
    let before = history_snapshot(&pool, &p).await;
    assert_rejected(&apply(
        &dir,
        PLAN,
        PLAN_SHA,
        &["--inject-failure-after-create"],
    ));
    assert_eq!(business_count(&pool).await, 0);
    let leaked:Vec<Uuid>=sqlx::query_scalar(
            "SELECT workflow_instance_id FROM workflow_instances WHERE workflow_instance_id=ANY($1) ORDER BY workflow_instance_id"
        )
        .bind(
            p["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| Uuid::parse_str(r["successorInstanceId"].as_str().unwrap()).unwrap())
                .collect::<Vec<_>>()
        )
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(
        leaked.is_empty(),
        "rollback leaked successor(s): {leaked:?}"
    );
    assert_eq!(history_snapshot(&pool, &p).await, before);
    let receipt = temp("final", b"");
    let ok = run(
        &dir,
        &[
            "apply",
            "--plan",
            PLAN,
            "--plan-sha256",
            PLAN_SHA,
            "--actor-principal",
            ACTOR,
            "--operator-code-coordinate",
            CODE,
            "--receipt-out",
            receipt.to_str().unwrap(),
        ],
    );
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    assert_eq!(business_count(&pool).await, 4);
    let after = history_snapshot(&pool, &p).await;
    for row in p["rows"].as_array().unwrap() {
        let source = Uuid::parse_str(row["sourceInstanceId"].as_str().unwrap()).unwrap();
        let old = row["sourceExpectedEventCount"].as_i64().unwrap();
        let existing_before = before["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["workflow_instance_id"] == json!(source))
            .collect::<Vec<_>>();
        let existing_after = after["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["workflow_instance_id"] == json!(source) && e["event_type"] == "FIXTURE")
            .collect::<Vec<_>>();
        assert_eq!(existing_before, existing_after);
        assert_eq!(existing_after.len() as i64, old);
    }
    assert_eq!(before["visits"], after["visits"]);
    assert_eq!(before["contexts"], after["contexts"]);
    let replay = apply(&dir, PLAN, PLAN_SHA, &[]);
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    assert_eq!(business_count(&pool).await, 4);
    let successor = Uuid::parse_str(row0["successorInstanceId"].as_str().unwrap()).unwrap();
    sqlx::query(
        "UPDATE workflow_instances SET workflow_state_version=2 WHERE workflow_instance_id=$1",
    )
    .bind(successor)
    .execute(&pool)
    .await
    .unwrap();
    assert_rejected(&apply(&dir, PLAN, PLAN_SHA, &[]));
    assert_eq!(business_count(&pool).await, 4);
    sqlx::query(
        "UPDATE workflow_instances SET workflow_state_version=1 WHERE workflow_instance_id=$1",
    )
    .bind(successor)
    .execute(&pool)
    .await
    .unwrap();
    let verify = run(
        &dir,
        &[
            "verify",
            "--plan",
            PLAN,
            "--plan-sha256",
            PLAN_SHA,
            "--actor-principal",
            ACTOR,
            "--operator-code-coordinate",
            CODE,
            "--receipt",
            receipt.to_str().unwrap(),
        ],
    );
    assert!(
        verify.status.success(),
        "{}",
        String::from_utf8_lossy(&verify.stderr)
    );
    assert!(
        dir.requests.lock().unwrap().len() >= 12,
        "fresh directory checks executed for failed-injection and successful commands"
    );
}
