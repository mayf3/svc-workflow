# H06 R2 mechanical-candidate preflight

Exact base and initial candidate: `59daec042786f0416a6a9061c0b638ba56c0d83e`.
Fresh `git ls-remote origin refs/heads/main` returns that same SHA.
Repository: `mayf3/svc-workflow`; isolated write surface:
`/private/tmp/svc-h06-manifest`, branch `ac-h06-migration-manifest`.
Single source writer: this H06 R2 turn; independent agents are read-only reviewers.
Persistent attributable mandate: `BRIEF-H06-MANIFEST-R2.md` and its inherited
`BRIEF-H06-MANIFEST.md`, explicitly supplied by the user on 2026-11-06.
R1's `STOPPED-H06-MANIFEST.md` remains unchanged historical evidence.

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
GOAL_OR_TARGET = extract one frozen manifest without changing either consumer
CURRENT_GAP = identical 86-tuples / 344 strings maintained in two source files
AUTHORITY_ACTION = REUSE
ROUTE_STAGE = IMPLEMENTATION (mechanical candidate preparation only)
AUTHORITY_ACCEPTED_IN_BASE = YES (governance and invariant authorities only)
IMPLEMENTATION_AUTHORITY = not_applicable (no semantic implementation permitted)
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = DURABLE
MUTATION_AUTHORIZATION = VALID (user H06 R2 mandate)
ISOLATED_WRITE_SURFACE = YES
CONTROLLED_RUNBOOK_REQUIRED = NO (no operation or database access)
SPEC_GAP_DEPENDENCY = NONE for byte-identical extraction; no fleet authority implied
EVIDENCE_REVIEWABILITY = PASS (tracked original snapshots, hashes, raw gate output)
LIVE_AUTHORITY_GAP = NONE asserted by this source-only extraction
IMPLEMENTATION_ALLOWED = YES for machine-proven mechanical candidate only
MERGE_READY = NO
OPERATION_ALLOWED = NO
NEXT_ACTION = CONTINUE
```

The accepted governance lock was introduced by `f4a591af81c2f40f330d8254fc2aaa9700270eb7`,
an ancestor of the base; deterministic integrity verification returned rc0.
Relevant accepted authority at base: Product V13 `CTR-V6-035..042`, Architecture
V0.4.7 `CTR-ARCH-026`, and adoption `CTR-ADOPT-008`. Product/Architecture and
adoption have no product implementation grant. The cutover child remains
`proposed / implementation_authority: none`, and is not implementation authority.

Independent read-only authority review found bounded mechanical preparation
eligible under `.agents/local/README.md`'s independent mechanical-exemption
review and final-maintainer-acceptance rules. Rust/module changes are
non-mechanical by default; eligibility here requires proven byte identity and
unchanged behavior. Reviewer recommendation is not maintainer acceptance.

Convergence disposition: RETAIN all historical mappings and both real consumers;
MIGRATE only their constant storage to one shared frozen source. The two
historical initializer snapshots are test evidence, never executable fallbacks.
No CTO mapping, dynamic identity query, SQL, control flow, CLI behavior, error,
plan artifact, general migration capability, production service, core repository,
or window tool is changed.

Evidence needed: pre-extraction exact initializer/string comparisons; restored
absolute fixture equals the base blob; shared constant equals both historical
snapshots and fixed SHA256; unchanged consumer bodies after removing declarations
and the new imports; all non-DB event_replay/admin_recovery/cutover tests rc0;
independent candidate review; exact candidate/base and remote readbacks.

Tier-1 includes cargo build, both relevant library filters, the pure recovery
golden test, two pure cutover tests and the new historical-bytes test. The 44
DB-backed recovery integration tests are NOT_RUN; their helper defaults to a
local database and applies migrations, so they cannot be run under this lane.
No broad integration filter or database configuration is used.

Tier-2 exact command: `scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh`.
Execution is explicitly forbidden this turn. State is `PENDING-DOCKER`; reported
daemon EOF root cause remains UNKNOWN. Requires Docker engine, postgres:16-alpine,
psql, Cargo/Rust and the exact frozen fixture. No merge before regulator's full
Tier-2 rc0 and authorized maintainer acceptance.

DONE_WHEN: Tier-1 rc0, exact-byte evidence and raw output in R2 receipt, commit,
push origin ac-h06-migration-manifest and gh PR referencing #1360 with explicit
Tier-2 pending marker. If gh times out, preserve committed work and stop.
EXPANSION_TRIGGER: any changed byte/behavior, required gate failure, authority
conflict, DB/service write, or need to alter a frozen fixture/harness. Stop and
write STOPPED-H06-MANIFEST-R2.md on failure; do not repair outside the mandate.
