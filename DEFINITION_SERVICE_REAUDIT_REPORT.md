# Workflow Definition Version Service — Independent Re-Audit (Post-Fix)

## 1. Review Metadata

| Field | Value |
|---|---|
| Repository | `/Users/yanfenma/workspace/project/svc-workflow` |
| Branch | `feat/definition-version-service-v0` |
| PR 2 Base SHA | `d8e980869a28d85518d622e269bc20cd0ea37632` |
| Original Implementation HEAD (1st audit) | `4f5d84c653426fd3d23068df74abffa14385abf3` |
| Fix Commit (this re-audit target) | `7e5283a04fbf376081d9e03d0901e9dc0c50c236` |
| Working-tree HEAD at re-audit | `7e5283a04fbf376081d9e03d0901e9dc0c50c236` (= fix commit) |
| Frozen Architecture Tag | `svc-workflow-architecture-v0.3.1-frozen` |
| PostgreSQL | PostgreSQL 16.14 (Homebrew) on x86_64-apple-darwin |
| Test Database | `svc_workflow` @ `localhost:5432` |

### Pre-review State Confirmation

```
git status --short             → (clean)
git branch --show-current      → feat/definition-version-service-v0
git rev-parse HEAD             → 7e5283a04fbf376081d9e03d0901e9dc0c50c236
git merge-base HEAD d8e9808    → d8e980869a28d85518d622e269bc20cd0ea37632
```

Working tree clean, HEAD == fix commit, merge-base correct. Re-audit scope is the
single fix commit `4f5d84c6..7e5283a0` (no implementation modifications performed;
only read-only audit + temporary throw-away concurrency harness, since removed).

### Contracts Read

```
docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md   (frozen)
docs/contracts/IMPLEMENTATION_CONTRACT_V0_1.md
docs/contracts/POSTGRES_STORAGE_CONTRACT_V0_1.md
docs/contracts/DEFINITION_SERVICE_FIX_CONTRACT_V0_1.md  (added by the fix)
DEFINITION_SERVICE_AUDIT_REPORT.md                       (original audit, preserved)
```

Priority applied: **frozen architecture → Implementation Contract → Definition Service
fix contract → actual implementation.** No personal preference used to overturn a
frozen rule.

---

## 2. Verdict

```
SVC_WORKFLOW_DEFINITION_SERVICE_REAUDIT_PASS
```

| Question | Answer |
|---|---|
| **Functional: may this merge?** | **Yes.** Both original Blockers (B-1 publish atomicity, B-2 schema validation) and all five original Highs (H-1 directed reachability, H-2 assignee rules, H-3 primary effect, H-4 lifecycle actors, H-5 domain authorization) are closed with verification. No new functional Blocker / High was introduced by the fix. |
| **Structural: does it satisfy the project guard?** | **No.** Four files exceed the 500-physical-line limit (see §10). This is a **Medium structural issue**, not a functional Blocker, and must not be masked as a data-consistency defect — but it does mean the project merge rule "single file ≤ 500 physical lines" is not satisfied. |

The two questions are reported separately, as instructed. The functional verdict is
`PASS_WITH_NOTES` (only Medium/Low remain). The structural guard is failed.

---

## 3. B-1 — Publish Atomicity: **CLOSED**

### 3.1 Full call chain (verified against source)

```
DefinitionService::publish_version(cmd)                         src/application/definition/lifecycle.rs:78
  ├─ ensure_principal_enabled(actor)                            autocommit SELECT
  ├─ repo.lock_version(version_id)                              autocommit SELECT ... FOR UPDATE  (pre-pass snapshot only)
  ├─ repo.get_definition(...)                                   autocommit SELECT
  ├─ repo.get_complete_graph(version_id)                        autocommit SELECT nodes + transitions
  ├─ graph::validate_graph(&graph)                              pure in-memory
  ├─ self.validate_json_schemas(&graph)                         pure in-memory (jsonschema compile)
  ├─ self.validate_fixed_principals(&nodes)                     autocommit SELECT per FIXED_PRINCIPAL
  ├─ digest::compute_digest(... from step-5 data ...)           pure in-memory, returns `precomputed_digest`
  └─ repo.atomic_publish(version_id, actor, &precomputed_digest) ★ SINGLE TRANSACTION ★
       src/store/postgres/definition_repository.rs:537
       BEGIN
         1. SELECT … FOR UPDATE               (lock version row)            definition_repository.rs:546
         2. verify version_status == DRAFT                                  definition_repository.rs:556
         3. SELECT workflow_definitions (read def in tx)                    definition_repository.rs:565
         4. SELECT domains.enabled                                          definition_repository.rs:578
         5. SELECT domain_role_bindings (DOMAIN_OWNER, enabled)             definition_repository.rs:592
         6. SELECT nodes  … ORDER BY order_index   (re-read graph in tx)    definition_repository.rs:608
         7. SELECT transitions … ORDER BY transition_key (re-read in tx)    definition_repository.rs:619
         8. digest::compute_digest(from in-tx nodes/transitions)            definition_repository.rs:640
         9. IF actual_digest != precomputed_digest
              → Err(ConcurrentModification); tx drops (rollback)            definition_repository.rs:652
        10. UPDATE version SET status='PUBLISHED',
                definition_digest, published_at, published_by_principal_id  definition_repository.rs:659
       COMMIT                                                              definition_repository.rs:675
```

Every step from the version lock through the final `UPDATE` runs **inside the same
`pool.begin()` … `tx.commit()`**. The `FOR UPDATE` lock is held across digest
re-computation and the status write. The `lock_version` in the service pre-pass is
now only a fast-fail snapshot (its lock is released immediately), but
`atomic_publish` re-acquires `FOR UPDATE` inside the transaction and is the
authoritative serialization point.

### 3.2 Single-transaction conditions satisfied

| Required step | In tx? | Evidence |
|---|---|---|
| `SELECT ... FOR UPDATE` | ✅ | `definition_repository.rs:548` (`fetch_optional(&mut *tx)`) |
| Verify DRAFT inside lock | ✅ | `definition_repository.rs:556-562` |
| Read domain, verify enabled + DOMAIN_OWNER | ✅ | `:578-605` |
| Re-read complete graph in tx | ✅ | `:608-628` |
| Compute digest from in-tx graph | ✅ | `:640-650` |
| Verify digest == caller-supplied | ✅ | `:652-656` → `ConcurrentModification` |
| UPDATE status + digest + actor | ✅ | `:659-672` |
| COMMIT | ✅ | `:675` |

The graph is read inside the same transaction; the digest is computed from the
in-tx graph; the persisted digest is the in-tx result (the `UPDATE` binds
`precomputed_digest`, which was just proven equal to the in-tx digest by step 9 —
if they differed the transaction already aborted). The schema/graph validation is
performed in the service pre-pass **outside** the tx, but equivalence is strictly
proven by the in-tx full-document digest comparison: identical digest ⇒ identical
canonical document ⇒ identical graph/schema/fields covered by the digest. The
digest covers all graph + schema + identity fields enumerated in
`digest.rs::CanonicalDefinitionDocument` (definition_key, version_number, dialect,
validator_version, context_schema, all node fields incl. assignee/instructions/
primary, all transition fields incl. submission_schema/metadata). No field that
affects graph validation is omitted.

### 3.3 Inconsistency behavior (Replace commits between Publish read and Publish write)

Reproduced directly via the **stale-digest** harness (temporary test, since
removed). Calling `atomic_publish(version_id, actor, stale_digest)` where
`stale_digest` differs from the in-tx digest yields:

```
Err(DefinitionError::ConcurrentModification(
    "definition graph changed during publish; retry with fresh data"))
```

Post-condition assertions (all passed):
- `version_status == DRAFT`
- `definition_digest IS NULL`
- `published_at IS NULL`
- `published_by_principal_id IS NULL`
- graph unchanged (still 3 nodes)

The whole publish transaction rolls back. No half-publish. The Replace's committed
graph remains intact. This matches §4.3 of the audit brief.

### 3.4 ReplaceDraftGraph transaction

`definition_repository.rs:283-385`:

```
BEGIN
  SELECT version_status::TEXT … FOR UPDATE      (lock + verify DRAFT)   :293-305
  DELETE FROM workflow_transition_definitions    :309
  DELETE FROM workflow_node_definitions          :315
  INSERT nodes (loop)                            :322-343
  INSERT transitions (loop)                      :346-365
  UPDATE context_schema (only if Some)           :371-380
COMMIT                                           :382
```

Single transaction; `FOR UPDATE` inside the tx; DRAFT verified inside the lock;
delete-before-insert order respects FKs; lock is not released early. The
service-level `lock_version` pre-check was **removed** (`draft_graph.rs:24-46`
now calls `get_version`, not `lock_version`), eliminating the TOCTOU window the
original audit flagged.

### 3.5 Deprecate / Revoke

`atomic_deprecate` (`:681-768`) and `atomic_revoke` (`:770-860`) each:

```
BEGIN
  SELECT … FOR UPDATE                  (lock)
  verify source status (PUBLISHED / PUBLISHED|DEPRECATED)
  SELECT domain (enabled) + DOMAIN_OWNER inside tx
  UPDATE version_status, deprecated_at/revoked_at,
         deprecated_by/revoked_by_principal_id
COMMIT
```

Each transition writes only its own actor column; the prior stage's actor column
is never touched (no overwrite). Verified by `test_three_stage_actors_all_preserved`.

---

## 4. B-1 Concurrency Reproduction (§5 of the brief — actually executed)

The committed suite only contains `test_manual_lock_blocks_replace_draft_graph`
(scenario 5.1). Scenarios 5.2–5.5 were **not** covered by committed tests. Per
the brief I executed them this round:

### 4.1 Scenario 5.1 — manual lock blocks Replace (committed test)

`tests/16_definition_service_audit_fix_tests.rs:1410`. Holds `FOR UPDATE` on the
version row from connection A; asserts `replace_draft_graph` from connection B
times out within 500 ms, then succeeds after A commits. **PASS.**

### 4.2 Scenario 5.2 — manual lock blocks Publish (SQL-level, this round)

Held `SELECT … FOR UPDATE` on the version row in `psql` connection A for 2 s.
From connection B issued `BEGIN; SELECT … FOR UPDATE; COMMIT;` (the exact first
statement of `atomic_publish`). Connection B blocked for **~1433 ms** and only
acquired the lock after A committed. This proves `atomic_publish`'s `FOR UPDATE`
waits for the row lock — it does **not** bypass it via a separate autocommit
query. **PASS.**

### 4.3 Scenario 5.3 — Replace commits first, Publish runs after (this round)

Temporary Rust harness: replace to an alternative 4-node graph and commit, then
call `service.publish_version()`. The service pre-pass re-reads the **current
(alt)** graph, computes the alt digest, then `atomic_publish` re-reads inside the
tx and confirms the digest matches. Result: **publish SUCCEEDED on the alt graph,
`stored_digest == digest(stored_graph)`** (asserted by re-reading nodes/transitions
from DB and recomputing). The publish either re-validates the new graph or fails
with `ConcurrentModification`; both outcomes preserve the invariant. **PASS.**

### 4.4 Scenario 5.4 — Publish commits first, Replace runs after (this round)

Publish first (version → PUBLISHED), then call `replace_draft_graph`. Result:
`Err(VersionNotDraft)` (caught by the repository's in-tx DRAFT check). Post-state:
`version_status == PUBLISHED`, graph is the original 3-node graph,
`stored_digest == digest(stored_graph)`. **PASS.**

### 4.5 Scenario 5.5 — true concurrent Publish + Replace (this round, 5 runs)

Two independent pools (two independent server connections), `tokio::join!` on
`publish_version` and `replace_draft_graph`. Ran 5 times. In every run Publish
won the row lock, Replace received `VersionNotDraft`, and the final state was
`PUBLISHED` with `stored_digest == digest(stored_graph)`, both `published_at` and
`published_by_principal_id` set. The invariant held in all 5 runs. **PASS.**

### 4.6 Direct digest-mismatch guard (this round, the heart of B-1)

Called `repo.atomic_publish(version_id, actor, all_zero_stale_digest)` directly.
Result: `Err(ConcurrentModification)`, and the transaction rolled back atomically
— `version_status == DRAFT`, `definition_digest IS NULL`, `published_at IS NULL`,
`published_by_principal_id IS NULL`, graph unchanged (3 nodes). The inverse call
with the **correct** digest succeeded, writing status + digest + actor + time
atomically. **This is the definitive B-1 closure.**

### 4.7 Verdict on B-1

All scenarios 5.1–5.5 plus the direct stale-digest reproduction confirm the
publish path is now a single transaction, the row lock spans digest computation
and the status write, the in-tx digest comparison detects any concurrent graph
mutation, and any mismatch rolls back atomically without partial writes.

> **B-1: CLOSED.** The original Blocker (publish split across autocommit
> statements; digest could mismatch the persisted graph) is resolved. No
> reproducible path to digest/graph mismatch remains.

**Test gap (Medium, not Blocker):** scenarios 5.2–5.5 and the direct
stale-digest reproduction are not present in the committed suite. They were
executed this round via a throw-away harness. Recommend adding at least the
stale-digest guard and a publish-vs-replace concurrent test to the permanent
suite so the invariant is protected against regressions.

---

## 5. B-2 — JSON Schema Validation: **CLOSED**

`lifecycle.rs:427-438`:

```rust
fn validate_json_schema(schema: &serde_json::Value) -> Result<(), String> {
    if !schema.is_object() {
        return Err("schema must be a JSON object".to_string());
    }
    check_external_refs(schema)?;                                   // pre-scan
    jsonschema::validator_for(schema)
        .map_err(|e| format!("schema failed to compile: {}", e))?;  // Result PROPAGATED
    Ok(())
}
```

### 5.1 Compilation error propagated

The previous `let _ = compiled; Ok(())` no-op is gone. `validator_for(schema)`
returns `Result<Validator, ValidationError>`; the `?` via `map_err` propagates any
compilation failure. `validate_json_schemas` (lifecycle.rs:246-278) pushes
`INVALID_CONTEXT_SCHEMA` / `INVALID_SUBMISSION_SCHEMA` errors for failures, and
`publish_version`/`validate_draft_version` reject when `validation_result.valid`
is false.

### 5.2 Coverage

- `WorkflowDefinitionVersion.context_schema` — validated (`:253-260`)
- Every `NodeDefinition.submission_schema` is on transitions; each
  `TransitionDefinition.submission_schema` — validated (`:263-275`)

Optional-null semantics: `if let Some(schema)` skips absent schemas (Draft may
store nothing); present schemas are compiled. Matches contract.

### 5.3 External references

`check_external_refs` (lifecycle.rs:448-478) recursively walks Object + Array
nodes and inspects `$ref`, `$dynamicRef`, `$recursiveRef`. Any value not starting
with `#` is rejected. Verified by unit tests:

| Case | Test | Result |
|---|---|---|
| `https://` ref | `validate_json_schema_rejects_https_ref` | ✅ rejected |
| `file://` ref | `validate_json_schema_rejects_file_ref` | ✅ rejected |
| Relative `../x.json` ref | `validate_json_schema_rejects_relative_ref` | ✅ rejected |
| Nested object ref | `validate_json_schema_rejects_nested_external_ref` | ✅ rejected |
| `$dynamicRef` external | `validate_json_schema_rejects_dynamic_ref_external` | ✅ rejected |
| Local `#/$defs/...` | `validate_json_schema_allows_local_fragment` | ✅ allowed |
| Bare `#` | `validate_json_schema_allows_bare_hash` | ✅ allowed |
| Invalid keyword structure | `validate_json_schema_rejects_invalid_keyword_structure` | ✅ rejected |

### 5.4 Network / file resolution note (defense in depth)

The `jsonschema` 0.47 crate is pulled with **default features** (`resolve-http`,
`resolve-file`, `tls-aws-lc-rs`) — `reqwest` + `hyper` + `rustls` are in the dep
tree (verified via `cargo tree -p jsonschema`). The resolver would, if reached,
attempt network/file access. **However**, `check_external_refs` runs **before**
`validator_for()` and rejects every non-fragment reference, so the resolver is
never invoked with an external URL. Combined with the schema being a pure local
JSON tree (no network at compile time), this is safe. Recommend as Low/Medium
follow-up: disable `resolve-http`/`resolve-file` features in `Cargo.toml`
(`jsonschema = { version = "0.47", default-features = false }`) so the capability
is removed at the dependency level rather than relying on the pre-scan alone.

### 5.5 Publish-failure atomicity

`test_invalid_schema_version_stays_draft` (test file :390) sets an invalid
`context_schema` (`{"type":123}`), attempts publish, then asserts the version is
still `DRAFT`, `definition_digest IS NULL`, `published_by_principal_id IS NULL`.
Schema validation runs in the service pre-pass (before `atomic_publish`), so a
failure returns `GraphValidationFailed` without ever beginning the publish
transaction — no partial writes are possible. **PASS.**

> **B-2: CLOSED.** Schema validation is no longer a no-op; compilation errors
> are propagated as typed errors; external references are pre-scanned and
> rejected; invalid schemas leave the version DRAFT with no partial publish
> fields.

---

## 6. H-1 — Directed Reachability: **CLOSED**

`graph_helpers.rs::compute_directed_reachable` (renamed from
`compute_weakly_reachable`) builds an adjacency list `source → target` only
(`:30-35`) and runs a forward BFS from the DRAFT node (`:37-53`). It no longer
traverses edges backwards.

Unit tests in `graph_helpers.rs`:
- `directed_chain_all_reachable` — 3-node chain all reachable ✅
- `isolated_node_not_reachable` — disconnected node flagged ✅
- `node_only_connected_via_reverse_edge_not_reachable` — node with only an
  outgoing-to-draft edge flagged ✅ (the original H-1 counterexample)
- `return_edge_does_not_help_unreachable_nodes` — node with a RETURN edge into
  the trunk, but no incoming directed edge, flagged ✅

Integration: `test_directed_unreachable_node_rejected` and
`test_node_only_reachable_via_backwards_edge_rejected` assert `NODE_NOT_REACHABLE`
at publish. Error text contains the unreachable node's key. Function name, test
names, and error text no longer reference "weakly connected".

> **H-1: CLOSED.** RETURN back-edges no longer make unreachable nodes pass.
> The frozen directed-reachability rule is enforced and tested.

---

## 7. H-2 — Assignee Rules: **CLOSED**

`graph.rs:98-190` implements the empty match arms. Per node type:

| Type | Rule enforced | Error code |
|---|---|---|
| TERMINAL | no `fixed_principal_id`; ref_type ≠ `FIXED_PRINCIPAL` | `TERMINAL_HAS_ASSIGNEE` |
| DRAFT | ref_type == `WORKFLOW_CREATOR`; no `fixed_principal_id` | `DRAFT_NOT_WORKFLOW_CREATOR`, `UNEXPECTED_FIXED_PRINCIPAL` |
| NORMAL / WORKFLOW_CREATOR | no `fixed_principal_id` | `UNEXPECTED_FIXED_PRINCIPAL` |
| NORMAL / DOMAIN_OWNER | no `fixed_principal_id` | `UNEXPECTED_FIXED_PRINCIPAL` |
| NORMAL / FIXED_PRINCIPAL | `fixed_principal_id` required | `FIXED_PRINCIPAL_MISSING_ID` |

### 7.1 FIXED_PRINCIPAL existence + enabled

`lifecycle.rs::validate_fixed_principals` (service layer, since the pure graph
validator cannot reach the DB) iterates nodes with `FIXED_PRINCIPAL`, and for each
calls `check_principal_exists` + `check_principal_enabled`. A missing or disabled
fixed principal returns `FixedPrincipalInvalid`. No silent pass on DB error: both
checks propagate `DefinitionError` via `?`. Called from `publish_version`
(`:125`) — and `ValidateDraftVersion` runs the same `validate_json_schemas` +
graph path so it exercises the graph-layer rules too.

### 7.2 Terminal placeholder rejection

Test `test_terminal_node_with_fixed_principal_rejected` flips a terminal's
`assignee_ref_type` to `FIXED_PRINCIPAL` with a principal id and asserts
`TERMINAL_HAS_ASSIGNEE`. No invalid placeholder assignee can slip through.

### 7.3 Coverage tests

| Scenario | Test | Result |
|---|---|---|
| Terminal with FIXED_PRINCIPAL | `test_terminal_node_with_fixed_principal_rejected` | ✅ `TERMINAL_HAS_ASSIGNEE` |
| Terminal without assignee (positive) | `test_terminal_without_assignee_allowed` | ✅ publish succeeds |
| DRAFT/WORKFLOW_CREATOR with fixed id | `test_workflow_creator_with_fixed_id_rejected` | ✅ `UNEXPECTED_FIXED_PRINCIPAL` |
| FIXED_PRINCIPAL missing id | `test_fixed_principal_missing_id_rejected` | ✅ `FIXED_PRINCIPAL_MISSING_ID` |
| FIXED_PRINCIPAL disabled | `test_fixed_principal_disabled_rejected` | ✅ `FixedPrincipalInvalid` |

> **H-2: CLOSED.** Terminal and non-terminal assignee rules are enforced and
> tested; FIXED_PRINCIPAL existence + enabled is verified at publish.

---

## 8. H-3 — Primary Transition Effect: **CLOSED**

`graph.rs:257-266`: inside the primary-trunk loop, the resolved transition's
effect is checked:

```rust
if trans.transition_effect != TransitionEffect::Advance {
    errors.push(GraphValidationError::new(
        "PRIMARY_NOT_ADVANCE", ...));
}
```

This catches a primary pointing at a RETURN or TERMINATE transition (or any
non-ADVANCE effect). Tests:
- `test_primary_effect_not_advance_rejected` — flips the primary transition's
  effect to RETURN, asserts `PRIMARY_NOT_ADVANCE` ✅
- `test_primary_advance_allowed` — valid ADVANCE primary publishes ✅

The RETURN/TERMINATE-specific `RETURN_IS_PRIMARY` / `TERMINATE_IS_PRIMARY` checks
remain as before; `PRIMARY_NOT_ADVANCE` is the new explicit guard. Terminal nodes
are still forbidden from having a primary (`TERMINAL_HAS_PRIMARY`).

> **H-3: CLOSED.** Primary transition effect must be ADVANCE; RETURN/TERMINATE
> primaries are rejected.

---

## 9. H-4 — Lifecycle Actor Fields: **CLOSED**

Migration 0008 adds the three actor columns; the fix wires them end-to-end.

### 9.1 Write paths

| Op | Column(s) set | Location |
|---|---|---|
| Publish | `published_by_principal_id`, `published_at` | `atomic_publish` `:659-672` |
| Deprecate | `deprecated_by_principal_id`, `deprecated_at` | `atomic_deprecate` `:751-763` |
| Revoke | `revoked_by_principal_id`, `revoked_at` | `atomic_revoke` `:843-855` |

Each lifecycle UPDATE sets **only** its own actor column. Subsequent transitions
do not overwrite prior actors (the column is never named in another stage's
UPDATE). Verified by `test_three_stage_actors_all_preserved` — after
publish → deprecate → revoke, all three of `published_by_principal_id`,
`deprecated_by_principal_id`, `revoked_by_principal_id` equal the actor.

### 9.2 Read paths

`WorkflowDefinitionVersionRow` (`repository_rows.rs:46-94`) now SELECTs all three
actor columns and maps them via `.map(PrincipalId::from_uuid)`. All three queries
that return `WorkflowDefinitionVersion` — `get_version` (`:172-182`),
`lock_version` (`:417-431`), `list_versions` (`:263-279`) — use the same column
list. NULL values map to `None` cleanly (the `Option<Uuid>` row field accepts
NULL). `get_complete_version_graph` returns the version via `get_version`, so it
also sees the actor fields.

### 9.3 NULL preservation

`test_unpublished_version_actor_fields_null` asserts all three actor columns are
NULL for a DRAFT version that has never been published.

> **H-4: CLOSED.** All three lifecycle actor columns are written by their
> respective operation, never overwritten by later stages, and read back by every
> query that returns a version.

---

## 10. H-5 — Domain Authorization: **CLOSED**

### 10.1 Reads

All four read entry points now call `ensure_domain_owner(actor, def.domain_id)`
**before** returning content:

| Query | Auth check location |
|---|---|
| `get_definition` | `lifecycle.rs:298` (after fetching def, before returning) |
| `get_definition_version` | `:326` |
| `list_definition_versions` | `:363` |
| `get_complete_version_graph` | `:399` |

`ensure_domain_owner` (service.rs:166-179) resolves
`domain_role_bindings` for `role_key = "DOMAIN_OWNER"` AND `enabled = true`
(repository `check_domain_role`, `:83-100`). A non-owner (or disabled binding)
gets `PermissionDenied`. The check runs **before** any graph/nodes are read in
the version/graph queries, so instructions, schemas, and fixed-principal refs are
not leaked to unauthorized callers.

### 10.2 Writes

| Op | DOMAIN_OWNER check |
|---|---|
| `CreateDefinition` | service.rs:49 |
| `CreateDraftVersion` | service.rs:108 |
| `ReplaceDraftGraph` | draft_graph.rs:44 |
| `ValidateDraftVersion` | lifecycle.rs:44 (**new** — closes the audit gap) |
| `PublishVersion` | inside `atomic_publish` `:592-605` |
| `DeprecateVersion` | inside `atomic_deprecate` `:735-748` |
| `RevokeVersion` | inside `atomic_revoke` `:827-840` |

The authorization basis is exclusively `DomainRoleBinding` with
`role_key = DOMAIN_OWNER` + `enabled = true`. No fallback to `isDomainAdmin` or
any `domains.owner_*` column. Cross-domain owners do not gain access to other
domains.

### 10.3 Coverage tests

| Scenario | Test | Result |
|---|---|---|
| Cross-domain read denied | `test_cross_domain_read_denied` | ✅ `PermissionDenied` |
| Cross-domain list denied | `test_cross_domain_list_versions_denied` | ✅ err |
| Domain owner can read | `test_domain_owner_can_read` | ✅ ok |
| Validate requires owner | `test_validate_draft_version_requires_owner` | ✅ `PermissionDenied` |
| Disabled principal denied | `test_disabled_principal_cannot_read` | ✅ `PrincipalDisabled` |

### 10.4 Domain enabled gate (also closes M-4)

All write paths now call `ensure_domain_enabled(domain_id)`:
`CreateDefinition` (service.rs:48), `CreateDraftVersion` (:105),
`ReplaceDraftGraph` (draft_graph.rs:43), `ValidateDraftVersion` (lifecycle.rs:43),
and the in-tx checks in `atomic_publish`/`atomic_deprecate`/`atomic_revoke`
(`:578-589`, `:722-733`, `:814-825`). `test_disabled_domain_blocks_write` flips
the domain to disabled and asserts publish fails with `DomainDisabled`.

> **H-5: CLOSED.** All reads require DOMAIN_OWNER of the target domain; all
> writes require DOMAIN_OWNER + domain enabled. No cross-domain leak path found.

---

## 11. Directly-Associated Medium Findings

### 11.1 M-1 — context_schema patch semantics: **IMPLEMENTED, test gap**

The repository (`definition_repository.rs:371-380`) uses
`if let Some(schema) = context_schema { UPDATE … SET context_schema = $1 }`.
Combined with the service field type `Option<serde_json::Value>` and the call
`cmd.context_schema.as_ref()` (`draft_graph.rs:168`), three states are
distinguished:

| Command value | `Option<&Value>` at repo | SQL effect |
|---|---|---|
| `None` | `None` | skip UPDATE → keep existing |
| `Some(Value::Null)` | `Some(&Null)` | UPDATE → column = NULL (clear) |
| `Some(obj)` | `Some(&obj)` | UPDATE → column = obj (replace) |

`serde_json::Value::Null` binds to SQL NULL via sqlx, so the clear path works.
The implementation is correct. **Gap:** there is no committed test that exercises
the explicit-null (clear) path — all tests pass `Some(object)`. Recommend adding
a clear-path test. This is a Medium test gap, not a correctness defect.

### 11.2 M-2 — remote ref resolution: **MITIGATED (pre-scan), feature-disable recommended**

See §5.4. `check_external_refs` prevents any external reference from reaching
`validator_for`. The `resolve-http`/`resolve-file` features remain enabled at the
crate level. Recommend disabling them in `Cargo.toml` as defense in depth.
**Low/Medium.**

### 11.3 M-3 — digest read-back: **CLOSED**

`test_digest_readback_consistency` (:1289) publishes a version, then **re-reads**
nodes + transitions + definition + version from PostgreSQL via
`get_complete_graph` / `get_definition` / `get_version`, rebuilds the
`node_key_map`/`transition_key_map`, recomputes the digest with the same
`digest::compute_digest`, and asserts `stored_digest == recomputed_digest` (and
length == 64). This is genuine read-back, not reuse of the publish return value.
**PASS.**

### 11.4 M-4 — domain enabled gate: **CLOSED** (see §10.4)

All write entry points now gate on `ensure_domain_enabled`.

### 11.5 M-5 — typed database errors: **CLOSED**

`map_db_error` (`definition_repository.rs:25-49`) maps:

| DB condition | Domain error |
|---|---|
| `23505` + message contains `definition_key` | `DefinitionKeyConflict` |
| `23505` + message contains `version_number` | `ConcurrentModification` |
| message contains `graph_immutable:` | `VersionNotDraft` |
| message contains `status_transition:` | `InvalidLifecycleTransition` |
| anything else | `StorageError(raw)` |

Mapping keys off the typed SQLSTATE (`23505`) and the trigger-defined message
prefixes (which the migration controls), not arbitrary English fragments. Unknown
errors fall through to `StorageError(raw)` — never silently swallowed. Applied
consistently across `create_definition`, `create_draft_version`,
`replace_draft_graph`, and the atomic lifecycle methods. **PASS.**

### 11.6 M-6 — CreateDefinition race: **CLOSED**

The `definition_key_exists` pre-check was **removed**
(`create_definition` in service.rs:41-83 no longer calls it; the trait method
remains but is unused by the create path). Uniqueness is enforced solely by the
DB unique index `(domain_id, definition_key)`; `23505` maps to
`DefinitionKeyConflict` (`definition_repository.rs:127-136`). Test
`test_concurrent_create_definition_unique` (:1364) issues two sequential creates
with the same key and asserts the first succeeds, the second returns
`DefinitionKeyConflict` — no `23505` or raw SQL leaks. **PASS.**

---

## 12. Migration Three-Way Verification (this round, actually executed)

Per the brief, the prior-round result was not reused. Three databases were
created this round, all dropped after verification.

| # | Database | Path | Result |
|---|---|---|---|
| 1 | `reaudit_fresh1` | empty → 0001–0008 | ✅ all 8 migrations applied, exit 0 each |
| 2 | `reaudit_upgrade` | 0001–0007 → 0008 | ✅ 0001-0007 exit 0; **actor columns absent (0)** before 0008; 0008 exit 0; **actor columns present (3)** after |
| 3 | `reaudit_fresh2` | drop+recreate → 0001–0008 | ✅ all 8 exit 0; actor columns present (3); trigger function present |

Commands used (representative):

```bash
createdb reaudit_fresh1
for i in 0001 0002 0003 0004 0005 0006 0007 0008; do
  psql -d reaudit_fresh1 -f migrations/${i}_*.sql
done
psql -d reaudit_fresh1 -tAc "SELECT count(*) FROM information_schema.columns
  WHERE table_name='workflow_definition_versions'
  AND column_name IN ('published_by_principal_id',
                      'deprecated_by_principal_id',
                      'revoked_by_principal_id);"   # → 3
```

Idempotency: re-applying 0008 to `reaudit_fresh2` succeeded (exit 0) because the
migration uses `CREATE OR REPLACE FUNCTION` and `ADD COLUMN IF NOT EXISTS`.

Post-migration state on `reaudit_fresh1`:
- enum `definition_version_status`: DRAFT, PUBLISHED, DEPRECATED, REVOKED (4)
- enum `node_type`: DRAFT, NORMAL, TERMINAL (3)
- enum `transition_effect`: ADVANCE, RETURN, TERMINATE (3)
- enum `assignee_ref_type`: WORKFLOW_CREATOR, DOMAIN_OWNER, FIXED_PRINCIPAL (3)
- graph immutability triggers on `workflow_node_definitions` /
  `workflow_transition_definitions`: 6 triggers
- actor columns on `workflow_definition_versions`: 3
- `fn_check_definition_graph_immutable`: present

> **Note on the prior audit's "18 enum values":** that count appears to have been
> inaccurate. The actual schema defines 4 + 3 + 3 + 3 = **13** enum values across
> the four enums, all matching the contract exactly. The migration is correct.

---

## 13. Test Counts (reconciled)

The fix commit's narrative inconsistency ("31 new tests" vs "125 + 46 = 171") is
reconciled below.

```
cargo test --lib -- --list                       →  54 lib (unit) tests
cargo test --tests -- --list  (per file):
   01_migration_tests.rs                            2
   02_domain_owner_tests.rs                         2
   03_context_revision_constraints.rs               4
   04_node_visit_constraints.rs                     3
   05_submission_constraints.rs                     3
   06_event_constraints.rs                          4
   07_command_constraints.rs                        9
   08_instance_constraints.rs                       8
   09_deferred_fk_tests.rs                          2
   10_definition_version_tests.rs                   5
   11_size_limit_tests.rs                           8
   12_graph_immutability_tests.rs                  16
   13_definition_service_tests.rs                   9
   14_definition_lifecycle_tests.rs                 6
   15_definition_graph_parent_move_tests.rs         5
   16_definition_service_audit_fix_tests.rs        31   ← matches "31" claim
                                                  ---
   integration total                              117

TOTAL                                            171
```

The "31" refers to the new file 16's integration tests. The "46 added" delta
vs the original 125 baseline = 31 (file 16) + 15 new lib unit tests
(lib went 39 → 54). The remaining 15 new unit tests live in
`lifecycle::tests` (13 schema-validation cases) and
`graph_helpers::tests` (5 directed-reachability cases) — note some lib tests
were also refactored/renamed, so the +15 is net. **The count is internally
consistent; both "31" and "171" are correct in their respective scopes.**

### 13.1 Test quality spot-checks

- Concurrency test `test_manual_lock_blocks_replace_draft_graph` uses a real
  second connection (the service's own pool), holds `FOR UPDATE` in a separate
  `pool.begin()` tx, and uses `tokio::time::timeout(500ms)` — a real timeout, not
  "stuck means locked". Reliable.
- Error assertions match typed domain errors (`INVALID_CONTEXT_SCHEMA`,
  `NODE_NOT_REACHABLE`, `TERMINAL_HAS_ASSIGNEE`, `PRIMARY_NOT_ADVANCE`,
  `PermissionDenied`, `DefinitionKeyConflict`, `ConcurrentModification`,
  `DomainDisabled`, `PrincipalDisabled`) — not generic failures.
- Tests use unique keys per run (`format!("...-{}", &Uuid::new_v4().to_string()[..8])`),
  so parallel runs don't collide on fixed keys.
- File 16 ran cleanly under both `--test-threads=1` and default parallelism across
  two consecutive `cargo test` invocations.

---

## 14. Commands Actually Executed This Round

```
cargo fmt --check                                      → PASS (exit 0)
cargo build                                            → PASS (Finished dev profile)
cargo clippy --all-targets --all-features -D warnings  → PASS (exit 0)
cargo test -- --test-threads=1                         → 171 passed, 0 failed
cargo test  (parallel, run 1)                          → 171 passed, 0 failed
cargo test  (parallel, run 2)                          → 171 passed, 0 failed
git diff --check                                       → PASS (exit 0)
```

PostgreSQL: `PostgreSQL 16.14 (Homebrew) on x86_64-apple-darwin`.
Test DB: `svc_workflow @ localhost:5432`.

Migration verification: three databases (`reaudit_fresh1`, `reaudit_upgrade`,
`reaudit_fresh2`) created, verified, and dropped this round (see §12).

B-1 concurrency reproduction: temporary harness files
`tests/99_reaudit_concurrency_tests.rs` (scenarios 5.3/5.4/5.5) and
`tests/99b_reaudit_stale_digest.rs` (stale + correct digest) were created,
compiled, run green, and then **deleted** before producing this report. The
working tree is clean. The `svc_workflow` DB's scratch seed data from the SQL-level
§5.2 reproduction was also cleaned up.

---

## 15. Structure Guard (physical lines)

Project guard (per audit brief §15): single file ≤ 500 physical lines,
single directory ≤ 20 direct children, directory depth ≤ 4, functional cohesion.
**Physical lines (including comments and blank lines), not code-only lines.**

```bash
find src tests -type f -name '*.rs' -print0 | xargs -0 wc -l | sort -nr
```

Top results:

```
   1464 tests/16_definition_service_audit_fix_tests.rs   ❌ 2.93× limit
    861 src/store/postgres/definition_repository.rs      ❌ 1.72× limit
    612 src/application/definition/lifecycle.rs          ❌ 1.22× limit
    527 src/domain/definition/graph.rs                  ❌ 1.05× limit
    446 src/domain/definition/graph_tests.rs             ✅
    435 tests/12_graph_immutability_tests.rs             ✅
    ... (all others under 500)
```

**Four files exceed the 500-physical-line limit.** This is a real structural
guard failure. It must not be masked as a data-consistency Blocker, but it must
also not be claimed compliant.

Directory depth and direct-children guards pass:
- Max depth: `src/store/postgres/*.rs` = 3 (within ≤ 4)
- Max direct children: `tests/` = 17 (within ≤ 20); all `src/*` directories ≤ 8

> **Note on the original audit's structure claim:** the first-round audit reported
> "Largest: definition_repository.rs = 493 lines" and "all structure guards
> pass." That was inaccurate even at the time (the file is 861 lines post-fix and
> was already large pre-fix), and the fix commit grew it further. The
> re-audit cannot ratify that claim.

---

## 16. New Risks Introduced by the Fix (§16 of the brief)

| # | Risk | Finding |
|---|---|---|
| 1 | `atomic_publish` holds the tx open for non-DB work | **No.** Inside the tx: SELECTs, in-process digest, UPDATE, COMMIT. Schema compilation happens in the service pre-pass (outside the tx). No external I/O in the lock. |
| 2 | Network access inside the lock | **No.** Schema validation is pre-tx; `atomic_publish` does only SQL + in-process digest. The `jsonschema` crate retains `resolve-http`/`resolve-file` features, but `check_external_refs` gates the validator, so the resolver is never reached during publish. |
| 3 | Lock-ordering inconsistency (Version/Domain/Principal/Definition) | **No deadlock risk.** All atomic operations lock **only** the `workflow_definition_versions` row (`FOR UPDATE`). Domain/Principal/Definition are plain non-locking SELECTs inside the same tx. `replace_draft_graph` also locks only the version row. No operation locks Domain or Principal rows, so there is no lock-ordering cycle. |
| 4 | Publish/Replace/Deprecate/Revoke deadlock | **No.** All four take the same single row lock on `workflow_definition_versions`. PostgreSQL serializes them; no circular wait. |
| 5 | `map_db_error` swallows unknown errors | **No.** Falls through to `StorageError(raw)`; nothing is silently discarded. |
| 6 | Actor-field NULL handling | **OK.** `Option<Uuid>` row fields accept NULL; `.map(PrincipalId::from_uuid)` yields `None`. Verified by `test_unpublished_version_actor_fields_null`. |
| 7 | Directed-reachability change affects "trunk reaches terminal" | **No.** The trunk-reaches-terminal algorithm (`graph.rs:365-393`) is independent of `compute_directed_reachable`; it walks `primary_targets` only. Unaffected. |
| 8 | Deep JSON recursion in `check_external_refs` | **Theoretical.** Recursion depth scales with JSON nesting. A pathologically deep schema could stress the stack. Not practically exploitable through normal publish input and not a DoS vector at realistic schema sizes. **Note/Low.** |
| 9 | New code implements Command Service / HTTP | **No.** Scope is still the Definition Service only. No HTTP, no CommandReceipt, no Instance code added. |
| 10 | Original `DEFINITION_SERVICE_AUDIT_REPORT.md` rewritten | **No.** Preserved verbatim; Verdict still reads `SVC_WORKFLOW_DEFINITION_SERVICE_AUDIT_BLOCKED` / `DO NOT MERGE`. The fix contract was added as a separate document. |

---

## 17. Findings Summary (this re-audit)

### Blocker
**None.** B-1 and B-2 are closed (see §3, §5). No new Blocker introduced.

### High
**None.** H-1–H-5 are closed (see §6–§10). No new High introduced.

### Medium

| # | Finding | Status |
|---|---|---|
| S-1 | **Structure guard failed.** Four files exceed the 500-physical-line limit: `tests/16_definition_service_audit_fix_tests.rs` (1464), `src/store/postgres/definition_repository.rs` (861), `src/application/definition/lifecycle.rs` (612), `src/domain/definition/graph.rs` (527). | Open — blocks the structural merge rule, not the functional merge. |
| M-1 | `context_schema` explicit-null (clear) path implemented but not covered by a committed test. | Open — test gap only. |
| M-2 | `jsonschema` default features `resolve-http`/`resolve-file` remain enabled; safety relies on the `check_external_refs` pre-scan. | Open — defense-in-depth recommended (`default-features = false`). |
| M-7 | B-1 concurrency coverage in the **committed** suite is limited to scenario 5.1. Scenarios 5.2–5.5 and the direct stale-digest guard were verified this round only via a throw-away harness. | Open — recommend permanent regression tests. |

### Low

| # | Finding |
|---|---|
| L-1 | `check_external_refs` recurses on arbitrarily nested JSON; a pathologically deep schema could stress the stack (not a practical DoS). |
| L-2 | Original audit's "18 enum values" and "definition_repository.rs = 493 lines / structure guards pass" claims were inaccurate; corrected in this report (13 enum values; 4 files over 500 lines). |
| L-3 | `definition_key_exists` trait method is now unused by the create path; could be removed for clarity. |

---

## 18. Minimum Follow-up Recommendations

1. **Structural (S-1, required for the project merge rule):** split the four
   over-limit files by responsibility:
   - `definition_repository.rs` → separate `atomic_publish`/`atomic_deprecate`/
     `atomic_revoke` into a `lifecycle_repository.rs` (or per-operation module);
     move `map_db_error` to a small `error_mapping.rs`.
   - `16_…_audit_fix_tests.rs` → split by finding group (B-1/B-2, H-1/H-2/H-3,
     H-4, H-5, M-x) into separate test files.
   - `lifecycle.rs` → extract `validate_json_schema` + `check_external_refs` +
     their unit tests into a `schema_validation.rs`.
   - `graph.rs` → extract the primary-trunk and reachability sections into
     `graph_validation/primary.rs` and `graph_validation/reachability.rs`.
2. **Tests (M-7):** add a permanent `atomic_publish` stale-digest test and a
   publish-vs-replace concurrent test (the throw-away harnesses used this round
   are suitable templates).
3. **Tests (M-1):** add a `context_schema` clear-path test (explicit `null` →
   column becomes NULL).
4. **Dependency hardening (M-2):** set
   `jsonschema = { version = "0.47", default-features = false }` (re-add only
   what is actually needed) so network/file resolution is removed at the
   dependency level.

None of the above block functional merge.

---

## 19. Final Return

```
Repository          : /Users/yanfenma/workspace/project/svc-workflow
Branch              : feat/definition-version-service-v0
PR 2 Base SHA       : d8e980869a28d85518d622e269bc20cd0ea37632
Original impl SHA   : 4f5d84c653426fd3d23068df74abffa14385abf3
Fix SHA             : 7e5283a04fbf376081d9e03d0901e9dc0c50c236
Verdict             : SVC_WORKFLOW_DEFINITION_SERVICE_REAUDIT_PASS

B-1 closed          : YES (single-tx atomic_publish; in-tx digest guard;
                          stale-digest → ConcurrentModification + full rollback;
                          scenarios 5.1–5.5 reproduced this round)
B-2 closed          : YES (validator_for Result propagated; external refs
                          pre-scanned; invalid schema → DRAFT preserved)
H-1 closed          : YES (directed BFS, source→target only; reverse-edge-only
                          nodes rejected)
H-2 closed          : YES (terminal/DRAFT/NORMAL assignee rules enforced;
                          FIXED_PRINCIPAL existence+enabled checked)
H-3 closed          : YES (PRIMARY_NOT_ADVANCE on non-ADVANCE primary)
H-4 closed          : YES (publish/deprecate/revoke each write own actor;
                          prior actors never overwritten; all 3 read paths)
H-5 closed          : YES (all reads + writes require DOMAIN_OWNER of target
                          domain; disabled domain blocked on all writes)
M-1                 : Implemented; explicit-null test gap (Medium)
M-2                 : Pre-scan mitigated; feature-disable recommended (Low/Med)
M-3                 : Closed (true DB read-back digest consistency test)
M-4                 : Closed (ensure_domain_enabled on all writes)
M-5                 : Closed (map_db_error; no silent swallow)
M-6                 : Closed (pre-check removed; DB unique constraint authoritative)

New Blocker/High    : NONE
Publish/Replace concurrency : Reproduced 5.1–5.5; invariant held in every case
Digest read-back    : stored_digest == digest(stored_graph) asserted
Migration (3 paths) : fresh1 / upgrade(0001-0007→0008) / fresh2 all green this round
Test counts         : 54 lib + 117 integration = 171 total
                        file 16 = 31 integration tests (matches "31" claim)
Serial test run     : 171 passed, 0 failed
Parallel run 1      : 171 passed, 0 failed
Parallel run 2      : 171 passed, 0 failed
Max physical lines  : 1464 (tests/16_definition_service_audit_fix_tests.rs)
Files over 500      : 4  (see §15)
PostgreSQL          : 16.14 (Homebrew), x86_64-apple-darwin
Report path         : /Users/yanfenma/workspace/project/svc-workflow/DEFINITION_SERVICE_REAUDIT_REPORT.md

git status --short  : ?? DEFINITION_SERVICE_REAUDIT_REPORT.md
                        (report only; no implementation files touched)

Functional merge    : ALLOWED  (no Blocker / High remains)
Structure guard     : FAILED   (4 files > 500 physical lines; Medium S-1)

SVC_WORKFLOW_DEFINITION_SERVICE_REAUDIT_PASS
```

---

*Report path: `/Users/yanfenma/workspace/project/svc-workflow/DEFINITION_SERVICE_REAUDIT_REPORT.md`*
*Generated: 2026-07-13*
*Auditor: ZCode (independent re-audit; no implementation modifications)*
