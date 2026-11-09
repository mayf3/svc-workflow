//! SVC_WORKFLOW_CURRENT_TASK_INPUT_V1: typed, bounded read request.
use super::query_types::WorkflowQueryError;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Debug)]
pub enum CurrentTaskInputError {
    Invalid,
    Stale,
    BodyChanged,
    BodyNotFound,
    Query(WorkflowQueryError),
}
impl From<WorkflowQueryError> for CurrentTaskInputError {
    fn from(value: WorkflowQueryError) -> Self {
        Self::Query(value)
    }
}
impl From<sqlx::Error> for CurrentTaskInputError {
    fn from(value: sqlx::Error) -> Self {
        WorkflowQueryError::StorageError(value.to_string()).into()
    }
}
pub(crate) fn inconsistent(message: &str) -> CurrentTaskInputError {
    WorkflowQueryError::InternalConsistency(message.into()).into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BodyKind {
    CurrentDetail,
    SubmissionPayload,
}

#[derive(Debug, Clone, Copy)]
pub struct ExpectedCurrent {
    pub node_visit_id: Uuid,
    pub workflow_state_version: i32,
    pub context_revision_id: Uuid,
}

#[derive(Debug, Clone)]
pub enum CurrentTaskInputRequest {
    Input {
        expected: Option<ExpectedCurrent>,
        submission_ids: Vec<Uuid>,
    },
    Body {
        expected: ExpectedCurrent,
        kind: BodyKind,
        id: Uuid,
        sha256: String,
        offset: usize,
    },
}

impl CurrentTaskInputRequest {
    /// Preserve duplicates until validation: collecting directly to a map would
    /// silently accept conflicting query keys.
    pub fn parse(pairs: Vec<(String, String)>) -> Result<Self, CurrentTaskInputError> {
        use CurrentTaskInputError::Invalid;
        let mut p = BTreeMap::new();
        for (key, value) in pairs {
            if ![
                "view",
                "expectedNodeVisitId",
                "expectedWorkflowStateVersion",
                "expectedContextRevisionId",
                "submissionIds",
                "bodyKind",
                "bodyId",
                "expectedBodySha256",
                "offset",
            ]
            .contains(&key.as_str())
                || p.insert(key, value).is_some()
            {
                return Err(Invalid);
            }
        }
        let uuid = |name: &str| -> Result<Uuid, CurrentTaskInputError> {
            Uuid::parse_str(p.get(name).ok_or(Invalid)?).map_err(|_| Invalid)
        };
        let has_expected = [
            "expectedNodeVisitId",
            "expectedWorkflowStateVersion",
            "expectedContextRevisionId",
        ]
        .iter()
        .filter(|key| p.contains_key(**key))
        .count();
        let expected = match has_expected {
            0 => None,
            3 => {
                let version = p["expectedWorkflowStateVersion"]
                    .parse::<i32>()
                    .map_err(|_| Invalid)?;
                if version < 1 {
                    return Err(Invalid);
                }
                Some(ExpectedCurrent {
                    node_visit_id: uuid("expectedNodeVisitId")?,
                    workflow_state_version: version,
                    context_revision_id: uuid("expectedContextRevisionId")?,
                })
            }
            _ => return Err(Invalid),
        };
        match p.get("view").map(String::as_str).unwrap_or("input") {
            "input" => {
                if ["bodyKind", "bodyId", "expectedBodySha256", "offset"]
                    .iter()
                    .any(|k| p.contains_key(*k))
                {
                    return Err(Invalid);
                }
                let mut ids = BTreeSet::new();
                if let Some(list) = p.get("submissionIds") {
                    for id in list.split(',') {
                        if !ids.insert(Uuid::parse_str(id).map_err(|_| Invalid)?) || ids.len() > 8 {
                            return Err(Invalid);
                        }
                    }
                }
                Ok(Self::Input {
                    expected,
                    submission_ids: ids.into_iter().collect(),
                })
            }
            "body" => {
                if p.contains_key("submissionIds") {
                    return Err(Invalid);
                }
                let expected = expected.ok_or(Invalid)?;
                let kind = match p.get("bodyKind").map(String::as_str) {
                    Some("CURRENT_DETAIL") => BodyKind::CurrentDetail,
                    Some("SUBMISSION_PAYLOAD") => BodyKind::SubmissionPayload,
                    _ => return Err(Invalid),
                };
                let sha256 = p.get("expectedBodySha256").ok_or(Invalid)?.clone();
                if sha256.len() != 64
                    || !sha256
                        .bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                {
                    return Err(Invalid);
                }
                let offset = p
                    .get("offset")
                    .map(|s| s.parse::<usize>())
                    .transpose()
                    .map_err(|_| Invalid)?
                    .unwrap_or(0);
                Ok(Self::Body {
                    expected,
                    kind,
                    id: uuid("bodyId")?,
                    sha256,
                    offset,
                })
            }
            _ => Err(Invalid),
        }
    }
    pub fn expected(&self) -> Option<ExpectedCurrent> {
        match self {
            Self::Input { expected, .. } => *expected,
            Self::Body { expected, .. } => Some(*expected),
        }
    }
}
