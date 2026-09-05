//! Workflow Instance application service.
//!
//! Provides the CreateWorkflowInstance use case with full idempotency,
//! authorization, and atomic consistency guarantees.

pub mod admin_recovery;
pub mod admin_repair;
pub mod archive;
pub mod assistance;
pub mod cancel;
pub mod create;
/// Canonical work-eligibility classification shared by read projections
/// (SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1).
pub mod eligibility;
pub mod execute_transition;
pub mod idempotency;
pub mod import;
pub mod query_service;
pub mod wake;
pub mod query_types;
pub mod revise;
pub mod revise_and_transition;
