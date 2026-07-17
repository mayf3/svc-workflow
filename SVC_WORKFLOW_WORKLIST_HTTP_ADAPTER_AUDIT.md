# Worklist HTTP Adapter V0 — Independent Audit Report

## Meta

| Field | Value |
|---|---|
| Audit Agent | ZCode (DeepSeek-v4 Flash) |
| Repository | https://github.com/mayf3/svc-workflow |
| Branch | `audit/worklist-http-adapter-v0` |
| Base SHA | `53c79ae4d58cbead3c0ec605beeb757a7fba38c2` |
| Implementation SHA | `06856ca5247b7074ae1c801625c7853109cdb07c` |
| Implementation Tree | 6 files, +905 lines |
| Status | **SVC_WORKFLOW_WORKLIST_HTTP_ADAPTER_AUDIT_BLOCKED** |
| ff-only merge | Not recommended (pending H-01 resolution) |

---

## Audit Scope

Two read-only endpoints:
- `GET /internal/v1/worklists/assigned-to-me`
- `GET /internal/v1/worklists/creator-owned-drafts`

Architecture: HTTP Handler → AuthContext → WorkflowQueryService → Page DTO

---

## Implementation Verification

### Modification Boundaries ✅ PASS

**Diff contains only the 6 declared files:**

| File | Status | Delta |
|---|---|---|
| `src/http/dto.rs` | Modified | +8 lines (`WorklistQuery` DTO) |
| `src/http/handlers/worklists.rs` | Added | +164 lines (handlers + cursor parsing + unit tests) |
| `src/http/handlers/mod.rs` | Modified | +1 line (module declaration) |
| `src/http/mod.rs` | Modified | +8 lines (2 route registrations) |
| `tests/17_workflow_runtime.rs` | Modified | +2 lines (test module declaration) |
| `tests/17_workflow_runtime/http/worklists.rs` | Added | +722 lines (12 integration tests + helpers) |

**NOT in diff** (verified absent):
- Migration ❌
- Schema ❌
- Kernel ❌
- Repository ❌ (pre-existing query_worklists.rs is unchanged)
- Command/Event/Receipt ❌
- Secret or credentials ❌
- Configuration changes ❌

---

## Audit Findings Detail

### 1. Actor 不可伪造 ✅ PASS

**Verification:** The actor identity comes exclusively from `AuthenticatedPrincipal` (an Axum `FromRequestParts` extractor), which verifies the JWT and extracts `principal_id` from `JWT.sub`. No query parameter, header, or body can override it.

The `WorklistQuery` DTO uses `#[serde(deny_unknown_fields)]`, causing any unknown query parameters (e.g., `?actorId=`, `?principalId=`, `?userId=`, `?creatorId=`, `?assigneeId=`) to produce 422 UNPROCESSABLE_ENTITY at deserialization time.

**Code references:**
- `src/http/handlers/worklists.rs:27-28`: `principal: AuthenticatedPrincipal` as handler parameter
- `src/http/handlers/worklists.rs:36,60`: `actor_principal_id: principal.principal_id.into_uuid()`
- `src/http/dto.rs:87`: `#[serde(deny_unknown_fields)]` on `WorklistQuery`

**Test:** `actor_comes_from_jwt_sub_not_query`: passes `?actorId=<assignee>` with outsider token → 422 confirmed.

### 2. Scope ✅ PASS

Both endpoints require `workflow.read` scope via `require_scope(&principal, "workflow.read")?;`.

**Test evidence:**
- `no_token_returns_401`: No token → 401 UNAUTHORIZED
- `missing_workflow_read_scope_returns_403`: Token with `workflow.execute` only → 403 FORBIDDEN
- `direct_agent_token_works`: Token with `workflow.read workflow.execute` → 200 OK

### 3. Direct Human / Direct Agent / OBO ✅ PASS

The handler does not distinguish between token types. `principal_id` always derives from `JWT.sub`.

| Token Type | Profile | Resolution |
|---|---|---|
| Direct Agent | `principal_type=agent, token_use=access` | `sub` → principal_id |
| Direct Human | `principal_type=human, token_use=access` | `sub` → principal_id (JWKS verifier accepts both) |
| OBO | `token_use=workflow_obo, sub=user, act.sub=agent` | `sub` (user) → principal_id; `act.sub` not used for query |

**OBO note:** The HS256 test verifier rejects OBO markers. Full OBO end-to-end testing requires JWKS mode environment. The handler code is token-type agnostic, so the worklist correctly returns the subject's items, not the delegating agent's items.

### 4. 跨 Domain 隔离 ❌ HIGH FINDING (H-01)

**The existing `WorkflowQueryService` worklist queries do NOT filter by domain permissions or domain enabled status.**

**Evidence in `src/store/postgres/workflow_instance_repository/query_worklists.rs`:**

**`list_assigned_to_me` (L45-128):**
```sql
WHERE v.assignee_principal_id = $1 AND n.node_type <> 'TERMINAL'
```
No JOIN to `domain_role_bindings`. No check on `domains.enabled`.

**`list_creator_owned_drafts` (L130-207):**
```sql
WHERE wi.created_by_principal_id = $1 AND n.node_type = 'DRAFT'
```
Same gap.

**Contrast with `authorized_snapshot` → `classify_visibility`** in `query_visibility.rs` (used by instance detail queries): checks `domain_role_bindings` for `DOMAIN_OWNER` role, providing domain-level access control.

**Impact:** An actor can see worklist items in:
- Domains where they have no role binding (not a member/owner)
- Disabled domains (`domains.enabled = false`)

**Pre-existing condition:** This gap exists in the Query Service layer (pre-dates this HTTP adapter). Per audit rules, this is reported as HIGH and the HTTP Handler must NOT patch it.

**Existing test limitation:** The `cross_domain_isolation` test only proves "same actor with valid memberships in multiple domains sees both" — it does not test the isolation boundary (actor in Domain A with valid access, Domain B without access).

### 5. assigned-to-me 语义 ✅ PASS

SQL only returns instances where:
- Current node visit's `assignee_principal_id` matches the actor
- Current node type is NOT `TERMINAL`

Double-validation after loading:
```rust
if base.current_assignee_principal_id != Some(query.actor_principal_id)
    || base.current_node_type.as_deref() == Some("TERMINAL")
```

**Returns only:**
- ✅ Current active assignment
- ❌ Historical assignments
- ❌ Workflow creator (unless also current assignee)
- ❌ Past participants
- ❌ Completed/terminal instances

### 6. creator-owned-drafts 语义 ✅ PASS

SQL filters by `wi.created_by_principal_id` and `n.node_type = 'DRAFT'`.

No handler-level re-interpretation of draft status. `context_editable` and `combined_executable` are computed by the query service based on:
- Definition version status (`PUBLISHED`/`DEPRECATED` = editable)
- Current assignee matches creator (for combined_executable)
- Advance transition availability

ADVANCEd instances disappear from draft list (verified by test `non_draft_not_returned_in_creator_drafts`).

### 7. 分页合同 ✅ PASS

| Parameter | Implementation | Specification |
|---|---|---|
| `beforeCreatedAt` | RFC 3339 via `chrono::DateTime::parse_from_rfc3339` | Required format |
| `beforeId` | UUID v4 via `uuid::Uuid::parse_str` | Required format |
| Cursor pairing | Both present XOR both absent | Strict enforcement |
| `limit` default | 20 | Contract default |
| `limit` max (assigned) | 20 | Contract max |
| `limit` max (drafts) | 50 | Contract max |
| `limit = 0` | `InvalidPagination` (422) | Fail closed |
| Order | `created_at DESC, workflow_instance_id DESC` | Stable keyset |
| Tie-break | UUID `workflow_instance_id` as second sort key | Deterministic |
| `next_cursor` | `{ created_at, id }` from last item | Correct shape |
| Last page | `next_cursor: null` | Correct |
| Empty page | `{ items: [], next_cursor: null }` | Correct |

Pagination is performed at SQL level (`LIMIT limit+1` with overflow detection). No secondary sorting, no in-memory pagination.

### 8. DTO ✅ PASS

Response types directly reuse canonical query types:
- `Page<AssignedWorkItem>` from `query_types`
- `Page<CreatorDraftItem>` from `query_types`

No new response DTOs. Request DTO `WorklistQuery` is minimal (3 optional fields).

### 9. 错误映射 ✅ PASS

| Query Error | HTTP Status | Code | Internal Leak |
|---|---|---|---|
| `PrincipalNotFound` | 404 | `principal_not_found` | No |
| `PrincipalDisabled` | 403 | `principal_disabled` | No |
| `WorkflowInstanceNotFoundOrNotVisible` | 404 | `workflow_instance_not_found_or_not_visible` | No |
| `RestrictedHistoryNotVisible` | 403 | `restricted_history_not_visible` | No |
| `InvalidPagination` | 422 | `invalid_pagination` | No |
| `InternalConsistency` | 500 | `internal_consistency_error` | No SQL/path leak |
| `StorageError` | 503 | `service_unavailable` | No |

Invalid query params (unknown fields) → 422 via `ApiError::from_query_rejection`.
Invalid cursors → 422 `invalid_cursor` via `parse_worklist_cursor`.

---

## Test Results

### Test Count

| Metric | Base (`53c79ae`) | Candidate (`06856ca`) | Delta |
|---|---|---|---|
| **Total tests (all binaries)** | **518** | **536** | **+18** |
| `svc_workflow` lib | 85 | 91 | +6 (cursor unit tests) |
| `17_workflow_runtime` | 315 | 327 | +12 (HTTP worklist tests) |

**18 new tests are precisely accounted for:**
- 6 unit tests in `src/http/handlers/worklists.rs` (cursor parsing)
- 12 integration tests in `tests/17_workflow_runtime/http/worklists.rs`

### Static Analysis

| Check | Result |
|---|---|
| `cargo fmt --check` | ✅ Passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅ Passed |
| `git diff --check` | ✅ Passed |

### Dynamic Test Results

| Run | `17_workflow_runtime` | All Binaries | Status |
|---|---|---|---|
| `--test-threads=1` | 327 passed, 0 failed | 536 passed, 0 failed | ✅ |
| Parallel run 1 | 327 passed, 0 failed | 536 passed, 0 failed | ✅ |
| Parallel run 2 | 327 passed, 0 failed | 536 passed, 0 failed | ✅ |

### Integration Test Coverage (12 tests)

| Test | Coverage | Status |
|---|---|---|
| `no_token_returns_401` | No token → 401 | ✅ |
| `missing_workflow_read_scope_returns_403` | Wrong scope → 403 | ✅ |
| `assigned_to_me_returns_current_assignee_only` | Correct assignee sees item | ✅ |
| `historical_assignee_not_returned` | Non-assignee doesn't see item | ✅ |
| `creator_owned_drafts_returns_only_own_drafts` | Creator sees draft, outsider doesn't | ✅ |
| `non_draft_not_returned_in_creator_drafts` | ADVANCE removes from drafts | ✅ |
| `cross_domain_isolation` | Actor sees items in multiple domains (both authorized) | ✅ |
| `direct_agent_token_works` | Agent token works | ✅ |
| `actor_comes_from_jwt_sub_not_query` | Query param `?actorId=` → 422 | ✅ |
| `invalid_cursor_returns_422` | Malformed/broken cursor → 422 | ✅ |
| `pagination_cursor_works` | 3 items, limit=1, traverse 3 pages, no duplicates | ✅ |
| `empty_results_return_empty_page` | No data → empty page | ✅ |

---

## Findings Summary

### Blocker: 0
None identified.

### High: 1

**H-01: Cross-domain isolation gap in worklist queries**

| Attribute | Detail |
|---|---|
| Location | `src/store/postgres/workflow_instance_repository/query_worklists.rs` |
| Issue | `list_assigned_to_me` and `list_creator_owned_drafts` SQL queries lack domain-level access filtering |
| Impact | Actor can see worklist items in domains where they have no role binding or where the domain is disabled |
| Root cause | Pre-existing limitation in Query Service layer (predates this HTTP adapter) |
| Remediation | Add domain role-binding or domain-enabled filter to worklist SQL queries |
| Per audit rules | "如果现有 Query Service 本身没有按 Domain 权限过滤，必须报告 High，不得由 HTTP Handler 临时拼接过滤掩盖问题" |

### Medium: 0
None identified.

### Low: 1

**L-01: No OBO token test coverage in test_hs256 mode**

| Attribute | Detail |
|---|---|
| Issue | test_hs256 verifier rejects OBO markers (`token_use`, `act`), preventing end-to-end OBO testing |
| Impact | Test gap only; handler is token-type agnostic |
| Workaround | Full OBO test requires JWKS mode test environment |

---

## Resources Cleanup

| Resource | Status |
|---|---|
| Database test data | ✅ Truncated all tables |
| Test Principal/Domain/Instance | ✅ Cleaned |
| JWT/RSA temporary files | None created |
| Background processes | ✅ None remaining |
| Listening ports | ✅ None remaining |
| Temporary scripts | ✅ None remaining |

---

## Conclusion

### Recommended Action

**Do NOT ff-only merge** while H-01 is unresolved.

The HTTP adapter implementation itself is correct and well-structured. All contract requirements (actor invariant, scope enforcement, DTO reuse, error mapping, pagination contract, assigned-to-me semantics, creator-owned-drafts semantics) are met.

However, the pre-existing cross-domain isolation gap in the Query Service worklist queries is a HIGH finding that must be resolved before this feature can be considered production-ready. The HTTP adapter correctly delegates to the Query Service and must not add its own domain filtering.

### Audit Status

```
SVC_WORKFLOW_WORKLIST_HTTP_ADAPTER_AUDIT_BLOCKED
```

**Rationale:** High finding H-01 (cross-domain isolation gap in worklist queries) blocks unconditional PASS.

---

## Audit Checklist Summary

| # | Item | Result |
|---|---|---|
| 1 | Actor invariant | ✅ PASS |
| 2 | Scope (`workflow.read`) | ✅ PASS |
| 3 | Direct Agent | ✅ PASS |
| 3 | Direct Human | ✅ PASS |
| 3 | OBO | ✅ PASS (handler agnostic) |
| 4 | Cross-domain isolation | ❌ **HIGH** |
| 4 | Disabled Principal | ✅ (handled at Query Service level) |
| 4 | Disabled Domain | ❌ **Not filtered** (H-01) |
| 5 | assigned-to-me semantics | ✅ PASS |
| 6 | creator-owned-drafts semantics | ✅ PASS |
| 7 | Pagination contract | ✅ PASS |
| 8 | DTO reuse | ✅ PASS |
| 9 | Error mapping | ✅ PASS |
| 10 | Base test count | 518 |
| 11 | Candidate test count | 536 |
| 12 | Serial test result | 536 passed |
| 13 | Parallel test 1 | 536 passed |
| 14 | Parallel test 2 | 536 passed |
| 15 | Migration/Schema/Kernel diff | ✅ None |
| 16 | Temporary resource cleanup | ✅ Clean |
| 17 | Blocker | 0 |
| 18 | High | 1 (H-01) |
| 19 | Medium | 0 |
| 20 | Low | 1 (L-01) |
