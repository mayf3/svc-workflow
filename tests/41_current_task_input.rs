//! Real TCP HTTP + isolated PostgreSQL checks for accepted CTI V1.
mod common;
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use svc_workflow::application::workflow_instance::create::{
    create_workflow_instance, CreateWorkflowInstanceResult,
};
use svc_workflow::application::workflow_instance::execute_transition::{
    execute_workflow_transition, execute_workflow_transition_with_policy,
};
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::*;
use svc_workflow::store::postgres::admission_gate::AdmissionGate;
use uuid::Uuid;
#[path = "17_workflow_runtime/query/helpers.rs"]
#[allow(dead_code)]
mod fixtures;
use fixtures::*;
#[path = "current_task_input/support.rs"]
mod support;
use support::*;

#[tokio::test]
async fn initial_input_is_exact_existing_full_detail_without_business_writes() {
    let h = Harness::new(json!({"title":"current original"})).await;
    let before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1")
            .bind(h.instance)
            .fetch_one(&h.pool)
            .await
            .unwrap();
    let input = h.input().await;
    assert_eq!(
        input["binding"]["workflow_instance_id"],
        h.instance.to_string()
    );
    assert_eq!(input["entry"]["status"], "INITIAL");
    assert_eq!(input["entry"]["submission_status"], "NONE_RECORDED");
    assert!(input["submissions"].as_array().unwrap().is_empty());
    assert_eq!(input["history_window_complete"], true);
    let bytes = h.bytes(&input, &input["detail_body"]).await;
    let detail: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        detail["current_context"]["payload"],
        json!({"title":"current original"})
    );
    let old = query_service(&h.pool)
        .get_workflow_instance_detail(
            svc_workflow::application::workflow_instance::query_types::GetWorkflowInstanceDetail {
                actor_principal_id: h.seed.owner,
                workflow_instance_id: h.instance,
            },
        )
        .await
        .unwrap();
    assert_eq!(detail, serde_json::to_value(old).unwrap()["detail"]);
    let after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1")
            .bind(h.instance)
            .fetch_one(&h.pool)
            .await
            .unwrap();
    assert_eq!(before, after);
}

#[tokio::test]
async fn restricted_and_unrelated_actors_never_receive_full_content() {
    let h = Harness::new(json!({"private":"context"})).await;
    let (status, _, _) = h.get(h.seed.creator, "").await;
    assert_eq!(status, 200);
    let (status, body, _) = h.get(h.seed.outsider, "").await;
    assert_eq!(status, 404);
    assert_eq!(
        body["error"]["code"],
        "workflow_instance_not_found_or_not_visible"
    );
    h.step(
        h.seed.creator,
        1,
        h.seed.draft_advance,
        Some(json!({"work":"submitted"})),
    )
    .await;
    let (status, body, _) = h.get(h.seed.creator, "").await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], "restricted_history_not_visible");
    assert_eq!(h.get(h.seed.assignee, "").await.0, 200);
    set_principal_enabled(&h.pool, h.seed.assignee, false).await;
    let (status, body, _) = h.get(h.seed.assignee, "").await;
    assert_eq!(status, 403);
    assert_eq!(body["error"]["code"], "principal_disabled");
}

#[tokio::test]
async fn malformed_and_partial_queries_do_not_fall_back_to_input() {
    let h = Harness::new(json!({})).await;
    let id = Uuid::new_v4();
    for query in [
        "view=other".into(),
        "view=input&view=input".into(),
        "actor=owner".into(),
        "offset=0".into(),
        "view=body".into(),
        format!("expectedNodeVisitId={id}"),
        "submissionIds=".into(),
        format!("submissionIds={id},{id}"),
        "submissionIds=not-uuid".into(),
        "expectedWorkflowStateVersion=-1".into(),
        "bodyKind=CURRENT_DETAIL".into(),
    ] {
        let (status, body, _) = h.get(h.seed.owner, &query).await;
        assert_eq!(status, 422, "query={query} body={body}");
        assert_eq!(body["error"]["code"], "invalid_current_task_input");
    }
}

#[tokio::test]
async fn entry_is_causal_and_same_visit_context_revision_is_not_corruption() {
    let h = Harness::new(json!({"revision":1})).await;
    let initial = h.input().await;
    svc_workflow::application::workflow_instance::revise::revise_workflow_context(
        &h.pool,
        AdmissionGate::disabled(),
        ReviseWorkflowContextCommand {
            principal_id: PrincipalId::from_uuid(h.seed.creator),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".into(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(h.instance),
            expected_workflow_state_version: 1,
            context_payload: json!({"revision":2}),
        },
    )
    .await
    .unwrap();
    let revised = h.input().await;
    assert_eq!(revised["entry"]["status"], "INITIAL");
    assert_eq!(
        revised["binding"]["node_visit_id"],
        initial["binding"]["node_visit_id"]
    );
    assert_eq!(
        revised["entry"]["context_revision_id"],
        initial["binding"]["context_revision_id"]
    );
    assert_ne!(
        revised["entry"]["context_revision_id"],
        revised["binding"]["context_revision_id"]
    );
    let step = h
        .step(
            h.seed.creator,
            2,
            h.seed.draft_advance,
            Some(json!({"actual":"incoming"})),
        )
        .await;
    let input = h.input().await;
    assert_eq!(input["entry"]["status"], "TRANSITION");
    assert_eq!(
        input["entry"]["submission_id"],
        step.submission_id.unwrap().to_string()
    );
    assert_eq!(
        input["entry"]["source_node_visit_id"],
        step.source_node_visit_id.to_string()
    );
    let (status, body, _) = h.get(h.seed.owner, &binding_query(&initial)).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "current_task_input_stale");
}

#[tokio::test]
async fn later_corrections_and_requested_older_originals_are_server_selected() {
    let h = Harness::new(json!({})).await;
    let mut ids = Vec::new();
    let mut version = 1;
    for cycle in 0..10 {
        let advance = h
            .step(
                h.seed.creator,
                version,
                h.seed.draft_advance,
                Some(json!({"correction":cycle})),
            )
            .await;
        version += 1;
        ids.push(advance.submission_id.unwrap());
        let feedback=h.step(h.seed.assignee,version,h.seed.return_transition,Some(json!({"reasonCode":"REVIEW","reason":"a reason is not semantic truth","rootCauseNodeVisitId":advance.current_node_visit_id,"relatedSubmissionIds":ids}))).await;
        version += 1;
        ids.push(feedback.submission_id.unwrap());
    }
    let input = h.input().await;
    assert_eq!(input["recent_submission_ids"].as_array().unwrap().len(), 8);
    assert_eq!(input["recent_submissions_truncated"], true);
    assert_eq!(input["return_feedback_truncated"], true);
    assert_eq!(input["return_feedback"].as_array().unwrap().len(), 4);
    assert_eq!(
        input["return_feedback"][0]["related_submission_ids"]
            .as_array()
            .unwrap()
            .len(),
        16
    );
    assert_eq!(
        input["return_feedback"][0]["related_submission_ids_truncated"],
        true
    );
    assert_eq!(
        input["recent_submission_ids"][0],
        ids.last().unwrap().to_string()
    );
    assert!(!input["recent_submission_ids"]
        .as_array()
        .unwrap()
        .contains(&json!(ids[0])));
    let missing = Uuid::new_v4();
    let (status, selected, _) = h
        .get(
            h.seed.owner,
            &format!("submissionIds={},{}", ids[0], missing),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(selected["input_status"], "INCOMPLETE");
    assert!(selected["requested_submissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == ids[0].to_string() && r["status"] == "FOUND"));
    let original = selected["submissions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["submission_id"] == ids[0].to_string())
        .unwrap();
    let body: Value = serde_json::from_slice(&h.bytes(&selected, &original["body"]).await).unwrap();
    assert_eq!(body, json!({"correction":0}));
}

#[tokio::test]
async fn long_unicode_bodies_reconstruct_and_stale_hash_never_emits_chunk() {
    let original = json!({"large":"😀\"\\\n".repeat(15000)});
    let h = Harness::new(original.clone()).await;
    let input = h.input().await;
    assert_eq!(input["input_status"], "INCOMPLETE");
    assert_eq!(input["detail_body"]["content_status"], "TRUNCATED");
    let detail: Value =
        serde_json::from_slice(&h.bytes(&input, &input["detail_body"]).await).unwrap();
    assert_eq!(detail["current_context"]["payload"], original);
    let d = &input["detail_body"];
    let q = format!(
        "view=body&{}&bodyKind=CURRENT_DETAIL&bodyId={}&expectedBodySha256={}",
        binding_query(&input),
        d["id"].as_str().unwrap(),
        "0".repeat(64)
    );
    let (status, body, _) = h.get(h.seed.owner, &q).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "current_task_body_changed");
    assert!(body.get("body").is_none());
    // The Domain read projection changes without changing this instance tuple.
    set_domain_enabled(&h.pool, h.seed.domain, false).await;
    let q = format!(
        "view=body&{}&bodyKind=CURRENT_DETAIL&bodyId={}&expectedBodySha256={}",
        binding_query(&input),
        d["id"].as_str().unwrap(),
        d["sha256"].as_str().unwrap()
    );
    let (status, body, _) = h.get(h.seed.owner, &q).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "current_task_body_changed");
}

#[tokio::test]
async fn combined_entry_uses_new_context_and_submission_causal_chain() {
    let h = Harness::new(json!({"old":true})).await;
    let r=svc_workflow::application::workflow_instance::revise_and_transition::revise_context_and_transition(&h.pool,AdmissionGate::disabled(),ReviseContextAndTransitionCommand{principal_id:PrincipalId::from_uuid(h.seed.creator),idempotency_key:Uuid::new_v4().to_string(),command_schema_version:"v1".into(),workflow_instance_id:WorkflowInstanceId::from_uuid(h.instance),expected_workflow_state_version:1,transition_definition_id:TransitionId::from_uuid(h.seed.draft_advance),context_payload:json!({"new":true}),submission_payload:json!({"review":"newcontext"})}).await.unwrap();
    let input = h.input().await;
    assert_eq!(
        input["entry"]["event_type"],
        "WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED"
    );
    assert_eq!(
        input["entry"]["context_revision_id"],
        r.current_context_revision_id.to_string()
    );
    assert_eq!(input["entry"]["submission_id"], r.submission_id.to_string());
}

#[tokio::test]
async fn exact_actor_detail_detects_target_eligibility_drift_without_state_change() {
    let h = Harness::new(json!({})).await;
    let (status, input, _) = h.get(h.seed.creator, "").await;
    assert_eq!(status, 200);
    let d = &input["detail_body"];
    set_principal_enabled(&h.pool, h.seed.assignee, false).await;
    let q = format!(
        "view=body&{}&bodyKind=CURRENT_DETAIL&bodyId={}&expectedBodySha256={}",
        binding_query(&input),
        d["id"].as_str().unwrap(),
        d["sha256"].as_str().unwrap()
    );
    let (status, body, _) = h.get(h.seed.creator, &q).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "current_task_body_changed");
}

#[path = "current_task_input/entry_and_integrity.rs"]
mod entry_and_integrity;
#[path = "current_task_input/negative_and_models.rs"]
mod negative_and_models;
#[path = "current_task_input/snapshot_and_failure.rs"]
mod snapshot_and_failure;
