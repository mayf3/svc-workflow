---
name: spec-governance
description: Route goal-driven work through independent Authority, Plan, and Assurance decisions without conflating Product Authority, one-operation mandates, implementation, runtime state, or Evidence.
---

# Spec Governance Skill V1

Use exactly one primary mode per invocation:

```text
PREFLIGHT   classify Authority, Plan, Assurance, readiness, and next action
AUTHOR      create or revise Product Authority when the route requires it
REVIEW      independently review an exact authority/change/final Head
COMPLIANCE  evaluate exact implementation or operation against exact Contracts
```

Change Briefs, ExecPlans, Execution Mandates, and Controlled Runbooks are artifacts, not modes.

## Read order

1. `AGENTS.md`;
2. `.agents/README.md`;
3. `.agents/local/README.md`;
4. directly relevant accepted authorities;
5. only the selected mode file.

Read the V1 protocol and Spec format only as needed.

## Shared invariants

```text
Product Authority creates long-lived obligations.
Execution Mandate constrains one operation.
Every mutation needs attributable target/scope/effects/Done When authorization and an isolated write surface.
Complexity determines Plan.
Failure consequence determines Assurance.
Investigation / Task / test / Review comment do not create Product Authority.
AMEND/NEW + CONTROLLED and every SUPERSEDE route are docs-first.
Load-bearing SPEC_GAP -> AMEND/SUPERSEDE/NEW + readiness NO + RE_PREFLIGHT.
Inaccessible required Evidence -> REQUIRED_GATE_FAILURE, not automatically FALSE_EVIDENCE.
Runtime is Observation, not authority.
Emergency pre-Spec action is Owner-authorized incident containment only; no durable new behavior.
DONE_WHEN met without EXPANSION_TRIGGER -> STOP.
```

For every mode, bind exact coordinates, preserve stable accepted IDs and whole-authority backlinks, separate Observation from Working Guess when it affects routing, distinguish deterministic checks from semantic judgment, and persist load-bearing results outside chat.

Choose `PREFLIGHT` before non-trivial code/operation unless a persisted record covers the exact request and coordinates. Choose `AUTHOR` only for `AMEND`, `SUPERSEDE`, or `NEW`; `REUSE` normally uses a Brief/ExecPlan. Choose `REVIEW` for independent exact-Head/affected-Contract/final-Head review. Choose `COMPLIANCE` after implementation or operation exists.

One Agent may perform Recorder, PREFLIGHT, and Planner cognitive functions. Independence is required only where Durable/Controlled assurance, authority, or local policy requires it; do not create a fixed Agent formation.
