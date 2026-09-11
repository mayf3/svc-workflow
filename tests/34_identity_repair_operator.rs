//! Focused conformance for the bounded offline identity repair operator
//! (P9 historical identity repair, migration 0025 +
//! `src/bin/identity_repair_v1.rs`).
//!
//! Governing authority:
//! `docs/specs/SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2.md`
//! CTR-CIR-001/004/006/007 (read-projection lineage surface). Test structure
//! follows `tests/27_trusted_fleet_principal_cutover_v1.rs` (binary-driven,
//! closed CLI, JSON outcomes).
//!
//! Coverage:
//!   - happy path: plan -> apply -> verify with real instance/visit rows;
//!   - HR-facing read-path enrichment: the projected current visit carries
//!     the canonical agent id while `assignee_principal_id` stays untouched;
//!   - plan hash gate (stale/wrong digest -> zero writes);
//!   - ambiguity abort (same successor claimed with a different canonical
//!     agent id), and the legitimate many-to-one case (same canonical id);
//!   - immutable check: visit/instance rows byte-identical before/after
//!     apply (including timestamps), event count unchanged, and the lineage
//!     table itself refuses UPDATE (append-only trigger);
//!   - idempotent re-apply -> NOOP; re-verify -> VERIFIED.
//!
//! Requires a PostgreSQL test database: set TEST_DATABASE_URL (the
//! svc-workflow-test-pg container on 127.0.0.1:55432 is the standard
//! runner); DATABASE_URL for the operator binary is derived from it.

#![allow(dead_code, unused_imports)]
#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use std::process::{Command, Output};
use uuid::Uuid;

use svc_workflow::application::workflow_instance::create::create_workflow_instance;
use svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition;
use svc_workflow::application::workflow_instance::query_service::WorkflowQueryService;
use svc_workflow::application::workflow_instance::query_types::{
    GetWorkflowInstanceDetail, LifecycleFilter, ListAssignedToMe, ListDomainInstances, StatusFilter,
};
use svc_workflow::domain::ids::{
    DefinitionVersionId, DomainId, PrincipalId, TransitionId, WorkflowInstanceId,
};
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
};
use svc_workflow::store::postgres::admission_gate::AdmissionGate;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_identity_repair_v1")
}

fn run(args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .env("DATABASE_URL", common::test_database_url())
        .env_remove("AUTH_DATABASE_URL")
        .output()
        .expect("run identity_repair_v1 binary")
}

/// Parse every JSON line of stdout (the binary prints one JSON object per
/// outcome; a CONFLICT plan prints its plan document first, then the error
/// line, mirroring the accepted cutover operator).
fn json_lines(output: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn last_json(output: &Output) -> Value {
    json_lines(output).last().cloned().unwrap_or_else(|| {
        panic!(
            "no JSON on stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn temp_file(tag: &str, content: &[u8]) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "identity-repair-34-{}-{}.json",
        tag,
        Uuid::new_v4()
    ));
    std::fs::write(&path, content).expect("write temp pairs file");
    path
}

fn pairs_file(tag: &str, pairs: &[Value]) -> std::path::PathBuf {
    temp_file(tag, serde_json::to_vec_pretty(pairs).unwrap().as_slice())
}

fn pair_json(source: Uuid, successor: Uuid, legacy: &str, canonical: &str) -> Value {
    json!({
        "sourcePrincipalId": source,
        "successorPrincipalId": successor,
        "legacyAgentId": legacy,
        "canonicalAgentId": canonical,
        "classification": "STALE_PRINCIPAL_WITH_UNIQUE_REPAIR",
        "evidence": {
            "authTwin": format!("{legacy} dual-directory verified 2026-09-07"),
            "source": "tests/34_identity_repair_operator"
        },
        "repairReason": "SOURCE_ONLY_STOP_NEW_V1-era naked-name principal; unique canonical twin mechanically proven"
    })
}

/// Seed: stale naked-name AGENT principal + canonical AGENT successor.
async fn seed_agent_principal(pool: &PgPool, agent_id: &str, display: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled, metadata)
         VALUES ($1, 'AGENT', $2, NULL, TRUE, jsonb_build_object('agent_id', $3))",
    )
    .bind(id)
    .bind(display)
    .bind(agent_id)
    .execute(pool)
    .await
    .expect("seed agent principal");
    id
}

async fn row_json(pool: &PgPool, sql: &str, id: Uuid) -> Value {
    sqlx::query(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("row to_jsonb readback")
        .get::<serde_json::Value, _>("row")
}

async fn scalar_i64(pool: &PgPool, sql: &str, id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("scalar readback")
}

struct Fixture {
    caller: Uuid,
    domain_id: Uuid,
    instance_id: Uuid,
    work_visit_id: Uuid,
    source: Uuid,
    successor: Uuid,
    draft_advance: Uuid,
}

/// Build a real published definition with a FIXED_PRINCIPAL work node owned
/// by the STALE source principal, create an instance and advance it onto the
/// work node so a live visit row exists (fixture pattern of test 33).
async fn seed_instance_on_source(pool: &PgPool) -> Fixture {
    let (caller, domain_id) = common::seed_principal_domain_with_owner(pool).await;
    let source = seed_agent_principal(
        pool,
        "writing-style-analyst-agent",
        "Stale Writing Style Analyst",
    )
    .await;
    let successor = seed_agent_principal(
        pool,
        "agt_writing-style-analyst-agent",
        "Canonical Writing Style Analyst",
    )
    .await;

    let def_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) \
         VALUES ($1, $2, $3, 'Identity Repair Def')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(format!("identity-repair-{}", &Uuid::new_v4().to_string()[..8]))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow_definition_versions \
         (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) \
         VALUES ($1, $2, 1, 'DRAFT', NULL)",
    )
    .bind(version_id)
    .bind(def_id)
    .execute(pool)
    .await
    .unwrap();

    let draft = Uuid::new_v4();
    let work = Uuid::new_v4();
    let done = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) \
         VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')",
    )
    .bind(draft)
    .bind(version_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) \
         VALUES ($1, $2, 'work', 'Work', 1, 'NORMAL', 'FIXED_PRINCIPAL', $3)",
    )
    .bind(work)
    .bind(version_id)
    .bind(source)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) \
         VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)",
    )
    .bind(done)
    .bind(version_id)
    .execute(pool)
    .await
    .unwrap();

    let draft_advance = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions \
         (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) \
         VALUES ($1, $2, 'advance-work', 'Advance', $3, $4, 'ADVANCE')",
    )
    .bind(draft_advance)
    .bind(version_id)
    .bind(draft)
    .bind(work)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2",
    )
    .bind(draft_advance)
    .bind(draft)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' \
         WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .execute(pool)
    .await
    .unwrap();

    let created = create_workflow_instance(
        pool,
        AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(caller),
            idempotency_key: format!("identity-repair-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
        execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(version_id),
            external_reference: None,
            external_url: None,
            metadata: json!({}),
            context_payload: json!({}),
        },
    )
    .await
    .expect("create instance on PUBLISHED version");

    let advanced = execute_workflow_transition(
        pool,
        AdmissionGate::disabled(),
        ExecuteWorkflowTransitionCommand {
            principal_id: PrincipalId::from_uuid(caller),
            idempotency_key: format!("identity-repair-adv-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(created.workflow_instance_id),
            expected_workflow_state_version: 1,
            transition_definition_id: TransitionId::from_uuid(draft_advance),
            submission_payload: None,
        },
    )
    .await
    .expect("advance draft -> work");

    let work_visit_id: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(advanced.workflow_instance_id)
    .fetch_one(pool)
    .await
    .unwrap();

    Fixture {
        caller,
        domain_id,
        instance_id: advanced.workflow_instance_id,
        work_visit_id,
        source,
        successor,
        draft_advance,
    }
}

/// Recompute UNRELATED_DIGEST exactly as the operator defines it (md5 over
/// all existing lineage rows, ordered by source), so assertions stay valid
/// on a shared test database that already holds lineage rows.
async fn unrelated_digest_from_db(pool: &PgPool) -> String {
    sqlx::query_scalar(
        "SELECT md5(COALESCE(string_agg(row_text, E'\\n' ORDER BY source_principal_id), ''))
         FROM (
           SELECT source_principal_id,
                  source_principal_id::text || '|' || successor_principal_id::text || '|'
                  || legacy_agent_id || '|' || canonical_agent_id || '|' || classification || '|'
                  || evidence::text || '|' || repair_reason AS row_text
           FROM workflow_identity_successor_lines
         ) t",
    )
    .fetch_one(pool)
    .await
    .expect("unrelated digest readback")
}

/// One sequential conformance run (tests share the lineage table, and the
/// plan hash covers the whole table state, so the binary-driven phases must
/// not interleave).
#[tokio::test]
async fn identity_repair_operator_end_to_end_conformance() {
    let pool = common::create_pool().await;
    let fx = seed_instance_on_source(&pool).await;
    let legacy = "writing-style-analyst-agent";
    let canonical = "agt_writing-style-analyst-agent";

    // -- Closed CLI: forbidden argument shapes never succeed -------------
    for args in [
        vec![],
        vec!["plan"],
        vec!["apply", "--pairs", "/dev/null"],
        vec!["verify", "--pairs", "/dev/null"],
        vec!["--old", "00000000-0000-0000-0000-000000000000"],
    ] {
        let output = run(&args);
        assert!(
            !output.status.success(),
            "forbidden args succeeded: {args:?}"
        );
        let body = last_json(&output);
        assert_eq!(body["writes"], 0);
        assert_eq!(body["outcome"], "CONFLICT");
    }

    // -- PLAN (read-only) ------------------------------------------------
    let pairs_path = pairs_file(
        "happy",
        &[pair_json(fx.source, fx.successor, legacy, canonical)],
    );
    let pairs_str = pairs_path.to_string_lossy().into_owned();
    let plan_output = run(&["plan", "--pairs", &pairs_str]);
    assert!(
        plan_output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&plan_output.stdout),
        String::from_utf8_lossy(&plan_output.stderr)
    );
    let plan_body = last_json(&plan_output);
    assert_eq!(plan_body["outcome"], "PLANNED");
    assert_eq!(plan_body["writes"], 0);
    let plan_sha = plan_body["planHash"]
        .as_str()
        .expect("planHash")
        .to_string();
    assert_eq!(plan_sha.len(), 64);

    let plan = &plan_body["plan"];
    assert_eq!(plan["schema"], "workflow_identity_repair_plan_v1");
    assert_eq!(plan["mode"], "READ_ONLY_CANONICAL_PLAN");
    assert_eq!(plan["summary"]["pairCount"], 1);
    assert_eq!(plan["summary"]["conflictCount"], 0);
    assert_eq!(plan["summary"]["ambiguityCount"], 0);
    assert_eq!(plan["summary"]["lineageAlreadyExistsCount"], 0);
    // One visit (the current work visit) is affected and actionable.
    assert_eq!(plan["summary"]["affectedAssignmentCount"], 1);
    assert_eq!(plan["summary"]["affectedActionableCount"], 1);
    assert_eq!(plan["pairs"][0]["lineageAlreadyExists"], false);
    let unrelated_before = plan["unrelatedDigest"]
        .as_str()
        .expect("unrelatedDigest")
        .to_string();
    assert_eq!(unrelated_before, unrelated_digest_from_db(&pool).await);
    let pairs_file_sha = plan["pairsFileSha256"].as_str().unwrap().to_string();

    // -- APPLY with a WRONG plan hash: fail closed, zero writes ----------
    let wrong_sha = "0".repeat(64);
    let receipt_wrong = temp_file("receipt-wrong", b"");
    let wrong_apply = run(&[
        "apply",
        "--pairs",
        &pairs_str,
        "--plan-sha256",
        &wrong_sha,
        "--receipt-out",
        &receipt_wrong.to_string_lossy(),
    ]);
    assert!(!wrong_apply.status.success(), "wrong plan hash must abort");
    let body = last_json(&wrong_apply);
    assert_eq!(body["outcome"], "CONFLICT");
    assert_eq!(body["writes"], 0);
    assert_eq!(
        scalar_i64(
            &pool,
            "SELECT count(*) FROM workflow_node_visits WHERE assignee_principal_id = $1",
            fx.source,
        )
        .await,
        1
    );

    // -- VERIFY before apply must fail (nothing applied yet) -------------
    let verify_pre = run(&["verify", "--pairs", &pairs_str, "--plan-sha256", &plan_sha]);
    assert!(
        !verify_pre.status.success(),
        "verify before apply must fail"
    );
    assert_eq!(last_json(&verify_pre)["outcome"], "CONFLICT");

    // -- Byte-identical preimage snapshot --------------------------------
    let visit_before = row_json(
        &pool,
        "SELECT to_jsonb(v) AS row FROM workflow_node_visits v WHERE node_visit_id = $1",
        fx.work_visit_id,
    )
    .await;
    let instance_before = row_json(
        &pool,
        "SELECT to_jsonb(wi) AS row FROM workflow_instances wi WHERE workflow_instance_id = $1",
        fx.instance_id,
    )
    .await;
    let events_before: i64 = scalar_i64(
        &pool,
        "SELECT count(*) FROM workflow_events WHERE workflow_instance_id = $1",
        fx.instance_id,
    )
    .await;

    // -- APPLY (committed) ------------------------------------------------
    let receipt_out = temp_file("receipt", b"");
    let apply_output = run(&[
        "apply",
        "--pairs",
        &pairs_str,
        "--plan-sha256",
        &plan_sha,
        "--receipt-out",
        &receipt_out.to_string_lossy(),
    ]);
    assert!(
        apply_output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&apply_output.stdout),
        String::from_utf8_lossy(&apply_output.stderr)
    );
    let applied = last_json(&apply_output);
    assert_eq!(applied["outcome"], "COMMITTED");
    assert_eq!(applied["writes"], 1);
    assert_eq!(applied["planSha256"], plan_sha.as_str());

    // Receipt schema (CTR-CIR-007 family receipt, as a JSON file).
    let receipt: Value =
        serde_json::from_slice(&std::fs::read(&receipt_out).expect("read receipt")).unwrap();
    assert_eq!(receipt["schema"], "workflow_identity_repair_receipt_v1");
    assert_eq!(receipt["commandFamily"], "WORKFLOW_IDENTITY_REPAIR_V1");
    assert_eq!(receipt["outcome"], "COMMITTED");
    assert_eq!(receipt["planSha256"], plan_sha.as_str());
    assert_eq!(receipt["pairsFileSha256"], pairs_file_sha.as_str());
    assert_eq!(receipt["immutability"]["visitRowsMutated"], 0);
    assert_eq!(receipt["immutability"]["eventRowsMutated"], 0);
    assert_eq!(receipt["immutability"]["historyRewriteCount"], 0);
    assert_eq!(receipt["pairs"][0]["lineageRowIdentical"], true);

    // -- IMMUTABILITY: history is byte-identical, including timestamps ----
    let visit_after = row_json(
        &pool,
        "SELECT to_jsonb(v) AS row FROM workflow_node_visits v WHERE node_visit_id = $1",
        fx.work_visit_id,
    )
    .await;
    let instance_after = row_json(
        &pool,
        "SELECT to_jsonb(wi) AS row FROM workflow_instances wi WHERE workflow_instance_id = $1",
        fx.instance_id,
    )
    .await;
    assert_eq!(
        visit_before, visit_after,
        "visit row must be byte-identical"
    );
    assert_eq!(
        instance_before, instance_after,
        "instance row must be byte-identical"
    );
    let events_after: i64 = scalar_i64(
        &pool,
        "SELECT count(*) FROM workflow_events WHERE workflow_instance_id = $1",
        fx.instance_id,
    )
    .await;
    assert_eq!(events_before, events_after);
    let events_as_repaired: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_events WHERE actor_principal_id = ANY($1)",
    )
    .bind(&[fx.source, fx.successor][..])
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        events_as_repaired, 0,
        "the repair operator must not write events as any repaired principal"
    );

    // The single durable delta: exactly one lineage row.
    let lineage: (Uuid, Uuid, String, String, String) = sqlx::query_as(
        "SELECT source_principal_id, successor_principal_id, legacy_agent_id, canonical_agent_id, classification \
         FROM workflow_identity_successor_lines WHERE source_principal_id = $1",
    )
    .bind(fx.source)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(lineage.0, fx.source);
    assert_eq!(lineage.1, fx.successor);
    assert_eq!(lineage.2, legacy);
    assert_eq!(lineage.3, canonical);
    assert_eq!(lineage.4, "STALE_PRINCIPAL_WITH_UNIQUE_REPAIR");

    // The lineage surface itself is append-only.
    let update_blocked = sqlx::query(
        "UPDATE workflow_identity_successor_lines SET canonical_agent_id = 'agt_evil' \
         WHERE source_principal_id = $1",
    )
    .bind(fx.source)
    .execute(&pool)
    .await;
    assert!(update_blocked.is_err(), "lineage UPDATE must be rejected");
    let delete_blocked =
        sqlx::query("DELETE FROM workflow_identity_successor_lines WHERE source_principal_id = $1")
            .bind(fx.source)
            .execute(&pool)
            .await;
    assert!(delete_blocked.is_err(), "lineage DELETE must be rejected");

    // -- HR-facing read-path enrichment -----------------------------------
    let query_service = WorkflowQueryService::new(pool.clone());
    let detail = query_service
        .get_workflow_instance_detail(GetWorkflowInstanceDetail {
            actor_principal_id: fx.caller,
            workflow_instance_id: fx.instance_id,
        })
        .await
        .expect("instance detail");
    let full = match detail {
        svc_workflow::application::workflow_instance::query_types::WorkflowInstanceDetail::Full(
            full,
        ) => full,
        other => panic!("expected full visibility, got {other:?}"),
    };
    // The assignee UUID is NEVER rewritten...
    assert_eq!(full.current_visit.assignee_principal_id, Some(fx.source));
    // ...but the projected identity now carries the canonical agent id.
    assert_eq!(
        full.current_visit.assignee_canonical_agent_id.as_deref(),
        Some(canonical)
    );

    let domain_list = query_service
        .list_domain_instances(ListDomainInstances {
            actor_principal_id: fx.caller,
            domain_id: fx.domain_id,
            before: None,
            limit: Some(10),
            definition_key: None,
            lifecycle: Some(LifecycleFilter::Active),
            current_node_key: None,
            assignee_principal_id: None,
            status: StatusFilter::All,
        })
        .await
        .expect("domain instance list");
    assert_eq!(domain_list.items.len(), 1);
    assert_eq!(
        domain_list.items[0].current_assignee_principal_id,
        Some(fx.source)
    );
    assert_eq!(
        domain_list.items[0]
            .current_assignee_canonical_agent_id
            .as_deref(),
        Some(canonical)
    );

    let worklist = query_service
        .list_assigned_to_me(ListAssignedToMe {
            actor_principal_id: fx.source,
            before: None,
            limit: Some(10),
        })
        .await
        .expect("assigned worklist for the stale principal");
    assert_eq!(worklist.items.len(), 1);
    assert_eq!(
        worklist.items[0]
            .detail
            .current_visit
            .assignee_canonical_agent_id
            .as_deref(),
        Some(canonical)
    );

    // Store helper follows exactly one edge and yields the successor.
    let resolved = svc_workflow::store::postgres::identity_successor::resolve_current_principal(
        &pool, fx.source,
    )
    .await
    .expect("resolve");
    assert_eq!(resolved, Some(fx.successor));
    let unresolved_identity =
        svc_workflow::store::postgres::identity_successor::resolve_current_principal(
            &pool, fx.caller,
        )
        .await
        .expect("resolve self");
    assert_eq!(
        unresolved_identity,
        Some(fx.caller),
        "no lineage row -> source itself"
    );

    // -- VERIFY ------------------------------------------------------------
    let verify_output = run(&["verify", "--pairs", &pairs_str, "--plan-sha256", &plan_sha]);
    assert!(
        verify_output.status.success(),
        "stdout={}",
        String::from_utf8_lossy(&verify_output.stdout)
    );
    let verified = last_json(&verify_output);
    assert_eq!(verified["outcome"], "VERIFIED");
    assert_eq!(verified["writes"], 0);
    assert_eq!(verified["pairs"][0]["projectionProofVisits"], 1);

    // -- Idempotent re-apply: NOOP with zero writes -----------------------
    let receipt_re = temp_file("receipt-reapply", b"");
    let reapply = run(&[
        "apply",
        "--pairs",
        &pairs_str,
        "--plan-sha256",
        &plan_sha,
        "--receipt-out",
        &receipt_re.to_string_lossy(),
    ]);
    assert!(
        reapply.status.success(),
        "stdout={}",
        String::from_utf8_lossy(&reapply.stdout)
    );
    let noop = last_json(&reapply);
    assert_eq!(noop["outcome"], "NOOP");
    assert_eq!(noop["writes"], 0);
    let receipt_re: Value =
        serde_json::from_slice(&std::fs::read(&receipt_re).expect("read reapply receipt")).unwrap();
    assert_eq!(receipt_re["outcome"], "NOOP");
    assert_eq!(receipt_re["skippedCount"], 1);

    // Re-verify still passes; a re-plan marks the pair LINEAGE_ALREADY_EXISTS.
    let verify_re = run(&["verify", "--pairs", &pairs_str, "--plan-sha256", &plan_sha]);
    assert!(verify_re.status.success());
    assert_eq!(last_json(&verify_re)["outcome"], "VERIFIED");
    let replan = run(&["plan", "--pairs", &pairs_str]);
    assert!(
        replan.status.success(),
        "identical existing lineage row must keep the plan emittable"
    );
    let replan_body = last_json(&replan);
    assert_eq!(
        replan_body["plan"]["pairs"][0]["lineageAlreadyExists"],
        true
    );
    assert_eq!(
        replan_body["plan"]["summary"]["lineageAlreadyExistsCount"],
        1
    );

    // Lineage digest changed because the repair row exists now.
    let unrelated_after = unrelated_digest_from_db(&pool).await;
    assert_ne!(unrelated_before, unrelated_after);

    let _ = pairs_path;

    // ------------------------------------------------------------------
    // PHASE: AMBIGUITY abort — the same successor claimed with a DIFFERENT
    // canonical agent id (against an existing lineage row) aborts
    // plan/apply with zero writes, while the SAME canonical id for another
    // stale source is the legitimate many-to-one case and applies cleanly.
    // ------------------------------------------------------------------
    let phase_ambiguity = || async {
        let pool = common::create_pool().await;
        let successor =
            seed_agent_principal(&pool, "agt_shared-canonical-agent", "Shared Canonical").await;
        let source_one = seed_agent_principal(&pool, "stale-one-agent", "Stale One").await;
        let source_two = seed_agent_principal(&pool, "stale-two-agent", "Stale Two").await;

        // Pair one applies cleanly.
        let pairs_one = pairs_file(
            "amb-one",
            &[pair_json(
                source_one,
                successor,
                "stale-one-agent",
                "agt_shared-canonical-agent",
            )],
        );
        let one_str = pairs_one.to_string_lossy().into_owned();
        let plan_one = run(&["plan", "--pairs", &one_str]);
        assert!(plan_one.status.success());
        let sha_one = last_json(&plan_one)["planHash"]
            .as_str()
            .unwrap()
            .to_string();
        let receipt_one = temp_file("receipt-amb-one", b"");
        let apply_one = run(&[
            "apply",
            "--pairs",
            &one_str,
            "--plan-sha256",
            &sha_one,
            "--receipt-out",
            &receipt_one.to_string_lossy(),
        ]);
        assert!(apply_one.status.success());
        assert_eq!(last_json(&apply_one)["outcome"], "COMMITTED");

        // Pair two claims the SAME successor with a DIFFERENT canonical id:
        // cross-source ambiguity -> plan aborts, nothing written.
        let pairs_conflict = pairs_file(
            "amb-conflict",
            &[pair_json(
                source_two,
                successor,
                "stale-two-agent",
                "agt_someone-else",
            )],
        );
        let conflict_str = pairs_conflict.to_string_lossy().into_owned();
        let plan_conflict = run(&["plan", "--pairs", &conflict_str]);
        assert!(!plan_conflict.status.success(), "ambiguous plan must fail");
        let conflict_plan_doc = json_lines(&plan_conflict);
        assert!(
            conflict_plan_doc
                .iter()
                .any(|line| line["plan"]["summary"]["ambiguityCount"] == 1),
            "plan document must report ambiguityCount=1: {conflict_plan_doc:?}"
        );
        let rows_for_two: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_identity_successor_lines WHERE source_principal_id = $1",
        )
        .bind(source_two)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(rows_for_two, 0, "ambiguous pair must not be written");

        // Pair two with the SAME canonical id is a legitimate many-to-one
        // repair (two stale sources, one canonical successor).
        let pairs_ok = pairs_file(
            "amb-ok",
            &[pair_json(
                source_two,
                successor,
                "stale-two-agent",
                "agt_shared-canonical-agent",
            )],
        );
        let ok_str = pairs_ok.to_string_lossy().into_owned();
        let plan_ok = run(&["plan", "--pairs", &ok_str]);
        assert!(
            plan_ok.status.success(),
            "many-to-one with same canonical id must plan"
        );
        let sha_ok = last_json(&plan_ok)["planHash"]
            .as_str()
            .unwrap()
            .to_string();
        let receipt_ok = temp_file("receipt-amb-ok", b"");
        let apply_ok = run(&[
            "apply",
            "--pairs",
            &ok_str,
            "--plan-sha256",
            &sha_ok,
            "--receipt-out",
            &receipt_ok.to_string_lossy(),
        ]);
        assert!(apply_ok.status.success());
        assert_eq!(last_json(&apply_ok)["outcome"], "COMMITTED");
        let verify_ok = run(&["verify", "--pairs", &ok_str, "--plan-sha256", &sha_ok]);
        assert!(verify_ok.status.success());
        assert_eq!(last_json(&verify_ok)["outcome"], "VERIFIED");
    };

    phase_ambiguity().await;

    // ------------------------------------------------------------------
    // PHASE: fail-closed plan conditions — missing source, disabled
    // successor, non-AGENT successor and a bad canonical grammar all abort
    // the plan with explicit conflicts and zero writes.
    // ------------------------------------------------------------------
    let phase_fail_closed = || async {
        let pool = common::create_pool().await;
        let successor =
            seed_agent_principal(&pool, "agt_gate-successor-agent", "Gate Successor").await;
        let source = seed_agent_principal(&pool, "stale-gate-agent", "Stale Gate").await;
        let ghost = Uuid::new_v4();

        // Missing source principal.
        let missing = pairs_file(
            "gate-missing",
            &[pair_json(ghost, successor, "ghost-agent", "agt_ghost")],
        );
        let out = run(&["plan", "--pairs", &missing.to_string_lossy().as_ref()]);
        assert!(!out.status.success());

        // Disabled successor.
        sqlx::query("UPDATE principals SET enabled = FALSE WHERE principal_id = $1")
            .bind(successor)
            .execute(&pool)
            .await
            .unwrap();
        let disabled = pairs_file(
            "gate-disabled",
            &[pair_json(
                source,
                successor,
                "stale-gate-agent",
                "agt_gate-successor-agent",
            )],
        );
        let out = run(&["plan", "--pairs", &disabled.to_string_lossy().as_ref()]);
        assert!(!out.status.success());
        let doc = json_lines(&out);
        assert!(doc.iter().any(|line| line["plan"]["pairs"][0]["conflicts"]
            .as_array()
            .is_some_and(|c| c
                .iter()
                .any(|x| x.as_str().unwrap_or("").contains("not enabled")))));

        // Non-AGENT successor.
        let human = {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO principals (principal_id, principal_type, display_name, enabled) VALUES ($1, 'HUMAN', 'Human Gate', TRUE)")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
            id
        };
        let not_agent = pairs_file(
            "gate-not-agent",
            &[pair_json(
                source,
                human,
                "stale-gate-agent",
                "agt_gate-successor-agent",
            )],
        );
        let out = run(&["plan", "--pairs", &not_agent.to_string_lossy().as_ref()]);
        assert!(!out.status.success());

        // Bad canonical grammar (pure validation, caught before the database).
        let bad_grammar = pairs_file(
            "gate-grammar",
            &[pair_json(
                source,
                successor,
                "stale-gate-agent",
                "gate-successor-agent",
            )],
        );
        let out = run(&["plan", "--pairs", &bad_grammar.to_string_lossy().as_ref()]);
        assert!(!out.status.success());
        assert_eq!(last_json(&out)["outcome"], "CONFLICT");
    };

    phase_fail_closed().await;
}

/// Admission (CTR-CIR-003) must evaluate the CANONICAL identity: the gate's
/// principal mapping — the exact surface `AdmissionGate::admit` delegates to
/// before the pinned directory reads — resolves a principal carrying a
/// lineage edge to its successor, passes a lineage-less principal through
/// unchanged, and is the identity mapping when no pool is wired (dormant
/// deploy / unit tests).
#[tokio::test]
async fn admission_resolves_principals_through_identity_lineage() {
    use std::collections::BTreeSet;

    let pool = common::create_pool().await;
    let source =
        seed_agent_principal(&pool, "admission-lineage-probe-agent", "Admission Lineage Probe")
            .await;
    let successor = seed_agent_principal(
        &pool,
        "agt_admission-lineage-probe-agent",
        "Canonical Admission Lineage Probe",
    )
    .await;
    let stranger =
        seed_agent_principal(&pool, "admission-lineage-stranger-agent", "Admission Stranger")
            .await;

    // Same row shape the offline operator writes (append-only table).
    sqlx::query(
        "INSERT INTO workflow_identity_successor_lines \
         (source_principal_id, successor_principal_id, legacy_agent_id, canonical_agent_id, \
          classification, evidence, repair_reason) \
         VALUES ($1, $2, 'admission-lineage-probe-agent', 'agt_admission-lineage-probe-agent', \
          'STALE_PRINCIPAL_WITH_UNIQUE_REPAIR', '{\"probe\":true}'::jsonb, \
          'admission canonicalization probe')",
    )
    .bind(source)
    .bind(successor)
    .execute(&pool)
    .await
    .expect("insert lineage row");

    // Pool wired: the lineage edge maps source -> successor; the stranger has
    // no edge and passes through unchanged.
    let gate = AdmissionGate::disabled().with_pool(Some(&pool));
    let mapped = gate
        .canonicalize_principals([source, successor, stranger])
        .await
        .expect("canonicalize");
    assert_eq!(
        mapped,
        BTreeSet::from([successor, stranger]),
        "lineage edge -> successor; no edge -> unchanged"
    );

    // No pool wired: identity mapping, nothing dropped.
    let gate = AdmissionGate::disabled();
    let mapped = gate
        .canonicalize_principals([source, stranger])
        .await
        .expect("identity mapping");
    assert_eq!(mapped, BTreeSet::from([source, stranger]));

    // No cleanup: the surface is append-only (UPDATE/DELETE rejected by
    // trigger); the row stays as operator-written evidence, like the
    // conformance run above. Fresh principals per run keep UNIQUE(source)
    // satisfied.
}
