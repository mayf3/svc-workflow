# Workflow Definition & Immutable Version Publishing Service — Audit Report

## 1. Review Metadata

| Field | Value |
|---|---|
| Repository | `/Users/yanfenma/workspace/project/svc-workflow` |
| Branch | `feat/definition-version-service-v0` |
| Base SHA | `d8e980869a28d85518d622e269bc20cd0ea37632` |
| Audit HEAD | `4f5d84c653426fd3d23068df74abffa14385abf3` |
| Frozen Architecture Tag | `svc-workflow-architecture-v0.3.1-frozen` |
| PostgreSQL Version | PostgreSQL 16.14 (Homebrew) on x86_64-apple-darwin |
| Test Database | `svc_workflow` on `localhost:5432` |

### Reviewed Commits

```
a64e41a97a080e27365e372c2ea48921295849f6  feat: add workflow definition version service
a90d5f1b62dcae78c75244d8f35cd85fa69879eb  refactor: align definition service structure and verification
4f5d84c653426fd3d23068df74abffa14385abf3  refactor: finalize definition service audit baseline
```

### Pre-review State Confirmation

```
git status --short    → (clean)
git branch --show-current → feat/definition-version-service-v0
git rev-parse HEAD    → 4f5d84c653426fd3d23068df74abffa14385abf3
git merge-base HEAD d8e9808 → d8e980869a28d85518d622e269bc20cd0ea37632
```

Working tree clean, HEAD correct, merge-base correct, audit scope is PR 2 only.

---

## 2. Contract Hierarchy Applied

```
SVC_WORKFLOW_ARCHITECTURE_V0_3_1 (FROZEN)
  → IMPLEMENTATION_CONTRACT_V0_1
  → POSTGRES_STORAGE_CONTRACT_V0_1
  → DEFINITION_SERVICE_CONTRACT_V0_1
  → Actual code and tests
```

No personal preference used to overturn frozen architecture.

---

## 3. PR Scope Verification

### In-scope items (present)

- ✅ Workflow Definition
- ✅ Draft Definition Version
- ✅ Draft Graph atomic replacement (`replace_draft_graph`)
- ✅ Publication validation (`validate_graph` + schema checks)
- ✅ Definition Digest (JCS + SHA-256)
- ✅ Publish / Deprecate / Revoke
- ✅ Internal queries (`get_definition`, `get_definition_version`, `list_definition_versions`, `get_complete_version_graph`)
- ✅ Minimal Domain Owner permission (`ensure_domain_owner`)
- ✅ PostgreSQL Repository (`PgDefinitionRepository`)
- ✅ Migration 0008 (`0008_fix_graph_definition_version_escape.sql`)
- ✅ Tests: `13_definition_service_tests.rs`, `14_definition_lifecycle_tests.rs`, `15_definition_graph_parent_move_tests.rs`

### Out-of-scope items (absent)

- ✅ No Workflow Instance code
- ✅ No Context Revision commands
- ✅ No Submission
- ✅ No Runtime Transition Engine
- ✅ No CommandReceipt idempotency framework
- ✅ No HTTP API
- ✅ No assigned-to-me
- ✅ No Legacy import
- ✅ No ADC or llm-todo modifications

**PR scope is clean.**

---

## 4. Structure Guard Results

| Check | Result |
|---|---|
| Max file ≤ 500 lines | ✅ Largest: `src/store/postgres/definition_repository.rs` = 493 lines |
| Directory depth ≤ 4 | ✅ Max depth = 2 |
| Direct children ≤ 20 | ✅ Max = 16 (`./tests`, `.`) |

All structure guards pass.

---

## 5. Lifecycle Audit (CreateDefinition / CreateDraftVersion / ReplaceDraftGraph / ValidateDraftVersion / Publish / Deprecate / Revoke / Get)

### 5.1 CreateDefinition

| Rule | Status | Note |
|---|---|---|
| Actor must exist and be enabled | ✅ | `ensure_principal_enabled` |
| Domain must exist and be enabled | ✅ | `ensure_domain_enabled` |
| Only current DOMAIN_OWNER may create | ✅ | `ensure_domain_owner` checks `domain_role_bindings` |
| `definition_key` unique within Domain | ⚠️ | See High-1 |
| Different Domains can share key | ✅ | Uniqueness is `(domain_id, definition_key)` |
| Concurrent create same key — no duplicate | ✅ | DB unique index + 23505 mapping |
| Unique conflict → stable domain error | ✅ | `DefinitionKeyConflict` |
| Length / null rules | ⚠️ | Returns `StorageError` instead of a stable validation error |

**High-1 (Check-then-act race in CreateDefinition):** `create_definition` first calls
`definition_key_exists` (autocommit `SELECT COUNT(*)`), then `create_definition`
(autocommit `INSERT`). Two concurrent `CreateDefinition` calls for the same key can
both pass the existence check; one will then hit the 23505 unique violation and be
mapped to `DefinitionKeyConflict`. The DB unique index prevents duplicate rows, so
this is *safe* but the error path is racy — the loser gets `DefinitionKeyConflict`
rather than the check-then-insert being atomic. Acceptable but worth noting: the
`definition_key_exists` pre-check is redundant defense and the authoritative
guarantee is the DB constraint.

### 5.2 CreateDraftVersion

| Rule | Status | Note |
|---|---|---|
| Only DRAFT created | ✅ | INSERT hardcodes `'DRAFT'` |
| version_number monotonic | ⚠️ | See High-2 |
| Concurrent create no duplicate number | ✅ | DB unique `(workflow_definition_id, version_number)` + 23505 → `ConcurrentModification` |
| No `MAX(version_number)+1` unlock race | ❌ | See High-2 |
| Multiple DRAFTs allowed | ✅ | Consistent with contract (no DB constraint forbidding it) |
| Definition/Domain/Actor re-confirmed | ⚠️ | See High-3 (not in same transaction) |
| metadata / schema / dialect / validator saved | ✅ | |
| Failure leaves no partial Version | ✅ | Single INSERT |

**High-2 (version_number race):** `next_version_number` computes
`MAX(version_number)+1` in an autocommit query, then `create_draft_version` does a
separate autocommit INSERT. Two concurrent `CreateDraftVersion` calls compute the
same `next_ver`; one succeeds, the other hits the unique constraint and is mapped
to `ConcurrentModification`. The caller must retry. The Definition Service Contract
§7 explicitly documents `MAX(version_number)+1` + unique constraint as the strategy,
so this matches the contract — but it means concurrent draft creation can produce
`ConcurrentModification` errors that the contract attributes to "防止重复". This is
functionally safe (no duplicate numbers) but the contract's claim
"version_number 通过 MAX(version_number) + 1 计算，唯一约束防止重复" is accurately
implemented. **Not a blocker** — the unique index is the real guarantee.

**High-3 (no single transaction for CreateDraftVersion):** The permission check
(`ensure_domain_owner`), version-number computation, and INSERT execute as three
separate autocommit statements on the pool. If the Domain Owner binding is revoked
between the check and the INSERT, the version is still created. Per the contract
§6 the actor must be re-verified, but this is a narrow window. Medium-High.

### 5.3 ValidateDraftVersion

- ✅ Locks version (`lock_version` with `FOR UPDATE`)
- ✅ Verifies DRAFT status
- ✅ Runs `validate_graph` + schema validation
- ⚠️ The `FOR UPDATE` lock is released immediately (autocommit), but since
  `validate_draft_version` is read-only and does not mutate state, this is
  acceptable.

### 5.4 PublishVersion — **SEE SECTION 7 (BLOCKER-1)**

### 5.5 DeprecateVersion / RevokeVersion

- ✅ Lock version, verify source status
- ✅ Domain owner check
- ✅ `update_version_status` mapped to correct column (`deprecated_at` / `revoked_at`)
- ❌ Lifecycle actor columns (`deprecated_by_principal_id`,
  `revoked_by_principal_id`) are NEVER set — see High-4
- ✅ DB trigger `trg_definition_version_status_transition` enforces legal transitions
  as a second line of defense

### 5.6 Queries (get_definition / get_definition_version / list_definition_versions / get_complete_version_graph)

- ✅ Each verifies actor principal is enabled
- ❌ No domain-owner / domain-membership authorization on read — see High-5

---

## 6. Draft Graph Atomic Replacement Audit

### `replace_draft_graph` (PostgreSQL repository)

The repository method correctly uses a single explicit transaction:

```
BEGIN
  SELECT version_status ... FOR UPDATE      (lock + verify DRAFT)
  DELETE FROM workflow_transition_definitions (old transitions)
  DELETE FROM workflow_node_definitions       (old nodes)
  INSERT nodes (loop)
  INSERT transitions (loop)
  UPDATE context_schema
COMMIT
```

| Rule | Status |
|---|---|
| Single transaction | ✅ (`pool.begin()` … `tx.commit()`) |
| Lock version row first | ✅ (`FOR UPDATE` inside tx) |
| Verify DRAFT inside lock | ✅ |
| Delete transitions before nodes | ✅ (respects FK) |
| Insert nodes before transitions | ✅ (transitions reference nodes) |
| Update context schema | ✅ |
| Order: primary FK is DEFERRABLE | ✅ (`fk_primary_advance_transition`) |

**Application-layer `replace_draft_graph`:** performs its own `lock_version`
(autocommit) *before* calling the repository's transactional replace. This means
the application-level DRAFT check and the transaction-level DRAFT check are
separate, but the transactional check is authoritative. **Acceptable.**

### Concern (Medium-1): application validates graph BEFORE the transaction

`draft_graph.rs` builds and validates the graph in memory, then calls the repo to
write. The graph written is the one the service validated. This is correct. However,
the `context_schema` update inside the transaction only runs `if let Some(schema)`
— if the command passes `None` for context_schema, the existing context_schema is
retained (not nulled). This matches "update context_schema" semantics only when a
new schema is supplied. Minor.

---

## 7. Publish Concurrency Safety — **BLOCKER**

This is the most important audit area and it contains a **Blocker**.

### 7.1 The publish flow is NOT a single transaction

`publish_version` in `lifecycle.rs` executes these steps as **independent autocommit
statements** on the connection pool (no surrounding `BEGIN`/`COMMIT`):

```
1. ensure_principal_enabled(actor)        — autocommit SELECT
2. lock_version(version_id)               — autocommit SELECT ... FOR UPDATE  ← LOCK RELEASED HERE
3. get_definition(def_id)                 — autocommit SELECT
4. ensure_domain_owner                    — autocommit SELECT
5. get_complete_graph(version_id)         — autocommit SELECT nodes + transitions
6. validate_graph (in-memory)
7. validate_fixed_principals              — autocommit SELECT per principal
8. compute_digest (in-memory, from step-5 data)
9. repo.publish_version(version_id, digest) — autocommit UPDATE ... SET status='PUBLISHED', digest=...
```

Evidence: `src/store/postgres/definition_repository.rs:387-401` — `lock_version`
calls `fetch_optional(&self.pool)` (line 395), which runs in an **implicit autocommit
transaction**. The `FOR UPDATE` row lock is released the instant the query returns,
long before step 9. Only `replace_draft_graph` (line 257) uses `pool.begin()`.

### 7.2 Concrete race: Publish vs ReplaceDraftGraph → digest/graph mismatch

The frozen Definition Service Contract §2 and §7 require:

> ReplaceDraftGraph 与 Publish：通过行锁序列化，不会同时成功
> Digest 计算基于锁内读取的一致数据

The implementation does **not** honor this. Consider two concurrent transactions:

```
Tx A (Publish):                        Tx B (ReplaceDraftGraph):
  step 2: lock_version (FOR UPDATE,
           autocommit) → DRAFT, lock RELEASED
                                       begin()
                                         SELECT ... FOR UPDATE (waits? no — A's lock is gone)
                                         → DRAFT, obtains lock
                                         DELETE old graph
                                         INSERT new graph B
                                         COMMIT  ← graph is now B
  step 5: get_complete_graph → reads graph B
           (or graph A if B hadn't committed yet — non-deterministic)
  step 8: compute_digest(graph B)
  step 9: UPDATE status='PUBLISHED', digest=digest(B)
           WHERE version_status='DRAFT'  ← succeeds
```

Depending on timing:
- If Tx B commits **between A's step 5 and A's step 9**: A computes digest from
  graph B, publishes graph B. Consistent — but only by luck of statement ordering.
- If Tx B commits **between A's step 2 and A's step 5**: A's step 5 reads graph B
  (READ COMMITTED sees B's committed version), digest matches graph B. Consistent
  but accidental.
- If Tx B is mid-transaction (uncommitted) when A's step 5 runs: A reads graph A
  (old), computes digest(graph A). Tx B then commits graph B. A's step 9 publishes
  with digest(A) **while the stored graph is B**. **PUBLISHED digest ≠ PUBLISHED
  graph.** This violates the frozen invariant
  "发布后图结构和业务字段冻结" and "Digest 与发布图不一致" (Blocker criterion §18).

### 7.3 Why the DB triggers do NOT save this

The graph immutability trigger `fn_check_definition_graph_immutable()` (migration
0008) queries `workflow_definition_versions.version_status` via a plain `SELECT`
inside the trigger function. In READ COMMITTED this is a non-locking snapshot read.
While the parent version is DRAFT, the trigger permits child writes. Because the
publish UPDATE (step 9) runs in autocommit and only takes its exclusive lock at
statement time, it does **not** serialize against a concurrent ReplaceDraftGraph
transaction that is already in flight.

The only structural lock interaction is the non-deferrable FK from
`workflow_node_definitions.definition_version_id` →
`workflow_definition_versions.definition_version_id`, which acquires a
`FOR KEY SHARE` lock on the parent row on child INSERT/UPDATE. `FOR KEY SHARE`
conflicts with `FOR UPDATE`, so once the publish UPDATE begins executing it blocks
a child writer — but the publish **digest was already computed before the UPDATE
began**, so the serialization comes too late for digest consistency.

### 7.4 Verdict on publish concurrency

**BLOCKER-1: PublishVersion does not hold the version row lock across digest
computation and status update.** The frozen contract requires the entire publish
operation (lock → read consistent graph → validate → compute digest → update
status + digest → atomic commit) to be one transaction. The implementation splits
it across multiple autocommit statements, creating a real, reproducible window
where a published version's persisted digest can correspond to a graph that was
replaced before the UPDATE ran. This is explicitly listed as a Blocker in §18:
"发布后图可变或 Digest 与发布图不一致" and "发布事务存在可复现的严重竞态".

The same structural flaw affects `validate_draft_version` and
`deprecate_version`/`revoke_version` (their `lock_version` is also autocommit and
disconnected from the subsequent `update_version_status`), though for
deprecate/revoke the consequence is a narrower status-flip race rather than a
digest mismatch.

---

## 8. Graph Validation Audit

### 8.1 Node rules

| Rule | Status | Note |
|---|---|---|
| ≥ 2 nodes | ✅ | `MIN_NODES` |
| Exactly one DRAFT | ✅ | `NO_DRAFT_NODE` / `MULTIPLE_DRAFT_NODES` |
| DRAFT is sole entry | ⚠️ | Implied by reachability, not explicitly enforced as "no other node has no incoming edges" |
| ≥ 1 TERMINAL | ✅ | `NO_TERMINAL_NODE` |
| `node_key` unique | ⚠️ | Comment claims hashmap checks it, but `nodes_by_key` is built without conflict detection — duplicate keys silently overwrite. DB unique index catches it at storage time. |
| `order_index` unique | ✅ | `DUPLICATE_ORDER_INDEX` |
| Terminal has no assignee | ❌ | See High-6 — not validated |
| Non-terminal has valid assignee | ⚠️ | Loop body is empty (lines 96-101) — no actual check |
| Terminal has no outgoing | ✅ | `TERMINAL_HAS_OUTGOING` |

### 8.2 Primary trunk

| Rule | Status | Note |
|---|---|---|
| Each non-terminal has exactly one primary | ✅ | `MISSING_PRIMARY` / `TERMINAL_HAS_PRIMARY` |
| Primary originates from node | ✅ | `PRIMARY_NOT_FROM_NODE` |
| Primary target orderIndex higher | ✅ | `PRIMARY_NOT_ADVANCING` |
| Primary effect must be ADVANCE | ❌ | See High-7 — not checked |
| Primary trunk acyclic | ✅ | `PRIMARY_CYCLE` |
| Trunk reaches TERMINAL | ✅ | `PRIMARY_TRUNK_NO_TERMINAL` |
| All nodes from DRAFT reachable | ❌ | See High-8 (BLOCKER-grade per §8.5) |

### 8.3 RETURN

| Rule | Status |
|---|---|
| Target non-terminal | ✅ `RETURN_TO_TERMINAL` |
| Target orderIndex < source | ✅ `RETURN_NOT_BACKWARD` |
| Not primary | ✅ `RETURN_IS_PRIMARY` |
| source ≠ target | ✅ (covered by `SELF_LOOP`) |

### 8.4 TERMINATE

| Rule | Status |
|---|---|
| Not primary | ✅ `TERMINATE_IS_PRIMARY` |
| Target is TERMINAL | ✅ `TERMINATE_TO_NON_TERMINAL` |

### 8.5 Reachability — **High-8**

`graph_helpers.rs::compute_weakly_reachable` traverses transitions in **both
directions** (lines 21-27: follows `source→target` AND `target→source`). This
computes the **weakly connected component** containing the draft node, not the set
of nodes **directed-reachable** from draft.

The frozen architecture §10.4 and §33.7 require "所有 Node 从 Draft 可达" (directed
reachability). A graph where a node X is only reachable via a RETURN edge
(`some_node → X` where `some_node` is downstream of X) would be incorrectly accepted
by weak connectivity but should be rejected: X has no directed path from Draft.

Concrete counterexample accepted by the current validator:

```
draft (order 0) --ADVANCE--> review (order 1)
review  (order 1) --ADVANCE--> done (order 2, terminal)
done    has an edge only reachable backwards? No — but consider:
draft (order 0) --ADVANCE--> a (order 1)
a      (order 1) --ADVANCE--> done (order 2, terminal)
orphan (order 3, terminal)  <-- no incoming, no outgoing from draft path
```
Weak connectivity from draft reaches {draft, a, done} — `orphan` is correctly
flagged `NODE_NOT_REACHABLE`. BUT:

```
draft (0) --ADVANCE--> review (1) --ADVANCE--> done (2, terminal)
review (1) --RETURN--> earlier_node_not_on_trunk (0.5)
```
If `earlier_node_not_on_trunk` has no ADVANCE from draft, the weak traversal still
reaches it via the RETURN edge running backwards. The node is *not* directed-reachable
from draft (you cannot ADVANCE to it), yet the validator accepts it. This violates
the frozen directed-reachability rule.

Severity: The contract §8.5 explicitly states "如果'所有节点从 Draft 可达'实际上没有
实现，这是 High." → **High-8.**

The unit test `node_not_reachable_from_draft` (graph_tests.rs:189) does NOT actually
test this — it adds a fully disconnected node and asserts `MISSING_PRIMARY` (line 212),
not `NODE_NOT_REACHABLE`. So the gap is both real and untested.

### 8.6 Assignee rules

| Rule | Status | Note |
|---|---|---|
| DRAFT must be WORKFLOW_CREATOR | ✅ | `DRAFT_NOT_WORKFLOW_CREATOR` |
| Terminal has no assignee | ❌ | See High-6 — `TERMINAL` branch in code is a no-op comment |
| Non-terminal must have assignee ref | ❌ | See High-6 — empty loop body |
| FIXED_PRINCIPAL must provide ID | ✅ | `FIXED_PRINCIPAL_MISSING_ID` |
| Non-FIXED must not provide ID | ✅ | `UNEXPECTED_FIXED_PRINCIPAL` |
| FIXED_PRINCIPAL exists & enabled | ✅ | `validate_fixed_principals` queries DB |

**High-6 (Terminal / non-terminal assignee rules not enforced):** `graph.rs:96-101`
and `graph.rs:450-453` contain empty match arms with only comments. The contract
§3.1.7 ("Terminal Node 没有负责人") and §3.1.8 ("非 Terminal Node 必须具有合法负责人引用")
are not validated. A TERMINAL node with `FIXED_PRINCIPAL` assignee, or a NORMAL node
with no assignee ref type, passes validation.

**High-7 (primary effect not verified):** The validator checks that the primary
transition originates from the node and advances orderIndex, but never checks that
`transition_effect == ADVANCE`. A node could point its `primary_advance_transition_id`
at a RETURN or TERMINATE transition and pass (the effect-mismatch is only caught
incidentally for RETURN/TERMINATE via `RETURN_IS_PRIMARY`/`TERMINATE_IS_PRIMARY`,
but only when the transition's own effect is RETURN/TERMINATE — an ADVANCE-labeled
primary that semantically should not be primary is not at issue here; the real gap
is the contract §3.2 "primary effect 必须为 ADVANCE" which is implied but the code
does not explicitly assert the primary transition's effect).

---

## 9. JSON Schema Compilation Audit — **BLOCKER-2**

`src/application/definition/lifecycle.rs:409-429`:

```rust
fn validate_json_schema(schema: &serde_json::Value) -> Result<(), String> {
    if !schema.is_object() {
        return Err("schema must be a JSON object".to_string());
    }
    let obj = schema.as_object().unwrap();
    if let Some(schema_field) = obj.get("$schema") {
        if !schema_field.is_string() {
            return Err("$schema must be a string".to_string());
        }
    }
    // Try to compile with jsonschema
    let compiled = jsonschema::validator_for(schema);
    // If it compiles without panic, it's syntactically valid
    let _ = compiled;
    Ok(())
}
```

`jsonschema::validator_for(schema)` returns `Result<Validator, ValidationError>`
(verified in the crate source: `validator_for` → `Validator::new`). The code
**discards the Result** with `let _ = compiled;` and unconditionally returns `Ok(())`.

**Effect:** Every `context_schema` and `submission_schema` passes "validation",
including schemas that fail to compile (invalid `$ref`, malformed keywords, type
errors). The contract §3.7 and §10 require schemas to be "真实编译" (truly compiled)
before publish. The architecture §10.4 requires "所有 Context 和 Submission Schema
合法".

This is a **Blocker**: invalid schemas can be published, contradicting the frozen
rule "Draft 编辑时可以暂存无效 Schema，Publish 必须拒绝" and the explicit Blocker
criterion "Graph 校验明显不满足冻结架构" (validation does not enforce a frozen
rule). Additionally, `ValidateDraftVersion` and `PublishVersion` use the same
no-op validator, so neither catches invalid schemas.

**Secondary concern (Medium-2):** the `jsonschema` crate (0.47) is compiled with
`reqwest` + `rustls`, meaning it can resolve remote `$ref` over the network. The
contract §10 warns about SSRF / file-read risk from external references. There is
no configuration disabling remote resolution. If the validator were actually
invoked (after fixing Blocker-2), a malicious schema with `$ref:
"https://internal/..."` could trigger outbound HTTP during publish. This is
currently dormant (because validation is a no-op) but must be addressed when
Blocker-2 is fixed.

---

## 10. Canonical Document & Digest Audit

### 10.1 Algorithm

`digest.rs` uses `jcs_canonicalize::sha256_jcs_hex(&doc)` which performs
JCS (RFC 8785) → SHA-256 → lowercase hex. ✅ Matches contract §5.

### 10.2 Canonical document coverage

| Field | In Digest | Note |
|---|---|---|
| definition_key | ✅ | |
| version_number | ✅ | |
| json_schema_dialect | ✅ | |
| validator_version | ✅ | |
| context_schema | ✅ | |
| nodes (all) | ✅ | sorted by node_key |
| node_key | ✅ | |
| display_name | ✅ | |
| order_index | ✅ | |
| node_type | ✅ | |
| assignee_ref_type | ✅ | |
| fixed_principal_id | ✅ | |
| instructions | ✅ | |
| primary_advance_transition_key | ✅ | Resolved via transition_key_by_id |
| metadata | ✅ | |
| transitions (all) | ✅ | sorted by transition_key |
| transition_key | ✅ | |
| source_node_key | ✅ | |
| target_node_key | ✅ | |
| transition_effect | ✅ | |
| submission_schema | ✅ | |
| metadata | ✅ | |

### 10.3 Excluded fields

| Field | Excluded | Note |
|---|---|---|
| node_id, transition_id | ✅ | Uses keys |
| created_at, updated_at | ✅ | |
| published_at, published_by | ✅ | |
| definition_version_id | ✅ | |

### 10.4 Sorting & determinism

- ✅ Nodes sorted by `node_key` (line 111)
- ✅ Transitions sorted by `transition_key` (line 139)
- ✅ primary uses transition_key (not UUID)
- ✅ JCS handles key order, number canonicalization
- ✅ Tests verify key-order, node-order, transition-order independence

### 10.5 Digest consistency with published graph

❌ **Undermined by Blocker-1**: even though the digest algorithm itself is correct
and deterministic, the digest is computed from graph data read in a separate
autocommit query and written in another autocommit UPDATE. A concurrent
ReplaceDraftGraph can change the graph between read and write, so the persisted
digest may not correspond to the persisted graph. The digest algorithm is sound;
the *use* of it is not transactionally safe.

### 10.6 Read-back consistency

There is no test that re-computes the digest from a published version's graph and
asserts equality with the stored `definition_digest`. The `test_publish_persists_digest`
test only checks length == 64. **Medium-3 (test gap).**

---

## 11. Migration 0008 Audit

### 11.1 Migration mechanics

| Check | Result |
|---|---|
| Applies on fresh DB (0001–0008) | ✅ Verified on `audit_def_fresh1`, `audit_def_fresh2` |
| Applies as upgrade (0001–0007 → 0008) | ✅ Verified on `audit_def_upgrade` |
| No developer-machine objects | ✅ |
| No seed/business data | ✅ |
| `CREATE OR REPLACE FUNCTION` (idempotent) | ✅ |

### 11.2 Trigger function correctness

The new `fn_check_definition_graph_immutable()`:

- INSERT: checks `NEW.definition_version_id` parent is DRAFT ✅
- UPDATE: checks `NEW.definition_version_id` parent is DRAFT ✅
  AND if `OLD.definition_version_id IS DISTINCT FROM NEW.definition_version_id`,
  also checks `OLD` parent is DRAFT ✅ (closes the PUBLISHED→DRAFT move escape)
- DELETE: checks `OLD.definition_version_id` parent is DRAFT ✅
- Parent missing → RAISE ✅ (lines 35-40, 74-79, 97-102)
- No `SECURITY DEFINER` ✅
- `LANGUAGE plpgsql` ✅
- ERRCODE `23000` ✅
- Error prefix `graph_immutable:` ✅

### 11.3 Parent-move tests (file 15)

All five scenarios verified with direct SQL:

| Scenario | Test | Result |
|---|---|---|
| PUBLISHED → DRAFT node move | `test_published_to_draft_node_move_rejected` | ✅ rejected |
| DRAFT → PUBLISHED node move | `test_draft_to_published_node_move_rejected` | ✅ rejected |
| PUBLISHED → DRAFT transition move | `test_published_to_draft_transition_move_rejected` | ✅ rejected |
| DRAFT → PUBLISHED transition move | `test_draft_to_published_transition_move_rejected` | ✅ rejected |
| DRAFT → DRAFT node move | `test_draft_to_draft_node_move_allowed` | ✅ allowed |

### 11.4 Lifecycle actor columns — **High-4**

Migration 0008 adds:

```sql
ALTER TABLE workflow_definition_versions
    ADD COLUMN IF NOT EXISTS published_by_principal_id UUID REFERENCES principals(principal_id),
    ADD COLUMN IF NOT EXISTS deprecated_by_principal_id UUID REFERENCES principals(principal_id),
    ADD COLUMN IF NOT EXISTS revoked_by_principal_id UUID REFERENCES principals(principal_id);
```

The DDL is correct (FK to principals, nullable). **However:**

1. The `publish_version` UPDATE (repository line 408-413) sets `published_at` but
   **never sets `published_by_principal_id`**.
2. The `update_version_status` (repository line 422-448) sets `deprecated_at` /
   `revoked_at` but **never sets `deprecated_by_principal_id` /
   `revoked_by_principal_id`**.
3. The `WorkflowDefinitionVersionRow` (repository_rows.rs:46-62) does **not SELECT**
   these three columns and hardcodes them to `None` in `into_domain` (lines 86-88).

The migration's stated intent ("track WHO performed each lifecycle action") is
entirely unfulfilled. The model declares the fields, the DB has the columns, but
no write path populates them and no read path returns them. **High-4.**

### 11.5 ON DELETE CASCADE check

All FKs use default `ON DELETE NO ACTION`. Grep confirms no `ON DELETE CASCADE`
anywhere in 0008 or prior migrations. ✅

---

## 12. Permission Boundary & Query Leakage Audit

### 12.1 Write operations

| Operation | Actor check | Domain enabled | DOMAIN_OWNER | Correct domain |
|---|---|---|---|---|
| CreateDefinition | ✅ | ✅ | ✅ | ✅ (owner_domain_id) |
| CreateDraftVersion | ✅ | ⚠️ | ✅ | ✅ (via get_definition_domain) |
| ReplaceDraftGraph | ✅ | ❌ | ✅ | ✅ |
| ValidateDraftVersion | ✅ | ❌ | ❌ | — |
| PublishVersion | ✅ | ❌ | ✅ | ✅ |
| DeprecateVersion | ✅ | ❌ | ✅ | ✅ |
| RevokeVersion | ✅ | ❌ | ✅ | ✅ |

Notes:
- CreateDraftVersion, ReplaceDraftGraph, Publish, Deprecate, Revoke do **not**
  call `ensure_domain_enabled`. A disabled Domain can still have definitions
  published/deprecated/revoked. The contract §6 requires "Domain 必须存在且启用".
  **Medium-4.**
- `ValidateDraftVersion` does not check DOMAIN_OWNER — any enabled principal can
  validate a draft. The contract §6 lists `ValidateDraftVersion` under DOMAIN_OWNER-only
  operations. **High-5 (permission bypass).**

### 12.2 Read operations — **High-5**

| Query | Actor enabled | Domain membership | Cross-domain leak risk |
|---|---|---|---|
| get_definition | ✅ | ❌ | ❌ High |
| get_definition_version | ✅ | ❌ | ❌ High |
| list_definition_versions | ✅ | ❌ | ❌ High |
| get_complete_version_graph | ✅ | ❌ | ❌ High |

None of the four query methods verify the actor belongs to the definition's domain
(or is the Domain Owner). Any enabled principal can read any definition, its
versions, and the complete graph — including `instructions`, `submission_schema`,
`fixed_principal_id` references, and internal node structure — across domains.

The contract §13 warns: "如果查询输入包含 actorPrincipalId，但实现没有授权检查，判断是否
可能泄漏跨 Domain Definition、Schema、Fixed Principal 或内部 instructions。不得假设'内部
API'天然安全。"

This is a real cross-Domain data leak. **High-5.**

---

## 13. Repository & Transaction Boundary Audit

### 13.1 Transaction usage

| Method | Transaction | Note |
|---|---|---|
| `create_definition` | ❌ autocommit | Single INSERT — OK |
| `create_draft_version` | ❌ autocommit | Single INSERT — OK (see High-2 for number race) |
| `replace_draft_graph` | ✅ `pool.begin()` … `tx.commit()` | Correct |
| `lock_version` | ❌ autocommit | **Blocker-1 contributor** |
| `publish_version` | ❌ autocommit | **Blocker-1** |
| `update_version_status` | ❌ autocommit | Status flip not atomic with lock |
| All reads | ❌ autocommit | OK for reads |

### 13.2 Single-transaction invariant

The Definition Service Contract §7 and the frozen architecture require that a
business operation uses exactly one transaction. `ReplaceDraftGraph` satisfies
this. `PublishVersion` violates it — it is a sequence of autocommit statements
with no shared transaction. **Blocker-1 (restated).**

### 13.3 SQL injection / parameter binding

All user input is bound via `.bind(...)`. ✅ No string concatenation of user
input. The `update_version_status` does use `format!` to interpolate a column
name (line 435-438), but the column name is derived from a match on a typed enum,
not user input. ✅ Safe.

### 13.4 Error mapping

| DB condition | Mapping | Note |
|---|---|---|
| 23505 on definition insert | `DefinitionKeyConflict` ✅ | |
| 23505 on version insert | `ConcurrentModification` ✅ | |
| Trigger `graph_immutable:` | `StorageError(raw)` ⚠️ | Raw message leaked; not mapped to a stable domain error |
| Trigger `status_transition` | `StorageError(raw)` ⚠️ | Same |
| Other SQL errors | `StorageError(e.to_string())` ⚠️ | Raw sqlx error string; may include schema details |

**Medium-5:** trigger-raised errors during ReplaceDraftGraph or lifecycle transitions
surface as opaque `StorageError` with the raw Postgres message. The contract §15
requires stable errors and forbids leaking full schema. The raw messages do contain
the trigger-defined prefix (`graph_immutable:`, etc.) which is acceptable, but the
mapping is not to a typed domain error.

---

## 14. Error Model Audit

The `DefinitionError` enum (error.rs) covers all 16 contract-required variants. ✅

| Issue | Severity |
|---|---|
| `create_definition` length validation returns `StorageError` instead of a dedicated validation error | Low |
| `parse_assignee_ref` returns `StorageError` for invalid enum string | Low |
| Trigger errors not mapped to `VersionNotDraft` / `InvalidLifecycleTransition` | Medium-5 |

The error model is structurally sound and HTTP-mappable. Naming and mapping gaps
are Medium/Low.

---

## 15. Test Credibility Audit

### 15.1 Counts

| Source | Count |
|---|---|
| Lib tests (`cargo test --lib -- --list`) | 39 |
| Integration tests | 86 |
| **Total** | **125** |

Matches the claimed "39 unit + 86 integration = 125" and the PR-1 baseline of 73
+ PR-2 increment of 52 = 125.

### 15.2 Test credibility issues

1. **Reachability test mis-targeted (High-8 evidence):** `node_not_reachable_from_draft`
   asserts `MISSING_PRIMARY`, not `NODE_NOT_REACHABLE`. The directed-reachability
   rule is effectively untested.
2. **Schema validation not tested for failure (Blocker-2 evidence):** No test
   publishes a version with a malformed schema and asserts rejection. The existing
   `invalid_json_schema_not_checked_by_graph_validation` test explicitly *documents*
   that graph validation skips schema checks — but the service-level
   `validate_json_schemas` that should catch it is a no-op.
3. **No publish-concurrency test:** No test runs Publish and ReplaceDraftGraph as
   truly concurrent transactions. Blocker-1 is undetected by the suite.
4. **No read-back digest consistency test (Medium-3).**
5. **`published_by` columns never asserted (High-4 evidence):** No lifecycle test
   checks that the actor columns are populated after publish/deprecate/revoke.
6. **Cross-domain read test missing (High-5 evidence):** No test verifies that a
   principal from domain B cannot read definitions in domain A.
7. **Seed helper violates contract:** `seed_workflow_definition` (common/mod.rs:168)
   creates the draft node with `assignee_ref_type = 'FIXED_PRINCIPAL'`, violating
   "Draft Node 必须是 WORKFLOW_CREATOR". This seed data would fail the real
   `validate_graph` if the tests exercised it. Low — but indicates the seed
   fixture diverges from valid domain data.

### 15.3 Run results

All 125 tests pass in all three required configurations (see §16).

---

## 16. Commands Actually Executed and Results

```
cargo fmt --check                                   → PASS (exit 0)
cargo build                                         → PASS (Finished dev profile)
cargo clippy --all-targets --all-features -D warnings → PASS (exit 0)
cargo test -- --test-threads=1                      → 125 passed, 0 failed
cargo test (parallel, run 1)                        → 125 passed, 0 failed
cargo test (parallel, run 2)                        → 125 passed, 0 failed
git diff --check                                    → PASS (exit 0)
```

### Test list commands

```
cargo test --lib -- --list    → 39 tests
cargo test --tests -- --list  → 125 tests (includes lib + integration)
cargo test -- --list          → 125 tests
```

### Migration verification (three databases)

| Database | Path | Result |
|---|---|---|
| `audit_def_fresh1` | empty → 0001–0008 | ✅ Applied, 18 enum values, 6 graph triggers |
| `audit_def_upgrade` | 0001–0007 → 0008 | ✅ Applied; 3 actor columns added |
| `audit_def_fresh2` | empty → 0001–0008 | ✅ Applied, 6 graph triggers |

All three verification databases were dropped after verification.

### PostgreSQL environment

```
PostgreSQL 16.14 (Homebrew) on x86_64-apple-darwin23.6.0
Test DB: svc_workflow @ localhost:5432
```

---

## 17. Findings Summary

### Blocker

| # | Finding | Location |
|---|---|---|
| B-1 | PublishVersion is not a single transaction. `lock_version` (autocommit `FOR UPDATE`) releases the row lock immediately; digest is computed from a separate autocommit read; the status/digest UPDATE is a third autocommit statement. A concurrent ReplaceDraftGraph can change the graph between digest computation and the publish UPDATE, producing a PUBLISHED version whose stored digest does not match its stored graph. This directly violates the frozen contract §2/§7 and the Blocker criterion "发布后图可变或 Digest 与发布图不一致" / "发布事务存在可复现的严重竞态". | `lifecycle.rs:64-145`, `definition_repository.rs:387-421` |
| B-2 | JSON Schema validation is a no-op. `validate_json_schema` calls `jsonschema::validator_for(schema)` and discards the `Result` (`let _ = compiled; Ok(())`). Invalid schemas that fail to compile pass validation. Publish does not reject invalid `context_schema` / `submission_schema`, violating frozen architecture §10.4 and contract §3.7/§10. | `lifecycle.rs:409-429` |

### High

| # | Finding | Location |
|---|---|---|
| H-1 | `compute_weakly_reachable` treats edges as undirected, computing weak connectivity rather than directed reachability from DRAFT. Nodes reachable only via a backwards RETURN edge are incorrectly accepted. Frozen rule "所有节点从 Draft 可达" (directed) is not enforced. | `graph_helpers.rs:10-31`, `graph.rs:303-317` |
| H-2 | Terminal-node and non-terminal-node assignee rules are not enforced. `graph.rs:96-101` and the `TERMINAL` match arm (lines 450-453) are empty. A TERMINAL node with an assignee, or a NORMAL node missing an assignee ref, passes validation. Contract §3.1.7/§3.1.8 violated. | `graph.rs:96-101, 450-453` |
| H-3 | Primary transition effect is not verified to be ADVANCE. The validator checks origin and orderIndex advance but never asserts `transition_effect == ADVANCE` on the primary. Contract §3.2.3 ("primary effect 必须为 ADVANCE") not explicitly enforced. | `graph.rs:150-180` |
| H-4 | Lifecycle actor columns (`published_by_principal_id`, `deprecated_by_principal_id`, `revoked_by_principal_id`) added by Migration 0008 are never written by `publish_version` / `update_version_status`, and never read by `WorkflowDefinitionVersionRow`. The migration's stated purpose is unfulfilled. | `definition_repository.rs:402-448`, `repository_rows.rs:46-90` |
| H-5 | Read queries (`get_definition`, `get_definition_version`, `list_definition_versions`, `get_complete_version_graph`) perform no domain-membership / domain-owner authorization. Any enabled principal can read any definition across domains, including instructions, schemas, and fixed-principal references. `ValidateDraftVersion` also lacks DOMAIN_OWNER check. Cross-domain data leak. | `lifecycle.rs:294-401, 26-57` |

### Medium

| # | Finding |
|---|---|
| M-1 | `replace_draft_graph` repository only updates `context_schema` when `Some`; passing `None` leaves the prior schema in place (cannot clear). Minor semantic gap. |
| M-2 | `jsonschema` crate 0.47 is built with `reqwest`+`rustls`, enabling remote `$ref` resolution. Once B-2 is fixed, a malicious schema could trigger outbound HTTP / potential SSRF during publish. Remote resolution must be disabled. |
| M-3 | No test re-computes the digest from a published version's graph and asserts equality with the stored `definition_digest`. |
| M-4 | Write operations other than `CreateDefinition` do not call `ensure_domain_enabled`. A disabled Domain can have drafts created, graphs replaced, and versions published/deprecated/revoked. Contract §6 requires Domain enabled. |
| M-5 | Trigger-raised errors (graph_immutable, status_transition) surface as opaque `StorageError(raw message)` rather than typed domain errors (`VersionNotDraft`, `InvalidLifecycleTransition`). |
| M-6 | `definition_key_exists` pre-check in `CreateDefinition` is a check-then-act race (mitigated by the DB unique index, but redundant). |

### Low / Notes

| # | Finding |
|---|---|
| L-1 | `create_definition` length validation returns `StorageError` instead of a dedicated validation error variant. |
| L-2 | `parse_assignee_ref` returns `StorageError` for invalid enum strings. |
| L-3 | `seed_workflow_definition` test fixture creates a DRAFT node with `FIXED_PRINCIPAL` assignee, violating the contract's Draft=WORKFLOW_CREATOR rule. Tests that rely on this seed do not exercise real validation. |
| L-4 | `nodes_by_key` HashMap construction silently drops duplicate keys; uniqueness relies entirely on the DB index. |
| L-5 | Comment in `graph.rs:107` claims source/target cross-version check is not fully verified — accurate; cross-version integrity relies on DB FKs. |
| L-6 | `next_version_number` uses `MAX+1` (documented in contract as the strategy); concurrent losers get `ConcurrentModification` and must retry. Matches contract; not a defect. |

---

## 18. Test Coverage Gaps

1. No truly-concurrent Publish + ReplaceDraftGraph test (B-1 undetected).
2. No malformed-schema publish-rejection test (B-2 undetected).
3. No directed-reachability-negative test (H-1 undetected).
4. No terminal-assignee-negative test (H-2 undetected).
5. No lifecycle-actor-column assertion (H-4 undetected).
6. No cross-domain read-denial test (H-5 undetected).
7. No digest read-back consistency test (M-3).
8. `node_not_reachable_from_draft` asserts the wrong error code (claims `MISSING_PRIMARY`, not `NODE_NOT_REACHABLE`).

---

## 19. Publish Concurrency Verdict

**NOT SAFE.** The publish flow is a sequence of autocommit statements, not a
single transaction. The `FOR UPDATE` lock acquired in `lock_version` is released
before digest computation and before the status UPDATE. A concurrent
ReplaceDraftGraph can mutate the graph between digest read and publish write,
yielding a PUBLISHED version whose persisted digest does not correspond to its
persisted graph. The DB triggers do not prevent this because (a) the trigger's
parent-status SELECT is non-locking under READ COMMITTED and (b) the publish
UPDATE's exclusive lock arrives too late to serialize against an in-flight
ReplaceDraftGraph transaction's graph reads. See Blocker B-1.

---

## 20. Minimum Fix Recommendations

### To close Blocker B-1 (publish transaction)

Wrap the entire `publish_version` flow in a single transaction and pass the
transaction (or a transaction-scoped executor) through the repository. Concretely:
add a `publish_version_tx(&mut self, tx: &mut Transaction<'_, Postgres>, ...)`
variant that performs `SELECT ... FOR UPDATE`, graph read, and the status/digest
UPDATE on the *same* transaction. The service must `begin()` once, do all reads
and writes on that transaction, and `commit()` once. The same restructuring is
needed for `deprecate_version` and `revoke_version` (the lock must span the status
UPDATE).

### To close Blocker B-2 (schema validation)

Replace the body of `validate_json_schema` with:

```rust
jsonschema::validator_for(schema)
    .map_err(|e| format!("schema failed to compile: {e}"))?;
Ok(())
```

and propagate the error so that publish/validate reject un-compilable schemas.
Additionally, construct the validator via `Validator::options().without_schema()`
or equivalent to disable remote `$ref` resolution (closes M-2).

### To close H-1 (directed reachability)

Replace `compute_weakly_reachable` with a directed BFS/DFS that only follows
`source_node_id → target_node_id` from the draft node. Add a test with a node
reachable only via a RETURN edge and assert `NODE_NOT_REACHABLE`.

### To close H-2 (assignee rules)

Implement the empty match arms: reject TERMINAL nodes carrying a non-null
`fixed_principal_id` (or any assignee ref other than the default); reject NORMAL
nodes with no assignee ref type.

### To close H-3 (primary effect)

In the primary-trunk loop, assert the resolved transition's
`transition_effect == ADVANCE`; emit `PRIMARY_NOT_ADVANCE` otherwise.

### To close H-4 (lifecycle actors)

Set `published_by_principal_id` / `deprecated_by_principal_id` /
`revoked_by_principal_id` in the publish/deprecate/revoke SQL (passing the actor
principal id), and extend `WorkflowDefinitionVersionRow` to SELECT and populate
them.

### To close H-5 (query authorization)

For each read query, resolve the definition's `domain_id` and verify the actor
holds an enabled `DOMAIN_OWNER` binding (or whatever read policy the contract
specifies) before returning data. Add `ensure_domain_owner` to
`ValidateDraftVersion`.

---

## 21. Verdict

```
SVC_WORKFLOW_DEFINITION_SERVICE_AUDIT_BLOCKED
```

### Merge Recommendation

**❌ DO NOT MERGE.** Two Blockers (B-1 publish transaction / digest consistency,
B-2 schema validation no-op) and five Highs (H-1 directed reachability, H-2
assignee rules, H-3 primary effect, H-4 lifecycle actors, H-5 query authorization)
must be resolved before merge.

### Reasoning

The PR builds cleanly, passes fmt/clippy, all 125 tests pass in serial and
parallel, the migration applies on all three verification paths, the structure
guards hold, the scope is clean, and the digest algorithm itself is correct.
However:

- **B-1** is a textbook TOCTOU that breaks the frozen "publish is atomic and the
  digest matches the published graph" invariant — the central safety promise of
  an immutable-version publishing service.
- **B-2** means the schema gate required at publish is effectively absent.
- **H-1** directly contradicts the frozen directed-reachability rule and is
  explicitly called out as High in the contract.
- **H-5** is a real cross-domain data leak that the contract specifically warns
  against.

These cannot be deferred to a follow-up without compromising the integrity
guarantees that justify the service's existence.

---

*Report path: `/Users/yanfenma/workspace/project/svc-workflow/DEFINITION_SERVICE_AUDIT_REPORT.md`*
*Generated: 2026-07-13*
*Auditor: ZCode (independent review, no implementation modifications)*
