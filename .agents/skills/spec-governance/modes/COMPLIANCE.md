# COMPLIANCE mode

## Goal

Evaluate an exact implementation against exact accepted Contracts in an exact environment.

## Required coordinates

```text
REPOSITORY
IMPLEMENTATION_BASE_COMMIT
IMPLEMENTATION_COMMIT
PRIMARY_GOVERNING_SPEC
GOVERNING_SPEC_COMMIT_OR_BLOB
ENVIRONMENT
EVALUATED_AT
```

Confirm the accepted Spec was present in the implementation base.

## Procedure

### 1. Enumerate Contracts

List every active Contract in the pinned Spec revision, including difficult manual or runtime Contracts.

### 2. Map implementation

For each Contract, identify implementation symbols, configuration, schema, operations, negative/bypass paths, migration, and rollback as applicable.

### 3. Map verification mechanisms

Identify tests, probes, requests, queries, logs, audits, canaries, and manual procedures. A mechanism is not evidence until executed at the bound coordinates.

### 4. Capture Observations and Evidence relations

Record executed results as stable `OBS-*` records with command/request/query, inputs, configuration, implementation and deployment identity, environment, time, result, and bounded provenance.

For each load-bearing Contract result, create an `EVD-*` relation from the relevant Observations to the Contract at the pinned governing-Spec revision with `SATISFIES`, `VIOLATES`, or `INCONCLUSIVE`, plus sufficiency and limitations.

### 5. Assign Contract results

```text
VERIFIED
DRIFTED
UNKNOWN
NOT_APPLICABLE
```

`NOT_APPLICABLE` requires a reason derived from the Contract, not implementation convenience.

### 6. Derive independent aggregate dimensions

```text
IMPLEMENTATION_STATE = NOT_STARTED | IN_PROGRESS | COMPLETE
VERIFICATION_STATE = NOT_RUN | PARTIAL | SUFFICIENT
CONFORMANCE = UNKNOWN | VERIFIED | DRIFTED
```

- any active Contract drift → `DRIFTED`;
- all applicable active Contracts verified with sufficient evidence, with every `NOT_APPLICABLE` result justified from the Contract → `VERIFIED`;
- otherwise → `UNKNOWN`.

### 7. Persist

Put the Contract matrix in the implementation PR. When evidence spans production, canary, migration, restart, external services, or time windows, create and link a repository report.

## Output

```text
SPEC_GOVERNANCE_MODE = COMPLIANCE
PRIMARY_GOVERNING_SPEC = <ID>
GOVERNING_SPEC_REVISION = <sha/blob>
IMPLEMENTATION_COMMIT = <sha>
ENVIRONMENT = <name>
EVALUATED_AT = <timestamp>
IMPLEMENTATION_STATE = NOT_STARTED | IN_PROGRESS | COMPLETE
VERIFICATION_STATE = NOT_RUN | PARTIAL | SUFFICIENT
CONFORMANCE = UNKNOWN | VERIFIED | DRIFTED
CONTRACTS_TOTAL = <n>
CONTRACTS_VERIFIED = <n>
CONTRACTS_DRIFTED = <n>
CONTRACTS_UNKNOWN = <n>
CONTRACTS_NOT_APPLICABLE = <n>
IMPLEMENTATION_READY_TO_MERGE = YES | NO
```

Merge readiness requires complete implementation, sufficient verification, and verified conformance.
