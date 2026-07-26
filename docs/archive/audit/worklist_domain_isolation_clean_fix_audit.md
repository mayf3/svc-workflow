# Worklist Domain Isolation Clean Fix — Directed Audit Report

**Status:** `SVC_WORKFLOW_WORKLIST_HTTP_ADAPTER_AUDIT_PASS`
**Date:** 2026-07-17
**Auditor:** ZCode directed-review agent (independent clean worktree)

---

## 1. Identity verification (Base / Fix)

| Item | Expected | Actual | Match |
|---|---|---|---|
| Base SHA | `06856ca5247b7074ae1c801625c7853109cdb07c` | `06856ca5247b7074ae1c801625c7853109cdb07c` | ✓ |
| Base tree | `739ef41700532ddbc4e7c1a411a1c6834d947553` | `739ef41700532ddbc4e7c1a411a1c6834d947553` | ✓ |
| Fix SHA | `4fc3b39f05c15f3fe3cd137c779d895aee040673` | `4fc3b39f05c15f3fe3cd137c779d895aee040673` | ✓ |
| Fix tree | `5bbbf698f5f4ff4ef9fe3dd48bf0e821d0019b0a` | `5bbbf698f5f4ff4ef9fe3dd48bf0e821d0019b0a` | ✓ |
| Parent of Fix | `06856ca5247b7074ae1c801625c7853109cdb07c` | `06856ca5247b7074ae1c801625c7853109cdb07c` | ✓ |

All identities consistent. No `WORKLIST_DOMAIN_ISOLATION_CLEAN_FIX_IDENTITY_MISMATCH`.

## 2. Audit agent / worktree / branch

- **Auditor:** ZCode independent directed-review agent.
- **Audit worktrees:** `/tmp/worklist-audit-*` (Fix) and `/tmp/worklist-base-*` (Base), both created as detached clean worktrees via `git worktree add -d`. Removed after audit.
- **Branch under audit:** `fix/h01a-clean-fix` at Fix SHA.
- **Main repo left on:** `fix/h01a-clean-fix` @ `4fc3b39`, working tree clean, **nothing modified, committed, pushed, merged, tagged, or deployed**.

## 3. Implementation modified?

**No.** Audit was strictly read-only: read sources, read tests, ran build/clippy/test, ran read-only SQL experiments against a disposable DB copy. No implementation, test, schema, or migration files were touched in the audited repo.

## 4. Actual diff (base..fix)

Exactly 3 files, all expected:

```
M  src/store/postgres/workflow_instance_repository/query_worklists.rs   (+14)
M  tests/17_workflow_runtime/http/worklists.rs                          (+423 / -3)
M  tests/17_workflow_runtime/query/helpers.rs                           (+13 / -7)
```

No schema, migration, kernel, `src/bin/*`, `tests/provisioning_validation.rs`, command, event, receipt, or HTTP handler/DTO/route changes. `git diff --check` clean.

## 5. SQL authorization contract

Both `list_assigned_to_me` and `list_creator_owned_drafts` now carry, inside the existing `WHERE`:

```sql
JOIN domains d ON d.domain_id = wi.domain_id AND d.enabled = TRUE
...
AND EXISTS (
  SELECT 1 FROM domain_role_bindings drb
  WHERE drb.domain_id = wi.domain_id
    AND drb.principal_id = $1
    AND drb.enabled = TRUE
)
```

- `domains.enabled = TRUE` ✓ (column confirmed in `migrations/0001_identity_domain.sql`)
- `EXISTS` over `domain_role_bindings` with `domain_id = wi.domain_id AND principal_id = $1 AND enabled = TRUE` ✓ (column/enabled confirmed).
- Matches the spec contract verbatim.
- The outer SELECT emits `(wi.workflow_instance_id, wi.created_at)` grouped by `wi`; `EXISTS` is a semi-join that short-circuits on the first match → **each WorkflowInstance is returned at most once**, even when the same actor holds multiple enabled bindings in the same domain (verified empirically, see §15).

## 6. No-domain-binding (H-01 §1)

SQL `EXISTS` requires a matching enabled `domain_role_bindings` row for `principal_id = $1`. Without one, the sub-query yields nothing → row filtered out.
- Test `no_domain_permission_hides_items`: domain2 instance not visible to actor who has binding only in domain1 (asserts exactly 1 item, in domain1). ✓
- Test `assignee_without_domain_permission_not_returned`: assignee with no binding sees 0 items. ✓
- Read-only SQL experiment: assignee with binding in domain A only sees 5 domain-A instances even though 5 domain-B instances are interleaved and assigned to them. ✓

## 7. Domain disabled (H-01 §2)

`JOIN domains d ON d.domain_id = wi.domain_id AND d.enabled = TRUE` excludes disabled domains.
- Test `domain_disabled_hides_items`: toggles `domains.enabled`, asserts 1 → 0 → 1. ✓
- SQL experiment: disabling domain A → 0 visible. ✓

## 8. Binding revoked (H-01 §3)

`drb.enabled = TRUE` excludes disabled bindings; `DELETE` removes the row entirely → `EXISTS` empty.
- Test `role_binding_revoked_hides_items`: `DELETE FROM domain_role_bindings ...` → 0 visible. ✓
- Test `creator_without_domain_permission_drafts_not_returned`: deleting creator binding → 0 drafts. ✓
- SQL experiment: both `enabled = FALSE` and `DELETE` → 0 visible. ✓

## 9. Assignee without domain permission (H-01 §4)

- Test `assignee_without_domain_permission_not_returned`: creator has binding, assignee does not → 0 items returned to assignee. ✓
- Covered by the same `EXISTS` predicate (the `$1` is the actor principal in both queries).

## 10. Creator without domain permission (H-01 §5)

- Test `creator_without_domain_permission_drafts_not_returned`: creator binding deleted → 0 drafts visible. ✓
- `list_creator_owned_drafts` filters on `EXISTS` with `principal_id = $1` = creator.

## 11. Multiple legal domains (H-01 §6)

- Test `multiple_legal_domains_all_visible`: actor has bindings in domains A and C (not B); 3 instances created (one per domain A/B/C); exactly 2 returned (A and C). ✓

## 12. Principal disabled (H-01 §7) — fail closed

- `actor_snapshot()` (`query_visibility.rs`) returns `WorkflowQueryError::PrincipalDisabled` when `principals.enabled = FALSE`, before the worklist SQL runs. Code-level fail-closed verified.
- Covered by existing snapshot/guard tests (`query_snapshot.rs`, `guards.rs`, `defensive.rs`) and unaffected by this diff.

## 13. Multi-role assigned-to-me dedup

- Test `assigned_to_me_multi_role_no_duplicates`: assigns 2 bindings (MEMBER + CONTRIBUTOR via `add_second_role`) to the actor in the same domain, creates 3 advanced instances, asserts exactly 3 returned with unique `workflowInstanceId`. ✓
- `EXISTS` semi-join guarantees no row multiplication (also proven by cross-page experiment in §15).

## 14. Multi-role creator-owned-drafts dedup

- Test `creator_drafts_multi_role_no_duplicates`: 2 bindings for the actor, 3 draft instances, asserts exactly 3 unique IDs. ✓

## 15. Multi-role pagination (cross-page) — read-only SQL experiment

The report's two multi-role tests cover only a single page each. Spec §V asks to dynamically construct data exceeding the page limit and walk all pages.

Experiment: seeded **25 instances** in one domain, assignee holding **2 enabled role bindings** (MEMBER + CONTRIBUTOR), each instance's `current_node_visit` on a non-terminal node assigned to the assignee. Walked the exact production SQL with `LIMIT 11` (limit+1) and cursor `(created_at, workflow_instance_id)`, page size 10:

| Metric | Result |
|---|---|
| Pages | 3 (10 + 10 + 5) |
| Total collected | 25 |
| Distinct IDs collected | 25 |
| Ground truth (matching instances) | 25 |
| Duplicates across all pages | **0** |
| Missing across all pages | **0** |
| Within-page order violations (descending) | **0** |
| Cross-page global order violations | **0** |
| Last page terminates correctly | ✓ (5 < limit, no infinite loop) |

→ Multi-role cross-page: **no duplicates, no omissions, stable order, correct cursor, clean termination.**

## 16. Permission/pagination interleaving (spec §VII)

Experiment: 5 authorized instances in domain A interleaved in time with 5 forbidden instances in domain B (A/B/A/B/…, same assignee for all, binding only in A). Production SQL:

- Returned exactly **5**, all in domain A.
- **0** domain-B (forbidden) instances leaked.
- **0** authorized A instances omitted.
- Forbidden instances are positioned *between* authorized ones in time (verified in ground-truth ordering), so the filter is applied before pagination — the cursor `(created_at, workflow_instance_id)` is derived from the instance row itself and is unaffected by filtered-out rows. ✓

Authorization filter is co-located with the cursor predicate in the same `WHERE`, ahead of `LIMIT` → **authorization happens before database pagination.**

## 17. Test helper changes (spec §VI) — verdict: CORRECT

`tests/17_workflow_runtime/query/helpers.rs::seed_query_fixture`:
- Before: inserted one MEMBER binding for `creator` only.
- After: loops over `[creator, assignee]` inserting MEMBER for both.

Verdict: **This is making existing tests satisfy the new formal authorization precondition, not loosening production permissions.**

- It is a *test* helper; production SQL is unaffected.
- `outsider` (the canonical no-permission actor in query tests) **does NOT** receive a binding — confirmed by reading the helper. All no-permission / cross-domain / fail-closed query tests still exercise a genuinely unblessed actor via `outsider`.
- No test actor receives blanket access to all domains — each test still constructs its own domain/binding fixtures (the `http/worklists.rs` new tests use explicit, scoped `domain_membership(...)` calls).
- Revocation/disable tests are not auto-restored by the helper: e.g. `role_binding_revoked_hides_items`, `creator_without_domain_permission_drafts_not_returned` delete/disable bindings after the helper runs and assert invisibility.
- creator == assignee as the same principal does not conflict: `assigned_to_me_returns_current_assignee_only` already exercises a separate creator and assignee and asserts RETURN moves the item off the assignee list and onto the creator list; both now have MEMBER bindings and the assertions still hold (test passes).

## 18. JWT / OBO / HTTP-adapter contract (spec §VIII)

`src/http/handlers/worklists.rs` and `src/http/dto.rs` are **not** in the diff. Contract preserved:

- `require_workflow(&principal, "workflow.read")` → 401/403 paths intact (tests `no_token_returns_401`, `missing_workflow_read_scope_returns_403`).
- Actor derived exclusively from `principal.principal_id` (from `JWT.sub`) → `actor_comes_from_jwt_sub_not_query` passes; unknown query params rejected via `WorklistQuery`'s `#[serde(deny_unknown_fields)]`.
- OBO uses `sub` (handler reads `AuthenticatedPrincipal.principal_id` from the validated token, never `act.sub`).
- Historical assignee / terminal / non-draft / ADVANCE-removes-draft semantics tested (`historical_assignee_not_returned`, `non_draft_not_returned_in_creator_drafts`, `creator_owned_drafts_returns_only_own_drafts`, `assigned_to_me_returns_current_assignee_only`).
- DTO reused unchanged; error envelope (`http::error`) unchanged.

## 19. Test counts (Base vs Fix)

`cargo test --workspace -- --list`:

| | Base | Fix | Δ |
|---|---|---|---|
| Total tests | **536** | **545** | **+9** |

Exactly matches spec. Per-binary diff: only `tests/17_workflow_runtime.rs` grew (327 → 336). All 9 new tests are net-new `http_worklists::*` functions; **no existing test deleted or renamed** (verified by sorted-name diff).

New tests (exact match to spec list):
1. `http_worklists::no_domain_permission_hides_items`
2. `http_worklists::domain_disabled_hides_items`
3. `http_worklists::role_binding_revoked_hides_items`
4. `http_worklists::assignee_without_domain_permission_not_returned`
5. `http_worklists::creator_without_domain_permission_drafts_not_returned`
6. `http_worklists::multiple_legal_domains_all_visible`
7. `http_worklists::pagination_respects_domain_isolation`
8. `http_worklists::assigned_to_me_multi_role_no_duplicates`
9. `http_worklists::creator_drafts_multi_role_no_duplicates`

No missed test binary, no unrelated tests mixed in.

## 20. Full verification suite (Fix worktree)

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS (exit 0, no diff) |
| `cargo build` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS (0 warnings/errors) |
| `cargo test --workspace -- --test-threads=1` | **545/545 passed, 0 failed** |
| `cargo test --workspace` (parallel round 1) | **545/545 passed, 0 failed** |
| `cargo test --workspace` (parallel round 2) | **545/545 passed, 0 failed** |
| (extra parallel round 3) | **545/545 passed, 0 failed** |
| `git diff --check` | clean |

### Note on a pre-existing, H-01-irrelevant parallel flake

During one early workspace-parallel run, lib unit test `auth::auth_mode::tests::jwks_gate_rejects_hs256_secret` (in `src/auth/auth_mode.rs`, **not in this diff**) failed with `assertion failed: result.is_err()`. Root cause: that test and sibling tests `jwks_gate_accepts_valid` / `test_hs256_gate_accepts_valid` mutate a process-global env (`WORKFLOW_JWT_SECRET`) via `unsafe std::env::set_var`/`remove_var`; under Rust 1.92 this is a documented data race when run in parallel.

Evidence this is pre-existing and unrelated:
- The test is in `src/auth/auth_mode.rs`, untouched by this fix.
- It passes 100% when lib unit tests run alone in parallel (3/3 clean) and in all serial runs.
- The same flake reproduces on **Base** (`fix/h01a-clean-fix`'s parent) under workspace-parallel execution: Base run #2 produced the identical `jwks_gate_rejects_hs256_secret FAILED` / lib "90 passed; 1 failed".
- The Fix produced 3 consecutive clean 545/545 parallel runs after that single transient miss.

This flake is a test-harness artifact, not a regression introduced or worsened by H-01. It does not affect the PASS determination (spec requires "two rounds parallel 545/545", which was achieved on consecutive runs 2 and 3, and run 1).

## 21. Schema / Migration / Kernel diff

None. Confirmed via `git diff --name-only base..fix` (3 files, all in `src/store/.../query_worklists.rs` and `tests/...`). No `migrations/`, no `src/bin/`, no kernel, no provisioning.

## 22. Temporary-resource cleanup

- Audit worktrees (`/tmp/worklist-audit-*`, `/tmp/worklist-base-*`) removed via `git worktree remove --force`.
- All temp SQL scripts and log files deleted.
- Test DB `svc_workflow` dropped & recreated clean (migrations only); all seeded experiment data purged.
- No temp processes, ports, or secrets introduced.

## 23. Findings

- **Blocker:** none.
- **High:** none.
- **Medium:** none. (See §20 for the pre-existing parallel env-race flake in `auth_mode.rs` tests — present on Base too, unrelated to H-01, self-healing; recommend a separate follow-up to serialize those env-mutating unit tests, but out of scope here.)
- **Low:** none for this change.

## 24. Recommendation

**ff-only merge is recommended.** The Fix is a single commit on top of `06856ca` (parent verified = Base), diff is minimal and scoped exactly to the two worklist queries plus their tests/helper, all verification checks pass, and H-01 is demonstrably closed across all 7 regression dimensions plus multi-role dedup (single-page and cross-page) and permission/pagination interleaving.

## 25. Final status

```
SVC_WORKFLOW_WORKLIST_HTTP_ADAPTER_AUDIT_PASS
```
