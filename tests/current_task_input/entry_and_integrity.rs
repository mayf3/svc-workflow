use super::*;

/// Append-only fixture seam: real tables/constraints remain enabled. This models
/// a future/administrative introducer without mutating existing immutable facts.
async fn introduce_fixture(h: &Harness, kind: &str, data: Option<Value>) -> Uuid {
    let mut tx = h.pool.begin().await.unwrap();
    let (old,context):(Uuid,Uuid)=sqlx::query_as("SELECT current_node_visit_id,current_context_revision_id FROM workflow_instances WHERE workflow_instance_id=$1").bind(h.instance).fetch_one(&mut *tx).await.unwrap();
    let visit = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id) VALUES($1,$2,$3,2,$4)").bind(visit).bind(h.instance).bind(h.seed.draft).bind(h.seed.creator).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,event_type,source_node_visit_id,target_node_visit_id,context_revision_id,event_data,actor_principal_id,from_node_id,to_node_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,2,'v1',$3,$4,$5,$6,$7,$8,$9,$9,1,2)").bind(Uuid::new_v4()).bind(h.instance).bind(kind).bind(old).bind(visit).bind(context).bind(data).bind(h.seed.owner).bind(h.seed.draft).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE workflow_instances SET current_node_visit_id=$1,workflow_state_version=2 WHERE workflow_instance_id=$2").bind(visit).bind(h.instance).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    visit
}

#[tokio::test]
async fn unknown_entry_with_nullable_data_is_incomplete_not_initial_or_storage_failure() {
    let h = Harness::new(json!({})).await;
    let visit = introduce_fixture(&h, "FUTURE_INTRODUCER", None).await;
    let input = h.input().await;
    assert_eq!(input["entry"]["status"], "UNSUPPORTED_ENTRY");
    assert_eq!(input["entry"]["target_node_visit_id"], visit.to_string());
    assert!(input["entry"]["event_type"].is_null());
    assert_eq!(input["input_status"], "INCOMPLETE");
}

#[tokio::test]
async fn bounded_administrative_fixtures_preserve_nontransition_references() {
    for kind in [
        "HUMAN_EXECUTOR_NORMALIZED",
        "PRINCIPAL_SUCCESSOR_MIGRATION_COMMITTED",
    ] {
        let h = Harness::new(json!({})).await;
        let visit = introduce_fixture(&h, kind, Some(json!({}))).await;
        let input = h.input().await;
        assert_eq!(input["entry"]["status"], "NON_TRANSITION_ENTRY");
        assert_eq!(input["entry"]["event_type"], kind);
        assert_eq!(input["entry"]["target_node_visit_id"], visit.to_string());
        assert_eq!(input["entry"]["submission_status"], "NONE_RECORDED");
        assert!(input["entry"]["transition_id"].is_null());
    }
}

#[tokio::test]
async fn actual_administrative_override_has_no_invented_transition() {
    use svc_workflow::domain::workflow_instance::recovery::*;
    let h = Harness::new(json!({})).await;
    sqlx::query("INSERT INTO domain_role_bindings(binding_id,domain_id,principal_id,role_key) VALUES($1,$2,$3,'WORKFLOW_ADMIN')").bind(Uuid::new_v4()).bind(h.seed.domain).bind(h.seed.owner).execute(&h.pool).await.unwrap();
    let r = svc_workflow::application::workflow_instance::admin_recovery::admin_emergency_override(
        &h.pool,
        AdmissionGate::disabled(),
        AdminEmergencyOverrideCommand {
            principal_id: PrincipalId::from_uuid(h.seed.owner),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".into(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(h.instance),
            expected_workflow_state_version: 1,
            operation: AdminEmergencyOperation::MoveToNode,
            target_node_id: NodeId::from_uuid(h.seed.normal),
            reason: "isolated test".into(),
            related_references: vec![AdminRelatedReference {
                resource_type: "TEST".into(),
                resource_id: "CTI-002".into(),
            }],
            expected_before_snapshot_digest: None,
        },
    )
    .await
    .unwrap();
    let input = h.input().await;
    assert_eq!(input["entry"]["status"], "NON_TRANSITION_ENTRY");
    assert_eq!(
        input["entry"]["event_type"],
        "ADMIN_EMERGENCY_OVERRIDE_COMMITTED"
    );
    assert!(input["entry"]["transition_id"].is_null());
    assert_eq!(
        input["binding"]["workflow_state_version"],
        r.workflow_state_version
    );
}

#[tokio::test]
async fn imported_normal_visit_is_initial_without_visit_number_guessing() {
    use svc_workflow::domain::workflow_instance::import::*;
    let mut h = Harness::new(json!({})).await;
    let service = Uuid::new_v4();
    sqlx::query("INSERT INTO principals(principal_id,principal_type,display_name) VALUES($1,'SERVICE','CTI isolated importer')").bind(service).execute(&h.pool).await.unwrap();
    sqlx::query("INSERT INTO domain_role_bindings(binding_id,domain_id,principal_id,role_key) VALUES($1,$2,$3,'WORKFLOW_MIGRATION')").bind(Uuid::new_v4()).bind(h.seed.domain).bind(service).execute(&h.pool).await.unwrap();
    let domain_key: String =
        sqlx::query_scalar("SELECT domain_key FROM domains WHERE domain_id=$1")
            .bind(h.seed.domain)
            .fetch_one(&h.pool)
            .await
            .unwrap();
    let node_key: String =
        sqlx::query_scalar("SELECT node_key FROM workflow_node_definitions WHERE node_id=$1")
            .bind(h.seed.normal)
            .fetch_one(&h.pool)
            .await
            .unwrap();
    let record = Uuid::new_v4();
    let snapshot = LegacyAdcImportSnapshotV1 {
        schema_version: SNAPSHOT_SCHEMA_VERSION.into(),
        requirement_id: record,
        domain_key,
        workflow_id: "cti-import".into(),
        workflow_snapshot: json!({"id":"cti-import"}),
        current_step: node_key,
        assignee_id: Some(h.seed.assignee),
        requester_id: Some(h.seed.creator),
        state_version: 9,
        updated_at: "2026-10-27T00:00:00Z".into(),
        context_payload: json!({"imported":true}),
    };
    let digest = snapshot.digest().unwrap();
    let r = svc_workflow::application::workflow_instance::import::import_legacy_workflow_instance(
        &h.pool,
        AdmissionGate::disabled(),
        ImportLegacyWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(service),
            command_schema_version: COMMAND_SCHEMA_VERSION.into(),
            domain_id: DomainId::from_uuid(h.seed.domain),
            definition_version_id: DefinitionVersionId::from_uuid(h.seed.version),
            imported_node_id: NodeId::from_uuid(h.seed.normal),
            legacy_record_id: record,
            legacy_snapshot: snapshot,
            expected_legacy_snapshot_digest: digest,
            legacy_creator_principal_id: Some(PrincipalId::from_uuid(h.seed.creator)),
            external_url: None,
            metadata: json!({}),
        },
    )
    .await
    .unwrap();
    h.instance = r.workflow_instance_id;
    let input = h.input().await;
    assert_eq!(input["entry"]["status"], "INITIAL");
    assert_eq!(input["entry"]["event_type"], "WORKFLOW_INSTANCE_IMPORTED");
    assert!(input["entry"]["source_node_visit_id"].is_null());
    assert_eq!(
        input["entry"]["context_revision_id"],
        r.current_context_revision_id.to_string()
    );
}

#[tokio::test]
async fn appended_digest_corruption_is_internal_not_partial_success() {
    let h = Harness::new(json!({})).await;
    let input = h.input().await;
    let id = Uuid::new_v4();
    // A valid unconsumed source Visit is used, so no immutable update, disabled
    // trigger, FK violation or already-consumed source is needed for this seam.
    sqlx::query("INSERT INTO workflow_submissions(submission_id,workflow_instance_id,source_node_visit_id,context_revision_id,author_principal_id,transition_id,payload,payload_digest,schema_version) VALUES($1,$2,$3,$4,$5,$6,'{}',$7,'v1')").bind(id).bind(h.instance).bind(input["binding"]["node_visit_id"].as_str().unwrap().parse::<Uuid>().unwrap()).bind(input["binding"]["context_revision_id"].as_str().unwrap().parse::<Uuid>().unwrap()).bind(h.seed.creator).bind(h.seed.draft_advance).bind("0".repeat(64)).execute(&h.pool).await.unwrap();
    let counts = business_counts(&h.pool).await;
    for query in [
        String::new(),
        format!(
            "view=body&{}&bodyKind=SUBMISSION_PAYLOAD&bodyId={id}&expectedBodySha256={}",
            binding_query(&input),
            "0".repeat(64)
        ),
    ] {
        let (status, b, _) = h.get(h.seed.owner, &query).await;
        assert_eq!(status, 500);
        assert_eq!(b["error"]["code"], "internal_consistency_error");
        assert!(b.get("body").is_none());
    }
    assert_eq!(counts, business_counts(&h.pool).await);
}

#[tokio::test]
async fn exact_entry_survives_newer_equal_timestamp_submissions_with_stable_uuid_order() {
    let h = Harness::new(json!({})).await;
    let entered = h
        .step(
            h.seed.creator,
            1,
            h.seed.draft_advance,
            Some(json!({"entry":true})),
        )
        .await;
    let current = h.input().await;
    let context: Uuid = current["binding"]["context_revision_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let mut inserted = Vec::new();
    // Distinct valid source Visits permit immutable appended facts at exactly the
    // same timestamp. The original introducing Event is deliberately untouched.
    for visit_number in 2..12 {
        let visit = Uuid::new_v4();
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id) VALUES($1,$2,$3,$4,$5)").bind(visit).bind(h.instance).bind(h.seed.draft).bind(visit_number).bind(h.seed.creator).execute(&h.pool).await.unwrap();
        let payload = json!({"later":visit_number});
        let digest = svc_workflow::domain::definition::digest::compute_sha256(
            jcs_canonicalize::canonicalize(&payload.to_string())
                .unwrap()
                .as_bytes(),
        );
        sqlx::query("INSERT INTO workflow_submissions(submission_id,workflow_instance_id,source_node_visit_id,context_revision_id,author_principal_id,transition_id,payload,payload_digest,schema_version,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'v1','2099-01-01T00:00:00Z')").bind(id).bind(h.instance).bind(visit).bind(context).bind(h.seed.creator).bind(h.seed.draft_advance).bind(payload).bind(digest).execute(&h.pool).await.unwrap();
        inserted.push(id);
    }
    let counts = business_counts(&h.pool).await;
    let input = h.input().await;
    inserted.sort_by(|a, b| b.cmp(a));
    assert_eq!(input["recent_submission_ids"], json!(&inserted[..8]));
    assert_eq!(
        input["entry"]["submission_id"],
        entered.submission_id.unwrap().to_string()
    );
    assert_eq!(
        input["submissions"][0]["submission_id"],
        entered.submission_id.unwrap().to_string()
    );
    assert_eq!(input["recent_submissions_truncated"], true);
    assert_eq!(counts, business_counts(&h.pool).await);
}

#[tokio::test]
async fn malformed_transition_entry_data_and_duplicate_introducers_fail_internally() {
    for duplicate in [false, true] {
        let h = Harness::new(json!({})).await;
        // New facts, not mutations of immutable Events. Both fixtures satisfy SQL
        // constraints but break the projection's required semantic relationship.
        let input = h.input().await;
        let source: Uuid = input["binding"]["node_visit_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let context: Uuid = input["binding"]["context_revision_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let target = if duplicate { source } else { Uuid::new_v4() };
        let mut tx = h.pool.begin().await.unwrap();
        if !duplicate {
            sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,1,$4,$5)").bind(target).bind(h.instance).bind(h.seed.normal).bind(h.seed.assignee).bind(h.seed.draft_advance).execute(&mut *tx).await.unwrap();
        }
        let data = json!({"transition_definition_id":h.seed.draft_advance,"source_node_visit_id":Uuid::new_v4(),"target_node_visit_id":target,"context_revision_id":context,"transition_effect":"ADVANCE","submission_payload_digest":null});
        sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,event_type,transition_effect,source_node_visit_id,target_node_visit_id,context_revision_id,event_data,actor_principal_id,from_node_id,to_node_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,2,'v1',$3,'ADVANCE',$4,$5,$6,$7,$8,$9,$10,1,2)").bind(Uuid::new_v4()).bind(h.instance).bind(if duplicate{"FUTURE_DUPLICATE"}else{"WORKFLOW_TRANSITION_COMMITTED"}).bind(if duplicate{None}else{Some(source)}).bind(target).bind(context).bind(data).bind(h.seed.creator).bind(h.seed.draft).bind(if duplicate{h.seed.draft}else{h.seed.normal}).execute(&mut *tx).await.unwrap();
        sqlx::query("UPDATE workflow_instances SET current_node_visit_id=$1,workflow_state_version=2 WHERE workflow_instance_id=$2").bind(target).bind(h.instance).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        let (status, b, _) = h.get(h.seed.owner, "").await;
        assert_eq!(status, 500);
        assert_eq!(b["error"]["code"], "internal_consistency_error");
    }
}

#[tokio::test]
async fn cross_instance_event_refs_are_rejected_by_live_composite_constraints() {
    let h = Harness::new(json!({})).await;
    let other = Harness::new(json!({})).await;
    let local = h.input().await;
    let foreign = other.input().await;
    let local_visit: Uuid = local["binding"]["node_visit_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let local_context: Uuid = local["binding"]["context_revision_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let foreign_visit: Uuid = foreign["binding"]["node_visit_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let foreign_context: Uuid = foreign["binding"]["context_revision_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let counts = business_counts(&h.pool).await;
    for (source, context, constraint) in [
        (
            foreign_visit,
            local_context,
            "fk_event_source_visit_same_instance",
        ),
        (local_visit, foreign_context, "fk_event_ctx_same_instance"),
    ] {
        let mut tx = h.pool.begin().await.unwrap();
        sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,event_type,source_node_visit_id,target_node_visit_id,context_revision_id,event_data,actor_principal_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,2,'v1','ADMIN_EMERGENCY_OVERRIDE_COMMITTED',$3,$4,$5,'{}',$6,1,2)").bind(Uuid::new_v4()).bind(h.instance).bind(source).bind(local_visit).bind(context).bind(h.seed.owner).execute(&mut *tx).await.unwrap();
        let err = tx
            .commit()
            .await
            .expect_err("composite FK must reject at commit without disabling constraints");
        let db = err.as_database_error().unwrap();
        assert_eq!(db.code().as_deref(), Some("23503"));
        assert_eq!(db.constraint(), Some(constraint));
    }
    assert_eq!(counts, business_counts(&h.pool).await);
    assert_eq!(h.input().await["binding"], local["binding"]);
}
