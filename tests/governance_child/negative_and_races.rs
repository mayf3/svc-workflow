use super::*;

#[sqlx::test]
async fn nonselected_terminal_has_review_but_no_resolve_proof(pool: PgPool) {
    let alt = Uuid::new_v4();
    let f = setup_with_schema_and_alternate(&pool, schema(), Some(alt)).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let b = bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
        .await
        .unwrap();
    let mut cmd = transition_command(&c, 1);
    cmd.transition_definition_id = TransitionId::from_uuid(alt);
    cmd.submission_payload = None;
    execute_workflow_transition(&pool, crate::common::admitting_gate(), cmd.clone())
        .await
        .unwrap();
    execute_workflow_transition(&pool, crate::common::admitting_gate(), cmd)
        .await
        .unwrap();
    let rows = review_rows(&pool).await;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].get("childSubmissionId").is_none());
    assert!(rows[0].get("childSubmissionDigest").is_none());
    assert_eq!(rows[0]["acceptanceTransitionId"], json!(f.transition));
    let d = serde_json::to_value(
        get_assistance_case(&pool, f.owner, case.assistance_case_id)
            .await
            .unwrap(),
    )
    .unwrap();
    let p = &d["governanceTerminalProof"];
    assert!(resolve_assistance(&pool,resolve_cmd(&f,case.assistance_case_id,json!({"bindingId":b["bindingId"],"terminalEventId":p["terminalEventId"],"submissionId":Uuid::new_v4(),"submissionDigest":"0".repeat(64),"acceptanceContractDigest":b["acceptanceContractDigest"]}))).await.is_err());
}
#[sqlx::test]
async fn exact_not_accepted_and_reference_mismatch_matrix_cannot_resolve(pool: PgPool) {
    for verdict in ["NOT_ACCEPTED", "ACCEPTED"] {
        let f = setup_with_schema(&pool, schema()).await;
        let c = child(&pool, &f).await;
        let case = request_case(&pool, &f).await;
        let b = bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
            .await
            .unwrap();
        finish(&pool, &c, &b, verdict).await;
        let d = serde_json::to_value(
            get_assistance_case(&pool, f.owner, case.assistance_case_id)
                .await
                .unwrap(),
        )
        .unwrap();
        let p = &d["governanceTerminalProof"];
        let reference = json!({"bindingId":b["bindingId"],"terminalEventId":p["terminalEventId"],"submissionId":p["submissionId"],"submissionDigest":p["submissionDigest"],"acceptanceContractDigest":b["acceptanceContractDigest"]});
        if verdict == "NOT_ACCEPTED" {
            assert!(
                resolve_assistance(&pool, resolve_cmd(&f, case.assistance_case_id, reference))
                    .await
                    .is_err()
            );
            continue;
        }
        for field in [
            "bindingId",
            "terminalEventId",
            "submissionId",
            "submissionDigest",
            "acceptanceContractDigest",
            "unknown",
        ] {
            let mut wrong = reference.clone();
            wrong[field] = if field.ends_with("Digest") {
                json!("0".repeat(64))
            } else {
                json!(Uuid::new_v4())
            };
            assert!(
                resolve_assistance(&pool, resolve_cmd(&f, case.assistance_case_id, wrong))
                    .await
                    .is_err(),
                "{field}"
            );
        }
        for field in [
            "bindingId",
            "terminalEventId",
            "submissionId",
            "submissionDigest",
            "acceptanceContractDigest",
        ] {
            let mut missing = reference.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                resolve_assistance(&pool, resolve_cmd(&f, case.assistance_case_id, missing))
                    .await
                    .is_err(),
                "missing {field}"
            );
        }
        let cmd = resolve_cmd(&f, case.assistance_case_id, reference);
        resolve_assistance(&pool, cmd.clone()).await.unwrap();
        resolve_assistance(&pool, cmd).await.unwrap();
    }
}
#[sqlx::test]
async fn bind_rejects_foreign_domain_incomplete_provenance_and_nonacceptance_schema(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let other = setup_with_schema(&pool, schema()).await;
    let case = request_case(&pool, &f).await;
    assert!(bind_governance_child(
        &pool,
        bind_cmd(&f, case.assistance_case_id, other.instance, 2)
    )
    .await
    .is_err());
    let legacy = setup(&pool).await;
    let legacychild = child(&pool, &legacy).await;
    let legacycase = request_case(&pool, &legacy).await;
    assert!(bind_governance_child(
        &pool,
        bind_cmd(
            &legacy,
            legacycase.assistance_case_id,
            legacychild.instance,
            2
        )
    )
    .await
    .is_err());
    let f2 = setup_with_schema(&pool, schema()).await;
    let c2 = child(&pool, &f2).await;
    let fake = insert_open_case(&pool, &f2, f2.visit).await;
    assert!(
        bind_governance_child(&pool, bind_cmd(&f2, fake, c2.instance, 1))
            .await
            .is_err()
    );
    let c = child(&pool, &f).await;
    sqlx::query("UPDATE workflow_instances SET cancelled=TRUE WHERE workflow_instance_id=$1")
        .bind(c.instance)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
            .await
            .is_err()
    );
}
#[sqlx::test]
async fn bind_and_terminal_race_never_loses_review_obligation(pool: PgPool) {
    use std::sync::Arc;
    use tokio::sync::Barrier;
    for _ in 0..5 {
        let barrier = Arc::new(Barrier::new(3));
        let bp = pool.clone();
        let tp = pool.clone();
        let bb = barrier.clone();
        let tb = barrier.clone();
        // Alternate terminal path needs no as-yet-unknown binding ID and is legal
        // before bind; the selected acceptance route still requires its envelope.
        // Create this race fixture with a graph-defined alternative terminal route.
        let alt = Uuid::new_v4();
        let f = setup_with_schema_and_alternate(&pool, schema(), Some(alt)).await;
        let c = child(&pool, &f).await;
        let case = request_case(&pool, &f).await;
        let cmd = bind_cmd(&f, case.assistance_case_id, c.instance, 2);
        let mut term = transition_command(&c, 1);
        term.transition_definition_id = TransitionId::from_uuid(alt);
        term.submission_payload = None;
        let bind = tokio::spawn(async move {
            bb.wait().await;
            bind_governance_child(&bp, cmd).await
        });
        let terminal = tokio::spawn(async move {
            tb.wait().await;
            execute_workflow_transition(&tp, crate::common::admitting_gate(), term).await
        });
        barrier.wait().await;
        let br = bind.await.unwrap();
        let tr = terminal.await.unwrap();
        assert!(tr.is_ok(), "{tr:?}");
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_governance_child_bindings WHERE assistance_case_id=$1",
        )
        .bind(case.assistance_case_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let wakes:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_outbox WHERE payload->>'phase'='GOVERNANCE_CHILD_REVIEW' AND payload->>'assistanceCaseId'=$1").bind(case.assistance_case_id.to_string()).fetch_one(&pool).await.unwrap();
        assert_eq!(n, wakes);
        assert_eq!(n, if br.is_ok() { 1 } else { 0 });
    }
}
#[sqlx::test]
async fn unlinked_opaque_supporting_payload_does_not_add_review_event_fields(pool: PgPool) {
    let f = setup(&pool).await;
    let case = request_case(&pool, &f).await;
    let mut cmd = resolve_cmd(
        &f,
        case.assistance_case_id,
        json!({"arbitrary":"opaque legacy value"}),
    );
    cmd.expected_workflow_state_version = 2;
    resolve_assistance(&pool, cmd).await.unwrap();
    let e:Value=sqlx::query_scalar("SELECT event_data FROM workflow_events WHERE workflow_instance_id=$1 AND event_type='ASSISTANCE_RESOLVED'").bind(f.instance).fetch_one(&pool).await.unwrap();
    assert!(
        e.get("governanceChildReview").is_none(),
        "unlinked event shape must be unchanged"
    );
}
#[sqlx::test]
async fn terminal_and_owner_replacement_serialize_to_current_recipient(pool: PgPool) {
    use std::sync::Arc;
    use tokio::sync::Barrier;
    for _ in 0..5 {
        let f = setup_with_schema(&pool, schema()).await;
        let c = child(&pool, &f).await;
        let case = request_case(&pool, &f).await;
        let b = bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
            .await
            .unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let bp = pool.clone();
        let tp = pool.clone();
        let bb = barrier.clone();
        let tb = barrier.clone();
        let mut cmd = transition_command(&c, 1);
        cmd.submission_payload = Some(envelope(&b, "ACCEPTED"));
        let terminal = tokio::spawn(async move {
            tb.wait().await;
            execute_workflow_transition(&tp, crate::common::admitting_gate(), cmd).await
        });
        let owner = tokio::spawn(async move {
            bb.wait().await;
            replace(&bp, &f, f.replacement_owner, "racing-replacement").await
        });
        barrier.wait().await;
        terminal.await.unwrap().unwrap();
        owner.await.unwrap();
        let rows:Vec<(Uuid,Value)>=sqlx::query_as("SELECT r.owner_principal_id,o.payload FROM workflow_governance_child_reviews r JOIN workflow_outbox o USING(outbox_id) WHERE r.binding_id=$1 AND r.retired_reason IS NULL").bind(Uuid::parse_str(b["bindingId"].as_str().unwrap()).unwrap()).fetch_all(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, f.replacement_owner);
        let d = serde_json::to_value(
            get_assistance_case(&pool, f.replacement_owner, case.assistance_case_id)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            d["governanceBinding"]["currentReview"]["reviewWakeId"],
            rows[0].1["reviewWakeId"]
        );
    }
}

#[sqlx::test]
async fn binding_and_review_identity_are_immutable_in_storage(pool: PgPool) {
    let f = setup_with_schema(&pool, schema()).await;
    let c = child(&pool, &f).await;
    let case = request_case(&pool, &f).await;
    let b = bind_governance_child(&pool, bind_cmd(&f, case.assistance_case_id, c.instance, 2))
        .await
        .unwrap();
    finish(&pool, &c, &b, "ACCEPTED").await;
    assert!(sqlx::query("UPDATE workflow_governance_child_bindings SET acceptance_contract_digest=$2 WHERE assistance_case_id=$1").bind(case.assistance_case_id).bind("0".repeat(64)).execute(&pool).await.is_err());
    assert!(sqlx::query("UPDATE workflow_governance_child_bindings SET terminal_event_id=NULL,terminal_proof=NULL WHERE assistance_case_id=$1").bind(case.assistance_case_id).execute(&pool).await.is_err());
    assert!(
        sqlx::query("UPDATE workflow_governance_child_reviews SET review_wake_id=$1")
            .bind(Uuid::new_v4())
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM workflow_governance_child_reviews")
        .execute(&pool)
        .await
        .is_err());
}
