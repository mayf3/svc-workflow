# ExecPlan

An ExecPlan manages execution complexity. It is not Product Authority and does not authorize mutation.

```text
TASK_ID =
TARGET_REPOSITORY =
BASE_HEAD =
PRIMARY_AUTHORITY =
AUTHORITY_ACTION =
PLAN_LEVEL = EXEC_PLAN
ASSURANCE_LEVEL =
EXECUTION_MANDATE = <reference | NOT_APPLICABLE>
GOAL_OR_TARGET =
CURRENT_GAP =
NON_GOALS =
DONE_WHEN =
EXPANSION_TRIGGER =
NEXT_REAL_ACTION =
```

## Dependencies and affected surface

- components/repositories:
- affected Contracts/invariants:
- exact external authorities:
- ordering constraints:
- parallel-safe work:
- sequential work:

## Phases

### Phase 1 — <name>

- preconditions:
- actions:
- expected observations:
- checkpoint:
- rollback/compensation:
- re-PREFLIGHT trigger:

Repeat only as needed.

## Validation

| Contract / boundary | Mechanism | Environment | Required Evidence | Failure result |
|---|---|---|---|---|

## Completion

```text
DONE_WHEN_MET = YES | NO
EXPANSION_TRIGGERED = YES | NO
NEXT_ACTION = CONTINUE | STOP | RE_PREFLIGHT | OWNER_DECISION
```
