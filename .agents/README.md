# Development Grammar V0

```text
GRAMMAR_VERSION = 0.1.0-draft.1
STATUS = accepted
ENFORCEMENT_LEVEL = manual_policy
```

This document defines the stable semantic grammar used by Agents and maintainers when investigating, specifying, implementing, reviewing, and operating software.

It is intentionally smaller than a project knowledge base. Repository-specific Product Direction, Architecture, Specs, investigations, implementation records, and runtime evidence remain in their owning repository.

After reading this file, read `.agents/local/README.md` when it exists.

### Operating context budget

For an ordinary task, load only:

1. this grammar;
2. the repository-local governance file;
3. the directly relevant Product Direction, Architecture, and governing Specs;
4. the Spec-governance Skill router and the single selected mode file.

The detailed protocol and format documents are reference material. Read only the sections needed to resolve the current mode; do not preload every rationale, template, example, or protocol section into every Agent turn.

---

## 1. Minimum operating loop

```text
1. Locate the governing authorities.
2. Classify the requested work: REUSE / AMEND / SUPERSEDE / NEW.
3. Decide whether it is mechanical; uncertainty means NON_MECHANICAL.
4. For non-mechanical implementation, verify that an accepted,
   implementation-authorizing Spec exists in the implementation PR base.
5. If not, stop implementation and create or revise a docs-only Spec.
6. Independently review the exact Spec commit.
7. An authorized actor accepts the exact final head.
8. Implement against the pinned accepted Spec revision.
9. Record Contract-by-Contract implementation and qualified evidence.
10. Report drift; do not edit accepted authority to excuse the implementation.
```

---

## 2. The semantic primitives

The grammar uses six **entity primitives** and one first-class **relational primitive**. Relation primitives are still primitives: they are minimal semantic building blocks, but they connect entities rather than naming another kind of thing.

### 2.1 Epistemic entity primitives: describing what we know

#### Observation

A direct, recorded encounter with source, runtime, data, behavior, or an executed verification result.

An Observation must state enough coordinates to be falsifiable and reproducible where feasible:

```text
subject
repository / source
commit or artifact identity
runtime environment
observed_at
method / command / request
result
provenance location
```

An Observation does not contain the explanation of why the result occurred.

#### Claim

An interpretation, generalization, causal explanation, or prediction supported or challenged by Observations through Evidence relations.

Every load-bearing Claim has a support state:

```text
SUPPORTED
INFERRED
OPEN_ASSUMPTION
```

Do not use `VERIFIED CLAIM`. A Claim remains an interpretation even when strongly supported.

#### State

A time-indexed projection of a clearly identified subject, built from Observations and necessary Claims.

```text
State = projection(subject, commit/environment/time, observations, claims)
```

State is not raw truth and has no independent evidence authority. Every load-bearing State statement cites provenance, an Observation ID, a Claim ID, or an Evidence ID.

“Current system state” is invalid unless it distinguishes such coordinates as:

- source tree or branch;
- deployed revision;
- runtime environment;
- persisted data;
- user-visible surface;
- observation time.

### 2.2 Normative entity primitives: describing what we choose and require

#### Goal

A desired outcome or problem to be solved. A Goal motivates work but does not by itself authorize code or create a system obligation.

#### Decision

A selected direction among alternatives. A Decision becomes normative only through an accepted authority and must identify what was chosen, what was rejected, and why.

#### Contract

A stable, testable obligation created by an accepted Decision.

A Contract states what the system must or must not do across relevant success, failure, identity, authorization, lifecycle, transaction, migration, compatibility, retry, and observability paths.

Every accepted Contract has a stable ID. IDs are never renumbered, reused, or assigned a new meaning.

### 2.3 Relational primitive: Evidence

Evidence is a first-class, auditable relation that states **why one or more qualified Observations count for or against a specific target at specific coordinates**.

Evidence is not the same thing as its source material:

```text
raw file / log / screenshot / test definition = provenance material
qualified recorded result                       = Observation
Observation linked to a target with polarity,
coordinates, strength, and limits              = Evidence
```

Each load-bearing Evidence relation has a stable ID such as `EVD-001` and records:

```text
source_observations
target_type and target_id
relation
repository / spec / implementation revision
environment and observed_at
strength or sufficiency
limitations
provenance
```

Allowed relation vocabulary depends on the target type:

```text
Claim or State assertion:
  SUPPORTS | CONTRADICTS

Contract at a pinned Spec revision:
  SATISFIES | VIOLATES | INCONCLUSIVE
```

The same Observation may support one Claim, contradict another, and be irrelevant to a third. That context dependence is precisely why Evidence is modeled as a relation rather than as an intrinsic label on a file or Observation.

```text
Test Definition ≠ Observation
Executed Test Result with coordinates = Observation
Qualified Observation-to-target relation = Evidence
```

---

## 3. The relation graph

```text
Observation(s)
  ├─ EVD: SUPPORTS / CONTRADICTS ─────────────> Claim or State assertion
  └─ EVD: SATISFIES / VIOLATES / INCONCLUSIVE > Contract at a pinned revision

Claim
  ├─ contributes to ──────────────────────────> State projection
  └─ informs ─────────────────────────────────> Decision

Goal + State + Claims
  └─ inform ──────────────────────────────────> Decision

Accepted Decision
  └─ creates ─────────────────────────────────> Contract

Implementation
  └─ is evaluated against ────────────────────> Contract
```

---

## 4. Type boundaries

The following equalities are forbidden:

```text
Provenance Material ≠ Observation
State ≠ Observation
Observation ≠ Evidence
Observation ≠ Claim
Claim ≠ Decision
Goal ≠ Decision
Decision ≠ Contract
Spec ≠ Implementation Plan
Test Definition ≠ Observation
Test Definition ≠ Evidence
Accepted Spec ≠ Implemented State
Implementation Progress ≠ Conformance
Runtime State ≠ Spec Lifecycle
Activity ≠ Knowledge
Newest Document ≠ Highest Authority
External Dependency ≠ Locally Governed Authority
```

Examples of invalid coercion:

- “The code currently does X, therefore X is the Contract.”
- “The tests pass, therefore production is conforming.”
- “We discussed Y, therefore Y was decided.”
- “A proposed Spec was merged, therefore it is accepted.”
- “A later low-level Spec conflicts with Product Direction, therefore the later file wins.”
- “This repository depends on another service, therefore it may define that service’s behavior.”

---

## 5. Authority model

### 5.1 Default precedence

A consuming repository must name its authorities. The default precedence is:

```text
Product Direction
> accepted Architecture / long-lived Invariant authority
> accepted governing Specs
> code, tests, runtime, and operational records
```

The first three layers are normative. Code, tests, and runtime are descriptive: they may conform or drift, but they do not silently rewrite authority.

A lower-level authority may refine a higher-level authority. It may not silently override or supersede it.

### 5.2 Cross-repository boundary

An external authority may be referenced only with:

```text
repository
stable authority ID
exact revision
relationship
```

A local Spec may state what it expects from an external authority. It may not accept, amend, supersede, or redefine the authority owned by another repository.

### 5.3 Central distribution boundary

This governance repository is not automatically authoritative in a consumer.

A consumer creates local authority by:

1. selecting an exact source commit;
2. vendoring the distribution into a docs-only adoption branch;
3. recording the pin in `.agents/governance.lock.json`;
4. preparing the vendored snapshot with `adoption.status: proposed` and null acceptance metadata;
5. independently reviewing the adoption;
6. allowing an authorized local actor to finalize `adoption.status: accepted`;
7. merging the accepted snapshot into the designated authority branch.

Preparing files is not acceptance, and an unmerged accepted-looking lock is not active authority. A later upstream release has no effect until the consumer explicitly updates and accepts it.

---

## 6. Authority mutation

### 6.1 Preflight classification

Every non-mechanical request is classified as exactly one of:

```text
REUSE       existing accepted authority already covers the work
AMEND       proposed authority needs revision, or accepted text receives an editorial or strictly additive change
SUPERSEDE   accepted normative meaning must change
NEW         no existing authority owns the decision
```

### 6.2 Accepted meaning is immutable

Plain language: an accepted rule may later be replaced, but it may not be silently rewritten while keeping the same number. Old references must always keep their original meaning.

After acceptance:

- Decision and Contract meaning cannot change under the same stable ID;
- Contract IDs cannot be repurposed;
- editorial corrections are allowed only when semantic delta is demonstrably `NONE`;
- a strictly additive accepted-Spec amendment may add new stable IDs only when it remains within the existing Goal, scope, authority, and accepted Decisions and changes no existing meaning;
- a new Decision, expanded scope, or independent obligation uses a new Spec authority;
- deletion, narrowing, expansion, reversal, or changed failure semantics of existing meaning require `SUPERSEDE`.

### 6.3 V0 supersession

V0 supports only whole-authority supersession.

```text
PARTIAL_SUPERSESSION_V0 = FORBIDDEN
```

A supersession transition is atomic in one docs-only change:

```text
new authority: status = accepted; supersedes = [old]
old authority: status = superseded; superseded_by = new
```

Per-Contract supersession requires a future explicit machine-readable authority graph and is not inferred from prose.

---

## 7. Spec lifecycle and other state dimensions

### 7.1 Governing Spec lifecycle

```text
proposed
accepted
superseded
```

A Spec is active authority only when it is `accepted` **and present in the repository's designated authority branch or the implementation base derived from it**. An `accepted` value on an unmerged PR branch is a candidate final state, not yet active repository authority.

A rejected proposal was never governing authority. Its lasting knowledge is recorded in an Investigation Record with a disposition such as `rejected`, `no_change`, or `reuse`; it is not added as a fourth governing lifecycle state.

### 7.2 Implementation progress

```text
NOT_STARTED
IN_PROGRESS
COMPLETE
```

### 7.3 Verification coverage

```text
NOT_RUN
PARTIAL
SUFFICIENT
```

### 7.4 Conformance result

```text
UNKNOWN
VERIFIED
DRIFTED
```

These dimensions must not be collapsed into one enum.

Valid examples:

```text
Spec lifecycle = accepted
Implementation state = COMPLETE
Verification state = SUFFICIENT
Conformance = DRIFTED
```

```text
Spec lifecycle = accepted
Implementation state = IN_PROGRESS
Verification state = PARTIAL
Conformance = UNKNOWN
```

---

## 8. Qualified conformance

`VERIFIED` is never a permanent property of a Spec.

A Conformance Record binds:

```text
spec_id
spec_revision or blob
implementation_commit
environment
evaluated_at
verification_state
conformance_result
evidence
```

If any bound Spec, code, configuration, data migration, or environment changes, the record remains valid historical evidence for the old tuple but does not automatically verify the new tuple.

Contract-level results are:

```text
VERIFIED
DRIFTED
UNKNOWN
NOT_APPLICABLE
```

Aggregate rule:

- any active Contract `DRIFTED` → aggregate `DRIFTED`;
- all active Contracts `VERIFIED` with sufficient evidence → aggregate `VERIFIED`;
- otherwise → aggregate `UNKNOWN`.

---

## 9. Review and acceptance roles

```text
Author ≠ independent semantic Reviewer
Review recommendation ≠ acceptance authority
```

Every review records:

```text
reviewed base commit
reviewed Spec commit
reviewer identity
recommendation
final accepted head
semantic delta after review
```

Any semantic change after the reviewed commit invalidates the review. A status-only or proven editorial-only change still requires the final head to be independently checked.

Only a repository owner or explicitly authorized maintainer may perform the acceptance action.

---

## 10. Mechanical and emergency boundaries

### 10.1 Mechanical exemption

```text
UNCERTAIN = NON_MECHANICAL
```

Mechanical changes are narrowly limited to such cases as:

- spelling correction;
- formatting with no interpretation change;
- deterministic generated output already required by an accepted Contract;
- a pure path move with machine evidence that semantics are unchanged.

These are not mechanical by default:

- dependency upgrades;
- schema, API, scope, permission, retry, timeout, or lifecycle changes;
- changed test expectations;
- deletion of apparently unused behavior;
- refactors that change module or trust boundaries.

A mechanical exemption is reviewed independently and persisted.

### 10.2 Emergency remediation

Immediate emergency action is limited to:

```text
rollback
shutdown / disable
containment
credential revocation or isolation
```

It requires owner authorization and an incident reference. It may not introduce durable new behavior. Permanent repair still requires normal Spec reconciliation.

---

## 11. Persistence rules

Durable knowledge must not exist only in chat.

- accepted authority lives in repository files;
- semantic review and acceptance records live in persistent PR records or repository reports;
- implementation conformance lives in the implementation PR or a report when evidence spans environments;
- rejected, no-change, or reuse investigations live in an Investigation Record, issue, or investigation PR with a stable link;
- raw evidence may remain in logs or external systems, but the provenance relation and required excerpt or query must be recorded.

---

## 12. Forward-only adoption

Do not bulk-migrate all historical documents.

Apply the grammar from the next non-mechanical change forward. Reconcile historical artifacts only when they become governing, are cited by new work, or conflict with an active authority.

```text
NO_BULK_HISTORY_REWRITE = YES
NO_CENTRAL_SPEC_DATABASE_V0 = YES
NO_SEMANTIC_CI_REVIEW_V0 = YES
```
