---
spec_id: SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V2
status: proposed
spec_kind: invariant
authority_level: governing_spec
implementation_authority: none
scope:
  - repository development process
governed_by: []
external_authorities:
  - repository: mayf3/agent-development-governance
    authority_id: AGENT_DEVELOPMENT_GOVERNANCE_V1
    revision: 902842735a69797b54016eeaa88d2f949f5879a9
    relation: constrained_by
supersedes:
  - SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V2

## 1. Goal

Adopt the exact Agent Development Governance v1.0.0 distribution for
`mayf3/svc-workflow` through this repository's own review and Owner acceptance
lifecycle.

After activation, repository work MUST route three independent questions:

```text
new or changed long-lived obligation -> Authority action
execution complexity                 -> Plan level
failure consequence                  -> Assurance level
```

A task MUST stop when its declared `DONE_WHEN` is met and no declared
`EXPANSION_TRIGGER` has fired.

This proposal prepares adoption only. It is not active authority until the
accepted final Head is merged into `main`.

## 2. Scope and non-goals

### In scope

- exact-commit vendoring of distribution `development-governance-v0`,
  version `1.0.0`;
- local lifecycle transition from the currently accepted adoption V1 to this
  whole-authority successor;
- preservation of repository-local governance, Product Direction,
  Architecture, governing Specs, acceptance actors, and persistence rules;
- deterministic integrity, route, and Spec-transition validators;
- forward-only use of Governance V1 for the next applicable task.

### Out of scope

This Spec does not authorize:

- product code, Rust, SQL, migrations, OpenAPI, SDK, tests that define product
  behavior, runtime configuration, deployment, or production mutation;
- changes to Product Direction, Architecture, product invariants, or accepted
  product Contracts;
- `AGENT_OPERATIONAL_LAYER_V1`;
- bulk rewriting of historical tasks, Specs, Reviews, investigations, or
  conformance records;
- GitHub settings, branch protection, Apps, brokers, WebAuthn, WORM, or other
  governance infrastructure;
- permission, Grant, Credential, Secret, identity, or database mutation;
- acceptance, Ready transition, or merge by the preparation Agent.

## 3. Authority and dependencies

```text
AUTHORITY_ACTION = SUPERSEDE
CURRENT_ACTIVE_LOCAL_AUTHORITY =
  SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1
CURRENT_ACTIVE_DISTRIBUTION_VERSION =
  0.1.0-draft.1
SUCCESSOR_CANDIDATE =
  SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V2
UPSTREAM_AUTHORITY =
  AGENT_DEVELOPMENT_GOVERNANCE_V1
UPSTREAM_RELEASE_TAG =
  v1.0.0
UPSTREAM_SOURCE_COMMIT =
  902842735a69797b54016eeaa88d2f949f5879a9
DISTRIBUTION_ID =
  development-governance-v0
PLAN_LEVEL =
  BRIEF
ASSURANCE_LEVEL =
  DURABLE
ROUTE_STAGE =
  AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE =
  NO
IMPLEMENTATION_ALLOWED =
  NO
OPERATION_ALLOWED =
  NO
MERGE_READY =
  NO
```

The accepted V1 adoption remains active until an authorized lifecycle-only
acceptance transaction atomically accepts this successor, supersedes V1 with
its backlink, updates local discovery metadata, receives an independent
final-Head recheck, and merges that exact accepted Head into `main`.

The upstream repository publishes exact bytes but does not remotely govern this
repository. The external authority is a constraint on the imported grammar,
not a grant of local supersession or implementation authority.

## 4. Current State

### STATE-ADOPT2-001 — Current active adoption

- Subject: `mayf3/svc-workflow` governance adoption.
- As of commit: `efdfb7e1a0e6a381b2ab000d48f842991d5c0bb6`.
- Environment: GitHub `main`.
- Observed at: `2026-09-01T22:48:46Z`.
- Basis: `OBS-ADOPT2-001`, `OBS-ADOPT2-002`.
- Projection: adoption V1 is accepted and pins
  `46f78c3f00d768d99a4c8c2da975b124bce042f9`,
  distribution version `0.1.0-draft.1`.

### STATE-ADOPT2-002 — Proposed candidate state

- Subject: this adoption preparation.
- As of revision: the future PR Head produced by this docs-only preparation.
- Environment: isolated adoption branch.
- Observed at: `2026-09-01T22:48:46Z`.
- Basis: `OBS-ADOPT2-003`, `OBS-ADOPT2-004`.
- Projection: v1.0.0 bytes and a proposed lock are prepared; no local
  acceptance or active-authority transition has occurred.

## 5. Observations

### OBS-ADOPT2-001 — Existing local lock is an accepted draft-version adoption

- Subject: `.agents/governance.lock.json`.
- Source revision: `efdfb7e1a0e6a381b2ab000d48f842991d5c0bb6`.
- Environment: GitHub `main`.
- Observed at: `2026-09-01T22:48:46Z`.
- Method: read the tracked lock and local adoption authority.
- Result: version `0.1.0-draft.1`, source commit
  `46f78c3f00d768d99a4c8c2da975b124bce042f9`,
  `adoption.status=accepted`.
- Provenance: repository files at the bound revision.

### OBS-ADOPT2-002 — Existing V1 adoption has accepted normative meaning

- Subject: `SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1`.
- Source revision: `efdfb7e1a0e6a381b2ab000d48f842991d5c0bb6`.
- Environment: GitHub `main`.
- Observed at: `2026-09-01T22:48:46Z`.
- Method: read frontmatter, Decisions, Contracts, and lifecycle text.
- Result: V1 is accepted and owns the current repository governance adoption.
- Provenance: `docs/specs/SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1.md`.

### OBS-ADOPT2-003 — v1.0.0 is an annotated exact-commit tag

- Subject: upstream tag `v1.0.0`.
- Source revision: tag object
  `bb98937d176890088da736fa4a45f48279f19d50`.
- Environment: GitHub.
- Observed at: `2026-09-01T22:48:46Z`.
- Method: resolve `refs/tags/v1.0.0`, then resolve the annotated tag object.
- Result: object type `tag`; target type `commit`; target commit
  `902842735a69797b54016eeaa88d2f949f5879a9`.
- Provenance: GitHub Git refs and tag-object APIs.

### OBS-ADOPT2-004 — Exact release manifest is internally consistent

- Subject: upstream `distribution/manifest.json`.
- Source revision: `902842735a69797b54016eeaa88d2f949f5879a9`.
- Environment: GitHub exact commit.
- Observed at: `2026-09-01T22:48:46Z`.
- Method: inspect manifest identity/version/file records and reproduce its
  canonical tracked bytes.
- Result: distribution `development-governance-v0`, version `1.0.0`,
  25 file entries, manifest SHA-256
  `c1fa620da4a16e4073d617e49eb5080487f2a117e3bab6502fd223afee0f06e0`.
- Provenance: upstream manifest blob
  `d4e37f492653260aa24878af1a9208f53122db5d`.

## 6. Claims and assumptions

### CLM-ADOPT2-001 — A successor is required

- Support state: SUPPORTED
- Supported by evidence: `EVD-ADOPT2-001`
- Contradicted by evidence: none known
- Uncertainty: none material to routing

Changing the accepted source commit, distribution version, active protocol,
route taxonomy, validators, and stop semantics changes accepted governance
meaning. It cannot be an in-place amendment of V1.

### CLM-ADOPT2-002 — Local authority remains sovereign

- Support state: SUPPORTED
- Supported by evidence: `EVD-ADOPT2-002`
- Contradicted by evidence: none known
- Uncertainty: none material to acceptance

Exact vendoring can adopt shared grammar without transferring Product
Direction, Architecture, acceptance, or runtime ownership to the upstream
repository.

## 7. Evidence relations

### EVD-ADOPT2-001 — Old and new exact revisions support SUPERSEDE

- Source observations: `OBS-ADOPT2-001`, `OBS-ADOPT2-002`,
  `OBS-ADOPT2-003`, `OBS-ADOPT2-004`
- Target: `CLM-ADOPT2-001`
- Relation: SUPPORTS
- Bound coordinates: consumer `efdfb7e1a0e6a381b2ab000d48f842991d5c0bb6`;
  upstream `902842735a69797b54016eeaa88d2f949f5879a9`
- Strength/sufficiency: sufficient to classify a whole-authority successor
- Limitations: does not constitute local acceptance
- Provenance: tracked authorities, lock, annotated tag, and manifest

### EVD-ADOPT2-002 — Vendor boundaries support local sovereignty

- Source observations: `OBS-ADOPT2-002`, `OBS-ADOPT2-004`
- Target: `CLM-ADOPT2-002`
- Relation: SUPPORTS
- Bound coordinates: same as `EVD-ADOPT2-001`
- Strength/sufficiency: sufficient for this adoption boundary
- Limitations: does not prove semantic conformance of future product work
- Provenance: local authority and upstream distribution/adoption contracts

## 8. Decisions

### DEC-ADOPT2-001 — Adopt the exact stable release

- Decision owner: repository owner `mayf3`
- Decision: prepare Governance v1.0.0 from exact source commit
  `902842735a69797b54016eeaa88d2f949f5879a9`.
- Rejected alternative: float to upstream `main`, a merge commit, or `latest`.
- Reason: exact bytes and local review must be reproducible.

### DEC-ADOPT2-002 — Preserve the compatibility distribution ID

- Decision owner: repository owner `mayf3`
- Decision: keep distribution ID `development-governance-v0` while setting
  version `1.0.0`.
- Rejected alternative: rename the distribution during consumer adoption.
- Reason: the upstream release and local compatibility contract require the
  existing ID.

### DEC-ADOPT2-003 — Keep adoption local and proposed

- Decision owner: repository owner `mayf3`
- Decision: prepare the vendor snapshot with `adoption.status=proposed`,
  `accepted_by=null`, and `accepted_at=null`.
- Rejected alternative: treat the upstream release or preparation Agent as
  local acceptance.
- Reason: only repository-local review and Owner acceptance can activate it.

### DEC-ADOPT2-004 — Preserve local extensions and product authority

- Decision owner: repository owner `mayf3`
- Decision: replace only manifest-owned shared files and lock/adoption
  metadata; preserve `AGENTS.md`, `.agents/local/**`, Product Direction,
  Architecture, Specs, acceptance actors, product code, and runtime rules.
- Rejected alternative: overwrite local templates or bulk-rewrite history.
- Reason: upstream publication does not own local product semantics.

### DEC-ADOPT2-005 — Use the three-axis route forward-only

- Decision owner: repository owner `mayf3`
- Decision: after activation, classify Authority, Plan, and Assurance
  independently and stop when `DONE_WHEN` is met without an
  `EXPANSION_TRIGGER`.
- Rejected alternative: use one heavy route for all non-mechanical work or
  retroactively rewrite historical artifacts.
- Reason: this is the v1.0.0 governance model and avoids governance drift.

## 9. Contracts

### CTR-ADOPT2-001 — Exact source and manifest

The adopted lock MUST name source repository
`mayf3/agent-development-governance`, source commit
`902842735a69797b54016eeaa88d2f949f5879a9`, version `1.0.0`, distribution
`development-governance-v0`, and exact manifest SHA-256
`c1fa620da4a16e4073d617e49eb5080487f2a117e3bab6502fd223afee0f06e0`.

### CTR-ADOPT2-002 — Proposed preparation

The preparation Head MUST record `adoption.status=proposed`,
`accepted_by=null`, and `accepted_at=null`. It MUST NOT claim active adoption,
Owner acceptance, Ready status, or merge authorization.

### CTR-ADOPT2-003 — Manifest-owned write set

Every `.agents/**` shared file changed or created by this adoption MUST be
listed by the exact upstream manifest and MUST match its size and SHA-256.
No unlisted shared path may be introduced as an imported governance file.

### CTR-ADOPT2-004 — Local extension preservation

`AGENTS.md`, `.agents/local/**`, Product Direction, Architecture, accepted
product Specs, acceptance actors, product code, and runtime rules MUST remain
unchanged. `docs/specs/README.md` MAY change only to record the local adoption
successor and the active V1 read order of the candidate.

### CTR-ADOPT2-005 — Local successor lifecycle

This proposal MUST remain a whole-authority successor of
`SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1`. While proposed, V1 remains
the active local adoption and MUST NOT receive a superseded backlink. A later
acceptance transaction MUST update both sides atomically.

### CTR-ADOPT2-006 — Three-axis routing

After this successor is accepted and merged, every non-trivial applicable task
MUST classify Authority action, Plan level, and Assurance level independently.
Plan complexity MUST NOT create Product Authority, and high consequence MUST
NOT create a new Spec when accepted Contracts already own the behavior.

### CTR-ADOPT2-007 — Stop control

Applicable tasks MUST define `DONE_WHEN`. Durable, controlled, or
expansion-prone work MUST define `EXPANSION_TRIGGER`. When `DONE_WHEN` is met
and no trigger fired, the task MUST stop rather than expand into optional work.

### CTR-ADOPT2-008 — Forward-only history

Governance V1 MUST apply from the next applicable task forward. This adoption
MUST NOT bulk-rewrite historical tasks, Specs, Reviews, investigations, or
conformance records.

### CTR-ADOPT2-009 — Validator availability and limits

The vendored `validate_governance_route.py` and
`validate_spec_transition.py` MUST be runnable with Python 3 and MUST reject
internally inconsistent route/lifecycle records covered by their schemas.
Their success MUST NOT be represented as semantic review or product
conformance.

### CTR-ADOPT2-010 — No product or runtime authority

Neither this proposed preparation nor its later acceptance may by itself
authorize product implementation, `AGENT_OPERATIONAL_LAYER_V1`, production
mutation, GitHub settings, permission/Grant/Credential/Secret changes, or
historical migration.

### CTR-ADOPT2-011 — Independent review before acceptance

An independent Reviewer MUST bind the exact Base and proposed adoption Head,
verify tag/commit/manifest/lock/vendor bytes/local preservation/validator
behavior, and recommend ACCEPT before Owner acceptance is allowed.

### CTR-ADOPT2-012 — Exact final-Head activation

Any Owner acceptance MUST create a lifecycle-only final accepted Head, preserve
the reviewed semantic meaning, receive an independent final-Head recheck, and
become active only when that exact Head is merged into `main`.

## 10. Acceptance

### ACC-ADOPT2-001 — Exact release pin

- Contracts: `CTR-ADOPT2-001`
- Method: resolve the annotated tag, recompute manifest digest, and inspect lock
- Environment: clean isolated candidate plus upstream exact commit
- Required evidence: tag object, commit target, manifest blob/digest, lock
- Expected result: all fixed identities and digests match exactly
- Failure condition: tag is lightweight, resolves elsewhere, or any fixed field differs

### ACC-ADOPT2-002 — Proposed lifecycle

- Contracts: `CTR-ADOPT2-002`
- Method: parse lock and candidate authority frontmatter
- Environment: proposed PR Head
- Required evidence: exact candidate files and PR state
- Expected result: proposed/null/null and no active/accepted claim
- Failure condition: any acceptance actor/time, active claim, Ready, or merge occurs

### ACC-ADOPT2-003 — Vendor byte closure

- Contracts: `CTR-ADOPT2-003`
- Method: verify every lock file against the exact manifest and candidate blob
- Environment: isolated candidate
- Required evidence: per-path size/SHA-256 results
- Expected result: all 25 entries match; no unlisted imported shared file exists
- Failure condition: missing, extra, size-mismatched, or digest-mismatched vendor path

### ACC-ADOPT2-004 — Local preservation

- Contracts: `CTR-ADOPT2-004`
- Method: compare Base and candidate changed paths and bytes
- Environment: Git comparison
- Required evidence: changed-file list and exact diffs
- Expected result: local extension/product paths are unchanged; index change is bounded
- Failure condition: local authority, Product Direction, code, runtime, or acceptance actor changes

### ACC-ADOPT2-005 — Successor boundary

- Contracts: `CTR-ADOPT2-005`
- Method: inspect V1/V2 frontmatter, index, and active local authority map
- Environment: proposed Head
- Required evidence: exact records and transition validation plan
- Expected result: V2 names V1; V1 remains accepted without backlink while V2 is proposed
- Failure condition: V1 is edited/superseded before authorized acceptance or partial supersession appears

### ACC-ADOPT2-006 — Three independent axes

- Contracts: `CTR-ADOPT2-006`
- Method: execute route-validator positive and negative fixtures
- Environment: Python 3 using vendored validator
- Required evidence: route JSON, commands, exit codes, output
- Expected result: valid `SUPERSEDE + BRIEF + DURABLE` authoring route passes;
  mismatched Authority, Plan, or Assurance declarations fail
- Failure condition: an inconsistent route passes or valid independent axes are rejected

### ACC-ADOPT2-007 — Done/expansion stopping

- Contracts: `CTR-ADOPT2-007`
- Method: route-validator fixtures for stop controls
- Environment: Python 3
- Required evidence: valid and invalid fixture outputs
- Expected result: `DONE_WHEN=true` and no trigger requires `STOP`; fired trigger
  requires re-PREFLIGHT or Owner decision
- Failure condition: optional continuation is accepted after Done When without trigger

### ACC-ADOPT2-008 — No historical rewrite

- Contracts: `CTR-ADOPT2-008`
- Method: inspect changed paths and history edits
- Environment: Base-to-Head diff
- Required evidence: changed-file list
- Expected result: no historical task/Spec/Review bulk rewrite
- Failure condition: unrelated historical records are rewritten

### ACC-ADOPT2-009 — Validator execution and non-semantic boundary

- Contracts: `CTR-ADOPT2-009`
- Method: Python compilation/help and positive/negative validator execution
- Environment: clean isolated candidate
- Required evidence: commands, exit codes, output
- Expected result: tools run deterministically and documentation states their limits
- Failure condition: tool cannot run, fails to reject covered contradictions, or claims semantic judgment

### ACC-ADOPT2-010 — No implementation or production effects

- Contracts: `CTR-ADOPT2-010`
- Method: changed-path inspection and repository/runtime state check
- Environment: candidate branch and unchanged runtime
- Required evidence: diff, PR metadata, no-operation record
- Expected result: docs/governance-only candidate with no external mutation
- Failure condition: any product, runtime, settings, permission, Secret, or operational effect occurs

### ACC-ADOPT2-011 — Independent semantic adoption review

- Contracts: `CTR-ADOPT2-011`
- Method: fresh independent exact-Head review
- Environment: detached clean worktree or equivalent isolated read surface
- Required evidence: complete review record with exact Base/Head and Reviewer identity
- Expected result: all adoption Contracts pass with zero blockers
- Failure condition: author self-review, stale coordinates, unverifiable evidence, or any blocker

### ACC-ADOPT2-012 — Final lifecycle recheck

- Contracts: `CTR-ADOPT2-012`
- Method: compare reviewed semantic Head with final accepted Head and merge result
- Environment: acceptance branch and GitHub main
- Required evidence: Owner acceptance, lifecycle diff, final review, merge ancestry
- Expected result: semantic delta none and exact accepted Head present in main
- Failure condition: semantic delta, stale Head, missing recheck, or unmerged accepted-looking branch

## 11. Alternatives and disposition

### ALT-ADOPT2-001 — Amend accepted V1 in place

Rejected. Changing the pinned source, active protocol, route model, validators,
and stop semantics changes accepted meaning.

### ALT-ADOPT2-002 — Float to upstream main

Rejected. It destroys exact-byte reviewability and local control.

### ALT-ADOPT2-003 — Mark adoption accepted during preparation

Rejected. Preparation and Owner acceptance are distinct acts.

### ALT-ADOPT2-004 — Rename the distribution ID

Rejected. `development-governance-v0` is the published compatibility identity
for version `1.0.0`.

## 12. Migration, compatibility, and rollback

This is a forward-only governance transition.

While the candidate is proposed:

- current V1 adoption remains active;
- product and runtime behavior do not change;
- new v1 tools exist only on the candidate branch.

After local acceptance and merge:

- Governance V1 becomes the active shared grammar;
- `SPEC_GOVERNANCE_V0.md` remains historical compatibility material;
- existing product authorities retain their meaning;
- historical work is not rewritten.

Rollback after merge is an ordinary Git revert restoring the previous lock and
exact vendored bytes together. Manual approximation of an older distribution is
forbidden.

## 13. Open questions

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
```

Non-normative follow-up: an independent adoption Reviewer must evaluate the
exact proposed Head. No review result is claimed by this preparation.
