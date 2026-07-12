# PostgreSQL Storage Foundation — Audit Report

## 1. Review Metadata

| Field | Value |
|---|---|
| Repository | `/Users/yanfenma/workspace/project/svc-workflow` |
| Branch | `feat/postgres-storage-foundation-v0` |
| Base SHA | `ba005e2bf4e3add7ea26cb89c608732ceba15745` |
| Review Commit | `81bcd13be5b8e6dd134890c326887d318595cf12` |
| Frozen Architecture Tag | `svc-workflow-architecture-v0.3.1-frozen` |
| PostgreSQL Version | PostgreSQL 16.14 (Homebrew) on x86_64-apple-darwin |
| Test Database | `svc_workflow` on `localhost:5432` (Docker/Homebrew PostgreSQL 16) |
| Test Database Start | Via Docker (`postgres:16-alpine`) + Homebrew PostgreSQL 16 |

## 2. Verification Commands and Results

### 2.1 `cargo fmt --check`
**Result:** ✅ PASS — no formatting errors.

### 2.2 `cargo build`
**Result:** ✅ PASS — build completes successfully.

### 2.3 `cargo clippy --all-targets --all-features -- -D warnings`
**Result:** ✅ PASS — no warnings or errors.

### 2.4 `git diff --check` (base..review)
**Result:** ✅ PASS — no whitespace errors.

### 2.5 `cargo test -- --test-threads=1`
**Result:** ❌ 1 FAILED (test isolation issue), 44 PASSED (of 45 total).

| Test File | Tests | Status |
|---|---|---|
| Unit (enums + ids) | 7 | ✅ 7 passed |
| `01_migration_tests` | 2 | ✅ 2 passed |
| `02_domain_owner_tests` | 2 | ✅ 2 passed |
| `03_runtime_constraints` | 10 | ✅ 10 passed |
| `04_event_constraints` | 4 | ✅ 4 passed |
| `05_command_constraints` | 5 | ✅ 5 passed |
| `06_instance_constraints` | 4 | ❌ 3 passed, 1 FAILED (isolation) |
| `07_deferred_fk_tests` | 2 | ✅ 2 passed |
| `08_definition_version_tests` | 5 | ✅ 5 passed |
| `09_size_limit_tests` | 4 | ✅ 4 passed |

**Failure detail:** `test_instance_domain_id_immutable` uses hardcoded domain_key `'other-domain'` which conflicts with a leftover row from a previous test run. In a clean database this test passes. This is a test isolation flaw.

### 2.6 `cargo test` (without `--test-threads=1`)
Not fully run due to the isolation failure, but individual test binaries were verified.

## 3. DDL vs Architecture Contract — Key Findings

### 3.1 Migration Completeness and Repeatability ✅
All 6 migration files apply cleanly to an empty database. No dependency on pre-existing extensions, schemas, or roles. SQLx discovers and applies all files. This is confirmed by the migration tests (`01_migration_tests.rs`).

### 3.2 Enum Design ✅
6 PostgreSQL enums match the contract exactly. The contract's known limitation about enum evolution (no `ALTER TYPE ... ADD VALUE` transaction safety) is correctly documented in the storage contract. No enum changes that would block this PR.

### 3.3 Identity and Principal Model ✅
`principals` has `enabled` column. `domains` uses `domain_key` with unique index. `domain_role_bindings` uses the correct `(domain_id, principal_id, role_key)` unique constraint. All match the contract.

## 4. Critical Findings

### 🔴 BLOCKER #1: Published Definition Version Sub-tables Unprotected

**File:** `migrations/0006_triggers_constraints.sql` (lines 154–182)

**Severity:** Blocker

**Description:**
The trigger `fn_check_definition_version_immutable()` on `workflow_definition_versions` only protects the **main table** fields (definition_digest, json_schema_dialect, context_schema, etc.) after PUBLISHED.

However, the sub-tables `workflow_node_definitions` and `workflow_transition_definitions` have **no triggers** preventing INSERT, UPDATE, or DELETE when the parent version is PUBLISHED, DEPRECATED, or REVOKED.

**Impact:**
Once a definition version is PUBLISHED:
- New nodes can be added to the frozen version
- Existing nodes can be modified (changing keys, order, assignee_ref_type, etc.)
- Existing transitions can be modified
- Nodes/transitions can be deleted

This directly violates the frozen architecture requirement:
> "发布后不可修改" (Section 6.2)
> "发布时冻结的内容：完整 NodeDefinition，完整 TransitionDefinition" (Section 6.3)

And the Blocker criterion from the review:
> "已发布 Definition 的图仍可被修改" → Blocker

**Required Fix:**
Add BEFORE INSERT/UPDATE/DELETE triggers on `workflow_node_definitions` and `workflow_transition_definitions` that check the parent `workflow_definition_versions.version_status`. If the version is not DRAFT, reject the modification.

Alternatively, add a CHECK constraint referencing the parent version status (though this requires a function/trigger approach since PostgreSQL CHECK constraints can't reference other tables directly in all cases).

**Tests Needed:**
- Test that inserting a new node into a PUBLISHED version fails
- Test that modifying an existing node in a PUBLISHED version fails
- Test that deleting a node from a PUBLISHED version fails
- Same 3 tests for transitions

---

### 🟠 HIGH #1: Test Isolation Failure — Hardcoded Domain Key

**File:** `tests/06_instance_constraints.rs` (line 53)

**Severity:** High

**Description:**
```rust
sqlx::query(
    "INSERT INTO domains (domain_id, domain_key, display_name, enabled) VALUES ($1, 'other-domain', 'Other', TRUE)"
)
```

The domain_key `'other-domain'` is hardcoded, while the test suite shares a single database without isolation. Any previous test run that created this key causes `test_instance_domain_id_immutable` to fail with a unique constraint violation before even reaching its actual assertion.

**Impact:**
- When run in CI or after other tests, this test flakily fails
- This blocks the test suite from passing reliably with `--test-threads=1`
- Prevents automated CI from cleanly passing

**Required Fix:**
Use a unique domain_key, e.g.:
```rust
let other_key = format!("other-domain-{}", &uuid::Uuid::new_v4().to_string()[..8]);
```

---

### 🟠 HIGH #2: Missing Test for `definition_version_id` Immutability

**File:** `tests/06_instance_constraints.rs` (lines 82–99)

**Severity:** High

**Description:**
The test named `test_instance_definition_version_id_immutable` does NOT test `definition_version_id` immutability. Instead, it tests that `workflow_state_version` can be changed (a projection field that SHOULD be mutable).

The contract (and DDL trigger) requires `definition_version_id` to be immutable after creation, just like `domain_id` and `created_by_principal_id`. There is no test coverage for this constraint.

**Impact:**
A regression in the trigger `fn_check_instance_immutable_fields` that removes `definition_version_id` protection would go undetected.

**Required Fix:**
Rename the existing test to `test_workflow_state_version_mutable` and add a new test `test_definition_version_id_immutable` that attempts to change `definition_version_id` and expects rejection.

---

### 🟡 MEDIUM #1: All Composite FKs Unnecessarily Deferred

**Files:** `migrations/0003_runtime.sql`, `migrations/0004_workflow_events.sql`, `migrations/0006_triggers_constraints.sql`

**Severity:** Medium

**Description:**
Every composite foreign key uses `DEFERRABLE INITIALLY DEFERRED`. The only FKs that genuinely need deferral are:
1. `fk_instance_current_ctx` (circular: instance ↔ context_revision)
2. `fk_instance_current_visit` (circular: instance ↔ node_visit)
3. `fk_previous_revision` (self-referencing circular within context_revisions)
4. `fk_primary_advance_transition` (node ↔ transition, circular within a single version)

The following FKs do NOT have circular dependencies and should be `NOT DEFERRABLE`:
- `fk_submission_visit_same_instance`
- `fk_submission_ctx_same_instance`
- `fk_event_source_visit_same_instance`
- `fk_event_target_visit_same_instance`
- `fk_event_ctx_same_instance`
- `fk_event_submission_same_instance`
- `fk_event_command`

**Impact:**
- Cross-instance integrity violations are only caught at COMMIT time, not at INSERT time
- Development debugging is more difficult — violations surface later than necessary
- A transaction could temporarily create cross-instance references within the transaction and only fail at commit

**Risk Assessment:**
Low practical risk for production because:
- The Command Service will always insert entities in the correct order within a single transaction
- The constraints will still catch violations at commit time
- No real-world scenario would cause the deferred FKs to silently pass through

But defensive DDL design suggests non-deferred for non-circular FKs.

---

### 🟡 MEDIUM #2: `external_url` and `metadata` Not Protected by Trigger

**File:** `src/migrations/0006_triggers_constraints.sql` (lines 123–148)

**Severity:** Medium

**Description:**
The frozen architecture (Section 11) explicitly states:
> `externalUrl` 和 `metadata` 在 v0.3.1 创建后不可修改。

The trigger `fn_check_instance_immutable_fields` protects `domain_id`, `definition_version_id`, `created_by_principal_id`, and `created_at`. However, `external_url` and `metadata` are NOT protected.

The storage contract (Section 5) lists only the above 4 fields as immutable for instances.

**Impact:**
After creation, someone can modify `external_url` or `metadata` on any instance, which contradicts the frozen architecture. However, these are display/navigation fields and do not affect workflow correctness or audit integrity.

---

### 🟡 MEDIUM #3: Incomplete Size Limit Test Coverage

**Files:** `tests/09_size_limit_tests.rs`

**Severity:** Medium

**Description:**
The DDL defines 7 size constraint checks (migration 0006):
1. ✅ Context payload ≤ 1 MiB — **tested**
2. ❌ Submission payload ≤ 1 MiB — **NOT tested**
3. ✅ Instance metadata ≤ 64 KiB — **tested**
4. ❌ Definition metadata ≤ 64 KiB — **NOT tested**
5. ❌ Definition version metadata ≤ 64 KiB — **NOT tested**
6. ❌ Receipt response body ≤ 1 MiB — **NOT tested**
7. ✅ Event data ≤ 256 KiB — **tested**

Only 4 of 7 constraints have test coverage.

**Impact:**
Regressions or misconfigurations in the untested constraints would go undetected.

---

### 🟡 MEDIUM #4: PROCESSING Receipt Identity Fields Not Frozen

**File:** `migrations/0006_triggers_constraints.sql`

**Severity:** Medium

**Description:**
The trigger `trg_command_receipts_completed_immutable` only blocks UPDATE/DELETE on COMPLETED receipts. A PROCESSING receipt's identity fields (`principal_id`, `idempotency_key`, `request_hash`, `command_type`, `command_id`) can be arbitrarily modified before completion.

**Impact:**
- A PROCESSING receipt's `request_hash` could be changed to match a different request
- `principal_id` and `idempotency_key` (the unique constraint) could be changed, breaking idempotency tracking
- `command_id` could be changed, breaking linkage to events

**Mitigation:**
In the current PR 1 scope, the Command Service hasn't been implemented yet. The service layer will handle this during normal operations. However, the DDL could provide defense-in-depth by freezing these fields at insert time.

**Severity justification:**
Medium rather than High because:
1. Only svc-workflow should have write access to these tables
2. The Command Service (future PR) will never modify these fields during normal PROCESSING→COMPLETED transition
3. Direct database access is out of scope for normal operations

---

### 🔵 LOW #1: Test File Exceeds 500 Lines

**File:** `tests/03_runtime_constraints.rs` — 561 lines

**Severity:** Low

**Description:**
Exceeds the 500-line file length guard by 61 lines. This is a code organization concern, not a functional issue.

**Suggestion:**
Split into smaller focused test files, e.g.:
- `tests/03a_context_revision_tests.rs`
- `tests/03b_node_visit_tests.rs`
- `tests/03c_submission_tests.rs`

---

### 🔵 LOW #2: Misnamed Test

**File:** `tests/06_instance_constraints.rs` (line 82)

**Severity:** Low

**Description:**
`test_instance_definition_version_id_immutable` actually tests that `workflow_state_version` can be changed. The test name is misleading. Should be renamed to `test_workflow_state_version_mutable`.

(Separate from HIGH #2 which requires a new test for actual `definition_version_id` immutability.)

---

### 🔵 LOW #3: No Test for Subtable Post-Publish Protection

**Referenced from:** BLOCKER #1

**Severity:** Low (as a test gap note; the functional gap itself is Blocker)

**Description:**
There are no tests verifying that `workflow_node_definitions` and `workflow_transition_definitions` cannot be modified after the parent definition version is published. This is a direct consequence of BLOCKER #1 — tests cannot be written for protection that doesn't exist.

---

## 5. Structure Guard Results

| Check | Result |
|---|---|
| Max file ≤ 500 lines | ❌ `tests/03_runtime_constraints.rs` = 561 lines |
| Directory depth ≤ 4 | ✅ Max depth = 2 (`src/store/postgres`) |
| Direct children ≤ 20 | ✅ `src/` = 7, `tests/` = 11 |
| No generated files counted | ✅ `Cargo.lock` excluded from count |

## 6. Security and Delete Semantics

| Check | Result |
|---|---|
| Real credentials committed? | ✅ No — `.env.example` uses safe defaults |
| Audit table cascade deletion? | ✅ No `ON DELETE CASCADE` anywhere — all FKs use default `NO ACTION` |
| Principal `disabled` column? | ✅ `principals.enabled` exists and defaults to `TRUE` |
| Dynamic SQL injection risk in migrations? | ✅ No dynamic SQL in migrations — plain DDL only |
| `SECURITY DEFINER` used? | ✅ Not present in any migration |
| Trigger functions unprotected? | ✅ Functions are owned by the migration user, no `SECURITY DEFINER` |
| Schema/table ownership defined? | ⚠️ Storage contract notes this PR uses `public` schema; independent `workflow` schema deferred |
| Instance deletion cascades to facts? | ✅ No cascade — all `REFERENCES` use default `NO ACTION` |

## 7. Test Coverage Gaps Summary

| Gap | Severity | Reference |
|---|---|---|
| Definition subtable post-publish protection | 🔴 BLOCKER | Section 4, BLOCKER #1 |
| `definition_version_id` immutability test missing | 🟠 HIGH | Section 4, HIGH #2 |
| Submission payload size limit untested | 🟡 MEDIUM | Section 4, MEDIUM #3 |
| Receipt response body size limit untested | 🟡 MEDIUM | Section 4, MEDIUM #3 |
| Definition/version metadata size limit untested | 🟡 MEDIUM | Section 4, MEDIUM #3 |
| PROCESSING receipt identity fields frozen test | 🟡 MEDIUM | Section 4, MEDIUM #4 |
| Subtable post-publish test (blocked by missing protection) | 🔵 LOW | Section 4, LOW #3 |

## 8. Verdict

```
SVC_WORKFLOW_POSTGRES_STORAGE_FOUNDATION_AUDIT_BLOCKED
```

**Rationale:** The presence of a **Blocker** (published Definition Version sub-tables unprotected — the graph structure of a frozen version can be modified) prevents this PR from being merged in its current state. This directly violates the frozen architecture and the Blocker criterion "已发布 Definition 的图仍可被修改."

The storage skeleton is otherwise well-designed. The migration system is sound, the Rust type system is clean, the immutable record triggers work correctly, and the circular FK pattern is properly implemented. Once the subtable protection is added (triggers on `workflow_node_definitions` and `workflow_transition_definitions`), this PR should pass.

## 9. Minimum Fix Recommendations

### Required Before Merge (Blocker)

1. **Add triggers on `workflow_node_definitions` and `workflow_transition_definitions`** (in `0006_triggers_constraints.sql`) to prevent INSERT, UPDATE, and DELETE when the parent `workflow_definition_versions.version_status` is not DRAFT.

   Approach: A trigger function that queries the parent version's status and raises an exception if it's PUBLISHED/DEPRECATED/REVOKED.

### Strongly Recommended Before Merge (High)

2. **Fix test isolation** in `tests/06_instance_constraints.rs` — use a unique domain_key instead of hardcoded `'other-domain'`.
3. **Rename and add test** in `tests/06_instance_constraints.rs` — rename `test_instance_definition_version_id_immutable` to `test_workflow_state_version_mutable` and add a new `test_definition_version_id_immutable` test.

### Recommended But Not Blocking (Medium)

4. Make non-circular composite FKs `NOT DEFERRABLE` for earlier violation detection.
5. Add `external_url` and `metadata` to the Instance immutable fields trigger.
6. Add missing size limit tests for submission payload, receipt response body, and metadata fields.
7. Consider adding BEFORE UPDATE trigger on PROCESSING receipts to freeze identity fields (`principal_id`, `idempotency_key`, `request_hash`, `command_type`, `command_id`).
8. Split `tests/03_runtime_constraints.rs` (561 lines) into smaller focused files.

## 10. Final Git Status

```
$ git status --short
(clean — only this report file is new)
```

---

*Report generated: 2026-07-13*
*Auditor: ZCode (automated review)*
