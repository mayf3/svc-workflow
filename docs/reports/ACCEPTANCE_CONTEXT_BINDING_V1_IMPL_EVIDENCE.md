# SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1 — Implementation Evidence Record

status: implementation_record (DRAFT — pending independent review; not an acceptance record)
spec: docs/specs/SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1.md (accepted; sha256
`3f705fe8e4f8a72b273b07ab85302478c681fce9a441f3e146d2f7b736932852`)
base: main `371ea8155d8fa3025ef26af75d128f3047736d6d` (accepted integration base)
branch: `codex/workflow-acceptance-impl-20261027`
date: 2026-10-27
environment: isolated PostgreSQL 16.15 at `127.0.0.1:55432`, database
`svc_workflow_acb_impl_27oct` (created for this task; default
`localhost:5432/svc_workflow` never touched). All test runs set
`TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/svc_workflow_acb_impl_27oct`.

## 1. SPEC_COMPLIANCE

```text
SPEC = SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1@accepted (base 371ea815)
IMPLEMENTATION_STATE = IMPLEMENTED (this branch; not merged)
VERIFICATION_STATE = PARTIAL: structural/replay legs of ACC-ACB-001..004
                     executed; ACC-ACB-005 fault/UNKNOWN legs NOT_RUN;
                     ACC-ACB-006 both REVOKED binary legs executed but
                     fleet inspection and independent serving-runtime
                     canary remain PENDING with the root validator
ACCEPTANCE_STATE = unchanged (Spec-level acceptance already recorded; this
                   record only binds implementation evidence)
PRODUCTION_APPLY_AUTHORITY = NONE (no deploy, no enablement, no business Workflow writes)
QUALIFICATION = structural conformance only. Per CTR-ACB-003 the validator
                never decides business truth: a structurally valid PASS with
                semantically wrong evidence passes the gate by design
                (acc3_structurally_valid_but_semantically_wrong_evidence_
                passes_gate pins this). Independent QA owns evidence truth.
```

## 2. Change surface (implementation PR diff)

```
src/domain/definition/acceptance_binding.rs                          (new; pure domain core + 26 unit tests after review fixes)
src/domain/definition/mod.rs                                         (module registration)
src/application/definition/lifecycle/publish.rs                      (CTR-ACB-001 publish gate)
src/application/definition/lifecycle/validation.rs                   (CTR-ACB-001 validate-draft gate)
src/application/definition_governance/diagnostics/catalog.rs         (2 closed-catalog projection codes)
src/store/postgres/workflow_instance_repository/validation_helpers.rs (CTR-ACB-002 create gate helper)
src/store/postgres/workflow_instance_repository/create_transaction.rs (CTR-ACB-002 gate call, before fact writes)
src/store/postgres/workflow_instance_repository/transition_transaction.rs (CTR-ACB-003 binding predicate)
src/store/postgres/workflow_instance_repository/combined_transaction.rs   (CTR-ACB-004 fail-closed backstop)
tests/41_acceptance_context_binding_v1.rs                            (new; 40 integration tests after review fixes)
tests/42_acceptance_binding_downgrade_gate.rs                        (new; CTR-ACB-006 old-binary leg)
docs/reports/evidence/20261027-acb-v1/*.log                          (historical captured output, including filtered excerpts)
```

No dependency changes; no migrations; no HTTP routes added; no Context
mutation path added; model3 combined rejection (CTR-VAI-012) untouched.

## 3. TDD — RED then GREEN

- RED (tests compiled against the unmodified base code, black-box):
  **11 passed / 24 failed**. The 24 failures MIX the intended product-signal
  gate assertions with test-fixture defects of my own harness (graph
  immutability trigger on a mis-ordered fixture, principal/actor mistakes,
  and an HTTP 201-vs-200 assertion). What this evidence PROVES is that the
  gate assertions failed on base where the contract requires a gate; it does
  NOT prove each of the 24 was a product regression. The file
  `evidence/20261027-acb-v1/red-run-tests-vs-base.log` is a FILTERED EXCERPT
  (test names/summaries only); the fuller raw output is preserved by the
  root at `source-first-red-run-observed.log`
  (sha256 `67df787df4f1c0f408b6d2ea8f9f911b7da4296bcd625f80fcb33e4bf87b1985`).
- GREEN (after implementation): **35/35 passed**
  (`evidence/20261027-acb-v1/green-run-tests-after-impl.log`).
- Initial domain unit tests: **25/25 passed** (`cargo test --lib acceptance_binding`).
  The new module is absent on base, but no separately executed baseline
  compilation failure is recorded for these unit tests; their RED is NOT_RUN.

Commands (both runs, from the worktree root):

```
export TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/svc_workflow_acb_impl_27oct
cargo test --test 41_acceptance_context_binding_v1 -- --test-threads=1
```

## 4. ACC → evidence map (executed 2026-10-27, exact branch/binary below)

| ACC row | Contracts | Result | Evidence |
|---|---|---|---|
| ACC-ACB-001 (annotation matrix, publish/validate, digest binding) | CTR-ACB-001 | **PASS (structural, service+HTTP)** | `tests/41`: acc1_valid_root_annotation_on_model3_primary_advance_publishes; acc1_annotation_is_bound_into_canonical_digest; rejects: changed/extra/missing pointer, nested (with valid root and nested-only), malformed values (null/true/string/array/wrong types), RETURN, TERMINATE, Legacy-DRAFT-entry, model2, validate_draft parity; `http_roundtrip::http_publish_rejects_annotation_on_return_transition` (real HTTP publish → 422, version stays DRAFT). The mixed initial RED output is qualified in §3; it does not establish an executed product failure for every listed case. |
| ACC-ACB-002 (Create catalog gate, zero delta, no metadata substitution) | CTR-ACB-002 | **PASS (structural)** | `tests/41`: acc2_valid_catalog_creates_instance_with_immutable_context; acc2_missing_catalog_rejected_zero_delta_with_failure_receipt (422 `context_validation_failed` receipt persisted; instance/visit/submission/event/context counts unchanged); acc2_create_rejects_malformed_catalog_shapes (empty, duplicate, case-sensitive-distinct legal, bad charset, 65-char id, blank/missing/4097-byte description, extra member, non-object entry, non-array, 129 entries; 64-char id + 4096-byte description boundary legal); acc2_metadata_cannot_substitute_for_formal_catalog; acc2_unannotated_definition_has_no_catalog_gate. |
| ACC-ACB-003 (binding predicate, cross-Instance isolation) | CTR-ACB-003 | **PASS (structural)** | `tests/41`: acc3_exact_current_uuid_and_full_pass_set_advances_once (exactly one visit/submission/event, no context delta); acc3_same_ids_other_instance_context_uuid_cannot_exchange_reports (both directions); acc3_wrong_uuid_shapes_rejected (arbitrary UUID, omitted, non-string, uppercase re-encoding); acc3_missing_unknown_substituted_and_duplicate_ids_rejected; acc3_non_pass_verdicts_rejected (FAIL/lowercase/numeric/missing verdict/missing id/non-array); acc3_third_instance_with_different_catalog_rejects_foreign_id_sets; acc3_structurally_valid_but_semantically_wrong_evidence_passes_gate (Spec's canary-evidence example — passes structurally, QA truth boundary pinned); acc3_annotated_advance_without_payload_still_submission_required; acc3_unannotated_advance_does_not_require_report. |
| ACC-ACB-004 (combined fail-closed; RETURN/TERMINATE unblocked) | CTR-ACB-004 | **PASS (structural)** | `tests/41`: acc4_combined_with_annotated_schema_fails_closed_zero_delta (Spec-sanctioned isolated drift fixture; resolvable transition; zero workflow-fact delta); acc4_model3_combined_still_rejected_as_legacy_command; acc4_return_executes_without_all_pass_after_rejected_report; acc4_terminate_executes_without_all_pass. |
| ACC-ACB-005 (CAS/replay/conflict, receipt discipline) | CTR-ACB-005 | **PARTIAL (structural/replay legs executed; injected storage/commit-fault and UNKNOWN-reconciliation legs NOT_RUN)** | `tests/41`: acc5_stale_cas_conflicts_with_zero_delta_and_receipt; acc5_success_replay_returns_original_without_gate_or_new_facts; acc5_failure_then_replay_returns_original_failure_zero_extra_facts (+ changed-payload same-key IdempotencyConflict). Existing UNKNOWN/commit-interruption semantics are unchanged code (T69 budget machinery untouched) and are covered by `tests/17_workflow_runtime` (451 passed, incl. receipt_stability); the injected storage/commit interruption canary was **NOT_RUN** here. |
| ACC-ACB-006 (compat + downgrade) | CTR-ACB-006 | **PARTIAL (unannotated compatibility + new-binary and old-binary REVOKED legs executed; pre-activation fleet inspection and independent serving-runtime canary NOT_RUN — root validator owns them)** | Unannotated compatibility: acc2_unannotated_definition_has_no_catalog_gate, acc3_unannotated_advance_does_not_require_report, plus full regression set below. Old-binary leg: `tests/42` with `ACB_OLD_BINARY=/tmp/acb-old-svc-workflow` (built from base `371ea815`, sha256 `4d56717e6a71af0ae50f02912b00c375a09a9555bde77ede30f80a28acd782bb`): real publish → real `DefinitionService::revoke_version` by enabled DOMAIN_OWNER → REVOKED readback → new binary refuses (`DefinitionVersionRevoked`) → old binary spawned as real HTTP server on the same isolated DB also refuses with `definition_version_revoked` (log: `evidence/20261027-acb-v1/old-binary-downgrade-gate.log`). New binary sha256 at commit: see §6. |

The source writer did not execute the independent serving-runtime canary;
the root validator has the separate isolated execution mandate. No production
deployment or enablement is authorized. ACC-ACB-006's pre-activation
operator obligation (inspect existing versions for the reserved key before
any opt-in version is served) remains an operator duty and was NOT executed.

## 5. Regression (same environment, same branch — PRE-COMMIT qualification)

All rows below were executed on the working tree BEFORE commit `47947a5`
existed; the content matched what became `47947a5`, but these are not
exact-commit runs. The only exact-commit executions so far are the
post-`44b60fe` runs of tests 41/42 and lib (see §9 and the fix-round
record). Full-suite re-execution at the final head is the independent
validator's call, not re-claimed here.

`evidence/20261027-acb-v1/regression-run.log`:

```
03_context_revision_constraints   4 passed
05_submission_constraints         3 passed
13_definition_service_tests       9 passed
14_definition_lifecycle_tests     6 passed
19_domain_owner_definition_governance 25 passed
36_execution_control_v1           6 passed
17_workflow_runtime             451 passed   (CAS/Receipt/RETURN/compat)
28_visit_activation_v1            9 passed
29_model3_authoring_conformance   1 passed
32_definition_publish_admission   4 passed
41_acceptance_context_binding_v1 35 passed
lib (unit incl. 25 new)         237 passed
```

## 6. Build artifacts

Historical first-round build commands are recorded below, not a recipe for
the current hash-pinned test42. That test now uses the archived binary in §9.
The first round shared `<worktree>/target`; every final candidate must be
rebuilt from its clean source commit and copied to a stable artifact path.

```
# old binary (pre-implementation base 371ea815, used for the downgrade leg)
git worktree add --detach /tmp/acb-old-binary-src 371ea8155d8fa3025ef26af75d128f3047736d6d
cd /tmp/acb-old-binary-src
CARGO_TARGET_DIR=<worktree>/target cargo build --bin svc-workflow
cp <worktree>/target/debug/svc-workflow /tmp/acb-old-svc-workflow
sha256: 4d56717e6a71af0ae50f02912b00c375a09a9555bde77ede30f80a28acd782bb

# new binary (this branch)
cd <worktree> && cargo build --bin svc-workflow
sha256: see PR description (recorded at the exact commit)
```

## 7. Reproducing this run's structural tests

The final test42 intentionally accepts only this task's already-owned
`127.0.0.1:55432/svc_workflow_acb_impl_27oct` database and the archived
`b2e4736f` executable with SHA256
`d97aa01913b9324675af45b2384075bd542c4c0d91d8fedc0e463c6857ec20d5`.
An arbitrary database suffix or a rebuilt base executable is rejected.
The following commands add test fixtures to that database; they do not
create/reset it. Run from the candidate worktree after its writer has stopped.

```sh
export TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/svc_workflow_acb_impl_27oct
export ACB_OLD_BINARY=/Users/yanfenma/.local/services/svc-workflow/releases/b2e4736f4f7e6350bcf1db44e016d583b20e2b21/svc-workflow
python3 - <<'PY'
import subprocess
import tempfile
from pathlib import Path
logs = Path(tempfile.mkdtemp(prefix="acb-recheck-"))
for suite in ["41_acceptance_context_binding_v1", "42_acceptance_binding_downgrade_gate"]:
    log = logs / (suite + ".log")
    with log.open("wb") as output:
        result = subprocess.run(["cargo", "test", "--test", suite, "--", "--test-threads=1"],
                                stdout=output, stderr=subprocess.STDOUT)
    print(suite, "cargo_exit=", result.returncode, "raw_output=", log)
    if result.returncode:
        raise SystemExit(result.returncode)
PY
```

These tests are source conformance evidence, not the complete independent
six-ACC serving-runtime acceptance. The separately prepared independent
harness is at
`/Users/yanfenma/workspace/artifacts/workflow-acceptance-binding-20261027/independent-canary-harness-r3/`.
Its README requires a completed source/build binding, a fresh dedicated
database, minimal child environment, fault cleanup and original-key
reconciliation. Its actual run and result remain separate evidence.

## 8. Implementation notes / reviewer pointers

- The annotation is validated strictly at three layers: publication
  (`validate_graph_annotation_placement`), runtime transition
  (`inspect_submission_schema` + report predicate), and a fail-closed
  any-depth backstop on combined revise-and-transition. A malformed present
  annotation never means absence anywhere.
- Create gate triggers on any-depth annotation presence across the version's
  transition schemas, then validates only the fixed `/acceptanceCriteria`
  pointer of the context payload; metadata cannot substitute.
- Runtime catalog/report failures are deterministic 422s
  (`submission_validation_failed`) with failure receipts and zero
  workflow-fact delta; missing/corrupt authoritative Context linkage is an
  internal-consistency failure (rollback, no receipt).
- HTTP presentation of the two new publish error codes is registered in the
  existing closed graph-diagnostics catalog (presentation projection only,
  never a validator).
- Error details are shape/count-only; catalog descriptions (Context text)
  are never echoed.

## 9. Fix round after the A1 independent review (2026-10-27)

Review bound: `ACB-SOURCE-SNAPSHOT-A1-INDEPENDENT-REVIEW`
(`/Users/yanfenma/workspace/artifacts/workflow-acceptance-binding-20261027/
source-snapshot-a1-review.json`, sha256 `a6b1a8de3f8612a793b33b4a273eae258dd24290b115d253cae9e6b4f5b0398c`),
3 P2 findings, all closed by bounded fixes on the same branch.

### Report corrections (supersede §3/§4 phrasing where they differ)

- The original RED run's **24 failures included test-fixture defects**, not
  all product regressions; the product-signal subset is the publish/create/
  transition/combined gate assertions. The first RED log is preserved as
  history, not as a per-case product claim.
- Draft graphs are seeded as DRAFT rows via SQL fixtures **before** invoking
  the real publish service; publication assertions themselves go through the
  real `publish_version`/HTTP router. Post-publish drift fixtures are limited
  to the Spec-sanctioned ACC-ACB-004 combined definition and the F1 drifted
  instances, which the review's counterexamples prescribe.
- Earlier pipeline summaries establish test **results** but did not record
  individual cargo exit statuses (piped `tail`/`grep` retained shell exits).
  This section's runs record the real cargo return codes.
- Historical `regression-run.log` predates the fix commit; the pre-fix
  broad regression (451 runtime / 237 lib) is not re-claimed for the fix
  delta — no evidence requires it (F1/F2 are confined to annotated paths;
  targeted suites cover them).
- `fact_counts` covers five business row counts plus Receipts; activation
  rows are included only in the F1 counters. Count equality is not complete
  zero-delta proof; the independent validator owns full before/after
  snapshots.

### F1 — deterministic 422 for catalog-content failures on a linked Context

`transition_transaction.rs`: with the authoritative Context row present and
correctly linked, a missing/malformed `acceptanceCriteria` catalog now fails
through `deterministic_failure!` → public 422 `submission_validation_failed`
with the failure Receipt and same-key replay; details are sanitized (no
Context text). The missing/wrong-instance authoritative-linkage branch stays
`InternalConsistency`. The composite FK `fk_instance_current_ctx`
(`current_context_revision_id, workflow_instance_id`) makes a wrong-instance
linkage unrepresentable at the SQL layer (the context-revisions table also
rejects UPDATE), so that branch is defense-in-depth; a new test pins the FK
fence itself. Evidence: `f1_existing_linked_context_with_missing_catalog_is_
deterministic_422` (422 + Receipt + replay + zero business/activation delta),
`f1_existing_linked_context_with_malformed_catalog_rejected` (empty array,
duplicate ids), `f1_wrong_instance_context_linkage_is_structurally_fenced`.

F1 RED honesty (corrected; supersedes any earlier claim of a product RED):
in the pre-fix RED runs both F1 tests failed at a FIXTURE defect — the
drift-instance INSERT hit the `payload_digest` CHECK (23514) before the
product path was ever reached (frozen raw log:
`source-review-fixes-red-observed.log`, sha256
`418ce4643a06b7c12fdcf395e5deb3344107759ffe3b4a0c4e0ed6e2490f2cb9`).
Only the two F2 tests are executed product RED from that round; the FK-fence
test passed as a preservation check. F1 therefore rests on the independent
static review finding PLUS the corrected-fixture GREEN (which does assert
the deterministic 422/Receipt/replay classification through the real
transition path). The old pre-fix product code was NOT re-run to manufacture
a historical F1 RED.

### F2 — reserved annotation rejected anywhere in contextSchema

`acceptance_binding.rs::validate_graph_annotation_placement` now rejects
`contains_annotation_at_any_depth(graph.context_schema)` — root, nested
object, or array, including null values — with the existing
`ACCEPTANCE_BINDING_MISPLACED` diagnostic; the sole legal location remains
the root of a model3 TASK primary ADVANCE submission schema; no business
data keys are reserved. Coverage: new unit test
`context_schema_annotation_rejected_at_any_depth` (4 shapes + silent
absence) and integration `f2_publish_rejects_annotation_in_context_schema`
(publish fails, version stays DRAFT) + `f2_validate_draft_flags_context_
schema_annotation`. RED first: both integration tests failed on pre-fix code
(published cleanly / no binding code), then passed after the fix.

### F3 — old-binary harness isolation and guaranteed cleanup

`tests/42` reworked: explicit isolated `TEST_DATABASE_URL` required (shared
default name and any default-port PostgreSQL refused; `isolated_db_guard_
rejects_shared_defaults`); child spawned via `env_clear` + a closed minimal
env with Forum sync and admission explicitly disabled (`old_binary_command_
isolation_is_closed` asserts the exact env set, pinned-off values, absence
of every integration key, and the owned cwd); owned temp cwd with an empty
`.env` and a copied `migrations/` (the server loads both relative to cwd);
`ChildGuard` RAII kills and reaps the child on every path; the instance is
`NON_BUSINESS_TEST` so no Forum binding/outbox facts are seeded at all.

### Old-binary identity (this round)

Archived pre-implementation binary used, read-only:
`/Users/yanfenma/.local/services/svc-workflow/releases/b2e4736f4f7e6350bcf1
db44e016d583b20e2b21/svc-workflow`, sha256 verified in-run and externally:
`d97aa01913b9324675af45b2384075bd542c4c0d91d8fedc0e463c6857ec20d5`, source
`b2e4736f4f7e6350bcf1db44e016d583b20e2b21` (pre-implementation deployment
line). The earlier session-local binary (`/tmp/acb-old-svc-workflow`, sha256
`4d56717e6a71af0ae50f02912b00c375a09a9555bde77ede30f80a28acd782bb`, built
from base `371ea815`) produced the §4 downgrade evidence and remains
historically valid; this round's leg uses the archived binary.

### Fix-round execution record (real cargo exit codes)

```
41_acceptance_context_binding_v1   40 passed  cargo_exit=0
42_acceptance_binding_downgrade_gate 3 passed cargo_exit=0
29_model3_authoring_conformance     1 passed  cargo_exit=0
32_definition_publish_admission     4 passed  cargo_exit=0
13_definition_service_tests         9 passed  cargo_exit=0
14_definition_lifecycle_tests       6 passed  cargo_exit=0
19_domain_owner_definition_governance 25 passed cargo_exit=0
lib (incl. new unit tests)        238 passed  cargo_exit=0
```
Full raw outputs: `/tmp/acb-final-logs/` (outside the source tree). RED
logs of that round: `/tmp/acb_f1f2_red.log` (first attempt: 5 failures incl.
fixture defects) and `/tmp/acb_f1f2_red2.log` (cargo exit 101; the two F2
failures are the executed product RED — the two F1 failures were the
payload_digest CHECK fixture defect described above; corrected frozen copy:
`source-review-fixes-red-observed.log`).

### Still open (unchanged scope)

Independent full HTTP/fault/UNKNOWN canary, per-command fault injection,
concurrency evidence and the serving-fleet activation gate remain PENDING
with the root independent validator; no production deployment or enablement
is authorized or performed here.
