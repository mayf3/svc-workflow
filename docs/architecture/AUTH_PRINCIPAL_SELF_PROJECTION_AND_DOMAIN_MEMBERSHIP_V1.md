# Auth Principal Self-Projection and Domain Membership V1

```text
STATUS=FROZEN
TYPE=Contract Change Request
CHANGE_PATH=svc-workflow internal HTTP contract
AUDIT_STOP_LEVEL=BLOCKER_HIGH_ONLY
```

## Architecture Decisions

```text
AUTH_IS_GLOBAL_IDENTITY_AUTHORITY=true
WORKFLOW_STORES_LOCAL_IDENTITY_PROJECTION=true

AGENT_SELF_PROJECTION_ALLOWED=true
DOMAIN_OWNER_TRIGGERED_PROJECTION_ALLOWED=false

SELF_PROJECTION_REQUIRES_DIRECT_MACHINE_TOKEN=true
OBO_SELF_PROJECTION_ALLOWED=false

PROJECTION_PRINCIPAL_ID_SOURCE=token.sub
REQUEST_BODY_PRINCIPAL_ID_ALLOWED=false
NAME_OR_EMAIL_IDENTITY_MATCHING_ALLOWED=false

DOMAIN_OWNER_CAN_MANAGE_DOMAIN_MEMBER=true
DOMAIN_OWNER_CAN_MANAGE_DOMAIN_OWNER=false
DOMAIN_OWNER_CAN_CREATE_WORKFLOW_PRINCIPAL=false

NORMAL_ONBOARDING_REQUIRES_WORKFLOW_ADMIN=false
DOMAIN_MEMBER_VISIBILITY_UNCHANGED=true
STATE_MACHINE_UNCHANGED=true
AUTH_CONTRACT_UNCHANGED=true
```

## Rationale

Auth-service does not expose a REST endpoint that accepts a bare `principal_id` UUID and returns MachinePrincipal status. Attempting to build such a contract would require modifying auth-service, which is outside the scope of this Contract Change Request.

The alternative design fixes Agent self-projection as the sole onboarding path:

1. An Agent presents its own Direct Machine Token (`token_use=access`, `principal_type=agent`, signed RS256)
2. svc-workflow verifies the token via JWKS (auth-service's existing formal authority)
3. The verified `token.sub` is projected as `workflow_principal_id` — the Agent explicitly establishes its local identity
4. A Domain Owner may then add the projected principal as a `DOMAIN_MEMBER`

This preserves auth-service as the single global identity authority while avoiding a cross-service principal lookup contract.

## Endpoints

### Self-Projection

```
PUT /internal/v1/principals/me
```

- Direct Machine Token only (`token_use=access`, no OBO)
- `principal_type=agent` (enforced by auth verification)
- `scope=workflow.read`
- Identity source: verified `token.sub`
- Creates local projection in `principals` table
- Does NOT create any `DomainRoleBinding`
- Idempotent: re-insertion succeeds; disabled/type-conflict fails

### Domain Member Management

```
GET    /internal/v1/domains/{domainId}/members
PUT    /internal/v1/domains/{domainId}/members/{principalId}
DELETE /internal/v1/domains/{domainId}/members/{principalId}
```

- Direct Machine Token only
- `scope=workflow.read` for GET, `scope=workflow.execute` for PUT/DELETE
- Caller must be `DOMAIN_OWNER` of the target domain
- Target must have completed self-projection (`principal_not_registered` otherwise)
- Only manages `DOMAIN_MEMBER` role

## Authorization

All endpoints use `token.sub` as the sole actor. No `client_id`, `azp`, `act.sub`, or request-body field may substitute.

## Audit

All mutations (self-projection, member add, member remove) write a durable record to `workflow_security_audits` in the same database transaction. `tracing::info!` is operational logging only.

## Exclusions

- auth-service is NOT modified
- Admin provisioning (`/internal/v1/admin/**`) is NOT modified
- State machine, task visibility, `DOMAIN_MEMBER` permissions unchanged
- No new roles or scopes
- No Batch 1 agent data operations
