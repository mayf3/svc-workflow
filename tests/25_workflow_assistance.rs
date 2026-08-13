//! Workflow Assistance V1 transaction, authorization, recovery, and resume tests.

#![allow(clippy::too_many_arguments)]

#[path = "25_workflow_assistance/core.rs"]
mod core;
#[path = "25_workflow_assistance/db_invariants.rs"]
mod db_invariants;
#[path = "25_workflow_assistance/helpers.rs"]
mod helpers;
#[path = "25_workflow_assistance/lifecycle.rs"]
mod lifecycle;
#[path = "25_workflow_assistance/permissions.rs"]
mod permissions;
#[path = "25_workflow_assistance/replay_history.rs"]
mod replay_history;
