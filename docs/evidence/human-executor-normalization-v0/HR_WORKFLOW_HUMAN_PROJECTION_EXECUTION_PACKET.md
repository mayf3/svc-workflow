# HR Workflow HUMAN projection execution packet

```text
PACKET_ID = G2_OWNER_HUMAN_WORKFLOW_PROJECTION_V0
PACKET_STATUS = FROZEN_FOR_HR_EXECUTION
EXECUTION_OWNER = credential-bearing HR runtime
SOURCE_MAIN_SHA = 455b9c0e4986aa7264f7ae9deffc7aaae567443f

TARGET_PRINCIPAL_ID = 8902db0d-429a-4e37-985c-f8b92d4b78fb
TARGET_PRINCIPAL_TYPE = HUMAN
TARGET_STATUS = active

SURFACE = existing allowlisted workflow.admin provisioning path
METHOD = POST
PATH = /internal/v1/admin/principals
IDEMPOTENCY_KEY = g2-human-projection-v0:8902db0d-429a-4e37-985c-f8b92d4b78fb
REQUEST_ID = g2-owner-human-projection-v0-20260914

PRODUCTION_WORKFLOW_NORMALIZATION_AUTHORIZED_IN_THIS_PACKET = NO
```

## Preconditions

The HR runtime must obtain its own direct Agent access token through its
existing credential-bearing runtime. The credential and token must remain
inside that runtime and must not be returned in logs, messages, receipts, or
review artifacts.

Before the write, mechanically verify only:

```text
token_use = access
principal_type = agent
audience = svc-workflow
scope includes workflow.admin
token subject is in WORKFLOW_PROVISIONING_PRINCIPAL_IDS
provisioning actor is already projected and enabled
```

Use the configured svc-workflow origin. Do not substitute another service,
database, Principal UUID, type, source, idempotency key, or request body.

## Exact request

Headers, in addition to the runtime's secret-isolated Bearer token:

```text
Content-Type: application/json
Idempotency-Key: g2-human-projection-v0:8902db0d-429a-4e37-985c-f8b92d4b78fb
X-Request-ID: g2-owner-human-projection-v0-20260914
```

Body:

```json
{
  "principalId": "8902db0d-429a-4e37-985c-f8b92d4b78fb",
  "principalType": "human",
  "enabled": true,
  "source": "auth-service"
}
```

Success response must identify the exact Principal and `enabled: true`. No
other provisioning endpoint or body is part of this packet.

## Fresh readback

After the POST, use the same authorized runtime to call:

```text
GET /internal/v1/admin/principals/8902db0d-429a-4e37-985c-f8b92d4b78fb
```

The gate passes only for this exact response projection:

```json
{
  "principalId": "8902db0d-429a-4e37-985c-f8b92d4b78fb",
  "principalType": "human",
  "enabled": true
}
```

Record only the HTTP status, sanitized response above, request ID,
idempotency key, actor Principal ID, and server receipt/readback coordinates.
Never record the credential or Bearer token.

```text
WORKFLOW_PRINCIPAL_PROJECTION = READY
PRINCIPAL_ID = 8902db0d-429a-4e37-985c-f8b92d4b78fb
PRINCIPAL_TYPE = HUMAN
STATUS = active
G2_PROJECTION_GATE = DONE
```

If the readback is missing, disabled, a different type, or a different UUID,
the gate fails and no Workflow normalization may start.

## Unknown outcome and stop rules

On timeout or lost response, do not mint a new idempotency key and do not alter
the body. First perform the exact GET readback. If the target is not ready, an
authorized operator may replay only the identical POST with the identical
idempotency key and body; `425 command_still_processing` remains a stop/readback
condition, not permission to issue another command identity.

Immediately stop on `principal_type_conflict`, any authorization/allowlist
failure, an unexpected existing Principal, or any response that cannot be
bound to the exact request.

## Explicit prohibitions

```text
NO second Human identity
NO Agent-to-Human conversion
NO raw database insert or update
NO 20-row Workflow normalization in this execution
NO Workflow transition, dispatch, completion, or evidence
NO unrelated workflow.admin mutation
NO credential or token disclosure
```

This packet authorizes exactly one idempotent Principal projection. It does not
authorize the later exact-20 Workflow mutation.
