# Governing Spec Format V0

```text
FORMAT_VERSION = 0.1.0-draft.1
STATUS = accepted
```

This file defines the minimum syntax contract for governing Specs. It is deliberately narrower than semantic review.

---

## 1. Location and identity

Governing Specs use a stable path:

```text
docs/specs/<SPEC_ID>.md
```

Do not move Specs among `proposed/`, `accepted/`, `rejected/`, or `implemented/` directories. Lifecycle is metadata; implementation and conformance are separate records.

`SPEC_ID` is stable, uppercase, and versioned, for example:

```text
AGENT_FORUM_CORE_INVARIANTS_V1
AUTH_SERVICE_CREDENTIAL_ROTATION_V2
```

A change to existing accepted normative meaning, or a new independent normative
meaning, uses a new Spec ID.

A strictly additive bounded elaboration under an existing accepted Decision
follows §14.2.

---

## 2. Required frontmatter

```yaml
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
```

### 2.1 `status`

```text
proposed | accepted | superseded
```

`status: accepted` becomes active repository authority only when that exact content is present in the repository's designated authority branch. Merely setting the field on an unmerged PR branch does not activate it.

### 2.2 `spec_kind`

```text
invariant | program | implementation
```

### 2.3 `authority_level`

For governing Specs:

```text
governing_spec
```

Product Direction and Architecture authorities may use equivalent frontmatter with `authority_id` and their own `authority_kind`; downstream Specs reference those stable IDs through `governed_by`.

### 2.4 `implementation_authority`

```text
none | contracts
```

`none` means acceptance does not authorize code. `contracts` means accepted Contracts may authorize bounded implementation after all base-branch and review rules are satisfied.

### 2.5 `governed_by`

Local parent authority IDs in precedence order. An empty list is allowed only when the repository explicitly identifies the Spec as a top-level local authority and the authorized maintainer accepts that role.

### 2.6 `external_authorities`

Each entry contains:

```yaml
external_authorities:
  - repository: mayf3/auth-service
    authority_id: AUTH_SERVICE_EXAMPLE_V1
    revision: 0123456789abcdef0123456789abcdef01234567
    relation: depends_on
```

Allowed relationships include:

```text
depends_on
constrained_by
interoperates_with
```

An external reference does not grant local supersession authority.

### 2.7 `supersedes` and `superseded_by`

V0 uses whole-Spec IDs only. Contract fragments such as `OLD_SPEC#CTR-003` are forbidden in supersession metadata.

Transition rules:

- proposed Spec: `superseded_by: null`;
- accepted replacement: `supersedes` names every fully replaced authority;
- superseded Spec: `superseded_by` names the accepted replacement;
- backlinks change atomically in the same docs-only change.

---

## 3. Required sections

A governing Spec contains these sections in this logical order. Additional sections are allowed when they do not blur primitive types.

```text
1. Goal
2. Scope and non-goals
3. Authority and dependencies
4. Current State
5. Observations
6. Claims and assumptions
7. Evidence relations
8. Decisions
9. Contracts
10. Acceptance
11. Alternatives and disposition
12. Migration, compatibility, and rollback
13. Open questions
```

A section may state `Not applicable` with a reason. It may not be silently omitted when the topic is relevant.

---

## 4. Stable item IDs

Recommended forms:

```text
OBS-001
CLM-001
STATE-001
EVD-001
DEC-001
CTR-AUTH-001
ACC-AUTH-001
ALT-001
```

Rules:

- IDs are stable after acceptance;
- IDs are never renumbered to close gaps;
- IDs are never reused after deletion or supersession;
- references use the exact ID, not only a section number;
- a Contract’s global identity is `<SPEC_ID>#<CONTRACT_ID>`.

---

## 5. Current State syntax

A load-bearing State statement records or references:

```text
subject
as_of_commit or artifact revision
environment
observed_at
basis: OBS-* / CLM-* / provenance
```

Example:

```markdown
- `STATE-AUTH-001` — Production token issuance mode is `v1` on deployment
  `deploy-2026-08-18.3`, observed at `2026-08-18T10:20:00Z`.
  Basis: `OBS-AUTH-003`, `OBS-AUTH-004`, `CLM-AUTH-001`.
```

Invalid:

```markdown
The current system guarantees v1 behavior.
```

The invalid form lacks subject coordinates and also coerces descriptive State into a normative Contract.

---

## 6. Observation syntax

Example:

```markdown
### OBS-AUTH-003 — Runtime banner reports v1 mode

- Subject: production auth-service process
- Source revision: `abc123...`
- Environment: production host group `auth-primary`
- Observed at: `2026-08-18T10:20:00Z`
- Method: inspect startup banner after verified restart
- Result: banner reports `mode=v1`
- Provenance: deployment log reference `...`
```

Do not add causal interpretation to the Observation result. Put interpretation in a Claim.

---

## 7. Claim syntax

Example:

```markdown
### CLM-AUTH-001 — The v1 issuer path is the active production path

- Support state: SUPPORTED
- Supported by evidence: `EVD-AUTH-001`
- Contradicted by evidence: none known
- Uncertainty: runtime evidence covers the observed deployment only
```

Allowed support states:

```text
SUPPORTED | INFERRED | OPEN_ASSUMPTION
```

An `OPEN_ASSUMPTION` that changes authority or Contract meaning blocks acceptance unless converted into an explicit owner decision or bounded risk Contract.

---

## 8. Evidence relation syntax

Example for a Claim or State assertion:

```markdown
### EVD-AUTH-001 — Runtime observations support the active-path Claim

- Source observations: `OBS-AUTH-003`, `OBS-AUTH-004`
- Target: `CLM-AUTH-001`
- Relation: SUPPORTS
- Bound coordinates: auth-service `abc123...`, production `auth-primary`, observed `2026-08-18T10:20:00Z`
- Strength/sufficiency: strong for the observed deployment
- Limitations: does not establish behavior on unobserved replicas or future revisions
- Provenance: deployment log and audit-query references `...`
```

In a governing Spec, the target is a Claim or State assertion and the relation is `SUPPORTS` or `CONTRADICTS`. Evidence that evaluates a Contract at an exact Spec revision uses `SATISFIES`, `VIOLATES`, or `INCONCLUSIVE` and is recorded later in a Conformance Record.

Raw files, screenshots, logs, or test definitions are provenance material. An executed result with coordinates is an Observation. The explicit source-to-target relation is Evidence.

---

## 9. Decision syntax

Example:

```markdown
### DEC-AUTH-001 — External consumers remain offline-verification-only

- Decision owner: repository owner
- Decision: external resource consumers validate signed tokens offline.
- Rejected alternative: generic online introspection.
- Reason: preserve the frozen trust boundary and avoid a new live dependency.
```

A Decision does not hide implementation detail that materially affects external behavior, failure semantics, security, migration, or lifecycle.

---

## 10. Contract syntax

Each Contract is independently reviewable and testable.

Example:

```markdown
### CTR-AUTH-001 — No external live status lookup

External resource consumers MUST validate Access Tokens using offline signature and
claim verification. They MUST NOT call auth-service for live principal, client,
or token status during request authorization.
```

Use normative verbs consistently:

```text
MUST
MUST NOT
MAY
```

A Contract states caller, subject, conditions, result, and relevant failures. Avoid vague forms such as “should work,” “handle correctly,” or “be secure.”

---

## 11. Acceptance syntax

Every active Contract has at least one Acceptance mapping.

Example:

```markdown
### ACC-AUTH-001 — Offline consumer verification

- Contracts: `CTR-AUTH-001`
- Method: integration test with auth-service unavailable after JWKS cache warmup
- Environment: staging
- Required evidence: executed command, implementation commit, configuration,
  request/response record, and service-call audit showing zero introspection calls
- Expected result: authorization succeeds from cached JWKS data with zero live
  auth-service calls
- Failure condition: any authorization-time call to auth-service fails acceptance
```

The syntax pass checks both directions:

- every referenced Contract exists;
- every active Contract has Acceptance coverage or an explicit evidence exception.

---

## 12. Alternatives and investigation disposition

A Spec records rejected alternatives that directly explain its accepted Decision.

When an investigation produces no new governing Spec, persist an Investigation Record instead. Do not leave a forever-proposed Spec solely to preserve rejected reasoning.

---

## 13. Open questions and acceptance

Before `status: accepted`:

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
```

Non-normative follow-up work may remain when it cannot change Decision or Contract meaning.

---

## 14. Accepted immutability

An accepted rule may later be replaced, but an existing stable ID always keeps
the meaning that was accepted for that ID.

### 14.1 Existing stable IDs

Text associated with an existing accepted Decision, Contract, Acceptance item,
or other stable normative ID may receive only editorial changes with proven
semantic delta `NONE`. An existing ID may not be narrowed, expanded, reversed,
deleted and reused, or assigned a different meaning.

### 14.2 Strictly additive amendment under the same `spec_id`

A docs-only `AMEND` MAY add new stable IDs to the same accepted Spec only when
all of the following are true:

- Goal is unchanged;
- scope and authority ownership are unchanged;
- `governed_by` and external-authority boundaries are unchanged;
- every accepted Decision remains unchanged;
- every existing stable ID and its Acceptance meaning remain unchanged;
- each new Contract is a bounded elaboration of an already accepted Decision,
  not a new independent product or architecture choice;
- the new IDs and their Contract-to-Acceptance mappings receive independent,
  exact-revision review;
- no partial supersession is introduced.

This seam permits a newly identified `CTR-*` that makes an existing accepted
Decision operationally complete. It does not permit changing an existing
`CTR-*` or smuggling a new Decision into the old authority.

### 14.3 `NEW` and `SUPERSEDE` boundaries

`NEW` is required for:

- a new Decision;
- expanded scope or changed authority ownership;
- a new independent obligation;
- an obligation that is not entailed by the existing accepted Decisions.

`SUPERSEDE` is required for:

- deletion, narrowing, expansion, reversal, or replacement of existing
  normative meaning;
- changed identity, authorization, failure, retry, timeout, transaction,
  lifecycle, migration, compatibility, or security semantics;
- removal of an existing obligation;
- changed Acceptance meaning that could cause a previously passing
  implementation to fail or a previously failing implementation to pass.

---

## 15. Syntax versus semantics

A deterministic parser may check:

- frontmatter presence and enums;
- ID uniqueness and patterns;
- required sections;
- local backlinks;
- Contract-to-Acceptance references;
- forbidden partial-supersession references;
- unresolved placeholders;
- governance lock integrity;
- presence and status of a Spec in a known base.

It cannot decide:

- whether a Claim is justified;
- whether a Contract is complete;
- whether an alternative was honestly considered;
- whether Product Direction has been silently overridden;
- whether evidence is sufficient in the real environment.

Those remain semantic review responsibilities.
