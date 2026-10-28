//! CTR-GCRS: real service transactions on a fresh SQLx database per test.
#![allow(dead_code, unused_imports, clippy::too_many_arguments)]
#[path = "25_workflow_assistance/helpers.rs"]
mod helpers;
use helpers::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use svc_workflow::{
    application::workflow_instance::{
        assistance::{bind_governance_child, get_assistance_case, resolve_assistance},
        create::create_workflow_instance,
        execute_transition::execute_workflow_transition,
    },
    domain::{
        ids::*,
        workflow_instance::{
            assistance::{AssistancePayload, BindGovernanceChildCommand, ResolveAssistanceCommand},
            commands::CreateWorkflowInstanceCommand,
        },
    },
    store::postgres::admission_gate::AdmissionGate,
};
use uuid::Uuid;

fn schema() -> Value {
    json!({"type":"object","required":["governanceAcceptance"],"properties":{"governanceAcceptance":{"type":"object","additionalProperties":false,"required":["bindingId","acceptanceContractDigest","sourceRequestPayloadDigest","verdict"],"properties":{"bindingId":{"type":"string","format":"uuid"},"acceptanceContractDigest":{"type":"string","pattern":"^[0-9a-f]{64}$"},"sourceRequestPayloadDigest":{"type":"string","pattern":"^[0-9a-f]{64}$"},"verdict":{"type":"string","enum":["ACCEPTED","NOT_ACCEPTED"]}}}}})
}
async fn child(pool: &PgPool, f: &Fixture) -> Fixture {
    let version: Uuid = sqlx::query_scalar(
        "SELECT definition_version_id FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(f.instance)
    .fetch_one(pool)
    .await
    .unwrap();
    let c = create_workflow_instance(
        pool,
        AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(f.agent),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".into(),
            execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
            domain_id: DomainId::from_uuid(f.domain),
            definition_version_id: DefinitionVersionId::from_uuid(version),
            external_reference: None,
            external_url: None,
            metadata: json!({}),
            context_payload: json!({}),
        },
    )
    .await
    .unwrap();
    Fixture {
        instance: c.workflow_instance_id,
        visit: c.current_node_visit_id,
        ..*f
    }
}
fn bind_cmd(f: &Fixture, case: Uuid, child: Uuid, expected: i32) -> BindGovernanceChildCommand {
    BindGovernanceChildCommand {
        principal_id: PrincipalId::from_uuid(f.owner),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".into(),
        assistance_case_id: AssistanceCaseId::from_uuid(case),
        governance_child_instance_id: WorkflowInstanceId::from_uuid(child),
        acceptance_transition_id: TransitionId::from_uuid(f.transition),
        expected_workflow_state_version: expected,
    }
}
fn envelope(b: &Value, verdict: &str) -> Value {
    json!({"governanceAcceptance":{"bindingId":b["bindingId"],"acceptanceContractDigest":b["acceptanceContractDigest"],"sourceRequestPayloadDigest":b["source"]["requestPayloadDigest"],"verdict":verdict}})
}
async fn finish(pool: &PgPool, f: &Fixture, b: &Value, verdict: &str) {
    let mut c = transition_command(f, 1);
    c.submission_payload = Some(envelope(b, verdict));
    execute_workflow_transition(pool, AdmissionGate::disabled(), c)
        .await
        .unwrap();
}
fn resolve_cmd(f: &Fixture, case: Uuid, proof: Value) -> ResolveAssistanceCommand {
    ResolveAssistanceCommand {
        principal_id: PrincipalId::from_uuid(f.owner),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".into(),
        assistance_case_id: AssistanceCaseId::from_uuid(case),
        expected_workflow_state_version: 3,
        resolution: AssistancePayload {
            message: "Reviewed exact terminal submission".into(),
            supporting_payload: Some(json!({"governanceChildReview":proof})),
        },
    }
}
async fn review_rows(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar("SELECT payload FROM workflow_outbox WHERE payload->>'phase'='GOVERNANCE_CHILD_REVIEW' ORDER BY created_at,outbox_id").fetch_all(pool).await.unwrap()
}

#[path = "governance_child/cases.rs"]
mod cases;
use cases::replace;
#[path = "common/mod.rs"]
mod common;
#[path = "governance_child/delivery.rs"]
mod delivery;
#[path = "governance_child/http.rs"]
mod http_tests;
#[path = "governance_child/negative_and_races.rs"]
mod negative_and_races;
