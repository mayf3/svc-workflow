---
spec_id: REPOSITORY_DEVELOPMENT_GOVERNANCE_ADOPTION_V1
status: proposed
spec_kind: invariant
authority_level: governing_spec
implementation_authority: none
scope:
  - owner/repository
governed_by: []
external_authorities:
  - repository: mayf3/agent-development-governance
    authority_id: AGENT_DEVELOPMENT_GOVERNANCE_BOOTSTRAP_V0
    revision: REPLACE_WITH_40_HEX_SOURCE_COMMIT
    relation: constrained_by
supersedes: []
superseded_by: null
owners:
  - repository-maintainers
---

# REPOSITORY_DEVELOPMENT_GOVERNANCE_ADOPTION_V1

## 1. Goal

Adopt an exact revision of the shared Development Grammar and Spec-governance
distribution while preserving this repository's local product and acceptance
authority.

## 2. Scope and non-goals

In scope:

- vendored governance bytes and lock;
- local authority precedence and acceptance actors;
- forward-only application to future non-mechanical work;
- explicit future update and rollback process.

Out of scope:

- product behavior changes;
- bulk migration of historical documents;
- claiming semantic CI or branch protection that is not active.

## 3. Authority and dependencies

```text
SOURCE_REPOSITORY = mayf3/agent-development-governance
SOURCE_COMMIT = <40-hex>
DISTRIBUTION_VERSION = <version>
MANIFEST_SHA256 = <sha256>
LOCAL_ACCEPTANCE_ACTOR = <role or identity>
```

The external distribution supplies grammar and protocol content. It does not own
this repository's Product Direction, Architecture, Specs, code, or acceptance
actions.

## 4. Current State

### STATE-ADOPT-001 — Existing local governance and enforcement

- Subject: this repository's governance surface
- As of commit: `<consumer-base-commit>`
- Environment: repository default authority branch and GitHub settings
- Observed at: `<timestamp>`
- Projection: `<existing .agents files, governing authorities, manual/automated gates, and known conflicts>`
- Basis: `OBS-ADOPT-004`, direct repository provenance, and any necessary Claim IDs

## 5. Observations

### OBS-ADOPT-001 — Source checkout is clean and exact

- Subject: governance distribution source
- Source revision: `<40-hex source commit>`
- Environment: local source checkout
- Observed at: `<timestamp>`
- Method: `git rev-parse HEAD` plus clean-worktree check
- Result: `<exact HEAD and clean/dirty result>`
- Provenance: `<command output or persistent record>`

### OBS-ADOPT-002 — Default vendor invocation produces a bounded no-write plan

- Subject: proposed consumer adoption
- Source revision: `<40-hex source commit>`
- Environment: clean consumer worktree at `<consumer-base-commit>`
- Observed at: `<timestamp>`
- Method: run `python3 tools/vendor.py --target <consumer-path>
  --source-commit <40-hex> --prepared-by <actor>`, then inspect the `DRY-RUN`
  file-operation plan
- Result: `<planned files to create/update, no-files-written confirmation, and any conflict result>`
- Provenance: `<captured DRY-RUN output>`

### OBS-ADOPT-003 — Lock and verifier bind the vendored bytes

- Subject: `.agents/governance.lock.json` and vendored distribution
- Source revision: `<40-hex source commit>`
- Environment: proposed consumer worktree
- Observed at: `<timestamp>`
- Method: inspect lock and run `.agents/tools/verify_governance.py`
- Result: `<lock identity, file digests, verifier result>`
- Provenance: `<lock path and executed verifier output>`

### OBS-ADOPT-004 — Local authority inventory and enforcement are recorded

- Subject: consumer authority map
- Source revision: `<consumer-base-commit>`
- Environment: repository files and GitHub settings
- Observed at: `<timestamp>`
- Method: inspect Product Direction, Architecture/invariants, accepted Specs,
  acceptance roles, branch protection, and required checks
- Result: `<bounded inventory and actual enforcement state>`
- Provenance: `<file paths, settings evidence, and query output>`

## 6. Claims and assumptions

### CLM-ADOPT-001 — Exact vendoring preserves local revision identity

- Support state: SUPPORTED | INFERRED | OPEN_ASSUMPTION
- Supported by evidence: `EVD-ADOPT-001`
- Contradicted by evidence: `<NONE or IDs>`
- Uncertainty: `<limits of the dry-run and verifier evidence>`

### CLM-ADOPT-002 — Forward-only adoption is compatible with this repository

- Support state: SUPPORTED | INFERRED | OPEN_ASSUMPTION
- Supported by evidence: `EVD-ADOPT-002`
- Contradicted by evidence: `<NONE or IDs>`
- Uncertainty: `<legacy authority or workflow risks>`

## 7. Evidence relations

### EVD-ADOPT-001 — Source, dry-run, lock, and verifier support exact adoption

- Source observations: `OBS-ADOPT-001`, `OBS-ADOPT-002`, `OBS-ADOPT-003`
- Target: `CLM-ADOPT-001`
- Relation: SUPPORTS
- Bound coordinates: source `<40-hex>`, consumer base `<40-hex>`, observed `<timestamp>`
- Strength/sufficiency: `<sufficiency for exact-byte and lock identity>`
- Limitations: does not itself prove local semantic acceptance
- Provenance: `<command outputs, planned file set, lock, and verifier record>`

### EVD-ADOPT-002 — Local inventory supports forward-only compatibility

- Source observations: `OBS-ADOPT-004`
- Target: `CLM-ADOPT-002` or `STATE-ADOPT-001`
- Relation: SUPPORTS
- Bound coordinates: consumer base `<40-hex>`, repository settings observed `<timestamp>`
- Strength/sufficiency: `<sufficiency for the bounded repository>`
- Limitations: future repository changes require a new evaluation
- Provenance: `<authority inventory and settings evidence>`

## 8. Decisions

### DEC-ADOPT-001 — Adopt exact vendored governance

- Decision owner: `<authorized local acceptance actor>`
- Decision: adopt the exact source commit recorded above.
- Rejected alternatives: floating `main`, `latest`, implicit remote authority,
  and an uninitialized default submodule.
- Reason: local bytes, visible diffs, exact base-branch identity, and explicit updates.
- Owner input remaining: `<NONE or specific decision>`

### DEC-ADOPT-002 — Preserve local product authority

- Decision owner: `<authorized local acceptance actor>`
- Decision: local Product Direction, Architecture, accepted Specs, and authorized
  maintainers remain authoritative for this repository.
- Rejected alternative: central repository automatically governs consumer product behavior.
- Reason: repository ownership and cross-repository authority boundaries.
- Owner input remaining: `<NONE or specific decision>`

## 9. Contracts

### CTR-ADOPT-001 — Exact revision

The repository MUST vendor the distribution from the exact source commit recorded
in `.agents/governance.lock.json`. Floating references MUST NOT activate governance.

### CTR-ADOPT-002 — Truthful adoption state

A prepared snapshot MUST remain `adoption.status: proposed` with null acceptance
metadata. Only the authorized local acceptance action MAY set
`adoption.status: accepted`.

### CTR-ADOPT-003 — Local authority map

`.agents/local/README.md` MUST identify Product Direction,
Architecture/invariants, Spec acceptance actors, mechanical-exemption reviewers,
emergency actors, and persistent investigation/conformance locations.

### CTR-ADOPT-004 — Explicit updates

No upstream change MAY alter local governance until a separate docs-only update
is reviewed, accepted, and merged in this repository.

### CTR-ADOPT-005 — Honest enforcement

The repository MUST represent manual policy, integrity checks, syntax gates,
branch protection, and semantic review according to their actual implemented state.

## 10. Acceptance

### ACC-ADOPT-001 — Exact source and vendored bytes

- Contracts: `CTR-ADOPT-001`
- Method: compare clean source `HEAD`, lock source commit, manifest, and vendored file digests
- Environment: clean source checkout and proposed consumer worktree
- Required evidence: `OBS-ADOPT-001`, `OBS-ADOPT-002`, `OBS-ADOPT-003`, lock, manifest, and diff
- Expected result: all identities match the exact source commit and no floating reference activates governance
- Failure condition: any commit or digest mismatch, dirty source ambiguity, or mutable reference is treated as adoption identity

### ACC-ADOPT-002 — Truthful proposed and accepted states

- Contracts: `CTR-ADOPT-002`
- Method: inspect proposed lock, authorized acceptance record, and accepted lock transition
- Environment: adoption PR before and after the explicit acceptance action
- Required evidence: proposed lock, reviewer record, acceptance actor identity, accepted lock, and final head
- Expected result: proposed metadata remains null; accepted metadata appears only after authorized finalization
- Failure condition: preparation claims acceptance or an unauthorized actor finalizes the lock

### ACC-ADOPT-003 — Local authority map completeness

- Contracts: `CTR-ADOPT-003`
- Method: review `.agents/local/README.md` against repository authority and role inventory
- Environment: exact adoption candidate commit
- Required evidence: `OBS-ADOPT-004`, local governance file, and reviewer finding
- Expected result: every required authority, actor, and persistence location is explicit and repository-owned
- Failure condition: a required authority or actor is absent, ambiguous, or delegated to the central repository

### ACC-ADOPT-004 — Explicit update and rollback boundary

- Contracts: `CTR-ADOPT-004`
- Method: change the upstream checkout without changing the consumer, then simulate a separate vendored update and revert
- Environment: temporary consumer repository pinned to the adoption commit
- Required evidence: before/after consumer tree identities, update diff, and rollback diff
- Expected result: upstream movement has no effect until a consumer commit; reverting that commit restores the prior pin
- Failure condition: consumer governance changes without a consumer commit or cannot be restored by reverting the complete update

### ACC-ADOPT-005 — Enforcement claims match reality

- Contracts: `CTR-ADOPT-005`
- Method: compare README/local governance claims with workflows, branch protection, required checks, and semantic review process
- Environment: repository files plus live GitHub settings at `<timestamp>`
- Required evidence: workflow files, settings/query evidence, and semantic review record
- Expected result: manual and automated enforcement are separately and truthfully described
- Failure condition: an unimplemented parser, required check, branch protection rule, or semantic gate is claimed as active

### Contract coverage

| Contract | Acceptance | Covered |
|---|---|---|
| `CTR-ADOPT-001` | `ACC-ADOPT-001` | YES |
| `CTR-ADOPT-002` | `ACC-ADOPT-002` | YES |
| `CTR-ADOPT-003` | `ACC-ADOPT-003` | YES |
| `CTR-ADOPT-004` | `ACC-ADOPT-004` | YES |
| `CTR-ADOPT-005` | `ACC-ADOPT-005` | YES |

## 11. Alternatives and disposition

Record materially considered floating branch, package, submodule, or local-copy
alternatives. For each, record disposition, reason, Evidence/Claims considered,
and what would reopen the option.

## 12. Migration, compatibility, and rollback

```text
MIGRATION = forward-only
HISTORICAL_REWRITE = none
ROLLBACK = revert the complete adoption or update commit

SUCCESSOR_PREPARATION =
  current adoption remains accepted with superseded_by = null
  proposed successor may declare supersedes = [current adoption]

SUCCESSOR_ACCEPTANCE =
  successor becomes accepted
  predecessor becomes superseded
  predecessor.superseded_by names successor
  both lifecycle directions change atomically in the final accepted Head
```

## 13. Open questions

```text
OPEN_OWNER_DECISIONS = <NONE or list>
NORMATIVE_TBD = <NONE or list>
UNRESOLVED_AUTHORITY_CONFLICT = <NONE or list>
PARTIAL_SUPERSESSION = <NONE or list>
READY_TO_MARK_ACCEPTED = NO
```
