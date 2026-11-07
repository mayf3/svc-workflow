# R3 execution environment and reproduction

**STOPPED:** the recorded full command returned rc101. The as-executed
runner omitted binary45's required `/svc_prereq_20261029_impl` database-name
suffix; 18 fixture guards failed before connection. The runner is preserved
without repair/retry. This document describes the actual failed run, not a
known-passing full-suite recipe. A later authorized continuation must fix
that fixture route before retrying.

All database endpoints are newly created, task-owned PostgreSQL 16 clusters
listening only on 127.0.0.1. No existing service, production database,
backlog, deployment, migration state, configuration, or checkout is used.
Data directories are retained after identity-checked shutdown:

- `/private/tmp/h1359-r3-pg/data`, port 56589 (focused/attribution/general);
- `/private/tmp/h1359-r3-pg-55432/data`, port 55432 (existing pinned fixtures);
- `/private/tmp/h1359-r3-pg-55483/data`, port 55483 (authorized-actions fixtures).

Cluster initdb uses `-U postgres --auth=trust --no-locale -E UTF8`, pg_ctl
uses `-h 127.0.0.1 -p <port> -k <task-cluster-root>`. Local trust roles and
fixture credentials belong only to these disposable clusters. The 55483
cluster has the test-required `swaar4` role. General integration binaries get
per-binary databases (zero-test command harnesses may share a base name);
the explicitly routed pinned-port/name fixtures retain their constraints
through `cargo-test-runner.py`. Binary45's suffix constraint was missed,
as recorded in the stop notice above.

Focused command (raw command, times, real rc and hashes in `focused.json`):

```sh
TEST_DATABASE_URL=postgres://postgres@127.0.0.1:56589/svc_workflow_test \
DATABASE_URL=postgres://postgres@127.0.0.1:56589/svc_workflow_test \
cargo test --locked --test 48_outbox_three_channels -- --test-threads=1 --nocapture
```

Full command (raw coordinates in `full.json`; each binary in
`full-binary-results.jsonl`; full unabridged output in `full.log`):

```sh
TEST_DATABASE_URL=postgres://postgres@127.0.0.1:56589/svc_workflow_test \
DATABASE_URL=postgres://postgres@127.0.0.1:56589/svc_workflow_test \
RUSTFLAGS='--cfg trusted_fleet_cutover_conformance --check-cfg=cfg(trusted_fleet_cutover_conformance)' \
CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER=/private/tmp/h1359-outbox/docs/evidence/1359-r3/cargo-test-runner.py \
cargo test --locked --jobs 4 -- --test-threads=1 --nocapture
```

The conformance cfg is the existing repository runner's convention from
`scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh:54`. The
existing runner itself was not used because its local branch binds another
task's database identity. The cutover fixture's workflow database has all
repository migrations applied individually with psql `-1 ON_ERROR_STOP`;
its auth database starts empty. Its read-only frozen source plan has SHA
`0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606`,
verified before execution. Creation/migration output is preserved in
`full-environment.log`.

No test is removed or ignored by this runner. The normal suite's existing
one ignored owner-wake database test remains ignored. Optional old-binary
downgrade and canonical deployed-principal legs are not supplied a binary
or identity. Their emitted NOT_RUN/skip messages are coverage limits, not
runtime acceptance. Fixture reuse flags are cleared to avoid older fixture
connections. The default suite's real return code remains the gate.

Attribution: use detached worktrees at the exact base and implementation
listed in `BRIEF-R3.md`, pin/transplant R2 fixture bytes, then run
`prepare-attribution.py`. On each tree, select `H1359_BACKOFF_ATTRIBUTION=1`
and run the focused binary with filter `sustained_`. Both expected rc101
results must reach the original backoff assertion, with the same test SHA;
different failure sites do not establish attribution. See `ATTRIBUTION.md`.
