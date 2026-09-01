---
authority_id: SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
status: accepted
authority_kind: product_direction
owning_repository: mayf3/svc-workflow
implementation_authority: none
production_apply_authority: none
supersedes:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_PRODUCT_BOUNDARY_V6

## 1. Goal and authority status

This document is the complete Owner-accepted Product Direction candidate for `svc-workflow`. It is a whole-authority successor to `SVC_WORKFLOW_PRODUCT_BOUNDARY_V5`, not an amendment and not a reader-side composition with V5.

```text
AUTHORITY_ID = SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
AUTHORITY_KIND = product_direction
STATUS = accepted
SUPERSEDES = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
PRODUCT_BOUNDARY_ACTION = SUPERSEDE
WHOLE_AUTHORITY_SUPERSESSION = YES
PARTIAL_SUPERSESSION = NONE
OWNER_USE_CASE = V5_COMPLETE_RESTATEMENT_PLUS_NODE_VISIT_CANONICAL_ACTIVATION_AND_DISPATCH_CUTOVER
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PRODUCT_DIRECTION_AUTHORIZES_IMPLEMENTATION_DIRECTLY = NO
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
CHILD_IMPLEMENTATION_SPEC_REQUIRED = YES
ARCHITECTURE_RECONCILIATION_REQUIRED = YES
```

V6 is Owner-accepted on the PR branch and remains non-active repository authority until this lifecycle-only final accepted candidate is independently rechecked and merged to `main`. The authorized atomic acceptance transaction has marked V6 accepted, marked V5 superseded with its `superseded_by` backlink, and switched `.agents/local/README.md` to V6 in the same docs-only change. V5 remains the accepted repository-active Product Direction on current `main` until that transaction is merged. This transaction does not directly authorize merge, implementation, database migration, production deployment, formal dispatch cutover, or production apply.

The Goal is to preserve every V5 boundary not explicitly changed below, including Domain isolation, immutable workflow facts, transition authority, audit, security, the split global permissions, the original CTO bounded successor exception, and the trusted-fleet exact-plan exception, while replacing V5's DRAFT/read-time-dispatchability direction with one canonical activation model:

```text
new Workflow node kinds = TASK | TERMINAL
TASK owner kinds = HUMAN | AGENT
Node Visit = the only runtime work unit
active non-terminal TASK Node Visit = exactly one canonical activation
HUMAN owner -> HUMAN_WORK_ITEM
AGENT owner -> DISPATCH_INTENT
TERMINAL -> no activation
Scheduler-facing wait primitive = nextEligibleAt only
```

The new normal path is driven by Node Visit activation. A Dispatch Intent is durable canonical work identity, not a read-time eligibility guess and not permission to start an Agent immediately. Scheduler policy, Agent mapping, delivery, Session selection, and attempt-scoped resource leases remain outside Workflow ownership. V6 permits only the bounded Scheduler-facing `nextEligibleAt` wait contract and controlled early wake by setting it to `now`; it does not create a general Scheduler, event platform, or Operator.

## 2. Scope and non-goals

### 2.1 In scope

V6 governs:

- the serial workflow engine, its immutable facts, Domain isolation, worklists, Definition and Instance lifecycle, Transition boundary, idempotency, and failure behavior;
- the two independent global permissions `GLOBAL_SCHEDULER_READ` and `GLOBAL_DOMAIN_ADMIN`;
- one dedicated administrative Agent Principal as the daily runtime actor in a single-user deployment;
- a repository-owned docs-only designation root for the exact Agent Principal, Client, and granted split permissions;
- direct-token execution by that Agent Principal;
- one exact Feishu app/tenant/conversation/sender command ingress as provenance and an Agent-core gate, never as `svc-workflow` authority;
- durable audit using the actual authenticated Agent Principal;
- capability-scoped child-authority sequencing and current conformance debt;
- the original CTO bounded one-time Principal successor migration already added to V2;
- the additive exact-plan-bound trusted fleet bounded successor exception in §17A, frozen to 86 exact successor pairs by `PLAN_SHA256`;
- the new-traffic `TASK | TERMINAL` node model and `HUMAN | AGENT` TASK-owner restriction;
- Node Visit as the only runtime work-unit identity, including repeated entry to the same definition `nodeId` under distinct `nodeVisitId` values;
- the exactly-one canonical activation invariant and the `HUMAN_WORK_ITEM | DISPATCH_INTENT` split;
- the Scheduler-facing `nextEligibleAt` contract, controlled early wake, retry/timeout/unknown-outcome boundaries, attempt-scoped leases, and repair-only scan boundary;
- the one-way cutover rule for new traffic and the bounded Legacy drain/migrate/manual-terminate/read-only-history modes;
- cross-repository ownership and local interoperability acceptance conditions for future `dsh-agent-core` integration; PR #87 remains only a fixed-coordinate non-authoritative observation/provenance reference.

### 2.2 Explicit non-goals

This Product Direction does not create or select an Agent, Principal UUID, Client, credential, permission Grant, designation root instance, database row, migration, API, HTTP Contract, OpenAPI surface, SDK, test, deployment, production change, auth-service change, or dsh-agent-core change. This lifecycle transaction accepts only V6; it does not merge V6 and does not accept, merge, mark Ready, or activate any child authority or external PR. It does not commit the frozen fleet plan artifact into this repository, and the frozen plan is not live database truth.

It does not add `HUMAN_TASK`, `AGENT_TASK`, `SERVICE_TASK`, `WAIT_EVENT`, or `WAIT_TIMER`; parallel nodes; dynamic forward branching; claim/pull assignment; ordinary reassignment; handoff; delegation; workflow-syntax timers; external-signal nodes; SLA orchestration; arbitrary script guards; built-in LLM execution; cross-Domain shared templates; in-flight template replacement; in-flight Domain transfer; physical Instance deletion; unrestricted global workflow content access; or a runtime break-glass grant.

It does not build Kafka, an Outbox platform, a generic event platform, a GitHub App, WORM storage, WebAuthn, a generic Operator, or a cross-repository rewrite. It does not delete the auth-service `SERVICE` Principal type. It does not create a Scheduler/Dispatcher implementation, Agent Session, message transport, notification ledger, fairness/priority/quota policy, Principal-to-Agent mapping, or node-syntax resource lock. It does not change Grants, role bindings, credentials, allowlists, production data, or runtime. Creating a Dispatch Intent MUST NOT itself start an Agent.

Long-term multi-Human governance is deferred, not forbidden. It may be introduced only by a lawful later Product Direction successor or an independent higher authority; no implementation or runtime configuration may silently reinterpret V6 to add it.

## 3. Authority and exact coordinates

```text
SVC_WORKFLOW_BASE_REF = github/main
SVC_WORKFLOW_BASE_COMMIT = c90d54cace46ff505ac54aa6215587d812cf9a78
CURRENT_MAIN_PRODUCT_DIRECTION_AT_ACCEPTANCE = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
CURRENT_MAIN_PRODUCT_DIRECTION_ACCEPTED_HEAD = b3c6d797d3a79655a8fd5b1c63016600d4631036
CURRENT_MAIN_PRODUCT_DIRECTION_MERGE_COMMIT = c90d54cace46ff505ac54aa6215587d812cf9a78
OWNER_ACCEPTED_BRANCH_CANDIDATE_PRODUCT_DIRECTION = SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
CURRENT_DISPATCHABILITY_PROPOSAL = SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1
CURRENT_DISPATCHABILITY_PROPOSAL_PR = https://github.com/mayf3/svc-workflow/pull/19
CURRENT_DISPATCHABILITY_PROPOSAL_BASE = c90d54cace46ff505ac54aa6215587d812cf9a78
CURRENT_DISPATCHABILITY_PROPOSAL_HEAD = 0c63d35a6e1291e7187e693e2a0ed1fec231eaf2
CURRENT_DISPATCHABILITY_PROPOSAL_STATE = OPEN / DRAFT / UNMERGED / PROPOSED / NON_AUTHORITATIVE
CURRENT_DISPATCHABILITY_PROPOSAL_GOVERNED_BY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
CURRENT_DISPATCHABILITY_PROPOSAL_IMPLEMENTATION_AUTHORITY = contracts (inert while proposed)
CURRENT_DISPATCHABILITY_PROPOSAL_SEMANTICS = read-time query projection / non-persisted / no canonical activation / no nextEligibleAt
CURRENT_DISPATCHABILITY_SOURCE_CANDIDATE = af450aa39e446683b8ae2b2edf99c4febdcfb068
CURRENT_DISPATCHABILITY_DISPOSITION = REWRITE_REQUIRED_NOT_ACCEPTABLE_OR_IMPLEMENTABLE_FOR_THIS_GOAL
CURRENT_DISPATCHABILITY_PR_LIFECYCLE_AUTHORITY = repository owner mayf3
OPEN_LOCAL_PR_CENSUS_OBSERVED_AT = 2026-09-01
OPEN_LOCAL_PR_CENSUS_COUNT = 5
OPEN_LOCAL_PR_CENSUS = PR #7 | PR #9 | PR #13 | PR #19 | PR #20
OPEN_LOCAL_PR_LIFECYCLE_AUTHORITY = repository owner mayf3
GLOBAL_READER_SPEC = SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1
GLOBAL_READER_ACCEPT_COMMIT = ea9ab2df0da7e58328ce5018164a2d2b6d6c14a9
GLOBAL_READER_IMPLEMENTATION_MERGE = bf875c265843b3e07570a96b734051e9cfe27a43
PRIMARY_ARCHITECTURE = SVC_WORKFLOW_ARCHITECTURE_V0_3_1 / ARCHITECTURE_FROZEN
CANCEL_ARCHIVE_REFINEMENT = SVC_WORKFLOW_CANCEL_ARCHIVE_GOVERNANCE_V0_3_2 / EFFECTIVE
DSH_AGENT_CORE_PR_87 = https://github.com/mayf3/dsh-agent-core/pull/87
DSH_AGENT_CORE_PR_87_HEAD_OBSERVED = 4260911960f33c5b91c38403f002207f717f4187
DSH_AGENT_CORE_PR_87_STATUS_OBSERVED = proposed / open draft
BLOCKED_CHILD_PR = https://github.com/mayf3/svc-workflow/pull/9
BLOCKED_CHILD_HEAD = 3056263c3fc964a2b225720dd2b859b47e296c2e
FLEET_PLAN_SCHEMA = workflow_trusted_fleet_successor_plan_v2
FLEET_PLAN_SIZE_BYTES = 540472
FLEET_PLAN_SHA256 = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606
FLEET_ROSTER_SHA256 = f046d18f76da838ba94775af7c960d0ee548f2e392c22e6c7b0e3add36cb8e5f
FLEET_PLAN_CANONICAL = YES
FLEET_PLAN_MODE = READ_ONLY_CANONICAL_PLAN
FLEET_PLAN_SNAPSHOT_UTC = 2026-08-24T01:03:53.192875+00:00
```

Authority precedence remains Product Direction, then accepted Architecture/long-lived invariant authority, then accepted governing child Specs, then descriptive code/tests/runtime/operations. V6 changes no external repository authority. On this PR branch the atomic V6-accepted/V5-superseded/authority-map transaction is complete; V5 remains active on current `main` until the independently rechecked transition is merged.

The accepted one-time child authority `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` and both V5-retained successor exceptions remain governed through the byte-for-meaning restatement in §§17/17A. Open svc-workflow Draft PR #7 remains independent. PR #9 retains its existing disposition and is neither modified nor merged by this round. At the authoring census, PR #7 is open/draft/unmerged at base `9ba2d87...` / head `a7f8d26...`; it affects only its stated replay closure and remains independently governed. PR #9 is open/draft/unmerged at base `327b74f...` / head `3056263...` and retains `SUPERSEDED_BY_FLEET_LOCAL_CHILD` with no lifecycle action here.

Open svc-workflow Draft PR #13 at base `2ff81ae...` / head `83fd493...` is the proposed `SVC_WORKFLOW_DOMAIN_OWNER_INSTANCE_LIST_V1`, governed by V4 plus Architecture v0.3.1, with `implementation_authority: none` and `production_apply_authority: none`. It defines an independent owner-scoped read-only Domain Workflow Instance list and defines neither canonical activation nor `nextEligibleAt`. It is non-authoritative, does not compete with V6 or the canonical-activation direction, and receives no lifecycle action from V6; repository owner `mayf3` retains its lifecycle authority.

The current dispatchability proposal is svc-workflow PR #19 at base `c90d54c...` / head `0c63d35...`, with `af450aa...` recorded by that Spec as its source candidate/lineage. PR #19 is open, draft, unmerged, proposed, governed by accepted V5, and declares `implementation_authority: contracts`; that declaration is inert while proposed. Its complete current meaning remains a read-time query projection on the existing Domain/global lists, is non-persisted, and creates neither canonical activation nor `nextEligibleAt`. The current Spec is non-authoritative and MUST NOT be accepted or implemented as this Goal's child or canonical dispatch authority. Any useful query semantics retained in the future must be re-investigated and rewritten under accepted V6 and the later lawful Architecture/implementation authority, without competing with canonical activation. V6 records only the local authority inventory and Goal disposition: it does not modify, close, merge, accept, or otherwise control PR #19; repository owner `mayf3` retains its lifecycle authority. The accepted `SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1` remains valid for its current read-only route gate; it is not the new Scheduler dispatch feed and grants no new role.

The frozen v0.3.1 Architecture still defines `DRAFT | NORMAL | TERMINAL`, while V6 proposes `TASK | TERMINAL` for new traffic. V6 is higher Product Direction but does not silently rewrite Architecture. Before implementation, an independently accepted Architecture successor/refinement must reconcile that conflict, and an independently accepted implementation Spec with `implementation_authority: contracts` must then cover the exact code/data/wire changes.

`mayf3/dsh-agent-core` PR #87 at observed head `4260911...` is a fixed-coordinate snapshot of an external proposed document, not local or external accepted authority. It is retained only as non-authoritative observation/provenance for the recurring-scan and Scheduler-management shapes seen at that coordinate. This repository imposes no lifecycle action on that PR and does not edit, accept, amend, split, close, supersede, or merge it. Future interoperability is accepted locally only when external periodic recovery behavior is Reconciler-only and Scheduler management is separated from normal dispatch. The external repository may satisfy those conditions by amending PR #87, adopting a replacement Spec, or using another locally lawful authority; it exclusively governs its PR and authority lifecycle.

### 3.1 Authority inventory and de-duplication

| Candidate/authority | Lifecycle at authoring coordinates | Disposition for this Goal |
|---|---|---|
| `SVC_WORKFLOW_PRODUCT_BOUNDARY_V5` | accepted and active on current `main`; superseded candidate on this PR branch | atomically superseded by accepted V6 on this branch; remains active only until the independently rechecked lifecycle transaction is merged |
| PR #13 `SVC_WORKFLOW_DOMAIN_OWNER_INSTANCE_LIST_V1@83fd493...` | base `2ff81ae...`; open, draft, unmerged, proposed; V4 + Architecture v0.3.1; `implementation_authority: none`; `production_apply_authority: none` | independent non-authoritative owner-scoped read-only Domain list; no canonical activation or `nextEligibleAt`; not competing V6/canonical authority; no V6 lifecycle action; repository owner retains lifecycle authority |
| PR #19 `SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1@0c63d35...` | open, draft, unmerged, proposed, V5-bound; `implementation_authority: contracts` inert while proposed | non-authoritative; read-time/non-persisted/no canonical activation/no `nextEligibleAt`; not acceptable or implementable as this Goal's child/canonical dispatch authority; retained query semantics require later lawful re-investigation/rewrite |
| `af450aa...` | source candidate/lineage recorded by PR #19, not the current proposal head | provenance only; MUST NOT be misclassified as the sole current dispatchability proposal or used as implementation authority |
| `SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1` | accepted | preserve current read-only route compatibility; do not use as canonical dispatch authority |
| v0.3.1 Architecture + v0.3.2 refinement | frozen/effective | preserve unchanged scopes, but require later Architecture reconciliation for the new node/activation model |
| `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` | accepted | preserve exact bounded exception; no general migration capability |
| `SVC_WORKFLOW_TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1` | proposed at base, implementation already represented as V5 conformance history | preserve V5's exact-plan boundary and production-gate separation; do not broaden |
| dsh-agent-core PR #87 at `4260911...` | fixed-coordinate external observation; proposed/open at observation time; non-authoritative | provenance only; local interoperability requires Reconciler-only periodic recovery and Scheduler management separated from normal dispatch; external repository chooses its lawful authority/lifecycle path |

#### 3.1.1 Complete authoring-time open local PR census

This is an inventory/de-duplication census, not an assertion that every PR has the same authority type or relationship. GitHub metadata and each sole docs proposal were inspected at the exact heads below. For self-referential PR #20, `8189481...` is the exact pre-amendment head at census time; the final amended head is recorded in the PR record after commit because a commit cannot embed its own Git identity.

| PR | Exact base / observed head | Lifecycle at census | Current authority classification and disposition |
|---|---|---|---|
| #7 | `9ba2d87e94f6d39ffdd6986b5a434546cb91d90c` / `a7f8d26b7a8f57da773bd7b05879ee485841fa58` | OPEN, DRAFT, unmerged; sole Spec proposed | independently governed replay-closure amendment to the existing bounded successor child; declares `implementation_authority: contracts` but activation is pending amendment acceptance/merge and production apply remains unauthorized; affects only its stated replay closure; no V6 lifecycle action |
| #9 | `327b74f138151a7f4d9d88e3881e54d203f1e8f6` / `3056263c3fc964a2b225720dd2b859b47e296c2e` | OPEN, DRAFT, unmerged; sole Spec proposed | V3-bound with `implementation_authority: none` and `production_apply_authority: none`; retains V5/V6 disposition `SUPERSEDED_BY_FLEET_LOCAL_CHILD`; no modification, closure, merge, acceptance, or other V6 lifecycle action |
| #13 | `2ff81ae47ab068216bd0012fa0e76a45dd2fb572` / `83fd493db26c5e9b5b00d7e308da3c372c4d9ca4` | OPEN, DRAFT, unmerged; sole Spec proposed | V4 + Architecture v0.3.1; no implementation/production authority; independent owner-scoped read-only Domain list; no canonical activation/`nextEligibleAt`; non-authoritative and not competing with V6; no V6 lifecycle action |
| #19 | `c90d54cace46ff505ac54aa6215587d812cf9a78` / `0c63d35a6e1291e7187e693e2a0ed1fec231eaf2` | OPEN, DRAFT, unmerged; sole Spec proposed | V5-bound read-time/non-persisted query projection; `implementation_authority: contracts` inert while proposed; source lineage `af450aa...`; retains §3.1 non-reuse/rewrite disposition and no V6 lifecycle action |
| #20 | `c90d54cace46ff505ac54aa6215587d812cf9a78` / `818948189aa7f4eb326e16ca3e5725fceaf0394d` pre-amendment observation | OPEN, non-draft, unmerged; V6 proposed at census time | this was the self proposed Product Direction candidate at the fixed census; its reviewed semantic head and later lifecycle-only final accepted candidate Head are reported in the PR record because a commit cannot embed its own identity |

Every PR lifecycle above remains owned by repository owner `mayf3`. V6 performs no close, modify, merge, accept, or other lifecycle action on PR #7, #9, #13, or #19. PR #20 is now an Owner-accepted, unmerged branch candidate; it is not active repository authority until independent final-head recheck and merge.

No new implementation Spec, Contract bundle, Architecture file, or external-repository authority is created in this Phase. The reviewed V6 semantic document inventories/dispositions the complete open local PR census while preserving each proposal's distinct authority relationship and does not mutate or control another PR. This acceptance transaction adds only the authorized lifecycle transition across V6, V5 frontmatter/backlink, and the local authority map. The V6 authority is necessary because accepted V5 normative meaning must change and V0 forbids partial supersession.

External Draft PRs remain classified exactly as preserved by V5:

```text
AUTH_SERVICE_PR_15_DISPOSITION = KEEP_DRAFT_DEFERRED
AUTH_SERVICE_PR_15_REQUIRED_FOR_AGENT_FIRST_V1 = NO
AUTH_SERVICE_PR_15_ACTIVE_ON_MAIN = NO
AUTH_SERVICE_PR_2_BLOCKS_AGENT_ADMIN_ROUTE = NO
AUTH_SERVICE_PR_2_RELATION = INDEPENDENT_LEGACY_SHUTDOWN_PROGRAM
```

PR #15 is a possible future input to multi-Human governance, not an Agent-first V1 prerequisite. PR #2 is an independent legacy shutdown Program and MUST NOT become a common mutable gate.

## 4. Current State, Observations, Claims, and Evidence

All State below is descriptive. Drift does not rewrite this Product Direction.

### 4.1 Current State

- `STATE-V6-001` — On `mayf3/svc-workflow@c90d54cace46ff505ac54aa6215587d812cf9a78`, V5 is accepted and active; no V6 exists on the base. Basis: `OBS-V6-001`, `EVD-V6-001`.
- `STATE-V6-002` — V5 authorizes DRAFT plus ordinary nodes, a read-time non-persisted `dispatchable` projection, and explicitly no Scheduler/wake/lease/reservation state. That meaning does not permit this Goal's immediate canonical activation. Basis: `OBS-V6-002`, `EVD-V6-002`.
- `STATE-V6-003` — Frozen Architecture v0.3.1 defines `DRAFT | NORMAL | TERMINAL`, immutable repeated Node Visits, and `Human | Agent | Service` Principals; v0.3.2 adds Cancel/Archive without changing node semantics. Basis: `OBS-V6-003`, `EVD-V6-003`.
- `STATE-V6-004` — svc-workflow PR #19 at base `c90d54c...` / head `0c63d35...` is the current open, draft, unmerged, proposed `SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1`, governed by V5 and declaring `implementation_authority: contracts` inert while proposed. Its `af450aa...` coordinate is source candidate/lineage, not the current proposal head. The current Spec defines only a read-time, non-persisted query projection and creates no canonical activation or `nextEligibleAt`. Basis: `OBS-V6-004`, `EVD-V6-002`.
- `STATE-V6-005` — At the fixed `2026-09-01` observation, dsh-agent-core PR #87 head `4260911...` was an open proposed external PR whose recurring-scan shape did not satisfy this repository's activation-driven interoperability condition. That snapshot creates no present-state, lifecycle, or authority dependency. Basis: `OBS-V6-006`, `EVD-V6-005`.

### 4.2 Observations

#### OBS-V6-001 — Active repository authority and base

- Subject: `mayf3/svc-workflow` source tree and GitHub main.
- Source revision: `c90d54cace46ff505ac54aa6215587d812cf9a78`.
- Environment/observed at: authoring worktree and GitHub remote, `2026-09-01`.
- Method: fetch `github/main`; inspect HEAD, V5 frontmatter, `.agents/local/README.md`, and PR #18 merge coordinates.
- Result: requested base is current; V5 is accepted/active; governance adoption is active; worktree was clean and detached before the new branch.
- Provenance: repository Git refs, `docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V5.md`, `.agents/local/README.md`, GitHub PR #18.

#### OBS-V6-002 — Accepted V5 cannot express the frozen Goal

- Subject: accepted V5.
- Source revision: `c90d54cace46ff505ac54aa6215587d812cf9a78`.
- Method: complete document inspection, especially §§1-2, 5, 8, 8A, 19-24.
- Result: V5 retains DRAFT/new ordinary-node semantics; dispatchability is read-time and non-persisted; Scheduler, wake, lease/reservation, and persistent dispatch state are excluded. The frozen Goal requires the opposite canonical activation/wait model.
- Provenance: `docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V5.md`.

#### OBS-V6-003 — Frozen Architecture requires later reconciliation

- Subject: frozen/effective local Architecture.
- Source revision: `c90d54cace46ff505ac54aa6215587d812cf9a78`.
- Method: complete inspection of v0.3.1 and v0.3.2.
- Result: v0.3.1 freezes `DRAFT | NORMAL | TERMINAL`, assignee references, repeated immutable Node Visits, and transition/event/transaction semantics; v0.3.2 changes only Cancel/Archive governance. No canonical activation or `nextEligibleAt` exists.
- Provenance: `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md`, `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_2.md`.

#### OBS-V6-004 — Current dispatchability proposal is a duplicate-risk input, not authority

- Subject: svc-workflow PR #19 and `SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1`.
- Source revision: PR base `c90d54cace46ff505ac54aa6215587d812cf9a78`; current PR head `0c63d35a6e1291e7187e693e2a0ed1fec231eaf2`; source candidate/lineage `af450aa39e446683b8ae2b2edf99c4febdcfb068`.
- Environment/observed at: GitHub and an exact-head local Git object, `2026-09-01`.
- Method: inspect current PR metadata and completely inspect its sole Spec at the exact head; compare frontmatter, declared source candidate, semantics, acceptance state, and implementation gate.
- Result: PR #19 is open, draft, unmerged, proposed, governed by V5, and declares `implementation_authority: contracts`, which is inert while proposed. Its read-time query projection is non-persisted and creates no canonical activation or `nextEligibleAt`. The current Spec is non-authoritative and cannot be accepted or implemented as this Goal's child/canonical dispatch authority; `af450aa...` is lineage rather than the current head.
- Provenance: `https://github.com/mayf3/svc-workflow/pull/19`, exact Git object `0c63d35...`, and `docs/specs/SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1.md` at that object.
- Limitation: this observation and V6 disposition neither modify nor control PR #19; repository owner `mayf3` retains lifecycle authority.

#### OBS-V6-005 — Accepted global reader remains read-only compatibility authority

- Subject: `SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1` and merged implementation.
- Source revisions: accepted head `ea9ab2df0da7e58328ce5018164a2d2b6d6c14a9`; implementation merge `bf875c265843b3e07570a96b734051e9cfe27a43`.
- Method: complete Spec inspection and V5 relationship review.
- Result: the Reader gates the existing global GET surface only, grants no writes, and does not create a canonical dispatch feed.
- Provenance: `docs/specs/SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1.md` and exact Git commits.

#### OBS-V6-006 — External PR #87 fixed-coordinate shape and provenance

- Subject: `mayf3/dsh-agent-core` PR #87.
- Source revision: head `4260911960f33c5b91c38403f002207f717f4187`, base observed `e40c1400266b57ae7746ac766e6b281cf1fbb943`.
- Environment/observed at: GitHub, `2026-09-01`.
- Method: GitHub PR metadata and full body/commit inventory.
- Result: at the fixed observation coordinate, an open draft proposed Spec contained a recurring 30-minute scanner/dispatcher section and a bounded Scheduler-tools section. The snapshot is non-authoritative, external to this repository, and does not authorize this repository to direct the PR's lifecycle.
- Provenance: `https://github.com/mayf3/dsh-agent-core/pull/87`.

#### OBS-V6-007 — No competing V6 or canonical-activation Product Direction exists

- Subject: svc-workflow branches and PRs.
- Source revision: GitHub state observed `2026-09-01`.
- Method: fresh-list all open repository PRs, fetch exact heads, inspect GitHub base/head/draft/merge metadata, and completely inspect each sole docs proposal for PR #7, #9, #13, and #19 plus the current V6 document in PR #20.
- Result: the complete open local census is #7 `a7f8d26...`, #9 `3056263...`, #13 `83fd493...`, #19 `0c63d35...`, and self PR #20 at pre-amendment head `8189481...`. PR #7 retains its independently governed replay-closure disposition; PR #9 retains `SUPERSEDED_BY_FLEET_LOCAL_CHILD`; PR #13 is an independent proposed V4/Architecture-bound owner-scoped read-only list with no implementation/production authority, canonical activation, or `nextEligibleAt`; PR #19 retains the V5-bound non-reuse/rewrite disposition in §3.1. No competing V6 or canonical-activation Product Direction candidate exists.
- Provenance: GitHub PR #7/#9/#13/#19/#20 metadata, exact Git refs, and their sole docs paths recorded in §3.1.1.
- Limitation: PR #20's final amended head is recorded externally after commit; this census neither equates the proposals' authority kinds nor changes any repository-owner-controlled PR lifecycle.

### 4.3 Claims and assumptions

#### CLM-V6-001 — Whole-authority supersession is required

- Support state: SUPPORTED.
- Supported by: `EVD-V6-001`, `EVD-V6-002`.
- Claim: V5's node, persistence, Scheduler, wake, and dispatch semantics change; `REUSE` and `AMEND` are invalid under V0 immutability.

#### CLM-V6-002 — Architecture and implementation remain separately gated

- Support state: SUPPORTED.
- Supported by: `EVD-V6-003`.
- Claim: higher Product Direction may choose the new direction, but implementation cannot begin while frozen lower Architecture conflicts or before an accepted implementation-authorizing Spec exists in the implementation base.

#### CLM-V6-003 — Canonical activation removes business-key discovery

- Support state: INFERRED.
- Supported by: `EVD-V6-004`.
- Claim: binding activation and dispatch identity to unique `nodeVisitId` and creating it atomically on entry permits normal dispatch without scanning by `nodeKey`, DRAFT, environment labels, or metadata conventions.
- Limitation: the bounded delivery mechanism and storage/API shape remain Phase 2 authority work.

#### CLM-V6-004 — `SERVICE` remains authentication-only for this model

- Support state: SUPPORTED.
- Supported by: `EVD-V6-003` and the Owner-frozen Product Direction persisted here.
- Claim: retaining Service Principals for inter-service authentication is compatible with rejecting them as new TASK owners; no conversion to Agent is required or permitted.

#### CLM-V6-005 — One-way cutover is safer than permanent dual authority

- Support state: INFERRED.
- Supported by: `EVD-V6-004`.
- Claim: direct new-model routing plus bounded Legacy drain/migrate/terminate/read-only history prevents silent fallback and competing canonical work identities.
- Limitation: exact rollout coordinates and migration plan belong to later accepted authority and execution gates.

#### CLM-V6-006 — External interoperability is constrained locally without governing PR #87

- Support state: SUPPORTED.
- Supported by: `EVD-V6-005`.
- Claim: svc-workflow may interoperate only with external periodic recovery that is Reconciler-only and with Scheduler management separated from normal dispatch. dsh-agent-core chooses whether to satisfy those conditions through PR #87, a replacement Spec, or another locally lawful authority; none is a svc-workflow child.

### 4.4 Evidence relations

#### EVD-V6-001 — Active V5 and governance rules support lifecycle classification

- Source observations: `OBS-V6-001`, `OBS-V6-002`.
- Target: `STATE-V6-001`, `CLM-V6-001`.
- Relation: SUPPORTS.
- Bound coordinates: svc-workflow `c90d54c...`, observed `2026-09-01`.
- Strength/sufficiency: direct accepted-authority and Git evidence; sufficient for `SUPERSEDE` classification.
- Limitations: does not accept V6.

#### EVD-V6-002 — V5 and current dispatchability proposal support non-reuse disposition

- Source observations: `OBS-V6-002`, `OBS-V6-004`, `OBS-V6-007`.
- Target: `STATE-V6-002`, `STATE-V6-004`, `CLM-V6-001`.
- Relation: SUPPORTS.
- Bound coordinates: V5 on `c90d54c...`; PR #19 base `c90d54c...`, current head `0c63d35...`, open/draft/unmerged; source candidate/lineage `af450aa...`.
- Strength/sufficiency: complete current-head document and PR-lifecycle comparison; sufficient to reject reuse, acceptance, or implementation of the current proposal as this Goal's child/canonical dispatch authority.
- Limitations: does not decide Phase 2 wire/storage design and does not modify or control PR #19's repository-owned lifecycle.

#### EVD-V6-003 — Frozen Architecture supports reconciliation and identity boundaries

- Source observations: `OBS-V6-003`, `OBS-V6-005`.
- Target: `STATE-V6-003`, `CLM-V6-002`, `CLM-V6-004`.
- Relation: SUPPORTS.
- Bound coordinates: svc-workflow `c90d54c...` and accepted Reader coordinates in §3.
- Strength/sufficiency: direct normative-source evidence.
- Limitations: Product Direction selection is normative, not an observed implementation.

#### EVD-V6-004 — Node Visit and proposal facts support canonical-activation inference

- Source observations: `OBS-V6-003`, `OBS-V6-004`.
- Target: `CLM-V6-003`, `CLM-V6-005`.
- Relation: SUPPORTS.
- Bound coordinates: Architecture coordinates and current PR #19/source-lineage coordinates in §3.
- Strength/sufficiency: sufficient for Product Direction choice; implementation details remain gated.
- Limitations: no runtime behavior or production data was inspected.

#### EVD-V6-005 — External PR snapshot supports local interoperability boundaries

- Source observations: `OBS-V6-006`.
- Target: `STATE-V6-005`, `CLM-V6-006`.
- Relation: SUPPORTS.
- Bound coordinates: dsh-agent-core PR #87 head `4260911...`, observed `2026-09-01`.
- Strength/sufficiency: direct fixed-coordinate PR metadata/content evidence, sufficient to reject normal-path recurring scans and bundled Scheduler management at the local integration boundary.
- Limitations: this local Product Direction cannot prescribe, mutate, accept, or otherwise govern the external PR or the replacement authority path chosen by dsh-agent-core.

## 5. Product positioning and qualifying workflow shape

`svc-workflow` is a platform-level, serial, governed workflow engine for Human- and Agent-owned work. It owns versioned Workflow Definition governance, Workflow Instance lifecycle, legal Transition execution, immutable history, strict normal-data-plane Domain isolation, canonical Node Visit activation, Human work items, Agent dispatch intents, Domain-local administration, bounded global administration, and idempotent concurrency-safe commands.

It guarantees that a known Principal acts on an authorized current Node Visit against an explicit Definition Version and workflow state version, and that every committed state change has immutable history. It validates workflow structure and JSON protocol shape; it does not decide payload business meaning or truth and does not run an LLM.

```text
one current node per Workflow Instance
one unique Node Visit per node entry
one concrete HUMAN or AGENT owner for every active non-terminal TASK Visit
one canonical activation for every active non-terminal TASK Visit
one deterministic normal forward path
JSON stage delivery
configured backward RETURN paths
configured or governed termination paths
```

### 5.1 Workflow Definition

A Definition is a Domain-owned versioned template containing node/Transition graphs, TASK owner references, Context and Submission schemas, and the deterministic normal path.

```text
DRAFT -> PUBLISHED -> DEPRECATED -> REVOKED
```

A Definition Version may still use lifecycle state `DRAFT` while being authored, but `DRAFT` is not a Workflow node kind. For new traffic, node kinds are exactly:

```text
TASK
TERMINAL
```

`HUMAN_TASK`, `AGENT_TASK`, `SERVICE_TASK`, `WAIT_EVENT`, `WAIT_TIMER`, legacy node kind `DRAFT`, and legacy node kind `NORMAL` are forbidden in a new Definition Version. TASK carries an owner reference that must resolve on entry to exactly one enabled canonical Principal of type `HUMAN` or `AGENT`; TERMINAL has no owner and no outgoing edge. A Service Principal remains legal for inter-service authentication but is not a TASK owner.

A Definition-Version draft may be edited and validated but cannot create normal production Instances. A Published version may create Instances and is immutable. A Deprecated version creates no new Instances while existing Instances continue only as accepted child authority allows. Revoked behavior is governed by accepted Architecture/child Specs. Publication freezes graph, schemas, owner references, ordering, validator semantics, and digest inputs. Archive/discovery is non-destructive. A Definition belongs to exactly one Domain; another Domain uses a separate Definition unless a later Product Direction changes that rule.

Existing legacy Definition Versions containing `DRAFT | NORMAL | TERMINAL` are historical/drain inputs only. They MUST NOT be cloned, republished, or selected for new traffic after the cutover barrier. New Context is created with the Instance. New-flow Context has no DRAFT-node editing privilege; no later mutation is authorized unless a separate accepted authority defines it.

### 5.2 Workflow Instance

An Instance is one independent execution of one immutable Definition Version in one Domain. It owns its current Context Revision, current Node Visit, workflow state version, lifecycle/governance metadata, and references to immutable history. It is not an upper-layer business object. Optional external references may correlate it to one, but the upper layer owns that object's identity and full data.

For new traffic, Instance creation enters its first node and creates the first Node Visit and canonical activation in the same state transaction. There is no separate “created but not activated” or legacy-DRAFT staging state. Lifecycle includes creation, Transition, graph-external Domain Owner cancellation, and non-destructive archive. Normal product APIs do not physically delete Instances. Cancel/archive retain facts and remain governed by accepted Architecture and child Specs.

### 5.3 Node Visit is the only runtime work unit

A Node Visit records one entry of one Instance into one definition `nodeId`. The same definition node may be entered repeatedly; every entry has a distinct `nodeVisitId` and monotonically valid visit identity under the reconciled Architecture. The definition `nodeId` is reusable structure; `nodeVisitId` is runtime work identity.

Activation, work item, dispatch intent, dispatch-attempt idempotency, wake, reconciliation, and repair MUST bind to `nodeVisitId`. They MUST NOT bind to `nodeKey`, display/business name, DRAFT, `test_env_deploy`, `ops-lock`, metadata strings, or any other business label. Scheduler, Dispatcher, Reconciler, Watchdog, and Repair logic MUST NOT recognize or branch on business `nodeKey`.

Node Visit remains immutable. A RETURN or repeated entry creates a new Visit and therefore a new activation. An old Visit is never reopened or retagged as current.

### 5.4 Canonical activation

Every active, non-terminal TASK Node Visit has exactly one canonical activation:

```text
TASK owner type HUMAN -> HUMAN_WORK_ITEM
TASK owner type AGENT -> DISPATCH_INTENT
TERMINAL              -> no activation
```

The activation kind is derived from the resolved canonical Principal type, never from a caller field or node name. Visit creation and activation creation commit atomically. For a Dispatch Intent, that same transaction generates the canonical server-authored activation timestamp and persists it as the initial `nextEligibleAt`; no caller value or post-commit timestamp write participates. If owner resolution, Principal status/type validation, uniqueness, activation timestamp generation, or activation persistence fails, the Visit/Transition/Instance change commits nothing. One `nodeVisitId` cannot have both activation kinds or more than one canonical activation.

On a successful Transition, cancel, or manual termination, the source Visit's active activation is closed or rendered non-active in the same authoritative transaction that changes current-work status. Entry to a target TASK creates its new activation in that transaction; entry to TERMINAL creates none. Repair may restore a missing canonical activation only through a separately authorized, audited, idempotent repair path; it never creates a second one.

A Human Work Item is the canonical Human action surface. A Dispatch Intent is the canonical Agent scheduling surface. Creation of a Dispatch Intent does not select an Agent runtime, acquire resources, send a message, open/reuse a Session, or start execution.

### 5.5 Transition, Context, and Submission

`ADVANCE` follows the normal configured direction, including normal terminal completion. `RETURN` moves to an allowed earlier non-terminal node and creates a new Visit. `TERMINATE` follows a configured graph edge to an exceptional terminal. Domain Owner `CANCEL` is graph-external governance, not a Transition effect.

Only the authorized current assignee performs a normal Transition. Submission, target Visit, current projection, one state-version increment, Workflow Event, command outcome, Receipt, and required audit commit atomically. Domain ownership, broad scope, global permission, or Agent designation does not imply Transition authority.

Context is versioned workflow input, not the complete upper-layer business record. Legacy Context revisions retain their historical chain; new-flow Context mutation after creation is not authorized by V6. A Visit immutably records node entry and owner/assignee snapshot; later Principal/Domain/Definition changes do not rewrite it. Submission is immutable JSON stage delivery; large resources may be URI/digest references, and schema validity is shape rather than business truth.

### 5.6 Authoritative facts and projections

The existing authoritative history consists of immutable `WorkflowContextRevision`, `NodeVisit`, `Submission`, and `WorkflowEvent` facts. New-flow canonical Activation becomes an additional authoritative work fact after Architecture/Spec reconciliation; it does not rewrite any of the four historical fact kinds. Current Context, current Visit, activation visibility, and workflow state version are projections over authoritative facts. A successful state command changes the version once and records its Event once; partial workflow/activation commits are forbidden. Timeline is a projection, not a second authority. Scheduler access grants neither timeline nor `EventData` access.

## 6. Domain isolation, worklists, and Domain-local administration

A Domain is the workflow business-ownership, Definition-management, permission, and audit boundary. Each Definition and Instance belongs to one Domain. Canonical Domain role binding is the only Domain Owner authority; no unrelated owner field duplicates it.

```text
NORMAL_DATA_PLANE_DOMAIN_ISOLATION = STRICT
GLOBAL_CONTROL_PLANE_EXCEPTION = AUTHORIZED_AND_BOUNDED
```

An ordinary Agent/member or Domain Owner cannot see another Domain merely because it exists. Domain Owner authority is Domain-local. Current assignees receive only authorized Instance-local access; historical participants receive only explicitly governed participation history. Scope, allowlist, service/Feishu identity, UI role, or combinations of Domain-local roles do not create cross-Domain authority. Lookup, list, count, cursor, denial, and serialization behavior must not leak another Domain's existence or facts. Cross-Domain authority exists only through §§7-9's two explicit permissions and enumerated data/operations. No Architecture, child Spec, API, SDK, migration, code, test, deployment, legacy role, or UI label may broaden it.

The product owns canonical Human Work Items, canonical Agent Dispatch Intents, legacy assigned-to-me tasks/creator-owned drafts only for permitted drain/history, Domain-local Instance/audit views for the effective Owner, and authorized feedback about a Principal's own Submissions. A work item remains a workflow activation, not an upper-layer Todo/business object. Scheduler-facing Dispatch Intents are separate from Human worklists and full Instance views.

`svc-workflow` owns local Principal/Domain/membership/ownership/workflow-role projections and bindings; global identity/authentication remain external. A Domain Owner remains Domain-local, cannot become global through role composition, and cannot rewrite Visit snapshots. Verified authentication supplies actor identity; request bodies do not.

## 7. Deployment actor and split global permission model

```text
DEPLOYMENT_MODE = SINGLE_USER
OWNER = mayf3
DAILY_EXECUTION_ACTOR = DEDICATED_ADMIN_AGENT_PRINCIPAL
HUMAN_PRINCIPAL_AS_RUNTIME_ACTOR = NOT_REQUIRED
ADMIN_AGENT_STRATEGY = NEW_DEDICATED_AGENT
EXISTING_BUSINESS_OR_CANARY_AGENT_REUSE = FORBIDDEN_BY_DEFAULT

GLOBAL_PERMISSIONS =
  GLOBAL_SCHEDULER_READ
  GLOBAL_DOMAIN_ADMIN
PERMISSION_MODEL = SPLIT
SAME_AGENT_MAY_HOLD_BOTH = YES
ONE_PERMISSION_IMPLIES_THE_OTHER = NO
PRODUCT_DIRECTION_GLOBAL_WORKFLOW_COORDINATOR = UI_LABEL_ONLY
```

The dedicated Agent is an actual auth-service Agent Principal and daily runtime actor. V6 selects no UUID or Client ID. Existing business/canary Agents are not reused by default; a later designation authority must identify a newly dedicated Agent and exact Client.

There are exactly two independent Product Direction permissions. The same designated Agent may hold either or both, but one never implies the other and both do not form a third authorization capability. V6 creates no composite runtime role. `GLOBAL_WORKFLOW_COORDINATOR` may be presentation text only in a conformant Product Direction surface; it is not a Product Direction permission, migration target, or authorization alias. The pre-existing server role with that name remains only as bounded compatibility debt; V6 neither denies that observed role exists nor authorizes any new binding for it.

Neither permission grants workflow content, Transition, reassignment, cancel/archive, Definition management, membership management, Assistance body, credentials, or audit-content access.

## 8. Canonical Dispatch Intent and `nextEligibleAt`

`GLOBAL_SCHEDULER_READ` remains the Product Direction capability for bounded cross-Domain Scheduler visibility, but its canonical new-model subject is `DISPATCH_INTENT`, not `Page<DomainInstanceSummary>` and not a computed `dispatchable` flag.

```text
PRODUCT_CAPABILITY = GLOBAL_SCHEDULER_READ
CANONICAL_SCHEDULER_SUBJECT = ACTIVE DISPATCH_INTENT
CANONICAL_WORK_IDENTITY = nodeVisitId
WAIT_PRIMITIVE = nextEligibleAt
CURRENT_GLOBAL_READER_ROUTE = COMPATIBILITY_AND_DIAGNOSTIC_ONLY
CURRENT_GLOBAL_READER_ROUTE_IS_DISPATCH_FEED = NO
NEW_GLOBAL_WORKFLOW_COORDINATOR_GRANTS_AUTHORIZED = NO
```

The minimum Scheduler-facing Dispatch Intent record is bounded to identifiers and time needed to route one active Visit:

```text
dispatchIntentId
nodeVisitId
workflowInstanceId
ownerPrincipalId
nextEligibleAt
createdAt
updatedAt
```

It MUST NOT expose or require `nodeKey`, definition/business names, Context/title, task label, Submission/history, timeline `EventData`, Assistance content, credentials/tokens, Receipt/audit payload, Transition options, business metadata, DRAFT, `test_env_deploy`, or `ops-lock`. The Scheduler and Dispatcher may use `nodeVisitId` and canonical Principal identity; they may not interpret workflow business node keys.

`nextEligibleAt` is a required server-authored timestamp on each active Dispatch Intent. The activation transaction generates one canonical server-authored activation timestamp and persists it in that same transaction as the initial `nextEligibleAt`, making the intent immediately eligible for Scheduler consideration without immediately starting an Agent. The initial value is not client-provided, is not filled in after commit, and is not required to equal a physical or “true” database commit instant. An implementation MUST NOT split Visit/activation creation, defer the write, or weaken atomicity merely to obtain a commit-instant value. The only Scheduler-facing wait predicate is:

```text
active Dispatch Intent AND nextEligibleAt <= authoritative now
```

If execution cannot proceed, the controlled attempt/outcome path records the next permitted concrete timestamp in `nextEligibleAt`. No second Scheduler-facing wait status, blocked-reason enum, DRAFT convention, timer node, event-wait node, metadata key, retry flag, or business-node exception may determine eligibility. Outcome/audit records may explain what happened, but the Scheduler MUST NOT use them as another wait predicate.

The existing `GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR` global-list route remains available only under its accepted compatibility semantics. V6 creates no new role/grant and does not automatically extend existing Reader/Coordinator bindings to the Dispatch Intent surface. The later implementation Spec must freeze a fail-closed server-side permission mapping within the `GLOBAL_SCHEDULER_READ` product capability and must separately reconcile current Reader/Coordinator compatibility.

## 8A. Wake, retry, timeout, attempts, and repair

An external event, dependency completion, or authorized manual action cannot directly start an Agent, create another activation, mutate a Workflow node, or bypass command validation. It may only invoke a controlled idempotent wake command that sets the active Dispatch Intent's `nextEligibleAt = now`. The command binds `nodeVisitId`, verifies the current activation, authorizes the authenticated actor server-side, writes durable audit, and is a no-op when the Visit is no longer current/active or the same wake was already applied.

Retry interval, backoff, fairness, priority, quota, Principal-to-Agent mapping, delivery, Session selection/reuse, and execution-attempt policy remain Scheduler/Dispatcher ownership under their own accepted repository authorities. Those policies may choose a future `nextEligibleAt` after a deterministic non-execution outcome; they cannot add another Workflow wait primitive.

Dispatch attempts and resource leases are attempt-scoped. A lease may protect one execution attempt or external resource, but it is not Workflow node syntax, not a Node Visit identity, and not a durable alternative to `nextEligibleAt`. Lease acquisition/loss never changes the Definition graph and never authorizes a new Visit or activation.

Attempt idempotency is bound to `nodeVisitId` plus a stable attempt identity. Same Visit/same attempt/same request replays the original outcome; same identity/different request conflicts without mutation. On timeout or lost response, the outcome is `outcome_unknown`; reconciliation uses the exact same Visit/attempt/request identity. Blind retry under a new identity, duplicate activation creation, or dispatch by node/business key is forbidden.

The normal main path is activation-driven: the Node Visit transaction durably creates the canonical Dispatch Intent and a bounded delivery mechanism makes that intent available to the Scheduler. A periodic scan of Workflow Instances, global summaries, business node keys, or metadata MUST NOT discover ordinary new work. Periodic scans are allowed only for Reconciler, Watchdog, or Repair purposes: detect missing/duplicate/stuck activation or delivery, preserve the canonical `nodeVisitId`, and repair idempotently with audit. This does not authorize Kafka, a generic Outbox/event platform, or a general Operator; the exact bounded delivery design belongs to later Architecture and implementation authority.

Future external interoperability must satisfy:

```text
PERIODIC_EXTERNAL_RECOVERY = RECONCILER_ONLY
SCHEDULER_MANAGEMENT = SEPARATE_FROM_NORMAL_DISPATCH
EXTERNAL_AUTHORITY_PATH = EXTERNAL_REPOSITORY_CHOICE
PR_87_RELATION = FIXED_COORDINATE_NON_AUTHORITATIVE_OBSERVATION_ONLY
```

`dsh-agent-core` may establish those properties by amending PR #87, adopting a replacement Spec, or using another locally lawful authority. Its repository owns that choice and every PR review/acceptance/merge/closure action. No svc-workflow document may treat the observed PR snapshot as accepted authority, require a particular PR lifecycle action, or implement/modify external code from this repository.

## 9. `GLOBAL_DOMAIN_ADMIN`

Allowed operations are only:

1. idempotent Domain creation;
2. atomic initial Owner assignment during creation, or disabled retention until a valid Owner is ready;
3. atomic Domain Owner replacement;
4. the minimum Domain/Principal selection directory required for those operations.

Creation/replacement preserves one effective Owner, canonical Principal identity, idempotency, transaction integrity, and durable audit. An enabled Domain cannot be left ownerless by partial success.

Complete minimum directory fields:

```text
Domain selection:
  domainId
  domainKey
  domainDisplayName
  domainEnabled
  currentOwnerPrincipalId
  currentOwnerPrincipalDisplayName

Principal selection:
  principalId
  principalDisplayName
  principalType
  principalEnabled
```

There is one logical record per Domain/Principal. The directory is only for create/replace selection and excludes membership, workflow counts/facts, email, Feishu identifiers, credentials, scopes, permission bindings, audit data, and content-derived fields. Child Specs may freeze search/pagination/nullability/grouping but not add globally readable data.

Excluded authority includes Workflow Instance/Context/Submission/Visit/timeline/Assistance/worklist content; Transition/Context revision/reassignment/cancel/archive; Definition lifecycle/assignment; membership; Domain-local audit content; any other Domain data write; self-grant; self-Owner; or actor derivation from body, Feishu, display name, service identity, scope, or allowlist.

```text
SELF_GRANT = FORBIDDEN
SELF_DOMAIN_OWNER = FORBIDDEN
SELF_DOMAIN_OWNER_RULE = CANONICAL_PRINCIPAL_EQUALITY
NO_ACCEPTED_LINKAGE_AUTHORITY = TREAT_DISTINCT_CANONICAL_PRINCIPALS_AS_DISTINCT
IMPLICIT_COMMON_CONTROL_INFERENCE = FORBIDDEN
FAIL_CLOSED_WHEN_LINKAGE_IS_ABSENT = NO
```

If authenticated actor Principal equals the new Owner Principal, reject. Distinct canonical UUIDs are treated as distinct; svc-workflow neither invents common control nor rejects solely because linkage proof is absent. Only an exact accepted external identity authority explicitly establishing linkage may extend the prohibition, and a child Spec must pin it. Runtime heuristics, Agent claims, request fields, or Feishu/service identity cannot.

## 10. Authentication, execution identity, and authorization

`svc-workflow` trusts only auth-service-verified token identity. The execution actor is the Agent Principal in verified `token.sub`, which must be the exact designated Admin Agent Principal UUID. The Client/intermediary, Feishu sender, conversation, display name, request-body actor, scope, allowlist, and Agent self-claim are not actor or permission facts.

Global role/binding state remains server-side and is not accepted from Agent-reported JWT claims or tool arguments. Each permission is independently evaluated on every protected request against the active repository designation and server-side binding. A disabled/revoked Principal or Client, invalid/expired token, inactive designation/binding, or missing permission fails closed.

Auth-service Principal type `SERVICE` remains valid for inter-service authentication. For new workflow activation it is explicitly invalid as TASK owner: Definition publication and runtime owner resolution MUST reject it fail closed, MUST NOT auto-convert it to `AGENT`, MUST NOT create a Human Work Item or Dispatch Intent for it, and MUST NOT place it in any Scheduler-visible due set. A request body, display name, scope, role, Agent mapping, or service credential cannot override canonical Principal type.

```text
HUMAN_PRINCIPAL_ADMINISTRATION_REQUIRED_FOR_V1 = NO
HUMAN_OBO_REQUIRED_FOR_V1 = NO
TWO_PERSON_APPROVAL_REQUIRED_FOR_V1 = NO
HUMAN_ROOT_REQUIRED_FOR_V1 = NO
LONG_TERM_MULTI_HUMAN_GOVERNANCE = DEFERRED_NOT_FORBIDDEN
```

## 11. Repository-owned trusted Admin Agent designation

A later docs-only authority must be established:

```text
AUTHORITY_ID = SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1
AUTHORITY_KIND = repository_owned_security_invariant
IMPLEMENTATION_AUTHORITY = none
OWNER = mayf3
```

It must record exactly:

```text
agentPrincipalId
clientId
grantedPermissions
activationTime
repository owner acceptance
supersedes
superseded_by
owners
```

`grantedPermissions` is a subset of the closed set `{GLOBAL_SCHEDULER_READ, GLOBAL_DOMAIN_ADMIN}`. No other permission or composite role is legal. The authority is inert while proposed/unmerged. It activates only after independent review, Owner acceptance, and merge to `main`. Runtime API, Feishu message, request body, Agent self-claim, token claim, database-only mutation, or deployment config cannot establish it.

```text
RUNTIME_SELF_GRANT = FORBIDDEN
RUNTIME_GRANT_API_REQUIRED = NO
DESIGNATION_EXPIRY_REQUIRED = NO
DESIGNATION_ROTATION = WHOLE_AUTHORITY_SUCCESSOR
DESIGNATION_REVOCATION = WHOLE_AUTHORITY_SUCCESSOR_OR_EMERGENCY_DISABLE
BREAK_GLASS_GRANT = NOT_SUPPORTED
```

Designation need not auto-expire. Access Tokens remain short-lived; Client secrets remain rotatable; Clients remain revocable; Principals and svc-workflow global bindings remain disableable. Revocation/disablement fails closed and creates a publication/commit barrier: the old Agent cannot release protected data or commit protected writes afterward.

Credential leak response order is:

1. revoke/rotate the auth-service Client;
2. disable the compromised global binding;
3. disable the Agent Principal if necessary;
4. write durable security audit;
5. grant no replacement Agent permission until a new designation successor is merged.

Agent replacement order is:

```text
old Agent revoke
-> new Agent Principal/Client verified
-> new root authority successor independently reviewed/Owner accepted/merged
-> new Agent active
```

Emergency disablement is containment, not a replacement grant. There is no runtime break-glass Agent.

## 12. Feishu single-user command ingress

```text
FEISHU_IS_AUTHORITY_SOURCE = NO
FEISHU_COMMAND_PROVENANCE = YES
SVC_WORKFLOW_EXECUTION_ACTOR = ADMIN_AGENT_PRINCIPAL
```

The Agent-core ingress gate must verify at least:

- exact trusted Feishu app;
- exact tenant;
- exact prebound single-user conversation;
- exact allowed sender identifier;
- verified event/message ID;
- timestamp, nonce, and replay protection.

These facts gate command admission and provide provenance in Agent core. They never become Human permissions or `svc-workflow` actor identity. The dedicated Agent calls svc-workflow with a direct token whose `sub` is its exact designated Agent Principal UUID.

Durable audit correlation must associate, without confusing authorization sources:

```text
Agent Principal UUID
Feishu tenant
conversation ID
sender ID
event/message ID
ingress correlation ID
svc-workflow request/receipt ID
```

Authorization uses only the verified Agent Principal and server-side active binding/designation. A Feishu sender ID is not `token.sub`; message text, mention, membership, and conversation binding cannot impersonate the actor. Feishu outage or rejected ingress creates no weaker fallback path.

## 13. Audit, failure, revocation barrier, retention, and idempotency

Every successful or authenticated-denied protected global operation requires durable audit, including Dispatch Intent reads, `nextEligibleAt` updates/wakes, reconciliation/repair, directory reads, Domain create/Owner replace, designation/grant activation, revoke/disable, and lifecycle/security actions. Unauthenticated denial follows existing authentication/security-audit semantics and never promotes unverified fields into actor facts.

Audit identifies actual authenticated Agent Principal, target/subject, independent permission/operation, decision/result, time, idempotency/correlation IDs, and non-sensitive reason codes. It carries Feishu provenance only when present. It does not copy Context, Submission, Assistance/supporting payload, `EventData`, credentials, tokens, Receipt bodies, unrestricted request/response bodies, or other sensitive content.

```text
AUDIT_RETENTION = 365_DAYS
FAILURE_POLICY = FAIL_CLOSED
AUDIT_PRODUCT_READ_API = NOT_SUPPORTED
EXTERNAL_AUDIT_EXPORT = NOT_SUPPORTED
```

A protected successful read must durably commit audit before data publication. If audit or authorization state is unavailable, no data is released. A protected write and required audit commit atomically. Revocation/disablement is rechecked at the publication/commit barrier; an older in-flight request cannot publish or commit after authority ends.

State-changing workflow/control-plane commands require client idempotency identities and canonical request comparison. Node-activation/dispatch/wake/repair identities bind `nodeVisitId`, authenticated actor, and complete command meaning. Same key/same request replays the original outcome; same key/different request conflicts without changing it. Conflicting writes serialize and workflow state versions are enforced where applicable. Facts/projections/events/activations/receipts/audits commit atomically.

When a client cannot know whether the authoritative write committed, return `outcome_unknown` (or child-wire equivalent). Reconciliation retries only the exact same request with the same key; generating a new key and blindly retrying is forbidden. Revocation, Owner-replacement races, retries, and compatibility routes cannot bypass current authorization.

## 14. External ownership, technology, and retained trade-offs

### 14.1 auth-service

Auth-service owns global identity, authentication, Agent Principals, Clients/credentials, token issuance, resolution, revocation, and signing keys. svc-workflow neither signs tokens nor treats Feishu/body fields as identity. Direct machine access uses auth-service RS256/JWKS verification, `aud=svc-workflow`, and canonical Agent Principal UUID in `token.sub`, subject to the accepted bounded JWKS cache trade-off.

Agent-first V1 requires a separate accepted auth-service child authority supplying only the designated Agent's needed audience/scope/grant. It must not grant a Human or any other Agent, place business role authority in a self-reported JWT claim, or depend on PR #15/PR #2. This Product Direction references but does not govern auth-service.

### 14.2 dsh-agent-core and Feishu

Agent-core/integration layers own Feishu transport, ingress verification, Scheduler policy, Principal-to-Agent mapping, Agent routing, delivery, Session selection/reuse, attempt-scoped resource leases, capability manifests, credential brokering, request/receipt correlation, and Agent execution dispatch. They must resolve actual Agent identity from trusted credentials/process binding, never model/tool/body self-report. svc-workflow owns canonical Node Visit activation, Dispatch Intent identity/`nextEligibleAt`, and final authorization against its Principal and server-side permissions.

### 14.3 Upper layers and UI

`adc-v2` and other business products own Requirement, Todo, project, priority, task label, Article, Campaign, business rules, and long-lived business state. They may correlate `workflowInstanceId` and use accepted contracts but cannot mutate workflow storage or persist competing workflow state authority.

UI products own presentation/navigation/interaction and may show `GLOBAL_WORKFLOW_COORDINATOR` only as a label. Labels do not authorize. External message/email/webhook/integration delivery belongs to adapters/business services.

### 14.4 Explicit non-ownership

svc-workflow does not own UI rendering, upper-layer business objects/content, business-specific decision logic, identity proofing, credential/token issuance, Feishu identity/permission administration, outbound delivery, built-in LLM/Agent execution, Scheduler policy, Agent mapping, Session lifecycle, attempt resource leasing, generic task labels/Context-title scheduling, or unrestricted cross-Domain content.

Shared PostgreSQL infrastructure changes no ownership: workflow tables may be written only through accepted svc-workflow command boundaries.

### 14.5 Technology and layers

```text
Rust
Axum
PostgreSQL
sqlx
tokio
tower-http
```

- `domain`: typed IDs/entities/enums/commands/events/permissions/errors, without HTTP ownership.
- `application`: use-case orchestration and authorization over ports, without Axum request mechanics.
- `http`: routes, authentication adaptation, strict DTOs, wire validation, response/error mapping.
- `store`: PostgreSQL repositories, atomic transactions, concurrency, immutable facts/projections/audit, migrations.
- `auth`: RS256/JWKS verification and accepted credential adaptation, never issuance.

Global control-plane behavior cannot be UI-only filtering, post-read handler redaction, or adapter bypass of application/storage authorization.

### 14.6 Retained trade-offs

No normal physical DELETE exists for Instances; cancel/archive preserve history. Single PostgreSQL without read/write separation remains allowed. Offline JWKS has the accepted bounded revocation-cache window. Scheduler surfaces intentionally lack full content, node/business keys, task label, and Context title. No break-glass grant exists. These trade-offs never weaken fail-closed authorization, audit, or Domain isolation.

## 15. Requirement ownership guide

| Requirement language | Owning product boundary |
|---|---|
| workflow, Instance, current node, advance, approval flow | svc-workflow |
| state machine, `ADVANCE`, `RETURN`, `TERMINATE` | svc-workflow |
| event sourcing, timeline, immutable workflow Event | svc-workflow |
| Definition/template/version publication/graph validation | svc-workflow |
| Domain and normal cross-Domain isolation | svc-workflow |
| Node Visit activation, Human Work Item, Dispatch Intent, `nextEligibleAt` | svc-workflow |
| Scheduler policy, Agent mapping, delivery, Session, attempt lease | dsh-agent-core / integration owner |
| legacy assigned worklist and creator-owned draft | svc-workflow, drain/history only |
| Instance cancel/archive | svc-workflow |
| Requirement/Todo/task board/article/campaign/business rule | owning upper-layer product |
| UI presentation/interaction | UI product |
| identity proofing/token issuance | auth-service |
| Feishu transport/notification/external delivery | integration layer |

## 16. Conformance debt at the authoring coordinates

V6 preserves V5's record of, and does not excuse, these gaps. It adds the new-model gaps below; no current implementation is implied:

### svc-workflow

- global query still reads Context title and may return terminal/archived records;
- protected global read lacks the complete disabled-Principal gate and durable audit-before-publication;
- legacy composite Coordinator role/binding remains;
- no-self-grant and no-self-owner are not comprehensively enforced;
- separated permission lifecycle is absent;
- minimum Human/Agent selection directory is absent;
- existing Domain-admin surface still requires narrowing and verification.
- no `TASK | TERMINAL` new-definition model exists;
- no owner-type rejection for Service-owned TASK activation exists;
- no canonical Human Work Item / Dispatch Intent invariant exists;
- no Scheduler-facing `nextEligibleAt` or controlled wake contract exists;
- no activation-driven normal delivery seam or activation Reconciler exists;
- no one-way new-traffic cutover barrier exists.

### dsh-agent-core

- no accepted activation-driven Dispatch Intent consumer exists;
- no qualified external authority/evidence yet establishes Reconciler-only periodic recovery for this integration;
- no qualified external authority/evidence yet establishes Scheduler management separated from normal dispatch;
- PR #87 at `4260911...` remains fixed-coordinate observation/provenance only; this repository requires no particular amendment, split, replacement, or lifecycle action on it;
- no accepted workflow Domain-admin capability manifest;
- no complete exact Feishu sender/app/tenant/conversation allowlist and replay gate for this route;
- Notification Ingress is currently a thin delivery adapter, not authority for administrative command ingress;
- actual actor must come from trusted credential/process resolution; Agent self-report is never authority.

### auth-service

- Agent Principal, Client, direct token, identity resolution, rotation, and revoke are reusable;
- designated global permission supply still requires an independent accepted child authority;
- Human Principal PR #15 is not a prerequisite;
- Agent-first V1 must not modify the Human route.

No current code, contract bundle, runtime, or report is deemed conforming by this list. Child compliance must use exact implementation and environment coordinates.

## 17. Original CTO bounded one-time Principal successor migration retained unchanged

V6 preserves and fully restates V5’s retained CTO bounded exception without changing its pair, scope, counts, child authority, or production gate and without creating ordinary reassignment. It remains a bounded Legacy exception and does not create a new-model activation path:

```text
AMENDMENT_ID = PBV2-ONE-TIME-SUCCESSOR-001
MIGRATION_KIND = ONE_TIME_SUCCESSOR
OLD_PRINCIPAL = 3e2439d2-fb54-44f5-afee-77aa17c40d22
NEW_PRINCIPAL = 4e5a4578-0645-4133-bd35-b80e453dfee9
ORDINARY_REASSIGNMENT = NOT_AUTHORIZED
HANDOFF = NOT_AUTHORIZED
DELEGATION = NOT_AUTHORIZED
GENERAL_REASSIGNMENT_API = NOT_AUTHORIZED
LONG_LIVED_PRODUCT_CAPABILITY = NO
PRODUCTION_APPLY_AUTHORIZED = NO
```

The offline tooling is fixed to that pair, cannot accept arbitrary UUIDs, and fails closed on scope drift. It may transfer only the independently reviewed nine currently enabled Domain authorities and the execution-time workflow responsibility that remains active, non-terminal, currently assigned to OLD, and exactly present in the reviewed plan. Any live mismatch conflicts with zero writes.

Historical attribution is immutable. Existing Visits/assignments are never updated, deleted, or relabeled. Exactly zero of the known 58 historical assignments and zero of the known 111 historical Visits are migrated/rewritten. Successor responsibility is represented only through newly appended Visit, Event, Receipt, and Audit facts.

Nine Domain transfers, eligible current-responsibility facts, projection changes, Receipt completion, and audit commit atomically. Partial success is forbidden. Exact rerun after success is NOOP with zero writes and zero new audits; mismatched metadata/post-state fails closed. No HTTP/SDK/reusable reassignment, handoff, delegation, or arbitrary-pair surface is created.

Implementation remains bounded by accepted `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1`. Production apply still requires a separate independently reviewed execution gate pinning implementation SHA, clean checkout, reviewed plan/digest, database identity, operator identity, and fail-closed preconditions. Open PR #7 affects only its stated replay closure and remains independently governed.

## 17A. Additive trusted-fleet exact-plan-bound bounded successor exception

### 17A.1 Frozen canonical plan artifact

V6 preserves V5's authority for only the exact frozen local evidence artifact and its reviewed contents:

```text
PLAN_SCHEMA = workflow_trusted_fleet_successor_plan_v2
PLAN_PATH = /Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json
PLAN_SIZE_BYTES = 540472
PLAN_SHA256 = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606
ROSTER_SHA256 = f046d18f76da838ba94775af7c960d0ee548f2e392c22e6c7b0e3add36cb8e5f
PLAN_MODE = READ_ONLY_CANONICAL_PLAN
PLAN_CANONICAL = YES
SNAPSHOT_OBSERVED_AT_UTC = 2026-08-24T01:03:53.192875+00:00
PLAN_PRODUCTION_CHANGE = NONE
PLAN_SUPERSEDES_SCHEMA = workflow_trusted_fleet_successor_plan_v1
PLAN_SUPERSEDES_SHA256 = 57f769d0bc9f0a4494dd37685da3cc8657b2dc5845f020858457dbecc35ce9b7
```

The artifact is a read-only canonical production snapshot; it performs and authorizes zero writes. Any future operator binds to `PLAN_SHA256` exactly. A byte-different plan, an unverified digest, or any runtime-supplied `OLD`/`NEW` parameter is outside authority. The artifact remains a local evidence file and is not committed by this PR; only its digest is frozen here. The frozen roster source is the dsh-agent-core account-recovery evidence file `docs/evidence/account-recovery-phase-a-20260823/plan-production-20260823015745Z.json` bound by `ROSTER_SHA256` and file sha256 `32d0b23753370156150babcaf0b108ad1d8c2b28f952e9586cf700142f9ec852`; the roster is itself frozen and is never dynamically expanded.

### 17A.2 Exact fleet authority and frozen counts

The exception authorizes only the exact successor pairs, Domain tuples, and active responsibility tuples contained in the frozen artifact:

```text
TOTAL_NEW_AGENTS = 86
EXACT_SUCCESSOR_PAIR_COUNT = 86
AMBIGUOUS_COUNT = 0
CONFLICT_COUNT = 0

WORKFLOW_PROJECTION_CREATE_COUNT = 85
WORKFLOW_PROJECTION_PRESENT_COUNT = 1

DOMAIN_OWNER_TRANSFER_COUNT = 8
DOMAIN_MEMBER_TRANSFER_COUNT = 752
DOMAIN_TUPLE_COUNT = 760

ACTIVE_RESPONSIBILITY_TRANSFER_COUNT = 80

DRAFT_OWNERSHIP_CANDIDATE_COUNT = 99
DRAFT_OWNERSHIP_MIGRATION_COUNT = 0
```

These counts are frozen authoring scope bound to the artifact digest — never live database truth and never permission to force counts at apply time. Every future apply re-verifies live state against the exact artifact rows and fails closed on drift.

For each exact pair, the only permitted future transaction shape is:

1. verify the NEW Auth Principal is exact and `active`;
2. NEW Workflow projection: create when missing, or require exact match when present;
3. transfer the reviewed Domain authority tuples;
4. transfer the reviewed active responsibility tuples;
5. write Event / Receipt / Audit;
6. verify the exact terminal state.

No step accepts arbitrary `OLD`/`NEW` input; identities come only from the frozen artifact rows.

### 17A.3 Duplicate identity exclusion and canonical pairs

Exactly one identity is excluded as a duplicate:

```text
EXCLUDED_AGENT_ID = efficiency-agent
EXCLUDED_PRINCIPAL_ID = d09f8849-073c-484a-978c-f375113c28b2
EXCLUDED_CLASSIFICATION = EXCLUDED_DUPLICATE_IDENTITY
EXCLUDED_MIGRATION_CANDIDATE = false
EXCLUDED_FUTURE_OPERATOR_WRITES = 0
```

The excluded identity is a disabled, non-canonical duplicate with an existing projection, zero enabled Domain bindings, and zero Visits. A future operator encountering it commits zero writes. The only canonical efficiency pair is:

```text
efficiency-manager / 95eab282-22c7-46a2-8580-abfef4942cdc
  -> agt_efficiency-agent / b21ddb23-42f6-47c4-a27f-bc44950e554c
```

The artifact also fixes, as two fully independent pairs:

```text
build-in-public-agent -> agt_build-in-public-agent
blog-agent -> agt_blog-agent
```

`blog-agent` pairs only with `agt_blog-agent`, never with `agt_build-in-public-agent`; agent labels remain explanatory provenance and are never runtime identity selectors.

### 17A.4 Workflow projection creation

Future apply must first resolve the current systemic worklist failure: 85 of 86 NEW Auth Principals are absent from the svc-workflow Principal projection, so assigned-to-me reads return `404 principal_not_found`. The one present projection is exactly `agt_build-in-public-agent`.

Each missing projection is created only for the exact NEW Principal in the frozen artifact. Forbidden:

- dynamically enumerating an 87th identity from Auth;
- display-name pairing;
- deriving an OLD identity by stripping the `agt_` prefix from a NEW agent_id;
- fuzzy or prefix matching;
- creating the excluded identity's projection or binding it anywhere;
- replacing any Auth Principal UUID.

Stripping the `agt_` prefix never constitutes mapping evidence; display-name, prefix, and fuzzy matching remain forbidden; exact pairs come only from the frozen artifact.

After creation, verification must show `workflow_my_tasks` no longer returns `principal_not_found`; with no current tasks, the correct terminal state is exactly HTTP 200 with `items = []`.

### 17A.5 Domain transfer

Only the 760 exact tuples in the artifact are processed.

`DOMAIN_OWNER`:

- exact OLD -> NEW transfer;
- atomic Owner replacement;
- never commits dual Owner;
- Domain unchanged.

`DOMAIN_MEMBER`:

- enable NEW;
- disable OLD;
- Domain and Role unchanged;
- never commits long-lived dual authority.

Any tuple drift — missing, additional, disabled, role-changed, Principal-changed, or otherwise mismatched against the artifact — sets that pair's writes to 0 and the outcome to `CONFLICT`.

### 17A.6 Active responsibility transfer

Only the 80 exact responsibility tuples are processed, and each is re-validated at apply time: the current Visit is current, active, non-terminal, not cancelled, not archived, currently assigned to OLD, and its expected workflow state version matches the artifact.

Apply must:

- append a same-node successor Visit;
- append a dedicated Event;
- append a Receipt;
- append durable Audit;
- CAS the workflow state version;
- preserve Instance and node;
- keep historical Visits immutable;
- keep historical assignments immutable.

Terminal, completed, cancelled, or archived records are never migrated or reactivated.

### 17A.7 Creator-owned drafts

The 99 creator-owned draft tuples keep `created_by_principal_id` unchanged:

```text
DRAFT_CREATOR_HISTORY_IMMUTABLE = YES
DRAFT_SUCCESSOR_MIGRATION = FORBIDDEN
```

If a "current maintainer" concept is ever needed, it requires a separate future draft-stewardship capability; this boundary never silently overwrites creator attribution.

### 17A.8 Canary and execution sequence

The complete future sequence is:

1. accepted fleet Product Boundary;
2. accepted local implementation Child;
3. independently reviewed operator;
4. production read-only plan recheck;
5. exact `PLAN_SHA256` review;
6. separate production apply authorization;
7. canary 1: `agt_build-in-public-agent`;
8. canary 2: `agt_efficiency-agent`;
9. remaining exact 84 pairs;
10. verify;
11. exact rerun NOOP.

```text
SPEC_ACCEPTED != PRODUCTION_APPLY_AUTHORIZED
```

Each pair commits in its own independent SERIALIZABLE transaction. One pair's failure never fabricates another pair's success.

### 17A.9 No durable general capability

```text
CTO_BOUNDED_SUCCESSOR_AUTHORITY = PRESERVED_UNCHANGED
FLEET_BOUNDED_SUCCESSOR_AUTHORITY = ADDITIVE_EXACT_PLAN_BOUND_ONLY
ORDINARY_REASSIGNMENT = STILL_FORBIDDEN
HANDOFF = STILL_FORBIDDEN
DELEGATION = STILL_FORBIDDEN
GENERAL_SUCCESSOR_API = FORBIDDEN
GENERAL_MIGRATION_CAPABILITY = NO
RUNTIME_OLD_NEW_PARAMETERS = FORBIDDEN
DYNAMIC_ROSTER_EXPANSION = FORBIDDEN
ONLINE_MANAGEMENT_API = FORBIDDEN
HISTORICAL_REWRITE = FORBIDDEN
TERMINAL_TASK_REACTIVATION = FORBIDDEN
PRODUCTION_APPLY_AUTHORIZED = NO
```

The retained CTO exception and the exact fleet mappings, including the Build in Public pair, are never abstracted into a caller-parameterized general migration mechanism.

### 17A.10 Blocked Child and PR disposition

Draft PR #9 is now dispositioned:

```text
PR_9_DISPOSITION = SUPERSEDED_BY_FLEET_LOCAL_CHILD
```

This round must not close, modify, or merge PR #9. Its single-pair Child meaning is superseded by the fleet boundary above; the future local implementation Child (sequence step 2) supersedes it and must carry its own independent review. The reviewed semantic V6 Head `bc4a13a968073e1a81ba3fb168d4bf5c3cc12ba9` received a fresh independent fixed-head `ACCEPT` review before Owner acceptance; the resulting lifecycle-only final accepted candidate Head still requires a fresh independent final-head recheck before merge. No earlier V5 review result transfers to V6.

## 18. Capability-scoped child authorities and ordering

No common all-Slices global gate is created. Each child authorizes only its own capability. V6 acceptance selects Product Direction only; while the accepted candidate remains unmerged it is not active repository authority and does not activate Phase 2.

### Slice A — Dedicated Admin Agent identity

Create/select a new dedicated Agent; establish exact Agent Principal UUID and exact Client; verify enabled status, credential ownership, rotation, and revoke. This Product Direction performs none of these actions.

### Slice B — Trusted Agent designation root

Independently review, Owner-accept, and merge `SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1` with exact Agent/Client and split permissions.

### Slice C — auth-service permission supply

An independent accepted auth-service child authority supplies only the designated Agent's required audience/scope/grant. It grants no Human or other Agent, is versioned/auditable/idempotent/revocable, and does not put business role authority in self-reported JWT claims.

### Slice D — svc-workflow Architecture and implementation authority

After V6 is accepted on `main`, one independently reviewed Architecture successor/refinement must reconcile v0.3.1 node/Principal/fact/transaction semantics with §§5 and 8. Only after that authority is accepted may an independently reviewed implementation Spec authorize the bounded new model. One complete implementation Spec may cover node activation, Dispatch Intent/`nextEligibleAt`, controlled wake, reconciliation, and one-way cutover if it remains independently reviewable; V6 does not require artificial Spec multiplication. Current PR #19 `SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1@0c63d35...`, sourced from `af450aa...`, cannot serve this role: it is non-authoritative and MUST NOT be accepted or implemented as this Goal's child/canonical dispatch authority. If its query semantics remain useful, they require later re-investigation and rewrite under accepted V6 and the lawful successor authorities, without competing with canonical activation. V6 performs no PR #19 lifecycle action; repository owner `mayf3` retains that authority.

### Slice E — svc-workflow Domain admin

An implementation-authorizing child Spec freezes Domain create, initial Owner, Owner replacement, no-self-grant, no-self-owner, atomic audit, minimum directory, and reconciliation of conflicting `IDENTITY_PROVISIONING_API_V0` semantics.

### Slice F — dsh-agent-core dispatch and Scheduler management

External dsh-agent-core authority owns Scheduler policy, mapping, delivery, Sessions, attempt leases, and bounded Scheduler management. For svc-workflow interoperability, periodic external recovery must be Reconciler-only and Scheduler management must be separated from normal dispatch. dsh-agent-core may satisfy those conditions by amending PR #87, replacing it with another Spec, or using any other locally lawful authority; that repository exclusively performs and decides its own authoring/review/acceptance/implementation/PR lifecycle. svc-workflow cannot prescribe or perform those actions.

### Slice G — cutover and Legacy execution gates

The local implementation authority must freeze the exact new-traffic cutover barrier, Legacy inventory, drain/migrate/manual-terminate/read-only-history modes, compatibility window, rollback containment, reconciliation, and production authorization. Spec acceptance, code merge, migration readiness, and production cutover are separate gates.

Slices may have dependency edges necessary for their own execution, but no Slice silently activates another. Assistance and Admin Recovery remain independent unless a child actually changes their data/semantics. Current HTTP/OpenAPI/SDK surfaces change only with their own accepted implementation authority.

## 19. Decisions

### DEC-V6-001 — Single-user dedicated Agent operation

- Decision owner: `mayf3`.
- Decision: daily global administration uses one new dedicated Agent Principal with direct token; Human runtime Principal/OBO/administration and two-person approval are not V1 prerequisites.
- Rejected: reuse an ordinary business/canary Agent; retain Human-root/two-approver V1 prerequisite.
- Owner input remaining: none.

### DEC-V6-002 — Repository designation replaces runtime grant governance

- Decision owner: `mayf3`.
- Decision: exact Agent/Client/split permissions are activated only by merged docs-only `SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1`; replacement uses whole-authority successor, with emergency disablement only for containment.
- Rejected: runtime self-grant, runtime grant API requirement, auto-expiring designation, break-glass grant.
- Owner input remaining: none.

### DEC-V6-003 — Split global permissions remain independent

- Decision owner: `mayf3`.
- Decision: preserve exactly the two split Product Direction permissions. `GLOBAL_SCHEDULER_READ` is the new-model Dispatch Intent read capability; `GLOBAL_DOMAIN_ADMIN` remains separate. Current `GLOBAL_WORKFLOW_READER`/Coordinator bindings do not automatically grant the new surface.
- Rejected: composite Coordinator runtime role or alias.
- Owner input remaining: none.

### DEC-V6-004 — Feishu is gated provenance, never authority

- Decision owner: `mayf3`.
- Decision: exact single-user Feishu ingress gates command admission in Agent core; svc-workflow actor remains the designated Agent Principal.
- Rejected: Feishu sender as Human Principal or body actor; Human OBO prerequisite.
- Owner input remaining: none.

### DEC-V6-005 — Preserve unaffected V5 boundaries

- Decision owner: `mayf3`.
- Decision: retain V5 Domain, security, admin, history, transition, audit, external-ownership, technology, retained-trade-off, and exact successor-exception meaning except where §§5, 8, 18, 23 and DEC-V6-007 through DEC-V6-014 explicitly replace node/activation/dispatch/cutover meaning.
- Rejected: partial supersession, reader-side composition with V5, or silently dropping unrelated V5 protections.
- Owner input remaining: none.

### DEC-V6-006 — Preserve the exact fleet bounded exception

- Decision owner: `mayf3`.
- Decision: preserve the CTO exception unchanged and add only §17A's trusted-fleet exception bound to the exact frozen plan artifact — 86 exact successor pairs, 85 projection creations, 760 exact Domain tuples, 80 exact active responsibilities, 99 immutable creator-owned drafts — with per-pair SERIALIZABLE apply, canary-first ordering, fail-closed drift, exact NOOP rerun, and separate production authorization.
- Rejected: modifying the CTO pair; keeping only the single Build in Public pair; arbitrary Principal migration with runtime OLD/NEW arguments; dynamic roster expansion; ordinary reassignment/handoff/delegation; general successor API; online management API; historical rewrite/reactivation; count-forcing.
- Owner input remaining: none.

### DEC-V6-007 — New nodes and owners use one closed model

- Decision owner: `mayf3`.
- Decision: new Definition Versions use exactly `TASK | TERMINAL`; TASK resolves to exactly one enabled `HUMAN | AGENT` Principal, while TERMINAL has no owner. Service remains authentication-only and is rejected at publication/activation.
- Rejected: `HUMAN_TASK`, `AGENT_TASK`, `SERVICE_TASK`, `WAIT_EVENT`, `WAIT_TIMER`, new-node DRAFT/NORMAL, Service-to-Agent conversion, or Scheduler admission for Service.
- Owner input remaining: none.

### DEC-V6-008 — Node Visit is canonical work identity

- Decision owner: `mayf3`.
- Decision: every node entry creates a distinct immutable `nodeVisitId`; activation, work, dispatch, idempotency, wake, reconciliation, and repair bind that identity only.
- Rejected: `nodeKey`, business/display name, DRAFT, `test_env_deploy`, `ops-lock`, metadata, or environment labels as work identity.
- Owner input remaining: none.

### DEC-V6-009 — Exactly one canonical activation

- Decision owner: `mayf3`.
- Decision: active non-terminal TASK Visit has exactly one atomic activation: Human owner -> Human Work Item; Agent owner -> Dispatch Intent; Terminal -> none. Dispatch Intent creation does not start an Agent.
- Rejected: read-time synthetic dispatchability as canonical work, zero/multiple activations, dual Human/Agent activations, or activation after the Visit transaction.
- Owner input remaining: none.

### DEC-V6-010 — `nextEligibleAt` is the only wait primitive

- Decision owner: `mayf3`.
- Decision: every active Dispatch Intent has required `nextEligibleAt`; its initial value is the canonical server-authored activation timestamp generated and persisted in the activation transaction, due means active plus `<= authoritative now`, non-execution stores the next concrete time, and external/dependency/manual wake may only set it to `now` through a controlled command.
- Rejected: client-authored initial time, post-commit timestamp fill, physical/true commit-instant equality as an atomicity-breaking requirement, wait-status enums, reason predicates, timer/event nodes, business-key exceptions, direct Agent start, or another Scheduler wait field.
- Owner input remaining: none.

### DEC-V6-011 — Attempts and leases stay below Workflow syntax

- Decision owner: `mayf3`.
- Decision: retry policy, mapping, delivery, Sessions, and resource leases are external attempt concerns; idempotency binds Visit plus attempt identity and exact request; unknown outcome reconciles the same identity.
- Rejected: lease/reservation as node syntax, blind new-key retry, or lease state as an alternative wait primitive.
- Owner input remaining: none.

### DEC-V6-012 — Activation drives the normal path

- Decision owner: `mayf3`.
- Decision: normal dispatch originates from committed canonical activation. Periodic scans are limited to Reconciler/Watchdog/Repair and may not discover ordinary work by scanning Instances/node keys.
- Rejected: recurring global-list dispatch scan, polling by business node key, or permanent repair path as primary dispatch.
- Owner input remaining: none.

### DEC-V6-013 — Cutover is one-way for new traffic

- Decision owner: `mayf3`.
- Decision: post-barrier new traffic enters the new model directly. A Legacy Instance that already existed before the barrier may continue in bounded DRAIN and append only the Visit, Submission, Event, Receipt/audit, and other accepted Legacy facts necessary to finish that Instance; those facts create no new Legacy Definition or Instance identity. Legacy otherwise remains limited to exact one-time migrate, manually terminate, and historical read-only replay. New Legacy Definition/Instance identity, new-traffic routing or fallback to Legacy, permanent dual track, and silent fallback are forbidden.
- Rejected: indefinite coexistence, automatic fallback on new-path failure, or replay that creates activation/mutates history.
- Owner input remaining: none.

### DEC-V6-014 — External interoperability is locally bounded without PR lifecycle control

- Decision owner: `mayf3` for this local boundary; external repository retains its own acceptance authority.
- Decision: svc-workflow interoperability requires future external periodic recovery to be Reconciler-only and Scheduler management to be separated from normal dispatch. dsh-agent-core may satisfy this through PR #87, a replacement Spec, or another locally lawful authority and exclusively governs all external PR lifecycle actions; PR #87 at `4260911...` remains fixed-coordinate non-authoritative observation/provenance only.
- Rejected: treat PR #87 as active dependency, require one specific amendment/split/merge/closure action, integrate its recurring scanner as the normal path, or govern dsh-agent-core from svc-workflow.
- Owner input remaining: none.

## 20. Normative Contracts

### CTR-V6-001 — Whole-authority lifecycle
V6 MUST remain non-active repository authority while unmerged. Its Owner-accepted branch candidate MUST replace all V5 meaning only through the completed atomic lifecycle transition with V5 backlink/authority-map updates, and V6 MUST NOT authorize implementation or production apply directly.

### CTR-V6-002 — Serial workflow product shape
svc-workflow MUST preserve §5's single-current-node, unique-Visit, single TASK-owner, exactly-one-activation deterministic serial workflow shape and MUST NOT add excluded orchestration capabilities without later authority.

### CTR-V6-003 — Definition lifecycle and immutability
New Definition Versions MUST use only `TASK | TERMINAL`; TASK owner resolution MUST be `HUMAN | AGENT`; Definition ownership, lifecycle, publication immutability, Domain locality, and non-destructive archive MUST otherwise satisfy §5.1.

### CTR-V6-004 — Instance and immutable history
Instance ownership, direct initial activation, non-physical deletion, immutable Context/Visit/Submission/Event facts, canonical Activation facts, projection semantics, and one-version/one-Event atomic state command MUST satisfy §§5.2-5.6.

### CTR-V6-005 — Transition actor and atomicity
Only the authorized current owner/assignee MAY perform normal Transition; global permission/designation MUST NOT imply it, and source-activation closure, target Visit/activation, all workflow facts, outcome, and audit MUST commit atomically.

### CTR-V6-006 — Strict normal Domain isolation
Ordinary Agent/member/Owner access, lookup/list/count/cursor/denial/serialization MUST preserve §6 isolation; only enumerated global permissions MAY cross Domains.

### CTR-V6-007 — Domain-local views and administration
Ordinary worklists, history, Owner views, membership, Definition governance, and Visit snapshots MUST remain bounded as in §6 and MUST NOT inherit global authority.

### CTR-V6-008 — Exactly two split global permissions
Authorization MUST preserve `GLOBAL_SCHEDULER_READ` and `GLOBAL_DOMAIN_ADMIN` independently. The former is the canonical Dispatch Intent read capability; current `GLOBAL_WORKFLOW_READER`/Coordinator global-list compatibility is not a third product permission and MUST NOT automatically grant Dispatch Intent access, Domain-admin, or write authority.

### CTR-V6-009 — Scheduler Dispatch Intent allowlist
The canonical Scheduler record MUST contain only and all §8 identifiers/timestamps and MUST bind work identity to `nodeVisitId`. It MUST NOT contain or require node/business keys or sensitive workflow content.

### CTR-V6-010 — Legacy global-list compatibility is not dispatch
The accepted Domain/global list authorization, population, response, error, and pagination semantics MUST remain unchanged until separately superseded. New Scheduler/Dispatcher normal-path code MUST NOT use those lists or `dispatchable` projection to discover work.

### CTR-V6-011 — Scheduler sensitive-content and business-key exclusion
Dispatch Intent reads MUST exclude Context/title, task label, definition/node/business keys and names, Submission/history, EventData, Assistance content/status/body, metadata, credentials, Receipt/audit content, transition options, and content-derived fields. `DRAFT`, `test_env_deploy`, `ops-lock`, and similar labels MUST have no Scheduler meaning.

### CTR-V6-012 — Domain-admin allowed surface
`GLOBAL_DOMAIN_ADMIN` MUST authorize only idempotent Domain create, atomic initial Owner/disabled fallback, atomic Owner replacement, and §9's minimum selection directory.

### CTR-V6-013 — Domain-admin excluded surface
The permission MUST NOT grant workflow content/write/Transition/cancel/archive/Definition/membership/audit-content/other Domain writes or infer actor from body/Feishu/display/service/scope/allowlist.

### CTR-V6-014 — No self-grant and no self-Owner
The Agent MUST NOT grant itself permission or set its own canonical Principal as Domain Owner, directly or through aliases/chains/retries/migrations; distinct UUIDs remain distinct absent exact accepted linkage authority.

### CTR-V6-015 — Dedicated actual Agent actor
The runtime actor MUST be the exact designated Admin Agent Principal in verified direct-token `sub`; ordinary Agents, Humans, Clients/intermediaries, Feishu senders, bodies, display names, claims, and self-report MUST NOT substitute.

### CTR-V6-016 — Server-side independent authorization and fail closed
Each protected request MUST evaluate active designation and server-side split binding; disabled/revoked Principal or Client, invalid/expired token, missing permission, inactive designation/binding, or unavailable authorization MUST fail closed.

### CTR-V6-017 — Trusted designation root contents and activation
`SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1` MUST contain every field and only closed permissions in §11 and MUST activate only after independent review, Owner acceptance, and merge to main; runtime inputs cannot create it.

### CTR-V6-018 — Designation rotation and emergency lifecycle
Designation has no required expiry; replacement/revocation MUST follow whole-authority successor or emergency disable containment, retain short token/rotatable secret/revocable Client/disableable Principal and binding controls, and provide no break-glass grant.

### CTR-V6-019 — Compromise and replacement order
Credential compromise and Agent replacement MUST follow §11's exact containment and successor order; no replacement Agent MAY receive authority before its merged root successor.

### CTR-V6-020 — Exact Feishu ingress gate
Agent core MUST admit administrative commands only for the exact app, tenant, prebound conversation, allowed sender, verified event/message ID, timestamp/nonce, and replay checks in §12.

### CTR-V6-021 — Feishu provenance is not authorization
Feishu/message facts MUST remain provenance only; svc-workflow MUST authorize only actual Agent Principal/server binding and MUST NOT treat sender ID as actor or Human permission.

### CTR-V6-022 — End-to-end correlation
Durable records MUST correlate the Agent Principal and every Feishu/request/receipt identifier enumerated in §12 without storing sensitive bodies or confusing provenance with authorization.

### CTR-V6-023 — Durable audit coverage
Successful and authenticated-denied Dispatch Intent reads, wake/`nextEligibleAt` mutations, reconciliation/repair, Domain create/Owner replace, and designation/grant/revoke/disable actions MUST produce durable non-sensitive audit identifying the actual authenticated Principal.

### CTR-V6-024 — Audit-before-read and atomic-write failure policy
Protected read audit MUST be durable before data publication; protected writes and audit MUST be atomic; audit/authorization failure MUST release/commit nothing.

### CTR-V6-025 — Revocation/disablement publication barrier
Revocation or disablement before publication/commit MUST prevent older in-flight reads from releasing data and writes from committing; the old Agent MUST cease operating.

### CTR-V6-026 — Idempotency and outcome reconciliation
Same-key/same-request MUST replay; same-key/different-request MUST conflict. Activation/dispatch/wake/repair idempotency MUST bind `nodeVisitId` plus complete request meaning. `outcome_unknown` MUST reconcile only by exact same-Visit/same-attempt/same-request retry, never a blind new key.

### CTR-V6-027 — Retention and sensitive audit exclusion
Required audit MUST be retained exactly 365 days and MUST exclude §13 sensitive content; no product audit-read API or external export is authorized.

### CTR-V6-028 — External ownership and direct-token supply
Auth-service/Agent-core/upper-layer/UI ownership MUST remain as §14 states. Scheduler policy, mapping, delivery, Sessions, and attempt leases require independently accepted external authority. Agent-first permission supply MUST NOT depend on Human PR #15 or legacy PR #2.

### CTR-V6-029 — Layer and storage trust boundary
The technology/layer/storage boundary in §14 MUST be preserved; global security MUST NOT rely on UI filtering, post-read redaction, adapter bypass, or shared-database access.

### CTR-V6-030 — Conformance debt remains unimplemented
No debt listed in §16 MAY be represented as compliant or implementation-authorized until exact accepted child Contracts and Contract-by-Contract evidence establish it.

### CTR-V6-031 — Capability-scoped child authority
Each Slice in §18 MUST have its required accepted authority before implementation and MUST NOT activate, broaden, or waive another Slice. Architecture reconciliation and an accepted local implementation Spec are mandatory before Phase 2 code; no Human-governance common gate exists.

### CTR-V6-032 — Exact one-time successor scope
The retained migration MUST be offline and fixed to §17's exact pair, nine reviewed enabled Domain authorities, and exact live eligible current responsibility; drift MUST commit zero writes.

### CTR-V6-033 — Successor historical immutability and append-only transfer
The migration MUST rewrite zero historical assignments/Visits, preserve the known 58/111 exclusions, and represent successor responsibility only through new Visit/Event/Receipt/Audit facts.

### CTR-V6-034 — Successor atomic NOOP and no durable product surface
The retained migration MUST commit atomically, exact-rerun NOOP with zero writes/audits, fail closed on mismatched metadata/post-state, create no general API/capability, and retain separate implementation/production gates.

### CTR-V6-035 — Fleet exception binds only to the frozen plan
The additive exception MUST authorize only the exact rows of the artifact with `PLAN_SHA256 = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606` (§17A.1); any other bytes, digest mismatch, runtime `OLD`/`NEW` parameter, label-based selection, or roster expansion MUST be rejected before writes.

### CTR-V6-036 — Excluded duplicate identity and canonical pairs
The efficiency duplicate `efficiency-agent`/`d09f8849-073c-484a-978c-f375113c28b2` MUST remain excluded with `EXCLUDED_FUTURE_OPERATOR_WRITES = 0`; only the canonical pair `efficiency-manager` -> `agt_efficiency-agent` MAY transfer efficiency authority, and `blog-agent` MUST pair only with `agt_blog-agent`.

### CTR-V6-037 — Projection creation and worklist terminal state
Each of the 85 missing NEW Workflow projections MUST be created only from the artifact's exact NEW Principal; the already-present `agt_build-in-public-agent` projection MUST exactly match; after creation `workflow_my_tasks` MUST stop returning `principal_not_found` and MUST return HTTP 200 with `items = []` when no current tasks exist.

### CTR-V6-038 — Exact Domain tuple transfer
Only the artifact's 760 exact Domain tuples MAY transfer: `DOMAIN_OWNER` by atomic OLD->NEW replacement without dual Owner, `DOMAIN_MEMBER` by enable-NEW/disable-OLD with Domain and Role unchanged; any tuple drift MUST yield zero writes for that pair with outcome `CONFLICT`.

### CTR-V6-039 — Append-only active responsibility transfer
Only the artifact's 80 exact responsibility tuples MAY transfer, each re-validated at apply time (current, active, non-terminal, not cancelled, not archived, assignee OLD, expected state version matching); apply MUST append same-node successor Visit, dedicated Event, Receipt, and Audit, CAS the state version, preserve Instance and node, and rewrite zero historical facts.

### CTR-V6-040 — Creator-owned draft immutability
All 99 creator-owned draft tuples MUST keep `created_by_principal_id` unchanged (`DRAFT_CREATOR_HISTORY_IMMUTABLE = YES`, `DRAFT_SUCCESSOR_MIGRATION = FORBIDDEN`); any maintainer concept requires a separate future draft-stewardship capability.

### CTR-V6-041 — Per-pair transaction isolation, canary order, exact NOOP
Each pair MUST commit in one independent SERIALIZABLE transaction following the §17A.8 sequence (canary 1 `agt_build-in-public-agent`, canary 2 `agt_efficiency-agent`, then the remaining exact 84); one pair's failure MUST NOT fabricate another pair's success; an exact successful rerun MUST be NOOP with zero writes and zero new audits.

### CTR-V6-042 — Fleet plan-first separate production gate
The complete §17A.8 sequence MUST be enforced before any fleet write: accepted fleet Product Boundary, accepted local implementation Child, independently reviewed operator, production read-only plan recheck, and exact `PLAN_SHA256` review occur before a separate explicit production apply authorization; no earlier milestone authorizes apply.

### CTR-V6-043 — PR disposition without lifecycle change
PR #9 MUST retain disposition `SUPERSEDED_BY_FLEET_LOCAL_CHILD` without being closed, modified, or merged this round. V6's semantic Head MUST receive a fresh independent audit before Owner acceptance; its lifecycle-only final accepted candidate Head MUST receive an independent final-head recheck before merge. `implementation_authority = none` and `production_apply_authority = none` MUST remain unchanged.

### CTR-V6-044 — Closed node and owner taxonomy
New activation MUST accept only TASK owned by an enabled canonical Human or Agent Principal, or TERMINAL with no owner. `HUMAN_TASK`, `AGENT_TASK`, `SERVICE_TASK`, `WAIT_EVENT`, `WAIT_TIMER`, legacy DRAFT/NORMAL, and any other node kind MUST fail before new-flow publication or activation.

### CTR-V6-045 — Service Principal fail-closed boundary
Service Principals MUST remain valid for inter-service authentication but MUST be rejected as new TASK owners. The system MUST NOT auto-convert Service to Agent, create either activation kind, schedule it, or accept caller-supplied type substitution.

### CTR-V6-046 — Node Visit identity
Each entry to a definition node MUST create a distinct immutable `nodeVisitId`, including RETURN/re-entry to the same `nodeId`. Activation, work, dispatch, idempotency, wake, reconciliation, and repair MUST bind `nodeVisitId`, never node/business/environment labels.

### CTR-V6-047 — Exactly-one atomic activation
Creation of an active non-terminal TASK Visit and its one canonical activation MUST be one transaction with an enforceable uniqueness/mutual-exclusion invariant. Any activation failure MUST roll back the Visit/Transition/Instance change; zero or multiple activations are invalid and fail closed.

### CTR-V6-048 — Activation kind and terminal behavior
Human-owned TASK MUST create exactly one Human Work Item; Agent-owned TASK MUST create exactly one Dispatch Intent; TERMINAL MUST create none. Source activation closure and target activation creation MUST be atomic with successful Transition/cancel/manual termination as applicable. Dispatch Intent creation MUST NOT start an Agent.

### CTR-V6-049 — Required `nextEligibleAt`
Every active Dispatch Intent MUST have one required server-authored `nextEligibleAt`. The activation transaction MUST generate one canonical server-authored activation timestamp and persist it in that same transaction as the initial `nextEligibleAt`. The initial value MUST NOT be client-authored or post-commit-filled and MUST NOT require equality to a physical/true commit instant at the cost of atomic Visit/activation creation. Scheduler eligibility MUST be exactly active intent with `nextEligibleAt <= authoritative now`. No second wait field/status/reason or business label MAY affect Scheduler eligibility.

### CTR-V6-050 — Controlled early wake
External event, dependency completion, or authorized manual action MAY only wake by an authenticated, authorized, idempotent command binding current `nodeVisitId` and setting `nextEligibleAt = now`. It MUST NOT create activation, start Agent execution, mutate node/owner, or bypass transition validation; stale/resolved Visit wake MUST have no workflow side effect.

### CTR-V6-051 — Retry, timeout, and unknown outcome
Deterministic non-execution MAY update only the active intent's future `nextEligibleAt` under accepted Scheduler policy plus durable outcome/audit. Timeout/lost response MUST return or record `outcome_unknown`; reconciliation MUST use the exact same Visit/attempt/request identity. Blind new-attempt replay of an unknown outcome is forbidden.

### CTR-V6-052 — Attempt-scoped resource lease
A resource lease MAY protect one external execution attempt but MUST NOT become Workflow node syntax, Visit identity, activation cardinality, or Scheduler wait state. Lease acquisition/loss MUST NOT create a Visit/activation or change the Definition graph.

### CTR-V6-053 — Activation-driven normal path
Committed canonical activation MUST drive ordinary new work. Scheduler/Dispatcher MUST NOT periodically scan Workflow Instances, Domain/global summaries, node keys, or metadata to discover normal work. The bounded delivery mechanism MUST preserve `nodeVisitId` and idempotency without creating a generic event platform.

### CTR-V6-054 — Reconciler/Watchdog/Repair only scans
Periodic scans MAY exist only for Reconciler, Watchdog, or Repair. They MUST detect missing/duplicate/stuck activation/delivery, never branch on business node keys, and repair only through authorized idempotent audited commands that preserve the one canonical activation.

### CTR-V6-055 — One-way new-traffic cutover
At the cutover barrier, all new traffic MUST create only new-model Definitions/Instances/Visits/activations or fail closed. It MUST NOT create a Legacy Definition or Legacy Instance, route or fall back new traffic to Legacy, silently fall back, or establish permanent dual-track authority. A Legacy Instance that existed before the barrier MAY, only in bounded DRAIN, append the Visit, Submission, Event, Receipt/audit, and other accepted Legacy facts required to finish its already-authorized flow; those append-only drain facts MUST NOT create new Legacy Definition or Instance identity.

### CTR-V6-056 — Legacy modes, migration, compatibility, and rollback
Legacy behavior after cutover MUST be limited to bounded drain of pre-barrier Instances, exact one-time migrate, manually terminate, and historical read-only replay. DRAIN MAY append only the accepted Legacy Visit, Submission, Event, Receipt/audit, and other facts necessary to finish an already-existing Instance, MUST preserve their immutable history, and MUST create no new Legacy Definition or Instance identity. Migration MUST be plan-bound, idempotent, append-only, and create one target Visit/activation without rewriting history. Replay MUST write nothing and schedule nothing. Rollback MAY pause/contain the new path but MUST NOT route new traffic back to Legacy or fabricate a Legacy Definition/Instance for new traffic.

### CTR-V6-057 — External interoperability and PR #87 provenance
svc-workflow MUST accept external dispatch interoperability only when periodic external recovery is Reconciler-only and Scheduler management is separated from normal dispatch under lawful external authority. dsh-agent-core MAY satisfy those conditions by amending PR #87, adopting a replacement Spec, or using another locally lawful authority. PR #87 at `4260911...` MUST remain a fixed-coordinate non-authoritative observation/provenance reference here; this repository MUST NOT prescribe, edit, accept, amend, split, close, merge, supersede, or implement that external PR or authority lifecycle.

### CTR-V6-058 — Parent, Architecture, and implementation lifecycle
V6 authorizes no implementation. Code may start only after V6 is accepted on main, conflicting Architecture is reconciled by accepted authority, and an independently reviewed accepted implementation Spec with `implementation_authority: contracts` is present in the implementation base. Current PR #19 / `SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1@0c63d35...` and its `af450aa...` source lineage MUST NOT satisfy that gate or be accepted/implemented as this Goal's child/canonical dispatch authority. V6 MUST NOT modify or control PR #19's owner-governed lifecycle.

## 21. Acceptance

Every item requires executed evidence at the implementation/authority revision named by its child; a test definition or prose assertion alone is not evidence.

### ACC-V6-001 — Lifecycle and supersession check
- Contracts: `CTR-V6-001`.
- Method/environment: repository frontmatter/backlink/map review on the reviewed semantic Head and lifecycle-only final accepted candidate Head.
- Expected: the Owner-accepted unmerged V6 candidate is inactive repository authority; the acceptance transaction changes V5 backlink/map atomically while implementation and production apply authority remain none; activation requires independently rechecked merge to `main`.
- Required evidence: exact Git diffs, reviewed commits, Owner receipt, final-head recheck, and main merge coordinate.
- Failure condition: the unmerged accepted V6 candidate is called active repository authority, V5 is partially superseded, or implementation/production apply is authorized.

### ACC-V6-002 — Serial-shape negative matrix
- Contracts: `CTR-V6-002`.
- Method/environment: child-spec contract and executable capability matrix.
- Expected: only §5 serial shape is available, with unique Visit and one activation per active TASK.
- Required evidence: exact endpoint/domain tests and implementation mapping.
- Failure condition: multiple current nodes/owners/activations or excluded parallel/dynamic/claim/timer/SLA/script/LLM/general reassignment capability becomes available.

### ACC-V6-003 — Definition lifecycle matrix
- Contracts: `CTR-V6-003`.
- Method/environment: integration tests over Definition lifecycle, new node/owner types, Principal resolution, and cross-Domain attempts.
- Expected: only new `TASK | TERMINAL` nodes publish/activate; TASK resolves to Human/Agent; lifecycle/immutability/Domain ownership holds.
- Required evidence: executed state matrix and storage diff.
- Failure condition: any excluded node/owner publishes/activates, published bytes mutate, invalid state creates Instances, archive rewrites facts, or Definition is shared cross-Domain.

### ACC-V6-004 — Instance/history integrity
- Contracts: `CTR-V6-004`.
- Method/environment: transactional and history-replay integration tests.
- Expected: no physical delete; immutable facts/activation/projection agree; one version/Event per success; initial Visit and activation commit together.
- Required evidence: executed queries and commit coordinates.
- Failure condition: fact rewrite/delete, orphan Visit/activation, partial commit, duplicate/missing Event, or projection/history divergence.

### ACC-V6-005 — Transition authority matrix
- Contracts: `CTR-V6-005`.
- Method/environment: test current Human/Agent owner, Owner, globally authorized Agent, and unrelated Principal across TASK->TASK, TASK->TERMINAL, RETURN, cancel, and manual terminate.
- Expected: only current owner succeeds normally; source/target activation and facts commit atomically.
- Required evidence: executed auth matrix, activation rows, transaction trace, and database audit.
- Failure condition: Admin/global permission transitions solely by status, source activation remains active, target activation is missing/duplicate, terminal gets activation, or partial facts commit.

### ACC-V6-006 — Cross-Domain noninterference
- Contracts: `CTR-V6-006`.
- Method/environment: ordinary Agent/Owner lookup/list/count/cursor/error/serialization matrix across two Domains.
- Expected: no cross-Domain fact or existence leak.
- Required evidence: executed responses and query/audit traces.
- Failure condition: an ordinary Agent or role combination obtains global access or any observable cross-Domain leak.

### ACC-V6-007 — Domain-local view boundary
- Contracts: `CTR-V6-007`.
- Method/environment: worklist/history/Owner/admin test matrix including Owner replacement.
- Expected: views remain participation/Domain scoped and old Visit snapshots remain unchanged.
- Required evidence: executed responses and immutable rows.
- Failure condition: old Owner retains access, another Domain appears, or Owner change rewrites Visit.

### ACC-V6-008 — Split-permission/current-role matrix
- Contracts: `CTR-V6-008`.
- Method/environment: product-permission review plus current global-list and future Dispatch Intent gate tests for Reader, Coordinator, explicit Scheduler binding, combinations, and neither.
- Expected: Reader/Coordinator compatibility remains bounded to current surfaces; only an accepted explicit Scheduler binding reaches Dispatch Intents; no role implies Domain-admin/write.
- Required evidence: accepted authority graph, executed allow/deny matrix, and role-binding diff.
- Failure condition: a third product permission appears, current Reader/Coordinator silently gains Dispatch Intent/write reach, or Scheduler read implies Domain admin.

### ACC-V6-009 — Dispatch Intent scheduler allowlist
- Contracts: `CTR-V6-009`.
- Method/environment: wire/schema/property tests with sensitive and business-key canaries.
- Expected: exactly §8 fields, canonical `nodeVisitId`, no node/business key or content.
- Required evidence: executed response-key snapshots, schema scanner, and marker scan.
- Failure condition: missing required identity/time, extra field, nodeKey/business label, or sensitive content appears.

### ACC-V6-010 — Legacy list compatibility/non-dispatch check
- Contracts: `CTR-V6-010`.
- Method/environment: run current Domain/global list authorization/default/pagination golden tests plus scheduler-source/call-graph scan.
- Expected: existing responses/errors remain compatible; new Scheduler normal path consumes no Domain/global summary or dispatchable query.
- Required evidence: executed golden diff and source/capability graph.
- Failure condition: legacy route changes without authority or Scheduler discovers normal work from the old list.

### ACC-V6-011 — Scheduler sensitive/business-key exclusion
- Contracts: `CTR-V6-011`.
- Method/environment: seeded markers in every forbidden source plus code/capability scans for nodeKey, DRAFT, `test_env_deploy`, and ops-lock branching.
- Expected: no forbidden content or business-key dependency reaches Scheduler/Dispatcher/Reconciler/Repair decisions.
- Required evidence: response corpus, marker/key scanner, and call graph.
- Failure condition: forbidden content appears or any business/metadata label changes scheduling/repair behavior.

### ACC-V6-012 — Domain-admin allowed operation matrix
- Contracts: `CTR-V6-012`.
- Method/environment: create with Owner, disabled fallback, replace Owner, and directory integration tests.
- Expected: only enumerated operations/fields work; Owner invariant is atomic.
- Required evidence: executed responses, rows, and audits.
- Failure condition: enabled ownerless Domain, partial replacement, extra directory data, or non-idempotent replay.

### ACC-V6-013 — Domain-admin excluded operation matrix
- Contracts: `CTR-V6-013`.
- Method/environment: attempt every excluded read/write using only global Domain-admin permission.
- Expected: all are denied without side effect or sensitive audit content.
- Required evidence: executed denial matrix and unchanged-state proof.
- Failure condition: workflow content/transition/reassignment/cancel/archive/Definition/membership/audit-content/other write succeeds.

### ACC-V6-014 — Self-grant/self-Owner attacks
- Contracts: `CTR-V6-014`.
- Method/environment: direct, alias, chained, retry, migration, same-UUID, distinct-UUID, and unproven-linkage cases.
- Expected: self-grant/same-Principal Owner denied; distinct Principals allowed unless exact accepted linkage applies.
- Required evidence: executed cases and canonical identity traces.
- Failure condition: Agent self-grants, sets itself Owner, evades via alias, or implementation invents common control.

### ACC-V6-015 — Actual actor anti-forgery matrix
- Contracts: `CTR-V6-015`.
- Method/environment: valid dedicated token plus ordinary Agent/Human/Client/Feishu/body/display/JWT-role/tool-argument forgery attempts.
- Expected: only exact token `sub` is actor; only designated Agent can proceed.
- Required evidence: verified claims, authorization trace, and durable audit actor.
- Failure condition: ordinary Agent gains global permission or any request body/self-report/Feishu field substitutes as admin actor.

### ACC-V6-016 — Disabled/revoked fail-closed matrix
- Contracts: `CTR-V6-016`.
- Method/environment: disable/revoke each Principal, Client, token, designation, binding, and permission; inject authorization-store failure.
- Expected: protected operation denies and releases/commits nothing.
- Required evidence: executed responses, publication checks, and state/audit traces.
- Failure condition: disabled Agent still reads/writes or unavailable authorization fails open.

### ACC-V6-017 — Root authority activation gate
- Contracts: `CTR-V6-017`.
- Method/environment: repository lifecycle and malformed-authority negative tests.
- Expected: all exact fields/closed permissions present; proposed/unmerged/runtime-created roots are inert.
- Required evidence: exact review/acceptance/merge and activation trace.
- Failure condition: runtime API/message/self-claim activates designation, missing field passes, or an unmerged authority grants access.

### ACC-V6-018 — Designation lifecycle controls
- Contracts: `CTR-V6-018`.
- Method/environment: token expiry, secret rotation, Client revoke, Principal/binding disable, attempted break-glass and runtime replacement.
- Expected: controls fail closed; only successor activates replacement; no break-glass exists.
- Required evidence: executed lifecycle matrix and audits.
- Failure condition: indefinite token, unrotatable/unrevocable credential, runtime grant/replacement, or break-glass authority.

### ACC-V6-019 — Compromise/replacement sequencing
- Contracts: `CTR-V6-019`.
- Method/environment: staged incident and Agent replacement rehearsal.
- Expected: old Agent stops before replacement; replacement remains denied until merged successor.
- Required evidence: ordered timestamps, revocation/binding/audit/root/main coordinates.
- Failure condition: old Agent continues after replacement/revoke or new Agent acts before successor merge.

### ACC-V6-020 — Feishu exact-ingress matrix
- Contracts: `CTR-V6-020`.
- Method/environment: vary app, tenant, conversation, sender, event signature/ID, timestamp, nonce, and replay.
- Expected: only exact fresh verified single-user ingress is admitted once.
- Required evidence: executed Agent-core gate matrix and ingress audit.
- Failure condition: non-owner sender, wrong app/tenant/conversation, stale/duplicate/unsigned event reaches command execution.

### ACC-V6-021 — Feishu provenance/actor separation
- Contracts: `CTR-V6-021`.
- Method/environment: send valid command while forging sender/body actor and compare token/audit identity.
- Expected: Agent Principal remains actor; Feishu values remain provenance.
- Required evidence: token verification, svc-workflow auth trace, audit record.
- Failure condition: Feishu sender ID becomes svc-workflow actor/permission or Human OBO is required.

### ACC-V6-022 — Correlation completeness
- Contracts: `CTR-V6-022`.
- Method/environment: end-to-end accepted and denied Feishu commands.
- Expected: all enumerated IDs correlate without sensitive bodies.
- Required evidence: joined durable records and redaction scan.
- Failure condition: missing correlation edge, actor/provenance conflation, or sensitive content copied.

### ACC-V6-023 — Protected audit coverage
- Contracts: `CTR-V6-023`.
- Method/environment: success/denial matrix for Dispatch Intent read, wake/time update, reconciliation/repair, Domain create/replace, designate/revoke/disable.
- Expected: one durable accountability record per required attempt with actual authenticated actor and bound `nodeVisitId` where applicable.
- Required evidence: executed operation/audit joins.
- Failure condition: required success/denial lacks durable audit or records a self-reported actor.

### ACC-V6-024 — Audit failure and atomicity
- Contracts: `CTR-V6-024`.
- Method/environment: inject audit failure before read publication and during write transaction.
- Expected: read returns no protected data; write and audit both roll back.
- Required evidence: network publication capture and database transaction proof.
- Failure condition: audit fails but data is returned or write commits without audit.

### ACC-V6-025 — Publication/commit revocation race
- Contracts: `CTR-V6-025`.
- Method/environment: pause requests after initial check, revoke/disable, then release.
- Expected: no response data/no protected commit; old Agent cannot operate.
- Required evidence: synchronized race trace and state/audit results.
- Failure condition: prechecked in-flight request publishes/commits after revoke/disable.

### ACC-V6-026 — Idempotency/unknown-outcome matrix
- Contracts: `CTR-V6-026`.
- Method/environment: same/different Visit/attempt/request-key concurrency and induced lost response before/after commit.
- Expected: replay/conflict/exact same-Visit/same-attempt/same-request reconciliation semantics.
- Required evidence: executed receipts, hashes, outcomes, and row counts.
- Failure condition: Visit identity is omitted/replaced by business key, same-key/different-request mutates, request double-commits, or unknown outcome retries with a new identity.

### ACC-V6-027 — Audit retention/redaction
- Contracts: `CTR-V6-027`.
- Method/environment: retention boundary and seeded-sensitive-marker scan.
- Expected: exact 365-day policy; no forbidden body/credential; no runtime read/export API.
- Required evidence: lifecycle execution and API/schema inventory.
- Failure condition: early deletion, configurable longer retention under V6, sensitive content, or audit read/export surface.

### ACC-V6-028 — External authority and PR disposition
- Contracts: `CTR-V6-028`.
- Method/environment: child-authority dependency graph review at exact revisions.
- Expected: Agent permission child is independent; Scheduler policy/mapping/delivery/lease stays externally owned; PR #15 remains deferred/non-prerequisite/non-active; PR #2 independent.
- Required evidence: exact external authority/PR/main coordinates.
- Failure condition: local Spec governs external execution behavior, unmerged PR becomes active/prerequisite, or external ownership is imported by implementation.

### ACC-V6-029 — Layer/trust bypass scan
- Contracts: `CTR-V6-029`.
- Method/environment: architecture/source query path and direct-storage/adaptor attack review.
- Expected: authorization/redaction enforced before broad read and through application/store boundaries.
- Required evidence: call graph, query projection, access tests.
- Failure condition: UI/handler-only redaction, adapter bypass, or direct shared-DB mutation authorizes behavior.

### ACC-V6-030 — Drift truth check
- Contracts: `CTR-V6-030`.
- Method/environment: exact-base conformance report against each debt item.
- Expected: unresolved items remain DRIFTED/UNKNOWN, never VERIFIED without qualified evidence.
- Required evidence: Contract-level conformance table.
- Failure condition: existing partial implementation is declared V6-compliant by existence, tests, or runtime alone.

### ACC-V6-031 — Slice non-escalation graph
- Contracts: `CTR-V6-031`.
- Method/environment: authority/dependency review plus attempted implementation bases missing V6, Architecture reconciliation, local implementation Spec, or external authority.
- Expected: each Slice enables only itself; every missing own authority fails preflight; unrelated Human/Assistance/Recovery authority does not become a common gate.
- Required evidence: accepted authority graph, implementation-base SHAs, and preflight results.
- Failure condition: code begins from an incomplete authority base, one Slice silently activates another, or an unrelated common gate is imposed.

### ACC-V6-032 — Exact successor scope drift
- Contracts: `CTR-V6-032`.
- Method/environment: reviewed plan, exact pair/nine rows, changed pair/row/live-current fixtures.
- Expected: only exact eligible plan succeeds; every drift commits zero.
- Required evidence: executed plan digest, pre/post rows, Receipt/audit.
- Failure condition: arbitrary pair, non-nine Domain scope, historical/ineligible responsibility, or drift commits.

### ACC-V6-033 — Successor history preservation
- Contracts: `CTR-V6-033`.
- Method/environment: pre/post digest and row-level history comparison.
- Expected: 58 historical assignments and 111 Visits unchanged; only new successor facts appended.
- Required evidence: executed digests, row counts, new fact lineage.
- Failure condition: any historical attribution is updated/deleted/relabeled or successor lacks append-only facts.

### ACC-V6-034 — Successor atomic NOOP/surface/gates
- Contracts: `CTR-V6-034`.
- Method/environment: failure injection, exact rerun, mismatched rerun, API/SDK inventory, production-gate review.
- Expected: all-or-nothing; exact rerun zero writes/audits; no general surface; production remains separately gated.
- Required evidence: executed transactions, counts, surface diff, gate record.
- Failure condition: partial commit, rerun side effect, reusable reassignment surface, or Spec/merge alone authorizes production.

### ACC-V6-035 — Frozen-plan binding negative matrix
- Contracts: `CTR-V6-035`.
- Method/environment: child/operator review using exact artifact bytes, byte-modified plan, mismatched digest, runtime-supplied OLD/NEW, and label/renamed-account fixtures.
- Expected: only the exact frozen artifact rows are selectable; every other input fails before writes.
- Required evidence: executed digest checks, source constants, and negative matrix transcript.
- Failure condition: any non-artifact identity or parameter reaches a write path.

### ACC-V6-036 — Excluded identity and canonical pair check
- Contracts: `CTR-V6-036`.
- Method/environment: operator fixtures for the excluded duplicate, canonical efficiency pair, Build in Public pair, and blog pair.
- Expected: excluded identity commits zero writes; efficiency transfers only via the canonical pair; blog and Build in Public pairs stay independent.
- Required evidence: executed matrix with pre/post row digests.
- Failure condition: the duplicate receives any write, or cross-pair confusion occurs.

### ACC-V6-037 — Projection creation and worklist terminal state
- Contracts: `CTR-V6-037`.
- Method/environment: disposable projection store with the 85 missing and 1 present identities.
- Expected: 85 projections created from exact artifact Principals; the present one exact-matched; `workflow_my_tasks` returns HTTP 200 with `items = []` (or real tasks) and no `principal_not_found`.
- Required evidence: created-row digests and executed worklist responses.
- Failure condition: a 87th/dynamic identity, display-name pairing, excluded-identity creation, or residual 404.

### ACC-V6-038 — Domain tuple exactness and conflict
- Contracts: `CTR-V6-038`.
- Method/environment: disposable Domain fixtures with exact, missing, extra, disabled, role-changed, and Principal-changed tuples.
- Expected: the exact 760 transfer atomically per pair; every drift yields zero writes with `CONFLICT`.
- Required evidence: pre/post tuples, transaction logs, and outcome records.
- Failure condition: dual Owner, long-lived dual member authority, Domain/Role change, or drift commits.

### ACC-V6-039 — Responsibility append-only and history immutability
- Contracts: `CTR-V6-039`.
- Method/environment: disposable current/terminal/cancelled/archived/state-version-mismatch fixtures.
- Expected: only the 80 re-validated exact tuples append successor Visit/Event/Receipt/Audit with CAS; all historical facts remain byte-identical.
- Required evidence: before/after history digests, new fact lineage, and CAS outcomes.
- Failure condition: ineligible reactivation, missing dedicated fact, wrong Instance/node, or any historical rewrite.

### ACC-V6-040 — Draft creator immutability
- Contracts: `CTR-V6-040`.
- Method/environment: all 99 draft tuples in a disposable store with a candidate successor operator run.
- Expected: zero `created_by_principal_id` changes and zero draft migrations.
- Required evidence: pre/post draft digests.
- Failure condition: any creator field rewrite or silent maintainer overwrite.

### ACC-V6-041 — Canary sequence and per-pair isolation
- Contracts: `CTR-V6-041`.
- Method/environment: full fleet rehearsal on a disposable store with injected per-pair failure and exact rerun.
- Expected: canary order holds; each pair commits independently SERIALIZABLE; one failure never fabricates another pair's success; exact rerun is a zero-write NOOP.
- Required evidence: ordered transcripts, per-pair transaction records, and rerun counts.
- Failure condition: pair writes merge, failure leaks into other pairs' outcomes, or rerun mutates.

### ACC-V6-042 — Fleet production gate sequence
- Contracts: `CTR-V6-042`.
- Method/environment: authority/implementation/plan/execution-record lifecycle review for the fleet apply.
- Expected: all ordered gates occur before any write; exact `PLAN_SHA256` is re-reviewed against the live recheck; apply lacks authority until the separate exact production authorization.
- Required evidence: exact commits, plan bytes/SHA, review receipt, execution authorization, and apply/verify/NOOP transcript.
- Failure condition: any earlier milestone implies apply, a write precedes the reviewed rechecked plan, or production apply is derived from acceptance alone.

### ACC-V6-043 — PR disposition and lifecycle invariance
- Contracts: `CTR-V6-043`.
- Method/environment: GitHub PR state plus V6 frontmatter and fresh audit record.
- Expected: PR #9 remains open/unmodified/unmerged with its disposition; V6 is an Owner-accepted unmerged branch candidate with no implementation/production authority and requires independent final-head recheck before merge.
- Required evidence: PR snapshots, lifecycle diff, and audit record.
- Failure condition: PR #9 changes, unmerged V6 is treated as active repository authority, or either independent semantic review or final-head recheck is skipped.

### ACC-V6-044 — Closed node/owner matrix
- Contracts: `CTR-V6-044`.
- Method/environment: publish and activate every allowed/excluded node kind and owner combination.
- Expected: only TASK+Human, TASK+Agent, and TERMINAL+no-owner pass their applicable phase.
- Required evidence: executed Definition validation, activation results, and zero-write denial proofs.
- Failure condition: any excluded node/owner publishes or reaches activation.

### ACC-V6-045 — Service owner rejection
- Contracts: `CTR-V6-045`.
- Method/environment: valid Service token/auth calls plus Definition/body/display/role/scope attempts to assign that Principal to TASK.
- Expected: service authentication remains valid for service calls; every TASK-owner activation attempt rejects before Visit/activation/Scheduler publication.
- Required evidence: verified identity traces, responses, transaction row counts, and Scheduler census.
- Failure condition: Service becomes Human/Agent, receives activation, or appears in due work.

### ACC-V6-046 — Re-entry and business-key negative matrix
- Contracts: `CTR-V6-046`.
- Method/environment: enter the same `nodeId` three times via RETURN; vary nodeKey/display/DRAFT/test_env_deploy/ops-lock metadata while holding Visit facts constant.
- Expected: three distinct Visits/activations; scheduling/idempotency behavior depends only on exact Visit identity.
- Required evidence: Visit/activation IDs, receipts, and negative source/runtime trace.
- Failure condition: Visit reused, old Visit reopened, or any label changes identity/behavior.

### ACC-V6-047 — Atomic activation fault matrix
- Contracts: `CTR-V6-047`.
- Method/environment: inject owner-resolution, uniqueness, activation-write, Event, Receipt, and audit failures during Instance creation and Transition.
- Expected: complete Visit+activation transaction or zero commit; uniqueness rejects duplicates.
- Required evidence: transaction traces, constraint results, and before/after row census.
- Failure condition: orphan Visit, missing activation, multiple activation, or partial projection/Event commit.

### ACC-V6-048 — Human/Agent/Terminal activation matrix
- Contracts: `CTR-V6-048`.
- Method/environment: TASK->TASK and TASK->TERMINAL paths across Human/Agent owners, plus cancel/manual terminate races.
- Expected: exact activation kind/cardinality, atomic source closure/target creation, terminal zero, no Agent start on creation.
- Required evidence: activation rows, execution-delivery audit, and transaction order.
- Failure condition: wrong/dual activation, terminal activation, stale source active, or Agent run begins from creation alone.

### ACC-V6-049 — `nextEligibleAt` clock/wait matrix
- Contracts: `CTR-V6-049`.
- Method/environment: activation transactions with server clock control, attempted client timestamp injection, an induced delay between transaction timestamp generation and physical commit, post-commit-write fault injection, past/equal/future timestamps, clock-boundary concurrency, and alternative-field canaries.
- Expected: the activation transaction generates one canonical server-authored activation timestamp and persists it atomically as initial `nextEligibleAt`; no physical commit-instant equality is required; due iff active and timestamp <= authoritative now; no other field affects due selection.
- Required evidence: transaction trace, canonical activation timestamp and persisted intent rows, zero post-commit timestamp writes, ordered due-query results, and query/source proof.
- Failure condition: null/missing time, client-authored value, post-commit fill, Visit/activation atomicity weakened to obtain a commit instant, another predicate, or early/late selection relative to the persisted canonical activation timestamp.

### ACC-V6-050 — Controlled wake matrix
- Contracts: `CTR-V6-050`.
- Method/environment: authorized/unauthorized external event, dependency, and manual wake; same-key replay; stale/resolved Visit; concurrent Transition.
- Expected: authorized current wake sets now once with audit; unauthorized/stale paths cause no workflow side effect; no Agent starts directly.
- Required evidence: requests, receipts, actor auth, before/after rows, audit, and delivery trace.
- Failure condition: arbitrary time/owner/node mutation, duplicate write, stale wake, bypass, or direct execution.

### ACC-V6-051 — Retry/timeout/outcome-unknown matrix
- Contracts: `CTR-V6-051`.
- Method/environment: deterministic non-execution, lost response before/after commit, same and changed attempt identities.
- Expected: deterministic path updates future time with audit; unknown outcome reconciles exact identity; changed identity cannot duplicate work.
- Required evidence: attempt receipts, timestamps, outcomes, and execution counts.
- Failure condition: second wait state, blind new-key retry, duplicate run, or unknown commit misreported deterministically.

### ACC-V6-052 — Lease separation
- Contracts: `CTR-V6-052`.
- Method/environment: acquire/lose/expire/conflict an attempt lease while observing Definition, Visit, activation, and due time.
- Expected: lease affects only attempt/resource mutual exclusion; Workflow syntax/identity/cardinality remains unchanged.
- Required evidence: lease and workflow-store diffs plus source ownership map.
- Failure condition: lease becomes node/Visit/wait state or creates/closes activation.

### ACC-V6-053 — Activation-driven path proof
- Contracts: `CTR-V6-053`.
- Method/environment: create an Agent TASK Visit with all periodic reconcilers disabled; trace normal delivery to Scheduler and scan source/capabilities.
- Expected: the committed intent reaches bounded Scheduler intake by `nodeVisitId`; no global/Instance/business-key scan is required; no generic event platform is introduced.
- Required evidence: end-to-end trace, call graph, job inventory, and changed-component ownership.
- Failure condition: work waits for a periodic discovery scan, nodeKey is consumed, or a generic platform is built.

### ACC-V6-054 — Reconciler/Watchdog/Repair boundary
- Contracts: `CTR-V6-054`.
- Method/environment: seed missing, duplicate, stuck-delivery, stale, and healthy activations; run each periodic component.
- Expected: only anomalies are detected/repaired, canonical Visit preserved, healthy work not redispatched, all repairs idempotent/audited.
- Required evidence: before/after invariant census, receipts, audits, and job definitions.
- Failure condition: ordinary work discovery/dispatch, business-key branching, second activation, or unaudited repair.

### ACC-V6-055 — New-traffic cutover barrier
- Contracts: `CTR-V6-055`.
- Method/environment: concurrent new-traffic requests across the exact cutover barrier with new-path failure injection, plus a pre-barrier Legacy Instance completing one bounded DRAIN transition after the barrier.
- Expected: every post-barrier new-traffic request creates only new-model facts or fails closed and never routes to Legacy; the pre-barrier Instance may append only its required accepted Legacy Visit/Submission/Event/Receipt/audit facts without creating a Legacy Definition or Instance identity.
- Required evidence: ordered route logs, barrier coordinate, pre-barrier Legacy Instance identity, Definition/Instance identity census, and classified Visit/Submission/Event/Receipt/audit/activation write census.
- Failure condition: new traffic creates/falls back to a Legacy Definition or Instance, permanent dual route or silent fallback exists, drain creates new Legacy identity, or the valid bounded drain is rejected merely because its required append-only facts commit after the barrier.

### ACC-V6-056 — Legacy mode and rollback matrix
- Contracts: `CTR-V6-056`.
- Method/environment: drain, exact one-time migration/rerun/drift, manual terminate, historical replay, new-path rollback/containment.
- Expected: only four allowed Legacy modes; bounded DRAIN of a pre-barrier Instance may append only necessary accepted Legacy Visit/Submission/Event/Receipt/audit facts and no new Legacy identity; migration is append-only/one activation/NOOP rerun; replay has zero writes; rollback never creates or routes a Legacy Definition/Instance for new traffic.
- Required evidence: pre-barrier Instance and Definition identities, classified drain write set, plan digest, before/after immutable history, activation rows, receipts/audits, full write census, and route state.
- Failure condition: drain creates new Legacy Definition/Instance identity or unrelated writes, valid necessary drain facts are categorically blocked, history is rewritten, migration repeats, replay writes/schedules, a forbidden Legacy operation occurs, or new traffic falls back.

### ACC-V6-057 — External interoperability/PR #87 ownership review
- Contracts: `CTR-V6-057`.
- Method/environment: exact-head review of the svc-workflow authority/integration graph and whichever lawful dsh-agent-core authority is presented for interoperability; retain PR #87 head `4260911...` only as fixed observation/provenance.
- Expected: local repo contains no external implementation or lifecycle command; qualified external behavior is Reconciler-only for periodic recovery and separates Scheduler management from normal dispatch; dsh-agent-core chose and governed its own authority/PR path.
- Required evidence: local changed-file list, fixed observation coordinate, exact accepted external authority/revision when later available, independent external review/acceptance records, and integration call/job graph.
- Failure condition: proposed PR is treated active, svc-workflow requires a specific PR amendment/split/merge/closure, recurring scanning becomes the normal path, Scheduler management remains coupled to normal dispatch, or the local repo modifies external code/authority.

### ACC-V6-058 — Authority-chain implementation gate
- Contracts: `CTR-V6-058`.
- Method/environment: preflight on candidate implementation bases with each required authority present/absent and with current PR #19 head `0c63d35...` or its `af450aa...` source lineage substituted.
- Expected: only a base containing accepted V6, accepted Architecture reconciliation, and an independently accepted implementation Spec permits code; the current dispatchability proposal and its source lineage always fail this Goal's child/canonical-authority gate, while V6 takes no action on PR #19's lifecycle.
- Required evidence: exact V6 and implementation base/head SHAs, PR #19 base/head/state/draft/unmerged coordinates, authority statuses, review/acceptance records, repository-owner lifecycle provenance, and preflight output.
- Failure condition: code starts from proposed/missing/conflicting authority; current PR #19 or `af450aa...` is accepted or implemented as this Goal's child/canonical dispatch authority; or V6 is used to modify, close, merge, accept, or otherwise control PR #19.

```text
CONTRACT_COUNT = 58
CONTRACTS_WITH_ACCEPTANCE = 58
ACCEPTANCE_COUNT = 58
DANGLING_CONTRACT_REFERENCES = 0
UNCOVERED_CONTRACTS = 0
ACCEPTANCE_WITHOUT_FAILURE_CONDITION = 0
```

## 22. Alternatives and disposition

- Retain V2 Human root/two-person grant/Human OBO prerequisite: rejected for single-user Agent-first V1; future multi-Human governance remains possible through later authority.
- Reuse an existing business/canary Agent: forbidden by default because administrative authority needs a dedicated identity and credential lifecycle.
- Add `GLOBAL_WORKFLOW_COORDINATOR` composite authorization: rejected; it destroys independent permission reasoning and enables alias bypass.
- Treat Feishu owner identity as svc-workflow Human actor: rejected; transport provenance is not token identity or permission.
- Runtime grant API/Agent self-grant: rejected; designation is repository-owned and reviewed.
- Auto-expiring designation: not required; short token/rotation/revoke/disable controls plus whole-authority replacement are selected.
- Partial V2 amendment: rejected in V3; identity/governance meaning changed and V0 permits only whole-authority supersession.
- Directly amend accepted V3 for the fleet: rejected; successor-scope meaning changes, so V4 is a whole-authority successor.
- Generalize CTO/fleet into arbitrary Principal migration: rejected; each exception remains independently exact and bounded, and the fleet exception stays digest-frozen.
- Keep the single Build in Public pair as the entire exception: superseded; the frozen plan v2 artifact supersedes plan v1, and Draft PR #9 is dispositioned `SUPERSEDED_BY_FLEET_LOCAL_CHILD`.
- Accept runtime OLD/NEW arguments or dynamic roster expansion: rejected; a future operator binds only to exact artifact rows.
- Use PR #15 or mutable PR #2 as common prerequisites: rejected; both are independent from Agent-first V1.
- Reuse or amend accepted V5: rejected; node taxonomy, persistence, Scheduler, wake, lease, and cutover meaning changes, requiring whole-authority `SUPERSEDE`.
- Accept or implement current PR #19 `SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1@0c63d35...` (source lineage `af450aa...`) as this Goal's child/canonical dispatch authority: rejected; it is open, draft, unmerged, proposed, V5-bound, read-time/non-persisted, and creates neither canonical activation nor `nextEligibleAt`. Any retained query semantics require later lawful re-investigation/rewrite, and V6 does not control the PR's repository-owner lifecycle.
- Preserve DRAFT/NORMAL for new traffic: rejected; those node kinds are Legacy-only after cutover.
- Create separate HUMAN/AGENT/SERVICE/WAIT node kinds: rejected; node kind remains `TASK`, with owner type resolving Human/Agent.
- Auto-convert Service to Agent or schedule Service TASKs: forbidden; inter-service authentication does not create workflow ownership.
- Bind work to nodeKey, business name, DRAFT, `test_env_deploy`, or `ops-lock`: forbidden; only `nodeVisitId` identifies runtime work.
- Retain read-time dispatchability as canonical work: rejected; canonical activation is durable and atomic with Visit entry.
- Add wait status/reason/timer/event primitives beside `nextEligibleAt`: rejected; Scheduler wait semantics must remain singular.
- Put resource lease in Workflow syntax: rejected; it belongs to execution attempt/resource mutual exclusion.
- Run recurring global scans as normal dispatch: rejected; only Reconciler/Watchdog/Repair may scan periodically.
- Permanently dual-run new and Legacy models or silently fall back new traffic: forbidden; bounded append-only completion facts for a pre-barrier Legacy Instance are DRAIN, not dual-run or new Legacy identity.
- Build Kafka/Outbox/general event platform, GitHub App, WORM, WebAuthn, generic Operator, or rewrite three repositories: out of scope and rejected for this Goal.
- Accept external recurring scans as normal dispatch or integrate Scheduler management inseparably with normal dispatch: rejected by the local interoperability boundary. Whether dsh-agent-core amends PR #87, replaces it, or uses another lawful authority is exclusively that repository's choice.

## 23. Migration, compatibility, containment, and rollback

The V6 acceptance transaction is docs-only and mutates no runtime. It preserves V5's exact principal-successor migrations, trusted-fleet boundary, security containment, current read-route compatibility, and production-gate separation. Existing `GLOBAL_WORKFLOW_READER`/Coordinator bindings gain no Dispatch Intent authority; no role/grant migration occurs here.

### 23.1 New-traffic cutover barrier

The later implementation/cutover authority must define one exact, auditable barrier after which every new-traffic Definition Version and Instance uses the new model. The barrier must be atomic from the caller-routing perspective: a new-traffic request either enters the new path or fails closed. A new-path failure never falls back to Legacy. The barrier does not prohibit an already-existing pre-barrier Legacy Instance from appending the bounded facts required by DRAIN. The exact timestamp/release/config coordinate is operational evidence, not a new Product Direction choice.

### 23.2 Permitted Legacy modes

After the barrier, Legacy may only:

```text
DRAIN                finish already-existing eligible Legacy Instances
ONE_TIME_MIGRATE     move an exact reviewed Legacy Instance to one new Visit/activation
MANUALLY_TERMINATE   close an existing Legacy Instance under authorized governance
HISTORICAL_REPLAY    reconstruct/read historical facts without writes or scheduling
```

The post-barrier prohibition targets new traffic and new Legacy identity: Legacy Definition creation/cloning/publishing, Legacy Instance creation, routing or falling back new traffic to Legacy, silent fallback, and permanent dual-write/dual-authority are forbidden. A pre-barrier Legacy Instance in bounded DRAIN may append the Visit, Submission, Event, Receipt/audit, and other accepted Legacy facts necessary to complete its already-authorized flow. Those facts are append-only, preserve historical facts, remain bound to the existing Definition/Instance identity, and do not create a new Legacy Definition or Instance.

### 23.3 One-time migration

Migration is plan-bound, exact-scope, fail-closed, append-only, and separately production-authorized. The reviewed plan preassigns the exact target `nodeVisitId`; apply creates exactly one target Visit and the correct canonical activation atomically, preserves Legacy history read-only, records mapping/audit/Receipt, and makes exact rerun a no-write replay. Drift, ambiguity, missing owner type, Service owner, partial write, or inability to prove atomicity yields zero committed writes. Unknown outcome reconciles the exact same plan/Visit/idempotency identity.

### 23.4 Compatibility

Current Domain/global lists, worklists, creator-owned drafts, and historical views retain their accepted wire behavior for Legacy drain/history until separately superseded. They are not canonical new dispatch feeds. New APIs/contracts must be versioned or proven compatible by the later implementation Spec; V6 chooses no endpoint, table, transport, or deployment mechanism.

### 23.5 Containment and rollback

Before production cutover, rollback may revert candidate code/config with no traffic/data effect. After new-model facts exist, rollback is containment: pause new intake/delivery, preserve committed Visit/activation facts, revoke/disable compromised access, and repair/reconcile under accepted authority. It MUST NOT route new traffic to Legacy, delete/relabel activations, rewrite history, or fabricate Legacy records. Product Direction reversal requires a lawful whole successor.

### 23.6 Separate execution gates

V6 acceptance, Architecture acceptance, implementation-Spec acceptance, code merge, migration readiness, external Scheduler readiness, production cutover, and Legacy migration/apply are distinct gates. None implies the next. This Product Direction authorizes no production action, transition, canary, Grant, message, Session, wake command, Scheduler job, migration, or apply.

## 24. Open Questions and acceptance readiness

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE_FOR_PRODUCT_DIRECTION_REVIEW
IMPLEMENTATION_BLOCKED_PENDING_ARCHITECTURE_RECONCILIATION = YES
PARTIAL_SUPERSESSION = NONE
DUPLICATE_AUTHORITY_RISK = DISPOSITIONED
OPEN_LOCAL_PR_CENSUS_OBSERVED_AT = 2026-09-01
OPEN_LOCAL_PR_CENSUS_COUNT = 5
OPEN_LOCAL_PR_7 = BASE_9ba2d87e94f6d39ffdd6986b5a434546cb91d90c_HEAD_a7f8d26b7a8f57da773bd7b05879ee485841fa58_OPEN_DRAFT_UNMERGED_PROPOSED_CONTRACTS_PENDING_ACCEPTANCE_INDEPENDENT_REPLAY_CLOSURE_PRODUCTION_UNAUTHORIZED
OPEN_LOCAL_PR_9 = BASE_327b74f138151a7f4d9d88e3881e54d203f1e8f6_HEAD_3056263c3fc964a2b225720dd2b859b47e296c2e_OPEN_DRAFT_UNMERGED_PROPOSED_V3_BOUND_IMPLEMENTATION_NONE_PRODUCTION_NONE_SUPERSEDED_BY_FLEET_LOCAL_CHILD
OPEN_LOCAL_PR_13 = BASE_2ff81ae47ab068216bd0012fa0e76a45dd2fb572_HEAD_83fd493db26c5e9b5b00d7e308da3c372c4d9ca4_OPEN_DRAFT_UNMERGED_PROPOSED_INDEPENDENT_NON_AUTHORITATIVE
OPEN_LOCAL_PR_13_AUTHORITY = V4_PLUS_ARCHITECTURE_V0_3_1_IMPLEMENTATION_NONE_PRODUCTION_NONE
OPEN_LOCAL_PR_13_SCOPE = OWNER_SCOPED_READ_ONLY_DOMAIN_LIST_NO_CANONICAL_ACTIVATION_NO_NEXT_ELIGIBLE_AT
OPEN_LOCAL_PR_19 = BASE_c90d54cace46ff505ac54aa6215587d812cf9a78_HEAD_0c63d35a6e1291e7187e693e2a0ed1fec231eaf2_OPEN_DRAFT_UNMERGED_PROPOSED_NON_AUTHORITATIVE
OPEN_LOCAL_PR_20_AT_CENSUS = BASE_c90d54cace46ff505ac54aa6215587d812cf9a78_PRE_AMENDMENT_HEAD_818948189aa7f4eb326e16ca3e5725fceaf0394d_OPEN_NON_DRAFT_UNMERGED_SELF_PROPOSED_CANDIDATE
OPEN_LOCAL_PR_20_REVIEWED_SEMANTIC_HEAD = bc4a13a968073e1a81ba3fb168d4bf5c3cc12ba9
OPEN_LOCAL_PR_20_CURRENT = OWNER_ACCEPTED_UNMERGED_BRANCH_CANDIDATE_NON_ACTIVE_PENDING_FINAL_HEAD_RECHECK_AND_MERGE
OPEN_LOCAL_PR_20_FINAL_ACCEPTED_HEAD = REPORTED_IN_PR_RECORD_AFTER_COMMIT_NOT_SELF_EMBEDDED
OTHER_OPEN_LOCAL_PR_LIFECYCLE_ACTION_BY_V6 = NONE
OPEN_LOCAL_PR_LIFECYCLE_AUTHORITY = REPOSITORY_OWNER_MAYF3
CURRENT_DISPATCHABILITY_PROPOSAL_PR = 19
CURRENT_DISPATCHABILITY_PROPOSAL_BASE = c90d54cace46ff505ac54aa6215587d812cf9a78
CURRENT_DISPATCHABILITY_PROPOSAL_HEAD = 0c63d35a6e1291e7187e693e2a0ed1fec231eaf2
CURRENT_DISPATCHABILITY_PROPOSAL_STATE = OPEN_DRAFT_UNMERGED_PROPOSED_NON_AUTHORITATIVE
CURRENT_DISPATCHABILITY_PROPOSAL_GOVERNED_BY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
CURRENT_DISPATCHABILITY_PROPOSAL_IMPLEMENTATION_AUTHORITY = CONTRACTS_INERT_WHILE_PROPOSED
CURRENT_DISPATCHABILITY_PROPOSAL_SEMANTICS = READ_TIME_QUERY_PROJECTION_NON_PERSISTED_NO_CANONICAL_ACTIVATION_NO_NEXT_ELIGIBLE_AT
CURRENT_DISPATCHABILITY_SOURCE_CANDIDATE = af450aa39e446683b8ae2b2edf99c4febdcfb068
CURRENT_DISPATCHABILITY_DISPOSITION = REWRITE_REQUIRED_NOT_ACCEPTABLE_OR_IMPLEMENTABLE_FOR_THIS_GOAL
CURRENT_DISPATCHABILITY_PR_LIFECYCLE_ACTION_BY_V6 = NONE
CURRENT_DISPATCHABILITY_PR_LIFECYCLE_AUTHORITY = REPOSITORY_OWNER_MAYF3
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
ARCHITECTURE_SUCCESSOR_OR_REFINEMENT_REQUIRED = YES
CHILD_IMPLEMENTATION_SPEC_REQUIRED = YES
SEMANTIC_REVIEW_RESULT = ACCEPT
OWNER_ACCEPTANCE = COMPLETE
READY_FOR_FINAL_HEAD_RECHECK = YES
NEW_NODE_KINDS = TASK | TERMINAL
TASK_OWNER_TYPES = HUMAN | AGENT
SERVICE_TASK_OWNER = FORBIDDEN_FAIL_CLOSED
CANONICAL_WORK_IDENTITY = nodeVisitId
CANONICAL_ACTIVATION = HUMAN_WORK_ITEM | DISPATCH_INTENT
SCHEDULER_WAIT_PRIMITIVE = nextEligibleAt
INITIAL_NEXT_ELIGIBLE_AT = CANONICAL_SERVER_AUTHORED_ACTIVATION_TIMESTAMP
ACTIVATION_TIMESTAMP_PERSISTENCE = SAME_ACTIVATION_TRANSACTION
CLIENT_AUTHORED_INITIAL_NEXT_ELIGIBLE_AT = FORBIDDEN
POST_COMMIT_INITIAL_NEXT_ELIGIBLE_AT_WRITE = FORBIDDEN
PHYSICAL_COMMIT_INSTANT_EQUALITY_REQUIRED = NO
NORMAL_PATH = ACTIVATION_DRIVEN
PERIODIC_SCAN = RECONCILER | WATCHDOG | REPAIR ONLY
NEW_TRAFFIC_LEGACY_FALLBACK = FORBIDDEN
LEGACY_CUTOVER_PROHIBITION_SCOPE = POST_BARRIER_NEW_TRAFFIC_AND_NEW_LEGACY_IDENTITY
PRE_BARRIER_LEGACY_DRAIN_APPEND_FACTS = VISIT | SUBMISSION | EVENT | RECEIPT_AUDIT | OTHER_NECESSARY_ACCEPTED_LEGACY_FACTS
PRE_BARRIER_LEGACY_DRAIN_CREATES_NEW_DEFINITION_OR_INSTANCE_IDENTITY = NO
CURRENT_GLOBAL_READ_GATE = COMPATIBILITY_ONLY / GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR
ROLE_OR_GRANT_CHANGE_AUTHORIZED = NO
EXTERNAL_PERIODIC_RECOVERY_INTEROPERABILITY = RECONCILER_ONLY
EXTERNAL_SCHEDULER_MANAGEMENT_INTEROPERABILITY = SEPARATE_FROM_NORMAL_DISPATCH
EXTERNAL_AUTHORITY_PATH = EXTERNAL_REPOSITORY_CHOICE
PR_87_RELATION = FIXED_COORDINATE_NON_AUTHORITATIVE_OBSERVATION_PROVENANCE_ONLY
PR_87_LIFECYCLE_COMMAND_BY_SVC_WORKFLOW = NONE

TRUSTED_AGENT_ROOT_REQUIRED = YES
ROOT_AUTHORITY_ID = SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1
ADMIN_AGENT_STRATEGY = NEW_DEDICATED_AGENT
HUMAN_PRINCIPAL_ADMINISTRATION_REQUIRED_FOR_V1 = NO
HUMAN_OBO_REQUIRED_FOR_V1 = NO
TWO_PERSON_APPROVAL_REQUIRED_FOR_V1 = NO
```

Exact table names, endpoint names, bounded delivery mechanism, permission key, timestamp precision, retry schedule, and production barrier coordinates belong to later Architecture/implementation/external authorities. They may not change the closed node/owner model, Visit identity, activation cardinality, singular wait primitive, wake semantics, ownership boundaries, or one-way cutover. The exact Admin Agent Principal UUID and Client ID remain owned by the later designation authority.

## 25. Lifecycle record

```text
ACCEPTANCE_STATUS = accepted
STATUS = accepted
AUTHORING_BASE_REF = github/main
AUTHORING_BASE = c90d54cace46ff505ac54aa6215587d812cf9a78
CURRENT_PARENT = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
CURRENT_PARENT_STATUS_ON_MAIN = accepted
CURRENT_PARENT_ACCEPTED_HEAD = b3c6d797d3a79655a8fd5b1c63016600d4631036
CURRENT_PARENT_MERGE_COMMIT = c90d54cace46ff505ac54aa6215587d812cf9a78
V5_TRANSITION = superseded candidate (frontmatter lifecycle only)
V5_FRONTMATTER_CHANGE = status_superseded + superseded_by_SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
V5_HISTORICAL_BODY = byte-identical from first H1 to EOF
LOCAL_AUTHORITY_MAP_CHANGE = switched_to_accepted_SVC_WORKFLOW_PRODUCT_BOUNDARY_V6_candidate
OPEN_LOCAL_PR_CENSUS_OBSERVED_AT = 2026-09-01
OPEN_LOCAL_PR_CENSUS_COUNT = 5
OPEN_LOCAL_PR_7 = BASE_9ba2d87e94f6d39ffdd6986b5a434546cb91d90c_HEAD_a7f8d26b7a8f57da773bd7b05879ee485841fa58_OPEN_DRAFT_UNMERGED_PROPOSED_CONTRACTS_PENDING_ACCEPTANCE_INDEPENDENT_REPLAY_CLOSURE_PRODUCTION_UNAUTHORIZED
OPEN_LOCAL_PR_9 = BASE_327b74f138151a7f4d9d88e3881e54d203f1e8f6_HEAD_3056263c3fc964a2b225720dd2b859b47e296c2e_OPEN_DRAFT_UNMERGED_PROPOSED_V3_BOUND_IMPLEMENTATION_NONE_PRODUCTION_NONE_SUPERSEDED_BY_FLEET_LOCAL_CHILD
OPEN_LOCAL_PR_13 = BASE_2ff81ae47ab068216bd0012fa0e76a45dd2fb572_HEAD_83fd493db26c5e9b5b00d7e308da3c372c4d9ca4_OPEN_DRAFT_UNMERGED_PROPOSED_INDEPENDENT_NON_AUTHORITATIVE
OPEN_LOCAL_PR_13_AUTHORITY = V4_PLUS_ARCHITECTURE_V0_3_1_IMPLEMENTATION_NONE_PRODUCTION_NONE
OPEN_LOCAL_PR_13_SCOPE = OWNER_SCOPED_READ_ONLY_DOMAIN_LIST_NO_CANONICAL_ACTIVATION_NO_NEXT_ELIGIBLE_AT
OPEN_LOCAL_PR_19 = BASE_c90d54cace46ff505ac54aa6215587d812cf9a78_HEAD_0c63d35a6e1291e7187e693e2a0ed1fec231eaf2_OPEN_DRAFT_UNMERGED_PROPOSED_NON_AUTHORITATIVE
OPEN_LOCAL_PR_20_AT_CENSUS = BASE_c90d54cace46ff505ac54aa6215587d812cf9a78_PRE_AMENDMENT_HEAD_818948189aa7f4eb326e16ca3e5725fceaf0394d_OPEN_NON_DRAFT_UNMERGED_SELF_PROPOSED_CANDIDATE
OPEN_LOCAL_PR_20_REVIEWED_SEMANTIC_HEAD = bc4a13a968073e1a81ba3fb168d4bf5c3cc12ba9
OPEN_LOCAL_PR_20_CURRENT = OWNER_ACCEPTED_UNMERGED_BRANCH_CANDIDATE_NON_ACTIVE_PENDING_FINAL_HEAD_RECHECK_AND_MERGE
OPEN_LOCAL_PR_20_FINAL_ACCEPTED_HEAD = REPORTED_IN_PR_RECORD_AFTER_COMMIT_NOT_SELF_EMBEDDED
OTHER_OPEN_LOCAL_PR_LIFECYCLE_ACTION_BY_V6 = NONE
OPEN_LOCAL_PR_LIFECYCLE_AUTHORITY = REPOSITORY_OWNER_MAYF3
CURRENT_DISPATCHABILITY_PROPOSAL_PR = 19
CURRENT_DISPATCHABILITY_PROPOSAL_BASE = c90d54cace46ff505ac54aa6215587d812cf9a78
CURRENT_DISPATCHABILITY_PROPOSAL_HEAD = 0c63d35a6e1291e7187e693e2a0ed1fec231eaf2
CURRENT_DISPATCHABILITY_PROPOSAL_STATE = OPEN_DRAFT_UNMERGED_PROPOSED_NON_AUTHORITATIVE
CURRENT_DISPATCHABILITY_PROPOSAL_GOVERNED_BY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
CURRENT_DISPATCHABILITY_PROPOSAL_IMPLEMENTATION_AUTHORITY = CONTRACTS_INERT_WHILE_PROPOSED
CURRENT_DISPATCHABILITY_PROPOSAL_SEMANTICS = READ_TIME_QUERY_PROJECTION_NON_PERSISTED_NO_CANONICAL_ACTIVATION_NO_NEXT_ELIGIBLE_AT
CURRENT_DISPATCHABILITY_SOURCE_CANDIDATE = af450aa39e446683b8ae2b2edf99c4febdcfb068
CURRENT_DISPATCHABILITY_DISPOSITION = REWRITE_REQUIRED_NOT_ACCEPTABLE_OR_IMPLEMENTABLE_FOR_THIS_GOAL
CURRENT_DISPATCHABILITY_PR_LIFECYCLE_ACTION_BY_V6 = NONE
CURRENT_DISPATCHABILITY_PR_LIFECYCLE_AUTHORITY = REPOSITORY_OWNER_MAYF3
DSH_AGENT_CORE_PR_87_HEAD_OBSERVED = 4260911960f33c5b91c38403f002207f717f4187
DSH_AGENT_CORE_PR_87_RELATION = FIXED_COORDINATE_NON_AUTHORITATIVE_OBSERVATION_PROVENANCE_ONLY
EXTERNAL_PERIODIC_RECOVERY_INTEROPERABILITY = RECONCILER_ONLY
EXTERNAL_SCHEDULER_MANAGEMENT_INTEROPERABILITY = SEPARATE_FROM_NORMAL_DISPATCH
EXTERNAL_AUTHORITY_PATH = EXTERNAL_REPOSITORY_CHOICE
DSH_AGENT_CORE_CHANGE = NONE
PLAN_SHA256_BOUND = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606
EXACT_FLEET_PAIR_COUNT = 86
WORKFLOW_PROJECTION_CREATE_COUNT = 85
DOMAIN_TRANSFER_COUNT = 760
ACTIVE_RESPONSIBILITY_TRANSFER_COUNT = 80
GENERAL_MIGRATION_CAPABILITY = NO
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
PRODUCTION_APPLY_AUTHORIZED = NO
INDEPENDENT_SEMANTIC_REVIEW = ACCEPT_AT_bc4a13a968073e1a81ba3fb168d4bf5c3cc12ba9
OWNER_ACCEPTANCE = COMPLETE_BY_mayf3
FINAL_HEAD_RECHECK_REQUIRED = YES
MERGE_PERFORMED = NO
PRODUCT_CODE_CHANGE = NONE
PRODUCTION_CHANGE = NONE
```

The Owner acceptance record in §27 binds the reviewed semantic Head and the lifecycle-only scope. This atomic branch transaction changes only V6 lifecycle wording/metadata, V5 lifecycle frontmatter/backlink, and the repository-local authority map. The final accepted candidate Head is reported in the PR record after commit and must receive an independent final-head recheck before any merge.

## 26. AUTHOR lifecycle output

```text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = NOT_APPLICABLE (Product Direction authority)
SPEC_KIND = NOT_APPLICABLE (product_direction)
AUTHORITY_ID = SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
AUTHORITY_KIND = product_direction
STATUS = accepted
AUTHORITY_LEVEL = highest local Product Direction
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
EXTERNAL_AUTHORITIES = NONE (PR #87 at `4260911...` is fixed-coordinate non-authoritative observation/provenance only)
OPEN_LOCAL_PR_CENSUS = PR_7_a7f8d26_OPEN_DRAFT | PR_9_3056263_OPEN_DRAFT | PR_13_83fd493_OPEN_DRAFT | PR_19_0c63d35_OPEN_DRAFT | PR_20_AT_CENSUS_SELF_PROPOSED_PRE_AMENDMENT_8189481
OPEN_LOCAL_PR_13_DISPOSITION = INDEPENDENT_NON_AUTHORITATIVE_OWNER_SCOPED_READ_ONLY_NO_IMPLEMENTATION_OR_PRODUCTION_AUTHORITY_NO_CANONICAL_ACTIVATION_OR_NEXT_ELIGIBLE_AT
OPEN_LOCAL_PR_OTHER_LIFECYCLE_ACTION_BY_V6 = NONE_REPOSITORY_OWNER_RETAINS_AUTHORITY
OPEN_LOCAL_PR_20_REVIEWED_SEMANTIC_HEAD = bc4a13a968073e1a81ba3fb168d4bf5c3cc12ba9
OPEN_LOCAL_PR_20_CURRENT = OWNER_ACCEPTED_UNMERGED_BRANCH_CANDIDATE_NON_ACTIVE_PENDING_FINAL_HEAD_RECHECK_AND_MERGE
OPEN_LOCAL_PR_20_FINAL_ACCEPTED_HEAD = REPORTED_IN_PR_RECORD_AFTER_COMMIT_NOT_SELF_EMBEDDED
CURRENT_DISPATCHABILITY_PROPOSAL = PR_19_AT_0c63d35a6e1291e7187e693e2a0ed1fec231eaf2_OPEN_DRAFT_UNMERGED_PROPOSED_V5_BOUND_NON_AUTHORITATIVE
CURRENT_DISPATCHABILITY_SOURCE_CANDIDATE = af450aa39e446683b8ae2b2edf99c4febdcfb068_LINEAGE_ONLY
CURRENT_DISPATCHABILITY_DISPOSITION = NOT_ACCEPTABLE_OR_IMPLEMENTABLE_AS_THIS_GOAL_CHILD_OR_CANONICAL_DISPATCH_AUTHORITY_REWRITE_IF_RETAINED
CURRENT_DISPATCHABILITY_PR_LIFECYCLE_ACTION_BY_V6 = NONE_REPOSITORY_OWNER_RETAINS_AUTHORITY
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
PARTIAL_SUPERSESSION = NONE
CONTRACT_COUNT = 58
CONTRACTS_WITH_ACCEPTANCE = 58
SEMANTIC_REVIEW_RESULT = ACCEPT
OWNER_ACCEPTANCE = COMPLETE
AUTHORING_READY_FOR_FINAL_HEAD_RECHECK = YES
INITIAL_NEXT_ELIGIBLE_AT = CANONICAL_SERVER_AUTHORED_ACTIVATION_TIMESTAMP_IN_SAME_TRANSACTION
LEGACY_CUTOVER_SCOPE = NEW_TRAFFIC_ONLY_WITH_BOUNDED_PRE_BARRIER_DRAIN_FACTS_ALLOWED
EXTERNAL_INTEROPERABILITY = RECONCILER_ONLY_PERIODIC_RECOVERY_AND_SEPARATE_SCHEDULER_MANAGEMENT
```

This is the lifecycle-only AUTHOR summary. Independent semantic review and Owner acceptance are complete. Independent final-head recheck and any later documentation merge remain separate gates; this transaction does not perform or directly authorize merge, implementation, migration, deployment, formal dispatch cutover, or production apply.

## 27. Acceptance Record (OWNER_WHOLE_AUTHORITY_ACCEPTANCE_TRANSITION_ONLY)

```text
ACCEPTANCE_STATUS = accepted
REVIEW_BASE = c90d54cace46ff505ac54aa6215587d812cf9a78
REVIEWED_SEMANTIC_HEAD = bc4a13a968073e1a81ba3fb168d4bf5c3cc12ba9
REVIEW_RESULT = ACCEPT
REVIEW_BLOCKERS = 0
REVIEW_RECORD = https://github.com/mayf3/svc-workflow/pull/20#issuecomment-5487276757
REVIEWER_ID = Codex independent subagent /root/fresh_boundary_audit_v6_bc4a13a
READY_TO_MARK_ACCEPTED = YES
SEMANTIC_DELTA_AFTER_REVIEW = NONE
ACCEPTED_BY = mayf3
ACCEPTED_AT = 2026-09-01T11:48:22Z
OWNER_ACCEPTANCE_SCOPE = V6_PRODUCT_DIRECTION_LIFECYCLE_ACCEPTANCE_TRANSACTION_ONLY
V5_TRANSITION = superseded candidate (frontmatter lifecycle only)
V5_HISTORICAL_BODY = byte-identical from first H1 to EOF
LOCAL_AUTHORITY_MAP = switched atomically to accepted V6 candidate
ALLOWED_DELTA = V6 lifecycle metadata and acceptance receipt + V5 frontmatter supersession/backlink + repository-local authority map
FINAL_ACCEPTED_HEAD = REPORTED_IN_PR_RECORD_AFTER_COMMIT_NOT_SELF_EMBEDDED
FINAL_HEAD_RECHECK = NOT_PERFORMED / REQUIRED_BEFORE_MERGE
ACTIVE_ON_MAIN = NO at acceptance commit (accepted candidate on PR #20 branch; repository-active only after independent final-head recheck and merge)
DIRECT_MERGE_AUTHORIZED_BY_THIS_TRANSACTION = NO
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
DATABASE_MIGRATION_AUTHORIZED = NO
PRODUCTION_DEPLOYMENT_AUTHORIZED = NO
FORMAL_DISPATCH_CUTOVER_AUTHORIZED = NO
PRODUCTION_APPLY_AUTHORIZED = NO
```

This acceptance is an Owner whole-authority lifecycle transition only. V6 becomes an accepted branch candidate, V5's frontmatter becomes superseded with its `superseded_by` backlink, and the repository-local authority map is switched atomically. The reviewed V6 product meaning and stable IDs are untouched. The lifecycle-only final accepted candidate Head must be persisted in the PR record and independently rechecked before any documentation merge.
