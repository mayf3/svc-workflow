# Governing Specs for svc-workflow

New governing Specs live at stable paths:

```text
docs/specs/<SPEC_ID>.md
```

Read, in order:

1. `AGENTS.md`;
2. `.agents/README.md`;
3. `.agents/local/README.md`;
4. `.agents/protocol/SPEC_GOVERNANCE_V0.md` and `SPEC_FORMAT_V0.md` as needed;
5. the relevant Product Direction, Architecture, accepted Specs, and exact external authorities.

## Lifecycle and implementation authority

```text
Spec lifecycle = proposed | accepted | superseded
Implementation authority = none | contracts
Implementation progress, verification coverage, runtime state, and conformance are separate dimensions.
```

A non-mechanical implementation may begin only when an accepted Spec with `implementation_authority: contracts` is already present in the implementation PR base and its active Contracts cover the request.

After governance adoption is active, proposed governing Specs remain on docs-only PR branches. The `main` authority branch should contain only accepted or superseded governing Specs, except for the one-time adoption transition while this bootstrap PR is still under review.

## Existing-authority bridge

Adoption is forward-only and does not bulk-migrate legacy documents.

- `PRODUCT-BOUNDARY.md` remains the repository Product Direction authority in its declared scope.
- Explicitly frozen/effective Architecture remains authoritative in its declared scope.
- Existing documents under `docs/contracts/`, `contracts/`, and other historical locations retain only the status established by their own text and review history.
- Directory location or recency alone does not make a legacy document accepted authority.
- New work that relies on a legacy authority must pin its exact path and revision and explain its relation to the new Spec.
- Conflicting authorities block implementation; “newer wins” and prose-only partial supersession are forbidden.

## Spec index

| Spec ID | Status | Kind | Implementation authority | Scope | Supersedes |
|---|---|---|---|---|---|
| `SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1` | accepted | invariant | none | repository development process | none |
| `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` | accepted | implementation | contracts | one-time Principal successor migration | none |
| `SVC_WORKFLOW_ACTIVE_AGENT_LIST_V1` | superseded | implementation | contracts (historical; effective none) | canonical current-executor projection and global-list filter | none |
| `SVC_WORKFLOW_ACTIVE_AGENT_LIST_V2` | accepted | implementation | contracts | canonical current-executor projection and global-list filter with lawful exact closure | `SVC_WORKFLOW_ACTIVE_AGENT_LIST_V1` |
| `SVC_WORKFLOW_EXECUTION_CONTROL_V1` | accepted in this candidate; effective after merge | implementation | contracts | canonical Forum binding, RETURN projection, execution escalation, push kick, HUMAN_REQUIRED narrowing, durable Owner Assistance wake authentication | none |
| `SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V1` | accepted in candidate; reviewed semantic head `1f19115`; effective after both companion Specs merge | implementation | contracts; production apply none | one-layer immutable Assistance-case/governance-child binding, terminal-review wake, Owner-replacement outbox hook, exact accepted-submission resolve proof; existing unlinked cases and CTR-SWEC-010 unchanged | none |
| `SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1` | superseded by V2; historical acceptance retained | no longer current; historical authority metadata retained | preserved V1 contracts and acceptance provenance | none |
| `SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V2` | superseded by V3; historically accepted by Codex root under mayf3 delegation at reviewed semantic head `1ac73756`; paired Core head `8de41570`; repair review SHA `6b987dce…d49b37ce` | historical contracts; no longer current | preserves typed readiness and READY-only projection; authorizes the exact success-2xx two-Event pair, deterministic non-2xx zero-fact replay, serialized cardinality guard, and all-or-none omitted coordinates for unrecoverable `UNKNOWN / LINEAGE_INCONSISTENT` | `SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1` |
| [`SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V3`](SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V3.md) | superseded by accepted V4 at lifecycle commit pair; historical semantic head `ea8ec3f4fdb8d9e7b4b24c1136074dfb26f3e5e7`, paired Core `b9d9d808e02d7b34ad44dd70597f505b9b947ef7`; independent review SHA `02e1da3a…bb1183` | historical contracts; no longer current | immutable creation-time `admissionObservation` and existing GCR `prerequisiteReadiness`; current readiness/Scheduler remain live | `SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V2`
| [`SVC_WORKFLOW_OWNER_OBLIGATION_WORKFLOWS_V1`](SVC_WORKFLOW_OWNER_OBLIGATION_WORKFLOWS_V1.md) | accepted under ROOT_OWNER_ACCEPTANCE.json after independent semantic review | contracts after paired lifecycle commit pair is in implementation base; production apply none | one published intake GCR singleton plus independent review Workflow singleton over existing WAE/WEC/QA/RETURN; exact durable obligations/effects, policy publication, receipt-bound outcomes and closed STOP behavior | none |
| [`SVC_WORKFLOW_EXECUTION_CONTROL_V2`](SVC_WORKFLOW_EXECUTION_CONTROL_V2.md) | accepted as paired whole successor | contracts; production apply none | Svc-owned responsibility/effect reconciliation using actual bounded producers and existing command receipts | `SVC_WORKFLOW_EXECUTION_CONTROL_V1` |
| [`SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V2`](SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V2.md) | accepted as paired whole successor | contracts; production apply none | exact opted-in intake bootstrap and separate review singleton; existing ordinary GCR behavior preserved outside opt-in | `SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V1` |
| [`SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V2`](SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V2.md) | accepted as paired whole successor | contracts; production apply none | closed workflow operation/prerequisite gate, atomic Context binding at gated entry, existing WAE and RETURN limits | `SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1` |
| [`SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V4`](SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V4.md) | accepted as paired whole successor | contracts; production apply none | preserves `admissionObservation` and `prerequisiteReadiness`; live readiness and READY-only Scheduler projection remain unchanged | `SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V3` |

Update this index when a governing Spec is accepted or superseded. The table is a discovery aid; each Spec's exact frontmatter and revision remain authoritative.


| [`SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V2`](SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V2.md) | accepted by Codex root under mayf3 delegation as whole successor to V1; reviewed candidate `19593b4d`; effective after merge | contracts after Product V9 / Architecture V0_4_4 and paired prerequisite Specs are present in implementation bases; production apply none | preserves `ACTIONABLE_NOW` / `WAITING_FOR_TIME` time projection; normal opted-in dispatch additionally requires separate Svc prerequisite `READY`; no WEP `BLOCKED` status | `SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1` |
