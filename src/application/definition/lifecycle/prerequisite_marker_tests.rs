use super::validate_json_schema;
use serde_json::json;
#[test]
fn prerequisite_marker_must_fail_closed() {
    for schema in [
        json!({"type":"object","x-workflow-prerequisite-v1":false}),
        json!({"type":"object","properties":{"nested":{"x-workflow-prerequisite-v1":true}}}),
        json!({"type":"object","x-workflow-prerequisite-v1":true}),
    ] {
        assert!(validate_json_schema(&schema).is_err(), "{schema}");
    }
}
