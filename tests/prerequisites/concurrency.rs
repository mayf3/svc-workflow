use super::integrity_support::*;
use super::*;
#[tokio::test]
async fn ordinary_global_cardinality_survives_stale_snapshots_and_forged_slots() {
    let p = pool().await;
    let f = setup(&p).await;
    let (v, _) = version(&p, &f, false, false).await;
    for isolation in ["READ COMMITTED", "REPEATABLE READ", "SERIALIZABLE"] {
        let a = create(&p, &f, v, json!({})).await;
        let b = create(&p, &f, v, json!({})).await;
        let c = receipt(&p, f.owner, "ORDINARY_STORAGE_TEST").await;
        let mut t = p.begin().await.unwrap();
        complete(&mut t, c, 200).await.unwrap();
        t.commit().await.unwrap();
        let mut left = p.begin().await.unwrap();
        let mut right = p.begin().await.unwrap();
        for tx in [&mut left, &mut right] {
            sqlx::query(&format!("SET TRANSACTION ISOLATION LEVEL {isolation}"))
                .execute(&mut **tx)
                .await
                .unwrap();
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM workflow_events WHERE command_id=$1"
                )
                .bind(c)
                .fetch_one(&mut **tx)
                .await
                .unwrap(),
                0
            );
        }
        let one = async {
            append(&mut left, c, a, "INSTANCE_CREATED", 1).await?;
            left.commit().await
        };
        let two = async {
            append(&mut right, c, b, "ANY_OTHER_EVENT", 2).await?;
            right.commit().await
        };
        let (x, y) = tokio::join!(one, two);
        assert_eq!(
            usize::from(x.is_ok()) + usize::from(y.is_ok()),
            1,
            "{isolation}: {x:?}/{y:?}"
        );
        let e = x.err().or_else(|| y.err()).unwrap();
        println!("{isolation} ordinary loser: {}", e);
        let rows: Vec<(i16, String)> = sqlx::query_as(
            "SELECT command_event_slot,event_type FROM workflow_events WHERE command_id=$1",
        )
        .bind(c)
        .fetch_all(&p)
        .await
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].0, 0,
            "caller slots must be overwritten for all ordinary event kinds"
        );
    }
}
#[tokio::test]
async fn event_guard_waits_for_the_actual_receipt_row() {
    let p = pool().await;
    let f = setup(&p).await;
    let c = receipt(&p, f.owner, "ORDINARY_STORAGE_TEST").await;
    let mut holder = p.begin().await.unwrap();
    let holder_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *holder)
        .await
        .unwrap();
    sqlx::query("SELECT command_id FROM workflow_command_receipts WHERE command_id=$1 FOR UPDATE")
        .bind(c)
        .fetch_one(&mut *holder)
        .await
        .unwrap();
    let mut worker = p.begin().await.unwrap();
    let worker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *worker)
        .await
        .unwrap();
    let task = tokio::spawn(async move {
        append(&mut worker, c, f.instance, "OTHER", 2).await?;
        worker.commit().await
    });
    let mut observed = false;
    for _ in 0..40 {
        let blockers: Vec<i32> = sqlx::query_scalar("SELECT pg_blocking_pids($1)")
            .bind(worker_pid)
            .fetch_one(&p)
            .await
            .unwrap();
        if blockers.contains(&holder_pid) {
            observed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert!(observed, "actual receipt holder must block Event insertion");
    holder.rollback().await.unwrap();
    task.await.unwrap().unwrap();
}
#[tokio::test]
async fn duplicate_atomic_commands_have_one_child_pair_and_one_receipt() {
    let p = pool().await;
    let f = setup(&p).await;
    let case = request_case(&p, &f).await;
    let (v, t) = version(&p, &f, false, true).await;
    let command=svc_workflow::domain::workflow_instance::governance_child::CreateAndBindGovernanceChildCommand{principal_id:PrincipalId::from_uuid(f.owner),idempotency_key:Uuid::new_v4().to_string(),command_schema_version:"v1".into(),assistance_case_id:AssistanceCaseId::from_uuid(case.assistance_case_id),definition_version_id:DefinitionVersionId::from_uuid(v),context_payload:json!({}),acceptance_transition_id:TransitionId::from_uuid(t),expected_workflow_state_version:2};
    let (x, y) = tokio::join!(
        svc_workflow::application::workflow_instance::assistance::create_and_bind_governance_child(
            &p,
            crate::common::admitting_gate(),
            command.clone()
        ),
        svc_workflow::application::workflow_instance::assistance::create_and_bind_governance_child(
            &p,
            crate::common::admitting_gate(),
            command.clone()
        )
    );
    let b = x.unwrap();
    assert_eq!(b, y.unwrap());
    let rows:Vec<(Uuid,i64)>=sqlx::query_as("SELECT r.command_id,count(e.event_id) FROM workflow_command_receipts r LEFT JOIN workflow_events e USING(command_id) WHERE r.principal_id=$1 AND r.idempotency_key=$2 GROUP BY r.command_id").bind(f.owner).bind(&command.idempotency_key).fetch_all(&p).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM workflow_instances WHERE definition_version_id=$1"
        )
        .bind(v)
        .fetch_one(&p)
        .await
        .unwrap(),
        1
    );
    // Simulated lost HTTP response: authoritative linked case detail + exact-key
    // replay independently recover this committed child, binding and digest.
    let detail = svc_workflow::application::workflow_instance::assistance::get_assistance_case(
        &p,
        f.owner,
        case.assistance_case_id,
    )
    .await
    .unwrap();
    assert_eq!(detail.governance_binding.unwrap(), b);
}
#[tokio::test]
async fn composite_partial_and_extra_pair_races_fail_in_all_isolations() {
    let p = pool().await;
    let f = setup(&p).await;
    let (v, _) = version(&p, &f, false, false).await;
    for isolation in ["READ COMMITTED", "REPEATABLE READ", "SERIALIZABLE"] {
        let a = create(&p, &f, v, json!({})).await;
        let b = create(&p, &f, v, json!({})).await;
        let c = receipt(&p, f.owner, "CREATE_AND_BIND_GOVERNANCE_CHILD").await;
        let mut left = p.begin().await.unwrap();
        let mut right = p.begin().await.unwrap();
        for tx in [&mut left, &mut right] {
            sqlx::query(&format!("SET TRANSACTION ISOLATION LEVEL {isolation}"))
                .execute(&mut **tx)
                .await
                .unwrap();
            sqlx::query("SELECT count(*) FROM workflow_events WHERE command_id=$1")
                .bind(c)
                .fetch_one(&mut **tx)
                .await
                .unwrap();
        }
        let one = async {
            append(&mut left, c, a, "INSTANCE_CREATED", 0).await?;
            complete(&mut left, c, 201).await?;
            left.commit().await
        };
        let two = async {
            append(&mut right, c, b, "GOVERNANCE_CHILD_BOUND", 0).await?;
            complete(&mut right, c, 201).await?;
            right.commit().await
        };
        let (x, y) = tokio::join!(one, two);
        assert!(
            x.is_err() && y.is_err(),
            "{isolation} partial pair {x:?}/{y:?}"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM workflow_events WHERE command_id=$1"
            )
            .bind(c)
            .fetch_one(&p)
            .await
            .unwrap(),
            0
        );
        // A real service-produced valid pair remains exact under two concurrent
        // late append attempts, independently of the callers' old snapshots.
        let (_, valid, _, _) = pair(&p).await;
        let mut left = p.begin().await.unwrap();
        let mut right = p.begin().await.unwrap();
        for tx in [&mut left, &mut right] {
            sqlx::query(&format!("SET TRANSACTION ISOLATION LEVEL {isolation}"))
                .execute(&mut **tx)
                .await
                .unwrap();
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM workflow_events WHERE command_id=$1"
                )
                .bind(valid)
                .fetch_one(&mut **tx)
                .await
                .unwrap(),
                2
            );
        }
        let one = async {
            append(&mut left, valid, a, "INSTANCE_CREATED", 0).await?;
            left.commit().await
        };
        let two = async {
            append(&mut right, valid, b, "GOVERNANCE_CHILD_BOUND", 0).await?;
            right.commit().await
        };
        let (x, y) = tokio::join!(one, two);
        assert!(x.is_err() && y.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM workflow_events WHERE command_id=$1"
            )
            .bind(valid)
            .fetch_one(&p)
            .await
            .unwrap(),
            2
        );
        println!("{isolation} composite partial0/existingpair2 invariant verified");
    }
}
