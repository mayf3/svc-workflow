> R4 STOPPED: this actual full run returned rc101 (1324 passed /10 failed /1 ignored), solely binary67. The45 route passed28/28 and48 passed22/22. No repair or rerun followed. This is a record of the failed attempt, not a passing full-suite recipe.

# R4 fixed-name routing and full-suite reproduction

R4 is a separate execution from the preserved R3 failure. The bounded
correction is `cargo-test-runner.py:19-20`: binary
`45_workflow_prerequisites` receives
`postgres://postgres@127.0.0.1:56589/svc_prereq_20261029_impl` in both
`TEST_DATABASE_URL` and `DATABASE_URL`. The existing Rust fixture checks this
fixed suffix, then creates its own fresh UUID database per test. No Rust
fixture or product source was changed in R4. See `runner-correction.diff`.

All endpoints below belong to newly created disposable PG16 clusters,
listening only on 127.0.0.1. `prepare-environment.py` refuses pre-existing
paths or occupied ports; the runner verifies the actual server data
directory, port and listen address before database writes.

| Port | Data directory | Route |
|---|---|---|
| 56589 | `/private/tmp/h1359-r4-pg/data` | General per-binary DBs, binary45 fixed entry name, isolated cutover workflow/auth |
| 55432 | `/private/tmp/h1359-r4-pg-55432/data` | Existing pinned fixtures 41,42,46,47 |
| 55483 | `/private/tmp/h1359-r4-pg-55483/data` | Existing binary67 role/name constraints |

Run preparation once, only when these paths do not exist and ports are
free. Its exact commands/output are in `full-environment.log` and live
identity readbacks in `environment.json`. The workflow cutover fixture
receives all repository migrations; the auth cutover fixture starts empty.
The read-only frozen plan SHA is checked before creation, matching the
repository's existing conformance recipe.

```sh
python3 docs/evidence/1359-r4/prepare-environment.py
env TEST_DATABASE_URL=postgres://postgres@127.0.0.1:56589/svc_workflow_test \
DATABASE_URL=postgres://postgres@127.0.0.1:56589/svc_workflow_test \
RUSTFLAGS='--cfg trusted_fleet_cutover_conformance --check-cfg=cfg(trusted_fleet_cutover_conformance)' \
CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER=/private/tmp/h1359-outbox/docs/evidence/1359-r4/cargo-test-runner.py \
python3 docs/evidence/1359-r4/run-record.py full /private/tmp/h1359-outbox \
cargo test --locked --jobs 4 -- --test-threads=1 --nocapture
```

`full.log` is unabridged combined output. `full.json` records the actual
command, start/end time, source/test/runner hashes and return code only
after completion. `full-binary-results.jsonl` records every completed
runner invocation and actual database URL/rc. `validation-input-manifest.json`
binds all tracked source/test/migration/build inputs before execution.
`summary.json` derives totals from the final raw output and binary records.

No tests are removed or newly ignored. The existing ignored database test
and optional old-binary/canonical-principal legs remain coverage limits,
as in R3. No old binary or production Principal is supplied. Fixture reuse
flags are cleared. Binary67 synthetic probes are retained under
`/private/tmp/h1359-r4-aa-results`; the suite is reproducible from this
script and records its probe paths in the full log. No production database,
existing service, other checkout, backlog, deployment or loaded config is
mutated. Source validation is distinct from production/runtime acceptance.
