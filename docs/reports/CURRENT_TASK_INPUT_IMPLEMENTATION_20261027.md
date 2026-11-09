# Current-task input implementation — service evidence

This candidate adds the authorized current-task read from accepted
`SVC_WORKFLOW_CURRENT_TASK_INPUT_V1`. Callers obtain the current binding, exact
introducing event, selected Submission originals and recent RETURN references
without choosing a history page. Bounded body continuation requires both the
current coordinate triple and the exact body digest.

## Route and authority

- `AUTHORITY_ACTION=REUSE`, `ROUTE_STAGE=IMPLEMENTATION`,
  `AUTHORITY_ACCEPTED_IN_BASE=YES`, `ASSURANCE_LEVEL=CONTROLLED`.
- Base: `52fcb13e63ac7c6696f2c3353ab5a09302cf2014` (accepted PR #89).
- Spec SHA-256:
  `173e11faddcb4a7d4b61f9c77f06f83db7672f1d68a255bbccbf42dd6be48cce`.
- User-delegated parent mandate: isolated implementation, local database/HTTP
  tests, bound candidate binary and draft PR. Production deployment, production
  database operations and merge are outside this writer's task.
- Single write surface: branch `codex/workflow-current-input-impl-20261027`,
  worktree `/Users/yanfenma/workspace/worktrees/codex-workflow-current-input-impl-20261027`.
- Accepted Spec, migrations, existing authorization and command behavior remain
  unchanged. The existing query surfaces are retained supported interfaces;
  this projection adds no ledger, workflow engine, dispatcher or semantic verdict.

Persistent preflight, plan, exact command receipts and raw HTTP evidence are in:
`/Users/yanfenma/workspace/artifacts/workflow-unattended-mechanisms-20261027/svc-current-input-implementation`.
The final handoff receipt in that directory binds the actual commit and copied
binary; this report does not embed a self-referential commit hash.

## Implementation and boundary

The new thin HTTP handler validates duplicate/unknown query fields and delegates
to small request, selection and body modules. The read reuses
`authorized_snapshot` (REPEATABLE READ), full visibility and existing denial
SecurityAudit handling. It calls `build_full` for the current detail value.

Entry selection uses the introducing Event and exact recorded Submission. Same
Visit Context revisions retain separate original/current Context IDs. Recent
windows use limit+1; explicit IDs are parsed, deduplicated and sorted. Bodies use
the existing JCS serializer and SHA-256, UTF-8 byte boundaries, mandatory expected
hash and bounded inline allocation. Missing/foreign references are masked;
internal inconsistency, storage unavailable and stale/body-changed conflicts
remain distinct failures.

All Event source/target/Context/Submission instance-membership checks are already
performed by `authorized_snapshot` through `load_base` and `validate_base` before
selection. The database also has deferred composite Event reference FKs.
Cross-instance UUIDs are not legal merely because the foreign row exists: the
new isolated test verifies both source-Visit and Context cases fail COMMIT with
SQLSTATE `23503`, leaving the prior facts unchanged. No constraint was disabled
and no immutable record was overwritten to fabricate an HTTP fault case.

## Executed evidence

The isolated database is
`svc_workflow_test_current_input_20261027_9db74db8` at `127.0.0.1:55432`.
The new tests enforce that hostname/port and database-name prefix before setup.
HTTP calls use actual TCP Axum listeners and runtime-generated RSA/JWT keys with
a local HTTP JWKS server. Responses are retained as exact `.response.raw` bytes
plus method/path, actor, status, parsed value, wire byte count and SHA-256.
Credentials are not written to these records. The PostgreSQL interruption test
terminates only its observed blocked read in this same isolated database.

| Evidence | Observed result |
| --- | --- |
| Existing query baseline | 38 passed, exit 0 |
| Existing Submission HTTP baseline | 12 passed, exit 0 |
| Initial endpoint RED | 6 failed, exit 101; valid fixtures reached missing route |
| Combined-entry RED | 1 failed, exit 101; exact source event-data key mismatch |
| Nullable unknown-entry RED | 1 failed, exit 101; SQL NULL event data wrongly decoded as storage failure |
| New service HTTP tests, final | 25 passed, exit 0 (`final-http-stable-jwks.receipt.json`) |
| Final focused regressions | Query 38, Submission HTTP 12, model3 activation 9 and library 212 passed; each exit 0 (`regression-*.receipt.json`) |

The first GREEN attempt also had a fixture error: Owner detail was assumed to
change when a target Principal was disabled, but existing actor blocking has
precedence. The corrected test uses actual Domain drift and separately checks
current-assignee target-eligibility drift. The first concurrency probe searched
only for a `FROM` clause; the real blocked query used a `JOIN`. These fixture
failures are not counted as product defects. A later initial setup read returned
401 `unknown_kid`; the CTI-owned JWKS fixture was changed from the shared raw TCP
stub to a proper HTTP server. Authentication implementation was not modified;
the exact cause of that one 401 is not asserted beyond the recorded observation.

## Acceptance mapping and remaining integration

| Acceptance | Service evidence and limit |
| --- | --- |
| ACC-CTI-001 | Real Owner/current-assignee/creator, restricted/unrelated/disabled/global-only actor reads; validation; business-row and success-audit counts; denial audit failure remains unavailable. |
| ACC-CTI-002 | Real legacy create/revise/transition/combined/import/admin override and model3 TASK/null Submission; bounded append-only normalization/unknown fixtures; duplicate/broken entry failures and live FK controls. |
| ACC-CTI-003 | Ten real ADVANCE/RETURN cycles, old correction retrieval, >8/>4 windows, >16 references; exact entry despite newer equal-timestamp submissions; deterministic UUID order and foreign-ID masking. |
| ACC-CTI-004 | Long Context/instructions/outgoing schema/Submission, escaping, Unicode and empty JSON; omitted/truncated descriptors; full byte/hash reconstruction; invalid offsets and stored digest corruption. |
| ACC-CTI-005 | New HTTP snapshot blocked while real Context revision commits; stale coordinates; same-triple body drift; fresh permission check; real PostgreSQL backend interruption and recovery with no business write. The baseline also exercises the shared snapshot mechanism with concurrent transitions. |
| ACC-CTI-006 | **Not executed by this service writer.** Separately accepted Broker implementation, actual tool parity, successor dispatch/ingress and external-control cutoff belong to the parent integration run. |

Service read tests do not prove semantic QA correctness, workflow completion,
production adoption or unattended reliability. Append-only fault fixtures prove
bounded integrity handling; they are not successful business transitions.
No new scheduler or manual-rescue loop is introduced. Independent exact-head
review and the separate consumer/runtime canary remain required for the claimed
integration scope. No production effect is claimed.

## Reproduction

Create a new isolated PostgreSQL database with prefix
`svc_workflow_test_current_input_20261027_`, set `TEST_DATABASE_URL` to that database
on `127.0.0.1:55432`, and optionally set `CTI_HTTP_EVIDENCE_DIR` to an empty artifact
directory. Run `cargo test --test 41_current_task_input -- --test-threads=1`.
The test target applies the repository's existing migrations and uses distinct
fixture UUIDs. Run the existing query/Submission/model3/lib commands listed in
the machine receipts for the focused regressions. Do not point these commands at
production or a shared non-test database.
