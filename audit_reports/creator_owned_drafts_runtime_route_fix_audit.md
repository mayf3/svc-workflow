# Audit Report: Creator-Owned Drafts Runtime Route Fix

## Identity

| Field | Value |
|-------|-------|
| Report type | Independent security & correctness audit |
| Component | `svc-workflow` — HTTP worklist routing |
| Fix branch | `fix/creator-owned-drafts-route` |

## Frozen Artifacts

| Artifact | SHA | Tree |
|----------|-----|------|
| **Base** (main prior to fix) | `4084c280f79a4cef5cf3122142635b61ec0d2dfb` | `6872478b44386041b9969693e2643a758ce2c2df` |
| **Fix** | `52638551f8d72bf1947dd171456946cf28e7c910` | `0d6c4c1a54db0ecc34da3a6f7eb62ca2f23c7758` |
| Fix parent | `4084c280f79a4cef5cf3122142635b61ec0d2dfb` | — |

## Diff Summary

3 files changed, 173 insertions(+), 6 deletions(-)

| File | Change |
|------|--------|
| `src/http/handlers/worklists.rs` | Added `creator_owned_drafts` handler; extended module header documentation to describe both endpoints; imported `CreatorDraftItem` and `ListCreatorOwnedDrafts` types. |
| `src/http/mod.rs` | Registered `GET /internal/v1/worklists/creator-owned-drafts` route alongside the existing `assigned-to-me` route. |
| `tests/17_workflow_runtime/http/worklists.rs` | Added 3 new test functions: `creator_owned_drafts_returns_only_own_drafts`, `non_draft_not_returned_in_creator_drafts`, and `unknown_path_returns_404`; extended `no_token_returns_401` and `missing_workflow_read_scope_returns_403` to also cover the new endpoint. |

## Audit Findings

### Valid Token — 200 OK
The handler requires `workflow.read` scope and authenticates via `AuthenticatedPrincipal` (JWT `sub` → Principal ID). A token with the correct scope returns `200 OK` with a `Page<CreatorDraftItem>` payload.

### Creator Isolation
The `list_creator_owned_drafts` query filters by `actor_principal_id` derived from `JWT.sub`. Creator A cannot see Creator B's drafts. Verified by the `creator_owned_drafts_returns_only_own_drafts` test which asserts the outsider receives zero items.

### Non-Draft Exclusion
The query service filters `status = DRAFT` only. Instances advanced beyond DRAFT are excluded. Verified by `non_draft_not_returned_in_creator_drafts`.

### Production Router Authenticity
The route is registered via `axum::Router::route("/internal/v1/worklists/creator-owned-drafts", get(...))` in the same module and router chain as the existing `assigned-to-me` endpoint, inheriting identical middleware, auth extraction, and error handling. The test suite exercises the real router stack via `axum::TestServer` (no mock router).

### Test Count Increase
| Metric | Base | Fix |
|--------|------|-----|
| `cargo test -- --list` | 553 | 556 |
| `tests/17_workflow_runtime/http/worklists.rs` test functions | 18 | 21 |

### Test Results
| Run mode | Result |
|----------|--------|
| Serial (`--test-threads=1`) | 556/556 passed |
| Parallel round 1 | 556/556 passed |
| Parallel round 2 | 556/556 passed |

### Blocker / High / Medium / Low Counts
| Severity | Count | Detail |
|----------|-------|--------|
| Blocker | 0 | — |
| High | 0 | — |
| Medium | 1 | HTTP integration tests do not independently exercise Domain Binding revocation, Domain disabled, multi-role deduplication, or `creator-owned-drafts` cross-page pagination at the HTTP layer. **These semantics are provided by the unmodified, previously-audited Query Service layer; the HTTP route fix does not alter them.** Recorded as test debt, not a release blocker. |
| Low | 0 | — |

### Merge Recommendation
**ff-only merge recommended.** The fix is a single-parent descendant of base with no conflicting changes. No rebase, squash, cherry-pick, or merge commit required.

## Scoring Rubric

- **Blocker**: Incorrect auth, data leak, non-functional route, test regression.
- **High**: Missing auth check, scope bypass, incorrect pagination state.
- **Medium**: Missing test coverage for edge cases at the HTTP layer.
- **Low**: Formatting, naming, or documentation only.

## Audit Scope Notes

The audit covers:
- Route registration correctness
- Auth / scope enforcement parity with existing endpoints
- Creator isolation in the HTTP handler
- Non-Draft filtering (via handler → query service contract)
- Test coverage for auth, isolation, exclusion, and 404 behavior
- fmt, clippy, build, test (serial + parallel) cleanliness

The audit does **not** re-audit:
- ADC V2 workflow semantics
- The Query Service `list_creator_owned_drafts` implementation (previously audited)
- Domain isolation at the SQL layer (previously audited)
- Provisioning, admin, or non-worklist endpoints
