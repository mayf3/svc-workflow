# Conformance Record

## Evaluation coordinates

```text
REPOSITORY = owner/repository
PRIMARY_GOVERNING_SPEC = EXAMPLE_BEHAVIOR_V1
GOVERNING_SPEC_REVISION = <commit/blob>
IMPLEMENTATION_BASE_COMMIT = <sha>
IMPLEMENTATION_COMMIT = <sha>
ENVIRONMENT = test | staging | production | <qualified environment>
EVALUATED_AT = <timestamp>
EVALUATOR_ID = <identity>
SPEC_PRESENT_IN_BASE = YES | NO
SPEC_STATUS_IN_BASE = accepted | proposed | superseded
```

## Contract matrix

| Contract | Implementation | Verification mechanism | Observation IDs | Evidence IDs | Result |
|---|---|---|---|---|---|
| `CTR-EXAMPLE-001` | `path:symbol` | test/probe/query/manual | `OBS-...` | `EVD-...` | VERIFIED / DRIFTED / UNKNOWN / NOT_APPLICABLE |

## Aggregate result

```text
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

Aggregation:

- any active Contract drifted → `DRIFTED`;
- all applicable active Contracts verified with sufficient evidence and every `NOT_APPLICABLE` result is Contract-justified → `VERIFIED`;
- otherwise → `UNKNOWN`.

## Evidence records

Each load-bearing result identifies `EVD-*` relations with source `OBS-*`, target Contract at the recorded governing-Spec revision, relation (`SATISFIES`, `VIOLATES`, or `INCONCLUSIVE`), bound coordinates, sufficiency, limitations, and provenance.

## Evidence qualifications and limits

- Spec revision limit: ...
- Implementation revision limit: ...
- Environment/configuration limit: ...
- Time/data limit: ...
- Unverified paths: ...

## Drift disposition

When drift exists:

- affected Contract IDs: ...
- containment: ...
- implementation repair: ...
- authority change required: YES | NO
- linked issue/PR/report: ...
