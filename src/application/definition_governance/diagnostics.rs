//! Safe presentation of canonical graph rejection; never a validator.
use crate::domain::definition::error::GraphValidationError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
mod catalog;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    code: String,
    message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphDiagnostics {
    errors: Vec<Rule>,
    truncated: bool,
}

impl GraphDiagnostics {
    pub(super) fn project(errors: Vec<GraphValidationError>) -> Self {
        let mut codes: BTreeSet<&str> = errors
            .iter()
            .map(|error| {
                if catalog::correction(&error.code).is_some() {
                    error.code.as_str()
                } else {
                    "GRAPH_VALIDATION_REJECTED"
                }
            })
            .collect();
        if codes.is_empty() {
            codes.insert("GRAPH_VALIDATION_REJECTED");
        }
        Self {
            truncated: codes.len() > 32,
            errors: codes
                .into_iter()
                .take(32)
                .map(|code| Rule {
                    code: code.into(),
                    message: catalog::correction(code).unwrap().into(),
                })
                .collect(),
        }
    }

    pub fn message(&self) -> String {
        let first = &self.errors[0];
        format!(
            "graph validation failed (rule: {}): {}",
            first.code.replace('_', " "),
            first.message
        )
    }

    /// Receipt data is untrusted: only the closed, bounded static projection may replay.
    pub(super) fn from_receipt(value: serde_json::Value) -> Option<Self> {
        let value: Self = serde_json::from_value(value).ok()?;
        if value.errors.is_empty()
            || value.errors.len() > 32
            || (value.truncated && value.errors.len() != 32)
        {
            return None;
        }
        let mut previous = "";
        for rule in &value.errors {
            if rule.code.as_str() <= previous
                || catalog::correction(&rule.code) != Some(rule.message.as_str())
            {
                return None;
            }
            previous = &rule.code;
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn catalog_is_complete_bounded_and_sanitizer_safe() {
        let spec =
            include_str!("../../../docs/specs/SVC_WORKFLOW_DEFINITION_GRAPH_DIAGNOSTICS_V1.md");
        let appendix = spec.split("## Appendix A").nth(1).unwrap();
        for code in appendix
            .lines()
            .filter_map(|l| l.strip_prefix("- `").and_then(|s| s.strip_suffix('`')))
        {
            let d = GraphDiagnostics::project(vec![GraphValidationError::new(code, "SQL SECRET")]);
            assert_eq!(d.errors[0].code, code);
            assert!(!d.message().contains("SQL SECRET"));
            assert!(d.message().len() <= 512);
            assert!(d.errors[0].message.is_ascii());
            assert!(d.errors[0].message.len() <= 350);
            // Same character class as the downstream opaque-token sanitizer.
            assert!(d
                .message()
                .split(|c: char| !(c.is_ascii_alphanumeric() || "._~+/-".contains(c)))
                .all(|s| s.len() < 24));
            assert_eq!(
                GraphDiagnostics::from_receipt(serde_json::to_value(&d).unwrap()),
                Some(d)
            );
        }
    }

    #[test]
    fn projection_deduplicates_sorts_caps_and_ignores_untrusted_strings() {
        let spec =
            include_str!("../../../docs/specs/SVC_WORKFLOW_DEFINITION_GRAPH_DIAGNOSTICS_V1.md");
        let mut errors: Vec<_> = spec
            .split("## Appendix A")
            .nth(1)
            .unwrap()
            .lines()
            .filter_map(|l| l.strip_prefix("- `").and_then(|s| s.strip_suffix('`')))
            .map(|code| GraphValidationError::new(code, "SECRET".repeat(10000)))
            .collect();
        let d = GraphDiagnostics::project(errors.clone());
        errors.reverse();
        errors.extend(errors.clone());
        assert_eq!(d, GraphDiagnostics::project(errors));
        assert_eq!(d.errors.len(), 32);
        assert!(d.truncated);
        let unknown =
            GraphDiagnostics::project(vec![GraphValidationError::new("SQL SECRET", "credential")]);
        assert_eq!(unknown.errors[0].code, "GRAPH_VALIDATION_REJECTED");
        assert_eq!(unknown, GraphDiagnostics::project(vec![]));
    }

    #[test]
    fn corrupt_receipt_fails_closed() {
        for value in [
            json!({"errors":[],"truncated":false}),
            json!({"errors":[{"code":"SELF_LOOP","message":"SECRET"}],"truncated":false}),
            json!({"errors":[{"code":"UNRECOGNIZED","message":"SECRET"}],"truncated":false}),
            json!({"errors":[],"truncated":false,"secret":"SQL"}),
        ] {
            assert!(GraphDiagnostics::from_receipt(value).is_none());
        }
        let good = GraphDiagnostics::project(vec![GraphValidationError::new("SELF_LOOP", "")]);
        let mut duplicate = serde_json::to_value(&good).unwrap();
        duplicate["errors"] = json!([
            duplicate["errors"][0].clone(),
            duplicate["errors"][0].clone()
        ]);
        assert!(GraphDiagnostics::from_receipt(duplicate).is_none());
    }
}
