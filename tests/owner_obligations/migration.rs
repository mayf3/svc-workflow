use super::*;

#[tokio::test]
async fn migration_exact_five_relations() {
    let p = pool().await;
    let names:Vec<String>=sqlx::query_scalar("SELECT tablename::text FROM pg_tables WHERE schemaname='public' AND (tablename LIKE 'workflow_owner_%' OR tablename='workflow_operation_effects') ORDER BY tablename").fetch_all(&p).await.unwrap();
    assert_eq!(
        names,
        vec![
            "workflow_operation_effects",
            "workflow_owner_dependency_origins",
            "workflow_owner_managed_instances",
            "workflow_owner_obligation_intents",
            "workflow_owner_policy_approvals"
        ]
    );
}
#[tokio::test]
async fn migration_auto_success_receipt_without_actual_facts_is_rejected_on_both_types() {
    let p = pool().await;
    let (owner, _) = seed(&p).await;
    for kind in [
        "AUTO_CREATE_AND_BIND_OWNER_INTAKE",
        "AUTO_CREATE_OWNER_REVIEW",
    ] {
        let mut tx = p.begin().await.unwrap();
        sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status,response_status,response_body) VALUES($1,$2,$3,$4,$5,'COMPLETED',201,'{}')").bind(Uuid::new_v4()).bind(owner).bind(Uuid::new_v4().to_string()).bind(kind).bind("a".repeat(64)).execute(&mut *tx).await.unwrap();
        assert!(
            tx.commit().await.is_err(),
            "{kind} zero-fact success must fail commit"
        );
    }
}
#[tokio::test]
async fn migration_forward_preserves_real_old_history_and_old_unique_rollback_refuses_new_pair() {
    let p = predecessor_pool().await;
    let (owner, domain) = seed(&p).await;
    let (v, _) = version(&p, domain, owner, json!({"type":"object"})).await;
    publish(&p, owner, v, None, "old-publish").await.unwrap();
    create(
        &p,
        owner,
        domain,
        v,
        json!({"business":"real predecessor data"}),
    )
    .await
    .unwrap();
    let before = history(&p).await;
    sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
        .await
        .unwrap()
        .run(&p)
        .await
        .unwrap();
    let after = history(&p).await;
    assert_eq!(before, after);
    println!(
        "actual historical business rows pre/post JCS SHA {}",
        svc_workflow::domain::definition::digest::compute_json_digest(&before).unwrap()
    );
    let (_, _, _, managed, _, _, _) = operation_fixture(&p).await;
    let frozen = history(&p).await;
    assert!(sqlx::query("CREATE UNIQUE INDEX isolated_test_old_command_unique ON workflow_events(command_id) WHERE command_id IS NOT NULL").execute(&p).await.is_err());
    assert_eq!(history(&p).await, frozen);
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_events e JOIN workflow_owner_managed_instances m ON m.creation_command_id=e.command_id WHERE m.workflow_instance_id=$1").bind(managed).fetch_one(&p).await.unwrap();
    assert_eq!(n, 2);
}

#[tokio::test]
async fn migration_dependency_origin_requires_the_exact_actual_birth_event() {
    let p = pool().await;
    let (owner, domain, _, managed, supplier, _, _) = operation_fixture(&p).await;
    let leaf = create(&p, owner, domain, supplier, json!({}))
        .await
        .unwrap();
    let leaf_id = leaf.workflow_instance_id;
    let fake_command = Uuid::new_v4();
    let effect = Uuid::new_v4();
    let mut tx = p.begin().await.unwrap();
    // A completed receipt whose result names a real instance is insufficient: its
    // command did not create that instance. Preserve the real immutable birth.
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status,response_status,response_body,response_digest) SELECT $1,principal_id,$2,command_type,request_hash,receipt_status,response_status,response_body,response_digest FROM workflow_command_receipts WHERE command_type='CREATE_WORKFLOW_INSTANCE' AND response_body->>'workflowInstanceId'=$3")
        .bind(fake_command).bind(Uuid::new_v4().to_string()).bind(leaf_id.to_string()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO workflow_operation_effects(effect_id,case_id,managed_instance_id,definition_version_id,node_definition_id,slot_key,descriptor_key,descriptor_version,descriptor_hash,definition_graph_digest,first_authorizing_visit_id,actor_principal_id,business_intent_digest,concrete_request_digest,state,command_id,result,result_digest) SELECT $1,m.case_id,w.workflow_instance_id,w.definition_version_id,n.node_id,n.metadata->'workflowOperation'->>'slotKey',d->>'key',(d->>'version')::int,d->>'hash',a.intake_graph_digest,v.node_visit_id,$2,$3,$3,'SUCCEEDED',$4,r.response_body,r.response_digest FROM workflow_owner_managed_instances m JOIN workflow_instances w ON w.workflow_instance_id=m.workflow_instance_id JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n ON n.node_id=v.node_id JOIN workflow_owner_policy_approvals a ON a.source_definition_version_id=m.policy_source_version_id CROSS JOIN LATERAL jsonb_array_elements(n.metadata->'workflowOperation'->'allowedOperations') d JOIN workflow_command_receipts r ON r.command_id=$4 WHERE m.workflow_instance_id=$5")
        .bind(effect).bind(owner).bind("a".repeat(64)).bind(fake_command).bind(managed).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO workflow_owner_dependency_origins(created_instance_id,parent_effect_id,parent_managed_instance_id,case_id,create_command_id,target_definition_version_id) SELECT $1,$2,$3,case_id,$4,$5 FROM workflow_owner_managed_instances WHERE workflow_instance_id=$3")
        .bind(leaf_id).bind(effect).bind(managed).bind(fake_command).bind(supplier).execute(&mut *tx).await.unwrap();
    assert!(tx.commit().await.is_err(), "a receipt without this instance's actual INSTANCE_CREATED Event cannot establish dependency provenance");
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM workflow_command_receipts WHERE command_id=$1")
            .bind(fake_command)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(n, 0);
}
