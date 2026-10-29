use super::*;

#[tokio::test]
async fn stops_disabled_policy_actor_and_plain_owner_action_cannot_bypass() {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    use svc_workflow::{
        domain::{ids::*, workflow_instance::assistance::*},
        store::postgres::{
            admission_gate::AdmissionGate,
            workflow_instance_repository::owner_obligation::{effects, reads},
        },
    };
    let case: Uuid = sqlx::query_scalar(
        "SELECT assistance_case_id FROM workflow_assistance_cases WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let sv: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let denied =
        svc_workflow::application::workflow_instance::assistance::escalate_assistance_to_human(
            &p,
            EscalateAssistanceCommand {
                principal_id: PrincipalId::from_uuid(owner),
                idempotency_key: "bypass".into(),
                command_schema_version: "v1".into(),
                assistance_case_id: AssistanceCaseId::from_uuid(case),
                expected_workflow_state_version: sv,
                escalation: AssistancePayload {
                    message: "plain route bypass".into(),
                    supporting_payload: None,
                },
            },
        )
        .await
        .unwrap_err();
    assert_eq!(denied, AssistanceError::GovernedOperationRequired);
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    sqlx::query("UPDATE principals SET enabled=FALSE WHERE principal_id=$1")
        .bind(pm)
        .execute(&p)
        .await
        .unwrap();
    let b = effects::Execute {
        expected_workflow_state_version: 1,
        expected_source_workflow_state_version: sv,
        operation: serde_json::from_value(
            m["currentNode"]["operation"]["allowedOperations"][0].clone(),
        )
        .unwrap(),
        arguments: json!({"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null}),
    };
    assert_eq!(
        effects::execute(
            &p,
            owner,
            Some(pm),
            AdmissionGate::disabled(),
            managed,
            visit,
            "create-dependency",
            b
        )
        .await
        .unwrap_err()
        .code,
        "SYSTEM_POLICY_ACTOR_UNAVAILABLE"
    );
    assert_eq!(
        reads::managed(&p, owner, managed, Some(pm)).await.unwrap()["state"],
        "STOPPED"
    );
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_operation_effects")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(n, 0);
}
#[tokio::test]
async fn stops_owner_replacement_before_birth_is_absorbing_across_aba_and_replay() {
    let p = pool().await;
    let (owner, domain, pm, source, _, _) = policy_fixture(&p).await;
    publish(&p, owner, source, Some(pm), "source")
        .await
        .unwrap();
    let s = create(
        &p,
        owner,
        domain,
        source,
        json!({"ownerIntakeContext":{},"ownerReviewContext":{}}),
    )
    .await
    .unwrap();
    request(&p, owner, s.workflow_instance_id, s.current_node_visit_id).await;
    let coordinator = actor(&p).await;
    let other = actor(&p).await;
    sqlx::query("INSERT INTO global_role_bindings(binding_id,principal_id,role_key) VALUES($1,$2,'GLOBAL_WORKFLOW_COORDINATOR')").bind(Uuid::new_v4()).bind(coordinator).execute(&p).await.unwrap();
    use svc_workflow::domain::{ids::*, provisioning::ReplaceOwnerCommand};
    for (new, key) in [(other, "owner-b"), (owner, "owner-a"), (owner, "owner-a")] {
        svc_workflow::application::provisioning::replace_owner(
            &p,
            &ReplaceOwnerCommand {
                domain_id: DomainId::from_uuid(domain),
                new_owner_id: PrincipalId::from_uuid(new),
            },
            key,
            "test",
            &PrincipalId::from_uuid(coordinator),
        )
        .await
        .unwrap();
    }
    let before:(String,Option<Uuid>,Option<Uuid>)=sqlx::query_as("SELECT state,managed_instance_id,owner_replacement_command_id FROM workflow_owner_obligation_intents WHERE kind='INTAKE'").fetch_one(&p).await.unwrap();
    assert_eq!(before.0, "STOPPED");
    assert!(before.1.is_none());
    assert!(before.2.is_some());
    svc_workflow::store::postgres::workflow_instance_repository::owner_obligation::reconcile::reconcile_pending(&p,pm,svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled()).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_owner_managed_instances")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(n, 0);
    let after:(String,Option<Uuid>,Option<Uuid>)=sqlx::query_as("SELECT state,managed_instance_id,owner_replacement_command_id FROM workflow_owner_obligation_intents WHERE kind='INTAKE'").fetch_one(&p).await.unwrap();
    assert_eq!(before, after);
}
#[tokio::test]
async fn stops_parent_cancel_retires_and_separate_receipted_cleanup_preserves_leaf() {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    use svc_workflow::{
        domain::{ids::*, workflow_instance::commands::CancelWorkflowInstanceCommand},
        store::postgres::{
            admission_gate::AdmissionGate,
            workflow_instance_repository::owner_obligation::{effects, reads, reconcile},
        },
    };
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    let sv: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let b = effects::Execute {
        expected_workflow_state_version: 1,
        expected_source_workflow_state_version: sv,
        operation: serde_json::from_value(
            m["currentNode"]["operation"]["allowedOperations"][0].clone(),
        )
        .unwrap(),
        arguments: json!({"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null}),
    };
    let result = effects::execute(
        &p,
        owner,
        Some(pm),
        AdmissionGate::disabled(),
        managed,
        visit,
        "create-dependency",
        b,
    )
    .await
    .unwrap();
    let leaf: Uuid =
        serde_json::from_value(result["result"]["workflowInstanceId"].clone()).unwrap();
    let cmd = CancelWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(owner),
        idempotency_key: "parent-cancel".into(),
        command_schema_version: "v1".into(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(source),
        expected_workflow_state_version: sv,
        reason: "Parent business cancelled".into(),
    };
    svc_workflow::application::workflow_instance::cancel::cancel_workflow_instance(
        &p,
        cmd,
        &"a".repeat(64),
    )
    .await
    .unwrap();
    reconcile::reconcile_pending(&p, pm, AdmissionGate::disabled())
        .await
        .unwrap();
    reconcile::reconcile_pending(&p, pm, AdmissionGate::disabled())
        .await
        .unwrap();
    let flags:Vec<(Uuid,bool)>=sqlx::query_as("SELECT workflow_instance_id,cancelled FROM workflow_instances WHERE workflow_instance_id=ANY($1) ORDER BY workflow_instance_id").bind(vec![managed,leaf]).fetch_all(&p).await.unwrap();
    assert!(flags.contains(&(managed, true)));
    assert!(flags.contains(&(leaf, false)));
    let rows:Vec<(String,i64)>=sqlx::query_as("SELECT r.command_type,count(e.event_id) FROM workflow_command_receipts r JOIN workflow_events e USING(command_id) WHERE r.idempotency_key LIKE 'owner-cleanup:%' GROUP BY r.command_id").fetch_all(&p).await.unwrap();
    assert_eq!(rows, vec![("CANCEL_WORKFLOW_INSTANCE".into(), 1)]);
    assert_eq!(
        reads::managed(&p, owner, managed, Some(pm)).await.unwrap()["state"],
        "RETIRED"
    );
}
#[tokio::test]
async fn stops_real_attempt_cap_records_actual_attempt_and_no_naked_wake() {
    let p = pool().await;
    let (owner, _, pm, managed, _, _, _) = operation_fixture(&p).await;
    use svc_workflow::{
        application::workflow_instance::execution_escalation::{
            compute_escalation_request_hash, create_system_execution_escalation,
        },
        store::postgres::workflow_instance_repository::{
            execution_escalation::SystemEscalationCommand, owner_obligation::reads,
        },
    };
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    let hash = compute_escalation_request_hash(
        &managed,
        &visit,
        "ATTEMPTS_EXHAUSTED",
        Some(3),
        Some("actual-isolated-attempt-3"),
        None,
    )
    .unwrap();
    let command = SystemEscalationCommand {
        principal_id: pm,
        idempotency_key: "cap".into(),
        request_hash: hash,
        workflow_instance_id: managed,
        node_visit_id: visit,
        reason: "ATTEMPTS_EXHAUSTED".into(),
        attempt_count: Some(3),
        last_attempt_id: Some("actual-isolated-attempt-3".into()),
        dispatch_intent_id: None,
    };
    create_system_execution_escalation(&p, command.clone())
        .await
        .unwrap();
    create_system_execution_escalation(&p, command)
        .await
        .unwrap();
    let row:(String,String,Option<String>,Uuid)=sqlx::query_as("SELECT state,stop_code,last_attempt_id,last_command_id FROM workflow_owner_obligation_intents WHERE kind='STOP'").fetch_one(&p).await.unwrap();
    assert_eq!(
        (row.0, row.1, row.2),
        (
            "STOPPED".into(),
            "ATTEMPT_LIMIT_REACHED".into(),
            Some("actual-isolated-attempt-3".into())
        )
    );
    let kind: String = sqlx::query_scalar(
        "SELECT command_type FROM workflow_command_receipts WHERE command_id=$1",
    )
    .bind(row.3)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(kind, "SYSTEM_EXECUTION_ESCALATION");
    assert_eq!(
        reads::managed(&p, owner, managed, Some(pm)).await.unwrap()["state"],
        "STOPPED"
    );
    let wakes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_outbox WHERE outbox_kind='OWNER_ASSISTANCE_WAKE'",
    )
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(wakes, 0);
}
#[tokio::test]
async fn stops_owner_replacement_races_actual_effect_without_duplicate_and_keeps_current_owner_readback(
) {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    use svc_workflow::{
        domain::{ids::*, provisioning::ReplaceOwnerCommand},
        store::postgres::{
            admission_gate::AdmissionGate,
            workflow_instance_repository::owner_obligation::{effects, reads},
        },
    };
    let coordinator = actor(&p).await;
    let next = actor(&p).await;
    sqlx::query("INSERT INTO global_role_bindings(binding_id,principal_id,role_key) VALUES($1,$2,'GLOBAL_WORKFLOW_COORDINATOR')").bind(Uuid::new_v4()).bind(coordinator).execute(&p).await.unwrap();
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    let sv: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let b = effects::Execute {
        expected_workflow_state_version: 1,
        expected_source_workflow_state_version: sv,
        operation: serde_json::from_value(
            m["currentNode"]["operation"]["allowedOperations"][0].clone(),
        )
        .unwrap(),
        arguments: json!({"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null}),
    };
    let replacement = ReplaceOwnerCommand {
        domain_id: DomainId::from_uuid(domain),
        new_owner_id: PrincipalId::from_uuid(next),
    };
    let caller = PrincipalId::from_uuid(coordinator);
    let (effect, changed) = tokio::join!(
        effects::execute(
            &p,
            owner,
            Some(pm),
            AdmissionGate::disabled(),
            managed,
            visit,
            "create-dependency",
            b.clone()
        ),
        svc_workflow::application::provisioning::replace_owner(
            &p,
            &replacement,
            "replace-race",
            "test",
            &caller
        )
    );
    assert!(changed.is_ok(), "{changed:?}");
    if let Err(e) = &effect {
        assert_eq!(e.code, "not_domain_owner");
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_owner_dependency_origins")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(n, if effect.is_ok() { 1 } else { 0 });
    let current = reads::managed(&p, next, managed, Some(pm)).await.unwrap();
    assert_eq!(current["state"], "STOPPED");
    assert_eq!(current["stop"]["code"], "OWNER_CHANGED_UNSUPPORTED");
    let denied = effects::execute(
        &p,
        owner,
        Some(pm),
        AdmissionGate::disabled(),
        managed,
        visit,
        "create-dependency",
        b,
    )
    .await
    .unwrap_err();
    assert_eq!(denied.code, "not_domain_owner");
    sqlx::query("INSERT INTO global_role_bindings(binding_id,principal_id,role_key) VALUES($1,$2,'GLOBAL_SCHEDULER_READ')").bind(Uuid::new_v4()).bind(pm).execute(&p).await.unwrap();
    let ready =
        svc_workflow::store::postgres::workflow_instance_repository::query_ready_intents::list(
            &p, pm, 100, None,
        )
        .await
        .unwrap();
    assert!(!ready.iter().any(|r| r.due.workflow_instance_id == managed));
}
#[tokio::test]
async fn stops_dependency_leaf_has_no_recursive_effect_and_nested_cap_uses_actual_leaf_visit() {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    use svc_workflow::{
        application::workflow_instance::execution_escalation::{
            compute_escalation_request_hash, create_system_execution_escalation,
        },
        store::postgres::{
            admission_gate::AdmissionGate,
            workflow_instance_repository::{
                execution_escalation::SystemEscalationCommand,
                owner_obligation::{effects, reads},
            },
        },
    };
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    let sv: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let b = effects::Execute {
        expected_workflow_state_version: 1,
        expected_source_workflow_state_version: sv,
        operation: serde_json::from_value(
            m["currentNode"]["operation"]["allowedOperations"][0].clone(),
        )
        .unwrap(),
        arguments: json!({"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null}),
    };
    let result = effects::execute(
        &p,
        owner,
        Some(pm),
        AdmissionGate::disabled(),
        managed,
        visit,
        "create-dependency",
        b.clone(),
    )
    .await
    .unwrap();
    let leaf: Uuid =
        serde_json::from_value(result["result"]["workflowInstanceId"].clone()).unwrap();
    let leaf_visit: Uuid =
        serde_json::from_value(result["result"]["currentNodeVisitId"].clone()).unwrap();
    assert_eq!(
        reads::managed(&p, owner, leaf, Some(pm)).await.unwrap()["managedKind"],
        "DEPENDENCY"
    );
    assert_eq!(
        effects::execute(
            &p,
            owner,
            Some(pm),
            AdmissionGate::disabled(),
            leaf,
            leaf_visit,
            "create-dependency",
            b
        )
        .await
        .unwrap_err()
        .code,
        "GOVERNED_OPERATION_REQUIRED"
    );
    let hash = compute_escalation_request_hash(
        &leaf,
        &leaf_visit,
        "ATTEMPTS_EXHAUSTED",
        Some(3),
        Some("leaf-attempt-3"),
        None,
    )
    .unwrap();
    create_system_execution_escalation(
        &p,
        SystemEscalationCommand {
            principal_id: pm,
            idempotency_key: "leaf-cap".into(),
            request_hash: hash,
            workflow_instance_id: leaf,
            node_visit_id: leaf_visit,
            reason: "ATTEMPTS_EXHAUSTED".into(),
            attempt_count: Some(3),
            last_attempt_id: Some("leaf-attempt-3".into()),
            dispatch_intent_id: None,
        },
    )
    .await
    .unwrap();
    let row:(Uuid,Uuid,String)=sqlx::query_as("SELECT managed_instance_id,node_visit_id,stop_code FROM workflow_owner_obligation_intents WHERE kind='STOP'").fetch_one(&p).await.unwrap();
    assert_eq!(
        row,
        (managed, leaf_visit, "NESTED_ASSISTANCE_UNSUPPORTED".into())
    );
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM workflow_owner_managed_instances),(SELECT count(*) FROM workflow_operation_effects),(SELECT count(*) FROM workflow_outbox WHERE outbox_kind='OWNER_ASSISTANCE_WAKE')").fetch_one(&p).await.unwrap();
    assert_eq!(counts, (1, 1, 0));
}
