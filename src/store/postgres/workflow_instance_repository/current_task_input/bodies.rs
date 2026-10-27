use crate::application::workflow_instance::current_task_input::{
    inconsistent, BodyKind, CurrentTaskInputError,
};
use crate::domain::definition::digest::compute_sha256;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

pub(super) struct CanonicalBody {
    pub kind: BodyKind,
    pub id: Uuid,
    pub text: String,
    pub sha256: String,
}
impl CanonicalBody {
    pub fn new(
        kind: BodyKind,
        id: Uuid,
        value: &Value,
        stored_digest: Option<&str>,
    ) -> Result<Self, CurrentTaskInputError> {
        let text = jcs_canonicalize::canonicalize(&value.to_string())
            .map_err(|_| inconsistent("current input canonicalization failed"))?;
        let sha256 = compute_sha256(text.as_bytes());
        if stored_digest.is_some_and(|stored| stored != sha256) {
            return Err(inconsistent("current input payload digest mismatch"));
        }
        Ok(Self {
            kind,
            id,
            text,
            sha256,
        })
    }
    pub fn chunk(&self, offset: usize, budget: usize) -> Result<Body, CurrentTaskInputError> {
        if offset > self.text.len() || !self.text.is_char_boundary(offset) {
            return Err(CurrentTaskInputError::Invalid);
        }
        let mut end = offset.saturating_add(budget).min(self.text.len());
        while !self.text.is_char_boundary(end) {
            end -= 1;
        }
        Ok(Body {
            kind: self.kind,
            id: self.id,
            sha256: self.sha256.clone(),
            encoding: "utf-8",
            serialization: "JCS",
            total_bytes: self.text.len(),
            offset,
            text: self.text[offset..end].to_string(),
            next_offset: (end < self.text.len()).then_some(end),
            content_status: if end == self.text.len() {
                "COMPLETE"
            } else if end == offset {
                "OMITTED"
            } else {
                "TRUNCATED"
            },
        })
    }
}

#[derive(Serialize)]
pub(super) struct Body {
    kind: BodyKind,
    id: Uuid,
    sha256: String,
    encoding: &'static str,
    serialization: &'static str,
    total_bytes: usize,
    offset: usize,
    pub text: String,
    next_offset: Option<usize>,
    content_status: &'static str,
}

/// Trim only inline bytes, in reverse allocation order. Never truncate JSON,
/// descriptors or references. Escaped wire length, not source length, is bound.
pub(super) fn bound_input_wire(value: &mut Value) -> Result<(), CurrentTaskInputError> {
    while serde_json::to_vec(value)
        .map_err(|_| inconsistent("input serialization"))?
        .len()
        > 65536
    {
        let mut trimmed = false;
        for item in value["submissions"]
            .as_array_mut()
            .ok_or_else(|| inconsistent("input bodies"))?
            .iter_mut()
            .rev()
        {
            if trim(&mut item["body"]) {
                trimmed = true;
                break;
            }
        }
        if !trimmed && !trim(&mut value["detail_body"]) {
            return Err(inconsistent("input metadata exceeds bound"));
        }
    }
    let content = value["detail_body"]["content_status"] == "COMPLETE"
        && value["submissions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["body"]["content_status"] == "COMPLETE");
    value["content_complete"] = Value::Bool(content);
    let refs = value["requested_submissions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["status"] == "FOUND");
    value["input_status"] = Value::String(
        if content && refs && value["entry"]["status"] != "UNSUPPORTED_ENTRY" {
            "COMPLETE_FOR_SELECTION"
        } else {
            "INCOMPLETE"
        }
        .into(),
    );
    // Replacing initial false/INCOMPLETE with longer complete values can add bytes.
    if serde_json::to_vec(value)
        .map_err(|_| inconsistent("input serialization"))?
        .len()
        > 65536
    {
        trim(&mut value["detail_body"]);
        return bound_input_wire(value);
    }
    Ok(())
}
fn trim(body: &mut Value) -> bool {
    let text = body["text"].as_str().unwrap_or("");
    if text.is_empty() {
        return false;
    }
    let mut end = text.len().saturating_sub(256);
    while !text.is_char_boundary(end) {
        end -= 1
    }
    body["text"] = Value::String(text[..end].to_string());
    body["next_offset"] = Value::from(end);
    body["content_status"] = Value::String(if end == 0 { "OMITTED" } else { "TRUNCATED" }.into());
    true
}
