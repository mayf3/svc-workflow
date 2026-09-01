# COMPLIANCE mode

## Goal

Evaluate exact implementation or operation against exact accepted Contracts in an exact environment, using proportional scope and qualified Evidence.

## Required coordinates

```text
REPOSITORY
PRIMARY_AUTHORITY
AUTHORITY_REVISION
IMPLEMENTATION_BASE_COMMIT
IMPLEMENTATION_COMMIT
ENVIRONMENT
EVALUATED_AT
ASSURANCE_LEVEL
EVALUATOR_ID
```

For `REUSE`, confirm the accepted implementation-authorizing Contract was present in the implementation Base. For controlled work, bind mandate, runbook, receipt, and post-state.

## Scope

Default:

```text
affected Contracts + directly dependent accepted invariants
```

Use the complete applicable matrix for controlled operations, releases, explicit full audits, or unbounded affected surfaces. Do not rerun prior passing mechanisms unless the change invalidates them.

## Procedure

1. enumerate applicable Contracts and reasons for affected/not-applicable status;
2. map implementation/config/schema/operation and negative/bypass paths;
3. map verification mechanisms;
4. execute checks and record `OBS-*` at bound coordinates;
5. create `EVD-*` relations to Contracts using `SATISFIES`, `VIOLATES`, or `INCONCLUSIVE`;
6. assign Contract results;
7. derive independent implementation, verification, and conformance dimensions;
8. compare post-state with mandate and stop boundary;
9. persist matrix and environment-spanning evidence.

Contract results:

```text
VERIFIED | DRIFTED | UNKNOWN | NOT_APPLICABLE
```

Aggregate:

```text
IMPLEMENTATION_STATE = NOT_STARTED | IN_PROGRESS | COMPLETE
VERIFICATION_STATE = NOT_RUN | PARTIAL | SUFFICIENT
CONFORMANCE_RESULT = UNKNOWN | VERIFIED | DRIFTED
```

Changed authority/implementation/environment/time coordinates do not inherit old `VERIFIED`.

## Output

```text
SPEC_GOVERNANCE_MODE = COMPLIANCE
PRIMARY_AUTHORITY = <ID>
AUTHORITY_REVISION = <sha/blob>
IMPLEMENTATION_COMMIT = <sha>
ENVIRONMENT = <name>
EVALUATED_AT = <timestamp>
ASSURANCE_LEVEL = ROUTINE | DURABLE | CONTROLLED
REVIEW_SCOPE = AFFECTED | FULL
EXECUTION_MANDATE = VALID | INVALID | NOT_APPLICABLE
RECEIPT = <reference | NOT_APPLICABLE>
IMPLEMENTATION_STATE = NOT_STARTED | IN_PROGRESS | COMPLETE
VERIFICATION_STATE = NOT_RUN | PARTIAL | SUFFICIENT
CONFORMANCE_RESULT = UNKNOWN | VERIFIED | DRIFTED
CONTRACTS_TOTAL = <n>
CONTRACTS_VERIFIED = <n>
CONTRACTS_DRIFTED = <n>
CONTRACTS_UNKNOWN = <n>
CONTRACTS_NOT_APPLICABLE = <n>
DONE_WHEN_MET = YES | NO
EXPANSION_TRIGGERED = YES | NO
IMPLEMENTATION_READY_TO_MERGE = YES | NO
OPERATION_COMPLETE = YES | NO | NOT_APPLICABLE
NEXT_ACTION = CONTINUE | STOP | RE_PREFLIGHT | OWNER_DECISION
```
