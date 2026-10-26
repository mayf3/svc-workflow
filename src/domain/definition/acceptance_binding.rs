//! SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1 — the closed opt-in annotation
//! and the binding predicates (CTR-ACB-001..003).
//!
//! Pure domain functions: no I/O, no authority interpretation. The validator
//! compares local facts (catalog shape, literal IDs, literal verdicts) and
//! the canonical Context UUID string; it never interprets evidence meaning,
//! fetches resources, or decides business truth (CTR-ACB-003: a true PASS
//! remains the reviewer's evidence-backed responsibility).
//!
//! Failure details name shapes and counts only — they never echo Context
//! text (catalog descriptions) (CTR-ACB-004 envelope discipline).

use uuid::Uuid;

use super::error::GraphValidationError;
use super::model::{SemanticModelVersion, WorkflowGraph};
use crate::domain::enums::{NodeType, TransitionEffect};

/// The reserved annotation key. A malformed present annotation never means
/// absence (CTR-ACB-001).
pub const ANNOTATION_KEY: &str = "x-workflow-acceptance-binding-v1";

/// Fixed literal pointers — not expressions, external references, or
/// caller-selected paths (CTR-ACB-001).
pub const CONTEXT_CATALOG_POINTER: &str = "/acceptanceCriteria";
pub const SUBMISSION_REPORT_POINTER: &str = "/acceptanceReport";

/// JSON member names addressed by the fixed pointers (inside the Context
/// payload and the submission payload respectively).
pub const CATALOG_MEMBER: &str = "acceptanceCriteria";
pub const REPORT_MEMBER: &str = "acceptanceReport";

/// Literal verdict required on every result entry (CTR-ACB-003.4).
pub const PASS_VERDICT: &str = "PASS";

pub const MAX_CATALOG_ITEMS: usize = 128;
pub const MAX_DESCRIPTION_BYTES: usize = 4096;
pub const MAX_CATALOG_ID_BYTES: usize = 64;

/// Marker for a well-formed root annotation with the exact fixed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptanceBinding;

/// True when the reserved key occurs anywhere in the value at any depth.
///
/// Used for fail-closed detection where placement validity does not matter
/// (Create gate trigger, combined revise-and-transition backstop): any
/// occurrence — including malformed or null values — counts (CTR-ACB-004).
pub fn contains_annotation_at_any_depth(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            map.contains_key(ANNOTATION_KEY)
                || map.values().any(contains_annotation_at_any_depth)
        }
        serde_json::Value::Array(items) => items.iter().any(contains_annotation_at_any_depth),
        _ => false,
    }
}

/// Strict root-only inspection of a submission schema (CTR-ACB-001).
///
/// * `Ok(None)` — annotation absent anywhere: existing behavior.
/// * `Ok(Some(AcceptanceBinding))` — the key appears exactly once, at the
///   root, with exactly the two fixed literal pointer values.
/// * `Err(detail)` — present but malformed: wrong shape, unknown members,
///   changed/missing pointer values, or a nested occurrence. A malformed
///   present annotation never means absence.
pub fn inspect_submission_schema(
    schema: &serde_json::Value,
) -> Result<Option<AcceptanceBinding>, String> {
    let root_present = schema
        .as_object()
        .is_some_and(|map| map.contains_key(ANNOTATION_KEY));
    if root_present {
        validate_annotation_value(&schema[ANNOTATION_KEY])?;
    }
    if nested_occurrence(schema) {
        return Err(format!(
            "{ANNOTATION_KEY} is reserved and may appear only at the root of a \
             transition submission schema; nested occurrence found"
        ));
    }
    if root_present {
        Ok(Some(AcceptanceBinding))
    } else {
        Ok(None)
    }
}

/// Scan for occurrences of the key strictly below the root level.
fn nested_occurrence(schema: &serde_json::Value) -> bool {
    let Some(map) = schema.as_object() else {
        // Non-object schemas cannot carry a root member, but they can still
        // nest the key inside arrays/objects.
        return contains_annotation_at_any_depth(schema);
    };
    for (key, value) in map {
        if key == ANNOTATION_KEY {
            // Occurrences inside the annotation value itself are nested.
            if let serde_json::Value::Object(inner) = value {
                if inner.values().any(contains_annotation_at_any_depth) {
                    return true;
                }
            } else if contains_annotation_at_any_depth(value) {
                return true;
            }
        } else if contains_annotation_at_any_depth(value) {
            return true;
        }
    }
    false
}

fn validate_annotation_value(value: &serde_json::Value) -> Result<(), String> {
    let Some(map) = value.as_object() else {
        return Err(format!(
            "{ANNOTATION_KEY} must be an object with exactly the members \
             contextCatalogPointer and submissionReportPointer"
        ));
    };
    if map.len() != 2 {
        return Err(format!(
            "{ANNOTATION_KEY} must contain exactly contextCatalogPointer and \
             submissionReportPointer; found {} member(s)",
            map.len()
        ));
    }
    for (member, expected) in [
        ("contextCatalogPointer", CONTEXT_CATALOG_POINTER),
        ("submissionReportPointer", SUBMISSION_REPORT_POINTER),
    ] {
        match map.get(member) {
            Some(serde_json::Value::String(s)) if s == expected => {}
            _ => {
                return Err(format!(
                    "{ANNOTATION_KEY}.{member} must be the literal string \"{expected}\""
                ))
            }
        }
    }
    Ok(())
}

/// CTR-ACB-002 catalog shape check. Returns the ordered catalog IDs.
///
/// The catalog MUST be an array of 1–128 objects, each with exactly `id` and
/// `description`. IDs fully match `[A-Za-z0-9][A-Za-z0-9_-]{0,63}` and are
/// unique by exact case-sensitive value. Descriptions are nonblank strings
/// of at most 4096 UTF-8 bytes.
pub fn validate_catalog(catalog: &serde_json::Value) -> Result<Vec<String>, String> {
    let Some(items) = catalog.as_array() else {
        return Err("acceptanceCriteria catalog must be a JSON array".to_string());
    };
    if items.is_empty() {
        return Err(format!(
            "acceptanceCriteria catalog must contain 1-{MAX_CATALOG_ITEMS} entries"
        ));
    }
    if items.len() > MAX_CATALOG_ITEMS {
        return Err(format!(
            "acceptanceCriteria catalog exceeds {MAX_CATALOG_ITEMS} entries"
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut ids = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Some(map) = item.as_object() else {
            return Err(format!(
                "acceptanceCriteria entry {index} must be an object with exactly id and description"
            ));
        };
        if map.len() != 2 {
            return Err(format!(
                "acceptanceCriteria entry {index} must contain exactly id and \
                 description; found {} member(s)",
                map.len()
            ));
        }
        let Some(id) = map.get("id").and_then(serde_json::Value::as_str) else {
            return Err(format!(
                "acceptanceCriteria entry {index} must carry a string id"
            ));
        };
        if !is_valid_catalog_id(id) {
            return Err(format!(
                "acceptanceCriteria entry {index} id must match \
                 [A-Za-z0-9][A-Za-z0-9_-]{{0,63}}"
            ));
        }
        if !seen.insert(id.to_string()) {
            return Err(format!("acceptanceCriteria contains duplicate id '{id}'"));
        }
        let Some(description) = map.get("description").and_then(serde_json::Value::as_str)
        else {
            return Err(format!(
                "acceptanceCriteria entry {index} must carry a string description"
            ));
        };
        if description.trim().is_empty() {
            return Err(format!(
                "acceptanceCriteria entry {index} description must be a nonblank string"
            ));
        }
        if description.len() > MAX_DESCRIPTION_BYTES {
            return Err(format!(
                "acceptanceCriteria entry {index} description exceeds \
                 {MAX_DESCRIPTION_BYTES} UTF-8 bytes"
            ));
        }
        ids.push(id.to_string());
    }
    Ok(ids)
}

fn is_valid_catalog_id(id: &str) -> bool {
    if id.is_empty() || id.len() > MAX_CATALOG_ID_BYTES {
        return false;
    }
    let mut chars = id.chars();
    let first = chars.next().expect("checked non-empty");
    if !first.is_ascii_alphanumeric() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// CTR-ACB-003.3/4 acceptance-report predicate, evaluated against the
/// submission payload.
///
/// `expected_context_revision_id` is the canonical UUID of the locked
/// Instance's `current_context_revision_id`; `catalog_ids` is the validated
/// catalog ID set of that exact Context. The report must be an object whose
/// `contextRevisionId` equals the canonical UUID string and whose `results`
/// carry every catalog ID exactly once with the literal verdict "PASS".
/// Additional report/result fields are governed by the submission schema —
/// the binding validator does not interpret them.
pub fn validate_acceptance_report(
    payload: &serde_json::Value,
    expected_context_revision_id: Uuid,
    catalog_ids: &[String],
) -> Result<(), String> {
    let report = payload.get(REPORT_MEMBER).ok_or_else(|| {
        format!("submission is missing the acceptanceReport object at \"{SUBMISSION_REPORT_POINTER}\"")
    })?;
    let Some(report) = report.as_object() else {
        return Err("acceptanceReport must be an object".to_string());
    };

    let expected = expected_context_revision_id.to_string();
    match report.get("contextRevisionId") {
        None => {
            return Err("acceptanceReport.contextRevisionId is required".to_string());
        }
        Some(serde_json::Value::String(s)) if s == &expected => {}
        Some(_) => {
            return Err(
                "acceptanceReport.contextRevisionId does not match the current \
                 Context revision of this instance"
                    .to_string(),
            );
        }
    }

    let Some(results) = report.get("results").and_then(serde_json::Value::as_array) else {
        return Err("acceptanceReport.results must be an array".to_string());
    };

    let mut seen = std::collections::BTreeSet::new();
    for (index, entry) in results.iter().enumerate() {
        let Some(map) = entry.as_object() else {
            return Err(format!("acceptanceReport results entry {index} must be an object"));
        };
        let Some(id) = map.get("id").and_then(serde_json::Value::as_str) else {
            return Err(format!(
                "acceptanceReport results entry {index} must carry a string id"
            ));
        };
        match map.get("verdict") {
            Some(serde_json::Value::String(v)) if v == PASS_VERDICT => {}
            _ => {
                return Err(format!(
                    "acceptanceReport results entry {index} verdict must be the \
                     literal string \"{PASS_VERDICT}\""
                ))
            }
        }
        if !seen.insert(id.to_string()) {
            return Err(format!("acceptanceReport contains duplicate result id '{id}'"));
        }
    }

    if seen.len() != catalog_ids.len() || catalog_ids.iter().any(|id| !seen.contains(id)) {
        let missing = catalog_ids.iter().filter(|id| !seen.contains(*id)).count();
        let unknown = seen.iter().filter(|id| !catalog_ids.contains(id)).count();
        if missing > 0 {
            return Err(format!(
                "acceptanceReport results are missing {missing} required catalog id(s)"
            ));
        }
        return Err(format!(
            "acceptanceReport results contain {unknown} id(s) that are not in \
             the instance's catalog"
        ));
    }
    Ok(())
}

/// CTR-ACB-001 publication/validate annotation matrix over a whole graph.
///
/// Returns one error per violating transition. Absence everywhere is valid.
/// A well-formed root annotation is valid only on a model3 (VisitActivation)
/// TASK's primary ADVANCE transition; RETURN, TERMINATE, non-model3, and
/// DRAFT-source-kind placements are rejected.
pub fn validate_graph_annotation_placement(
    semantic_model: SemanticModelVersion,
    graph: &WorkflowGraph,
) -> Vec<GraphValidationError> {
    let mut errors = Vec::new();
    for transition in &graph.transitions {
        let Some(schema) = transition.submission_schema.as_ref() else {
            continue;
        };
        match inspect_submission_schema(schema) {
            Ok(None) => {}
            Ok(Some(_)) => {
                if let Some(reason) = placement_violation(semantic_model, graph, transition) {
                    errors.push(GraphValidationError::new(
                        "ACCEPTANCE_BINDING_MISPLACED",
                        format!(
                            "transition '{}': {}",
                            transition.transition_key, reason
                        ),
                    ));
                }
            }
            Err(detail) => {
                errors.push(GraphValidationError::new(
                    "ACCEPTANCE_BINDING_MALFORMED",
                    format!("transition '{}': {}", transition.transition_key, detail),
                ));
            }
        }
    }
    errors
}

fn placement_violation(
    semantic_model: SemanticModelVersion,
    graph: &WorkflowGraph,
    transition: &super::model::TransitionDefinition,
) -> Option<String> {
    if semantic_model != SemanticModelVersion::VisitActivation {
        return Some(
            "the acceptance binding opt-in is allowed only on a model3 \
             (VISIT_ACTIVATION) definition version"
                .to_string(),
        );
    }
    if !matches!(transition.transition_effect, TransitionEffect::Advance) {
        return Some(
            "the acceptance binding opt-in is allowed only on ADVANCE transitions"
                .to_string(),
        );
    }
    let source = graph
        .nodes
        .iter()
        .find(|node| node.node_id == transition.source_node_id)?;
    if source.node_type != NodeType::TASK {
        return Some(
            "the acceptance binding opt-in source node kind must be TASK".to_string(),
        );
    }
    if source.primary_advance_transition_id != Some(transition.transition_id) {
        return Some(
            "the acceptance binding opt-in is allowed only on the source TASK's \
             primary ADVANCE transition"
                .to_string(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_annotation() -> serde_json::Value {
        json!({
            "contextCatalogPointer": CONTEXT_CATALOG_POINTER,
            "submissionReportPointer": SUBMISSION_REPORT_POINTER,
        })
    }

    fn schema_with(annotation: Option<serde_json::Value>) -> serde_json::Value {
        let mut schema = json!({
            "type": "object",
            "required": ["acceptanceReport"],
            "properties": {"acceptanceReport": {"type": "object"}},
        });
        if let Some(a) = annotation {
            schema[ANNOTATION_KEY] = a;
        }
        schema
    }

    // -- inspect_submission_schema ----------------------------------------

    #[test]
    fn absent_annotation_is_none() {
        let schema = schema_with(None);
        assert_eq!(inspect_submission_schema(&schema), Ok(None));
    }

    #[test]
    fn exact_root_annotation_is_some() {
        let schema = schema_with(Some(valid_annotation()));
        assert_eq!(inspect_submission_schema(&schema), Ok(Some(AcceptanceBinding)));
    }

    #[test]
    fn malformed_values_never_mean_absence() {
        for bad in [
            json!(null),
            json!(true),
            json!("text"),
            json!([]),
            json!({}),
            json!({"contextCatalogPointer": "/acceptanceCriteria"}),
            json!({"contextCatalogPointer": "/other",
                   "submissionReportPointer": "/acceptanceReport"}),
            json!({"contextCatalogPointer": "/acceptanceCriteria",
                   "submissionReportPointer": "/acceptanceReport",
                   "extra": 1}),
        ] {
            let schema = schema_with(Some(bad.clone()));
            assert!(
                inspect_submission_schema(&schema).is_err(),
                "{bad} must be malformed"
            );
        }
    }

    #[test]
    fn nested_occurrence_is_rejected_even_with_valid_root() {
        let mut schema = schema_with(Some(valid_annotation()));
        schema["properties"]["acceptanceReport"]["properties"][ANNOTATION_KEY] =
            valid_annotation();
        assert!(inspect_submission_schema(&schema).is_err());
    }

    #[test]
    fn nested_only_occurrence_is_rejected() {
        let mut schema = schema_with(None);
        schema["properties"][ANNOTATION_KEY] = valid_annotation();
        assert!(inspect_submission_schema(&schema).is_err());
    }

    #[test]
    fn nested_occurrence_inside_annotation_value_rejected() {
        let mut value = valid_annotation();
        value["inner"] = json!({ANNOTATION_KEY: valid_annotation()});
        let schema = schema_with(Some(value));
        assert!(inspect_submission_schema(&schema).is_err());
    }

    #[test]
    fn key_inside_array_is_detected_at_any_depth() {
        let schema = json!({"anyOf": [true, {ANNOTATION_KEY: valid_annotation()}]});
        assert!(contains_annotation_at_any_depth(&schema));
        assert!(inspect_submission_schema(&schema).is_err());
    }

    // -- validate_catalog --------------------------------------------------

    fn catalog_entry(id: &str, description: &str) -> serde_json::Value {
        json!({"id": id, "description": description})
    }

    #[test]
    fn valid_catalog_returns_ordered_ids() {
        let catalog = json!([catalog_entry("C1", "a"), catalog_entry("C2", "b")]);
        assert_eq!(
            validate_catalog(&catalog).unwrap(),
            vec!["C1".to_string(), "C2".to_string()]
        );
    }

    #[test]
    fn catalog_boundaries_are_exact() {
        // 64-char id and 4096-byte description are inside the limits.
        let long_id = "A".repeat(64);
        let long_desc = "x".repeat(4096);
        let ok = json!([catalog_entry(&long_id, &long_desc)]);
        assert!(validate_catalog(&ok).is_ok());

        let too_long_id = "A".repeat(65);
        let catalog = json!([catalog_entry(&too_long_id, "d")]);
        assert!(validate_catalog(&catalog).is_err());

        let too_long_desc = "x".repeat(4097);
        let catalog = json!([catalog_entry("C1", &too_long_desc)]);
        assert!(validate_catalog(&catalog).is_err());
    }

    #[test]
    fn catalog_id_charset_is_closed() {
        for id in ["-bad", "_bad", "has space", "has.dot", "", "é"] {
            let catalog = json!([catalog_entry(id, "d")]);
            assert!(validate_catalog(&catalog).is_err(), "{id}");
        }
        for id in ["C1", "a", "Z9", "a-b_c9"] {
            let catalog = json!([catalog_entry(id, "d")]);
            assert!(validate_catalog(&catalog).is_ok(), "{id}");
        }
    }

    #[test]
    fn catalog_uniqueness_is_case_sensitive() {
        let catalog = json!([catalog_entry("C1", "a"), catalog_entry("c1", "b")]);
        assert!(validate_catalog(&catalog).is_ok());
        let catalog = json!([catalog_entry("C1", "a"), catalog_entry("C1", "b")]);
        assert!(validate_catalog(&catalog).is_err());
    }

    #[test]
    fn catalog_shape_violations_rejected() {
        for catalog in [
            json!([]),
            json!({}),
            json!(["C1"]),
            json!([{"id": "C1"}]),
            json!([{"id": "C1", "description": "a", "extra": true}]),
            json!([{"id": "C1", "description": "   "},]),
            json!([{"id": "C1", "description": 5}]),
            json!([catalog_entry("C1", "a"), catalog_entry("C1", "b")]),
            json!((0..129).map(|i| catalog_entry(&format!("C{i}"), "d")).collect::<Vec<_>>()),
        ] {
            assert!(validate_catalog(&catalog).is_err(), "{catalog}");
        }
        // 128 entries are exactly at the limit.
        let full = json!((0..128)
            .map(|i| catalog_entry(&format!("C{i}"), "d"))
            .collect::<Vec<_>>());
        assert_eq!(validate_catalog(&full).unwrap().len(), 128);
    }

    // -- validate_acceptance_report ----------------------------------------

    fn ctx() -> Uuid {
        Uuid::parse_str("5f2c0b1e-1c1d-4c6e-9d1a-8b2e4f6a7c8d").unwrap()
    }

    fn report(context: &str, entries: serde_json::Value) -> serde_json::Value {
        json!({"acceptanceReport": {"contextRevisionId": context, "results": entries}})
    }

    fn passes(ids: &[&str]) -> serde_json::Value {
        json!(ids
            .iter()
            .map(|id| json!({"id": id, "verdict": "PASS"}))
            .collect::<Vec<_>>())
    }

    #[test]
    fn exact_report_passes() {
        let payload = report(&ctx().to_string(), passes(&["C1", "C2"]));
        assert!(validate_acceptance_report(&payload, ctx(), &["C1".into(), "C2".into()]).is_ok());
    }

    #[test]
    fn order_is_irrelevant() {
        let payload = report(&ctx().to_string(), passes(&["C2", "C1"]));
        assert!(validate_acceptance_report(&payload, ctx(), &["C1".into(), "C2".into()]).is_ok());
    }

    #[test]
    fn wrong_uuid_shapes_rejected() {
        for context in [
            Uuid::new_v4().to_string(),
            ctx().to_string().to_uppercase(),
            "not-a-uuid".to_string(),
        ] {
            let payload = report(&context, passes(&["C1", "C2"]));
            assert!(
                validate_acceptance_report(&payload, ctx(), &["C1".into(), "C2".into()])
                    .is_err(),
                "{context}"
            );
        }
        let payload = json!({"acceptanceReport": {"results": passes(&["C1", "C2"])}});
        assert!(validate_acceptance_report(&payload, ctx(), &["C1".into(), "C2".into()]).is_err());
    }

    #[test]
    fn id_set_must_match_exactly_once() {
        for entries in [
            passes(&["C1"]),
            passes(&["C1", "C2", "CX"]),
            passes(&["C1", "C1"]),
            passes(&[]),
        ] {
            let payload = report(&ctx().to_string(), entries);
            assert!(
                validate_acceptance_report(&payload, ctx(), &["C1".into(), "C2".into()])
                    .is_err()
            );
        }
    }

    #[test]
    fn verdict_must_be_literal_pass() {
        for verdict in [
            serde_json::Value::from("FAIL"),
            serde_json::Value::from("pass"),
            serde_json::Value::from("PASS "),
            serde_json::Value::from(1),
            serde_json::Value::from(true),
        ] {
            let entries = json!([
                {"id": "C1", "verdict": "PASS"},
                {"id": "C2", "verdict": verdict}
            ]);
            let payload = report(&ctx().to_string(), entries);
            assert!(
                validate_acceptance_report(&payload, ctx(), &["C1".into(), "C2".into()])
                    .is_err(),
                "{verdict}"
            );
        }
    }

    #[test]
    fn extra_report_fields_are_not_interpreted() {
        let payload = json!({
            "acceptanceReport": {
                "contextRevisionId": ctx().to_string(),
                "results": [{"id": "C1", "verdict": "PASS", "evidence": "whatever"}],
                "note": "free-form",
            },
            "unrelated": true,
        });
        assert!(validate_acceptance_report(&payload, ctx(), &["C1".into()]).is_ok());
    }

    #[test]
    fn missing_report_object_is_rejected() {
        let payload = json!({"something": "else"});
        assert!(validate_acceptance_report(&payload, ctx(), &["C1".into()]).is_err());
    }

    // -- validate_graph_annotation_placement --------------------------------

    use crate::domain::definition::model::{
        NodeDefinition, TransitionDefinition,
    };
    use crate::domain::ids::{NodeId, TransitionId};

    fn node(node_id: NodeId, node_type: NodeType, primary: Option<TransitionId>) -> NodeDefinition {
        NodeDefinition {
            node_id,
            definition_version_id: crate::domain::ids::DefinitionVersionId::from_uuid(Uuid::new_v4()),
            node_key: "n".to_string(),
            display_name: "N".to_string(),
            order_index: 0,
            node_type,
            assignee_ref: None,
            instructions: None,
            primary_advance_transition_id: primary,
            metadata: None,
            created_at: chrono::Utc::now(),
        }
    }

    fn transition(
        transition_id: TransitionId,
        source: NodeId,
        target: NodeId,
        effect: TransitionEffect,
        schema: Option<serde_json::Value>,
    ) -> TransitionDefinition {
        TransitionDefinition {
            transition_id,
            definition_version_id: crate::domain::ids::DefinitionVersionId::from_uuid(Uuid::new_v4()),
            transition_key: "t".to_string(),
            display_name: "T".to_string(),
            source_node_id: source,
            target_node_id: target,
            transition_effect: effect,
            submission_schema: schema,
            metadata: None,
            created_at: chrono::Utc::now(),
        }
    }

    fn ids() -> (NodeId, NodeId, TransitionId) {
        (
            NodeId::from_uuid(Uuid::new_v4()),
            NodeId::from_uuid(Uuid::new_v4()),
            TransitionId::from_uuid(Uuid::new_v4()),
        )
    }

    #[test]
    fn placement_model3_primary_advance_from_task_is_valid() {
        let (source, target, tid) = ids();
        let graph = WorkflowGraph {
            nodes: vec![
                node(source, NodeType::TASK, Some(tid)),
                node(target, NodeType::TERMINAL, None),
            ],
            transitions: vec![transition(
                tid,
                source,
                target,
                TransitionEffect::Advance,
                Some(schema_with(Some(valid_annotation()))),
            )],
            context_schema: None,
        };
        assert!(validate_graph_annotation_placement(SemanticModelVersion::VisitActivation, &graph)
            .is_empty());
    }

    #[test]
    fn placement_nonprimary_advance_rejected() {
        let (source, target, tid) = ids();
        let (other_tid) = TransitionId::from_uuid(Uuid::new_v4());
        let graph = WorkflowGraph {
            nodes: vec![
                node(source, NodeType::TASK, Some(other_tid)),
                node(target, NodeType::TERMINAL, None),
            ],
            transitions: vec![transition(
                tid,
                source,
                target,
                TransitionEffect::Advance,
                Some(schema_with(Some(valid_annotation()))),
            )],
            context_schema: None,
        };
        let errors =
            validate_graph_annotation_placement(SemanticModelVersion::VisitActivation, &graph);
        assert_eq!(errors[0].code, "ACCEPTANCE_BINDING_MISPLACED");
    }

    #[test]
    fn placement_return_terminate_rejected() {
        for effect in [TransitionEffect::Return, TransitionEffect::Terminate] {
            let (source, target, tid) = ids();
            let graph = WorkflowGraph {
                nodes: vec![
                    node(source, NodeType::TASK, Some(tid)),
                    node(target, NodeType::TERMINAL, None),
                ],
                transitions: vec![transition(
                    tid,
                    source,
                    target,
                    effect,
                    Some(schema_with(Some(valid_annotation()))),
                )],
                context_schema: None,
            };
            let errors =
                validate_graph_annotation_placement(SemanticModelVersion::VisitActivation, &graph);
            assert_eq!(errors[0].code, "ACCEPTANCE_BINDING_MISPLACED");
        }
    }

    #[test]
    fn placement_non_model3_rejected() {
        for model in [
            SemanticModelVersion::Legacy,
            SemanticModelVersion::Minimal,
        ] {
            let (source, target, tid) = ids();
            let graph = WorkflowGraph {
                nodes: vec![
                    node(source, NodeType::DRAFT, Some(tid)),
                    node(target, NodeType::TERMINAL, None),
                ],
                transitions: vec![transition(
                    tid,
                    source,
                    target,
                    TransitionEffect::Advance,
                    Some(schema_with(Some(valid_annotation()))),
                )],
                context_schema: None,
            };
            let errors = validate_graph_annotation_placement(model, &graph);
            assert_eq!(errors[0].code, "ACCEPTANCE_BINDING_MISPLACED");
        }
    }

    #[test]
    fn placement_malformed_annotation_rejected_on_any_model() {
        let (source, target, tid) = ids();
        let graph = WorkflowGraph {
            nodes: vec![
                node(source, NodeType::TASK, Some(tid)),
                node(target, NodeType::TERMINAL, None),
            ],
            transitions: vec![transition(
                tid,
                source,
                target,
                TransitionEffect::Advance,
                Some(schema_with(Some(json!(null)))),
            )],
            context_schema: None,
        };
        let errors =
            validate_graph_annotation_placement(SemanticModelVersion::VisitActivation, &graph);
        assert_eq!(errors[0].code, "ACCEPTANCE_BINDING_MALFORMED");
    }

    #[test]
    fn placement_unannotated_graph_is_silent() {
        let (source, target, tid) = ids();
        let graph = WorkflowGraph {
            nodes: vec![
                node(source, NodeType::TASK, Some(tid)),
                node(target, NodeType::TERMINAL, None),
            ],
            transitions: vec![transition(
                tid,
                source,
                target,
                TransitionEffect::Advance,
                Some(schema_with(None)),
            )],
            context_schema: None,
        };
        assert!(validate_graph_annotation_placement(SemanticModelVersion::Legacy, &graph).is_empty());
    }
}
