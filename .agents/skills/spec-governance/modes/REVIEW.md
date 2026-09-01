# REVIEW mode

## Goal

Independently evaluate an exact candidate, affected implementation surface, controlled operation, or final accepted Head without creating Product Authority or expanding into open-ended research.

## Coordinates

```text
REPOSITORY
REVIEW_KIND = SPEC | AFFECTED_CONTRACT | CONTROLLED_OPERATION | FINAL_HEAD
REVIEW_TARGET_HEAD
BASE_HEAD
CURRENT_BASE_HEAD
REVIEWER_ID
AUTHOR_ID
ASSURANCE_LEVEL
```

Do not rely on unrecorded chat or inaccessible author-only material.

## Passes

- **Authority/route:** local ownership, exact parents/refs, one unique Authority action, explicit implementation authority, proposal boundaries, no partial supersession, mandate scope.
- **Primitives/Evidence:** Observation versus interpretation, State coordinates, Claim support, load-bearing Evidence relations, reviewability, Secret-safe receipts.
- **Scope/stop:** Goal/Gap, non-goals, allowed/forbidden effects, Done When, Expansion Trigger, optional infrastructure not masquerading as progress.
- **Contracts/Acceptance:** relevant semantic paths; each Acceptance can produce Required Evidence and reject a concrete wrong implementation.
- **Base impact:** unrelated Base movement gets bounded conflict/authority/behavior/Evidence checking; candidate/relevant authority/behavior/conflict changes invalidate affected review.

## Findings

A Blocker uses one class:

```text
CONTRACT_VIOLATION
REPOSITORY_INVARIANT_VIOLATION
CONCRETE_REGRESSION
SECURITY_OR_DATA_LOSS
FALSE_EVIDENCE
SCOPE_ESCALATION
REQUIRED_GATE_FAILURE
```

Every Blocker states `SOURCE`, `COUNTEREXAMPLE`, `IMPACT`, `MINIMAL_CLOSURE`. Legal sources are accepted Product Authority, accepted local governance/invariant authority, a pre-existing active machine gate, or a valid Execution Mandate.

Use `SPEC_GAP`, `FOLLOW_UP`, or `TOOLING_DEBT` for non-Blockers. A load-bearing gap still makes dependent readiness false. Inaccessible required Evidence is `REQUIRED_GATE_FAILURE`; use `FALSE_EVIDENCE` only for fabrication, material distortion, or false execution claim.

## Output

```text
SPEC_GOVERNANCE_MODE = REVIEW
REVIEW_KIND = SPEC | AFFECTED_CONTRACT | CONTROLLED_OPERATION | FINAL_HEAD
SPEC_REVIEW = ACCEPT | REVISE | NOT_APPLICABLE
REVIEW_TARGET_HEAD = <sha>
BASE_HEAD = <sha>
CURRENT_BASE_HEAD = <sha>
REVIEWER_ID = <identity>
AUTHOR_INDEPENDENCE = PASS | FAIL | NOT_REQUIRED
AUTHORITY_REVIEW = PASS | FAIL
PRIMITIVE_BOUNDARY_REVIEW = PASS | FAIL | NOT_APPLICABLE
CONTRACT_REVIEW = PASS | FAIL | NOT_APPLICABLE
ACCEPTANCE_COVERAGE_REVIEW = PASS | FAIL | NOT_APPLICABLE
MANDATE_SCOPE_REVIEW = PASS | FAIL | NOT_APPLICABLE
EVIDENCE_REVIEWABILITY = PASS | FAIL | NOT_APPLICABLE
BASE_IMPACT = NONE | BOUNDED | RELEVANT
BLOCKERS = <n>
SPEC_GAPS = <n>
FOLLOW_UPS = <n>
TOOLING_DEBT = <n>
IMPLEMENTATION_ALLOWED = YES | NO | NOT_APPLICABLE
MERGE_READY = YES | NO | NOT_APPLICABLE
OPERATION_ALLOWED = YES | NO | NOT_APPLICABLE
NEXT_ACTION = CONTINUE | STOP | RE_PREFLIGHT | OWNER_DECISION
```

Review is not acceptance.

Final accepted-Head recheck binds `FINAL_ACCEPTED_HEAD`, `ACCEPTANCE_ACTOR`, `ACCEPTED_AT`, `SEMANTIC_DELTA_AFTER_REVIEW`, and `FINAL_HEAD_RECHECK`. Any normative semantic delta requires a new independent review.
