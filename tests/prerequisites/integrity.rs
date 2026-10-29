use super::integrity_support::*;
use super::*;
#[tokio::test]
async fn receipt_completion_and_event_mutation_independently_enforce_matrix() {
    let p = pool().await;
    let f = setup(&p).await;
    // Completion surface alone rejects a 2xx receipt with zero Events.
    let c = receipt(&p, f.owner, "CREATE_AND_BIND_GOVERNANCE_CHILD").await;
    let mut tx = p.begin().await.unwrap();
    complete(&mut tx, c, 201).await.unwrap();
    assert!(tx.commit().await.is_err());
    // Real deterministic failure remains durable and replays, zero facts.
    let mut tx = p.begin().await.unwrap();
    complete(&mut tx, c, 422).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM workflow_events WHERE command_id=$1")
            .bind(c)
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
    // Event surface only: a deliberately corrupt pre-existing 2xx/zero-event
    // receipt is seeded in this test DB without queuing the receipt trigger.
    let event_only = receipt(&p, f.owner, "CREATE_AND_BIND_GOVERNANCE_CHILD").await;
    let mut tx = p.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    complete(&mut tx, event_only, 201).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = p.begin().await.unwrap();
    append(&mut tx, event_only, f.instance, "INSTANCE_CREATED", 1)
        .await
        .unwrap();
    let e = tx.commit().await.unwrap_err();
    assert!(e.to_string().contains("exactly two events"), "{e}");
    // Wrong kind is rejected before a pair, and caller slots cannot create room.
    let wrong = receipt(&p, f.owner, "CREATE_AND_BIND_GOVERNANCE_CHILD").await;
    let mut tx = p.begin().await.unwrap();
    assert!(append(&mut tx, wrong, f.instance, "OTHER", 2)
        .await
        .is_err());
    tx.rollback().await.unwrap();
}
#[tokio::test]
async fn completed_pair_negative_shapes_rollback_without_touching_history() {
    let p = pool().await;
    let (f, c, _, b) = pair(&p).await;
    let original:Value=sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(e) ORDER BY event_id) FROM workflow_events e WHERE command_id=$1").bind(c).fetch_one(&p).await.unwrap();
    for variant in [
        "zero",
        "one",
        "non2xx",
        "wrong_type",
        "wrong_response",
        "foreign_domain",
    ] {
        let mut tx = p.begin().await.unwrap();
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE workflow_command_receipts SET receipt_status='PROCESSING' WHERE command_id=$1",
        )
        .bind(c)
        .execute(&mut *tx)
        .await
        .unwrap();
        match variant {
            "zero" => {
                sqlx::query("DELETE FROM workflow_events WHERE command_id=$1")
                    .bind(c)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
            }
            "one" => {
                sqlx::query("DELETE FROM workflow_events WHERE command_id=$1 AND event_type='INSTANCE_CREATED'").bind(c).execute(&mut *tx).await.unwrap();
            }
            "wrong_type" => {
                sqlx::query("UPDATE workflow_events SET event_type='OTHER' WHERE command_id=$1 AND event_type='GOVERNANCE_CHILD_BOUND'").bind(c).execute(&mut *tx).await.unwrap();
            }
            "wrong_response" => {
                sqlx::query("UPDATE workflow_command_receipts SET response_body='{}'::jsonb WHERE command_id=$1").bind(c).execute(&mut *tx).await.unwrap();
            }
            "foreign_domain" => {
                let other = setup(&p).await;
                sqlx::query(
                    "UPDATE workflow_instances SET domain_id=$2 WHERE workflow_instance_id=$1",
                )
                .bind(f.instance)
                .bind(other.domain)
                .execute(&mut *tx)
                .await
                .unwrap();
            }
            _ => {}
        }
        sqlx::query("SET LOCAL session_replication_role=origin")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE workflow_command_receipts SET receipt_status='COMPLETED',response_status=$2 WHERE command_id=$1").bind(c).bind(if variant=="non2xx"{422}else{201}).execute(&mut *tx).await.unwrap();
        assert!(tx.commit().await.is_err(), "must reject {variant}");
        let after:Value=sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(e) ORDER BY event_id) FROM workflow_events e WHERE command_id=$1").bind(c).fetch_one(&p).await.unwrap();
        assert_eq!(after, original, "rollback preserves {variant}");
    }
    let extra = create(
        &p,
        &f,
        sqlx::query_scalar(
            "SELECT definition_version_id FROM workflow_instances WHERE workflow_instance_id=$1",
        )
        .bind(f.instance)
        .fetch_one(&p)
        .await
        .unwrap(),
        json!({}),
    )
    .await;
    let mut tx = p.begin().await.unwrap();
    let e = append(&mut tx, c, extra, "GOVERNANCE_CHILD_BOUND", 0)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("idx_wf_event_command_slot"));
    tx.rollback().await.unwrap();
    let mut tx = p.begin().await.unwrap();
    assert!(
        sqlx::query("UPDATE workflow_events SET command_event_slot=0 WHERE command_id=$1")
            .bind(c)
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    let rows:Vec<(String,i16)>=sqlx::query_as("SELECT event_type,command_event_slot FROM workflow_events WHERE command_id=$1 ORDER BY event_type").bind(c).fetch_all(&p).await.unwrap();
    assert_eq!(
        rows,
        vec![
            ("GOVERNANCE_CHILD_BOUND".into(), 2),
            ("INSTANCE_CREATED".into(), 1)
        ]
    );
    assert_eq!(b["status"], "BOUND");
}
#[tokio::test]
async fn completed_composite_null_status_is_not_a_success_or_rejection() {
    let p = pool().await;
    let (_, c, _, _) = pair(&p).await;
    let mut tx = p.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE workflow_command_receipts SET response_status=NULL WHERE command_id=$1")
        .bind(c)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        sqlx::query("SELECT assert_command_event_integrity($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .is_err(),
        "NULL status must fail closed rather than SQL three-valued logic accepting the pair"
    );
    tx.rollback().await.unwrap();
}
