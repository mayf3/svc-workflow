---
spec_id: SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1
status: accepted
spec_kind: invariant
authority_level: governing_spec
implementation_authority: none
scope:
  - mayf3/svc-workflow
  - repository-development-process
governed_by: []
external_authorities:
  - repository: mayf3/agent-development-governance
    authority_id: AGENT_DEVELOPMENT_GOVERNANCE_BOOTSTRAP_V0
    revision: 46f78c3f00d768d99a4c8c2da975b124bce042f9
    relation: constrained_by
supersedes: []
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1

## 1. Goal

Adopt an exact, locally reviewable revision of the shared Development Grammar and Spec-governance protocol so future `svc-workflow` changes preserve authority, reasoning, implementation, and evidence boundaries across Agent sessions.

```text
GOAL = make future non-mechanical development spec-first, independently reviewable, and contract-by-contract auditable
SUCCESS_OUTCOME = every future implementation can identify its exact authority and the exact evidence that verifies or violates it
```

## 2. Scope and non-goals

### In scope

- exact-commit vendoring of `development-governance-v0`;
- a proposed governance lock with per-file digests;
- the repository-local authority map, acceptance actors, exception actors, and persistence locations;
- stable locations and format for future governing Specs;
- forward-only use for future non-mechanical work;
- explicit governance update and rollback boundaries.

### Out of scope

- any Rust, SQL, migration, API, OpenAPI, SDK, authentication, authorization, runtime, deployment, or product-behavior change;
- retroactively rewriting all historical Architecture, Contract, investigation, audit, or implementation documents;
- declaring every document under `docs/contracts/` accepted authority;
- claiming a full Spec parser, base-branch merge gate, semantic CI reviewer, required status check, or branch protection that is not active;
- authorizing implementation of any product or architecture change.

## 3. Authority and dependencies

```text
SOURCE_REPOSITORY = mayf3/agent-development-governance
SOURCE_COMMIT = 46f78c3f00d768d99a4c8c2da975b124bce042f9
DISTRIBUTION = development-governance-v0
DISTRIBUTION_VERSION = 0.1.0-draft.1
MANIFEST_SHA256 = 58b5b28bb801538fe62be0ac98a7bc539ff34ec24fa368c48996dd40d8653ba0
CONSUMER_BASE_COMMIT = 8cda3d05e1c22814b7aeaace97d317380df83836
LOCAL_ACCEPTANCE_ACTOR = repository owner mayf3
IMPLEMENTATION_AUTHORITY = none
```

The external repository supplies grammar, protocol, schemas, templates, Skill procedures, and an integrity verifier. It does not own or supersede `svc-workflow` Product Direction, Architecture, governing Specs, code, runtime, or acceptance actions.

This Spec is a top-level local authority only for repository development process. Product authority precedence remains locally owned and is declared in `.agents/local/README.md`.

## 4. Current State

### STATE-ADOPT-001 — Governance is absent from the adoption base

- Subject: repository-level Development Grammar and Spec-governance surface
- As of commit: `8cda3d05e1c22814b7aeaace97d317380df83836`
- Environment: `mayf3/svc-workflow` default branch `main`
- Observed at: `2026-08-19T14:33:51Z`
- Projection: the base has product, Architecture, Contract, audit, test, and runtime artifacts, but no active `.agents/governance.lock.json` adoption and no repository-wide accepted-Spec-before-implementation protocol.
- Basis: `OBS-ADOPT-004`, `OBS-ADOPT-005` and repository tree provenance.

### STATE-ADOPT-002 — Local product authorities pre-exist this adoption

- Subject: local normative authority inventory
- As of commit: `8cda3d05e1c22814b7aeaace97d317380df83836`
- Environment: repository source tree
- Observed at: `2026-08-19T14:33:51Z`
- Projection: `PRODUCT-BOUNDARY.md` is the active product boundary; `SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md` is frozen Architecture; `SVC_WORKFLOW_ARCHITECTURE_V0_3_2.md` is an effective bounded refinement for Cancel / Archive; additional legacy documents retain only their previously established authority.
- Basis: `OBS-ADOPT-003` and direct file provenance.

## 5. Observations

### OBS-ADOPT-001 — Exact upstream distribution revision is identified

- Subject: shared governance distribution
- Repository/source: `mayf3/agent-development-governance`
- Commit/artifact: `46f78c3f00d768d99a4c8c2da975b124bce042f9`
- Environment: GitHub default branch and immutable commit contents
- Observed at: `2026-08-19T14:33:51Z`
- Method: read the exact `main` branch commit, `README.md`, consumer adoption protocol, and `distribution/manifest.json` through GitHub at the pinned revision.
- Result: `main` resolves to the pinned commit; the manifest declares distribution version `0.1.0-draft.1` and exactly 17 vendored files.
- Provenance: upstream branch, commit, README, adoption protocol, and distribution manifest at the pinned revision.

### OBS-ADOPT-002 — Vendored file bytes preserve source Git blob identity

- Subject: 17 distributed governance files prepared for `svc-workflow`
- Repository/source: source `mayf3/agent-development-governance` and target `mayf3/svc-workflow`
- Commit/artifact: source `46f78c3f00d768d99a4c8c2da975b124bce042f9`; consumer base `8cda3d05e1c22814b7aeaace97d317380df83836`
- Environment: GitHub Git Data API object preparation
- Observed at: `2026-08-19T14:33:51Z`
- Method: create each source UTF-8 file as a target repository blob and compare the resulting target Git blob SHA with the source blob SHA.
- Result: all 17 resulting target blob SHAs equal their corresponding source blob SHAs; no distributed file has a content, encoding, or newline delta.
- Provenance: source distribution manifest, source file blob identities, and target blob-creation responses.

### OBS-ADOPT-003 — Existing local authority is explicit and bounded

- Subject: `svc-workflow` Product Direction and Architecture
- Repository/source: `mayf3/svc-workflow`
- Commit/artifact: `8cda3d05e1c22814b7aeaace97d317380df83836`
- Environment: repository source tree
- Observed at: `2026-08-19T14:33:51Z`
- Method: inspect `PRODUCT-BOUNDARY.md`, `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md`, and `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_2.md`.
- Result: the repository already has an active product boundary, a frozen core Architecture, and an effective bounded Cancel / Archive refinement that this adoption must preserve rather than replace.
- Provenance: the three named files at the consumer base commit.

### OBS-ADOPT-004 — No governance adoption is active in the base

- Subject: consumer governance adoption surface
- Repository/source: `mayf3/svc-workflow`
- Commit/artifact: `8cda3d05e1c22814b7aeaace97d317380df83836`
- Environment: repository source tree
- Observed at: `2026-08-19T14:33:51Z`
- Method: inspect the base tree for `AGENTS.md`, `.agents/governance.lock.json`, vendored governance protocol, local authority declaration, and `docs/specs/` index.
- Result: the exact-commit consumer adoption surfaces defined by the upstream protocol are absent from the base.
- Provenance: consumer base tree and changed-path inventory for this docs-only bootstrap.

### OBS-ADOPT-005 — Automated enforcement is not currently a protected merge gate

- Subject: `main` branch enforcement
- Repository/source: GitHub branch settings for `mayf3/svc-workflow`
- Commit/artifact: `main @ 8cda3d05e1c22814b7aeaace97d317380df83836`
- Environment: live GitHub repository settings
- Observed at: `2026-08-19T14:33:51Z`
- Method: inspect the default branch metadata and protection status.
- Result: `main` is not protected and has no required status-check enforcement; therefore this adoption can truthfully claim only manual policy plus available distribution-integrity tooling.
- Provenance: GitHub branch metadata for `main` at the observation time.

## 6. Claims and assumptions

### CLM-ADOPT-001 — Exact vendoring preserves distribution revision identity

- Support state: SUPPORTED
- Supported by evidence: `EVD-ADOPT-001`
- Contradicted by evidence: none known
- Uncertainty: source Git blob equality proves exact distributed bytes; local files and local acceptance remain separate consumer-owned content and actions.

### CLM-ADOPT-002 — Forward-only adoption is compatible with existing authority

- Support state: SUPPORTED
- Supported by evidence: `EVD-ADOPT-002`
- Contradicted by evidence: none known
- Uncertainty: each future change must still reconcile the exact legacy authorities it touches; this adoption does not pre-classify every historical document.

### CLM-ADOPT-003 — Manual-policy labeling is the only honest initial enforcement claim

- Support state: SUPPORTED
- Supported by evidence: `EVD-ADOPT-003`
- Contradicted by evidence: none known
- Uncertainty: repository settings may change later and require a new observation and local-governance update.

## 7. Evidence relations

### EVD-ADOPT-001 — Source and target identities support exact vendoring

- Source observations: `OBS-ADOPT-001`, `OBS-ADOPT-002`
- Target: `CLM-ADOPT-001`
- Relation: SUPPORTS
- Bound coordinates: source `46f78c3f00d768d99a4c8c2da975b124bce042f9`, consumer base `8cda3d05e1c22814b7aeaace97d317380df83836`, observed `2026-08-19T14:33:51Z`
- Strength/sufficiency: strong for exact distributed-file byte identity and source revision selection
- Limitations: does not perform local semantic review or acceptance
- Provenance: manifest, source blob identities, target blob responses, and proposed governance lock.

### EVD-ADOPT-002 — Authority inventory supports forward-only compatibility

- Source observations: `OBS-ADOPT-003`, `OBS-ADOPT-004`
- Target: `CLM-ADOPT-002`, `STATE-ADOPT-002`
- Relation: SUPPORTS
- Bound coordinates: consumer base `8cda3d05e1c22814b7aeaace97d317380df83836`, observed `2026-08-19T14:33:51Z`
- Strength/sufficiency: strong for preserving the three named authorities and avoiding bulk historical reclassification
- Limitations: future Specs must still identify additional relevant legacy authorities individually
- Provenance: named local authority files, local governance map, and base-tree inventory.

### EVD-ADOPT-003 — Live settings support honest manual enforcement labeling

- Source observations: `OBS-ADOPT-005`
- Target: `CLM-ADOPT-003`
- Relation: SUPPORTS
- Bound coordinates: GitHub `main`, observed `2026-08-19T14:33:51Z`
- Strength/sufficiency: direct for current branch-protection and required-check state
- Limitations: point-in-time repository setting only
- Provenance: GitHub branch metadata.

## 8. Decisions

### DEC-ADOPT-001 — Adopt the exact vendored distribution

- Decision owner: repository owner `mayf3`
- Decision: prepare the exact distribution at source commit `46f78c3f00d768d99a4c8c2da975b124bce042f9` using a proposed local lock and ordinary Git review.
- Rejected alternatives: floating `main`, `latest`, runtime fetch, and a submodule required for every Agent session.
- Reason: exact bytes exist in every clone, implementation bases can contain the governing rules, updates are explicit diffs, and upstream movement cannot silently change local authority.
- Owner decision remaining: none; semantic adoption review and the explicit acceptance act remain process steps, not unresolved design choices.

### DEC-ADOPT-002 — Preserve repository-owned product authority

- Decision owner: repository owner `mayf3`
- Decision: the shared distribution governs development expression and workflow only; local Product Direction, Architecture, accepted Specs, and authorized maintainers remain authoritative for `svc-workflow`.
- Rejected alternative: treating the central governance repository as remote product authority.
- Reason: cross-repository ownership boundaries and explicit local acceptance.
- Owner decision remaining: none.

### DEC-ADOPT-003 — Apply governance forward-only

- Decision owner: repository owner `mayf3`
- Decision: use the grammar from the next non-mechanical change forward and reconcile historical artifacts only when they become governing, are cited, or conflict with active authority.
- Rejected alternative: bulk rewriting all historical Architecture, Contract, audit, and investigation material into the new format.
- Reason: preserve history, avoid false authority changes, and learn from real Spec cycles before expanding machinery.
- Owner decision remaining: none.

### DEC-ADOPT-004 — Label enforcement according to reality

- Decision owner: repository owner `mayf3`
- Decision: initial enforcement is manual policy with deterministic vendored-byte integrity verification; no full syntax gate, base-branch gate, branch protection, or semantic CI is claimed.
- Rejected alternative: describing available instructions or schemas as an unbypassable merge gate.
- Reason: truthful enforcement is itself a governance invariant.
- Owner decision remaining: none.

## 9. Contracts

### CTR-ADOPT-001 — Exact source revision

The repository MUST identify the governance distribution by exact 40-hex source commit and MUST record that commit in `.agents/governance.lock.json`. A floating branch, tag name without resolved commit, or `latest` reference MUST NOT activate local governance.

### CTR-ADOPT-002 — Exact distributed bytes

Every path listed in the distribution manifest MUST match the locked size and SHA-256. Vendored distribution files MUST NOT be edited locally outside an explicit governance update or recovery process.

### CTR-ADOPT-003 — Truthful adoption state

The authoring snapshot MUST remain `adoption.status: proposed` with null `accepted_by` and `accepted_at`. Only the authorized local acceptance actor MAY prepare an accepted lock after independent review and final-head binding.

### CTR-ADOPT-004 — Local authority ownership

`.agents/local/README.md` MUST preserve and identify repository-owned Product Direction, Architecture, Spec acceptance actors, mechanical-exemption reviewers, emergency actors, and durable investigation/conformance locations. The external governance repository MUST NOT be represented as owning `svc-workflow` product behavior.

### CTR-ADOPT-005 — Forward-only historical bridge

Adoption MUST NOT bulk reclassify, demote, accept, supersede, or rewrite historical documents. Existing authorities retain only their previously established status and scope. Future work MUST resolve relevant legacy authority explicitly.

### CTR-ADOPT-006 — Explicit update and rollback

No upstream governance change MAY alter this repository until a separate docs-only consumer update is pinned, reviewed, accepted, and merged. Rollback MUST restore the previous lock and distributed bytes together by reverting the complete adoption or update commit.

### CTR-ADOPT-007 — Honest enforcement

Repository documentation MUST distinguish manual policy, distribution integrity, syntax tooling, base-branch enforcement, semantic review, required checks, and branch protection according to their actual state.

### CTR-ADOPT-008 — No product implementation authority

Acceptance of this adoption Spec MUST NOT authorize Rust, SQL, schema, API, SDK, runtime, deployment, or other product implementation. Future non-mechanical implementation requires a separate accepted Spec with `implementation_authority: contracts` already present in its base.

## 10. Acceptance

### ACC-ADOPT-001 — Source and manifest identity

- Contracts: `CTR-ADOPT-001`
- Method: compare the lock source repository/commit/version/manifest digest with the exact upstream commit and manifest.
- Environment: adoption candidate branch
- Inputs/configuration: upstream `46f78c3f00d768d99a4c8c2da975b124bce042f9`
- Required evidence: upstream branch/commit record, manifest content and SHA-256, and proposed lock
- Expected result: all source identities are exact and no floating identifier activates governance
- Failure condition: any source field is mutable, unresolved, or mismatched.

### ACC-ADOPT-002 — Vendored integrity

- Contracts: `CTR-ADOPT-002`
- Method: run `python3 .agents/tools/verify_governance.py --target .` and independently compare the candidate diff with the upstream manifest.
- Environment: clean checkout of the exact adoption candidate head
- Inputs/configuration: proposed governance lock and 17 vendored files
- Required evidence: executed verifier output, candidate head, and changed-path inventory
- Expected result: verifier exits zero and all distributed files match the lock
- Failure condition: missing file, extra locked path, size mismatch, digest mismatch, unsafe path, or invalid adoption metadata.

### ACC-ADOPT-003 — Proposed and accepted state separation

- Contracts: `CTR-ADOPT-003`
- Method: inspect the authoring lock, independent review record, authorized acceptance transition, and final-head recheck.
- Environment: adoption PR before and after authorized finalization
- Inputs/configuration: exact reviewed and final candidate commits
- Required evidence: proposed lock, reviewer identity and recommendation, accepted lock, acceptance actor, accepted time, and semantic-delta comparison
- Expected result: authoring does not fabricate acceptance; final accepted head has `SEMANTIC_DELTA_AFTER_REVIEW = NONE`
- Failure condition: non-null acceptance metadata during authoring, unauthorized acceptance, or unreviewed semantic delta.

### ACC-ADOPT-004 — Local authority completeness

- Contracts: `CTR-ADOPT-004`, `CTR-ADOPT-005`
- Method: review `.agents/local/README.md` against the three named local authorities, legacy locations, actors, persistence locations, and forward-only rules.
- Environment: exact adoption candidate head
- Inputs/configuration: consumer base and proposed local files
- Required evidence: `OBS-ADOPT-003`, `OBS-ADOPT-004`, local authority map, and independent reviewer finding
- Expected result: local ownership is explicit, existing authority is preserved, and no directory or recency shortcut silently creates authority
- Failure condition: a required actor or authority is missing, central governance is given product ownership, or historical documents are bulk reclassified.

### ACC-ADOPT-005 — Update and rollback isolation

- Contracts: `CTR-ADOPT-006`
- Method: demonstrate that upstream movement leaves the consumer tree unchanged; inspect a simulated separate update diff and complete Git revert boundary.
- Environment: temporary or review checkout pinned to the adoption candidate
- Inputs/configuration: current lock and a distinct upstream revision or controlled fixture
- Required evidence: before/after consumer tree identities, update plan, and rollback plan
- Expected result: only a consumer commit changes local governance, and a complete revert restores the prior snapshot
- Failure condition: upstream movement mutates local files without a consumer commit or rollback separates the lock from vendored bytes.

### ACC-ADOPT-006 — Enforcement claims match repository reality

- Contracts: `CTR-ADOPT-007`
- Method: compare `.agents/local/README.md` with live GitHub branch protection, required checks, repository workflows, and available tools.
- Environment: repository files and live GitHub settings
- Inputs/configuration: exact adoption candidate and observation time
- Required evidence: branch metadata, workflow inventory, local enforcement table, and reviewer finding
- Expected result: only manual policy and available distribution-integrity tooling are claimed initially
- Failure condition: an absent syntax gate, base gate, required check, branch protection rule, or semantic CI is described as active.

### ACC-ADOPT-007 — Product implementation remains blocked

- Contracts: `CTR-ADOPT-008`
- Method: inspect this Spec frontmatter, PR changed paths, and any attempted implementation preflight.
- Environment: adoption candidate and the first subsequent non-mechanical request
- Inputs/configuration: `implementation_authority: none`
- Required evidence: frontmatter, changed-path inventory, and PREFLIGHT output
- Expected result: this PR changes governance/docs only, and subsequent product implementation remains blocked until separately authorized
- Failure condition: product code is included in this PR or this Spec is cited as product implementation authority.

### Contract coverage

| Contract | Acceptance | Evidence class | Covered |
|---|---|---|---|
| `CTR-ADOPT-001` | `ACC-ADOPT-001` | repository identity | YES |
| `CTR-ADOPT-002` | `ACC-ADOPT-002` | deterministic verifier | YES |
| `CTR-ADOPT-003` | `ACC-ADOPT-003` | review and acceptance record | YES |
| `CTR-ADOPT-004` | `ACC-ADOPT-004` | semantic authority review | YES |
| `CTR-ADOPT-005` | `ACC-ADOPT-004` | semantic authority review | YES |
| `CTR-ADOPT-006` | `ACC-ADOPT-005` | update / rollback demonstration | YES |
| `CTR-ADOPT-007` | `ACC-ADOPT-006` | files plus live settings | YES |
| `CTR-ADOPT-008` | `ACC-ADOPT-007` | frontmatter and diff | YES |

## 11. Alternatives and disposition

### ALT-ADOPT-001 — Copy a simplified local governance document

- Disposition: rejected
- Reason: it would fork semantics immediately, lose exact upstream identity, and make future updates manual semantic merges.
- Evidence/Claims considered: `CLM-ADOPT-001`
- What would reopen: a future decision to permanently fork the shared grammar under a new local governance authority.

### ALT-ADOPT-002 — Follow upstream `main` dynamically

- Disposition: rejected
- Reason: upstream changes could alter local development authority without a consumer review or base-branch commit.
- Evidence/Claims considered: `CLM-ADOPT-001`
- What would reopen: not under V0; any dynamic model would require an explicit new trust and update authority.

### ALT-ADOPT-003 — Use a Git submodule

- Disposition: rejected
- Reason: Agents would depend on submodule initialization, the governing bytes would not be ordinary local diff content, and implementation bases could omit the initialized authority.
- Evidence/Claims considered: `DEC-ADOPT-001`
- What would reopen: reliable mandatory submodule initialization and a proven simpler authority workflow across all Agent environments.

### ALT-ADOPT-004 — Bulk migrate all historical documents now

- Disposition: rejected
- Reason: reformatting could silently change authority, fabricate acceptance, erase provenance, and create work unrelated to the next real decision.
- Evidence/Claims considered: `CLM-ADOPT-002`, `STATE-ADOPT-002`
- What would reopen: a bounded future Spec identifies a concrete conflict or query requirement that cannot be solved forward-only.

### ALT-ADOPT-005 — Add full CI and branch protection in this bootstrap

- Disposition: deferred
- Reason: the upstream distribution supplies integrity tooling and schemas but does not implement a full semantic or base-branch gate; enforcement must not be overstated. Two or three real Spec cycles should expose the smallest useful deterministic checks.
- Evidence/Claims considered: `CLM-ADOPT-003`
- What would reopen: after the first real Specs provide fixtures and the repository owner explicitly authorizes required checks and branch protection.

## 12. Migration, compatibility, and rollback

```text
MIGRATION = forward-only governance adoption
HISTORICAL_REWRITE = none
PRODUCT_DATA_MIGRATION = none
PRODUCT_RUNTIME_CHANGE = none
COMPATIBILITY = existing local authorities retain their established status and scope
UPDATE = separate docs-only exact-revision adoption PR
ROLLBACK = revert the complete adoption or update commit, including lock and vendored bytes
EMERGENCY_CONTAINMENT = not applicable to this docs-only bootstrap
```

## 13. Open questions

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
INDEPENDENT_REVIEW_PENDING = YES
AUTHORIZED_ACCEPTANCE_PENDING = YES
READY_TO_MARK_ACCEPTED = YES
```

The remaining work is procedural: independent review of the exact candidate, authorized preparation of the accepted lock and Spec status, and an independent final-head recheck. It is not delegated product design.
