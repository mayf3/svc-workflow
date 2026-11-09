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

## 6. 当前 main 重 PREFLIGHT/base-impact 与活性父本对账 (2026-11-09)

Sections 1–5 above retain the historical `bfc28c1` preparation record, including
its unexecuted RED predictions. Their four-SQL-change execution plan is no
longer a current-main work order. This bounded update records local Git source
observations only; it does not execute that plan or accept the proposal.

```text
EXECUTION_MANDATE = user's B1/B2 docs-only blocker closure; no production access
PARENT_HEAD = c64642467ceb2b410d96a8ce07b72dbb4e04b98e
HISTORICAL_BASE_HEAD = bfc28c15320acab8cf7d7a36128cd5c9db9a1b1a
CURRENT_BASE_HEAD = local origin/main e4efe23238dad67d0726da34e32a2c1bf8b27714
REMOTE_FRESHNESS = NOT_VERIFIED (ls-remote failed: proxy 127.0.0.1:7890 unreachable)
AUTHORITY_ACTION = AMEND (named proposed target; metadata and report only)
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = DURABLE
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO (this proposed updated_at Spec)
IMPLEMENTATION_READY = NO
PRODUCTION_APPLY_AUTHORITY = none
ACCEPTANCE = NOT_PERFORMED
DONE_WHEN = B1 metadata and B2 bounded fact correction reviewed; one docs commit
EXPANSION_TRIGGER = implementation, acceptance, or production access (forbidden)
NEXT_ACTION = RE_PREFLIGHT before any future acceptance or implementation
```

Static base-impact readback at `CURRENT_BASE_HEAD` (the three helpers are under
`src/store/postgres/workflow_instance_repository/`):

| Surface | Observed current-main state | Remaining implication |
|---|---|---|
| `transition_helpers.rs:254` | `updated_at = now()` already present | Do not reapply the historical transition SQL hunk. |
| `combined_helpers.rs:121` | `updated_at = now()` already present | Do not reapply the historical combined SQL hunk. |
| `revise_transaction.rs:479-480` | State-version clause has the comma; `updated_at = now()` already present | Do not reapply the historical revise SQL hunk. |
| `tests/34_updated_at_write_paths.rs` | Blob `0195831ef7532513db30be323e29dda6261425fc` equals both `e068dd0621068201dc1794133a7952b933e18dfe` and #134 landing commit `64fb848c1590f1089ef31267f449cba5e499d71d` (merged by `e4efe23`) | Already landed; blob identity is not an executed test result. |
| `repair_transaction.rs:531-533` | Projection UPDATE still omits `updated_at` | Repair remains an outstanding CTR-UAM-002 obligation, subject to authority and fresh implementation PREFLIGHT. |

The proposal's recorded parents are historical at this base. Frontmatter and
reciprocal successor links show Architecture `V0_4_3 → V0_4_4 → V0_4_5 → V0_4_6
→ V0_4_7` and Product `V8 → V9 → V10 → V11 → V12 → V13`. The accepted,
non-superseded successors recorded in this snapshot are
`docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_4_7.md` and
`docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V13.md`; both retain
`implementation_authority: none` and `production_apply_authority: none`.
Their recorded paired-lifecycle effectiveness condition is not independently
verified across repositories here. V0_4_3 and V8 are now `status: superseded`.
This metadata-only correction leaves the proposal's historical `governed_by`
unchanged; before acceptance, reconcile it against the active parent Contracts
and review any required proposal amendment. Existing source cannot supply
missing Product Authority or retroactively accept this proposal.
Full frontmatter-schema validation still reports the pre-existing strict-ID
pattern mismatch for `governed_by[1] = SVC_WORKFLOW_ARCHITECTURE_V0_4_3`;
the required B1 fields are present, but no full-schema PASS is claimed.

Re-PREFLIGHT the remaining repair and the full CTR-UAM-001–007 evidence matrix
against a fresh base and reconciled authority before future implementation.
The named behavior cases, negative controls, other path families, created_at,
untouched historical rows and rollback obligations remain unverified here.
No database, build, or functional test was run in this correction; current
conformance remains NOT_VERIFIED. Neither §3's historical main RED predictions
nor #1301's candidate-line RED/GREEN are converted into current-main results.
