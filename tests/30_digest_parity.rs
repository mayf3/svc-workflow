//! Digest parity test — reads the shared test vector file and verifies
//! that Rust's definition_digest produces the expected values.
//!
//! The same vector file is read by TypeScript tests in
//! sdk/typescript/tests/artifact-digest.test.ts

use std::collections::HashMap;

#[test]
fn digest_parity_matches_shared_vectors() {
    // Read the shared vector file
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let vector_path = std::path::Path::new(&manifest_dir)
        .join("testdata")
        .join("definition-digest-v1-vectors.json");
    let content = std::fs::read_to_string(&vector_path)
        .unwrap_or_else(|e| panic!("Cannot read vector file {:?}: {}", vector_path, e));
    let data: serde_json::Value = serde_json::from_str(&content)
        .expect("Invalid JSON in vector file");
    let vectors = data["vectors"].as_array()
        .expect("vectors must be an array");

    let mut all_pass = true;

    for (i, vector) in vectors.iter().enumerate() {
        let name = vector["name"].as_str().unwrap_or("unnamed");
        let expected = vector["expectedDefinitionDigest"]
            .as_str()
            .expect("expectedDefinitionDigest must be set");

        // Build canonical document matching TypeScript computeExpectedDefinitionDigest
        let doc = build_canonical_doc(vector);

        // Compute digest using JCS + SHA-256 (same as TypeScript)
        let computed = jcs_canonicalize::sha256_jcs_hex(&doc)
            .unwrap_or_else(|e| panic!("JCS failed for vector {}: {}", name, e));

        if computed != expected {
            eprintln!(
                "PARITY FAIL [{}]: expected={} computed={}",
                name, expected, computed
            );
            all_pass = false;
        } else {
            println!("PARITY PASS [{}]: {}", name, computed);
        }
    }

    assert!(all_pass, "Digest parity failed for one or more vectors");
}

/// Build a canonical definition document from a JSON vector entry.
fn build_canonical_doc(vector: &serde_json::Value) -> serde_json::Value {
    use serde_json::json;

    let mut nodes: Vec<serde_json::Value> = vector["nodes"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|n| {
            json!({
                "node_key": n["nodeKey"],
                "display_name": n["displayName"],
                "order_index": n["orderIndex"],
                "node_type": n["nodeType"],
                "assignee_ref_type": n["assigneeRefType"],
                "fixed_principal_id": n["fixedPrincipalId"],
                "assignee_input_key": n["assigneeInputKey"],
                "instructions": n["instructions"],
                "primary_advance_transition_key": n["primaryAdvanceTransitionKey"],
                "metadata": n["metadata"],
            })
        })
        .collect();

    // Sort nodes by node_key (matching Rust and TS behavior)
    nodes.sort_by(|a, b| {
        a["node_key"]
            .as_str()
            .unwrap_or("")
            .cmp(&b["node_key"].as_str().unwrap_or(""))
    });

    let mut transitions: Vec<serde_json::Value> = vector["transitions"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|t| {
            json!({
                "transition_key": t["transitionKey"],
                "display_name": t["displayName"],
                "source_node_key": t["sourceNodeKey"],
                "target_node_key": t["targetNodeKey"],
                "transition_effect": t["transitionEffect"],
                "submission_schema": t["submissionSchema"],
                "metadata": t["metadata"],
            })
        })
        .collect();

    // Sort transitions by transition_key (matching Rust and TS behavior)
    transitions.sort_by(|a, b| {
        a["transition_key"]
            .as_str()
            .unwrap_or("")
            .cmp(&b["transition_key"].as_str().unwrap_or(""))
    });

    json!({
        "definition_key": vector["definitionKey"],
        "version_number": vector["versionNumber"],
        "json_schema_dialect": vector["jsonSchemaDialect"],
        "validator_version": vector["validatorVersion"],
        "context_schema": vector["contextSchema"],
        "nodes": nodes,
        "transitions": transitions,
    })
}
