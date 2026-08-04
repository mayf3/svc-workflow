# Workflow Runtime HTTP Contract V1

**Status:** Current-state freeze
**Date:** 2026-07-26
**Contract Bundle Version:** 1.4.1
**Service Version:** 0.3.1
**Database Schema Version:** 0014
**Runtime apiContractVersion:** internal-v0
**Scope:** Runtime workflow endpoints only (control-plane `/internal/v1/admin/**` excluded)

---

## 1. Base URLs

All endpoints are served from a single service with no path prefix:

```
http://<host>:<port>
```

The service binds to `WORKFLOW_BIND_ADDR:WORKFLOW_PORT` (default `127.0.0.1:8989`).

---

## 2. Endpoints

### 2.1 Health / Liveness

| Method | Path        | Scope         | Auth | Description           |
|--------|-------------|---------------|------|-----------------------|
| GET    | `/healthz`  | none          | no   | Returns `{"status":"ok"}` |
| GET    | `/readyz`  | none          | no   | Returns `{"status":"ready"}` or 503 with `migration_version_mismatch` |
| GET    | `/version`  | none          | no   | Service version metadata |

**Ready check** verifies the database migration version equals `EXPECTED_MIGRATION_VERSION` (currently `14`). A mismatch returns HTTP 503 with error code `migration_version_mismatch`.

### 2.2 Workflow Instances

| Method | Path                                      | Scope              | Auth | Description            |
|--------|-------------------------------------------|--------------------|------|------------------------|
| POST   | `/internal/v1/workflow-instances`         | `workflow.execute` | yes  | Create instance        |
| GET    | `/internal/v1/workflow-instances/{id}`    | `workflow.read`    | yes  | Get instance detail    |
| POST   | `/internal/v1/workflow-instances/{id}/transitions` | `workflow.execute` | yes | Execute transition |
| GET    | `/internal/v1/workflow-instances/{id}/timeline`    | `workflow.read`    | yes | Event timeline   |
| GET    | `/internal/v1/workflow-instances/{id}/submissions` | `workflow.read`    | yes | Submission history |

### 2.3 Worklists

| Method | Path                                                    | Scope           | Auth | Description                |
|--------|----------------------------------------------------------|-----------------|------|----------------------------|
| GET    | `/internal/v1/worklists/assigned-to-me`                   | `workflow.read` | yes  | Items assigned to actor    |
| GET    | `/internal/v1/worklists/creator-owned-drafts`             | `workflow.read` | yes  | Drafts created by actor    |

### 2.4 Domain Owner List

| Method | Path                                        | Scope           | Auth | Description                     |
|--------|----------------------------------------------|-----------------|------|---------------------------------|
| GET    | `/internal/v1/workflow-instances/domain`     | `workflow.read` | yes  | Domain-wide instance list (DOMAIN_OWNER only) |

### 2.5 Agent Self-Projection

| Method | Path                             | Scope              | Auth | Description                                           |
|--------|----------------------------------|--------------------|------|-------------------------------------------------------|
| PUT    | `/internal/v1/principals/me`      | `workflow.read`    | yes  | Self-project verified token identity into local principals table |

Agents use this endpoint to project their Auth V1 Direct Machine identity into the workflow service. The projected `principal_id` equals `token.sub`. Only `token_use=access` (Direct) tokens are accepted; OBO tokens are rejected.

The request body is empty — the identity comes exclusively from the verified JWT.

**Responses:**
- `200` — principal already projected or newly created (`{"principalId": "...", "created": true|false}`)
- `403` — `direct_token_required` (OBO or delegation detected)
- `403` — `principal_disabled` (existing projection is disabled)
- `409` — `principal_projection_conflict` (existing principal has a different type)

### 2.6 Domain Member Management

| Method | Path                                                          | Scope               | Auth | Description                                      |
|--------|---------------------------------------------------------------|---------------------|------|--------------------------------------------------|
| GET    | `/internal/v1/domains/{domainId}/members`                     | `workflow.read`     | yes  | List DOMAIN_MEMBER bindings for a domain          |
| PUT    | `/internal/v1/domains/{domainId}/members/{principalId}`       | `workflow.execute`  | yes  | Add a principal as DOMAIN_MEMBER (must be self-projected first) |
| DELETE | `/internal/v1/domains/{domainId}/members/{principalId}`       | `workflow.execute`  | yes  | Remove a DOMAIN_MEMBER binding                    |

All endpoints require a Direct Machine Token (`token_use=access`). The caller must be `DOMAIN_OWNER` of the target domain.

The target principal must have completed self-projection (`PUT /internal/v1/principals/me`). Members that have not self-projected return `principal_not_registered`.

**GET Members** uses cursor pagination with `beforeCreatedAt` (RFC 3339) and `beforeId` (UUID) query parameters, same convention as section 2.4.

**PUT Add Member** is idempotent. Re-adding an existing member returns success. Adding a `DOMAIN_OWNER` as a member returns `principal_is_owner`.

**DELETE Remove Member** only affects `DOMAIN_MEMBER` bindings. `DOMAIN_OWNER` bindings cannot be modified. Returns `member_not_found` if no active binding exists.

**Error codes specific to these endpoints:**
- `direct_token_required` (403) — OBO or delegated token used
- `principal_not_registered` (404) — target has not self-projected
- `principal_projection_conflict` (409) — existing principal type conflict
- `principal_disabled` (403) — principal is disabled
- `not_domain_owner` (403) — caller is not a domain owner
- `principal_is_owner` (409) — target is a DOMAIN_OWNER, cannot be a member
- `member_not_found` (404) — no active DOMAIN_MEMBER binding to remove

### 2.7 Domain Definition Management

| Method | Path | Scope | Auth | Description |
|--------|------|-------|------|-------------|
| GET | `/internal/v1/domains/{domainId}/definitions` | `workflow.read` | yes | List definitions in domain (paginated) |
| GET | `/internal/v1/domains/{domainId}/definitions/{definitionId}` | `workflow.read` | yes | Get definition detail + versions |
| POST | `/internal/v1/domains/{domainId}/definitions` | `workflow.execute` | yes | Create workflow definition (idempotent) |
| POST | `/internal/v1/domains/{domainId}/definitions/{definitionId}/versions` | `workflow.execute` | yes | Create new draft version (idempotent) |
| PUT | `/internal/v1/domains/{domainId}/definitions/{definitionId}/draft` | `workflow.execute` | yes | Replace draft graph (idempotent) |
| POST | `/internal/v1/domains/{domainId}/definitions/{definitionId}/publish` | `workflow.execute` | yes | Publish draft version (idempotent) |
| POST | `/internal/v1/domains/{domainId}/definitions/{definitionId}/archive` | `workflow.execute` | yes | Archive definition (idempotent) |

All write endpoints require a Direct Machine Token (`token_use=access`). The caller must be `DOMAIN_OWNER` of the target domain.

**List Definitions** uses cursor pagination with `beforeCreatedAt` (RFC 3339) and `beforeId` (UUID) query parameters. Optional `includeArchived` query parameter (default false) controls whether archived definitions are returned.

**Publish** requires `versionId` and `expectedRevision` in the request body.
`expectedRevision` is the current `definition_digest` of the version. If the digest
does not match, the request is rejected with `revision_conflict` (409) to prevent
silent overwrites.

**List Definitions** returns `not_domain_owner` if the caller is not a DOMAIN_OWNER.
All other definition-specific endpoints return `definition_not_found` for any
permission or existence failure, preventing cross-domain object existence leaks.

**Error codes specific to these endpoints:**
- `definition_not_found` (404) — definition does not exist or is not visible
- `definition_key_conflict` (409) — definition key already exists in the domain
- `definition_version_immutable` (409) — version is not in DRAFT status
- `definition_not_editable` (409) — definition is archived
- `revision_conflict` (409) — expectedRevision does not match current digest

---

## 3. Authentication

### 3.1 JWT Bearer Token

All runtime endpoints (except health/ready/version) require a Bearer JWT in the `Authorization` header.

**JWT Claims:**

| Field            | Value                  | Notes                             |
|------------------|------------------------|-----------------------------------|
| `sub`            | Principal UUID         | Actor identity                    |
| `iss`            | `auth-service`         |                                   |
| `aud`            | `svc-workflow`         |                                   |
| `scope`          | Space-separated scopes | e.g., `"workflow.execute workflow.read"` |
| `principal_type` | `agent`                | Both current auth modes reject other values with 401 `invalid_principal_type` |
| `type`           | `access`               |                                   |
| `version`        | `v1`                   |                                   |

**Auth modes:** `TestHs256` (for development) or `Jwks` (production via JWKS endpoint).

### 3.2 Scope Enforcement

| Scope               | Required For                                    |
|---------------------|-------------------------------------------------|
| `workflow.execute`  | POST create, POST transition                    |
| `workflow.read`     | GET detail, GET timeline, GET submissions, GET worklists, GET domain list |

Missing scope returns `403` with code `forbidden`.

---

## 4. Wire Format

### 4.1 Request Body Convention

**All request DTOs** use `camelCase` field names with `deny_unknown_fields`:
- Create: `domainId`, `definitionVersionId`, `externalReference`, `externalUrl`, `metadata`, `contextPayload`
- Transition: `transitionDefinitionId`, `expectedWorkflowStateVersion`, `submissionPayload`
- Timeline query: `after`, `limit`
- Worklist query: `beforeCreatedAt`, `beforeId`, `limit`
- Domain list query: `domainId`, `beforeCreatedAt`, `beforeId`, `limit`, `definitionKey`, `lifecycle`, `currentNodeKey`, `assigneePrincipalId`, `status`

### 4.2 Response Body Convention

**Create** and **Transition** responses use `camelCase`.

**Timeline** response envelope uses `camelCase` for `nextCursor` (integer), while
the event items inside `items` retain their actual `snake_case` storage/query DTO
field names (`event_id`, `workflow_instance_id`, `event_sequence`, and so on).

**Worklist** and **Domain list** responses use `snake_case` (from `query_types::Page`):
- `items` — array of items
- `next_cursor` — composite cursor object with `created_at` (RFC 3339 string) and `id` (UUID), or `null` for last page

### 4.3 Error Envelope

All errors return:

```json
{
  "error": {
    "code": "error_code_string",
    "message": "Human-readable message",
    "details": { ... }
  }
}
```

`details` is optional and omitted when not applicable.

### 4.4 Pagination

**Worklist and Domain List** use a composite cursor:
- Query params: `beforeCreatedAt` (RFC 3339) + `beforeId` (UUID)
- Both must be present together or both absent
- Invalid/missing values return 422 `invalid_cursor`
- Response cursor field: `next_cursor` (object with `created_at`, `id`) or `null`
- Sort: `created_at DESC, workflow_instance_id DESC`
- Default limit: 20
- Maximum limit: 100 (values exceeding max return 422)

**Timeline** uses a numeric cursor:
- Query param: `after` (integer event sequence)
- Response cursor field: `nextCursor` (integer)
- Default/sort documented in codebase

### 4.5 Idempotency

**Create** and **Transition** require the `Idempotency-Key` header (1-128 visible ASCII characters). Replay returns the same response. Conflict returns 409 `idempotency_conflict` with no details.

### 4.6 Detail Visibility

Instance detail returns:
- `{"visibility": "full", "detail": {...}}` — for the current assignee and domain owner
- `{"visibility": "historical_participant", "detail": {...}}` — for past participants

Non-visible instances return 404 (same response as non-existent instance ID).

---

## 5. Authorization Boundaries

### 5.1 Domain isolation

All queries filter by domain membership (`domain_role_bindings` with `MEMBER`, `CONTRIBUTOR`, or `DOMAIN_OWNER` role and `enabled = TRUE`). A principal without an active binding in a domain cannot see instances from that domain in worklists.

### 5.2 Domain Owner List

`GET /internal/v1/workflow-instances/domain` additionally requires `DOMAIN_OWNER` role for the domain. Non-owners get 404.

### 5.3 Current Assignee gates

Transition execution requires the principal to be the current node visit's assignee. Non-assignees get 403 `principal_not_assignee`.

### 5.4 Disabled principal / domain

- Disabled principal: JWT authentication succeeds (token is valid) but application-level checks may return 403 `principal_disabled`
- Disabled domain: instances remain visible to domain owners (current behavior)

---

## 6. Control-Plane Exclusion

The following admin endpoints exist at `/internal/v1/admin/**` but are **excluded from the Runtime Contract V1**:

```
POST   /internal/v1/admin/principals
GET    /internal/v1/admin/principals/{principalId}
POST   /internal/v1/admin/domains
GET    /internal/v1/admin/domains/{domainId}
PUT    /internal/v1/admin/domains/{domainId}/role-bindings/{principalId}
DELETE /internal/v1/admin/domains/{domainId}/role-bindings/{principalId}
PUT    /internal/v1/admin/domains/{domainId}/owner
GET    /internal/v1/admin/definition-versions/{definitionVersionId}
```

These belong to the control plane and are outside the runtime contract scope.

---

## 7. Version Metadata

Health endpoint:
- `/healthz` → `{"status":"ok"}`
- `/readyz` → `{"status":"ready"}` or `{"error":{"code":"migration_version_mismatch","message":"..."}}` with 503

Version endpoint:
```json
{
  "service": "svc-workflow",
  "version": "0.3.1",
  "gitSha": "<git-commit>",
	  "schemaVersion": "0014",
  "apiContractVersion": "internal-v0"
}
```
