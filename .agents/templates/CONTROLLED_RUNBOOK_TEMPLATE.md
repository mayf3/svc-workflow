# Controlled Runbook

A Controlled Runbook is an Assurance artifact. It may be embedded in a Brief and does not itself authorize execution or create an ExecPlan.

```text
RUNBOOK_ID =
EXECUTION_MANDATE =
ACTOR_OR_ROLE =
TARGET =
ENVIRONMENT =
ISOLATED_WRITE_SURFACE =
EMERGENCY_STATE = NONE | ACTIVE
EMERGENCY_ACTION = NONE | ROLLBACK | DISABLEMENT | SHUTDOWN |
                   REVOCATION | ISOLATION | CONTAINMENT
INCIDENT_REFERENCE = <reference | NOT_APPLICABLE>
DURABLE_NEW_BEHAVIOR = NO
POST_INCIDENT_AUTHORITY_RECONCILIATION_REQUIRED = YES | NO | NOT_APPLICABLE
EXPECTED_PRE_STATE =
ALLOWED_EFFECTS =
FORBIDDEN_EFFECTS =
MAX_ATTEMPTS =
ABORT_CONDITIONS =
UNKNOWN_OUTCOME_PROBE =
RETRY_RULE =
COMPENSATION_OR_CONTAINMENT =
DONE_WHEN =
NEGATIVE_POSTCONDITIONS =
```

## Preconditions

- exact authority/implementation coordinates:
- mandate validity:
- access/identity:
- isolated worktree or equivalent isolated write surface:
- emergency Owner authorization and incident reference when active:
- backup/rollback or containment:
- Secret-safe logging:
- independent verifier availability:

## Exact operation

1. ...
2. ...

## Receipt

```text
MANDATE_ID
RUNBOOK_ID
ACTOR
ENVIRONMENT
STARTED_AT / ENDED_AT
ATTEMPT
PRE_STATE_REFERENCE
OPERATION_RESULT
POST_STATE_REFERENCE
ABORT / RETRY / COMPENSATION
SECRET_DISCLOSURE = NO
```

## Independent verification

State verifier, exact coordinates, checks, Evidence relation, and result. When Done When is met without an Expansion Trigger, stop.
