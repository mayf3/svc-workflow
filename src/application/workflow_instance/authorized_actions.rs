//! Endpoint-only response types for WORKFLOW_INSTANCE_AUTHORIZED_ACTIONS_V1.
//! Shared Full detail, worklists and current-task-input retain their old types.
use serde::Serialize;
use uuid::Uuid;

use super::query_types::{FullWorkflowInstanceDetail, ParticipantWorkflowInstanceDetail};

#[derive(Debug, Clone, Copy)]
pub struct ActionCredential {
    pub execute_scope: bool,
    pub write_gate_open: bool,
    pub max_returns_per_edge: u32,
}

#[derive(Debug, Serialize)]
#[serde(tag = "visibility", content = "detail", rename_all = "snake_case")]
pub enum AuthorizedInstanceDetail {
    Full(Box<InstanceDetailWithAuthorizedActionsV1>),
    HistoricalParticipant(ParticipantWorkflowInstanceDetail),
}

#[derive(Debug, Serialize)]
pub struct InstanceDetailWithAuthorizedActionsV1 {
    #[serde(flatten)]
    pub full: FullWorkflowInstanceDetail,
    pub authorized_actions_scope: &'static str,
    pub authorized_actions_status: AuthorizedActionsStatus,
    pub authorized_actions: Vec<AuthorizedAction>,
    pub current_task_type: CurrentTaskType,
}

#[derive(Debug, Serialize)]
pub struct CurrentTaskType {
    pub source: &'static str,
    pub node_type: String,
    pub business_task_type: Option<()>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorizedActionsStatus {
    Available,
    NoneForActor,
    MissingExecuteScope,
    WriteGateClosed,
    UnsupportedModel,
}

#[derive(Debug, Serialize)]
pub enum RequestValidation {
    #[serde(rename = "REQUIRED_AT_EXECUTION")]
    RequiredAtExecution,
}

#[derive(Debug, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum AuthorizedAction {
    Transition {
        transition_id: Uuid,
        transition_effect: String,
        request_validation: RequestValidation,
    },
    RequestAssistance {
        request_validation: RequestValidation,
    },
    EscalateAssistance {
        assistance_case_id: Uuid,
        request_validation: RequestValidation,
    },
    ResolveAssistance {
        assistance_case_id: Uuid,
        request_validation: RequestValidation,
    },
    Cancel {
        request_validation: RequestValidation,
    },
    Archive {
        request_validation: RequestValidation,
    },
}
