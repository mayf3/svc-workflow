# UPDATED_AT Main-Bound Minimal Integration Preparation V1

```text
DATE = 2026-10-27
STATUS = PLAN ONLY — nothing in this document has been implemented or executed on a main-bound branch
TASK_LINE = business 7cd02846-67e1-4458-a096-297a7c8fd32f / formal parent 8121ef1d-0138-4431-b4f8-95dd86b24fe5
BASE = main bfc28c15320acab8cf7d7a36128cd5c9db9a1b1a (fresh ls-remote verified)
PREREQUISITE = docs/specs/SVC_WORKFLOW_INSTANCE_UPDATED_AT_MAINTENANCE_V1.md must be ACCEPTED before this plan executes (main authority branch carries only accepted specs; implementation_authority activates on acceptance)
```

## 1. Verified net delta (main bfc28c1 → candidate fix head e068dd06)

The full `main..e068dd06` diff spans 43 files (9 commits carrying #80
auditor/provisioning, #82 identity convergence, #84 D1 RETURN projection +
READY_CONTRACT, input-contract removal, SDK changes, release scripts, two
trusted-fleet plan JSONs). Per the no-carryover instruction, all of it is
excluded. The verified minimal relevant subset is exactly:

```text
M  src/store/postgres/workflow_instance_repository/transition_helpers.rs   +1 line  (updated_at = now(),  in update_instance UPDATE)
M  src/store/postgres/workflow_instance_repository/combined_helpers.rs    +1 line  (updated_at = now(),  in update_instance UPDATE)
M  src/store/postgres/workflow_instance_repository/revise_transaction.rs  +2/-1    (add corrected clause: "…workflow_state_version = $2, \" + \"updated_at = now() \\" — WITH the comma; e068 blob db8c451, main blob 2b3166a3)
A  tests/34_updated_at_write_paths.rs                                     new file (e068 version incl. revise-only case)
```

Hunk texts are exactly the `git diff bfc28c1..e068dd06 -- <three files>` output
recorded in the #1301 evidence directory. This is the original three-path
candidate's net delta, not full conformance to the proposed Spec. The proposal
also includes `repair_transaction.rs:531`, which needs the same transactional
timestamp maintenance and its own evidence. The recommended future main-bound
product delta is therefore these three SQL sites plus that fourth repair site;
additional test assertions are required by §4. None is implemented here.

## 2. Fixture dependency verification for tests/34 on main (read-only checks done 2026-10-27)

| tests/34 dependency | On main bfc28c1 | Evidence |
|---|---|---|
| `HttpConfig` literal fields (admission, bind_addr, request_body_max_bytes, request_timeout_seconds, jwks_config, provisioning_config, execution_control, auth_v1_canary_config) | ALL present | src/http/state.rs:41–53 |
| `svc_workflow::http::{router, AppState, ExecutionControlConfig}` | present | src/http/mod.rs, state.rs |
| `common::{create_pool, seed_principal_domain_with_owner, MockJwksServer, v1_token}` | present | tests/common/mod.rs (main is a superset: +43 lines unused by 34) |
| `WorkflowExecutionClass::{Business, NonBusinessTest}` | present | src/domain/enums.rs:152–153 |
| `http::router(AppState::new(pool, &config), &config)` test pattern | established | tests/17_workflow_runtime/http/* on main |
| application `revise_workflow_context` + command struct | present | src/application/workflow_instance/revise.rs, src/domain/workflow_instance/commands.rs |

No dependency on auditor/provisioning-global-role/identity-convergence/
RETURN-projection/READY_CONTRACT/input-contract-removal/SDK changes.
Conclusion: the e068 version of tests/34 is field- and API-compatible with
main bfc28c1 as a new test file.

## 3. Expected failure modes on the main baseline (unexecuted hypotheses)

Static inspection finds no candidate-line missing-comma SQL syntax defect on
main. The expected tests/34 results below remain hypotheses until actually run:

- `revise_only_maintains_updated_at`: expected to fail on the `updated_at > created_at`
  assertion (silent stale timestamp — the actual 7cd defect on main);
- `advance_and_return_maintain_updated_at`: expected to fail on the ADVANCE bump assertion.

This differs from the candidate-line RED (`syntax error at or near
"updated_at"`), which is historical fact from the #1301 evidence run on
c819124c and MUST NOT be presented as a main-baseline observation.

## 4. Future execution plan (after Spec acceptance; NOT executed in this round)

1. After an authorized actor accepts the exact reviewed Spec and it is present
   in the implementation base, create an isolated branch off fresh main. The
   recommended sequencing is this minimal main-bound repair, without merging
   PR #85's unrelated base commits; retain #85 as historical candidate evidence.
2. Author behavior tests mapped to all seven Contracts. Reuse tests/34's two
   cases, then add actual timestamp assertions for TERMINATE, combined and
   repair plus failed/conflict/aborted/read/replay controls and created_at
   preservation. Existing tests/21 and tests/22 are functional coverage only:
   neither has updated_at assertions at bfc28c1. They cannot prove this repair
   and are not predicted timestamp-defect RED tests.
3. RED: execute the targeted new assertions on the pre-fix main tree and record
   observed failures, including any failure unlike §3's predictions. A harness
   failure is not evidence of the intended defect.
4. GREEN: apply only the four named SQL maintenance changes, rerun affected
   assertions and directly dependent functional regressions (tests/17 relevant
   context_revision/combined cases, tests/03, tests/21 and tests/22). Inspect
   the other six path families against the Spec matrix. Report missing evidence
   as NOT_VERIFIED; no whole-Spec PASS from two tests/34 cases. If delivery is
   split, repair remains an explicit outstanding CTR-UAM-002 obligation.
5. Environment: `TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/<new-dedicated-db>`
   (fresh dedicated database per run; production `svc_workflow` /
   `svc_workflow_auditor` never connected; same isolation pattern as #1301).
6. Build: `cargo build --locked --release` on a clean tree (build.rs clean-tree
   gate); record command/output/exit/HEAD/artifact sha256 — a NEW run; the
   historical e068 artifact (c73e2415…) and its test logs are #1301 facts and
   are never backfilled onto a main candidate.
7. Independent implementation review binds the fresh main base, four named SQL
   sites and matrix-mapped tests, excluding unrelated branch changes. Runtime
   adoption and rollback require a separately bound deployment profile; source
   revert, build and isolated test success cannot substitute for those steps.

All seven Contract rows currently have `MAIN_BOUND_EVIDENCE = NOT_RUN`.
Historical e068/a1 evidence remains useful for its original candidate and
environment only. Source/main readiness, full-Spec conformance and production
acceptance are separate claims.

## 5. Explicit non-claims

- No new-main test PASS is claimed (nothing ran on a main-bound branch yet).
- The #1301 RED/GREEN/clean-build evidence and the independent a1 validation
  remain historical facts attached to e068dd06/c819124c; they are reused as
  history, not as verification of this plan.
- The production/merge status of 7cd (merge_to_main15 blocked by
  merge_not_verified) and of 8121 (merge_to_main18; separate M3 production
  gap) is unchanged by this docs round; this plan is not production success.
