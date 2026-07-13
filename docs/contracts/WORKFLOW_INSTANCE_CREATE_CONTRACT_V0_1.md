# Workflow Instance Create — Implementation Contract v0.1

```text
Status: IMPLEMENTATION_CONTRACT
Version: v0.1
Architecture: SVC_WORKFLOW_ARCHITECTURE_V0_3_1 (ARCHITECTURE_FROZEN)
PR: 3A
```

## 1. Command Input

```rust
pub struct CreateWorkflowInstanceCommand {
    pub principal_id: PrincipalId,
    pub idempotency_key: String,
    pub command_schema_version: String,
    pub domain_id: DomainId,
    pub definition_version_id: DefinitionVersionId,
    pub external_reference: Option<String>,
    pub external_url: Option<String>,
    pub metadata: serde_json::Value,
    pub context_payload: serde_json::Value,
}
```

The following values are never accepted from the client — they are server-generated or resolved:
`workflow_instance_id`, `context_revision_id`, `node_visit_id`, `event_id`, `command_id`, `workflow_state_version`, `event_sequence`, `revision_number`, `visit_number`, `initial_node_id`, `resolved_assignee_principal_id`, `definition_digest`, `created_at`.

## 2. Authorization

1. **Principal**: Must exist and be `enabled = true`.
2. **Domain Membership**: Caller must have at least one active (`enabled = true`) binding in `domain_role_bindings` for the target domain. Any role key is sufficient — `DOMAIN_OWNER` is not required.
3. **Domain**: Must exist and be `enabled = true`.
4. **Cross-domain**: The definition version must belong to the specified domain. A principal who is owner of domain A cannot create instances in domain B using domain B's definitions, even if they are an owner of domain A.

## 3. Lock Order

The only row lock acquired is:

```
workflow_definition_versions WHERE definition_version_id = $1 FOR UPDATE
```

This is the same lock acquired by `atomic_publish`, `atomic_deprecate`, and `atomic_revoke` in the Definition Service. All operations lock a single row, so no deadlock cycle exists.

## 4. Transaction Steps

```
BEGIN

  1. INSERT INTO workflow_command_receipts ON CONFLICT (principal_id, idempotency_key) DO NOTHING RETURNING command_id
     If receipt already exists → branch to idempotent replay (§6)

  2. SELECT ... FROM workflow_definition_versions WHERE id = $1 FOR UPDATE
     Lock the version row

  3. SELECT domain_id FROM workflow_definitions WHERE id = $1
     Verify version belongs to the input domain_id

  4. SELECT enabled FROM domains WHERE domain_id = $1
     Verify domain exists and is enabled

  5. SELECT enabled FROM principals WHERE principal_id = $1
     Verify principal exists and is enabled (re-check inside tx)

  6. SELECT 1 FROM domain_role_bindings WHERE domain_id = $1 AND principal_id = $2 AND enabled = TRUE LIMIT 1
     Verify domain membership

  7. Verify version status = PUBLISHED

  8. SELECT node_id, assignee_ref_type, fixed_principal_id FROM workflow_node_definitions
     WHERE definition_version_id = $1 AND node_type = 'DRAFT'
     Read the unique DRAFT node

  9. Resolve assignee (§5)

  10. Validate context_payload against context_schema (§7)

  11. INSERT INTO workflow_instances (with current_context_revision_id and current_node_visit_id
      pointing to IDs inserted in steps 12-13 — DEFERRED FK)

  12. INSERT INTO workflow_context_revisions (revision_number = 1, previous_revision_id = NULL)

  13. INSERT INTO workflow_node_visits (visit_number = 1, entered_by_transition_id = NULL)

  14. INSERT INTO workflow_events (INSTANCE_CREATED, event_sequence = 1)

  15. UPDATE workflow_command_receipts SET receipt_status = 'COMPLETED',
      response_status, response_body, response_digest

COMMIT
```

**Circular FK resolution**: `workflow_instances.current_context_revision_id` and `current_node_visit_id` reference `workflow_context_revisions` and `workflow_node_visits` respectively, which themselves reference `workflow_instances`. These FKs are `DEFERRABLE INITIALLY DEFERRED`. All three rows are inserted in the same transaction, so the FK constraints are satisfied at commit time.

## 5. Assignee Resolution

### WORKFLOW_CREATOR
The command principal becomes the initial assignee:
```
resolved_assignee_principal_id = command.principal_id
```

### DOMAIN_OWNER
Query the single enabled DOMAIN_OWNER for the target domain:
```sql
SELECT principal_id FROM domain_role_bindings
WHERE domain_id = $1 AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE
LIMIT 1
```
Then verify the owner principal exists and is `enabled = true`.

### FIXED_PRINCIPAL
Use the `fixed_principal_id` stored in the DRAFT node's `assignee_ref`. Verify the principal exists and is `enabled = true`.

If resolution fails at any step (not found, disabled, missing fixed_principal_id, no enabled DOMAIN_OWNER), the entire creation fails atomically with `AssigneeResolutionFailed`.

## 6. Idempotency

### 6.1 Request Hash Computation

The request hash is computed over the canonical JCS-normalized request envelope:

```json
JCS({
  "commandSchemaVersion": "...",
  "commandType": "CREATE_WORKFLOW_INSTANCE",
  "routeParameters": {},
  "requestBody": {
    "principalId": "...",
    "domainId": "...",
    "definitionVersionId": "...",
    "contextPayload": ...,
    "metadata": ...,
    "externalReference": null,
    "externalUrl": null
  }
}) → SHA-256
```

The idempotency key itself is excluded from the hash. `routeParameters` is always `{}` (no HTTP route). The `requestBody` is a nested object containing all command fields except the idempotency key.

### 6.2 First Request

```sql
INSERT INTO workflow_command_receipts (...)
VALUES (...)
ON CONFLICT (principal_id, idempotency_key) DO NOTHING
RETURNING command_id
```

If a row is returned, this transaction owns the request. Proceed with creation.

### 6.3 Existing Idempotency Key

If INSERT returns no row:

```sql
SELECT ... FROM workflow_command_receipts
WHERE principal_id = $1 AND idempotency_key = $2
FOR UPDATE
```

**Same request_hash, COMPLETED**: Replay the stored response body. No second instance, event, or state version is created. The stored response is returned as-is (extracting instance IDs from the JSON response).

**Different request_hash**: Write a `workflow_command_attempt_audits` entry with `attempt_type = 'IDEMPOTENCY_CONFLICT'`. Return `IdempotencyConflict` with the original `command_id` and `request_hash`. The original receipt is never modified.

**Same request_hash, PROCESSING**: Return `CommandStillProcessing`. Never take over or modify the original command.

### 6.4 Deterministic Failure Replay

Deterministic business failures (disabled principal, invalid context, etc.) are persisted as COMPLETED receipts with an error response body. Replaying the same idempotent request returns the same error response.

## 7. Context Validation

### Schema Validation

If the definition version has a `context_schema`:
1. Compile the schema using `jsonschema::validator_for`
2. Validate `context_payload` against the compiled schema
3. Any validation error returns `ContextValidationFailed`

If `context_schema` is `None`, any valid JSON is accepted (subject to size limits).

The schema validator is the same `jsonschema` 0.47 crate used by the Definition Service. External `$ref` resolution is not performed — the schema is already validated at publish time to contain only local fragment references.

### Service-Layer Size Limits

| Field | Limit | Check Method |
|---|---|---|
| `context_payload` | 1 MiB | `serde_json::to_vec` → `.len()` |
| `metadata` | 64 KiB | `serde_json::to_vec` → `.len()` |

### Database Size Limits (defense in depth)

| Table / Column | Limit | Mechanism |
|---|---|---|
| `workflow_context_revisions.payload` | 1 MiB | `chk_ctx_payload_size` (pg_column_size) |
| `workflow_instances.metadata` | 64 KiB | `chk_instance_metadata_size` (pg_column_size) |

Service-layer limits are checked on raw serialized bytes before JSONB encoding. Database limits are checked on PostgreSQL's JSONB binary storage size, which may differ slightly due to type overhead.

## 8. Event Field Matrix

| Field | Value |
|---|---|
| `event_type` | `INSTANCE_CREATED` |
| `source_node_visit_id` | NULL |
| `target_node_visit_id` | Initial NodeVisit ID |
| `context_revision_id` | Context Revision #1 ID |
| `submission_id` | NULL |
| `before_workflow_state_version` | 0 |
| `after_workflow_state_version` | 1 |
| `event_sequence` | 1 |
| `actor_principal_id` | Caller principal |
| `command_id` | Current Receipt command_id |
| `event_schema_version` | `"v1"` |

### Event Data

```json
{
  "definitionVersionId": "...",
  "definitionDigest": "...",
  "initialNodeId": "...",
  "assigneeResolutionType": "WORKFLOW_CREATOR"
}
```

`event_data_digest` = JCS(event_data) → SHA-256

The full context payload is never duplicated in the event data.

## 9. Success Response

```json
{
  "workflowInstanceId": "...",
  "workflowStateVersion": 1,
  "currentContextRevisionId": "...",
  "currentNodeVisitId": "...",
  "eventSequence": 1
}
```

`response_digest` = JCS(response) → SHA-256. The response is stable and persisted — idempotent replay returns the exact same response body.

## 10. Deterministic Failure vs Infrastructure Failure

### Deterministic (persisted as COMPLETED)

| Condition | Status Code | Error Code |
|---|---|---|
| Domain not found | 404 | `domain_not_found` |
| Domain disabled | 403 | `domain_disabled` |
| Principal not found | 404 | `principal_not_found` |
| Principal disabled | 403 | `principal_disabled` |
| No domain membership | 403 | `domain_membership_required` |
| Cross-domain violation | 403 | `cross_domain_violation` |
| Version not found | 404 | `definition_version_not_found` |
| Version not PUBLISHED | 409 | `version_not_published` |
| Context validation failed | 422 | `context_validation_failed` |
| Size limit exceeded | 413 | `size_limit_exceeded` |
| Assignee resolution failed | 422 | `assignee_resolution_failed` |

### Infrastructure (transaction rolls back)

- Connection drops
- Database unavailable
- Serialization failures
- Unknown SQL errors

## 11. Migration

Migration 0009 adds `external_reference TEXT` to `workflow_instances`:
```sql
ALTER TABLE workflow_instances
    ADD COLUMN IF NOT EXISTS external_reference TEXT
    CHECK (external_reference IS NULL OR char_length(external_reference) <= 512);
```

All other required tables (workflow_instances, workflow_context_revisions, workflow_node_visits, workflow_events, workflow_command_receipts, workflow_command_attempt_audits) already exist in migrations 0003-0006.

## 12. Limitations (not implemented in this PR)

- Context Revision #2+
- Context modification (revise)
- Submission / Transition / RETURN / TERMINATE
- Admin emergency override
- HTTP / gRPC / CLI
- Timer / Signal / Reassign / Subject / Parallel workflow
- Cross-instance references
