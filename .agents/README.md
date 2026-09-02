# Development Grammar V1

```text
GRAMMAR_VERSION = 1.0.1
GOVERNING_AUTHORITY = AGENT_DEVELOPMENT_GOVERNANCE_V1
STATUS = accepted
ENFORCEMENT_LEVEL = manual_policy_plus_deterministic_integrity
```

This is the shared operating grammar for repositories developed across many Agent sessions. Product Direction, Architecture, governing Specs, investigations, implementation records, and runtime evidence remain in their owning repository.

After this file, read `.agents/local/README.md` when present.

## Context budget

For an ordinary task, load only:

1. this grammar;
2. repository-local governance;
3. directly relevant accepted authorities;
4. the Spec-governance router and one selected mode.

Do not preload every protocol, template, rationale, prior review, or example.

## Minimum loop

```text
1. Fix target repository, candidate Head, and integration Base.
2. State GOAL/TARGET and CURRENT_GAP.
3. Locate Product Authority in precedence order.
4. Separate OBSERVATION from WORKING_GUESS when interpretation affects routing.
5. Classify independently: AUTHORITY_ACTION, PLAN_LEVEL, ASSURANCE_LEVEL.
6. Check Execution Mandate, load-bearing SPEC_GAP, Evidence reviewability,
   live authority gap, and candidate/Base movement.
7. Select the shortest authorized route and artifact set.
8. Execute only within accepted Contracts and the mandate.
9. Review the affected surface with executed Evidence.
10. DONE_WHEN met without EXPANSION_TRIGGER -> STOP.
```

## Semantic primitives

Epistemic entities:

- **Observation** — direct recorded result with source, revision, environment, time, method, result, and provenance.
- **Claim** — interpretation with `SUPPORTED`, `INFERRED`, or `OPEN_ASSUMPTION` support state.
- **State** — a time- and coordinate-bound projection from Observations and necessary Claims.

Normative entities:

- **Goal** — desired outcome; it does not authorize mutation.
- **Decision** — selected direction made by an owning authority.
- **Contract** — stable testable obligation created by an accepted Decision.

**Evidence** is a qualified relation from Observation(s) to a named Claim, State assertion, or Contract at exact coordinates.

```text
Test Definition != Observation
Executed Result with coordinates = Observation
Qualified Observation-to-target relation = Evidence
Activity != Knowledge
Activity != Progress
```

Load-bearing Evidence in a governing Spec or Conformance Record uses stable `EVD-*` identity and records source, target, relation, coordinates, sufficiency, limitations, and provenance. A Routine Change Brief does not require a complete research graph unless a load-bearing interpretation depends on it.

## Product Authority and Execution Mandate

Only active accepted Product Direction, Architecture/invariant authority, or governing Spec in the owning repository may create or change long-lived Product Contracts.

These do not create Product Authority:

```text
Investigation
Task or prompt
Change Brief
ExecPlan
Execution Mandate
Controlled Runbook
test definition
runtime state
Review comment
```

A valid **Execution Mandate** authorizes and constrains one task or operation. It binds an attributable issuer, target, scope, allowed/forbidden effects, and `DONE_WHEN`. A controlled mutation additionally binds actor/role, environment, exact operation or operation class, abort conditions, Secret handling, receipt requirements, and validity/attempt bounds. It cannot create or weaken Product Contracts. An Agent-authored Brief cannot self-authorize.

Every mutation—including code, documentation, configuration, schema, behavior-defining tests, generated files, and operational state—MUST have attributable authorization before it begins. The authorization may be carried by a Task, Issue, PR, Brief, or dedicated mandate; it need not become a separate large document, but it MUST remain persistent and reviewable. All write work MUST use an isolated worktree or equivalent isolated write surface bound to an exact parent, isolated ref, and single intended tree without mutating another active checkout.

## Three independent PREFLIGHT axes

### Authority action

```text
REUSE | AMEND | SUPERSEDE | NEW
```

- `REUSE`: active accepted Product Authority already decides the behavior and no Contract meaning changes.
- `AMEND`: a named proposed authority changes inside its declared scope, ownership, and bounded Decision identity; or accepted authority gets strictly additive new IDs under unchanged Goal, scope, ownership, and accepted Decisions.
- `SUPERSEDE`: accepted meaning is deleted, narrowed, expanded, reversed, replaced, or receives different failure semantics. Use a whole-authority successor and atomic backlinks.
- `NEW`: no active accepted authority owns the bounded independent decision, or a targeted proposal's scope, ownership, or bounded Decision identity changes.

`AMEND_OR_NEW_PENDING_OWNERSHIP` is investigation-only and must resolve before any readiness boundary.

An accepted authority permits implementation only when it declares `implementation_authority: contracts` and the request is inside those Contracts. An accepted Program or `implementation_authority: none` does not authorize child implementation.

### Plan level

```text
NONE | BRIEF | EXEC_PLAN
```

- `NONE`: trivial bounded work with machine-supported semantic identity.
- `BRIEF`: bounded non-trivial work.
- `EXEC_PLAN`: multi-phase, cross-component, dependency-sensitive, migratory, or otherwise complex execution.

Risk alone does not create an ExecPlan.

### Assurance level

```text
ROUTINE | DURABLE | CONTROLLED
```

- `ROUTINE`: readily reversible and low consequence.
- `DURABLE`: persisted/public/package/lifecycle/migration surface requiring independent affected-Contract review.
- `CONTROLLED`: identity, auth, permission, Secret, Grant, destructive migration, deletion, production activation, cross-repository public protocol, irreversible operation, or comparable high consequence.

A Controlled Runbook is an Assurance artifact, not a Plan level. A one-shot operation may be `BRIEF + CONTROLLED`.

## Default routes

| Authority | Assurance | Default route |
|---|---|---|
| `REUSE` | `ROUTINE` | Brief as needed + implementation + focused Evidence |
| `REUSE` | `DURABLE` | Brief/ExecPlan + implementation + independent affected-Contract review |
| `REUSE` | `CONTROLLED` | valid mandate + exact runbook + receipt + independent post-state verification; no new Spec solely for risk |
| `AMEND/NEW` | `ROUTINE/DURABLE` | Spec delta and implementation may share one atomic PR when local authority permits |
| `AMEND/NEW` | `CONTROLLED` | docs-first Product Authority, then controlled execution |
| `SUPERSEDE` | any | docs-first whole-authority successor |

### Route stage and docs-first gate

Every structured route records:

```text
ROUTE_STAGE = AUTHORITY_AUTHORING | IMPLEMENTATION | OPERATION
AUTHORITY_ACCEPTED_IN_BASE = YES | NO | NOT_APPLICABLE
```

`AMEND/NEW + CONTROLLED` remains in `AUTHORITY_AUTHORING` with implementation and operation forbidden until the new authority is accepted in the relevant base. Every `SUPERSEDE` route is docs-first and cannot share a same-stage implementation or operation. After the authority is accepted, the actual implementation or operation is a new task routed as `REUSE`.

`AMEND/NEW + ROUTINE/DURABLE` MAY combine Spec delta and implementation only when local authority explicitly permits that atomic route. This exception never applies to `CONTROLLED` or `SUPERSEDE`.

Implementation detail does not become a Contract because an Investigation, PR body, test, or Reviewer repeats it. A public interface, permission, security, durable-data, lifecycle, or compatibility obligation cannot be demoted by calling it an implementation detail.

## One legal effect per artifact

- Standing Spec / Spec delta — Product Authority.
- Change Brief — one change's goal, gap, route, scope, Evidence, and stop boundary.
- ExecPlan — phases, dependencies, checkpoints, rollback, and re-PREFLIGHT triggers.
- Execution Mandate — one operation's authorization and limits.
- Controlled Runbook — dangerous operation steps, aborts, receipts, and verification.
- Receipt / Conformance Record — what happened and what it verifies.
- Investigation — durable non-authoritative findings and alternatives.

## Load-bearing gaps and Evidence

A Reviewer may identify a missing long-lived decision but cannot write it in Review. When implementation, merge, or operation depends on it:

```text
SPEC_GAP_DEPENDENCY = LOAD_BEARING
AUTHORITY_ACTION = AMEND | SUPERSEDE | NEW
at least one applicable readiness boundary = NO
all readiness boundaries = NOT_APPLICABLE  # forbidden
NEXT_ACTION = RE_PREFLIGHT
```

`REUSE` and `AMEND_OR_NEW_PENDING_OWNERSHIP` cannot cross this readiness boundary. `OWNER_DECISION_REQUIRED = YES` may record that the re-PREFLIGHT needs an Owner decision, but `OWNER_DECISION` cannot replace the required `RE_PREFLIGHT` route result.

Required Evidence must be accessible to the designated independent Reviewer, reproducible in an authorized environment, or represented by a sanitized coordinate-bound receipt from a legally independent actor.

```text
inaccessible / unverifiable / unknown provenance -> REQUIRED_GATE_FAILURE
fabricated / materially distorted / falsely claimed execution -> FALSE_EVIDENCE
```

Secret values are not exposed to prove reviewability.

## Live state ahead of authority

```text
LIVE_STATE = Observation, not authority
EXPANSION = FROZEN
AUTO_DELETE = NO
PERMANENT_GRANDFATHER = NO
```

An authorized Owner/risk actor issues temporary scope-bound containment with an expiry or closure condition. Close the long-lived gap docs-first, perform minimum reconcile after acceptance, independently verify conformance, end containment, and stop.

## Emergency containment

A pre-Spec emergency action is limited to rollback, disablement or shutdown, revocation, isolation, or equivalent containment. It MUST have attributable Owner authorization and an incident reference, MUST NOT introduce durable new behavior, and MUST require permanent repair to return through normal PREFLIGHT and Product Authority. Emergency containment may authorize the minimum operation needed to reduce harm; it does not authorize feature implementation, merge, or permanent semantics.

## Review coordinates and Blockers

```text
REVIEW_TARGET_HEAD = exact candidate under review
BASE_HEAD = integration snapshot used for review
CURRENT_BASE_HEAD = current branch tip at impact recheck
```

Unrelated Base movement is not candidate-Head drift. Re-review only when candidate semantics, relevant authority, affected behavior/Evidence, or a real conflict changes.

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

Every Blocker states `SOURCE`, `COUNTEREXAMPLE`, `IMPACT`, and `MINIMAL_CLOSURE`. Legal sources are active accepted Product Authority, accepted local governance/invariant authority, a pre-existing active machine gate, or a valid Execution Mandate. Investigation, proposed tests, task product prose, Reviewer preference, and Review comments are not Product-Contract sources.

Other findings are `SPEC_GAP`, `FOLLOW_UP`, or `TOOLING_DEBT`. Tooling debt blocks a product only when it causes false pass, harms non-test data, hides a concrete security/data-loss failure, or is itself an accepted deliverable.

## Goal and stop controls

Every non-trivial route records:

```text
GOAL_OR_TARGET
CURRENT_GAP
AUTHORITY_ACTION
PRIMARY_AUTHORITY
PLAN_LEVEL
ASSURANCE_LEVEL
EVIDENCE_NEEDED
DONE_WHEN
```

Durable, controlled, or expansion-prone work also records `EXPANSION_TRIGGER`. Use `NEXT_REAL_ACTION` when governance/infrastructure drift is plausible.

```text
DONE_WHEN met + EXPANSION_TRIGGER not fired -> STOP
```

Optional platform work, extra fault research, Agent availability, sunk effort, and non-load-bearing harness imperfection are not expansion triggers.

## Proportional conformance

A Conformance Record binds exact Product Authority revision, implementation revision, environment, evaluation time, implementation state, verification state, `conformance_result`, executed Observations, and Evidence relations.

Standard review covers affected Contracts and directly dependent accepted invariants. Controlled operations, releases, explicit full audits, and unbounded surfaces use the complete applicable matrix. A prior mechanism is not rerun unless the new change invalidates it.

`VERIFIED` applies only to the exact bound tuple and is never a permanent property of a Spec.

## Adoption and history

This source repository is not automatically authoritative in a consumer. A consumer vendors an exact commit, reviews it locally, records preparation separately from acceptance, and activates it only after merge into its authority branch.

Do not bulk-rewrite history. Apply V1 from the next applicable change forward.

`.agents/protocol/SPEC_GOVERNANCE_V0.md` is historical compatibility material. The active workflow is `.agents/protocol/SPEC_GOVERNANCE_V1.md`.
