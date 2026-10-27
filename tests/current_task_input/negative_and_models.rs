use super::*;

#[tokio::test]
async fn model3_task_entry_allows_null_submission() {
    let mut h = Harness::new(json!({})).await;
    let (version, _, _, _, _, advance, _, _) =
        seed_v1_definition(&h.pool, h.seed.domain, h.seed.assignee, false).await;
    h.instance = create_workflow_instance(
        &h.pool,
        AdmissionGate::disabled(),
        make_command(h.seed.creator, h.seed.domain, version),
    )
    .await
    .unwrap()
    .workflow_instance_id;
    assert_eq!(h.input().await["entry"]["status"], "INITIAL");
    let r = h.step(h.seed.creator, 1, advance, None).await;
    assert!(r.submission_id.is_none());
    let input = h.input().await;
    assert_eq!(input["entry"]["status"], "TRANSITION");
    assert_eq!(input["entry"]["submission_status"], "NONE_RECORDED");
    assert!(input["entry"]["submission_id"].is_null());
    assert!(input["submissions"].as_array().unwrap().is_empty());
    assert_eq!(h.get(h.seed.assignee, "").await.0, 200);
}

#[tokio::test]
async fn global_grants_do_not_escalate_and_denial_audit_failure_is_closed() {
    let h = Harness::new(json!({"secret":"test-only"})).await;
    for role in [
        "GLOBAL_WORKFLOW_READER",
        "GLOBAL_WORKFLOW_COORDINATOR",
        "GLOBAL_SCHEDULER_READ",
    ] {
        sqlx::query(
            "INSERT INTO global_role_bindings(binding_id,principal_id,role_key) VALUES($1,$2,$3)",
        )
        .bind(Uuid::new_v4())
        .bind(h.seed.outsider)
        .bind(role)
        .execute(&h.pool)
        .await
        .unwrap();
        let counts = business_counts(&h.pool).await;
        let (status, b, _) = h.get(h.seed.outsider, "").await;
        assert_eq!(status, 404);
        assert_eq!(
            b["error"]["code"],
            "workflow_instance_not_found_or_not_visible"
        );
        assert_eq!(counts, business_counts(&h.pool).await);
    }
    let audits:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_security_audits WHERE principal_id=$1 AND details->>'queryType'='CurrentTaskInput'").bind(h.seed.outsider).fetch_one(&h.pool).await.unwrap();
    assert_eq!(audits, 3);
    let before = business_counts(&h.pool).await;
    let success_audits: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_security_audits")
        .fetch_one(&h.pool)
        .await
        .unwrap();
    h.input().await;
    assert_eq!(before, business_counts(&h.pool).await);
    assert_eq!(
        success_audits,
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM workflow_security_audits")
            .fetch_one(&h.pool)
            .await
            .unwrap()
    );
    let guard = QueryAuditTriggerGuard::install(&h.pool, h.seed.outsider).await;
    let (status, b, _) = h.get(h.seed.outsider, "").await;
    assert_eq!(status, 503);
    assert_eq!(b["error"]["code"], "service_unavailable");
    assert!(b.get("binding").is_none());
    drop(guard);
    assert_eq!(h.get(h.seed.outsider, "").await.0, 404);
}

#[tokio::test]
async fn foreign_refs_offsets_and_permission_rechecks_remain_exact() {
    let h = Harness::new(json!({"utf8":"😀"})).await;
    let other = Harness::new(json!({})).await;
    let foreign = other
        .step(
            other.seed.creator,
            1,
            other.seed.draft_advance,
            Some(json!({"foreign":true})),
        )
        .await
        .submission_id
        .unwrap();
    let input = h.input().await;
    let (status, b, _) = h
        .get(h.seed.owner, &format!("submissionIds={foreign}"))
        .await;
    assert_eq!(status, 200);
    assert_eq!(
        b["requested_submissions"][0]["status"],
        "NOT_FOUND_OR_NOT_VISIBLE"
    );
    assert_eq!(b["input_status"], "INCOMPLETE");
    for (kind, id) in [
        ("SUBMISSION_PAYLOAD", foreign),
        ("CURRENT_DETAIL", Uuid::new_v4()),
    ] {
        let (status, b, _) = h
            .get(
                h.seed.owner,
                &format!(
                    "view=body&{}&bodyKind={kind}&bodyId={id}&expectedBodySha256={}",
                    binding_query(&input),
                    "0".repeat(64)
                ),
            )
            .await;
        assert_eq!(status, 404);
        assert_eq!(
            b["error"]["code"],
            "current_task_body_not_found_or_not_visible"
        );
    }
    let d = &input["detail_body"];
    let bytes = h.bytes(&input, d).await;
    let invalid_utf8 = bytes.windows(4).position(|b| b == "😀".as_bytes()).unwrap() + 1;
    let base = format!(
        "view=body&{}&bodyKind=CURRENT_DETAIL&bodyId={}&expectedBodySha256={}",
        binding_query(&input),
        d["id"].as_str().unwrap(),
        d["sha256"].as_str().unwrap()
    );
    for offset in [invalid_utf8, bytes.len() + 1] {
        let (status, b, _) = h
            .get(h.seed.owner, &format!("{base}&offset={offset}"))
            .await;
        assert_eq!(status, 422);
        assert_eq!(b["error"]["code"], "invalid_current_task_input");
    }
    let (status, b, _) = h
        .get(h.seed.owner, &format!("{base}&offset={}", bytes.len()))
        .await;
    assert_eq!(status, 200);
    assert_eq!(b["body"]["text"], "");
    assert_eq!(b["body"]["content_status"], "COMPLETE");
    assert!(b["body"]["next_offset"].is_null());
    set_principal_enabled(&h.pool, h.seed.owner, false).await;
    assert_eq!(h.get(h.seed.owner, &base).await.0, 403);
}

#[tokio::test]
async fn parsed_uuid_duplicates_and_numeric_overflow_reject() {
    let h = Harness::new(json!({})).await;
    let id = Uuid::new_v4();
    let binding = binding_query(&h.input().await);
    for q in [format!("submissionIds={id},{}",id.to_string().to_uppercase()),format!("view=body&{binding}&bodyKind=CURRENT_DETAIL&bodyId={id}&expectedBodySha256={}&offset=18446744073709551616","0".repeat(64)),"expectedWorkflowStateVersion=2147483648&expectedNodeVisitId=00000000-0000-0000-0000-000000000000&expectedContextRevisionId=00000000-0000-0000-0000-000000000000".into()] {assert_eq!(h.get(h.seed.owner,&q).await.0,422);}
}

#[tokio::test]
async fn large_submission_and_empty_json_reconstruct_stored_digest() {
    let h = Harness::new(json!({})).await;
    let original =
        json!({"escaped":"\u{0001}😀\"".repeat(12000),"empty":{},"array":[],"null":null});
    let s = h
        .step(
            h.seed.creator,
            1,
            h.seed.draft_advance,
            Some(original.clone()),
        )
        .await
        .submission_id
        .unwrap();
    let input = h.input().await;
    let d = &input["submissions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["submission_id"] == s.to_string())
        .unwrap()["body"];
    let exact: Value = serde_json::from_slice(&h.bytes(&input, d).await).unwrap();
    assert_eq!(exact, original);
    let stored: String = sqlx::query_scalar(
        "SELECT payload_digest FROM workflow_submissions WHERE submission_id=$1",
    )
    .bind(s)
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert_eq!(d["sha256"], stored);
}

#[tokio::test]
async fn omitted_selected_body_keeps_exact_zero_offset_descriptor() {
    let h = Harness::new(json!({"context":"😀".repeat(10000)})).await;
    let first = h
        .step(
            h.seed.creator,
            1,
            h.seed.draft_advance,
            Some(json!({"large":"x".repeat(20000)})),
        )
        .await;
    h.step(h.seed.assignee,2,h.seed.return_transition,Some(json!({"reasonCode":"TEST","reason":"y".repeat(20000),"rootCauseNodeVisitId":first.current_node_visit_id,"relatedSubmissionIds":[first.submission_id.unwrap()]}))).await;
    let input = h.input().await;
    let omitted = input["submissions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["body"]["content_status"] == "OMITTED")
        .expect("later selected body must retain descriptor when inline budget is consumed");
    assert_eq!(omitted["body"]["offset"], 0);
    assert_eq!(omitted["body"]["next_offset"], 0);
    assert_eq!(omitted["body"]["text"], "");
    let exact: Value = serde_json::from_slice(&h.bytes(&input, &omitted["body"]).await).unwrap();
    assert_eq!(exact, json!({"large":"x".repeat(20000)}));
}

#[tokio::test]
async fn long_instructions_and_outgoing_schema_remain_exact_full_detail() {
    let mut h = Harness::new(json!({})).await;
    let (version, _, _, _, _, _, _, _) =
        seed_v1_definition(&h.pool, h.seed.domain, h.seed.assignee, true).await;
    h.instance = create_workflow_instance(
        &h.pool,
        AdmissionGate::disabled(),
        make_command(h.seed.creator, h.seed.domain, version),
    )
    .await
    .unwrap()
    .workflow_instance_id;
    let input = h.input().await;
    assert_eq!(input["content_complete"], false);
    let reconstructed: Value =
        serde_json::from_slice(&h.bytes(&input, &input["detail_body"]).await).unwrap();
    let old = query_service(&h.pool)
        .get_workflow_instance_detail(
            svc_workflow::application::workflow_instance::query_types::GetWorkflowInstanceDetail {
                actor_principal_id: h.seed.owner,
                workflow_instance_id: h.instance,
            },
        )
        .await
        .unwrap();
    assert_eq!(reconstructed, serde_json::to_value(old).unwrap()["detail"]);
    let text = reconstructed.to_string();
    assert!(text.contains("instructions 😀"));
    assert!(text.contains("schema 😀"));
}
