---
name: spec-governance
description: Route spec-first work to PREFLIGHT, AUTHOR, REVIEW, or COMPLIANCE without conflating authority, implementation, runtime state, or evidence.
---

# Spec Governance Skill V0

Use exactly one primary mode per invocation:

```text
PREFLIGHT   decide whether implementation may start
AUTHOR      investigate and draft a docs-only governing Spec
REVIEW      independently review an exact Spec revision
COMPLIANCE  evaluate an exact implementation against exact Contracts
```

## Read order

Always read:

1. `AGENTS.md`;
2. `.agents/README.md`;
3. `.agents/local/README.md`;
4. relevant Product Direction, Architecture, and accepted Specs;
5. only the selected mode file under `modes/`.

Read `.agents/protocol/SPEC_GOVERNANCE_V0.md` and `SPEC_FORMAT_V0.md` as reference when the selected mode needs the detailed rule. They are not mandatory full-context preload for every task.

## Mode files

```text
modes/PREFLIGHT.md
modes/AUTHOR.md
modes/REVIEW.md
modes/COMPLIANCE.md
```

## Shared invariants

```text
State is a time-indexed projection, not raw truth.
Observation is not Claim.
Claim is not Decision.
Decision is not Contract.
Accepted Spec is not implemented state.
Implementation progress is not conformance.
Evidence is a first-class relation, not raw provenance material.
Test definition is not Observation or Evidence.
External dependency is not locally governed authority.
```

For every mode:

- record exact repository and commit coordinates;
- preserve stable IDs;
- treat authority conflict as a blocker;
- never infer partial supersession from prose;
- never reuse an accepted Contract ID for changed meaning;
- persist important results outside chat;
- distinguish manual policy from actual deterministic enforcement.

## Mode selection

Choose `PREFLIGHT` before code unless an already persisted preflight record covers the exact base and request.

Choose `AUTHOR` when preflight returns `AMEND`, `SUPERSEDE`, or `NEW` and a docs-only authority change is required.

Choose `REVIEW` only as an independent semantic reviewer of a fixed Spec commit.

Choose `COMPLIANCE` after implementation exists and the governing Spec revision is pinned.

## Emergency exception

Rollback, disablement, shutdown, credential revocation, or isolation may use the emergency seam defined in the protocol. Durable new behavior may not.
