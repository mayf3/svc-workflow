//! #1344 independent real-router acceptance overlay; no product test replacement.
#![allow(dead_code)]
#[path="common/mod.rs"] mod common;
#[path="authorized_actions_r4/support.rs"] mod support;
#[path="authorized_actions_r4/fixture.rs"] mod fixture;
#[path="authorized_actions_r4/http.rs"] mod http;

#[path="authorized_actions_r4/matrix.rs"] mod matrix;

#[path="authorized_actions_r4/preservation.rs"] mod preservation;

#[path="authorized_actions_r4/managed.rs"] mod managed;
#[path="authorized_actions_r4/guards.rs"] mod guards;
#[path="authorized_actions_r4/faults.rs"] mod faults;
#[path="authorized_actions_r4/snapshot.rs"] mod snapshot;
#[path="authorized_actions_r4/linked.rs"] mod linked;
#[path="authorized_actions_r4/review.rs"] mod review;
#[path="authorized_actions_r4/coverage.rs"] mod coverage;
#[path="authorized_actions_r4/coverage_cleanup.rs"] mod coverage_cleanup;
