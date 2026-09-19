# Conformance Record

## Bound tuple

```text
REPOSITORY =
PRIMARY_AUTHORITY =
AUTHORITY_REVISION =
IMPLEMENTATION_BASE_COMMIT =
IMPLEMENTATION_COMMIT =
ENVIRONMENT =
EVALUATED_AT =
EVALUATOR_ID =
ASSURANCE_LEVEL = ROUTINE | DURABLE | CONTROLLED
REVIEW_SCOPE = AFFECTED | FULL
EXECUTION_MANDATE = <reference | NOT_APPLICABLE>
RECEIPT = <reference | NOT_APPLICABLE>
```

## Affected-surface justification

- affected Contracts:
- directly dependent accepted invariants:
- omitted Contracts and why unaffected:
- reason full matrix is or is not required:

## Contract matrix

| Contract | Implementation / operation | Mechanism | Observation IDs | Evidence IDs | Result |
|---|---|---|---|---|---|
| `CTR-EXAMPLE-001` | `path:symbol` or operation | test/probe/query/manual | `OBS-...` | `EVD-...` | VERIFIED / DRIFTED / UNKNOWN / NOT_APPLICABLE |

Every `NOT_APPLICABLE` result requires a Contract-derived reason.

## Aggregate

```text
IMPLEMENTATION_STATE = NOT_STARTED | IN_PROGRESS | COMPLETE
VERIFICATION_STATE = NOT_RUN | PARTIAL | SUFFICIENT
CONFORMANCE_RESULT = UNKNOWN | VERIFIED | DRIFTED
CONTRACTS_TOTAL =
CONTRACTS_VERIFIED =
CONTRACTS_DRIFTED =
CONTRACTS_UNKNOWN =
CONTRACTS_NOT_APPLICABLE =
DONE_WHEN_MET = YES | NO
EXPANSION_TRIGGERED = YES | NO
IMPLEMENTATION_READY_TO_MERGE = YES | NO
OPERATION_COMPLETE = YES | NO | NOT_APPLICABLE
NEXT_ACTION = CONTINUE | STOP | RE_PREFLIGHT | OWNER_DECISION
```

Any drift produces aggregate `DRIFTED`; all applicable Contracts verified with sufficient Evidence produces `VERIFIED`; otherwise `UNKNOWN`.

`VERIFIED` applies only to the exact authority/implementation/environment/time tuple. A changed coordinate does not inherit it.

Each load-bearing result names `EVD-*` relations with source Observations, target Contract, relation, exact coordinates, sufficiency, limitations, and provenance.
