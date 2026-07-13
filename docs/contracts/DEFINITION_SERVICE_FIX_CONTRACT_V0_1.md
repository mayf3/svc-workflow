# Definition Service Audit Fix — Implementation Contract v0.1

```text
Status: IMPLEMENTATION_CONTRACT (audit fix supplement)
Version: v0.1
Architecture: SVC_WORKFLOW_ARCHITECTURE_V0_3_1 (ARCHITECTURE_FROZEN)
Base: IMPLEMENTATION_CONTRACT_V0_1, POSTGRES_STORAGE_CONTRACT_V0_1
Fix SHA: 4f5d84c6 → (new commit)
```

This document records the concrete fixes applied to close the Definition Service
audit findings (B-1, B-2, H-1–H-5, M-1, M-4, M-5, M-6). It supplements the
frozen architecture and base implementation contract; it does not change them.

---

## 1. Transaction Atomicity (B-1)

### 1.1 PublishVersion

`PgDefinitionRepository::atomic_publish()` executes the entire publish in one
SQLx transaction:

```
BEGIN
  1. SELECT ... FOR UPDATE (lock version row)
  2. Verify DRAFT status
  3. Read domain, verify enabled + DOMAIN_OWNER
  4. Re-read complete graph (nodes + transitions)
  5. Re-compute digest from data read inside transaction
  6. If digest != caller-supplied precomputed_digest → ConcurrentModification
  7. UPDATE status = PUBLISHED, digest, published_by_principal_id
COMMIT
```

The service calls `atomic_publish(version_id, actor, precomputed_digest)`
after its own validation round (graph, schema, fixed principals, digest).
The digest is re-computed inside the transaction as a consistency check:
if a concurrent `ReplaceDraftGraph` modified the graph between the service's
read and the atomic publish, the digest will not match and the operation
fails with `ConcurrentModification` (caller retries).

### 1.2 ReplaceDraftGraph

The repository's `replace_draft_graph()` was already transactional with
`FOR UPDATE` lock. The service-level `lock_version` call was **removed**
to eliminate the TOCTOU window between the service lock and the repository
transaction.

The service now calls `get_version` (no lock) for an initial DRAFT check,
then the repository's transaction provides the authoritative lock.

### 1.3 DeprecateVersion / RevokeVersion

Brand-new `atomic_deprecate()` and `atomic_revoke()` methods on
`PgDefinitionRepository` execute status transitions in a single transaction:

```
BEGIN
  1. SELECT ... FOR UPDATE (lock version row)
  2. Verify current status (PUBLISHED for deprecate; PUBLISHED|DEPRECATED for revoke)
  3. Verify domain enabled + DOMAIN_OWNER
  4. UPDATE version_status, actor timestamp + actor principal_id
COMMIT
```

### 1.4 Lock Coordination

Both `atomic_publish` and `replace_draft_graph` take `FOR UPDATE` on the
same `workflow_definition_versions` row. PostgreSQL serializes the two via
row-level locking:
- If Replace obtains the lock first, Publish re-reads the new graph inside
  its transaction and the digest check fails → Publish fails (retry).
- If Publish obtains the lock first, Replace sees PUBLISHED status and fails.

---

## 2. JSON Schema Validation (B-2 / M-2)

### 2.1 Compilation check

`validate_json_schema()` now calls `jsonschema::validator_for(schema)` and
**propagates the `Result`** via `map_err`. A schema that fails to compile
(including those with invalid `$ref`, malformed keywords, type errors)
produces a `GraphValidationError` with code `INVALID_CONTEXT_SCHEMA` or
`INVALID_SUBMISSION_SCHEMA`.

### 2.2 External reference prohibition

`check_external_refs()` recursively traverses the schema JSON tree before
compilation, rejecting any `$ref`, `$dynamicRef`, or `$recursiveRef` value
that does **not** start with `#` (local fragment only).

Rejected patterns:
- `http://` / `https://` remote references
- `file://` local file references
- Relative paths (e.g. `../schema.json`)

Allowed:
- `#` (root)
- `#/$defs/...` or `#/definitions/...` local fragments

### 2.3 Schema dialect

The `jsonschema` crate (v0.47) is compiled with the default features set,
which include `resolve-http` and `resolve-file`. The `check_external_refs`
pre-flight check prevents the validator from ever being invoked with a schema
containing external references, so the resolver is never triggered during
publish.

---

## 3. Directed Reachability (H-1)

`compute_weakly_reachable` → renamed to `compute_directed_reachable`.

The function builds an adjacency list from `source_node_id → target_node_id`
(forward direction only) and performs a BFS from the DRAFT node. It no longer
traverses edges backwards (target → source).

A node that is only reachable via a backwards RETURN edge (i.e., the node has
an outgoing edge to a downstream node, but no incoming edge from a directed
path originating at DRAFT) will **not** be considered reachable.

Validation error code remains `NODE_NOT_REACHABLE`.

---

## 4. Node Assignee Rules (H-2)

Strictly enforced per frozen contract §3.1.7 and §3.1.8:

| Node type   | Rule |
|-------------|------|
| DRAFT       | Must be `WORKFLOW_CREATOR`. Must not have `fixed_principal_id`. |
| NORMAL      | Must have a legal assignee ref type. `FIXED_PRINCIPAL` requires a non-null `fixed_principal_id`. `WORKFLOW_CREATOR` and `DOMAIN_OWNER` must not have `fixed_principal_id`. |
| TERMINAL    | Must not have `FIXED_PRINCIPAL` ref type. Must not have `fixed_principal_id`. |

Error codes: `DRAFT_NOT_WORKFLOW_CREATOR`, `TERMINAL_HAS_ASSIGNEE`,
`FIXED_PRINCIPAL_MISSING_ID`, `UNEXPECTED_FIXED_PRINCIPAL`.

---

## 5. Primary Transition Effect (H-3)

During primary-trunk validation, each node's `primary_advance_transition_id`
is resolved and the corresponding transition's `transition_effect` is checked.
If the effect is not `ADVANCE`, a new error `PRIMARY_NOT_ADVANCE` is emitted.

This catches:
- Primary pointing to a RETURN transition
- Primary pointing to a TERMINATE transition
- Primary pointing to a transition with any non-ADVANCE effect

---

## 6. Lifecycle Actor Fields (H-4)

### 6.1 Write paths

| Operation  | Column set                  |
|------------|-----------------------------|
| Publish    | `published_by_principal_id` |
| Deprecate  | `deprecated_by_principal_id` |
| Revoke     | `revoked_by_principal_id`  |

Each is set to `actor_principal_id` from the command. Each retains its value
through subsequent lifecycle transitions (no overwrite).

### 6.2 Read paths

`WorkflowDefinitionVersionRow` now SELECTs all three actor columns and maps
them to `PrincipalId` values in `WorkflowDefinitionVersion`. All repository
queries that return `WorkflowDefinitionVersion` (`get_version`, `lock_version`,
`list_versions`) include the three columns.

---

## 7. Domain Authorization (H-5)

### 7.1 Read operations

All four read queries now verify the caller is a DOMAIN_OWNER for the
definition's domain before returning data:
- `get_definition`
- `get_definition_version`
- `list_definition_versions`
- `get_complete_version_graph`

If not authorized, `PermissionDenied` is returned. No definition content
(instructions, schemas, fixed principal references) is leaked.

### 7.2 Write authorization

`ValidateDraftVersion` now requires DOMAIN_OWNER (closing the gap reported
in the audit).

### 7.3 Domain enabled gate (M-4)

All write operations requiring domain access now call `ensure_domain_enabled()`
before proceeding:
- `CreateDraftVersion`
- `ReplaceDraftGraph`
- `ValidateDraftVersion`
- `PublishVersion`
- `DeprecateVersion`
- `RevokeVersion`

`CreateDefinition` already had this check.

---

## 8. Typed Database Errors (M-5)

`map_db_error()` maps known PostgreSQL error conditions to stable domain errors:

| DB condition                          | Domain error                  |
|---------------------------------------|-------------------------------|
| `23505` + `definition_key`           | `DefinitionKeyConflict`       |
| `23505` + `version_number`           | `ConcurrentModification`      |
| Trigger `graph_immutable:` prefix    | `VersionNotDraft`             |
| Trigger `status_transition:` prefix  | `InvalidLifecycleTransition`  |
| All other errors                     | `StorageError(raw)`           |

Applied throughout `PgDefinitionRepository` in place of raw `StorageError`.

---

## 9. CreateDefinition Race Fix (M-6)

Removed the redundant `definition_key_exists` pre-check from
`create_definition`. Uniqueness is guaranteed solely by the DB unique
constraint `(domain_id, definition_key)`. The repository maps `23505`
to `DefinitionKeyConflict`.

---

## 10. Configurable context_schema Patch (M-1)

No longer required a dedicated `PatchField` type. The existing `Option<Value>`
distinguishes three states:
- `None` → no update (keep existing context_schema)
- `Some(Value::Null)` → explicit clear (sets column to NULL)
- `Some(object)` → replace

Documented in the repository's `replace_draft_graph` method.

---

## 11. Digest Consistency (M-3)

The read-back digest test (`test_digest_readback_consistency`) in
`16_definition_service_audit_fix_tests.rs` verifies:
1. Publish a version with a complete graph
2. Re-read all graph data from the database
3. Re-compute the JCS + SHA-256 digest using the same algorithm
4. Assert: stored digest == recomputed digest
