# PREFLIGHT mode

## Goal

Select the shortest authorized route by classifying Product Authority, execution complexity, failure consequence, and readiness independently.

## Procedure

1. Bind target repository, `REVIEW_TARGET_HEAD`, `BASE_HEAD`, `CURRENT_BASE_HEAD`, Goal/target, and Current Gap.
2. Read local precedence, exact adoption lock, Product Direction, Architecture/invariants, overlapping accepted Specs, named proposed target, and exact external authorities.
3. Record qualified Observations; separate a Working Guess when interpretation changes routing.
4. Choose one Authority action:

```text
REUSE | AMEND | SUPERSEDE | NEW
```

Named proposal:

```text
scope/ownership/bounded Decision identity unchanged -> AMEND
any changed -> NEW
```

Accepted authority:

```text
already decides behavior, no meaning change -> REUSE
strictly additive new IDs under unchanged accepted Decisions -> AMEND
accepted meaning changes -> SUPERSEDE
no owner for independent decision -> NEW
```

5. Choose Plan from complexity: `NONE | BRIEF | EXEC_PLAN`.
6. Choose Assurance from consequence: `ROUTINE | DURABLE | CONTROLLED`.
7. Check route stage, authority accepted in base, implementation authority, attributable mutation authorization, target/scope/effects/Done When, isolated write surface, Controlled Runbook, load-bearing gap, Evidence reviewability, live authority gap, emergency containment, Base impact, Done When, and Expansion Trigger.
8. Use the route table in `.agents/README.md`.

## Output

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
TARGET_REPOSITORY = <owner/repository>
REVIEW_TARGET_HEAD = <sha | NOT_APPLICABLE>
BASE_HEAD = <sha | NOT_APPLICABLE>
CURRENT_BASE_HEAD = <sha | NOT_APPLICABLE>
ROUTE_STAGE = AUTHORITY_AUTHORING | IMPLEMENTATION | OPERATION
AUTHORITY_ACCEPTED_IN_BASE = YES | NO | NOT_APPLICABLE
GOAL_OR_TARGET = <outcome>
CURRENT_GAP = <gap>
OBSERVATIONS = <qualified items>
WORKING_GUESS = <item | NOT_APPLICABLE>
AUTHORITY_ACTION = REUSE | AMEND | SUPERSEDE | NEW |
                   AMEND_OR_NEW_PENDING_OWNERSHIP
PRIMARY_AUTHORITY = <ID@revision | NONE>
RELATED_AUTHORITIES = <IDs@revisions | NONE>
IMPLEMENTATION_AUTHORITY = contracts | none | unknown | not_applicable
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = YES | NO
PLAN_LEVEL = NONE | BRIEF | EXEC_PLAN
ASSURANCE_LEVEL = ROUTINE | DURABLE | CONTROLLED
EXECUTION_MANDATE = VALID | INVALID | NOT_APPLICABLE
MUTATION_AUTHORIZATION = VALID | INVALID | NOT_APPLICABLE
ISOLATED_WRITE_SURFACE = YES | NO | NOT_APPLICABLE
CONTROLLED_RUNBOOK_REQUIRED = YES | NO
SPEC_GAP_DEPENDENCY = NONE | NON_LOAD_BEARING | LOAD_BEARING
EVIDENCE_REVIEWABILITY = PASS | FAIL | NOT_APPLICABLE
LIVE_AUTHORITY_GAP = NONE | DETECTED
OWNER_DECISION_REQUIRED = YES | NO
EMERGENCY_STATE = NONE | ACTIVE
EMERGENCY_ACTION = NONE | ROLLBACK | DISABLEMENT | SHUTDOWN |
                   REVOCATION | ISOLATION | CONTAINMENT
INCIDENT_REFERENCE = <reference | NOT_APPLICABLE>
BASE_IMPACT = NONE | BOUNDED | RELEVANT
IMPLEMENTATION_ALLOWED = YES | NO | NOT_APPLICABLE
MERGE_READY = YES | NO | NOT_APPLICABLE
OPERATION_ALLOWED = YES | NO | NOT_APPLICABLE
EVIDENCE_NEEDED = <items>
DONE_WHEN = <observable result>
EXPANSION_TRIGGER = <condition | NOT_APPLICABLE>
NEXT_REAL_ACTION = <product-facing action | NOT_APPLICABLE>
NEXT_ACTION = CONTINUE | STOP | RE_PREFLIGHT | OWNER_DECISION
```

Do not start semantic implementation or mutation when the relevant readiness flag is `NO`.

Hard routing effects:

```text
AMEND/NEW + CONTROLLED before authority acceptance -> docs-first; no implementation/operation
SUPERSEDE -> docs-first whole-authority successor; no same-stage implementation/operation
accepted authority, later work -> new REUSE task

any allowed mutation -> attributable authorization + isolated write surface
controlled allowed mutation -> target + actor/environment + exact operation
                               + abort/Secret/receipt/attempt bounds + runbook

LOAD_BEARING SPEC_GAP -> no REUSE; no unresolved pending action;
                         at least one applicable readiness = NO;
                         NEXT_ACTION = RE_PREFLIGHT

emergency -> Owner + incident + containment-only + no durable new behavior
             + later normal authority reconciliation
```
