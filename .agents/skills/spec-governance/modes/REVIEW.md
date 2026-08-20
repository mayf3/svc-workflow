# REVIEW mode

## Goal

Independently decide whether an exact Spec revision is safe to recommend for acceptance.

## Fix coordinates first

```text
REPOSITORY
REVIEWED_BASE_COMMIT
REVIEWED_SPEC_COMMIT
SPEC_PATH
REVIEWER_ID
AUTHOR_ID
```

The reviewer must not rely on unrecorded chat history.

## Review passes

### Authority

Check parent precedence, repository ownership, exact external refs, no silent override, no overlapping conflicting accepted authority, no partial supersession, complete whole-Spec backlinks, and an authorized acceptance actor.

### Primitive types

Check State coordinates and basis; Observation versus interpretation; Claim support state; explicit `EVD-*` source, target, polarity, coordinates, sufficiency, and limits; explicit Decisions; and qualified Evidence rather than filenames or test definitions.

### Scope and decisions

Check that no material product choice remains for implementation, non-goals are explicit, Program versus implementation authority is clear, and owner decisions are resolved.

### Contracts

Check relevant identity, authorization, negative paths, trust boundary, transaction, lifecycle, timeout, retry, idempotency, unknown outcome, migration, compatibility, rollback, audit, operations, and security semantics.

### Acceptance

Check every active Contract has coverage, the verification can produce executed evidence at a named environment, and the failure condition rejects a wrong or bypassed implementation.

### Immutability

Check accepted ancestors. Any change to existing normative meaning requires a new Spec and whole-Spec supersession. A strictly additive amendment is allowed only with new stable IDs inside unchanged Goal, scope, authority, and accepted Decisions. Reused IDs with changed meaning and prose-only partial supersession fail review.

## Recommendation output

```text
SPEC_GOVERNANCE_MODE = REVIEW
SPEC_REVIEW = ACCEPT | REVISE
READY_TO_MARK_ACCEPTED = YES | NO
REVIEWED_BASE_COMMIT = <sha>
REVIEWED_SPEC_COMMIT = <sha>
REVIEWER_ID = <identity>
AUTHOR_INDEPENDENCE = PASS | FAIL
AUTHORITY_REVIEW = PASS | FAIL
PRIMITIVE_BOUNDARY_REVIEW = PASS | FAIL
CONTRACT_REVIEW = PASS | FAIL
ACCEPTANCE_COVERAGE_REVIEW = PASS | FAIL
IMMUTABILITY_REVIEW = PASS | FAIL
BLOCKERS = <n>
ACCEPTANCE_ACTOR_REQUIRED = <identity or role>
```

Review recommendation is not acceptance.

## Final-head binding

After an authorized actor prepares the accepted head, independently compare it with the reviewed Spec commit:

```text
FINAL_ACCEPTED_HEAD = <sha>
SEMANTIC_DELTA_AFTER_REVIEW = NONE | DETECTED
FINAL_HEAD_RECHECK = PASS | FAIL
```

Any semantic delta requires a new review.
