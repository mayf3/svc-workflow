# Workflow Instance Create v0 — Audit Report

## 1. Meta

| Field | Value |
|---|---|
| Repository | `/Users/yanfenma/workspace/project/svc-workflow` |
| Branch | `feat/workflow-instance-create-v0` |
| Base SHA | `c7870dfc2938b81eedf616bc36a17ae8f64135ec` |
| Original implementation SHA | `8dce3d20f64affc98bc2bdcda231d233f2b8bbee` |
| Audit HEAD | `2efc01cbec0fed63c956defdb54ba8a632f8ade5` |
| PostgreSQL version | 16.14 (Homebrew) |
| Migration count | 9 (0001–0009) |
| Architecture document | `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md` (FROZEN) |
| Implementation contract | `docs/contracts/IMPLEMENTATION_CONTRACT_V0_1.md` |
| Instance Create contract | `docs/contracts/WORKFLOW_INSTANCE_CREATE_CONTRACT_V0_1.md` |
| Storage contract | `docs/contracts/POSTGRES_STORAGE_CONTRACT_V0_1.md` |

### Test counts

| Category | Count |
|---|---|
| Unit tests (`#[test]` in `src/`) | 54 |
| Integration tests (`#[tokio::test]` in `tests/`) | 156 |
| Total | 210 |
| Workflow Instance Create tests (`tests/17_workflow_instance_create*`) | 39 |

### Command results

| Command | Result |
|---|---|
| `git status --short` | (clean) |
| `git diff --check` | (clean) |
| `cargo fmt --check` | (clean) |
| `cargo build` | (passed) |
| `cargo clippy --all-targets --all-features -- -D warnings` | (passed) |
| `cargo test -- --test-threads=1` | **210 passed** |
| `cargo test` (1st parallel) | **209 passed, 1 failed** |
| `cargo test` (2nd parallel) | **208 passed, 2 failed** |
| `cargo test` (3rd parallel) | **209 passed, 1 failed** |

---

## 2. Transaction Topology

### 2.1 Actual transaction order (from `create_transaction.rs`)

```
BEGIN
  1. try_insert_receipt()           — INSERT ON CONFLICT DO NOTHING RETURNING
     If no row → replay_existing_receipt() → FOR UPDATE + replay/conflict/processing
  
  === Reached only by the one request that owns the receipt ===
  
  2. lock_and_validate_version()    — SELECT ... FOR UPDATE on definition_version
  3. validate_domain_enabled()      — SELECT inside same tx
  4. validate_principal_enabled()   — SELECT inside same tx (re-verification)
  5. validate_domain_membership()   — SELECT inside same tx
  6. read_draft_node()             — SELECT WHERE definition_version_id AND node_type='DRAFT'
  7. resolve_assignee()            — SELECT based on ASSIGNEE_REF_TYPE
  8. validate_context_schema()     — In-process jsonschema validation (no SQL)
  9. INSERT workflow_instances     — WITH pre-generated IDs, deferred FK references
  10. INSERT workflow_context_revisions — revision_number=1, previous_revision_id=NULL
  11. INSERT workflow_node_visits   — visit_number=1, entered_by_transition_id=NULL
  12. INSERT workflow_events        — INSTANCE_CREATED, event_sequence=1
  13. UPDATE complete_receipt       — PROCESSING → COMPLETED, 200 + response_body
COMMIT
```

### 2.2 Key observations

**All operations use the same `sqlx::Transaction`**. Every read and write goes through `&mut tx`. There is no autocommit query, no intermediate commit, and no separate connection.

**`pre_validate_principal`** (in `create.rs`) runs before the transaction begins as a fast-fail optimization. This is acceptable because the principal is re-verified inside the transaction at step 4 with full transactional consistency. No TOCTOU risk — the pre-check only avoids unnecessary transaction startup for a nonexistent/disabled principal.

**The `Definition Version` lock (step 2, `FOR UPDATE`) is held until COMMIT.** All subsequent reads (domain, principal, membership, draft node, assignee) happen while the lock is held, ensuring consistency.

**Circular FK resolution**: `workflow_instances` references `current_context_revision_id` and `current_node_visit_id` that don't yet exist at INSERT time. The FKs are `DEFERRABLE INITIALLY DEFERRED` and validated at COMMIT. Verified working by `test_deferred_fk_committed_successfully`.

**All writes happen in a single transaction**. No partial commit scenario exists in the success path.

### 2.3 Verdict: ✅ Transaction is atomic

---

## 3. CommandReceipt Idempotency State Machine

### 3.1 State machine

```
                    ┌─ ON CONFLICT DO NOTHING ──┐
                    │                            │
               row returned                  no row
                    │                            ▼
               ┌────┴────┐          SELECT ... FOR UPDATE
               │ OWNER   │                    │
               └────┬────┘          ┌─────────┼──────────┐
                    │               │         │          │
              proceed with      COMPLETED  PROCESSING  (not found)
              creation          same hash             defensive err
                    │               │         │
               write facts      replay     "StillProcessing"
               complete receipt  response
                    │
               COMMIT
```

### 3.2 Receipt vs runtime facts relationship

**Deterministic failure** (e.g., domain disabled):
- The receipt INSERT succeeds (same tx).
- Validation fails → `complete_receipt()` is called inside the same tx → status=COMPLETED, response=error.
- Transaction COMMITs.
- **Result**: COMPLETED receipt with error, NO runtime facts (instance/context/visit/event).

**Infrastructure failure** (e.g., DB connection, trigger rejection):
- The entire transaction rolls back.
- **Result**: NO receipt (receipt INSERT is rolled back), NO runtime facts.

**Contract correctness**: The code correctly implements the contract. The implementation report statement "基础设施不回滚 Receipt" is **contradicted by the actual code** — the receipt IS rolled back with the rest of the transaction on infrastructure failure. The correct statement is "基础设施失败无残留 Receipt", which matches both code and contract.

| Scenario | Receipt persists? | Runtime facts persist? |
|---|---|---|
| Success | COMPLETED (200) | Yes (instance + ctx + visit + event) |
| Deterministic failure | COMPLETED (error) | No |
| Infrastructure failure | No | No |
| Idempotent replay | COMPLETED (original) | No (reuses original) |
| Conflict (hash mismatch) | COMPLETED (original) | No (original untouched) |

### 3.3 PROCESSING receipt retention

A PROCESSING receipt that never completes (e.g., from a crashed command) will cause all subsequent requests with the same key to receive `CommandStillProcessing`. The v0 contract explicitly defers recovery/retry/cleanup. This is documented as a limitation.

### 3.4 Verdict: ✅ Idempotency implementation matches contract

---

## 4. requestHash Computation

### 4.1 Actual code structure

The `compute_request_hash` function builds:

```json
{
  "command_schema_version": "v1",
  "command_type": "CREATE_WORKFLOW_INSTANCE",
  "route_parameters": {},
  "request_body": {
    "principal_id": "<uuid>",
    "domain_id": "<uuid>",
    "definition_version_id": "<uuid>",
    "context_payload": { ... },
    "metadata": { ... },
    "external_reference": null,
    "external_url": null
  }
}
```

Then applies `jcs_canonicalize::sha256_jcs_hex()` (JCS + SHA-256).

### 4.2 Key checks

| Check | Status |
|---|---|
| idempotency_key excluded from hash | ✅ Explicitly excluded (`_idempotency_key` is unused) |
| principalId in requestBody | ✅ `RequestBody` includes `principal_id` |
| No `serde(flatten)` | ✅ The `request_body` field is a nested struct |
| JCS used for canonicalization | ✅ `jcs_canonicalize::sha256_jcs_hex` |
| Map key order insensitive | ✅ JCS sorts object keys |
| `routeParameters` = `{}` | ✅ `serde_json::json!({})` |
| All business fields covered | ✅ |
| `null` / empty object semantics clear | ✅ `Option<String>` → None serializes as `null` |

### 4.3 Caveat: snake_case vs contract's camelCase

The contract shows `camelCase` keys (`commandSchemaVersion`, `requestBody`, `principalId`), but the actual Rust structs derive `Serialize` without rename attributes, producing `snake_case` keys. This is **self-consistent** — hashes are computed and compared within the same codebase — but the contract description should match the code.

### 4.4 Verdict: ✅ requestHash is self-consistent and correct

---

## 5. Authorization

### 5.1 Authorization checks (all inside transaction)

| Check | Code | Contract match | Test coverage |
|---|---|---|---|
| Principal exists & enabled | `validate_principal_enabled()` | ✅ | `test_disabled_principal_rejected` |
| Domain exists & enabled | `validate_domain_enabled()` | ✅ | `test_disabled_domain_rejected` |
| Domain membership (any role) | `validate_domain_membership()` | ✅ | `test_no_domain_membership_rejected` |
| Cross-domain version check | `lock_and_validate_version()` | ✅ | `test_cross_domain_version_rejected` |
| Definition belongs to domain | `lock_and_validate_version()` | ✅ | (same test) |
| Version status = PUBLISHED | `lock_and_validate_version()` | ✅ | `test_draft_version_rejected`, `test_deprecated_version_rejected`, `test_revoked_version_rejected` |
| Disabled binding not valid | `WHERE enabled = TRUE` in query | ✅ | (covered by membership tests) |
| Disabled owner → resolution fails | `resolve_assignee()` | ✅ | `test_disabled_domain_owner_assignee_rejected` |

### 5.2 Membership scope

The contract says "any role key is sufficient — DOMAIN_OWNER is not required." The code queries `SELECT enabled FROM domain_role_bindings WHERE domain_id = $1 AND principal_id = $2 AND enabled = TRUE LIMIT 1`. Any enabled role (MEMBER, DOMAIN_OWNER, etc.) passes. ✅

### 5.3 Cross-domain restriction

A principal in domain A cannot create instances using domain B's definitions, even if they have a role in domain A. The `lock_and_validate_version` cross-checks the version's `workflow_definition.domain_id` against the command's `domain_id`. ✅

### 5.4 Verdict: ✅ Authorization implementation matches contract exactly

---

## 6. Assignee Resolution

| Type | Code path | Contract match | Test |
|---|---|---|---|
| WORKFLOW_CREATOR | Returns `principal_uuid` (the caller) | ✅ | `test_create_success_wf_creator` |
| DOMAIN_OWNER | Queries binding with role_key='DOMAIN_OWNER', enabled=TRUE; verifies principal exists and enabled | ✅ | `test_create_success_domain_owner_assignee`, `test_disabled_domain_owner_assignee_rejected` |
| FIXED_PRINCIPAL | Uses `draft_node.fixed_principal_id`; verifies principal exists and enabled | ✅ | `test_create_success_fixed_principal_assignee`, `test_disabled_fixed_principal_assignee_rejected` |

All three types verify that the resolved principal exists and is `enabled = true`. If any step fails, returns `AssigneeResolutionFailed`. ✅

**Domain Owner source of truth**: Uses `domain_role_bindings` table, not a redundant `domains.owner_principal_id` field. ✅

### Verdict: ✅ Assignee resolution matches contract

---

## 7. Context & Digest

### 7.1 Context payload validation

| State | Behavior |
|---|---|
| `context_schema = NULL` | Any JSON accepted (subject to size limits) |
| `context_schema = Some(schema)` | `jsonschema::validator_for()` compiles and validates |

**CRITICAL FINDING**: All 39 create tests use `context_schema = NULL` (seeded by `seed_published_definition_wf_creator` and variants). There is **zero test coverage** of the `context_schema = Some(...)` branch. The `validate_context_schema()` function at `validation_helpers.rs:151–173` is never exercised with a real schema.

While the code reads correctly (standard `jsonschema` crate use), the lack of validation test means:
- Schema compilation errors would not be caught
- Validation rejection of invalid payloads is not proven
- The core contract guarantee "invalid context rejected" is not demonstrated

### 7.2 Size limits

| Field | Service layer | DB layer | Tests |
|---|---|---|---|
| context_payload ≤ 1 MiB | `serde_json::to_vec().len() > 1024*1024` | `chk_ctx_payload_size` (pg_column_size) | `test_context_payload_too_large_rejected` |
| metadata ≤ 64 KiB | `serde_json::to_vec().len() > 64*1024` | `chk_instance_metadata_size` (pg_column_size) | `test_metadata_too_large_rejected` |
| Failed size check → no runtime facts | Service layer rejects before tx starts | N/A | `test_failure_no_runtime_artifacts_left` |

The size limit failures happen **outside the transaction** (in `validate_context_size()` in `create.rs`), so no PROCESSING receipt is created. This is correct per the contract — a pre-transaction rejection leaves no trace.

### 7.3 Context digest

`payload_digest = SHA-256(JCS(context_payload))`. Test `test_create_context_digest_readback` reads back the digest from the DB and recomputes. ✅

### 7.4 Verdict: ⚠️ Context schema validation code is correct but UNTESTED for non-null schemas

---

## 8. Initial Facts and Event Matrix

### 8.1 Initial facts after successful creation

| Entity | Asserted properties | Verified by |
|---|---|---|
| WorkflowInstance | `workflow_state_version=1`, `domain_id` correct, `definition_version_id` correct, `created_by_principal_id` correct | `verify_creation()`, `test_create_current_pointers_correct` |
| WorkflowContextRevision | `revision_number=1`, `previous_revision_id=NULL`, correct payload/digest | `verify_creation()`, `test_create_context_digest_readback` |
| NodeVisit | `visit_number=1`, `node_id` = DRAFT node, `entered_by_transition_id=NULL`, correct resolved assignee | `verify_creation()` |
| WorkflowEvent (INSTANCE_CREATED) | One event, sequence=1, correct matrix (see below) | `test_exactly_one_event_per_creation`, `test_create_event_field_matrix_correct` |
| CommandReceipt | COMPLETED, status 200, correct response digest | `test_create_all_records_present`, `test_create_response_digest_readback` |

### 8.2 Event field matrix

| Field | Contract value | Actual value | Match |
|---|---|---|---|
| `event_type` | `INSTANCE_CREATED` | `INSTANCE_CREATED` | ✅ |
| `event_sequence` | 1 | 1 | ✅ |
| `old_workflow_state_version` | 0 | 0 | ✅ |
| `new_workflow_state_version` | 1 | 1 | ✅ |
| `source_node_visit_id` | NULL | NULL | ✅ |
| `target_node_visit_id` | Initial NodeVisit ID | `node_visit_id` | ✅ |
| `context_revision_id` | Revision #1 ID | `context_revision_id` | ✅ |
| `submission_id` | NULL | NULL | ✅ |
| `command_id` | Receipt.command_id | `actual_command_id` | ✅ |
| `actor_principal_id` | Caller principal | `principal_uuid` | ✅ |
| `event_schema_version` | "v1" | `EVENT_SCHEMA_VERSION` ("v1") | ✅ |
| `event_data` | `{definitionVersionId, definitionDigest, initialNodeId, assigneeResolutionType}` | `{definition_version_id, definition_digest, initial_node_id, assignee_resolution_type}` (snake_case) | ✅ content, ⚠️ key casing |

**One-to-one mapping**: Verified by `test_command_id_matches_event` that exactly one event joins to one receipt. ✅

**Global event cursor**: Not implemented (deferred per architecture). ✅

### 8.3 Verdict: ✅ Event matrix correct

---

## 9. Fault Injection & Atomicity Tests

### 9.1 Test coverage

| Scenario | Test | Mechanism | Verified |
|---|---|---|---|
| Event INSERT failure → rollback | `test_event_failure_rolls_back_everything` | TRIGGER BEFORE INSERT on events | ✅ No instance, no ctx, no visit, no event. Receipt also gone (tx rolled back) |
| Instance INSERT failure → rollback | `test_infrastructure_failure_no_residual_receipt` | TRIGGER BEFORE INSERT on instances | ✅ No receipt (tx rolled back) |
| Deterministic failure → COMPLETED receipt, no facts | `test_deterministic_failure_no_runtime_facts_left` | Domain disabled | ✅ Receipt persisted (COMPLETED with error), no instance |
| Deterministic failure → replayable | `test_deterministic_failure_replayable` | Same as above | ✅ Second call returns same error |
| Deferred FK resolution at commit | `test_deferred_fk_committed_successfully` | Normal creation | ✅ All FKs resolve |

### 9.2 Gap: Receipt completion failure not independently tested

The report maps "Receipt完成失败回滚 Runtime" to `test_infrastructure_failure_no_residual_receipt`, but this test triggers on **Instance INSERT**, not on **Receipt completion UPDATE**. There is no test that:
1. Successfully inserts instance, context, visit, event
2. Fails on the final `complete_receipt` UPDATE
3. Verifies all runtime facts are rolled back

**Impact**: The path from "all runtime facts committed" to "receipt completion UPDATE fails" is not directly tested. However, since all writes are in the same transaction, any failure before COMMIT will roll back everything — the same mechanism that `test_event_failure_rolls_back_everything` proves. Transactional semantics ensure correctness without a dedicated test for this specific failure point. This is **Medium** at most.

### 9.3 Cleanup concern

The temporary trigger/function injection uses `CREATE OR REPLACE` on global objects (`fn_test_fail_event`, `trg_test_fail_event`, etc.) and cleans up with `DROP ... IF EXISTS`. This works in serial mode but causes **test pollution** in parallel mode (see §12).

### 9.4 Verdict: ✅ Atomicity tests cover the key failure modes. Receipt-completion failure is theoretically covered by tx rollback but not independently tested.

---

## 10. Migration 0009

### 10.1 Migration content

```sql
ALTER TABLE workflow_instances
    ADD COLUMN IF NOT EXISTS external_reference TEXT
    CHECK (external_reference IS NULL OR char_length(external_reference) <= 512);
```

### 10.2 Verification

| Check | Status |
|---|---|
| Applies on empty DB (0001–0009) | ✅ (verified by migration tests) |
| Upgrades from 0001–0008 to 0009 | ✅ |
| Re-applies safely | ✅ `IF NOT EXISTS` prevents duplicate column error |
| Column type matches contract | ✅ `TEXT` nullable |
| Length constraint | ✅ `char_length <= 512` |
| Constraint name | ⚠️ Auto-generated as `workflow_instances_external_reference_check` (not a named `chk_*` constraint) |

### 10.3 `IF NOT EXISTS` risk

The `IF NOT EXISTS` clause would silently skip adding the column if `external_reference` already existed with a different type (e.g., `INTEGER`). This is a theoretical risk in edge-case schema repair scenarios, not a practical concern for normal migration flow. Migration safety is standard for `sqlx::migrate!` which tracks already-applied migrations.

### 10.4 Contract synchronization

The Storage Contract §11 (audit fix summary) does NOT mention `external_reference` or Migration 0009 — it was written before this migration was added. The Instance Create Contract §11 does document it. The Storage Contract should be updated to reflect the new column.

### 10.5 Verdict: ✅ Migration is safe and correct

---

## 11. Code Structure & Guards

| Metric | Value | Limit | Status |
|---|---|---|---|
| Max file size | 446 lines (`src/domain/definition/graph_tests.rs`) | 500 | ✅ |
| Create module size | 434 lines (`create_transaction.rs`) | 500 | ✅ |
| `tests/` direct children | 20 (17 `.rs` + 3 dirs) | 20 | ⚠️ At limit |
| Max directory depth | 4 | 4 | ✅ |
| No `utils.rs` | ✅ | — | ✅ |
| CommandReceipt framework scoped to PR | ✅ | — | ✅ |

### Verdict: ✅ Structure is clean

---

## 12. Default Parallel Test Failure Analysis

### 12.1 Failure pattern

| Run | Failed tests | Root cause |
|---|---|---|
| Serial (1 thread) | 0 | — |
| Parallel run 1 | `test_command_id_matches_event` | Trigger `trg_test_fail_event` active |
| Parallel run 2 | `test_command_id_matches_event`, `test_exactly_one_event_per_creation` | Triggers `trg_test_fail_event` and `trg_test_fail_instance` active |
| Parallel run 3 | `test_deferred_fk_committed_successfully` | Trigger `trg_test_fail_instance` active |

### 12.2 Root cause

The fault-injection tests (`test_event_failure_rolls_back_everything`, `test_infrastructure_failure_no_residual_receipt`) use **global DDL** to install temporary triggers:

```sql
CREATE OR REPLACE FUNCTION fn_test_fail_event() RETURNS TRIGGER ...
CREATE OR REPLACE TRIGGER trg_test_fail_event BEFORE INSERT ON workflow_events ...
```

These triggers are **database-global**, not session-local. When tests run in parallel threads:
1. Test A installs a trigger
2. Before Test A cleans up (DROP TRIGGER/DROP FUNCTION), Test B runs its own creation
3. Test B's creation is blocked by the trigger installed by Test A

The cleanup code (`DROP ... IF EXISTS`) runs **after** the test assertion, so if a concurrent test hits the trigger before cleanup, it fails.

### 12.3 Contributing factors

- All tests share a **single database** `svc_workflow` — no per-test database or schema isolation
- Each test binary runs `sqlx::migrate::Migrator::run()` which applies all pending migrations — is idempotent but adds startup overhead
- The trigger names (`trg_test_fail_event`, `trg_test_fail_instance`) conflict across test threads via `CREATE OR REPLACE`

### 12.4 Severity

**High** — default `cargo test` (which is `cargo test -- --test-threads=0`, i.e., automatic parallelism) consistently fails with test pollution.

### 12.5 Minimum fix (not implementing, just recommending)

1. **Option A** (preferred): Use `CREATE TRIGGER ... ON ...` without `OR REPLACE`, and use unique trigger names per test invocation (e.g., append a UUID). Clean up in `DROP TRIGGER IF EXISTS` with the same unique name.
2. **Option B**: Run fault-injection tests in serial (`#[serial_test::serial]`).
3. **Option C**: Use a separate database/schema for each test binary (via `PGDATABASE` or `PGSCHEMA`).

### 12.6 Verdict: ⚠️ High — parallel tests consistently fail due to trigger pollution

---

## 13. 40-Item Mapping Verification

After reviewing the test files against the contract requirements:

| # | Requirement | Test(s) | Status |
|---|---|---|---|
| 1–8 | Normal creation | `normal_create.rs` (8 tests) | ✅ |
| 9–14 | Definition gates | `definition_gates.rs` (6 tests) | ✅ |
| 15–19 | Authorization | `authorization.rs` (5 tests) | ✅ |
| 20–24 | Context validation | `context_validation.rs` (4 tests) | ✅ (but none with non-NULL schema) |
| 25–34 | Idempotency | `idempotency.rs` (10 tests) | ✅ |
| 35 | Deterministic failure → no facts | `atomicity.rs:test_deterministic_failure_no_runtime_facts_left` | ✅ |
| 36 | Infrastructure failure → no receipt | `atomicity.rs:test_infrastructure_failure_no_residual_receipt` | ✅ (maps to both 34 and 36 in report — single test) |
| 37 | Deterministic failure replayable | `idempotency.rs:test_deterministic_failure_replayable` | ✅ |
| 38 | Exactly one event per creation | `atomicity.rs:test_exactly_one_event_per_creation` | ✅ |
| 39 | Event field matrix | `normal_create.rs:test_create_event_field_matrix_correct` | ✅ |
| 40 | Context digest readback | `normal_create.rs:test_create_context_digest_readback` | ✅ |

### Mapping issues found

- **Item 34 & 36 map to the same test** (`test_infrastructure_failure_no_residual_receipt`). This test verifies infrastructure failure (instance INSERT blocked) and checks no receipt remains. But the report items describe different scenarios:
  - Item 34: Infrastructure failure → no residual receipt ✅
  - Item 36: Receipt completion failure → rollback runtime facts ⚠️ (not independently tested, see §9.2)
  
- **Item 39 & 40 are distinct tests** in the actual code (`test_create_event_field_matrix_correct` and `test_create_context_digest_readback`) — they are not duplicates.

- **Non-null context_schema tests are missing**: No test seeds a definition with `context_schema = { ... }` and validates payloads against it. Items 20–24 appear "covered" but none exercise the `Some(schema)` branch.

---

## 14. Findings

### 14.1 Blocker — None

No blocker-level issue was found. The core transaction is atomic, idempotency is correct, authorization is properly enforced, and the event matrix matches the contract.

### 14.2 High

| # | Finding | Details | Severity |
|---|---|---|---|
| H1 | **Default parallel test consistently fails** | 3/3 parallel runs failed with 1–2 failures each due to global DDL trigger pollution from fault-injection tests. | **High** |
| H2 | **Non-null context_schema validation untested** | All 39 create tests use `context_schema = NULL`. The `validate_context_schema()` `Some(schema)` branch has zero test coverage. While the code reads correctly, this is a core contract guarantee that is not demonstrated. | **High** |

### 14.3 Medium

| # | Finding | Details |
|---|---|---|
| M1 | Report contradiction on infrastructure failure | Implementation report says "基础设施不回滚 Receipt" but actual code rolls back the receipt with the transaction on infrastructure failure. Correct behavior matches "基础设施失败无残留 Receipt". |
| M2 | requestHash contract/documentation mismatch | Contract shows camelCase keys (`commandSchemaVersion`, `requestBody`, `principalId`) but code produces snake_case. Hash is self-consistent, but contract should match implementation. |
| M3 | Receipt completion failure not independently tested | The final `complete_receipt` UPDATE failure path is covered by general transaction rollback semantics but has no dedicated fault-injection test. |
| M4 | tests/ directory at boundary limit | 20 direct children (17 .rs + 3 subdirs) — exactly at the 20 limit, no room for growth without reorganization. |

### 14.4 Low / Notes

| # | Finding |
|---|---|
| L1 | Migration 0009 constraint name is auto-generated, not a named `chk_*` constraint |
| L2 | Event type string `INSTANCE_CREATED` differs from architecture doc's `WORKFLOW_INSTANCE_CREATED` — matches instance create contract, so not an implementation issue |
| L3 | `external_url` URI scheme allowlist not implemented (explicitly deferred for v0, contract states `file://` should be rejected but no test) |
| L4 | Storage Contract §11 (audit fix) does not mention `external_reference` or Migration 0009 — needs update |
| L5 | `pre_validate_principal` runs before transaction — acceptable because principal is re-verified inside transaction |

---

## 15. Verdict

```
SVC_WORKFLOW_INSTANCE_CREATE_AUDIT_PASS_WITH_NOTES
```

### Rationale

**No Blocker found.** The core transaction is atomic, idempotent, and consistent. Authorization, assignee resolution, context digest, and event field matrix all match the frozen contracts.

**Two High issues exist** (H1: parallel test pollution, H2: context schema validation untested), but neither causes incorrect production behavior for the tested scenarios. The context validation code reads correctly — the gap is test coverage, not logic error. The parallel test pollution is a test infrastructure issue, not a production correctness issue.

### Merge condition

**Allow merge with the following conditions:**

1. **H1 (parallel test pollution)** must be addressed in a follow-up or the same PR before the next environment uses `cargo test` as its CI entry point. If CI runs `cargo test -- --test-threads=1`, this can be deferred. **Recommendation**: File a follow-up issue and fix within same milestone.

2. **H2 (context schema validation test gap)** should add at least one test with a non-null `context_schema` that exercises both valid and invalid payloads. **Recommendation**: Add before merge — it's one test function and one seed variant.

3. The implementation report's claim "基础设施不回滚 Receipt" should be corrected to match the code.

### Minimal fix recommendations

1. **For H1**: In `atomicity.rs`, use a scope-guarded approach for trigger injection — or add `#[serial_test::serial]` (from the `serial_test` crate) to the fault-injection tests, or use unique trigger names with a UUID suffix to avoid cross-test interference.

2. **For H2**: Add a test helper `seed_published_definition_with_schema()` that sets `context_schema` to a non-null JSON Schema, then add tests:
   - `test_context_schema_valid_payload_accepted`
   - `test_context_schema_invalid_payload_rejected`

---

## 16. Summary

| Question | Answer |
|---|---|
| Is creation atomic? | ✅ Yes, single transaction |
| Is idempotency concurrent-safe? | ✅ Yes, INSERT ON CONFLICT + FOR UPDATE |
| Are deterministic vs infrastructure failure semantics correct? | ✅ Yes. Deterministic → COMPLETED receipt + error + no facts. Infrastructure → full rollback. |
| Is context schema truly validated? | ⚠️ Code is correct but untested for non-null schemas |
| Is event field matrix correct? | ✅ Yes |
| Are there any 40-item mapping errors? | ⚠️ Item 34/36 share same test; item 36 not independently tested; no non-null schema test |
| What causes parallel test failure? | Global DDL triggers from fault-injection tests leak to concurrent threads |
| PostgreSQL version | 16.14 |
| Migration count | 9 |
| Unit / Integration / Create tests | 54 / 156 / 39 |
| Report path | `./WORKFLOW_INSTANCE_CREATE_AUDIT_REPORT.md` |
| `git status --short` | (clean) |
| Allow merge? | **Yes, with conditions** (see §15) |
| Final status | `SVC_WORKFLOW_INSTANCE_CREATE_AUDIT_PASS_WITH_NOTES` |
