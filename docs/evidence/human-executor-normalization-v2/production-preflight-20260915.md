# HUMAN executor normalization V2 production preflight

```text
OBSERVED_AT=2026-09-15T10:45:00+08:00
ENVIRONMENT=production svc_workflow_dogfood_clean
SOURCE_MAIN=7c3beec0ee058aa896b86e58443c502a48d23d11
TRANSACTION_MODE=BEGIN READ ONLY
SECRET_MATERIAL_RECORDED=NO
V1_PLAN_SHA256=bba710b9790fed4c0136b9a0f33186f87f11be9e5da3f76e08784bfbce8dd871
V2_PLAN_SHA256=57146935b5aef4a6d737cc3d709d1f8967052616bd730ec10dea367b2f94d0c5
```

The clean release-built V1 operator was run with `--plan` only. It returned:

```json
{"error":"CONFLICT: workflow 012e72de-4851-454e-b5f2-05b0db15707d terminal state is not exact","outcome":"CONFLICT","writes":0}
```

Exact-ID read-only inspection showed why: one V1 member advanced after the V1
snapshot. The remaining exact plan is mechanically ready:

```text
V2_PLAN_ROWS=17
V2_EXACT_PREIMAGES=17
V2_TARGET_VISIT_COLLISIONS=0
TARGET_PRINCIPAL_ID=8902db0d-429a-4e37-985c-f8b92d4b78fb
TARGET_PRINCIPAL_TYPE=HUMAN
TARGET_PRINCIPAL_ENABLED=true
```

The newly excluded Workflow persisted this transition after V1's observation:

```text
WORKFLOW=0dbf2597-c6f5-4446-aef6-4a5232bc8a1e
TITLE=修车 — 车辆保养/维修
STATE_VERSION=3
EVENT=WORKFLOW_TRANSITION_COMMITTED
EFFECT=ADVANCE
TRANSITION_KEY=advance-to-completed
EVENT_AT=2026-09-15T10:38:02.295786+08:00
CURRENT_NODE_KEY=completed
CURRENT_NODE_TYPE=TERMINAL
CURRENT_ASSIGNEE=NULL
```

The two V1 exclusions remain terminal and unchanged in scope:

```text
2edf5b53-1dd9-4c93-b356-4029d3fe1adb|2|completed|TERMINAL|NULL
f0ebdef1-8cab-4b97-82ac-af92b8ed3e12|2|completed|TERMINAL|NULL
```

Queries used only exact UUIDs from the frozen plan and joined canonical
Instance, current Visit, Context, Definition, node, and Principal rows. The
transaction was committed read-only. No production row was modified.
