# HUMAN executor normalization V1 production preflight

```text
OBSERVED_AT=2026-09-15T09:51:54+08:00
ENVIRONMENT=production svc_workflow_dogfood_clean
SOURCE_MAIN=b1c9a02fbffc7386d428863d0a91bcea98499f64
V0_PLAN_SHA256=b349e203c00ac82e286666a89dbedd6a17f77e0221090a1a9f2db51d8a253199
V1_PLAN_SHA256=bba710b9790fed4c0136b9a0f33186f87f11be9e5da3f76e08784bfbce8dd871
TRANSACTION_MODE=BEGIN READ ONLY
SECRET_MATERIAL_RECORDED=NO
```

The release-built V0 operator was run with `--plan` only. It returned:

```json
{"error":"CONFLICT: workflow 012e72de-4851-454e-b5f2-05b0db15707d terminal state is not exact","outcome":"CONFLICT","writes":0}
```

The conflict results from the pre-write deterministic Event-sequence scan: two
V0 rows now have a persisted sequence 2 Event. Exact-ID read-only queries then
produced these bounded results:

```text
V0_PLAN_ROWS=20
V0_EXACT_PREIMAGES=18
V0_TARGET_VISITS_PRESENT=0
V0_NORMALIZATION_RECEIPTS=0
V0_NORMALIZATION_EVENTS=0
V0_NORMALIZATION_AUDITS=0

V1_PLAN_ROWS=18
V1_EXACT_PREIMAGES=18
V1_TARGET_VISIT_COLLISIONS=0

TARGET_PRINCIPAL_ID=8902db0d-429a-4e37-985c-f8b92d4b78fb
TARGET_PRINCIPAL_TYPE=HUMAN
TARGET_PRINCIPAL_ENABLED=true
```

The two excluded exact IDs have the following persisted current facts:

```text
WORKFLOW=2edf5b53-1dd9-4c93-b356-4029d3fe1adb
STATE_VERSION=2
EVENT=WORKFLOW_TRANSITION_COMMITTED
EFFECT=ADVANCE
TRANSITION_KEY=advance-to-completed
EVENT_AT=2026-09-14T18:10:34.906070+08:00
CURRENT_NODE_KEY=completed
CURRENT_NODE_TYPE=TERMINAL
CURRENT_ASSIGNEE=NULL

WORKFLOW=f0ebdef1-8cab-4b97-82ac-af92b8ed3e12
STATE_VERSION=2
EVENT=WORKFLOW_TRANSITION_COMMITTED
EFFECT=ADVANCE
TRANSITION_KEY=advance-to-completed
EVENT_AT=2026-09-14T18:44:16.956585+08:00
CURRENT_NODE_KEY=completed
CURRENT_NODE_TYPE=TERMINAL
CURRENT_ASSIGNEE=NULL
```

Queries joined only exact UUIDs from the two frozen TSV plans to
`workflow_instances`, current/source/target `workflow_node_visits`,
`workflow_context_revisions`, DefinitionVersion/Definition/node definitions,
Principals, Events, Receipts, and security audits. No title or description was
used to select V1 membership. The database transaction was committed read-only;
no production row was modified.
