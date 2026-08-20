# Spec Governance Protocol V0

```text
PROTOCOL_VERSION = 0.1.0-draft.1
STATUS = accepted
ENFORCEMENT_LEVEL = manual_policy
```

This protocol turns the Development Grammar into a repository workflow. The executable Agent procedure is in `.agents/skills/spec-governance/SKILL.md`; the syntax contract is in `.agents/protocol/SPEC_FORMAT_V0.md`.

---

## 1. Scope

The protocol applies to non-mechanical work that changes durable system behavior, public or internal contracts, authority boundaries, data semantics, lifecycle, failure handling, operational guarantees, or architecture.

It governs:

- authority discovery;
- Spec authoring and lifecycle;
- independent review;
- acceptance binding;
- implementation preconditions;
- conformance evaluation;
- supersession;
- investigation persistence;
- emergency containment reconciliation.

It does not prescribe a project-management system, deployment platform, database, programming language, or central registry.

---

## 2. Required repository surfaces

A consumer has, at minimum:

```text
AGENTS.md
.agents/README.md
.agents/local/README.md
.agents/governance.lock.json
.agents/protocol/SPEC_GOVERNANCE_V0.md
.agents/protocol/SPEC_FORMAT_V0.md
.agents/skills/spec-governance/SKILL.md
.agents/skills/spec-governance/modes/<MODE>.md
.agents/tools/verify_governance.py
docs/specs/README.md
docs/specs/<SPEC_ID>.md
```

`AGENTS.md` is a thin entrypoint. `.agents/README.md` owns the shared grammar. `.agents/local/README.md` owns repository-specific authority, acceptance roles, and exceptions. `docs/specs/` owns governing Specs.

Do not duplicate full rules across all four locations.

### 2.1 Adoption activation

The vendor operation first creates a proposed lock:

```text
adoption.status = proposed
accepted_by = null
accepted_at = null
```

Preparation is not acceptance. An authorized local actor may finalize `adoption.status = accepted` only after the adoption review. The shared distribution becomes active local governance only after that accepted snapshot and local authority declaration are merged into the designated authority branch.

The vendor tool binds the declared source commit to the clean source checkout and refuses to overwrite previously vendored files that no longer match the existing lock without an explicit recovery override.

---

## 3. Authority declaration

Every repository names its local precedence and acceptance actors in `.agents/local/README.md` or an equivalent machine-readable manifest.

Every Product Direction or Architecture authority that can govern downstream Specs has:

```text
authority_id
status
authority_kind
owning_repository
```

Every governing Spec declares:

```text
spec_id
status
spec_kind
authority_level
implementation_authority
scope
governed_by
external_authorities
supersedes
superseded_by
owners
```

### 3.1 Default authority kinds

```text
product_direction
architecture
governing_spec
```

### 3.2 Spec kinds

```text
invariant       long-lived constraints
program         decomposition, ordering, and child-Spec boundaries
implementation bounded behavior change and executable Contracts
```

`spec_kind` does not silently grant implementation authority. The explicit field is:

```text
implementation_authority: none | contracts
```

A Program Spec normally uses `none`. An accepted Program Spec does not authorize all child implementation.

---

## 4. Preflight

Before changing code, configuration, schema, generated artifacts, tests that define behavior, or operational logic:

1. identify the target repository and exact base commit;
2. read local Product Direction, Architecture, and accepted Specs in precedence order;
3. identify external authorities at exact revisions;
4. classify the change as mechanical or non-mechanical;
5. classify authority handling as `REUSE`, `AMEND`, `SUPERSEDE`, or `NEW`;
6. identify one primary governing Spec and any related authorities;
7. verify whether the primary Spec is accepted and present in the implementation base;
8. verify that it has `implementation_authority: contracts` and covers the requested scope.

Required output:

```text
PREFLIGHT_MODE = REUSE | AMEND | SUPERSEDE | NEW
CHANGE_CLASS = MECHANICAL | NON_MECHANICAL
MECHANICAL_EXEMPTION_REVIEW = ACCEPT | REJECT | NOT_APPLICABLE
GOVERNANCE_ADOPTION_STATUS = accepted | proposed | absent | source_repository
PRIMARY_GOVERNING_SPEC = <ID | NONE>
GOVERNING_SPEC_REVISION = <commit/blob | NONE>
SPEC_PRESENT_IN_BASE = YES | NO | NOT_APPLICABLE
SPEC_STATUS_IN_BASE = accepted | proposed | superseded | NONE
IMPLEMENTATION_AUTHORITY = contracts | none | UNKNOWN
IMPLEMENTATION_ALLOWED = YES | NO
```

`IMPLEMENTATION_ALLOWED = YES` only when:

- the change is independently accepted as mechanical; or
- an accepted implementation-authorizing Spec is present in the implementation base and the change remains within its Contracts.

---

## 5. Authoring

A new or superseding Spec is docs-only. It follows this investigation chain:

```text
Goal
→ authority and scope
→ Observations
→ Claims / assumptions
→ Evidence relations
→ Current State projection
→ Decision
→ Contracts
→ Acceptance
→ alternatives and disposition
```

### 5.1 Observation requirements

Each load-bearing Observation has a stable ID such as `OBS-001` and records relevant coordinates:

- repository and commit;
- file and line or symbol;
- command, request, query, or reproduction method;
- environment and configuration identity;
- timestamp when runtime-sensitive;
- exact result or a bounded excerpt;
- provenance location.

### 5.2 State requirements

Each load-bearing State statement cites `OBS-*`, `CLM-*`, `EVD-*`, or direct provenance. A State section may summarize; it may not become an unsourced bypass around Observation requirements.

### 5.3 Claim requirements

Each load-bearing Claim has a stable ID such as `CLM-001` and one support state:

```text
SUPPORTED
INFERRED
OPEN_ASSUMPTION
```

Each `SUPPORTED` or `INFERRED` Claim references the `EVD-*` relations that support or contradict it. `OPEN_ASSUMPTION` records that sufficient supporting Evidence does not yet exist.

Assumptions that affect authority or Contract meaning block acceptance until resolved or explicitly owned as a Contract risk.

### 5.4 Evidence requirements

Each load-bearing Evidence relation in a governing Spec has a stable ID such as `EVD-001`. It links one or more `OBS-*` records to a specific Claim or State assertion. Contract-targeted Evidence belongs in a Conformance Record, not in the governing Spec.

An Evidence record states:

- source Observation IDs;
- target type and stable target ID;
- relation: `SUPPORTS` or `CONTRADICTS`;
- bound repository, Spec, implementation, environment, and time coordinates;
- strength or sufficiency;
- limitations;
- provenance.

A raw artifact or test definition is not Evidence. A qualified executed result is an Observation; it becomes Evidence only through an explicit, bounded relationship to a target.

### 5.5 Decision requirements

Each normative Decision has a stable ID such as `DEC-001`. It states:

- the selected direction;
- the authority that may make the decision;
- alternatives considered;
- rejected reasons;
- whether owner input remains required.

### 5.6 Contract requirements

Each Contract has a stable ID such as `CTR-AUTH-001`.

Contracts cover relevant:

- success behavior;
- caller and subject identity;
- authorization and scope;
- trust boundary;
- failure and denial behavior;
- transaction and atomicity;
- lifecycle and deletion;
- timeout, retry, idempotency, and unknown outcomes;
- migration, compatibility, and rollback;
- observability and audit;
- operational and security boundary.

Contract IDs are never renumbered or reused after acceptance.

### 5.7 Acceptance requirements

Every active Contract is covered by at least one Acceptance item, or is explicitly marked as requiring runtime/manual evidence with a reason.

Acceptance items identify:

- Contract IDs;
- verification method;
- environment;
- expected result;
- required evidence;
- failure condition that would make an incorrect implementation fail.

A test filename alone is not an Acceptance result.

---

## 6. Review

Review is performed by an Agent or maintainer independent from the authoring act.

The reviewer checks:

1. authority precedence and ownership;
2. whether any lower-level Spec overrides a parent;
3. whether external dependencies are merely referenced rather than governed;
4. whether partial supersession is attempted;
5. primitive type boundaries;
6. State provenance;
7. Claim support status;
8. Evidence relation identity, coordinates, polarity, and limits;
9. decisions still deferred to implementation;
10. Contract completeness across negative and lifecycle paths;
11. reverse coverage from every Contract to Acceptance;
12. compatibility, migration, emergency, and rollback seams;
13. implementability without chat history;
14. stable IDs and lifecycle transitions;
15. whether the proposed change is actually `AMEND`, `SUPERSEDE`, or `NEW`.

Required recommendation:

```text
SPEC_REVIEW = ACCEPT | REVISE
READY_TO_MARK_ACCEPTED = YES | NO
REVIEWED_BASE_COMMIT = <sha>
REVIEWED_SPEC_COMMIT = <sha>
REVIEWER_ID = <identity>
ACCEPTANCE_ACTOR_REQUIRED = <identity or role>
```

Review is not acceptance.

---

## 7. Acceptance binding

An authorized maintainer performs the acceptance action after review.

The persistent review or acceptance record binds:

```text
REVIEWED_BASE_COMMIT
REVIEWED_SPEC_COMMIT
REVIEWER_ID
FINAL_ACCEPTED_HEAD
ACCEPTANCE_ACTOR
ACCEPTED_AT
SEMANTIC_DELTA_AFTER_REVIEW
```

Allowed result:

```text
SEMANTIC_DELTA_AFTER_REVIEW = NONE
```

If the delta from the reviewed Spec commit to the final accepted head changes Goal, authority, scope, Decision, Contract, Acceptance, migration, compatibility, failure, or security meaning, the review is invalid and must be rerun.

The final accepted head must be independently rechecked even when the intended delta is status-only.

---

## 8. Spec lifecycle

### 8.1 Proposed

- not normative;
- may change freely in a docs-only PR;
- may be reviewed;
- cannot authorize implementation.

### 8.2 Accepted

- becomes active repository authority only after the accepted content is present in the designated authority branch;
- an accepted value on an unmerged PR branch is not yet active authority;
- normative within its declared scope and authority once active;
- must already exist in the implementation PR base;
- Decision and Contract meaning is immutable under the same IDs;
- editorial-only changes require review and `SEMANTIC_DELTA = NONE`;
- strictly additive amendments may introduce new stable IDs only within unchanged Goal, scope, authority, and accepted Decisions.

### 8.3 Superseded

- remains historical authority for the time and revisions when it was active;
- does not govern new implementation after the superseding authority becomes active;
- contains a machine-readable `superseded_by` backlink.

### 8.4 Rejected and no-change outcomes

A proposal that never became authority is not `superseded` and does not become a governing `rejected` Spec.

Persist it as an Investigation Record with:

```text
disposition = rejected | no_change | reuse | deferred
reason
observations and claims considered
what would reopen the question
links to affected authorities
```

---

## 9. Amend and supersede

### 9.1 AMEND

`AMEND` is valid for:

- any change while a Spec remains proposed;
- editorial-only repair to accepted text with no normative semantic delta;
- a strictly additive accepted-Spec change that adds new stable item IDs while preserving the existing Goal, scope, authority, accepted Decisions, and every existing Decision and Contract meaning.

A strictly additive amendment must be independently reviewed and pinned by exact revision. A new Decision, expanded scope, or independent obligation uses `NEW`. A change to existing normative meaning uses `SUPERSEDE`.

### 9.2 SUPERSEDE

V0 supersedes a whole authority, not selected paragraphs or Contracts.

The docs-only transition atomically changes:

```text
NEW.status = accepted
NEW.supersedes = [OLD]
OLD.status = superseded
OLD.superseded_by = NEW
```

If a new authority cannot fully replace the old authority, the work requires either:

- a new non-conflicting authority that refines the same parent; or
- a future explicit partial-authority model.

Do not simulate partial supersession in prose.

---

## 10. Implementation

An implementation PR records:

```text
PRIMARY_GOVERNING_SPEC
RELATED_ACCEPTED_AUTHORITIES
IMPLEMENTATION_BASE_COMMIT
GOVERNING_SPEC_COMMIT_OR_BLOB
SPEC_PRESENT_IN_BASE = YES
SPEC_STATUS_IN_BASE = accepted
IMPLEMENTATION_COMMIT
```

The implementation must not modify the normative meaning it is implementing.

If implementation discovers a missing or incorrect Contract:

1. stop the affected semantic implementation;
2. report the gap;
3. create a separate docs-only Spec change;
4. review and accept it;
5. rebase or restart implementation on a base containing the accepted authority.

---

## 11. Compliance

Compliance is Contract-by-Contract, not PR-by-impression.

Required Conformance Record coordinates:

```text
spec_id
spec_revision
implementation_commit
environment
evaluated_at
implementation_state
verification_state
conformance_result
evidence_ids
```

Required table:

```text
Contract ID
→ implementation location
→ verification mechanism
→ executed Observation(s) and `EVD-*` relation(s)
→ result
```

Aggregate outputs:

```text
IMPLEMENTATION_STATE = NOT_STARTED | IN_PROGRESS | COMPLETE
VERIFICATION_STATE = NOT_RUN | PARTIAL | SUFFICIENT
CONFORMANCE = UNKNOWN | VERIFIED | DRIFTED
IMPLEMENTATION_READY_TO_MERGE = YES | NO
```

`IMPLEMENTATION_READY_TO_MERGE = YES` requires:

```text
IMPLEMENTATION_STATE = COMPLETE
VERIFICATION_STATE = SUFFICIENT
CONFORMANCE = VERIFIED
```

The record persists in the implementation PR. When evidence spans production, migration, canary, restarts, or external services, persist a repository report and link it from the PR.

---

## 12. Drift

Drift means implementation or runtime does not satisfy accepted authority at the evaluated tuple.

When drift is found:

- identify affected Contract IDs;
- identify the implementation and environment revisions;
- preserve evidence;
- assess safety and containment;
- do not edit the accepted Spec to match the drift;
- repair implementation or explicitly supersede authority through a separate process.

A legal and important state is:

```text
Spec lifecycle = accepted
Implementation state = COMPLETE
Conformance = DRIFTED
```

---

## 13. Mechanical exemption

The author states a mechanical reason. An independent reviewer accepts or rejects the exemption.

Persistent record:

```text
SPEC_REQUIRED = NO
MECHANICAL_REASON = <bounded reason>
MECHANICAL_EXEMPTION_REVIEWED_BY = <identity>
MECHANICAL_EXEMPTION_RESULT = ACCEPT | REJECT
```

Uncertainty, dependency changes, changed test expectations, changed boundaries, and behavior-preserving claims without evidence are non-mechanical.

---

## 14. Emergency handling

Emergency containment may precede a full Spec cycle only when limited to rollback, disablement, shutdown, credential revocation, or isolation.

Required record:

```text
INCIDENT_REFERENCE
OWNER_APPROVAL
ACTION_KIND
DURABLE_NEW_BEHAVIOR = NO
POST_INCIDENT_SPEC_RECONCILIATION_REQUIRED = YES
```

A durable code fix follows the normal Spec-first process.

---

## 15. Enforcement levels

V0 names enforcement honestly:

```text
MANUAL_POLICY = implemented by instructions and review practice
DISTRIBUTION_INTEGRITY = implemented by deterministic tooling
SPEC_SYNTAX_GATE = schema published, full verifier not implemented
BASE_BRANCH_MERGE_GATE = not implemented by this distribution
SEMANTIC_REVIEW = human/Agent judgment, not CI
```

A repository may add deterministic gates later, but must not claim enforcement that branch protection and required checks do not actually provide.
