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
VERIFICATION_STATE = EXECUTED for ACC-ACB-001..005 structural scope and
                     ACC-ACB-006 new-binary + old-binary downgrade leg
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
src/domain/definition/acceptance_binding.rs                          (new; pure domain core + 25 unit tests)
src/domain/definition/mod.rs                                         (module registration)
src/application/definition/lifecycle/publish.rs                      (CTR-ACB-001 publish gate)
src/application/definition/lifecycle/validation.rs                   (CTR-ACB-001 validate-draft gate)
src/application/definition_governance/diagnostics/catalog.rs         (2 closed-catalog projection codes)
src/store/postgres/workflow_instance_repository/validation_helpers.rs (CTR-ACB-002 create gate helper)
src/store/postgres/workflow_instance_repository/create_transaction.rs (CTR-ACB-002 gate call, before fact writes)
src/store/postgres/workflow_instance_repository/transition_transaction.rs (CTR-ACB-003 binding predicate)
src/store/postgres/workflow_instance_repository/combined_transaction.rs   (CTR-ACB-004 fail-closed backstop)
tests/41_acceptance_context_binding_v1.rs                            (new; 35 integration tests)
tests/42_acceptance_binding_downgrade_gate.rs                        (new; CTR-ACB-006 old-binary leg)
docs/reports/evidence/20261027-acb-v1/*.log                          (executed evidence originals)
```

No dependency changes; no migrations; no HTTP routes added; no Context
mutation path added; model3 combined rejection (CTR-VAI-012) untouched.

## 3. TDD — RED then GREEN

- RED (tests compiled against the unmodified base code, black-box):
  **11 passed / 24 failed** — every publish/create/transition/combined gate
  test failed exactly where the contract requires a gate that base lacks.
  Original: `evidence/20261027-acb-v1/red-run-tests-vs-base.log`
  (full run captured at execution time; command below).
- GREEN (after implementation): **35/35 passed**
  (`evidence/20261027-acb-v1/green-run-tests-after-impl.log`).
- Domain unit tests: **25/25 passed** (`cargo test --lib acceptance_binding`;
  these compile-fail on base because the module does not exist there — that
  is their RED).

Commands (both runs, from the worktree root):

```
export TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/svc_workflow_acb_impl_27oct
cargo test --test 41_acceptance_context_binding_v1 -- --test-threads=1
```

## 4. ACC → evidence map (executed 2026-10-27, exact branch/binary below)

| ACC row | Contracts | Result | Evidence |
|---|---|---|---|
| ACC-ACB-001 (annotation matrix, publish/validate, digest binding) | CTR-ACB-001 | **PASS (structural, service+HTTP)** | `tests/41`: acc1_valid_root_annotation_on_model3_primary_advance_publishes; acc1_annotation_is_bound_into_canonical_digest; rejects: changed/extra/missing pointer, nested (with valid root and nested-only), malformed values (null/true/string/array/wrong types), RETURN, TERMINATE, Legacy-DRAFT-entry, model2, validate_draft parity; `http_roundtrip::http_publish_rejects_annotation_on_return_transition` (real HTTP publish → 422, version stays DRAFT). RED originals show all of these publishing/failing-wrongly on base. |
| ACC-ACB-002 (Create catalog gate, zero delta, no metadata substitution) | CTR-ACB-002 | **PASS (structural)** | `tests/41`: acc2_valid_catalog_creates_instance_with_immutable_context; acc2_missing_catalog_rejected_zero_delta_with_failure_receipt (422 `context_validation_failed` receipt persisted; instance/visit/submission/event/context counts unchanged); acc2_create_rejects_malformed_catalog_shapes (empty, duplicate, case-sensitive-distinct legal, bad charset, 65-char id, blank/missing/4097-byte description, extra member, non-object entry, non-array, 129 entries; 64-char id + 4096-byte description boundary legal); acc2_metadata_cannot_substitute_for_formal_catalog; acc2_unannotated_definition_has_no_catalog_gate. |
| ACC-ACB-003 (binding predicate, cross-Instance isolation) | CTR-ACB-003 | **PASS (structural)** | `tests/41`: acc3_exact_current_uuid_and_full_pass_set_advances_once (exactly one visit/submission/event, no context delta); acc3_same_ids_other_instance_context_uuid_cannot_exchange_reports (both directions); acc3_wrong_uuid_shapes_rejected (arbitrary UUID, omitted, non-string, uppercase re-encoding); acc3_missing_unknown_substituted_and_duplicate_ids_rejected; acc3_non_pass_verdicts_rejected (FAIL/lowercase/numeric/missing verdict/missing id/non-array); acc3_third_instance_with_different_catalog_rejects_foreign_id_sets; acc3_structurally_valid_but_semantically_wrong_evidence_passes_gate (Spec's canary-evidence example — passes structurally, QA truth boundary pinned); acc3_annotated_advance_without_payload_still_submission_required; acc3_unannotated_advance_does_not_require_report. |
| ACC-ACB-004 (combined fail-closed; RETURN/TERMINATE unblocked) | CTR-ACB-004 | **PASS (structural)** | `tests/41`: acc4_combined_with_annotated_schema_fails_closed_zero_delta (Spec-sanctioned isolated drift fixture; resolvable transition; zero workflow-fact delta); acc4_model3_combined_still_rejected_as_legacy_command; acc4_return_executes_without_all_pass_after_rejected_report; acc4_terminate_executes_without_all_pass. |
| ACC-ACB-005 (CAS/replay/conflict, receipt discipline) | CTR-ACB-005 | **PASS (structural; injected storage-failure leg NOT_RUN)** | `tests/41`: acc5_stale_cas_conflicts_with_zero_delta_and_receipt; acc5_success_replay_returns_original_without_gate_or_new_facts; acc5_failure_then_replay_returns_original_failure_zero_extra_facts (+ changed-payload same-key IdempotencyConflict). Existing UNKNOWN/commit-interruption semantics are unchanged code (T69 budget machinery untouched) and are covered by `tests/17_workflow_runtime` (451 passed, incl. receipt_stability); the injected storage/commit interruption canary was **NOT_RUN** here. |
| ACC-ACB-006 (compat + downgrade) | CTR-ACB-006 | **PASS (new binary + old binary leg); fleet-activation inspection NOT_RUN (operator obligation)** | Unannotated compatibility: acc2_unannotated_definition_has_no_catalog_gate, acc3_unannotated_advance_does_not_require_report, plus full regression set below. Old-binary leg: `tests/42` with `ACB_OLD_BINARY=/tmp/acb-old-svc-workflow` (built from base `371ea815`, sha256 `4d56717e6a71af0ae50f02912b00c375a09a9555bde77ede30f80a28acd782bb`): real publish → real `DefinitionService::revoke_version` by enabled DOMAIN_OWNER → REVOKED readback → new binary refuses (`DefinitionVersionRevoked`) → old binary spawned as real HTTP server on the same isolated DB also refuses with `definition_version_revoked` (log: `evidence/20261027-acb-v1/old-binary-downgrade-gate.log`). New binary sha256 at commit: see §6. |

Not done by design (out of this task's mandate): live canaries with real
HTTP serving require a separate operation mandate; no production deployment
or enablement; no business Workflow writes. ACC-ACB-006's pre-activation
operator obligation (inspect existing versions for the reserved key before
any opt-in version is served) remains an operator duty and was NOT executed.

## 5. Regression (same environment, same branch)

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

Single candidate target cache: `<worktree>/target` (shared by both builds;
dependency set identical — no dependency changes in this PR).

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

## 7. Shortest independent re-run (canary-grade)

```
# 0) isolated DB (never the default localhost:5432/svc_workflow)
export PGPASSWORD=postgres
psql -h 127.0.0.1 -p 55432 -U postgres -d postgres \
  -c "CREATE DATABASE svc_workflow_acb_recheck_<suffix>"
export TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/svc_workflow_acb_recheck_<suffix>

# 1) structural suite (ACC-001..005)
cargo test --test 41_acceptance_context_binding_v1 -- --test-threads=1

# 2) downgrade gate (ACC-006 old-binary leg)
git worktree add --detach /tmp/acb-base-src 371ea8155d8fa3025ef26af75d128f3047736d6d
(cd /tmp/acb-base-src && CARGO_TARGET_DIR=$PWD/target cargo build --bin svc-workflow)
cp /tmp/acb-base-src/target/debug/svc-workflow /tmp/acb-old-svc-workflow
git worktree remove /tmp/acb-base-src
export ACB_OLD_BINARY=/tmp/acb-old-svc-workflow
cargo test --test 42_acceptance_binding_downgrade_gate -- --test-threads=1

# 3) real HTTP publish→create→transition canary (new binary, isolated DB):
#    tests/41 http_roundtrip::http_publish_create_transition_roundtrip_with_binding
#    already drives the real router; for a live-server canary:
WORKFLOW_PORT=18989 DATABASE_URL=$TEST_DATABASE_URL \
WORKFLOW_JWKS_URL=<jwks> WORKFLOW_JWT_ISSUER=auth-service \
WORKFLOW_JWT_AUDIENCE=svc-workflow AUTH_V1_CANARY_ENABLED=true \
AUTH_V1_CANARY_WRITE_ENABLED=true AUTH_V1_CANARY_ALLOWED_CLIENT_ID=<client> \
WORKFLOW_PROVISIONING_PRINCIPAL_IDS=<admin-uuid> \
./target/debug/svc-workflow
```

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
