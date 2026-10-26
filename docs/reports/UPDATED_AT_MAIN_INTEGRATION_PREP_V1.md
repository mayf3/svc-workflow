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
recorded in the #1301 evidence directory; applying them to bfc28c1 is the
entire production-code delta.

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

## 3. Expected failure modes on the main baseline (for the future RED run)

On unpatched main there is **no SQL syntax error** (the broken clause does not
exist there — revise succeeds):

- `revise_only_maintains_updated_at`: FAILS on the `updated_at > created_at`
  assertion (silent stale timestamp — the actual 7cd defect on main);
- `advance_and_return_maintain_updated_at`: FAILS on the ADVANCE bump assertion.

This differs from the candidate-line RED (`syntax error at or near
"updated_at"`), which is historical fact from the #1301 evidence run on
c819124c and MUST NOT be presented as a main-baseline observation.

## 4. Future execution plan (after Spec acceptance; NOT executed in this round)

1. New isolated branch off fresh main; dedicated worktree; user root untouched.
2. Apply §1's exact 4-file delta as one commit referencing the accepted Spec ID.
3. RED: run tests/34 on the pre-fix tree — expect §3's two assertion failures.
4. GREEN: apply the delta; run tests/34 (expect 2/2), plus on-surface suites:
   tests/17 context_revision + combined filters, tests/03, tests/21
   (cancel/archive), tests/22 (repair — expected to stay RED only if the
   repair gap is in scope per Spec §5.3; otherwise out of scope by declaration).
5. Environment: `TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/<new-dedicated-db>`
   (fresh dedicated database per run; production `svc_workflow` /
   `svc_workflow_auditor` never connected; same isolation pattern as #1301).
6. Build: `cargo build --locked --release` on a clean tree (build.rs clean-tree
   gate); record command/output/exit/HEAD/artifact sha256 — a NEW run; the
   historical e068 artifact (c73e2415…) and its test logs are #1301 facts and
   are never backfilled onto a main candidate.
7. Independent review PR (normal, non-draft only after acceptance of the Spec;
   draft acceptable for early review), diff vs main = exactly §1.

## 5. Explicit non-claims

- No new-main test PASS is claimed (nothing ran on a main-bound branch yet).
- The #1301 RED/GREEN/clean-build evidence and the independent a1 validation
  remain historical facts attached to e068dd06/c819124c; they are reused as
  history, not as verification of this plan.
- The production/merge status of 7cd (merge_to_main15 blocked by
  merge_not_verified) and of 8121 (merge_to_main18; separate M3 production
  gap) is unchanged by this docs round; this plan is not production success.
