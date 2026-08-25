//! Focused conformance for the exact-plan-bound trusted fleet cutover.
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::process::{Command, Output};

const PLAN: &str =
    "/Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json";
const PLAN_SHA: &str = "0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606";

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_trusted_fleet_principal_cutover_v1")
}

fn run(args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .env_remove("DATABASE_URL")
        .env_remove("AUTH_DATABASE_URL")
        .env_remove("MIGRATION_ACTOR_PRINCIPAL_ID")
        .output()
        .expect("run cutover binary")
}

#[test]
fn frozen_plan_digest_and_counts_are_exact() {
    let raw = std::fs::read(PLAN).expect("read frozen plan");
    assert_eq!(raw.len(), 540_472);
    assert_eq!(hex::encode(Sha256::digest(&raw)), PLAN_SHA);
    let value: serde_json::Value = serde_json::from_slice(&raw).expect("parse plan");
    assert_eq!(value["schema"], "workflow_trusted_fleet_successor_plan_v2");
    assert_eq!(value["fleet_rows"].as_array().unwrap().len(), 86);
    assert_eq!(value["domain_tuples"].as_array().unwrap().len(), 760);
    assert_eq!(
        value["current_responsibility_tuples"]
            .as_array()
            .unwrap()
            .len(),
        80
    );
    assert_eq!(
        value["creator_owned_draft_tuples"]
            .as_array()
            .unwrap()
            .len(),
        99
    );
    assert_eq!(
        value["creator_owned_draft_tuples"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["migration_candidate"] == true)
            .count(),
        0
    );
    assert_eq!(
        value["domain_tuples"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["role"] == "DOMAIN_OWNER")
            .count(),
        8
    );
    assert_eq!(
        value["domain_tuples"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["role"] == "DOMAIN_MEMBER")
            .count(),
        752
    );
}

#[test]
fn cli_exposes_only_closed_modes_and_scopes() {
    for args in [
        vec![],
        vec!["--old", "00000000-0000-0000-0000-000000000000"],
        vec!["--new", "00000000-0000-0000-0000-000000000000"],
        vec!["--agent-id", "agt_efficiency-agent"],
        vec!["--plan", PLAN],
        vec!["--apply", "--scope", "arbitrary"],
        vec!["--apply", "--scope", "remaining-fleet", "--plan"],
    ] {
        let output = run(&args);
        assert!(
            !output.status.success(),
            "forbidden args succeeded: {args:?}"
        );
        let body: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON error");
        assert_eq!(body["writes"], 0);
        assert_eq!(body["outcome"], "CONFLICT");
    }
    for args in [
        vec!["--plan"],
        vec!["--verify"],
        vec!["--apply", "--scope", "build-in-public-canary"],
        vec!["--apply", "--scope", "efficiency-canary"],
        vec!["--apply", "--scope", "remaining-fleet"],
    ] {
        let output = run(&args);
        assert!(!output.status.success());
        let body: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON error");
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("DATABASE_URL is required"));
    }
}

#[tokio::test]
async fn empty_disposable_databases_fail_loud_with_zero_workflow_writes() {
    let workflow_url = std::env::var("TEST_WORKFLOW_DATABASE_URL").expect("runner workflow DB");
    let auth_url = std::env::var("TEST_AUTH_DATABASE_URL").expect("runner auth DB");
    let auth = PgPool::connect(&auth_url).await.unwrap();
    sqlx::raw_sql(
        "CREATE TYPE principal_type AS ENUM ('agent','service');
         CREATE TYPE principal_status AS ENUM ('active','disabled');
         CREATE TABLE machine_principals(
           id uuid PRIMARY KEY,
           principal_type principal_type NOT NULL,
           agent_id text UNIQUE,
           external_ref text UNIQUE,
           status principal_status NOT NULL,
           display_name text
         );",
    )
    .execute(&auth)
    .await
    .unwrap();

    let output = Command::new(binary())
        .arg("--plan")
        .env("DATABASE_URL", &workflow_url)
        .env("AUTH_DATABASE_URL", &auth_url)
        .output()
        .expect("run cutover plan");
    assert!(!output.status.success());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON error");
    assert_eq!(body["writes"], 0);
    assert_eq!(body["outcome"], "CONFLICT");
    assert!(
        body["error"].as_str().unwrap().contains("Auth pair missing")
            || body["error"].as_str().unwrap().contains("is missing"),
        "unexpected plan failure: {body}"
    );

    let workflow = PgPool::connect(&workflow_url).await.unwrap();
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_security_audits")
        .fetch_one(&workflow)
        .await
        .unwrap();
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_command_receipts")
        .fetch_one(&workflow)
        .await
        .unwrap();
    assert_eq!((audits, receipts), (0, 0));

    let plan: serde_json::Value = serde_json::from_slice(&std::fs::read(PLAN).unwrap()).unwrap();
    let pair = plan["fleet_rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["new_agent_id"] == "agt_build-in-public-agent")
        .unwrap();
    let old: uuid::Uuid = pair["old_principal_id"].as_str().unwrap().parse().unwrap();
    let new: uuid::Uuid = pair["new_principal_id"].as_str().unwrap().parse().unwrap();
    let old_agent = pair["old_agent_id"].as_str().unwrap();
    let new_agent = pair["new_agent_id"].as_str().unwrap();
    let old_external = pair["old_principal_external_ref"].as_str().unwrap();
    let new_external = pair["new_principal_external_ref"].as_str().unwrap();
    let actor: uuid::Uuid = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".parse().unwrap();
    let display = "Build in Public — Formal Auth Name";
    sqlx::query("INSERT INTO machine_principals(id,principal_type,agent_id,external_ref,status,display_name) VALUES($1,'agent',$2,$3,'active','Legacy Build'),($4,'agent',$5,$6,'active',$7)")
        .bind(old).bind(old_agent).bind(old_external).bind(new).bind(new_agent).bind(new_external).bind(display)
        .execute(&auth).await.unwrap();
    sqlx::query("INSERT INTO principals(principal_id,principal_type,display_name,enabled) VALUES($1,'HUMAN','Migration Actor',TRUE),($2,'AGENT','Legacy Build',TRUE),($3,'AGENT',$4,TRUE)")
        .bind(actor).bind(old).bind(new).bind(display)
        .execute(&workflow).await.unwrap();
    for tuple in plan["domain_tuples"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["old_principal_id"].as_str() == Some(pair["old_principal_id"].as_str().unwrap())
        })
    {
        let domain: uuid::Uuid = tuple["domain_id"].as_str().unwrap().parse().unwrap();
        let key = tuple["domain_key"].as_str().unwrap();
        let role = tuple["role"].as_str().unwrap();
        sqlx::query(
            "INSERT INTO domains(domain_id,domain_key,display_name,enabled) VALUES($1,$2,$2,TRUE)",
        )
        .bind(domain)
        .bind(key)
        .execute(&workflow)
        .await
        .unwrap();
        sqlx::query("INSERT INTO domain_role_bindings(binding_id,domain_id,principal_id,role_key,enabled) VALUES($1,$2,$3,$4,TRUE)")
            .bind(uuid::Uuid::new_v4()).bind(domain).bind(old).bind(role)
            .execute(&workflow).await.unwrap();
    }
    let apply = Command::new(binary())
        .args(["--apply", "--scope", "build-in-public-canary"])
        .env("DATABASE_URL", &workflow_url)
        .env("AUTH_DATABASE_URL", &auth_url)
        .env("MIGRATION_ACTOR_PRINCIPAL_ID", actor.to_string())
        .output()
        .unwrap();
    assert!(
        apply.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&apply.stdout),
        String::from_utf8_lossy(&apply.stderr)
    );
    let applied: serde_json::Value = serde_json::from_slice(&apply.stdout).unwrap();
    assert_eq!(applied["outcome"], "COMMITTED");
    let old_domains: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM domain_role_bindings WHERE principal_id=$1 AND enabled",
    )
    .bind(old)
    .fetch_one(&workflow)
    .await
    .unwrap();
    let new_domains: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM domain_role_bindings WHERE principal_id=$1 AND enabled",
    )
    .bind(new)
    .fetch_one(&workflow)
    .await
    .unwrap();
    assert_eq!((old_domains, new_domains), (0, 9));
    let audit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_security_audits")
        .fetch_one(&workflow)
        .await
        .unwrap();
    assert_eq!(audit_count, 1);

    let rerun = Command::new(binary())
        .args(["--apply", "--scope", "build-in-public-canary"])
        .env("DATABASE_URL", &workflow_url)
        .env("AUTH_DATABASE_URL", &auth_url)
        .env("MIGRATION_ACTOR_PRINCIPAL_ID", actor.to_string())
        .output()
        .unwrap();
    assert!(
        rerun.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&rerun.stdout),
        String::from_utf8_lossy(&rerun.stderr)
    );
    let rerun_body: serde_json::Value = serde_json::from_slice(&rerun.stdout).unwrap();
    assert_eq!(rerun_body["outcome"], "NOOP");
    assert_eq!(rerun_body["writes"], 0);
    assert_eq!(rerun_body["newAudits"], 0);
    let audit_count_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM workflow_security_audits")
            .fetch_one(&workflow)
            .await
            .unwrap();
    assert_eq!(audit_count_after, 1);
}
