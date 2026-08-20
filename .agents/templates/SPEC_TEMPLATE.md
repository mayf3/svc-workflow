---
spec_id: EXAMPLE_BEHAVIOR_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
scope:
  - example-service
governed_by:
  - EXAMPLE_PRODUCT_DIRECTION_V1
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - repository-maintainers
---

# EXAMPLE_BEHAVIOR_V1

## 1. Goal

State the desired outcome and the problem being solved. A Goal is not yet a Contract.

```text
GOAL = ...
SUCCESS_OUTCOME = ...
```

## 2. Scope and non-goals

### In scope

- ...

### Out of scope

- ...

## 3. Authority and dependencies

```text
PRIMARY_PARENT_AUTHORITY = EXAMPLE_PRODUCT_DIRECTION_V1
IMPLEMENTATION_AUTHORITY = contracts
EXTERNAL_AUTHORITIES = NONE
AUTHORITY_CONFLICT = NONE
```

Explain how this Spec refines its parent without overriding it. For each external authority, record owning repository, stable ID, exact revision, and relationship.

## 4. Current State

Each load-bearing State statement is time-indexed and cites `OBS-*`, `CLM-*`, or direct provenance.

### STATE-001 — <bounded subject>

- Subject: ...
- As of commit/artifact: ...
- Environment: ...
- Observed at: ...
- Projection: ...
- Basis: `OBS-001`, `CLM-001`

## 5. Observations

### OBS-001 — <direct recorded result>

- Subject: ...
- Repository/source: ...
- Commit/artifact: ...
- Environment: ...
- Observed at: ...
- Method: ...
- Result: ...
- Provenance: ...

## 6. Claims and assumptions

### CLM-001 — <interpretation>

- Support state: SUPPORTED | INFERRED | OPEN_ASSUMPTION
- Supported by evidence: `EVD-001`
- Contradicted by evidence: none known
- Uncertainty: ...

## 7. Evidence relations

### EVD-001 — <why named Observations count for or against a target>

- Source observations: `OBS-001`
- Target: `CLM-001` | `STATE-001`
- Relation: SUPPORTS | CONTRADICTS
- Bound coordinates: ...
- Strength/sufficiency: ...
- Limitations: ...
- Provenance: ...

## 8. Decisions

### DEC-001 — <selected direction>

- Decision owner: ...
- Decision: ...
- Rejected alternatives: `ALT-001`
- Reason: ...
- Owner decision remaining: NONE

## 9. Contracts

### CTR-EXAMPLE-001 — <testable obligation>

The system MUST ...

The system MUST NOT ...

Relevant caller, subject, conditions, failure behavior, lifecycle, transaction, retry, migration, rollback, observability, and security semantics are: ...

## 10. Acceptance

### ACC-EXAMPLE-001 — <verification that can reject a wrong implementation>

- Contracts: `CTR-EXAMPLE-001`
- Method: ...
- Environment: ...
- Inputs/configuration: ...
- Required evidence: executed result bound to implementation commit and environment
- Expected result: ...
- Failure condition: ...

### Contract coverage

| Contract | Acceptance | Evidence class | Covered |
|---|---|---|---|
| `CTR-EXAMPLE-001` | `ACC-EXAMPLE-001` | executed test/runtime/manual | YES |

## 11. Alternatives and disposition

### ALT-001 — <alternative>

- Disposition: rejected
- Reason: ...
- Evidence/Claims considered: ...
- What would reopen: ...

## 12. Migration, compatibility, and rollback

```text
MIGRATION = ...
COMPATIBILITY = ...
ROLLBACK = ...
EMERGENCY_CONTAINMENT = ...
```

## 13. Open questions

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
READY_TO_MARK_ACCEPTED = NO
```
