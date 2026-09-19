# AUTHOR mode

## Goal

Create or revise only the Product Authority required by a resolved `AMEND`, `SUPERSEDE`, or `NEW` route. Do not turn complex or dangerous `REUSE` work into a Spec.

## Entry conditions

```text
AUTHORITY_ACTION = AMEND | SUPERSEDE | NEW
AUTHORITY_OWNERSHIP = resolved
PRIMARY_AUTHORITY / TARGET_PROPOSAL = resolved
PLAN_LEVEL = classified
ASSURANCE_LEVEL = classified
```

`AMEND_OR_NEW_PENDING_OWNERSHIP` is not authoring-ready.

## Procedure

1. Freeze owning repository, exact parents/external references, action, long-lived scope/non-goals, implementation authority, and docs-first requirement.
2. Treat Task, Brief, ExecPlan, Investigation, test, runtime state, and Review finding as input, not Product Authority.
3. Create stable `OBS/CLM/STATE/EVD` only when a load-bearing interpretation or acceptance decision depends on it; Routine execution detail stays in the Brief/ExecPlan.
4. Resolve identity, permission, lifecycle, failure, migration, compatibility, security, public behavior, scope, ownership, and bounded Decision identity before implementation relies on them.
5. Write stable Contracts across relevant success, negative, identity, authorization, trust, transaction, lifecycle, deletion, retry/timeout, unknown-outcome, migration, compatibility, rollback, audit, operations, and security paths.
6. Map every Contract to Acceptance with `Contracts`, `Method`, `Environment`, `Required evidence`, `Expected result`, and `Failure condition`. Include negative examples that reject a wrong classifier/implementation.
7. Select route:

```text
AMEND/NEW + ROUTINE/DURABLE -> combined Spec delta + code MAY be used if local authority permits
AMEND/NEW + CONTROLLED      -> docs-first
SUPERSEDE                   -> docs-first
```

8. Run authority, immutability, lifecycle, syntax, scope, and reverse Acceptance coverage passes.

## Output

```text
SPEC_GOVERNANCE_MODE = AUTHOR
AUTHORITY_ACTION = AMEND | SUPERSEDE | NEW
SPEC_ID = <ID>
STATUS = proposed
IMPLEMENTATION_AUTHORITY = none | contracts
PRIMARY_PARENT_AUTHORITY = <ID | NONE>
EXTERNAL_AUTHORITIES = <qualified refs | NONE>
PLAN_LEVEL = NONE | BRIEF | EXEC_PLAN
ASSURANCE_LEVEL = ROUTINE | DURABLE | CONTROLLED
DOCS_FIRST_REQUIRED = YES | NO
OPEN_OWNER_DECISIONS = NONE | <items>
NORMATIVE_TBD = NONE | <items>
PARTIAL_SUPERSESSION = NONE | DETECTED
CONTRACT_COUNT = <n>
CONTRACTS_WITH_ACCEPTANCE = <n>
AUTHORING_READY_FOR_REVIEW = YES | NO
NEXT_ACTION = REVIEW | RE_PREFLIGHT | OWNER_DECISION
```
