#![allow(dead_code, unused_imports)]
#[path = "common/mod.rs"]
mod common;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
#[path = "owner_obligations/support.rs"]
mod support;
use support::*;
#[path = "owner_obligations/http_support.rs"]
mod http_support;
async fn call(
    app: &axum::Router,
    jwk: &common::MockJwksServer,
    owner: Uuid,
    method: &str,
    path: &str,
    body: Value,
) -> (u16, Value) {
    http_support::http(
        app.clone(),
        owner,
        jwk,
        method,
        path,
        Some(&Uuid::new_v4().to_string()),
        body,
    )
    .await
}
async fn draft(
    app: &axum::Router,
    jwk: &common::MockJwksServer,
    owner: Uuid,
    domain: Uuid,
    schema: Value,
) -> (Uuid, Uuid) {
    let (status, d) = call(
        app,
        jwk,
        owner,
        "POST",
        &format!("/internal/v1/domains/{domain}/definitions"),
        json!({"definitionKey":Uuid::new_v4().to_string(),"displayName":"Synthetic API authoring"}),
    )
    .await;
    assert_eq!(status, 200, "{d}");
    let definition = serde_json::from_value(d["workflowDefinitionId"].clone()).unwrap();
    let (status, v) = call(
        app,
        jwk,
        owner,
        "POST",
        &format!("/internal/v1/domains/{domain}/definitions/{definition}/versions"),
        json!({"semanticModelVersion":3,"contextSchema":schema}),
    )
    .await;
    assert_eq!(status, 200, "{v}");
    (
        definition,
        serde_json::from_value(v["definitionVersionId"].clone()).unwrap(),
    )
}
fn node(key: &str, order: i32, principal: Option<Uuid>, next: Option<&str>) -> Value {
    json!({"node_key":key,"display_name":key,"order_index":order,"node_type":if principal.is_some(){"TASK"}else{"TERMINAL"},"assignee_ref_type":principal.map(|_|"FIXED_PRINCIPAL"),"fixed_principal_id":principal,"primary_advance_transition_key":next})
}
fn edge(key: &str, a: &str, b: &str, schema: Option<Value>) -> Value {
    json!({"transition_key":key,"display_name":key,"source_node_key":a,"target_node_key":b,"transition_effect":"ADVANCE","submission_schema":schema})
}
async fn read(
    app: &axum::Router,
    jwk: &common::MockJwksServer,
    owner: Uuid,
    domain: Uuid,
    d: Uuid,
    v: Uuid,
) -> Value {
    let (status, r) = call(
        app,
        jwk,
        owner,
        "GET",
        &format!("/internal/v1/domains/{domain}/definitions/{d}/versions/{v}"),
        json!({}),
    )
    .await;
    assert_eq!(status, 200, "{r}");
    r
}
fn id(graph: &Value, key: &str) -> Uuid {
    serde_json::from_value(
        graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["node_key"] == key)
            .unwrap()["node_id"]
            .clone(),
    )
    .unwrap()
}

struct Fixture {
    pool: PgPool,
    app: axum::Router,
    jwk: common::MockJwksServer,
    owner: Uuid,
    domain: Uuid,
    definition: Uuid,
    version: Uuid,
    body: Value,
}
impl Fixture {
    async fn new() -> Self {
        let pool = pool().await;
        let (owner, domain) = seed(&pool).await;
        let pm = actor(&pool).await;
        let qa = actor(&pool).await;
        let jwk = common::MockJwksServer::start().await;
        let app = http_support::app(&pool, &jwk, pm).await;
        let (sd, sv) = draft(&app, &jwk, owner, domain, json!({"type":"object"})).await;
        let sb = json!({"definitionVersionId":sv,"nodes":[node("work",0,Some(owner),Some("finish")),node("done",1,None,None)],"transitions":[edge("finish","work","done",Some(json!({"type":"object"})))]});
        let (s, r) = call(
            &app,
            &jwk,
            owner,
            "PUT",
            &format!("/internal/v1/domains/{domain}/definitions/{sd}/draft"),
            sb,
        )
        .await;
        assert_eq!(s, 200, "{r}");
        let (s, r) = call(
            &app,
            &jwk,
            owner,
            "POST",
            &format!("/internal/v1/domains/{domain}/definitions/{sd}/publish"),
            json!({"versionId":sv}),
        )
        .await;
        assert_eq!(s, 200, "{r}");
        let supplier = read(&app, &jwk, owner, domain, sd, sv).await;
        let digest = supplier["version"]["definition_digest"].clone();
        let schema = json!({"type":"object","x-workflow-owner-obligation-role-v1":"INTAKE","properties":{"workflowPrerequisite":svc_workflow::domain::definition::prerequisite::property_schema()}});
        let (definition, version) = draft(&app, &jwk, owner, domain, schema).await;
        let mut produce = node("produce", 0, Some(owner), Some("to-qa"));
        produce["metadata"] = json!({"workflowOperation":{"version":1,"slotKey":"create-leaf","allowedOperations":[svc_workflow::domain::definition::owner_obligation::registry()[0]["descriptor"]],"requiredForAdvance":true,"targetDefinitionVersionId":sv,"targetDefinitionDigest":digest}});
        let mut qa = node("qa", 1, Some(qa), Some("finish"));
        qa["metadata"] = json!({"opaque":{"producer":{"nodeKey":"unchanged","slotKey":null},"array":[null,1]},"workflowPrerequisiteGate":{"version":1,"kind":"TERMINAL_SUBMISSION","producer":{"nodeKey":"produce","slotKey":"create-leaf"}},"workflowOperation":{"version":1,"slotKey":"create-second","allowedOperations":[svc_workflow::domain::definition::owner_obligation::registry()[0]["descriptor"]],"requiredForAdvance":true,"targetDefinitionVersionId":sv,"targetDefinitionDigest":digest,"prerequisiteProducer":{"nodeKey":"produce","slotKey":"create-leaf"}}});
        let body = json!({"definitionVersionId":version,"nodes":[produce,qa,node("done",2,None,None)],"transitions":[edge("to-qa","produce","qa",None),edge("finish","qa","done",Some(acceptance_schema()))]});
        Self {
            pool,
            app,
            jwk,
            owner,
            domain,
            definition,
            version,
            body,
        }
    }
    fn put_path(&self) -> String {
        format!(
            "/internal/v1/domains/{}/definitions/{}/draft",
            self.domain, self.definition
        )
    }
    async fn put_key(&self, body: Value, key: &str) -> (u16, Value) {
        http_support::http(
            self.app.clone(),
            self.owner,
            &self.jwk,
            "PUT",
            &self.put_path(),
            Some(key),
            body,
        )
        .await
    }
    async fn put(&self, body: Value) -> (u16, Value) {
        self.put_key(body, &Uuid::new_v4().to_string()).await
    }
    async fn get(&self) -> Value {
        read(
            &self.app,
            &self.jwk,
            self.owner,
            self.domain,
            self.definition,
            self.version,
        )
        .await
    }
    async fn publish(&self) -> (u16, Value) {
        call(
            &self.app,
            &self.jwk,
            self.owner,
            "POST",
            &format!(
                "/internal/v1/domains/{}/definitions/{}/publish",
                self.domain, self.definition
            ),
            json!({"versionId":self.version}),
        )
        .await
    }
}
fn metadata(g: &Value, key: &str) -> Value {
    g["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node_key"] == key)
        .unwrap()["metadata"]
        .clone()
}
fn refs(m: &Value) -> [&Value; 2] {
    [
        &m["workflowPrerequisiteGate"]["producer"],
        &m["workflowOperation"]["prerequisiteProducer"],
    ]
}
#[tokio::test]
async fn one_put_normalizes_both_positions_and_publishes_over_http() {
    let f = Fixture::new().await;
    let mut unordered = f.body.clone();
    unordered["nodes"].as_array_mut().unwrap().reverse();
    let (s, r) = f.put(unordered).await;
    assert_eq!(s, 200, "{r}");
    let g = f.get().await;
    let m = metadata(&g, "qa");
    for pr in refs(&m) {
        assert_eq!(
            pr,
            &json!({"nodeId":id(&g,"produce"),"slotKey":"create-leaf"})
        );
    }
    assert_eq!(m["opaque"], f.body["nodes"][1]["metadata"]["opaque"]);
    // Runtime remains canonical-only; no authoring key fallback.
    assert!(
        svc_workflow::domain::definition::owner_obligation::gate(Some(&m))
            .unwrap()
            .is_some()
    );
    let mut raw_key = m.clone();
    raw_key["workflowPrerequisiteGate"]["producer"] =
        json!({"nodeKey":"produce","slotKey":"create-leaf"});
    assert!(svc_workflow::domain::definition::owner_obligation::gate(Some(&raw_key)).is_err());
    let (s, r) = f.publish().await;
    assert_eq!(s, 200, "{r}");
    let published = f.get().await;
    let mut helper_input = authored(&published);
    helper_input["contextSchema"] = published["version"]["context_schema"].clone();
    for n in helper_input["nodes"].as_array_mut().unwrap() {
        n["node_id"] = json!(id(&published, n["node_key"].as_str().unwrap()));
    }
    for t in helper_input["transitions"].as_array_mut().unwrap() {
        t["transition_id"] = published["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["transition_key"] == t["transition_key"])
            .unwrap()["transition_id"]
            .clone();
    }
    println!(
        "ACTUAL_PUBLISHED_HELPER_INPUT {}",
        json!({"published_readback":helper_input})
    );
    println!(
        "HTTP_AUTHORING_ONE_PUT {}",
        json!({"definition":f.definition,"version":f.version,"producer":id(&g,"produce"),"references":refs(&m),"publish":r})
    );
}
fn authored(g: &Value) -> Value {
    let nodes = g["nodes"].as_array().unwrap();
    let ts = g["transitions"].as_array().unwrap();
    let key = |id: &Value| nodes.iter().find(|n| n["node_id"] == *id).unwrap()["node_key"].clone();
    json!({"definitionVersionId":g["version"]["definition_version_id"],"nodes":nodes.iter().map(|n|json!({"node_key":n["node_key"],"display_name":n["display_name"],"order_index":n["order_index"],"node_type":n["node_type"],"assignee_ref_type":n["assignee_ref"]["ref_type"],"fixed_principal_id":n["assignee_ref"]["fixed_principal_id"],"assignee_input_key":n["assignee_ref"]["assignee_input_key"],"instructions":n["instructions"],"metadata":n["metadata"],"primary_advance_transition_key":ts.iter().find(|t|t["transition_id"]==n["primary_advance_transition_id"]).map(|t|t["transition_key"].clone())})).collect::<Vec<_>>(),"transitions":ts.iter().map(|t|json!({"transition_key":t["transition_key"],"display_name":t["display_name"],"source_node_key":key(&t["source_node_id"]),"target_node_key":key(&t["target_node_id"]),"transition_effect":t["transition_effect"],"submission_schema":t["submission_schema"],"metadata":t["metadata"]})).collect::<Vec<_>>()})
}
async fn assert_rejected_unchanged(f: &Fixture, b: Value, label: &str) {
    let before = f.get().await;
    let (s, r) = f.put(b).await;
    assert_eq!(s, 422, "{label}: {r}");
    assert_eq!(
        r["error"]["code"], "graph_validation_failed",
        "{label}: {r}"
    );
    assert_eq!(f.get().await, before, "{label} mutated graph/schema");
    println!(
        "HTTP_REJECT_ZERO_GRAPH {}",
        json!({"case":label,"status":s,"response":r})
    );
}
#[tokio::test]
async fn canonical_read_edit_copy_foreign_stale_and_rename_are_closed() {
    let f = Fixture::new().await;
    assert_eq!(f.put(f.body.clone()).await.0, 200);
    let first = f.get().await;
    let mut original = authored(&first);
    original["definitionVersionId"] = json!(f.version);
    // Existing caller IDs on raw nodes remain ignored.
    original["nodes"][0]["node_id"] = json!(id(&first, "produce"));
    original["nodes"][0]["nodeId"] = json!(id(&first, "produce"));
    let (s, r) = f.put(original.clone()).await;
    assert_eq!(s, 200, "{r}");
    let second = f.get().await;
    assert_ne!(id(&first, "produce"), id(&second, "produce"));
    for pr in refs(&metadata(&second, "qa")) {
        assert_eq!(pr["nodeId"], json!(id(&second, "produce")));
    }
    assert_rejected_unchanged(&f, original, "stale same-Version preimage").await;
    let mut renamed = authored(&second);
    renamed["definitionVersionId"] = json!(f.version);
    renamed["nodes"][0]["node_key"] = json!("renamed");
    for t in renamed["transitions"].as_array_mut().unwrap() {
        if t["source_node_key"] == "produce" {
            t["source_node_key"] = json!("renamed");
        }
    }
    assert_rejected_unchanged(&f, renamed.clone(), "old canonical key removed").await;
    renamed["nodes"][1]["metadata"]["workflowPrerequisiteGate"]["producer"] =
        json!({"nodeKey":"renamed","slotKey":"create-leaf"});
    renamed["nodes"][1]["metadata"]["workflowOperation"]["prerequisiteProducer"] =
        json!({"nodeKey":"renamed","slotKey":"create-leaf"});
    let (s, r) = f.put(renamed).await;
    assert_eq!(s, 200, "explicit rename: {r}");
    let canonical = f.get().await;
    let source_before = canonical.clone();
    let (d, v) = draft(
        &f.app,
        &f.jwk,
        f.owner,
        f.domain,
        canonical["version"]["context_schema"].clone(),
    )
    .await;
    let path = format!("/internal/v1/domains/{}/definitions/{d}/draft", f.domain);
    let mut copy = authored(&canonical);
    copy["definitionVersionId"] = json!(v);
    let (s, r) = call(&f.app, &f.jwk, f.owner, "PUT", &path, copy.clone()).await;
    assert_eq!(s, 422, "foreign source ID: {r}");
    copy["nodes"][1]["metadata"]["workflowPrerequisiteGate"]["producer"] =
        json!({"nodeKey":"renamed","slotKey":"create-leaf"});
    copy["nodes"][1]["metadata"]["workflowOperation"]["prerequisiteProducer"] =
        json!({"nodeKey":"renamed","slotKey":"create-leaf"});
    let (s, r) = call(&f.app, &f.jwk, f.owner, "PUT", &path, copy).await;
    assert_eq!(s, 200, "{r}");
    let copied = read(&f.app, &f.jwk, f.owner, f.domain, d, v).await;
    assert_ne!(id(&copied, "renamed"), id(&canonical, "renamed"));
    assert_eq!(f.get().await, source_before);
    for (def, ver) in [(f.definition, f.version), (d, v)] {
        let (s, r) = call(
            &f.app,
            &f.jwk,
            f.owner,
            "POST",
            &format!(
                "/internal/v1/domains/{}/definitions/{def}/publish",
                f.domain
            ),
            json!({"versionId":ver}),
        )
        .await;
        assert_eq!(s, 200, "{r}");
    }
    println!(
        "HTTP_READ_EDIT_COPY {}",
        json!({"firstId":id(&first,"produce"),"secondId":id(&second,"produce"),"renamedId":id(&canonical,"renamed"),"copiedId":id(&copied,"renamed"),"publishBoth":200})
    );
}
#[tokio::test]
async fn closed_shapes_semantics_and_schema_failures_preserve_graph() {
    let f = Fixture::new().await;
    assert_eq!(f.put(f.body.clone()).await.0, 200);
    let invalid = vec![
        Value::Null,
        json!([]),
        json!("produce"),
        json!({}),
        json!({"nodeKey":"produce"}),
        json!({"nodeKey":"produce","nodeId":Uuid::new_v4(),"slotKey":"create-leaf"}),
        json!({"nodeKey":"produce","slotKey":"create-leaf","extra":1}),
        json!({"nodeKey":"","slotKey":"create-leaf"}),
        json!({"nodeKey":1,"slotKey":"create-leaf"}),
        json!({"nodeKey":"missing","slotKey":"create-leaf"}),
        json!({"nodeKey":"produce","slotKey":""}),
        json!({"nodeKey":"produce","slotKey":null}),
        json!({"nodeId":"invalid","slotKey":"create-leaf"}),
        json!({"nodeId":Uuid::new_v4(),"slotKey":"create-leaf"}),
        json!({"nodeKey":"qa","slotKey":"create-second"}),
        json!({"nodeKey":"done","slotKey":"create-leaf"}),
        json!({"nodeKey":"produce","slotKey":"wrong"}),
    ];
    for path in [
        "/nodes/1/metadata/workflowPrerequisiteGate/producer",
        "/nodes/1/metadata/workflowOperation/prerequisiteProducer",
    ] {
        for (i, r) in invalid.iter().enumerate() {
            let mut b = f.body.clone();
            *b.pointer_mut(path).unwrap() = r.clone();
            assert_rejected_unchanged(&f, b, &format!("{path} closed-{i}")).await;
        }
    }
    let mut b = f.body.clone();
    b["nodes"][1]["metadata"]["workflowPrerequisiteGate"]
        .as_object_mut()
        .unwrap()
        .remove("producer");
    assert_rejected_unchanged(&f, b, "gate producer absent").await;
    let mut b = f.body.clone();
    b["nodes"][0]["node_key"] = json!("qa");
    assert_rejected_unchanged(&f, b, "duplicate key").await;
    let mut b = f.body.clone();
    b["nodes"][0]["order_index"] = json!(5);
    assert_rejected_unchanged(&f, b, "future producer").await;
    let mut b = f.body.clone();
    b["nodes"][0].as_object_mut().unwrap().remove("metadata");
    assert_rejected_unchanged(&f, b, "non-create producer").await;
    let mut b = f.body.clone();
    b["nodes"][0]["metadata"]["workflowOperation"]["allowedOperations"] =
        json!([svc_workflow::domain::definition::owner_obligation::registry()[2]["descriptor"]]);
    assert_rejected_unchanged(&f, b, "wrong producer operation").await;
    let mut b = f.body.clone();
    b["contextSchema"] = json!({"type":"object"});
    assert_rejected_unchanged(&f, b, "no managed role").await;
    let mut b = f.body.clone();
    b["contextSchema"] = json!({"type":"broken"});
    assert_rejected_unchanged(&f, b, "invalid effective schema").await;
    let mut b = f.body.clone();
    b["nodes"][2]["metadata"] = b["nodes"][1]["metadata"].clone();
    assert_rejected_unchanged(&f, b, "terminal gate placement").await;
    let mut b = f.body.clone();
    b["transitions"][0]["target_node_key"] = json!("done");
    assert_rejected_unchanged(&f, b, "unreachable producer path").await;
    let mut b = f.body.clone();
    b["contextSchema"] = Value::Null;
    // Existing HTTP Option<Value> parsing treats explicit null as absent;
    // retain that public behavior (the internal command's clear is separate).
    let schema_before = f.get().await["version"]["context_schema"].clone();
    let (status, response) = f.put(b).await;
    assert_eq!(status, 200, "existing HTTP null semantics: {response}");
    assert_eq!(f.get().await["version"]["context_schema"], schema_before);
    let mut b = f.body.clone();
    b["contextSchema"] = json!({"type":"object","x-workflow-owner-obligation-role-v1":"REVIEW","properties":{"workflowPrerequisite":svc_workflow::domain::definition::prerequisite::property_schema()}});
    assert_rejected_unchanged(&f, b, "gate forbidden REVIEW role").await;
    // Optional absence stays absent; unrelated opaque identity-like fields survive.
    let mut b = f.body.clone();
    b["nodes"][1]["metadata"]["workflowOperation"]
        .as_object_mut()
        .unwrap()
        .remove("prerequisiteProducer");
    let (s, r) = f.put(b).await;
    assert_eq!(s, 200, "{r}");
    let g = f.get().await;
    assert!(metadata(&g, "qa")["workflowOperation"]
        .get("prerequisiteProducer")
        .is_none());
    let facts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM workflow_instances),(SELECT count(*) FROM workflow_events),(SELECT count(*) FROM workflow_owner_managed_instances)").fetch_one(&f.pool).await.unwrap();
    assert_eq!(facts, (0, 0, 0));
}
fn hash(v: &Value) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(serde_json::to_string(v).unwrap().as_bytes()))
}
#[tokio::test]
async fn original_parsed_hash_failure_replay_and_historical_success_replay() {
    let f = Fixture::new().await;
    let success_key = Uuid::new_v4().to_string();
    let success = f.put_key(f.body.clone(), &success_key).await;
    assert_eq!(success.0, 200);
    let before = f.get().await;
    assert_eq!(f.put_key(f.body.clone(), &success_key).await, success);
    let mut changed = f.body.clone();
    changed["nodes"][1]["metadata"]["workflowPrerequisiteGate"]["producer"] = Value::Null;
    assert_eq!(f.put_key(changed.clone(), &success_key).await, success);
    assert_eq!(f.get().await, before);
    let stored:String=sqlx::query_scalar("SELECT request_hash FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2").bind(f.owner).bind(&success_key).fetch_one(&f.pool).await.unwrap();
    assert_eq!(
        stored,
        hash(
            &json!({"commandType":"DEFINITION_REPLACE_DRAFT","command":{"definitionVersionId":f.version}})
        )
    );
    let failed_key = Uuid::new_v4().to_string();
    let rejected = f.put_key(changed.clone(), &failed_key).await;
    assert_eq!(rejected.0, 422, "{rejected:?}");
    assert_eq!(f.put_key(changed.clone(), &failed_key).await, rejected);
    use svc_workflow::application::definition::commands::{
        RawNodeDefinition, RawTransitionDefinition,
    };
    let nodes: Vec<RawNodeDefinition> = serde_json::from_value(changed["nodes"].clone()).unwrap();
    let ts: Vec<RawTransitionDefinition> =
        serde_json::from_value(changed["transitions"].clone()).unwrap();
    let parsed = json!({"definitionVersionId":f.version,"contextSchema":null,"nodes":nodes,"transitions":ts});
    let rb:Value=sqlx::query_scalar("SELECT response_body FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2").bind(f.owner).bind(&failed_key).fetch_one(&f.pool).await.unwrap();
    assert_eq!(rb["graphInputHash"], hash(&parsed));
    let mut spelling = changed.clone();
    spelling["nodes"][1]["metadata"]["workflowPrerequisiteGate"]["producer"] =
        json!({"nodeId":id(&before,"produce"),"slotKey":"create-leaf"});
    let (s, r) = f.put_key(spelling, &failed_key).await;
    assert_eq!(s, 409, "{r}");
    // Raw DTO-ignored IDs are not added to parsed-input identity.
    changed["nodes"][0]["node_id"] = json!(Uuid::new_v4());
    changed["nodes"][0]["nodeId"] = json!(Uuid::new_v4());
    assert_eq!(f.put_key(changed, &failed_key).await, rejected);
    assert_eq!(f.get().await, before);
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=ANY($2)").bind(f.owner).bind(vec![success_key,failed_key]).fetch_one(&f.pool).await.unwrap();
    assert_eq!(count, 2);
    println!(
        "HTTP_HASH_REPLAY {}",
        json!({"successRequestHash":stored,"failureGraphInputHash":rb["graphInputHash"],"successChangedBody":200,"failureChangedBody":409,"graphUnchanged":true})
    );
}
async fn token_call(
    f: &Fixture,
    token: &str,
    method: &str,
    path: &str,
    body: Value,
) -> (u16, Value) {
    let r = f
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .header("idempotency-key", Uuid::new_v4().to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status().as_u16();
    let b = to_bytes(r.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&b).unwrap())
}
#[tokio::test]
async fn actual_auth_visibility_and_enabled_guards_precede_normalization() {
    let f = Fixture::new().await;
    assert_eq!(f.put(f.body.clone()).await.0, 200);
    let before = f.get().await;
    let outsider = actor(&f.pool).await;
    let tk = |who, scope| common::v1_token(who, scope, "gcr-http", 300, &f.jwk.key_pair);
    let mut bad = f.body.clone();
    bad["nodes"][1]["metadata"]["workflowPrerequisiteGate"]["producer"] =
        json!({"nodeId":Uuid::new_v4(),"slotKey":"FOREIGN_SECRET"});
    for (token, status) in [
        (tk(f.owner, "workflow.read"), 403),
        (tk(outsider, "workflow.execute workflow.read"), 404),
    ] {
        let (s, r) = token_call(&f, &token, "PUT", &f.put_path(), bad.clone()).await;
        assert_eq!(s, status, "{r}");
        assert!(!r.to_string().contains("FOREIGN_SECRET"));
    }
    let path = format!(
        "/internal/v1/domains/{}/definitions/{}/versions/{}",
        f.domain, f.definition, f.version
    );
    let (s, r) = token_call(&f, &tk(outsider, "workflow.read"), "GET", &path, json!({})).await;
    assert_eq!(s, 404, "{r}");
    let obo = common::v1_obo_token(
        f.owner,
        outsider,
        "workflow.execute workflow.read",
        Some("gcr-http"),
        300,
        &f.jwk.key_pair,
    );
    let (s, r) = token_call(&f, &obo, "PUT", &f.put_path(), bad.clone()).await;
    assert_eq!(s, 403, "{r}");
    sqlx::query("UPDATE principals SET enabled=false WHERE principal_id=$1")
        .bind(f.owner)
        .execute(&f.pool)
        .await
        .unwrap();
    let (s, r) = token_call(
        &f,
        &tk(f.owner, "workflow.execute workflow.read"),
        "PUT",
        &f.put_path(),
        bad.clone(),
    )
    .await;
    assert_ne!(s, 200, "{r}");
    assert!(!r.to_string().contains("FOREIGN_SECRET"));
    sqlx::query("UPDATE principals SET enabled=true WHERE principal_id=$1")
        .bind(f.owner)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE domains SET enabled=false WHERE domain_id=$1")
        .bind(f.domain)
        .execute(&f.pool)
        .await
        .unwrap();
    let (s, r) = f.put(bad).await;
    assert_ne!(s, 200, "{r}");
    sqlx::query("UPDATE domains SET enabled=true WHERE domain_id=$1")
        .bind(f.domain)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(f.get().await, before);
}
#[tokio::test]
async fn concurrent_canonical_replacements_use_actual_locked_preimage() {
    let f = Fixture::new().await;
    assert_eq!(f.put(f.body.clone()).await.0, 200);
    let first = f.get().await;
    let mut body = authored(&first);
    body["definitionVersionId"] = json!(f.version);
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("SELECT definition_version_id FROM workflow_definition_versions WHERE definition_version_id=$1 FOR UPDATE").bind(f.version).fetch_one(&mut *tx).await.unwrap();
    let run = |body: Value| {
        let app = f.app.clone();
        let owner = f.owner;
        let path = f.put_path();
        let token = common::v1_token(
            owner,
            "workflow.execute workflow.read",
            "gcr-http",
            300,
            &f.jwk.key_pair,
        );
        tokio::spawn(async move {
            let r = app
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(path)
                        .header("authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .header("idempotency-key", Uuid::new_v4().to_string())
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = r.status().as_u16();
            let b = to_bytes(r.into_body(), usize::MAX).await.unwrap();
            (status, serde_json::from_slice::<Value>(&b).unwrap())
        })
    };
    let a = run(body.clone());
    let b = run(body);
    let mut seen = 0;
    for _ in 0..250 {
        seen=sqlx::query_scalar::<_,i64>("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'SELECT version_status%FOR UPDATE%'").fetch_one(&f.pool).await.unwrap();
        if seen >= 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(
        seen, 2,
        "both actual HTTP writers must be waiting on locked Version"
    );
    tx.commit().await.unwrap();
    let aa = a.await.unwrap();
    let bb = b.await.unwrap();
    let mut statuses = vec![aa.0, bb.0];
    statuses.sort();
    assert_eq!(statuses, vec![200, 422], "{aa:?} {bb:?}");
    let g = f.get().await;
    let m = metadata(&g, "qa");
    for pr in refs(&m) {
        assert_eq!(pr["nodeId"], json!(id(&g, "produce")));
    }
    assert_ne!(id(&first, "produce"), id(&g, "produce"));
    let (s, r) = f.publish().await;
    assert_eq!(s, 200, "{r}");
    println!(
        "HTTP_CONCURRENT_PREIMAGE {}",
        json!({"blockedWritersObserved":seen,"outcomes":[aa,bb],"producer":id(&g,"produce"),"publish":r})
    );
}
#[tokio::test]
async fn concurrent_replace_and_publish_never_publish_unresolved_references() {
    let f = Fixture::new().await;
    assert_eq!(f.put(f.body.clone()).await.0, 200);
    let first = f.get().await;
    let mut body = authored(&first);
    body["definitionVersionId"] = json!(f.version);
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("SELECT definition_version_id FROM workflow_definition_versions WHERE definition_version_id=$1 FOR UPDATE").bind(f.version).fetch_one(&mut *tx).await.unwrap();
    let token = common::v1_token(
        f.owner,
        "workflow.execute workflow.read",
        "gcr-http",
        300,
        &f.jwk.key_pair,
    );
    let spawn = |method: &str, path: String, body: Value| {
        let app = f.app.clone();
        let token = token.clone();
        let method = method.to_owned();
        tokio::spawn(async move {
            let r = app
                .oneshot(
                    Request::builder()
                        .method(method.as_str())
                        .uri(path)
                        .header("authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .header("idempotency-key", Uuid::new_v4().to_string())
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = r.status().as_u16();
            let b = to_bytes(r.into_body(), usize::MAX).await.unwrap();
            (status, serde_json::from_slice::<Value>(&b).unwrap())
        })
    };
    let replace = spawn("PUT", f.put_path(), body);
    let publish = spawn(
        "POST",
        format!(
            "/internal/v1/domains/{}/definitions/{}/publish",
            f.domain, f.definition
        ),
        json!({"versionId":f.version}),
    );
    let mut seen = 0;
    for _ in 0..250 {
        seen=sqlx::query_scalar::<_,i64>("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%workflow_definition_versions%FOR UPDATE%'").fetch_one(&f.pool).await.unwrap();
        if seen >= 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(seen, 2);
    tx.commit().await.unwrap();
    let a = replace.await.unwrap();
    let b = publish.await.unwrap();
    assert!(
        matches!((a.0, b.0), (200, 200) | (200, 409) | (409, 200)),
        "{a:?} {b:?}"
    );
    let g = f.get().await;
    let model = svc_workflow::domain::definition::model::WorkflowGraph {
        nodes: serde_json::from_value(g["nodes"].clone()).unwrap(),
        transitions: serde_json::from_value(g["transitions"].clone()).unwrap(),
        context_schema: Some(g["version"]["context_schema"].clone()),
    };
    assert!(svc_workflow::domain::definition::owner_obligation::validate(3, &model).is_empty());
    for p in refs(&metadata(&g, "qa")) {
        assert_eq!(p["nodeId"], json!(id(&g, "produce")));
    }
    println!(
        "HTTP_REPLACE_PUBLISH_SERIALIZATION {}",
        json!({"blockedWritersObserved":seen,"replace":a,"publish":b,"postState":g["version"]["version_status"],"canonicalProducer":id(&g,"produce")})
    );
}
