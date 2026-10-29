use super::*;
#[test]
fn exact_opt_in_and_unmarked_business_data() {
    let schema = json!({"type":"object",MARKER:true,"required":["workflowPrerequisite"],"properties":{"workflowPrerequisite":property_schema()}});
    assert!(validate_schema(&schema).unwrap());
    let value = json!({"workflowPrerequisite":{"kind":"TERMINAL_SUBMISSION","workflowInstanceId":Uuid::new_v4(),"definitionVersionId":Uuid::new_v4(),"transitionId":Uuid::new_v4()}});
    assert!(parse(Some(&schema), &value).unwrap().is_some());
    assert!(parse(Some(&json!({"type":"object"})), &value)
        .unwrap()
        .is_none());
    for field in ["submissionId", "status", "predicate"] {
        let mut bad = value.clone();
        bad["workflowPrerequisite"][field] = json!("x");
        assert!(parse(Some(&schema), &bad).is_err());
    }
    let mut bad = schema.clone();
    bad["properties"]["workflowPrerequisite"]["additionalProperties"] = json!(true);
    assert!(validate_schema(&bad).is_err());
}
