pub(crate) mod intents;
pub mod policy;
pub mod reconcile;
use serde_json::{json, Value};
#[derive(Debug, Clone)]
pub struct OwnerError {
    pub status: u16,
    pub code: String,
    pub receipt_body: Value,
}
impl OwnerError {
    pub fn new(status: u16, code: &str) -> Self {
        Self {
            status,
            code: code.into(),
            receipt_body: json!({"error":code.to_ascii_lowercase()}),
        }
    }
    pub fn with_body(mut self, body: Value) -> Self {
        self.receipt_body = body;
        self
    }
    pub fn body(&self) -> Value {
        self.receipt_body.clone()
    }
}
pub(crate) fn storage(e: impl std::fmt::Display) -> OwnerError {
    tracing::error!(error=%e,"owner obligation storage failure");
    OwnerError::new(503, "service_unavailable")
}
impl From<crate::domain::workflow_instance::assistance::AssistanceError> for OwnerError {
    fn from(e: crate::domain::workflow_instance::assistance::AssistanceError) -> Self {
        Self::new(e.status_code() as u16, e.code())
            .with_body(super::assistance_transaction::error_body(&e))
    }
}
pub mod effects;
pub(crate) mod guards;
pub mod reads;

pub(crate) mod case_projection;
pub(crate) mod transition_context;
