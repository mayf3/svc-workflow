# svc-workflow Identity & Workflow Provisioning API v0

```text
Status: FROZEN_FOR_PROVISIONING_READY
```

This contract defines the identity provisioning API for `svc-workflow`. It
provides CRUD endpoints for principals, domains, role bindings, and
read-only definition-version queries, enabling automated provisioning
without direct database writes.

---

## 1. Scope & Actor

All provisioning endpoints require:

| Requirement | Details |
|---|---|
| Scope | `workflow.provision` |
| Allow-list | `JWT.sub` must be in `WORKFLOW_PROVISIONING_PRINCIPAL_IDS` |
| Token type | `token_use=access` only (OBO rejected) |
| Principal type | `agent` only (human tokens rejected for provisioning) |

The provisioning actor (`JWT.sub`) is recorded in all audit logs.

---

## 2. Endpoints

All under `/internal/v1/admin/`:

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/internal/v1/admin/principals` | Upsert a principal |
| GET | `/internal/v1/admin/principals/{principalId}` | Read a principal |
| POST | `/internal/v1/admin/domains` | Upsert a domain |
| GET | `/internal/v1/admin/domains/{domainId}` | Read a domain |
| PUT | `/internal/v1/admin/domains/{domainId}/role-bindings/{principalId}` | Create/enable role binding |
| DELETE | `/internal/v1/admin/domains/{domainId}/role-bindings/{principalId}` | Disable role binding |
| PUT | `/internal/v1/admin/domains/{domainId}/owner` | Atomic owner replacement |
| GET | `/internal/v1/admin/definition-versions/{definitionVersionId}` | Query definition version |

---

## 3. Principal

POST body:
```json
{
  "principalId": "uuid",
  "principalType": "human | agent",
  "enabled": true,
  "source": "auth-service",
  "sourceRevision": "optional"
}
```

Rules:
- `principalType` cannot change after creation (`409 principal_type_conflict`)
- `source` required and non-empty
- `enabled` can toggle between `true` and `false`
- Idempotent: same body + same `Idempotency-Key` → same response

---

## 4. Domain

POST body:
```json
{
  "domainId": "uuid",
  "domainKey": "stable-key",
  "displayName": "optional",
  "enabled": true
}
```

Rules:
- `domainKey` must be unique across all domains
- Changing `domainKey` to a key used by another domain → `409 domain_identity_conflict`
- `domainKey` required, non-empty

---

## 5. Role Binding

PUT body:
```json
{
  "roleKey": "DOMAIN_OWNER",
  "enabled": true
}
```

DELETE body:
```json
{
  "roleKey": "DOMAIN_OWNER"
}
```

Rules:
- Principal must exist and be enabled
- Domain must exist and be enabled (for PUT)
- Second active `DOMAIN_OWNER` → `409 domain_owner_conflict`
- DELETE soft-deletes (sets `enabled=false`, records `disabled_at`)

---

## 6. Owner Replacement

PUT body:
```json
{
  "newOwnerPrincipalId": "uuid"
}
```

Rules:
- Atomic: disables old owner + enables new owner in a single transaction
- If no current owner, only enables the new one
- New owner must exist and be enabled

---

## 7. Definition Version Query

GET response:
```json
{
  "definitionVersionId": "uuid",
  "definitionKey": "...",
  "versionNumber": 1,
  "versionStatus": "PUBLISHED",
  "digest": "sha256hex",
  "nodeCount": 3,
  "transitionCount": 2,
  "canCreateInstances": true
}
```

`canCreateInstances` is `true` only for `PUBLISHED` versions.

---

## 8. Idempotency

All write endpoints require `Idempotency-Key` header (1–128 visible ASCII chars).
Reuses the existing `workflow_command_receipts` table with `command_type TEXT`.

Replay behavior:
- Same key + same body → stored response returned
- Same key + different body → `409 idempotency_conflict`
- Processing → `425 command_still_processing`

---

## 9. Error Codes

| HTTP | Code | Meaning |
|------|------|---------|
| 400 | `invalid_idempotency_key` | Malformed key |
| 401 | `missing_token` / `invalid_token` | Auth failure |
| 403 | `insufficient_scope` | Missing `workflow.provision` |
| 403 | `provisioning_not_allowed` | Not in allow-list / OBO token |
| 404 | `principal_not_found` | Principal does not exist |
| 404 | `domain_not_found` | Domain does not exist |
| 404 | `definition_version_not_found` | Version does not exist |
| 409 | `idempotency_conflict` | Key reused with different body |
| 409 | `principal_type_conflict` | Type mismatch on existing principal |
| 409 | `domain_identity_conflict` | Domain key owned by another domain |
| 409 | `domain_owner_conflict` | Domain already has an active owner |
| 422 | `invalid_input` | Validation failure |
| 500 | `internal_consistency_error` | Unexpected state |
| 503 | `service_unavailable` | Storage unavailable |

---

## 10. Configuration

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `WORKFLOW_PROVISIONING_PRINCIPAL_IDS` | Yes | — | Comma-separated UUID list of provisioning actors |

---

## 11. Not implemented (this version)

- Principal provisioning API for auth-service identities
- Bulk import / batch operations
- ADC Mapping Ledger
- Space/Domain creation
- Definition/workflow creation
