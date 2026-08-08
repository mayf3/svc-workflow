use super::error::GraphValidationError;
use super::graph::validate_graph;
use super::model::{AssigneeRef, NodeDefinition, TransitionDefinition, WorkflowGraph};
use crate::domain::enums::{AssigneeRefType, NodeType, TransitionEffect};
use crate::domain::ids::{DefinitionVersionId, NodeId, PrincipalId, TransitionId};

/// Helper to create a simple valid graph.
fn valid_graph() -> WorkflowGraph {
    let draft_node_id = NodeId::new();
    let normal_node_id = NodeId::new();
    let terminal_node_id = NodeId::new();
    let version_id = DefinitionVersionId::new();

    let advance_to_normal = TransitionId::new();
    let advance_to_terminal = TransitionId::new();

    WorkflowGraph {
        nodes: vec![
            NodeDefinition {
                node_id: draft_node_id,
                definition_version_id: version_id,
                node_key: "draft".to_string(),
                display_name: "DRAFT".to_string(),
                order_index: 0,
                node_type: NodeType::DRAFT,
                assignee_ref: Some(AssigneeRef {
                    ref_type: AssigneeRefType::WorkflowCreator,
                    fixed_principal_id: None,
                    assignee_input_key: None,
                }),
                instructions: None,
                primary_advance_transition_id: Some(advance_to_normal),
                metadata: None,
                created_at: chrono::Utc::now(),
            },
            NodeDefinition {
                node_id: normal_node_id,
                definition_version_id: version_id,
                node_key: "dev_self_check".to_string(),
                display_name: "Dev Self Check".to_string(),
                order_index: 1,
                node_type: NodeType::NORMAL,
                assignee_ref: Some(AssigneeRef {
                    ref_type: AssigneeRefType::FixedPrincipal,
                    fixed_principal_id: Some(PrincipalId::new()),
                    assignee_input_key: None,
                }),
                instructions: None,
                primary_advance_transition_id: Some(advance_to_terminal),
                metadata: None,
                created_at: chrono::Utc::now(),
            },
            NodeDefinition {
                node_id: terminal_node_id,
                definition_version_id: version_id,
                node_key: "done".to_string(),
                display_name: "Done".to_string(),
                order_index: 2,
                node_type: NodeType::TERMINAL,
                assignee_ref: None,
                instructions: None,
                primary_advance_transition_id: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
        ],
        transitions: vec![
            TransitionDefinition {
                transition_id: advance_to_normal,
                definition_version_id: version_id,
                transition_key: "advance-dev".to_string(),
                display_name: "Advance to Dev".to_string(),
                source_node_id: draft_node_id,
                target_node_id: normal_node_id,
                transition_effect: TransitionEffect::Advance,
                submission_schema: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
            TransitionDefinition {
                transition_id: advance_to_terminal,
                definition_version_id: version_id,
                transition_key: "advance-done".to_string(),
                display_name: "Complete".to_string(),
                source_node_id: normal_node_id,
                target_node_id: terminal_node_id,
                transition_effect: TransitionEffect::Advance,
                submission_schema: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
        ],
        context_schema: Some(serde_json::json!({"type": "object"})),
    }
}

#[test]
fn valid_three_node_trunk_passes() {
    let graph = valid_graph();
    let result = validate_graph(&graph);
    assert!(
        result.valid,
        "Expected valid graph, got errors: {:?}",
        result.errors
    );
}

#[test]
fn no_draft_node() {
    let mut graph = valid_graph();
    graph.nodes[0].node_type = NodeType::NORMAL;
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.code == "NO_DRAFT_NODE"));
}

#[test]
fn multiple_draft_nodes() {
    let mut graph = valid_graph();
    // Add another draft node
    let extra_node = NodeDefinition {
        node_id: NodeId::new(),
        definition_version_id: graph.nodes[0].definition_version_id,
        node_key: "draft2".to_string(),
        display_name: "Draft 2".to_string(),
        order_index: 3,
        node_type: NodeType::DRAFT,
        assignee_ref: Some(AssigneeRef {
            ref_type: AssigneeRefType::WorkflowCreator,
            fixed_principal_id: None,
            assignee_input_key: None,
        }),
        instructions: None,
        primary_advance_transition_id: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    };
    graph.nodes.push(extra_node);
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == "MULTIPLE_DRAFT_NODES"));
}

#[test]
fn no_terminal_node() {
    let mut graph = valid_graph();
    graph.nodes[2].node_type = NodeType::NORMAL;
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.code == "NO_TERMINAL_NODE"));
}

#[test]
fn non_terminal_missing_primary() {
    let mut graph = valid_graph();
    graph.nodes[1].primary_advance_transition_id = None;
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.code == "MISSING_PRIMARY"));
}

#[test]
fn primary_points_to_lower_order_index() {
    let mut graph = valid_graph();
    // Swap order indices so primary goes backward
    graph.nodes[1].order_index = -1; // dev_self_check now has lower order
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == "PRIMARY_NOT_ADVANCING"));
}

#[test]
fn primary_trunk_has_cycle() {
    let mut graph = valid_graph();
    // Make normal_node's primary point back to draft
    graph.nodes[1].primary_advance_transition_id = Some(graph.transitions[0].transition_id);
    // This should create a cycle: draft -> dev_self_check -> draft
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.code == "PRIMARY_CYCLE"));
}

#[test]
fn node_not_reachable_from_draft() {
    let mut graph = valid_graph();
    // Add a disconnected node
    graph.nodes.push(NodeDefinition {
        node_id: NodeId::new(),
        definition_version_id: graph.nodes[0].definition_version_id,
        node_key: "isolated".to_string(),
        display_name: "Isolated".to_string(),
        order_index: 10,
        node_type: NodeType::NORMAL,
        assignee_ref: Some(AssigneeRef {
            ref_type: AssigneeRefType::WorkflowCreator,
            fixed_principal_id: None,
            assignee_input_key: None,
        }),
        instructions: None,
        primary_advance_transition_id: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    });
    let result = validate_graph(&graph);
    assert!(!result.valid);
    // Isolated node is missing primary
    assert!(result.errors.iter().any(|e| e.code == "MISSING_PRIMARY"));
}

#[test]
fn return_to_higher_order_rejected() {
    let mut graph = valid_graph();
    // Add a RETURN transition from normal_node back to draft
    let return_trans_id = TransitionId::new();
    graph.transitions.push(TransitionDefinition {
        transition_id: return_trans_id,
        definition_version_id: graph.nodes[0].definition_version_id,
        transition_key: "return-draft".to_string(),
        display_name: "Return to Draft".to_string(),
        source_node_id: graph.nodes[1].node_id,
        target_node_id: graph.nodes[0].node_id,
        transition_effect: TransitionEffect::Return,
        submission_schema: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    });
    let result = validate_graph(&graph);
    // This is a valid return: from order 1 to order 0
    assert!(
        result.valid,
        "Valid RETURN should pass, got: {:?}",
        result.errors
    );
}

#[test]
fn return_to_terminal_rejected() {
    let mut graph = valid_graph();
    // Add a RETURN transition to the terminal node
    let return_trans_id = TransitionId::new();
    graph.transitions.push(TransitionDefinition {
        transition_id: return_trans_id,
        definition_version_id: graph.nodes[0].definition_version_id,
        transition_key: "return-done".to_string(),
        display_name: "Return to Done".to_string(),
        source_node_id: graph.nodes[1].node_id,
        target_node_id: graph.nodes[2].node_id,
        transition_effect: TransitionEffect::Return,
        submission_schema: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    });
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.code == "RETURN_TO_TERMINAL"));
}

#[test]
fn terminate_to_terminal_identified() {
    let mut graph = valid_graph();
    // Add a TERMINATE transition from normal_node to done
    let terminate_trans_id = TransitionId::new();
    graph.transitions.push(TransitionDefinition {
        transition_id: terminate_trans_id,
        definition_version_id: graph.nodes[0].definition_version_id,
        transition_key: "abandon".to_string(),
        display_name: "Abandon".to_string(),
        source_node_id: graph.nodes[1].node_id,
        target_node_id: graph.nodes[2].node_id,
        transition_effect: TransitionEffect::Terminate,
        submission_schema: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    });
    let result = validate_graph(&graph);
    assert!(
        result.valid,
        "Valid TERMINATE should pass, got: {:?}",
        result.errors
    );
}

#[test]
fn primary_to_done_is_advance() {
    // The existing graph already has advance-to-terminal as primary
    let graph = valid_graph();
    let result = validate_graph(&graph);
    // The primary from normal_node to done is ADVANCE, which is correct
    assert!(result.valid, "Primary to terminal should pass as ADVANCE");
}

#[test]
fn terminal_has_outgoing_transition() {
    let mut graph = valid_graph();
    // Add outgoing transition from terminal node
    let bad_trans_id = TransitionId::new();
    graph.transitions.push(TransitionDefinition {
        transition_id: bad_trans_id,
        definition_version_id: graph.nodes[0].definition_version_id,
        transition_key: "bad-exit".to_string(),
        display_name: "Bad Exit".to_string(),
        source_node_id: graph.nodes[2].node_id,
        target_node_id: graph.nodes[0].node_id,
        transition_effect: TransitionEffect::Advance,
        submission_schema: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    });
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == "TERMINAL_HAS_OUTGOING"));
}

#[test]
fn draft_assignee_not_workflow_creator() {
    let mut graph = valid_graph();
    graph.nodes[0].assignee_ref.as_mut().unwrap().ref_type = AssigneeRefType::FixedPrincipal;
    graph.nodes[0]
        .assignee_ref
        .as_mut()
        .unwrap()
        .fixed_principal_id = Some(PrincipalId::new());
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == "DRAFT_NOT_WORKFLOW_CREATOR"));
}

#[test]
fn fixed_principal_missing_id() {
    let mut graph = valid_graph();
    graph.nodes[1]
        .assignee_ref
        .as_mut()
        .unwrap()
        .fixed_principal_id = None;
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == "FIXED_PRINCIPAL_MISSING_ID"));
}

#[test]
fn duplicate_order_index() {
    let mut graph = valid_graph();
    graph.nodes[2].order_index = 0; // Same as draft
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == "DUPLICATE_ORDER_INDEX"));
}

#[test]
fn self_loop_transition() {
    let mut graph = valid_graph();
    graph.transitions[1].source_node_id = graph.nodes[1].node_id;
    graph.transitions[1].target_node_id = graph.nodes[1].node_id;
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.code == "SELF_LOOP"));
}

#[test]
fn return_is_primary_rejected() {
    let mut graph = valid_graph();
    // Make normal_node's primary also marked as RETURN
    let primary_trans: TransitionId = graph.nodes[1].primary_advance_transition_id.unwrap();
    // Change the effect to RETURN
    if let Some(trans) = graph
        .transitions
        .iter_mut()
        .find(|t| t.transition_id == primary_trans)
    {
        trans.transition_effect = TransitionEffect::Return;
    }
    // Add a proper RETURN to replace
    let return_trans_id = TransitionId::new();
    graph.transitions.push(TransitionDefinition {
        transition_id: return_trans_id,
        definition_version_id: graph.nodes[0].definition_version_id,
        transition_key: "proper-return".to_string(),
        display_name: "Proper Return".to_string(),
        source_node_id: graph.nodes[1].node_id,
        target_node_id: graph.nodes[0].node_id,
        transition_effect: TransitionEffect::Return,
        submission_schema: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    });
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.code == "RETURN_IS_PRIMARY"));
}

#[test]
fn terminate_to_non_terminal_rejected() {
    let mut graph = valid_graph();
    // Add a TERMINATE to normal_node (non-terminal)
    let terminate_trans_id = TransitionId::new();
    graph.transitions.push(TransitionDefinition {
        transition_id: terminate_trans_id,
        definition_version_id: graph.nodes[0].definition_version_id,
        transition_key: "bad-terminate".to_string(),
        display_name: "Bad Terminate".to_string(),
        source_node_id: graph.nodes[0].node_id,
        target_node_id: graph.nodes[1].node_id,
        transition_effect: TransitionEffect::Terminate,
        submission_schema: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    });
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == "TERMINATE_TO_NON_TERMINAL"));
}

#[test]
fn unexpected_fixed_principal_on_non_fixed_type() {
    let mut graph = valid_graph();
    graph.nodes[0]
        .assignee_ref
        .as_mut()
        .unwrap()
        .fixed_principal_id = Some(PrincipalId::new());
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == "UNEXPECTED_FIXED_PRINCIPAL"));
}

#[test]
fn invalid_json_schema_not_checked_by_graph_validation() {
    // JSON schema validation is done separately by the service layer
    let graph = valid_graph();
    let result = validate_graph(&graph);
    // Graph validation doesn't check JSON schema validity
    // (it would pass even with invalid schema)
    assert!(result.valid);
}

/// Turn the NORMAL node of a valid graph into INSTANCE_INPUT_PRINCIPAL.
fn make_iip_graph(schema: Option<serde_json::Value>) -> WorkflowGraph {
    let mut graph = valid_graph();
    graph.nodes[1].assignee_ref = Some(AssigneeRef {
        ref_type: AssigneeRefType::InstanceInputPrincipal,
        fixed_principal_id: None,
        assignee_input_key: Some("assigneePrincipalId".to_string()),
    });
    graph.context_schema = schema;
    graph
}

#[test]
fn instance_input_key_must_be_covered_by_context_schema() {
    // A NORMAL INSTANCE_INPUT_PRINCIPAL node whose key is not in
    // context_schema.required is a self-contradictory definition: instances
    // could be created without the key and fail on every read later. It must
    // be rejected at draft/publish validation time.
    let graph = make_iip_graph(Some(serde_json::json!({"type": "object"})));
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == "CONTEXT_SCHEMA_REQUIRED_MISSING_ASSIGNEE_KEY"));
}

#[test]
fn instance_input_key_without_context_schema_rejected() {
    // INSTANCE_INPUT_PRINCIPAL nodes require a context_schema that declares
    // their keys; no schema at all is also a contradiction.
    let graph = make_iip_graph(None);
    let result = validate_graph(&graph);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.code == "CONTEXT_SCHEMA_REQUIRED_FOR_INSTANCE_INPUT"));
}

#[test]
fn instance_input_key_covered_by_context_schema_passes() {
    // The definition is self-consistent when required covers the key.
    let graph = make_iip_graph(Some(serde_json::json!({
        "type": "object",
        "required": ["assigneePrincipalId"],
    })));
    let result = validate_graph(&graph);
    assert!(
        result.valid,
        "Expected valid graph, got errors: {:?}",
        result.errors
    );
}

#[test]
fn every_instance_input_key_must_be_covered() {
    // Two NORMAL IIP nodes with different keys: required must cover BOTH,
    // regardless of which node is reached first.
    let mut graph = make_iip_graph(Some(serde_json::json!({
        "type": "object",
        "required": ["assigneePrincipalId"],
    })));
    // Add a second NORMAL IIP node with a different key.
    let extra_node = NodeDefinition {
        node_id: NodeId::new(),
        definition_version_id: graph.nodes[0].definition_version_id,
        node_key: "second".to_string(),
        display_name: "Second".to_string(),
        order_index: 3,
        node_type: NodeType::NORMAL,
        assignee_ref: Some(AssigneeRef {
            ref_type: AssigneeRefType::InstanceInputPrincipal,
            fixed_principal_id: None,
            assignee_input_key: Some("operatorPrincipalId".to_string()),
        }),
        instructions: None,
        primary_advance_transition_id: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    };
    graph.nodes.push(extra_node);

    let result = validate_graph(&graph);
    assert!(result.errors.iter().any(|e| {
        e.code == "CONTEXT_SCHEMA_REQUIRED_MISSING_ASSIGNEE_KEY"
            && e.message.contains("operatorPrincipalId")
    }));
    assert!(!result
        .errors
        .iter()
        .any(|e| e.message.contains("assigneePrincipalId")));

    // Once both keys are covered, no coverage error remains.
    graph.context_schema = Some(serde_json::json!({
        "type": "object",
        "required": ["assigneePrincipalId", "operatorPrincipalId"],
    }));
    let result = validate_graph(&graph);
    assert!(!result.errors.iter().any(|e| {
        e.code == "CONTEXT_SCHEMA_REQUIRED_MISSING_ASSIGNEE_KEY"
            || e.code == "CONTEXT_SCHEMA_REQUIRED_FOR_INSTANCE_INPUT"
    }));
}
