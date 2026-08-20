# Spec Review Record

## Review coordinates

```text
REPOSITORY = owner/repository
SPEC_ID = EXAMPLE_BEHAVIOR_V1
SPEC_PATH = docs/specs/EXAMPLE_BEHAVIOR_V1.md
REVIEWED_BASE_COMMIT = <sha>
REVIEWED_SPEC_COMMIT = <sha>
REVIEWER_ID = <identity>
AUTHOR_ID = <identity>
REVIEWED_AT = <timestamp>
```

## Review result

```text
SPEC_REVIEW = ACCEPT | REVISE
READY_TO_MARK_ACCEPTED = YES | NO
AUTHOR_INDEPENDENCE = PASS | FAIL
AUTHORITY_REVIEW = PASS | FAIL
PRIMITIVE_BOUNDARY_REVIEW = PASS | FAIL
CONTRACT_REVIEW = PASS | FAIL
ACCEPTANCE_COVERAGE_REVIEW = PASS | FAIL
IMMUTABILITY_REVIEW = PASS | FAIL
BLOCKERS = <n>
ACCEPTANCE_ACTOR_REQUIRED = <identity or role>
```

## Findings

### BLOCKER 1 — <title>

- Affected authority / primitive / Contract: ...
- Finding: ...
- Why it matters: ...
- Required change: ...
- Evidence/provenance: ...

## Accepted direction that must not be redesigned

- ...

## Final-head binding

Complete after an authorized actor prepares the accepted head.

```text
FINAL_ACCEPTED_HEAD = <sha>
ACCEPTANCE_ACTOR = <identity>
ACCEPTED_AT = <timestamp>
SEMANTIC_DELTA_AFTER_REVIEW = NONE | DETECTED
FINAL_HEAD_RECHECK = PASS | FAIL
```

Any semantic delta requires a new independent review.
