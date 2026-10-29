//! CTR-SWPO-001..006 on a fresh run-scoped PostgreSQL database. No skipped DB tests.
#![allow(dead_code, unused_imports)]
#[path = "common/mod.rs"]
mod common;
#[path = "25_workflow_assistance/helpers.rs"]
mod helpers;
#[path = "prerequisites/http_support.rs"]
mod http_support;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use helpers::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use svc_workflow::{
    application::workflow_instance::create::create_workflow_instance,
    domain::{
        enums::WorkflowExecutionClass, ids::*,
        workflow_instance::commands::CreateWorkflowInstanceCommand,
    },
    store::postgres::admission_gate::AdmissionGate,
};
use tower::ServiceExt;
use uuid::Uuid;
#[path = "prerequisites/support.rs"]
mod support;
use support::*;
#[path = "prerequisites/admission.rs"]
mod admission;
#[path = "prerequisites/atomic.rs"]
mod atomic;
#[path = "prerequisites/lineage.rs"]
mod lineage;
#[path = "prerequisites/poll.rs"]
mod poll;
#[path = "prerequisites/publication.rs"]
mod publication;
#[path = "prerequisites/race.rs"]
mod race;
#[path = "prerequisites/readiness.rs"]
mod readiness;
