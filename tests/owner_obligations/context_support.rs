use super::*;
use axum::Router;
pub struct Fixture {
    pub owner: Uuid,
    pub domain: Uuid,
    pub pm: Uuid,
    pub managed: Uuid,
    pub source: Uuid,
    pub supplier: Uuid,
    pub terminal: Uuid,
    pub entry: Uuid,
    pub gate1: Uuid,
    pub producer2: Uuid,
    pub gate2: Uuid,
    pub qa: Uuid,
    pub back1: Uuid,
    pub back2: Uuid,
}
fn acb() -> Value {
    json!({"type":"object","x-workflow-acceptance-binding-v1":{"contextCatalogPointer":"/acceptanceCriteria","submissionReportPointer":"/acceptanceReport"},"required":["acceptanceReport"],"properties":{"acceptanceReport":{"type":"object"}}})
}
pub fn report(id: Uuid) -> Value {
    json!({"acceptanceReport":{"contextRevisionId":id,"results":[{"id":"C1","verdict":"PASS"}]}})
}
pub fn reference(f: &Fixture, id: Uuid) -> Value {
    json!({"kind":"TERMINAL_SUBMISSION","workflowInstanceId":id,"definitionVersionId":f.supplier,"transitionId":f.terminal})
}
pub fn initial() -> Value {
    json!({"businessInput":{"immutable":1},"acceptanceCriteria":[{"id":"C1","description":"real receipt and QA"}]})
}
pub async fn fixture(p: &PgPool) -> Fixture {
    let (owner, domain) = seed(p).await;
    let pm = actor(p).await;
    let (supplier, terminal) = version(p, domain, owner, json!({"type":"object"})).await;
    sqlx::query(
        "UPDATE workflow_transition_definitions SET submission_schema=$2 WHERE transition_id=$1",
    )
    .bind(terminal)
    .bind(json!({"type":"object"}))
    .execute(p)
    .await
    .unwrap();
    publish(p, owner, supplier, None, "supplier").await.unwrap();
    let target_digest: String = sqlx::query_scalar(
        "SELECT definition_digest FROM workflow_definition_versions WHERE definition_version_id=$1",
    )
    .bind(supplier)
    .fetch_one(p)
    .await
    .unwrap();
    let (intake, it) = managed_version(p, domain, owner, "INTAKE").await;
    sqlx::query("UPDATE workflow_definition_versions SET context_schema=$2 WHERE definition_version_id=$1").bind(intake).bind(json!({"type":"object","additionalProperties":false,"x-workflow-owner-obligation-role-v1":"INTAKE","required":["businessInput","acceptanceCriteria"],"properties":{"businessInput":{"type":"object"},"acceptanceCriteria":{"type":"array"},"workflowPrerequisite":svc_workflow::domain::definition::prerequisite::property_schema()}})).execute(p).await.unwrap();
    let entry:Uuid=sqlx::query_scalar("SELECT node_id FROM workflow_node_definitions WHERE definition_version_id=$1 AND node_key='task'").bind(intake).fetch_one(p).await.unwrap();
    let qa:Uuid=sqlx::query_scalar("SELECT node_id FROM workflow_node_definitions WHERE definition_version_id=$1 AND node_key='qa'").bind(intake).fetch_one(p).await.unwrap();
    sqlx::query("UPDATE workflow_node_definitions SET order_index=CASE WHEN node_type='TERMINAL' THEN 5 ELSE 4 END WHERE definition_version_id=$1 AND node_key<>'task'").bind(intake).execute(p).await.unwrap();
    let gate1 = Uuid::new_v4();
    let producer2 = Uuid::new_v4();
    let gate2 = Uuid::new_v4();
    for (node, key, order) in [
        (gate1, "gate-one", 1),
        (producer2, "producer-two", 2),
        (gate2, "gate-two", 3),
    ] {
        sqlx::query("INSERT INTO workflow_node_definitions(node_id,definition_version_id,node_key,display_name,order_index,node_type,assignee_ref_type,fixed_principal_id) VALUES($1,$2,$3,$3,$4,'TASK','FIXED_PRINCIPAL',$5)").bind(node).bind(intake).bind(key).bind(order).bind(owner).execute(p).await.unwrap();
    }
    let descriptor =
        svc_workflow::domain::definition::owner_obligation::registry()[0]["descriptor"].clone();
    for (node, slot) in [(entry, "create-first"), (producer2, "create-second")] {
        sqlx::query("UPDATE workflow_node_definitions SET metadata=$2 WHERE node_id=$1").bind(node).bind(json!({"workflowOperation":{"version":1,"slotKey":slot,"allowedOperations":[descriptor],"requiredForAdvance":true,"targetDefinitionVersionId":supplier,"targetDefinitionDigest":target_digest}})).execute(p).await.unwrap();
    }
    for (node, producer, slot) in [
        (gate1, entry, "create-first"),
        (gate2, producer2, "create-second"),
    ] {
        sqlx::query("UPDATE workflow_node_definitions SET metadata=$2 WHERE node_id=$1").bind(node).bind(json!({"workflowPrerequisiteGate":{"version":1,"kind":"TERMINAL_SUBMISSION","producer":{"nodeId":producer,"slotKey":slot}}})).execute(p).await.unwrap();
    }
    let first_t: Uuid = sqlx::query_scalar(
        "SELECT primary_advance_transition_id FROM workflow_node_definitions WHERE node_id=$1",
    )
    .bind(entry)
    .fetch_one(p)
    .await
    .unwrap();
    sqlx::query("UPDATE workflow_transition_definitions SET target_node_id=$2,submission_schema=$3 WHERE transition_id=$1").bind(first_t).bind(gate1).bind(acb()).execute(p).await.unwrap();
    for (a, b, key) in [
        (gate1, producer2, "after-first"),
        (producer2, gate2, "enter-second"),
        (gate2, qa, "after-second"),
    ] {
        let t = Uuid::new_v4();
        sqlx::query("INSERT INTO workflow_transition_definitions(transition_id,definition_version_id,transition_key,display_name,source_node_id,target_node_id,transition_effect,submission_schema) VALUES($1,$2,$3,$3,$4,$5,'ADVANCE',$6)").bind(t).bind(intake).bind(key).bind(a).bind(b).bind(acb()).execute(p).await.unwrap();
        sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id=$2 WHERE node_id=$1").bind(a).bind(t).execute(p).await.unwrap();
    }
    let back1 = Uuid::new_v4();
    let back2 = Uuid::new_v4();
    for (t, a, b, key) in [
        (back1, gate2, gate1, "back-first"),
        (back2, qa, gate2, "back-second"),
    ] {
        sqlx::query("INSERT INTO workflow_transition_definitions(transition_id,definition_version_id,transition_key,display_name,source_node_id,target_node_id,transition_effect,submission_schema) VALUES($1,$2,$3,$3,$4,$5,'RETURN',$6)").bind(t).bind(intake).bind(key).bind(a).bind(b).bind(json!({"type":"object"})).execute(p).await.unwrap();
    }
    publish(p, owner, intake, None, "intake").await.unwrap();
    let (review, rt) = managed_version(p, domain, owner, "REVIEW").await;
    publish(p, owner, review, None, "review").await.unwrap();
    let(source,_)=version(p,domain,owner,json!({"type":"object","required":["ownerIntakeContext","ownerReviewContext"],"properties":{"ownerIntakeContext":{"type":"object"},"ownerReviewContext":{"type":"object"}},"x-workflow-owner-obligation-v1":{"version":1,"intakeDefinitionVersionId":intake,"intakeAcceptanceTransitionId":it,"reviewDefinitionVersionId":review,"reviewCompletionTransitionId":rt}})).await;
    publish(p, owner, source, Some(pm), "source").await.unwrap();
    let s = create(
        p,
        owner,
        domain,
        source,
        json!({"ownerIntakeContext":initial(),"ownerReviewContext":{}}),
    )
    .await
    .unwrap();
    request(p, owner, s.workflow_instance_id, s.current_node_visit_id).await;
    svc_workflow::store::postgres::workflow_instance_repository::owner_obligation::reconcile::reconcile_pending(p,pm,svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled()).await.unwrap();
    let managed = sqlx::query_scalar(
        "SELECT workflow_instance_id FROM workflow_owner_managed_instances WHERE kind='INTAKE'",
    )
    .fetch_one(p)
    .await
    .unwrap();
    Fixture {
        owner,
        domain,
        pm,
        managed,
        source: s.workflow_instance_id,
        supplier,
        terminal,
        entry,
        gate1,
        producer2,
        gate2,
        qa,
        back1,
        back2,
    }
}
pub async fn current(p: &PgPool, id: Uuid) -> (i32, Uuid, Uuid, Uuid) {
    sqlx::query_as("SELECT w.workflow_state_version,w.current_context_revision_id,w.current_node_visit_id,v.node_id FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id WHERE w.workflow_instance_id=$1").bind(id).fetch_one(p).await.unwrap()
}
pub async fn effect(p: &PgPool, f: &Fixture) -> Value {
    use svc_workflow::store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::{effects, reads},
    };
    let m = reads::managed(p, f.owner, f.managed, Some(f.pm))
        .await
        .unwrap();
    let version: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(f.source)
    .fetch_one(p)
    .await
    .unwrap();
    effects::execute(p,f.owner,Some(f.pm),AdmissionGate::disabled(),f.managed,serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap(),m["currentNode"]["operation"]["slotKey"].as_str().unwrap(),effects::Execute{expected_workflow_state_version:current(p,f.managed).await.0,expected_source_workflow_state_version:version,operation:serde_json::from_value(m["currentNode"]["operation"]["allowedOperations"][0].clone()).unwrap(),arguments:json!({"domainId":f.domain,"definitionVersionId":f.supplier,"contextPayload":{},"metadata":null})}).await.unwrap()
}
pub async fn post(
    app: Router,
    jwk: &common::MockJwksServer,
    actor: Uuid,
    id: Uuid,
    key: &str,
    body: Value,
) -> (u16, Value) {
    http_support::http(
        app,
        actor,
        jwk,
        "POST",
        &format!("/internal/v1/workflow-instances/{id}/transitions"),
        Some(key),
        body,
    )
    .await
}
pub async fn primary(p: &PgPool, node: Uuid) -> Uuid {
    sqlx::query_scalar(
        "SELECT primary_advance_transition_id FROM workflow_node_definitions WHERE node_id=$1",
    )
    .bind(node)
    .fetch_one(p)
    .await
    .unwrap()
}
