# SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1 — Independent Code Audit (r1)

> Date: 2026-09-02 · Auditor: independent coordination agent (not the implementation author role)
> Subject: impl branch `impl/visit-activation-v1` @ `d799bf2` + remediation `90acfa3`
> Authority: accepted `SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1` (PR #23, merged `f2a11cd`) under accepted `SVC_WORKFLOW_ARCHITECTURE_V0_4_0` (PR #21, merged `b5bb7ec`) + Owner ruling `KEEP_ACCEPTED_V6`.

## Verdict

```text
CODE_AUDIT = PASS (after 3 findings remediated in-round)
SPEC_COMPLIANCE = PASS
TESTS = PASS (workspace green on run-scoped fresh DB; 1 documented pre-existing env-dependent failure identical on pristine base)
```

## Mechanical verification matrix (CTR-VAI-001..014)

| CTR | Obligation | Evidence (file:line at 90acfa3) | Result |
|---|---|---|---|
| 001 | semantic model 3 + DB-enforced Instance/Definition equality | migrations/0023:27,33,49,53,58 (CHECK (1,2,3), uq+composite FK, immutability trigger); create_transaction.rs inserts instance model column | PASS |
| 002 | TASK node encoding, no Minimal aliasing | enums.rs TASK variant; 0019 tests updated to (1,2,3); draft/publish dispatch arm 3 | PASS |
| 003 | immutable activation facts, exactly-one | 0023:99 UNIQUE(node_visit_id), :102 kind-conditional CHECK, 3× fn_prevent_modification triggers; test `activation_facts_are_immutable` | PASS |
| 004 | create atomic closure | create_transaction.rs:269 (entry lookup), :298 (owner type check), :409-424 (insert_activation, same tx, NOW()-authored initial) | PASS |
| 005 | transition closure | transition_transaction.rs:515 (close required), :575 (target activation; TERMINAL none) | PASS |
| 006 | cancel/archive | cancel_transaction.rs:340-345 (close, reason CANCELLED); archive_transaction.rs:305-314 (fail-closed ActiveActivationExists → 409) | PASS |
| 007 | admin move/terminate | override_transaction.rs:313-318,338,410-418 (v1 target-kind check, owner check, closure reasons ADMIN_MOVE/ADMIN_TERMINATE, target activation) | PASS |
| 008 | wake | wake.rs:54-56 (scope+direct token+GLOBAL_SCHEDULER_READ fail-closed); wake_transaction.rs:355-369 (5 no-op classes incl. VERSION_MISMATCH, ALREADY_DUE), applied path = eligibility fact + 1 version + 1 Event + receipt, replay via camelCase receipt body | PASS |
| 009 | due poll | dispatch_intents.rs (limit 1-100 422); query_dispatch_intents.rs:54 (role check in same snapshot), :89 (closed excluded), singular due predicate `<= now()`, exactly 7 fields (serde camelCase) | PASS |
| 010 | GLOBAL_SCHEDULER_READ role value | provisioning/mod.rs:54 const; provisioning handlers accept it (2 sites); no binding created by code/migration | PASS |
| 011 | v1 graph validator | visit_activation_validator.rs wired at draft_graph.rs:207 + publish.rs:73; 14 unit tests incl. cycle/entry/unreachable fixtures | PASS |
| 012 | legacy mutual protection + rebuild validation | revise_transaction.rs (typed 422), combined_transaction.rs (typed 422), rebuild_transaction.rs:142 (activation consistency), legacy paths untouched | PASS |
| 013 | compatibility | no change to existing list/worklist/error surfaces (diff audit: only additive routes + error arms); EXPECTED_MIGRATION_VERSION=23; no dispatchable flag anywhere (grep = 0) | PASS |
| 014 | wake idempotency + audit | compute_wake_request_hash (JCS envelope); receipt replay/conflict; attempt audit on no-op; security audit on role denial | PASS |

## Findings (all remediated in commit 90acfa3)

1. **[P1, wire] Revise rejection fresh-path leaked 500** — the v1 rejection returned
   `InternalConsistency` (500 `internal_consistency_error`) on the fresh path while the
   durable receipt recorded 422 `legacy_command_not_supported_for_semantic_model`.
   Fixed with a typed `ReviseWorkflowContextError::LegacyCommandNotSupported` (422) +
   combined-error conversion; locked by test assertions on `revise_error_code`/`revise_error_label`.
2. **[P2, wire] Combined revise-and-transition rejection used 409** `transition_not_applicable`
   instead of the Spec's 422 label. Fixed with `ReviseContextAndTransitionError::LegacyCommandNotSupported`.
3. **[P2, audit] Wake scheduler-role denial wrote no durable audit**, violating
   CTR-ARCH-039 (authenticated-denied protected activation operations). Fixed: a
   non-sensitive `workflow_security_audits` row (`WAKE_DISPATCH_INTENT_DENIED`) is written
   on denial.

## Test evidence

- `tests/28_visit_activation_v1.rs`: 9/9 (ACC-VAI-001..011 executable matrix).
- `visit_activation_validator_tests`: 14/14.
- Full `cargo test --workspace` on run-scoped fresh DB: **181 passed / 1 failed** — the
  single failure (`27_trusted_fleet_principal_cutover_v1::empty_disposable_databases_fail_loud_with_zero_workflow_writes`)
  is environment-dependent (requires `TEST_WORKFLOW_DATABASE_URL`/`TEST_AUTH_DATABASE_URL`
  from the conformance script) and fails **identically on pristine base `efdfb7e`**
  (verified in the audit): documented pre-existing baseline per Spec §6.
- `git diff --check` clean.

## Scope conformance

- No `dispatchable` / `dispatch_blocked_reasons` / `dispatchableOnly` anywhere (Owner
  ruling KEEP_ACCEPTED_V6) — mechanical grep = 0.
- No Grant/principal/credential/allowlist changes; role value accepted in provisioning
  validation only, no binding provisioned.
- `implementation_authority: contracts` exercised exactly within the Spec scope;
  `production_apply_authority` remains none; migration applied only to disposable local
  test databases (dropped after runs).
