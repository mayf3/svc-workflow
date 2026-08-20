---
spec_id: SVC_WORKFLOW_CROSS_DOMAIN_ISOLATION_ENFORCEMENT_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
scope:
  - mayf3/svc-workflow
  - cross-domain-isolation
  - global-workflow-coordinator-removal
  - global-runtime-surfaces
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V1
  - SVC_WORKFLOW_ARCHITECTURE_V0_3_1
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_CROSS_DOMAIN_ISOLATION_ENFORCEMENT_V1

## 1. Goal

Restore and enforce the existing Product Boundary rule that workflow facts are isolated by Domain. Remove or hard-disable every ordinary product-runtime capability that derives cross-Domain visibility or write authority from `GLOBAL_WORKFLOW_COORDINATOR`.

```text
OWNER_DECISION = OPTION_A
CROSS_DOMAIN_VISIBILITY = FORBIDDEN
GLOBAL_WORKFLOW_COORDINATOR = NOT_AUTHORIZED_AS_PRODUCT_ROLE
PRODUCT_BOUNDARY_ACTION = REUSE
PRODUCT_BOUNDARY_CHANGE = NONE
ORDINARY_RUNTIME_GLOBAL_VISIBILITY = REMOVE
COORDINATOR_CREATE_DOMAIN = REMOVE
COORDINATOR_REPLACE_DOMAIN_OWNER = REMOVE
COORDINATOR_ASSISTANCE_VISIBILITY = REMOVE
COORDINATOR_GLOBAL_INSTANCE_VISIBILITY = REMOVE
```

This Spec is a bounded implementation Spec under the existing Product Boundary and frozen v0.3.1 Architecture. It does not establish a new Product Direction, a replacement coordinator, or a narrower coordinator role.

Success means an enabled Principal can observe or operate on workflow facts only through an explicit role or participation relationship in the fact's own Domain, except for the already-authorized control-plane provisioning surface defined below. No global role, role combination, old binding, old route, old SDK, rollback, replay, or deployment race may restore cross-Domain product visibility.

## 2. Scope and non-goals

### 2.1 In scope

- removal or hard-disablement of `GLOBAL_WORKFLOW_COORDINATOR` as a product permission;
- removal or hard-disablement of global Workflow Instance listing;
- removal or hard-disablement of Coordinator `HUMAN_REQUIRED` Assistance list and detail visibility;
- removal or hard-disablement of Coordinator Domain creation and Domain Owner replacement;
- retirement of grant/revoke surfaces for the global role;
- treatment of existing `global_role_bindings` rows and Migration `0020`;
- authorization, transaction, in-flight request, replay, enumeration, audit, rollout, compatibility, and rollback contracts;
- corresponding Rust, SQL, forward migration, OpenAPI, HTTP contract bundle, SDK, tests, configuration, metrics, logs, audit, and documentation changes in a later implementation task.

### 2.2 Out of scope

- changing `PRODUCT-BOUNDARY.md`;
- changing or superseding `SVC_WORKFLOW_ARCHITECTURE_V0_3_1`;
- changing or superseding the v0.3.2 Cancel / Archive authority;
- changing legitimate Domain-local workflow, Definition, Assistance, Cancel, or Archive authority;
- introducing a bounded, renamed, delegated, emergency, read-only, migration, or otherwise repackaged Coordinator product role;
- modifying product code, migrations, contracts, SDK, tests, deployment configuration, production data, existing bindings, or the paused Assistance draft in this authoring change;
- evaluating implementation conformance in this authoring change;
- authorizing direct manual production-database mutation outside the implementation and rollout Contracts.

### 2.3 Authoring-change boundary

This proposal changes only this governing Spec. It does not execute any revocation, disable any binding, remove any route, or alter any production state. Product implementation is prohibited until an authorized actor accepts an exact reviewed Spec revision and that accepted revision is present in the implementation base.

## 3. Authority and dependencies

### 3.1 Primary authority chain

1. `SVC_WORKFLOW_PRODUCT_BOUNDARY_V1` — `PRODUCT-BOUNDARY.md`, effective Product Direction. It defines Domain isolation as multi-Domain support with cross-Domain invisibility and identifies Admin/Provisioning as an in-scope platform capability.
2. `SVC_WORKFLOW_ARCHITECTURE_V0_3_1` — `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md`, `ARCHITECTURE_FROZEN`. It defines Domain as permission and audit boundary; Domain Owner visibility is Domain-local; current assignee and historical participation are explicit instance-local visibility sources.
3. This Spec, if accepted — bounded implementation Contracts that enforce those existing authorities.

`SVC_WORKFLOW_CANCEL_ARCHIVE_GOVERNANCE_V0_3_2` at `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_2.md` remains the related effective authority for Cancel / Archive only. This Spec neither changes its Domain Owner authority nor supersedes any part of it.

The accepted `SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1` governs this authoring and later compliance process. It does not supply product authority.

### 3.2 Authority classification

```text
PREFLIGHT_MODE = NEW
CHANGE_CLASS = NON_MECHANICAL
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V1
RELATED_PARENT_AUTHORITY = SVC_WORKFLOW_ARCHITECTURE_V0_3_1
RELATED_CANCEL_ARCHIVE_AUTHORITY = SVC_WORKFLOW_CANCEL_ARCHIVE_GOVERNANCE_V0_3_2
PRODUCT_BOUNDARY_ACTION = REUSE
PRODUCT_BOUNDARY_CHANGE = NONE
PARTIAL_SUPERSESSION = NONE
IMPLEMENTATION_AUTHORITY = contracts
```

The current code, tests, migrations, OpenAPI, HTTP contract bundle, SDK, configuration, and runtime-shaped documents are descriptive current state. They do not authorize `GLOBAL_WORKFLOW_COORDINATOR` and do not override the Product Boundary.

### 3.3 Source coordinates

```text
REPOSITORY = mayf3/svc-workflow
BASE_COMMIT = c7830e58578d7c7360710f2449c48cb801da773e
SOURCE_ENVIRONMENT = clean branch worktree at the exact base before authoring; subsequent docs-only Spec change excluded from source observations
SOURCE_OBSERVED_AT = 2026-08-20T15:02:28Z
RUNTIME_ENVIRONMENT_OBSERVED = NONE
PRODUCTION_DATABASE_OBSERVED = NO
```

The paused file `/Users/yanfenma/workspace/project/svc-workflow-assistance-spec/docs/specs/SVC_WORKFLOW_ASSISTANCE_V1.md` is not an authority, is not Evidence for this Spec, and is not modified or copied.

### 3.4 Decision provenance and acceptance boundary

The frozen Option A fields are persisted verbatim in §1 and will be durably bound to this exact Spec commit and its Draft PR. They are authoring inputs, not repository Evidence and not active authority before acceptance. The additional migration, failure, compatibility, rollback, audit, and security selections below are proposed Spec-level Decisions required to make Option A implementable without implementation-agent discretion; they are not claimed to be pre-existing Product Direction. Repository owner acceptance of the exact reviewed revision is the durable decision act. If the owner does not accept any one of these selections, this proposal MUST be revised and reviewed again; implementation remains blocked.

`OPEN_OWNER_DECISIONS = NONE` means this proposal contains no semantic choice delegated to the implementation Agent. It does not mean the proposed Decisions are active before authorized acceptance.

## 4. Current State

### STATE-XDOMAIN-001 — Accepted Product Direction forbids cross-Domain visibility

- Subject: Product Direction and core Architecture.
- As of commit: `c7830e58578d7c7360710f2449c48cb801da773e`.
- Environment: repository source tree.
- Projection: the effective Product Boundary says Domain isolation means cross-Domain invisibility; v0.3.1 grants Domain-wide workflow visibility to that Domain's Owner and grants instance visibility through current or historical participation, not through a deployment-global product role.
- Basis: `OBS-XDOMAIN-001`, `CLM-XDOMAIN-001`, `EVD-XDOMAIN-001`.

### STATE-XDOMAIN-002 — Current source implements a global role and global surfaces

- Subject: source implementation and published surfaces.
- As of commit: `c7830e58578d7c7360710f2449c48cb801da773e`.
- Environment: repository source tree; runtime not executed.
- Projection: the source defines `GLOBAL_WORKFLOW_COORDINATOR`, persists bindings, provides grant/revoke, cross-Domain instance and Assistance reads, Domain creation, and Domain Owner replacement, and encodes portions of those surfaces in OpenAPI, the HTTP bundle, SDK, and tests.
- Basis: `OBS-XDOMAIN-002` through `OBS-XDOMAIN-008`, `CLM-XDOMAIN-002`, `EVD-XDOMAIN-002` through `EVD-XDOMAIN-005`.

### STATE-XDOMAIN-003 — Existing redaction does not restore Domain isolation

- Subject: global list and Coordinator Assistance projections.
- As of commit: `c7830e58578d7c7360710f2449c48cb801da773e`.
- Environment: repository source tree.
- Projection: the global instance projection exposes IDs, Domain IDs, creator and assignee identity, node facts, Definition key, Context `title`, and timestamps. The Coordinator Assistance projection exposes case, Domain, instance, requester identity, request payload, and escalation payload. Omitting other fields does not make either projection Domain-local.
- Basis: `OBS-XDOMAIN-003`, `OBS-XDOMAIN-004`, `CLM-XDOMAIN-003`, `EVD-XDOMAIN-003`.

### STATE-XDOMAIN-004 — Revocation and authorization are not one fail-closed response boundary

- Subject: role-check, data-query, revoke, and response paths.
- As of commit: `c7830e58578d7c7360710f2449c48cb801da773e`.
- Environment: repository source tree.
- Projection: the global list checks the role in one transaction and queries data afterward; Assistance checks the role separately from its query; role revocation soft-disables a row. The source does not establish a single distributed in-flight barrier that prevents a previously authorized request from publishing data after containment begins.
- Basis: `OBS-XDOMAIN-006`, `CLM-XDOMAIN-004`, `EVD-XDOMAIN-004`.

### STATE-XDOMAIN-005 — Implementation conformance has not been evaluated

- Subject: exact implementation at the base against this proposed Spec.
- As of commit: `c7830e58578d7c7360710f2449c48cb801da773e`.
- Environment: authoring worktree only.
- Projection: current source visibly conflicts with the parent authority and proposed removal direction, but Contract-by-Contract COMPLIANCE against an accepted Spec revision and an exact implementation commit has not occurred.
- Basis: `OBS-XDOMAIN-009`, `CLM-XDOMAIN-005`, `EVD-XDOMAIN-006`.

```text
CURRENT_IMPLEMENTATION_EXISTS = YES
CURRENT_IMPLEMENTATION_CONFLICTS_WITH_AUTHORITY = YES
RETROACTIVE_IMPLEMENTATION_AUTHORIZATION = NO
CONFORMANCE_NOT_YET_EVALUATED = YES
```

## 5. Observations

### OBS-XDOMAIN-001 — Parent authorities define Domain-local visibility

- Subject: Product Boundary and Architecture authorization model.
- Repository/source: `mayf3/svc-workflow`.
- Commit/artifact: `c7830e58578d7c7360710f2449c48cb801da773e`.
- Environment: source tree.
- Observed at: `2026-08-20T15:02:28Z`.
- Method: read `PRODUCT-BOUNDARY.md`, v0.3.1, v0.3.2, and the local authority map.
- Result: Product Boundary line 13 states cross-Domain invisibility; v0.3.1 lines 295-304 define Domain as permission and audit boundary, lines 324-379 define Domain Owner through Domain-local binding, and lines 383-434 limit visibility to Domain Owner, current assignee, and historical participant relationships. v0.3.2 lines 108-128 grant Cancel / Archive only to the current Domain Owner and deny cross-Domain Principals.
- Provenance: the named files at the base commit.

### OBS-XDOMAIN-002 — Migration 0020 and provisioning create a global role lifecycle

- Subject: global role persistence and provisioning.
- Repository/source: `migrations/0020_global_role_bindings.sql`; provisioning domain/application/store/HTTP files.
- Commit/artifact: base commit.
- Environment: source tree.
- Observed at: `2026-08-20T15:02:28Z`.
- Method: read Migration `0020`, `src/domain/provisioning/mod.rs`, `src/application/provisioning/mod.rs`, `src/store/postgres/provisioning_repository/mod.rs`, `src/http/handlers/provisioning/global_role_bindings.rs`, and route registration.
- Result: Migration `0020` creates `global_role_bindings`; code defines `GLOBAL_WORKFLOW_COORDINATOR`, permits admin allowlisted PUT grant/upsert and DELETE soft revoke, and re-enables an existing row by setting `enabled=TRUE` and clearing `disabled_at`.
- Provenance: migration lines 1-28; repository lines 400-457; application lines 414-578; handlers lines 19-99; `src/http/mod.rs` lines 240-248.

### OBS-XDOMAIN-003 — Global instance list returns a deployment-global projection

- Subject: global Workflow Instance listing.
- Repository/source: HTTP handler, query service/repository, OpenAPI, and tests.
- Commit/artifact: base commit.
- Environment: source tree.
- Observed at: `2026-08-20T15:02:28Z`.
- Method: read the route, handler, query authorization, SQL projection, OpenAPI operation, and integration tests.
- Result: `GET /internal/v1/workflow-instances/global` authorizes solely through the global binding and queries without a Domain predicate. It returns instance and Domain IDs, Definition key, creator, current assignee, current node, Context `title`, and timestamps across all Domains. Tests explicitly require multi-Domain visibility and title projection.
- Provenance: `src/http/mod.rs` lines 115-121; `instances.rs` lines 158-215; `query_service.rs` lines 101-129; `query_global_instances.rs` lines 1-178; OpenAPI lines 485-548; `tests/23_global_coordinator.rs` lines 270-354 and 414-454.

### OBS-XDOMAIN-004 — Coordinator Assistance surfaces expose cross-Domain case content

- Subject: `HUMAN_REQUIRED` Assistance list and detail.
- Repository/source: Assistance HTTP/store/domain projections, OpenAPI, HTTP contract, SDK, and tests.
- Commit/artifact: base commit.
- Environment: source tree.
- Observed at: `2026-08-20T15:02:28Z`.
- Method: read Coordinator Assistance projection and queries plus their published clients/tests.
- Result: the global role can list every `HUMAN_REQUIRED` case and can use the ordinary case-detail route as a fallback. The projection includes case, Domain, instance, Definition and node identifiers, `requestedByPrincipalId`, full request payload, and escalation payload. The SDK exposes both the list method and a detail union containing this projection.
- Provenance: `assistance_transaction/query.rs` lines 102-173 and 270-373; `handlers/assistance.rs` lines 226-275; OpenAPI lines 280-343 and schema lines 2172 onward; HTTP contract lines 16-28; SDK client lines 335-368 and schemas lines 635-661; Assistance tests at `tests/25_workflow_assistance/core.rs` lines 142-178 and `tests/26_workflow_assistance_http.rs` lines 160-229.

### OBS-XDOMAIN-005 — Coordinator can create Domains and replace arbitrary Domain Owners

- Subject: ordinary agent-facing Domain management.
- Repository/source: coordinator handlers, shared provisioning service, repository, routes, and tests.
- Commit/artifact: base commit.
- Environment: source tree.
- Observed at: `2026-08-20T15:02:28Z`.
- Method: read both handler paths and their shared commands.
- Result: a Principal with `workflow.execute`, a direct token, and the global binding can call `POST /internal/v1/domains` and `PUT /internal/v1/domains/{domainId}/owner`. Owner replacement disables the prior owner and enables any enabled target Principal. Tests require both Coordinator calls to succeed and replay.
- Provenance: `src/http/mod.rs` lines 153-167; `coordinator_domains.rs` lines 1-155; provisioning application lines 580-651; repository lines 459-527; `tests/24_coordinator_domain_management.rs` lines 184-267.

### OBS-XDOMAIN-006 — Principal checks, revocation, and auditing are uneven across surfaces

- Subject: disabled Principal, role revoke, denied reads, and provisioning audit.
- Repository/source: query visibility, Assistance query, provisioning repository/application.
- Commit/artifact: base commit.
- Environment: source tree.
- Observed at: `2026-08-20T15:02:28Z`.
- Method: compare actor checks, transactions, and audit writes.
- Result: instance detail checks `principals.enabled` and writes `workflow_security_audits` for certain denied reads; Assistance global queries check enabled Principal but do not write a denied-request audit; global list role checking does not use the common actor snapshot and does not write a denied audit; provisioning emits structured logs rather than a durable SecurityAudit. Revocation soft-disables a binding, while global-list authorization and data query occur in separate transactions.
- Provenance: `query_visibility.rs` lines 84-151 and 392-441; Assistance query lines 181-190 and 270-373; provisioning application lines 20-35 and 414-651; repository lines 435-457.

### OBS-XDOMAIN-007 — Published surface inventory is inconsistent but still exposes global behavior

- Subject: OpenAPI, HTTP contract bundle, SDK, errors, and docs.
- Repository/source: `contracts/workflow-http/v1`, `sdk/typescript`, HTTP errors.
- Commit/artifact: base commit.
- Environment: source tree.
- Observed at: `2026-08-20T15:02:28Z`.
- Method: search all repository files for the role and retired paths, then read matched artifacts.
- Result: OpenAPI publishes global list and Coordinator Assistance; the HTTP contract publishes Coordinator Assistance; SDK publishes Coordinator Assistance but not global list; runtime routes additionally expose Coordinator Domain writes and global-role provisioning even where the runtime contract excludes control-plane paths. Error catalogs include `global_coordinator_required`.
- Provenance: OpenAPI lines 280-343 and 485-548; HTTP contract lines 16-36; SDK client/schema matches; `contracts/workflow-http/v1/errors.json`; route inventory.

### OBS-XDOMAIN-008 — Tests currently assert the unauthorized product direction

- Subject: executable test definitions.
- Repository/source: Coordinator and Assistance integration tests and SDK tests.
- Commit/artifact: base commit.
- Environment: source tree; tests not executed for this authoring task.
- Observed at: `2026-08-20T15:02:28Z`.
- Method: read relevant test definitions without treating them as executed Evidence.
- Result: tests assert multi-Domain instance visibility, role grant/revoke, Coordinator Domain creation, arbitrary Owner replacement, Coordinator Assistance list/detail, and SDK parsing of the global Assistance projection.
- Provenance: `tests/23_global_coordinator.rs`, `tests/24_coordinator_domain_management.rs`, `tests/25_workflow_assistance`, `tests/26_workflow_assistance_http.rs`, and `sdk/typescript/tests/client.test.ts`.

### OBS-XDOMAIN-009 — Governance is accepted and implementation authority is absent before this Spec

- Subject: repository development governance and existing governing Specs.
- Repository/source: `.agents/governance.lock.json`, governance verifier, and `docs/specs`.
- Commit/artifact: base commit.
- Environment: clean authoring worktree.
- Observed at: `2026-08-20T15:02:28Z`.
- Method: run `python3 .agents/tools/verify_governance.py --target . --require-accepted`; read the adoption Spec and Spec index.
- Result: vendored governance bytes match an accepted lock. The only existing governing Spec is process-only with `implementation_authority: none`; no accepted product Spec retroactively authorizes the global role.
- Provenance: verifier output and named files at the base.

### OBS-XDOMAIN-010 — Existing legitimate Owner management is a separate control-plane surface

- Subject: retained Domain Owner replacement path.
- Repository/source: admin routes, provisioning auth, Product Boundary, v0.3.1.
- Commit/artifact: base commit.
- Environment: source tree.
- Observed at: `2026-08-20T15:02:28Z`.
- Method: read admin route registration and provisioning authentication.
- Result: `/internal/v1/admin/domains/{domainId}/owner` is separate from the Coordinator runtime route and is guarded by `workflow.admin` plus `WORKFLOW_PROVISIONING_PRINCIPAL_IDS`. Product Boundary includes Admin/Provisioning, and v0.3.1 requires atomic Owner replacement and SecurityAudit but does not authorize Coordinator self-service.
- Provenance: `src/http/mod.rs` lines 211-248; `src/http/handlers/provisioning/mod.rs`; `src/application/provisioning/config.rs`; Product Boundary line 15; v0.3.1 lines 351-379.

## 6. Claims and assumptions

### CLM-XDOMAIN-001 — Option A reuses rather than changes Product Direction

- Support state: SUPPORTED
- Supported by evidence: `EVD-XDOMAIN-001`.
- Contradicted by evidence: none known.
- Uncertainty: none affecting normative meaning.

### CLM-XDOMAIN-002 — The global role is implementation drift, not accepted product authority

- Support state: SUPPORTED
- Supported by evidence: `EVD-XDOMAIN-001`, `EVD-XDOMAIN-002`, `EVD-XDOMAIN-006`.
- Contradicted by evidence: lower-authority current code and contract artifacts describe the role but cannot authorize it.
- Uncertainty: runtime deployment and persisted-row counts were not observed; those affect migration execution evidence, not the Decision.

### CLM-XDOMAIN-003 — A minimized cross-Domain projection still violates Domain isolation

- Support state: SUPPORTED
- Supported by evidence: `EVD-XDOMAIN-003`.
- Contradicted by evidence: none at the parent-authority level.
- Uncertainty: none; the forbidden property is cross-Domain visibility itself, not only payload size.

### CLM-XDOMAIN-004 — Soft revoke alone is insufficient for rollout and in-flight fail-closed behavior

- Support state: SUPPORTED
- Supported by evidence: `EVD-XDOMAIN-004`.
- Contradicted by evidence: no source-level response publication barrier was found.
- Uncertainty: production topology was not observed, so the implementation must record topology-specific drain evidence during COMPLIANCE.

### CLM-XDOMAIN-005 — Current implementation cannot be declared compliant by accepting this Spec

- Support state: SUPPORTED
- Supported by evidence: `EVD-XDOMAIN-002` through `EVD-XDOMAIN-006`.
- Contradicted by evidence: none.
- Uncertainty: exact future implementation commit is not yet available.

### CLM-XDOMAIN-006 — Existing admin provisioning can be retained without creating a replacement Coordinator

- Support state: SUPPORTED
- Supported by evidence: `EVD-XDOMAIN-005`.
- Contradicted by evidence: none, provided the retained path satisfies the actor, self-grant, transaction, and audit Contracts below.
- Uncertainty: none affecting normative meaning.

No `OPEN_ASSUMPTION` changes authority, Contract, compatibility, migration, rollback, audit, or security meaning.

## 7. Evidence relations

### EVD-XDOMAIN-001 — Parent authority supports strict Domain isolation

- Source observations: `OBS-XDOMAIN-001`.
- Target: `CLM-XDOMAIN-001`, `CLM-XDOMAIN-002`, `STATE-XDOMAIN-001`.
- Relation: SUPPORTS.
- Bound coordinates: repository base commit, source tree, observed `2026-08-20T15:02:28Z`.
- Strength/sufficiency: strong for Product Direction and Architecture scope.
- Limitations: does not evaluate deployed runtime behavior.
- Provenance: Product Boundary, v0.3.1, v0.3.2, and local authority map.

### EVD-XDOMAIN-002 — Role and route inventory supports implementation-drift Claim

- Source observations: `OBS-XDOMAIN-002`, `OBS-XDOMAIN-003`, `OBS-XDOMAIN-005`.
- Target: `CLM-XDOMAIN-002`, `STATE-XDOMAIN-002`.
- Relation: SUPPORTS.
- Bound coordinates: base commit and source tree.
- Strength/sufficiency: strong for existence of read/write/global-binding code paths.
- Limitations: source presence does not prove production deployment or row counts.
- Provenance: named code, migration, route, and test files.

### EVD-XDOMAIN-003 — Projection fields support the redaction-insufficiency Claim

- Source observations: `OBS-XDOMAIN-003`, `OBS-XDOMAIN-004`.
- Target: `CLM-XDOMAIN-003`, `STATE-XDOMAIN-003`.
- Relation: SUPPORTS.
- Bound coordinates: base commit and source tree.
- Strength/sufficiency: strong for the fields and absence of Domain predicates.
- Limitations: no network response was executed in this docs-only task.
- Provenance: SQL projections, DTOs, OpenAPI, SDK schemas, and test definitions.

### EVD-XDOMAIN-004 — Authorization transaction boundaries support the race Claim

- Source observations: `OBS-XDOMAIN-006`.
- Target: `CLM-XDOMAIN-004`, `STATE-XDOMAIN-004`.
- Relation: SUPPORTS.
- Bound coordinates: base commit and source tree.
- Strength/sufficiency: strong for source-level separation of checks and queries.
- Limitations: does not measure a deployed race frequency.
- Provenance: query service/repository and provisioning transaction code.

### EVD-XDOMAIN-005 — Existing control-plane separation supports retaining bounded admin provisioning

- Source observations: `OBS-XDOMAIN-010`.
- Target: `CLM-XDOMAIN-006`.
- Relation: SUPPORTS.
- Bound coordinates: base commit and source tree.
- Strength/sufficiency: moderate; retention remains conditional on the new Contracts.
- Limitations: current durable SecurityAudit behavior is incomplete and must change.
- Provenance: Product Boundary, Architecture, provisioning auth, and routes.

### EVD-XDOMAIN-006 — Governance and surface inventory support pending conformance

- Source observations: `OBS-XDOMAIN-007`, `OBS-XDOMAIN-008`, `OBS-XDOMAIN-009`.
- Target: `CLM-XDOMAIN-002`, `CLM-XDOMAIN-005`, `STATE-XDOMAIN-005`.
- Relation: SUPPORTS.
- Bound coordinates: base commit and accepted governance lock.
- Strength/sufficiency: strong for the need for a separate accepted Spec, implementation, and COMPLIANCE pass.
- Limitations: test definitions are provenance material, not executed Contract Evidence.
- Provenance: governance verifier, Specs, contracts, SDK, and tests.

## 8. Decisions

### DEC-XDOMAIN-001 — Enforce Option A without a Coordinator role

- Decision authority and acceptance actor: repository owner `mayf3`; this proposed selection becomes normative only when that actor accepts the exact reviewed Spec revision.
- Decision: `GLOBAL_WORKFLOW_COORDINATOR` is not an authorized product role. Ordinary product runtime has no deployment-global workflow or Assistance visibility and no Coordinator Domain-management power.
- Rejected alternatives: retain read-only Coordinator; retain a smaller/bounded Coordinator; rename or split the role; treat current implementation as requirement.
- Reason: all alternatives preserve the Product Boundary violation.
- Owner decision remaining: none.

### DEC-XDOMAIN-002 — Derive visibility only inside the target Domain

- Decision authority and acceptance actor: repository owner `mayf3`; this proposed selection becomes normative only when that actor accepts the exact reviewed Spec revision.
- Decision: visibility comes only from an enabled role binding in the target Domain or an explicit current/historical participation relationship authorized by v0.3.1 and related accepted Specs. Global roles and role combinations are not visibility sources.
- Rejected alternative: global-role overlay followed by field redaction.
- Reason: projection minimization cannot satisfy cross-Domain invisibility.
- Owner decision remaining: none.

### DEC-XDOMAIN-003 — Disable bindings transactionally, then retain a powerless tombstone temporarily

- Decision authority and acceptance actor: repository owner `mayf3`; this proposed selection becomes normative only when that actor accepts the exact reviewed Spec revision.
- Decision: a forward migration atomically disables all existing global bindings and installs a database-level prohibition on enabled global bindings before any mixed-version application rollout. The table is retained only as powerless historical/tombstone data for 30 days and then physically removed no later than day 90.
- Rejected alternatives: direct production edits; relying only on new application code; keeping active-compatible rows indefinitely; deleting the table before old binaries are excluded.
- Reason: fail closed during rolling deployment while preserving bounded cleanup provenance.
- Owner decision remaining: none.

### DEC-XDOMAIN-004 — Remove published surfaces with no authorization compatibility window

- Decision authority and acceptance actor: repository owner `mayf3`; this proposed selection becomes normative only when that actor accepts the exact reviewed Spec revision.
- Decision: there is no compatibility period in which cross-Domain authorization remains usable. Retired runtime paths return `404 route_not_found` through audited tombstone handlers during the observation window and through a uniform authenticated `/internal/v1/**` fallback after physical removal; the generic fallback preserves denial audit without preserving a role-specific handler. OpenAPI, HTTP contracts, SDK methods/types, errors, tests, role constants, and docs stop advertising the capability in the first enforcement release.
- Rejected alternatives: `403`, `410`, or `501` as steady-state retired-route semantics; opt-in legacy flag; deprecation while still returning data.
- Reason: 404 does not preserve a role oracle or suggest a supported capability; security removal cannot wait for client migration.
- Owner decision remaining: none.

### DEC-XDOMAIN-005 — Use containment-first, drain, migrate, then application rollout

- Decision authority and acceptance actor: repository owner `mayf3`; this proposed selection becomes normative only when that actor accepts the exact reviewed Spec revision.
- Decision: deployment order is ingress containment of retired surfaces, drain/cancel in-flight retired requests, transactional database disable-and-prohibit migration, enforcement application rollout, contract/SDK rollout, evidence observation, and finally table/handler removal.
- Rejected alternatives: application-first rolling deployment; reversible flag defaulting on; rollback that removes containment.
- Reason: old binaries must be powerless at every mixed-version point.
- Owner decision remaining: none.

### DEC-XDOMAIN-006 — Retain only the existing admin Owner-management path

- Decision authority and acceptance actor: repository owner `mayf3`; this proposed selection becomes normative only when that actor accepts the exact reviewed Spec revision.
- Decision: only the existing `/internal/v1/admin/domains/{domainId}/owner` control-plane operation may replace a Domain Owner. The actor must be an enabled direct AGENT Principal with `workflow.admin` and explicit provisioning allowlist membership, must name one exact Domain, and must not set itself as Owner. Current Domain Owner self-service and any Coordinator path are forbidden.
- Rejected alternatives: Coordinator owner replacement; Owner self-replacement; role-combination-based owner replacement; new bounded Coordinator.
- Reason: preserve Product Boundary Admin/Provisioning while preventing indirect self-elevation.
- Owner decision remaining: none.

### DEC-XDOMAIN-007 — Treat audit and response publication as part of authorization

- Decision authority and acceptance actor: repository owner `mayf3`; this proposed selection becomes normative only when that actor accepts the exact reviewed Spec revision.
- Decision: denied cross-Domain requests, retired-surface calls, cleanup, revocation, and Owner replacement have durable audit obligations. Containment/revoke is not complete until pre-existing privileged requests cannot publish a payload.
- Rejected alternative: logs-only audit or authorization check detached from response publication.
- Reason: security and rollback evidence must be reviewable after the fact.
- Owner decision remaining: none.

### DEC-XDOMAIN-008 — Require new implementation and exact COMPLIANCE

- Decision authority and acceptance actor: repository owner `mayf3`; this proposed selection becomes normative only when that actor accepts the exact reviewed Spec revision.
- Decision: acceptance authorizes only a separate implementation task bounded by the Contracts. The exact implementation commit must later receive Contract-by-Contract COMPLIANCE; current code is not retroactively authorized.
- Rejected alternative: mark current behavior compliant because the Spec documents it.
- Reason: accepted authority, implementation progress, and conformance are distinct.
- Owner decision remaining: none.

## 9. Contracts

### Domain isolation

#### CTR-XDOMAIN-ISOLATION-001 — Domain-local visibility sources only

For every ordinary product read, the service MUST derive visibility only from a binding role that a parent or accepted governing authority explicitly authorizes for the target object's Domain (under v0.3.1, the enabled `DOMAIN_OWNER` binding), or from the explicit current/historical participation relationships authorized by v0.3.1. A participation relationship authorizes only the related Workflow Instance and never all facts in its Domain. A generic member binding is not a Domain-wide workflow-data visibility source. The service MUST NOT derive visibility from any deployment-global role, claim, allowlist, configuration, or role combination.

#### CTR-XDOMAIN-ISOLATION-002 — No deployment-global workflow projection

The ordinary product API, application services, repositories, SDK, and HTTP contract MUST NOT provide a projection that lists, searches, counts, filters, aggregates, exports, or streams workflow or Assistance facts across more than one Domain in one authorization context.

#### CTR-XDOMAIN-ISOLATION-003 — No role-combination bypass

Possessing `workflow.read`, `workflow.execute`, `workflow.admin`, provisioning allowlist membership, Domain roles in other Domains, former global bindings, or any combination of them MUST NOT grant workflow or Assistance visibility in a Domain unless the caller independently satisfies that target Domain's accepted visibility rule. Control-plane provisioning authority MUST NOT be treated as workflow-data read authority.

### Role lifecycle

#### CTR-XDOMAIN-ROLE-001 — Global role is invalid and non-grantable

`GLOBAL_WORKFLOW_COORDINATOR` MUST NOT be an accepted product permission, runtime authorization branch, supported role constant, request value, token claim, configuration entry, seed, or grant target. Attempts to create or re-enable that role through any supported interface MUST fail without changing data.

#### CTR-XDOMAIN-ROLE-002 — Existing bindings become powerless atomically

The containment migration MUST, in one database transaction, lock the global-binding write boundary, set every existing binding to disabled with a non-null disable time, record cleanup audit facts, and install a database-enforced rule that rejects any insert or update whose resulting `enabled` value is true. Transaction failure MUST leave the prior schema/data unchanged and block application rollout.

#### CTR-XDOMAIN-ROLE-003 — Disabled Principal and stale binding cannot authorize

A disabled Principal MUST receive no permission from a global binding, cached authorization, receipt, replay, or pre-issued token. Principal disablement and global-role containment MUST invalidate the relevant authorization state before any later response can expose protected data.

#### CTR-XDOMAIN-ROLE-004 — No reactivation by rollback

No rollback, down migration, old binary, manual replay, seed, restore procedure, or feature flag MAY re-enable a global binding or make a disabled/tombstoned row authoritative. A rollback plan that requires restoring cross-Domain permission is invalid.

### Read surfaces

#### CTR-XDOMAIN-READ-001 — Global instance list is retired

`GET /internal/v1/workflow-instances/global` MUST return no workflow data. During the audited tombstone period, an authenticated call MUST return `404` with the standard `route_not_found` envelope regardless of former role, filters, IDs, or data. After handler removal, a uniform authenticated `/internal/v1/**` fallback MUST preserve the same status, envelope, authentication ordering, denial-audit class, and response headers; it MUST classify only the attempted route string and MUST NOT perform Coordinator, object, Domain, or Principal lookup.

#### CTR-XDOMAIN-READ-002 — Cross-Domain per-object reads are indistinguishable

When the required audit path is available, for retained instance, timeline, submission, Definition, and Assistance detail routes, a caller lacking target-Domain visibility MUST receive the same `404` not-found-or-not-visible status, code, body shape, headers, pagination behavior, and identifier-dependent externally observable behavior as a well-formed nonexistent identifier. The response MUST expose no object, Domain, version, lifecycle, role, or existence fact.

#### CTR-XDOMAIN-READ-003 — Cross-Domain sensitive fields never leave the service

A cross-Domain denial MUST NOT expose requester identity, creator or assignee identity, Context title or payload, `supportingPayload`, request/escalation/resolution payload, Definition or node facts, state/version differences, timestamps, existence, or count. Redaction after a global query is forbidden; authorization MUST prevent retrieval or projection for the caller.

### Write surfaces

#### CTR-XDOMAIN-WRITE-001 — Coordinator Domain creation is retired

`POST /internal/v1/domains` MUST NOT exist as a Coordinator or ordinary runtime creation path and MUST return audited `404 route_not_found` during the tombstone period. Domain creation remains only on the existing admin provisioning route under its independent control-plane authority.

#### CTR-XDOMAIN-WRITE-002 — Coordinator Owner replacement is retired

`PUT /internal/v1/domains/{domainId}/owner` MUST NOT exist as a Coordinator or ordinary runtime Owner-management path and MUST return audited `404 route_not_found` without checking target Domain or Principal existence.

#### CTR-XDOMAIN-WRITE-003 — Global-role provisioning surfaces are retired

PUT and DELETE `/internal/v1/admin/global-role-bindings/{principalId}` MUST return audited `404 route_not_found` and MUST NOT grant, reactivate, revoke, mutate, or reveal a binding. Bulk containment and cleanup MUST occur through the governed forward migration, not a retained product API.

### Domain Owner management

#### CTR-XDOMAIN-OWNER-001 — Retained actor and Domain scope

Only an enabled AGENT Principal using an access token whose authenticated subject is the actor and that has no OBO/delegation identity (a direct token), and that simultaneously has `workflow.admin` and membership in `WORKFLOW_PROVISIONING_PRINCIPAL_IDS`, MAY call the retained admin Owner-replacement route. Authentication/directness, Principal enabled/type, scope, and allowlist MUST be evaluated in that order before target lookup. Each command MUST target exactly one explicit Domain and one enabled target Principal. Neither the caller's former global binding nor Domain roles in another Domain affect this authority.

#### CTR-XDOMAIN-OWNER-002 — Atomic replacement, no self-grant, durable audit

The retained admin Owner replacement MUST reject `actorPrincipalId == newOwnerPrincipalId`. It MUST lock the target Principal and Domain, validate both, disable the old Owner binding, enable the new Owner binding, complete the idempotency receipt, and write a durable SecurityAudit containing actor, target Domain, old Owner, new Owner, command/request identity, reason/reference, and result in one transaction. Any failure MUST commit none of those state changes.

#### CTR-XDOMAIN-OWNER-003 — Owner authority remains Domain-local

Becoming Owner of one Domain MAY grant only powers that parent or separately accepted authorities assign to that Domain Owner in that same Domain; this Spec creates none of those positive powers. It MUST NOT grant access to any other Domain. No caller may use a retired Coordinator path, role provisioning, replay, or self-assignment to obtain Owner-derived timeline, Cancel, Archive, Definition management, or Assistance resolve authority. The Cancel / Archive references enforce non-elevation relative to v0.3.2 and do not modify, supersede, or re-authorize that authority.

### Assistance

#### CTR-XDOMAIN-ASSISTANCE-001 — Global HUMAN_REQUIRED list is retired

`GET /internal/v1/assistance-cases/human-required` MUST return audited `404 route_not_found` and MUST NOT list, count, page, filter, or reveal any Assistance Case to a former Coordinator.

#### CTR-XDOMAIN-ASSISTANCE-002 — Detail has no Coordinator fallback

`GET /internal/v1/assistance-cases/{assistanceCaseId}` MUST NOT fall back to a global-role projection. A former Coordinator without independently accepted Domain-local or participant authority MUST receive the same 404 as a nonexistent Case whether status is `OWNER_PENDING`, `HUMAN_REQUIRED`, `RESOLVED`, or `VOIDED`. This Contract does not authorize requester, Owner, status, or lifecycle semantics that lack separate accepted authority.

#### CTR-XDOMAIN-ASSISTANCE-003 — No Assistance authority is created by this Spec

This Spec authorizes only removal of global Coordinator Assistance visibility. It MUST NOT be cited to authorize the current assignee request path, Owner inbox/escalate/resolve, requester detail, Assistance lifecycle, or their transaction/idempotency semantics. A later implementation MAY retain a Domain-local Assistance surface only when an accepted authority present in that implementation base independently authorizes its exact actors and behavior; otherwise that surface remains unauthorized and MUST be hard-disabled. In either case, removing Coordinator visibility MUST NOT be implemented by expanding another actor's visibility.

### Migration and data

#### CTR-XDOMAIN-MIGRATION-001 — Migration 0020 remains immutable history

Migration `0020_global_role_bindings.sql` MUST NOT be edited, reordered, or treated as normative authority. A new forward migration MUST implement containment. Fresh database installation MUST apply `0020` as history and then apply the containment migration before readiness can succeed.

#### CTR-XDOMAIN-MIGRATION-002 — Tombstone retention and data minimization

For the first 30 continuous days after full enforcement rollout, the table MAY remain only with all rows disabled and database enforcement preventing activation. During that interval, code MAY access it only from bounded migration/cleanup verification, not product authorization. During retention, tombstone rows MUST retain exactly `binding_id`, `principal_id`, `role_key`, `enabled`, `created_at`, and `disabled_at`; `enabled` MUST be false and `disabled_at` non-null. Only the migration executor and bounded cleanup verifier MAY read them. No new product field or row may be added.

#### CTR-XDOMAIN-MIGRATION-003 — Mandatory physical deletion window

All rollback artifacts and deployed binaries that require `global_role_bindings` MUST be retired by day 30 after full enforcement rollout. The table, indexes, constraints, role-specific database code, and tombstone handlers MUST be physically removed in the first deployment after day 30 and no later than day 90. Before drop, an executed check MUST prove zero enabled rows, no mutation after the identified containment transaction committed, completed cleanup audit, and no deployed code dependency. Missing any proof blocks the drop; missing the day-90 deadline makes conformance `DRIFTED` and does not permit reactivation.

### Compatibility and published contracts

#### CTR-XDOMAIN-COMPAT-001 — No authorization feature flag

No feature flag, allowlist, environment variable, emergency switch, configuration profile, or per-client exception MAY enable any retired global capability. If a temporary rollout flag controls only selection between an audited 404 tombstone handler and absent-route fallback, its default MUST be deny, it MUST be impossible to return product data, and it MUST be deleted with the tombstone handler.

#### CTR-XDOMAIN-COMPAT-002 — Old client failure semantics

After admission containment begins, old clients MUST receive no cross-Domain data. A `403 global_coordinator_required` MAY be published only by an old request whose response publication completed before the recorded admission-containment boundary. At or after that boundary, every in-flight old authorization branch MUST be cancelled or have its response discarded, and no new request may enter it. After enforcement routing is active, retired paths MUST return `404 route_not_found`. `410` and `501` MUST NOT be used. Retained per-object cross-Domain denials MUST use the indistinguishable 404 required by `CTR-XDOMAIN-READ-002`.

#### CTR-XDOMAIN-COMPAT-003 — OpenAPI, HTTP contract, SDK, errors, tests, and docs remove capability

The first enforcement release MUST remove retired operations, schemas, role errors, role constants, SDK methods/types/exports, contract fixtures, conformance expectations, tests that assert success, metrics labels that present the role as valid, operational scripts (including Coordinator Owner-readiness paths), and documentation that advertises it. OpenAPI, HTTP contract prose, errors, changelog, manifest file set/digests/version, runtime contract/schema version metadata, SDK constants/digests/exports, and generated artifacts MUST be regenerated from the capability-free source and agree with one another. Replacement tests MUST assert denial/removal and Domain-local behavior. Compatibility shims MUST NOT make network calls to retired paths.

#### CTR-XDOMAIN-COMPAT-004 — Provisioning allowlist is not data authority

`WORKFLOW_PROVISIONING_PRINCIPAL_IDS` MAY continue to authorize existing admin provisioning operations, but MUST NOT authorize global workflow/Assistance reads, ordinary runtime Domain creation, the retired runtime Owner route, or global-role grant/revoke. Configuration validation MUST reject any new Coordinator/global-visibility setting.

### Rollout and rollback

#### CTR-XDOMAIN-ROLLBACK-001 — Containment-first deployment order

Deployment MUST proceed in this order: (1) install ingress deny for every request path or retained-route authorization branch that can consult a global binding, including the Assistance detail fallback, plus both global-role provisioning methods; (2) stop admission and drain or cancel every in-flight request that has made or could make a global-binding authorization decision without publishing its payload; (3) execute and verify the containment migration; (4) deploy enforcement binaries to all instances; (5) publish capability-free contracts/SDK; (6) observe audits/metrics; (7) complete mandatory physical deletion. Traffic eligibility MUST be enforced externally for old binaries that cannot recognize the containment schema; new binaries' readiness MUST fail whenever the required containment schema is absent.

#### CTR-XDOMAIN-ROLLBACK-002 — Rollback preserves denial

Application rollback MAY restore only a binary proven not to depend on enabled global bindings and proven to return no retired data. Database rollback MUST NOT remove the activation prohibition or restore enabled rows. If no safe binary is available, rollback means roll forward, shutdown, or traffic isolation—not permission restoration. Rollback execution MUST record artifact identities and post-rollback denial evidence.

### Audit and observability

#### CTR-XDOMAIN-AUDIT-001 — Denied requests are durably audited without sensitive payload

Every authenticated call to a retired surface and every denied cross-Domain retained-route request MUST write a durable SecurityAudit with request/correlation ID, actor, action, route class, target identifier hash where supplied, denial reason, response status, and time. A well-formed nonexistent identifier attempt MUST use the same externally indistinguishable audit path and public denial class, with the internal reason protected from the caller. Audit/log/metric records MUST NOT contain tokens, Context title/payload, Assistance payload, `supportingPayload`, requester identity as request content, or other protected body fields. If the durable audit write is unavailable, both a cross-Domain existing-object attempt and the paired nonexistent-object attempt MUST fail before any protected body is constructed and MUST return the same `503 security_audit_unavailable` envelope and headers; no data response may be published. The authenticated internal fallback uses the same outage rule.

#### CTR-XDOMAIN-AUDIT-002 — Cleanup, revoke, Principal disable, and Owner replacement are audited

The containment migration MUST write a durable batch audit with affected-row count and a deterministic integrity digest, migration identity, authenticated executor identity, start/commit time, and result. The digest MUST use JCS canonicalization of the ordered cleanup record plus SHA-256; it detects accidental mismatch but is not represented as cryptographic tamper evidence or an external trust anchor. Principal disable and retained Owner replacement MUST have durable before/after authority audit. Idempotent replay MUST reference the original audit and MUST NOT create a contradictory success record.

#### CTR-XDOMAIN-AUDIT-003 — Metrics prove absence, not a supported role

Metrics MUST count retired-route attempts, cross-Domain denials, cleanup results, blocked activation attempts, and rollback-denial checks without actor or object high-cardinality labels. Logs and dashboards MUST describe the role as retired/forbidden, never as active or degraded. Observation-period evidence MUST include zero successful retired responses and zero enabled-binding writes.

### Security and concurrency

#### CTR-XDOMAIN-SECURITY-001 — Containment and in-flight boundary is fail closed

Containment/revoke begins at the recorded admission-containment boundary when admission to every path or retained-route branch that can use global-binding authorization is disabled. From that point, no newly admitted or already in-flight request using that authorization branch may publish product data. The rollout MUST cancel or discard pending response bodies before the containment transaction commits. The implementation MUST provide a cross-process barrier or equivalent protocol proving that a request authorized before containment cannot emit a payload after containment is declared complete.

#### CTR-XDOMAIN-SECURITY-002 — Replay and caches cannot resurrect data

No command receipt, HTTP cache, application cache, SDK cache, retry, idempotency replay, database snapshot, replica, or queued response MAY return a prior successful global list, Assistance projection, Domain create, or Owner-replace response after containment. Retired read responses MUST be non-cacheable; privileged write receipts MAY remain historical but replay MUST return denial and MUST NOT reapply or disclose the old success body.

#### CTR-XDOMAIN-SECURITY-003 — Identifier enumeration is forbidden

Case ID, Instance ID, Definition ID, Domain ID, Principal ID, cursor, filter, status, and workflow state/version differences MUST NOT let a caller distinguish a cross-Domain object from a nonexistent object. Acceptance MUST exercise existing/nonexisting pairs across active, terminal, cancelled, archived, and all Assistance statuses and compare status, code, body shape, headers, pagination, audit class, and any deterministic version-dependent behavior.

#### CTR-XDOMAIN-SECURITY-004 — Surface-complete removal

Implementation MUST search and account for every role/surface reference in Rust, SQL, migrations, OpenAPI, HTTP contract bundle, SDK, tests, deployment/configuration, seeds, scripts, metrics, logs, audit, docs, fixtures, generated artifacts, and security/enumeration paths. Any executable ordinary-runtime reference that can grant, test, or consume global Coordinator authority fails this Contract. Historical Migration `0020` remains permanent immutable history. Non-executable forensic, audit, conformance, and historical records MAY retain the literal only when clearly labeled historical and non-authoritative. Bounded migration/tombstone references are temporary under `CTR-XDOMAIN-MIGRATION-001` through `003`.

## 10. Acceptance

Acceptance items define future verification. None has been executed by this docs-only authoring change. Every result MUST bind the accepted Spec revision, exact implementation commit, environment, configuration, migration identity, and execution time.

### ACC-XDOMAIN-001 — Domain-local authorization matrix

- Contracts: `CTR-XDOMAIN-ISOLATION-001`, `CTR-XDOMAIN-ISOLATION-003`, `CTR-XDOMAIN-OWNER-003`.
- Method: integration matrix across two Domains for owner, member, current assignee, creator, historical participant, provisioning admin, former Coordinator, disabled Principal, and combinations of scopes/roles.
- Environment: migrated test database and production-equivalent application topology.
- Required evidence: executed requests, actor/binding fixtures, implementation commit, response records, and SecurityAudit rows.
- Expected result: each actor sees only facts explicitly authorized for the target object; a generic member and a provisioning admin alone see no Domain-wide workflow data, while participation visibility remains limited to the related Instance.
- Failure condition: any global role/scope/allowlist/role combination yields visibility or Owner-derived power in another Domain.

### ACC-XDOMAIN-002 — No global projection remains

- Contracts: `CTR-XDOMAIN-ISOLATION-002`, `CTR-XDOMAIN-READ-001`, `CTR-XDOMAIN-ASSISTANCE-001`.
- Method: call retired global instance and `human-required` paths with former Coordinator, admin, owner, ordinary, and disabled identities; inspect route and repository inventories.
- Environment: mixed-version containment and full enforcement environments.
- Required evidence: response/audit records and static route/query inventory.
- Expected result: no call returns data; final routing returns `404 route_not_found`; no global repository query remains.
- Failure condition: global list or Coordinator Assistance remains accessible, filterable, countable, or discoverable.

### ACC-XDOMAIN-003 — Cross-Domain sensitive-field denial

- Contracts: `CTR-XDOMAIN-READ-002`, `CTR-XDOMAIN-READ-003`, `CTR-XDOMAIN-ASSISTANCE-002`.
- Method: compare cross-Domain and nonexistent IDs for instance detail, timeline, submissions, Definition detail, and every Assistance status using payloads containing unique requester IDs, Context titles, and `supportingPayload` sentinels; add repository-level SQL/query instrumentation proving authorization reads only the minimum object-to-Domain metadata and does not fetch or construct protected payload fields for a denied caller.
- Environment: integration and staging.
- Required evidence: paired requests/responses, logs, audit records, and packet/body capture with secrets removed.
- Expected result: identical 404-class external semantics and no sentinel or identity leakage.
- Failure condition: requester identity, Context title, `supportingPayload`, status/version, existence, or any other protected fact appears or changes the response oracle.

### ACC-XDOMAIN-004 — Global role cannot be granted or revived

- Contracts: `CTR-XDOMAIN-ROLE-001`, `CTR-XDOMAIN-ROLE-003`, `CTR-XDOMAIN-ROLE-004`, `CTR-XDOMAIN-COMPAT-004`.
- Method: attempt API grant/revoke, direct fixture insert/update, old binary upsert, seed, config, disabled-Principal token, restore, and down/rollback paths.
- Environment: migrated test database plus mixed-version rollout rehearsal.
- Required evidence: SQL errors, HTTP responses, row snapshots, config validation, and audits.
- Expected result: no enabled row or effective permission can be created; disabled Principal never succeeds.
- Failure condition: an old binding, disabled Principal, old binary, allowlist, or rollback regains permission.

### ACC-XDOMAIN-005 — Transactional binding containment

- Contracts: `CTR-XDOMAIN-ROLE-002`, `CTR-XDOMAIN-MIGRATION-001`.
- Method: seed multiple enabled/disabled bindings, run migration under concurrent grant attempts, inject failure before commit, retry, and install from an empty database through all migrations.
- Environment: disposable PostgreSQL matching production major version.
- Required evidence: migration logs, locks, before/after rows, constraints/triggers, failure rollback, and readiness output.
- Expected result: one successful transaction disables all rows and prohibits activation; failed attempt changes nothing; fresh install ends contained.
- Failure condition: partial disable, missing disable time, concurrent enabled row, edited `0020`, or readiness before containment.

### ACC-XDOMAIN-006 — Coordinator writes are gone

- Contracts: `CTR-XDOMAIN-WRITE-001`, `CTR-XDOMAIN-WRITE-002`, `CTR-XDOMAIN-WRITE-003`.
- Method: call all retired POST/PUT/DELETE paths using former Coordinator, provisioning admin, Domain Owner, self-target, other-target, valid/invalid IDs, and replay keys.
- Environment: integration and staging.
- Required evidence: responses, zero database mutations, receipt inventory, and audits.
- Expected result: audited `404 route_not_found`, no Domain/binding/Owner mutation, and no replay of old success.
- Failure condition: Coordinator creates a Domain, replaces any Owner, grants/revokes a role, or learns target existence.

### ACC-XDOMAIN-007 — Retained Owner-management boundary

- Contracts: `CTR-XDOMAIN-OWNER-001`, `CTR-XDOMAIN-OWNER-002`.
- Method: exercise retained admin route with every missing/present actor condition, self-target, disabled target, concurrent replacement, idempotent replay, and injected audit/receipt failure.
- Environment: PostgreSQL integration.
- Required evidence: lock/transaction traces, binding state, receipt, SecurityAudit, and response.
- Expected result: only qualified admin may set another enabled Principal as Owner of the named Domain; replacement and audit are atomic.
- Failure condition: self-grant, Coordinator path, owner self-service, partial replacement, missing audit, or cross-Domain side effect.

### ACC-XDOMAIN-008 — Owner-derived indirect elevation is blocked

- Contracts: `CTR-XDOMAIN-OWNER-003`, `CTR-XDOMAIN-SECURITY-003`.
- Method: former Coordinator/admin attempts to make itself Owner directly, via replay, role combination, race, stale receipt, and crafted target IDs, then attempts timeline, Cancel, Archive, Definition management, and Assistance resolve.
- Environment: integration with two Domains.
- Required evidence: request chain, final bindings, responses, and audits.
- Expected result: no self-assignment and no indirect privilege.
- Failure condition: any sequence yields Owner-derived authority in an unauthorized Domain.

### ACC-XDOMAIN-009 — Assistance authority non-creation

- Contracts: `CTR-XDOMAIN-ASSISTANCE-003`.
- Method: inspect the implementation preflight, accepted-authority inventory, route disposition, and tests for every retained Assistance behavior.
- Environment: clean checkout of the exact implementation base and implementation commit.
- Required evidence: exact accepted authority revision for each retained Assistance actor/behavior, or denial/removal evidence when no such authority exists; Coordinator denial tests in either case.
- Expected result: this Spec is cited only for Coordinator-visibility removal, no unsupported Assistance behavior is treated as authorized, and no replacement actor receives expanded visibility.
- Failure condition: implementation cites this Spec to authorize Assistance lifecycle/actors, retains behavior without separate accepted authority, or shifts global visibility to another role.

### ACC-XDOMAIN-010 — Tombstone retention and physical deletion

- Contracts: `CTR-XDOMAIN-MIGRATION-002`, `CTR-XDOMAIN-MIGRATION-003`.
- Method: inspect day-0, day-30, and final-drop deployments and verify artifact retirement, no post-containment-commit mutation, cleanup digest, schema deletion, and no code dependency.
- Environment: staging rehearsal and production conformance report.
- Required evidence: timestamped schema/row queries, deployment manifests, audit/metric window, and drop migration result.
- Expected result: powerless tombstone only during allowed window and complete physical deletion by day 90.
- Failure condition: enabled row, product read of tombstone, old artifact after day 30, premature unproven drop, or table present after day 90.

### ACC-XDOMAIN-011 — Compatibility and old clients fail closed

- Contracts: `CTR-XDOMAIN-COMPAT-001`, `CTR-XDOMAIN-COMPAT-002`.
- Method: use released old SDK/client binaries before containment, during drain, after enforcement, and after handler removal; inspect all flags/configurations.
- Environment: rolling-deployment rehearsal.
- Required evidence: client versions, routing phase, responses, cancellation proof, and configuration dump.
- Expected result: no phase returns new cross-Domain data after containment begins; steady state is 404; no enabling flag exists.
- Failure condition: a compatibility window, feature flag, 410/501 steady state, or old client returns data.

### ACC-XDOMAIN-012 — Published surface and test inventory is capability-free

- Contracts: `CTR-XDOMAIN-COMPAT-003`, `CTR-XDOMAIN-SECURITY-004`.
- Method: repository-wide reference inventory plus OpenAPI/contract/SDK generation and compile/test run.
- Environment: clean checkout of exact implementation commit.
- Required evidence: changed-path inventory, generated diffs, zero unexpected reference report, and executed test outputs.
- Expected result: no executable, advertising, authorization, SDK, or success-assertion reference remains; immutable `0020`, clearly labeled non-authoritative historical records, and bounded migration/audit provenance are the only allowed literal references until final drop.
- Failure condition: old SDK/route bypass, advertised role, success test, role constant, config, script, or executable global query remains.

### ACC-XDOMAIN-013 — Containment-first rolling deployment

- Contracts: `CTR-XDOMAIN-ROLLBACK-001`, `CTR-XDOMAIN-SECURITY-001`.
- Method: production-topology rollout rehearsal with long-running list/detail requests, queued responses, concurrent grants, and old/new instances.
- Environment: staging topology equivalent to production.
- Required evidence: topology manifest (instance count, ingress, queues/caches/replicas, migration runner), ingress state, drain/cancellation records, migration commit, instance versions, external traffic-eligibility decisions, response capture, and new-binary readiness.
- Expected result: no payload is published after containment begins, old binaries are externally removed from traffic even when their own readiness lacks containment awareness, and no old instance can serve a successful global-authorization request.
- Failure condition: application-first exposure, undrained response, successful mixed-version request, or readiness without containment.

### ACC-XDOMAIN-014 — Rollback cannot reopen authority

- Contracts: `CTR-XDOMAIN-ROLLBACK-002`, `CTR-XDOMAIN-ROLE-004`.
- Method: roll back each deployable application artifact and exercise database rollback/recovery procedure under old tokens/bindings.
- Environment: staging rollback rehearsal.
- Required evidence: artifact/schema identities, binding state, denial responses, and rollback audit.
- Expected result: every allowed rollback remains deny-only; unsafe binary is refused and traffic remains isolated.
- Failure condition: rollback re-enables a binding, route, global read, Domain creation, or Owner replacement.

### ACC-XDOMAIN-015 — Replay and caches are powerless

- Contracts: `CTR-XDOMAIN-SECURITY-002`.
- Method: replay old idempotency keys and stored success receipts; exercise HTTP/application caches, queued responses, retry workers, and database snapshots after containment.
- Environment: migration fixture and staging.
- Required evidence: cache headers/config, receipt before/after, response records, and zero writes.
- Expected result: only denial is returned and no old success body or mutation is replayed.
- Failure condition: any cache, receipt, replay, snapshot, or queue returns protected data or applies a retired write.

### ACC-XDOMAIN-016 — Enumeration resistance

- Contracts: `CTR-XDOMAIN-SECURITY-003`, `CTR-XDOMAIN-READ-002`.
- Method: automated paired-oracle suite across real/nonexistent UUIDs, Case/Instance states, versions, cursors, malformed-but-well-routed inputs, and repeated samples.
- Environment: integration and staging.
- Required evidence: normalized status/header/body comparisons, audit-class comparisons, and bounded timing analysis.
- Expected result: no stable identifier- or state-dependent distinction beyond authentication and syntactic validation that occurs before object lookup.
- Failure condition: Case ID, Instance ID, state/version, status, cursor, or timing class reliably enumerates cross-Domain existence.

### ACC-XDOMAIN-017 — Durable audit is complete and payload-safe

- Contracts: `CTR-XDOMAIN-AUDIT-001`, `CTR-XDOMAIN-AUDIT-002`.
- Method: execute denied, cleanup, Principal-disable, Owner-replace, replay, and injected audit-failure scenarios; query durable audit storage and logs.
- Environment: integration and staging.
- Required evidence: request IDs, audit rows/digests, transaction outcomes, and redaction scan.
- Expected result: every required event is durable, correlated, atomic where required, and contains no protected payload; paired existing-cross-Domain and nonexistent attempts share the public audit/denial class, and audit outage yields identical `503 security_audit_unavailable` responses before protected-body construction.
- Failure condition: unrecorded denial/cleanup, unaudited revoke, logs-only evidence, contradictory replay audit, payload/token/title leakage, or privileged success when audit fails.

### ACC-XDOMAIN-018 — Metrics and observation window prove zero success

- Contracts: `CTR-XDOMAIN-AUDIT-003`.
- Method: inspect metrics schema/dashboards and collect the full tombstone observation window.
- Environment: staging and production conformance report.
- Required evidence: metric definitions, cardinality review, time series, and linked audits.
- Expected result: attempts/denials/blocked writes are countable; successful retired responses and enabled-binding writes remain zero; no role is presented as supported.
- Failure condition: missing metrics, sensitive/high-cardinality labels, any successful retired response, enabled write, or active-role dashboard language.

### Contract coverage

| Contract | Acceptance |
|---|---|
| `CTR-XDOMAIN-ISOLATION-001` | `ACC-XDOMAIN-001` |
| `CTR-XDOMAIN-ISOLATION-002` | `ACC-XDOMAIN-002` |
| `CTR-XDOMAIN-ISOLATION-003` | `ACC-XDOMAIN-001` |
| `CTR-XDOMAIN-ROLE-001` | `ACC-XDOMAIN-004`, `ACC-XDOMAIN-012` |
| `CTR-XDOMAIN-ROLE-002` | `ACC-XDOMAIN-005` |
| `CTR-XDOMAIN-ROLE-003` | `ACC-XDOMAIN-004`, `ACC-XDOMAIN-013`, `ACC-XDOMAIN-015` |
| `CTR-XDOMAIN-ROLE-004` | `ACC-XDOMAIN-004`, `ACC-XDOMAIN-014` |
| `CTR-XDOMAIN-READ-001` | `ACC-XDOMAIN-002` |
| `CTR-XDOMAIN-READ-002` | `ACC-XDOMAIN-003`, `ACC-XDOMAIN-016` |
| `CTR-XDOMAIN-READ-003` | `ACC-XDOMAIN-003` |
| `CTR-XDOMAIN-WRITE-001` | `ACC-XDOMAIN-006` |
| `CTR-XDOMAIN-WRITE-002` | `ACC-XDOMAIN-006` |
| `CTR-XDOMAIN-WRITE-003` | `ACC-XDOMAIN-006` |
| `CTR-XDOMAIN-OWNER-001` | `ACC-XDOMAIN-007` |
| `CTR-XDOMAIN-OWNER-002` | `ACC-XDOMAIN-007` |
| `CTR-XDOMAIN-OWNER-003` | `ACC-XDOMAIN-001`, `ACC-XDOMAIN-008` |
| `CTR-XDOMAIN-ASSISTANCE-001` | `ACC-XDOMAIN-002` |
| `CTR-XDOMAIN-ASSISTANCE-002` | `ACC-XDOMAIN-003` |
| `CTR-XDOMAIN-ASSISTANCE-003` | `ACC-XDOMAIN-009` |
| `CTR-XDOMAIN-MIGRATION-001` | `ACC-XDOMAIN-005` |
| `CTR-XDOMAIN-MIGRATION-002` | `ACC-XDOMAIN-010` |
| `CTR-XDOMAIN-MIGRATION-003` | `ACC-XDOMAIN-010` |
| `CTR-XDOMAIN-COMPAT-001` | `ACC-XDOMAIN-011` |
| `CTR-XDOMAIN-COMPAT-002` | `ACC-XDOMAIN-011` |
| `CTR-XDOMAIN-COMPAT-003` | `ACC-XDOMAIN-012` |
| `CTR-XDOMAIN-COMPAT-004` | `ACC-XDOMAIN-004` |
| `CTR-XDOMAIN-ROLLBACK-001` | `ACC-XDOMAIN-010`, `ACC-XDOMAIN-012`, `ACC-XDOMAIN-013`, `ACC-XDOMAIN-018` |
| `CTR-XDOMAIN-ROLLBACK-002` | `ACC-XDOMAIN-014` |
| `CTR-XDOMAIN-AUDIT-001` | `ACC-XDOMAIN-017` |
| `CTR-XDOMAIN-AUDIT-002` | `ACC-XDOMAIN-017` |
| `CTR-XDOMAIN-AUDIT-003` | `ACC-XDOMAIN-018` |
| `CTR-XDOMAIN-SECURITY-001` | `ACC-XDOMAIN-013` |
| `CTR-XDOMAIN-SECURITY-002` | `ACC-XDOMAIN-015` |
| `CTR-XDOMAIN-SECURITY-003` | `ACC-XDOMAIN-008`, `ACC-XDOMAIN-016` |
| `CTR-XDOMAIN-SECURITY-004` | `ACC-XDOMAIN-012` |

```text
CONTRACT_COUNT = 35
CONTRACTS_WITH_ACCEPTANCE = 35
ACCEPTANCE_EXECUTED_IN_THIS_CHANGE = NO
```

## 11. Alternatives and disposition

### ALT-XDOMAIN-001 — Keep a read-only or redacted Coordinator

- Disposition: rejected.
- Reason: any deployment-global workflow/Assistance visibility violates the Product Boundary; redaction does not change scope.
- Evidence/Claims considered: `CLM-XDOMAIN-001`, `CLM-XDOMAIN-003`.
- What would reopen: only a new Product Direction that explicitly changes Domain isolation; this Spec cannot do so.

### ALT-XDOMAIN-002 — Introduce a bounded or renamed Coordinator

- Disposition: rejected.
- Reason: it repackages the unauthorized role and contradicts the frozen Owner decision.
- Evidence/Claims considered: `DEC-XDOMAIN-001`.
- What would reopen: not within this authority or implementation task.

### ALT-XDOMAIN-003 — Keep role grant/revoke but remove read routes

- Disposition: rejected.
- Reason: a grantable dormant permission is an unsafe rollback and compatibility seam and has no accepted product meaning.
- Evidence/Claims considered: `CLM-XDOMAIN-002`, `CLM-XDOMAIN-004`.
- What would reopen: none under Option A.

### ALT-XDOMAIN-004 — Drop the table immediately before rollout

- Disposition: rejected.
- Reason: old binaries would fail unpredictably during mixed-version deployment, and cleanup evidence would be lost before safe artifact retirement.
- Evidence/Claims considered: `CLM-XDOMAIN-004`.
- What would reopen: a proven single-instance atomic outage deployment with equivalent durable cleanup evidence; it would still require exact Spec amendment before acceptance, not implementation discretion.

### ALT-XDOMAIN-005 — Return 410 or 501 to old clients

- Disposition: rejected.
- Reason: those statuses advertise a known former/special capability and create a route oracle. Steady-state failure is 404.
- Evidence/Claims considered: `DEC-XDOMAIN-004`.
- What would reopen: only a new accepted compatibility Decision.

### ALT-XDOMAIN-006 — Permit provisioning admin self-assignment as Owner

- Disposition: rejected.
- Reason: self-assignment turns control-plane authority into workflow-data authority and creates the indirect privilege path this closure must eliminate.
- Evidence/Claims considered: `CLM-XDOMAIN-006`, `DEC-XDOMAIN-006`.
- What would reopen: a separate Product Direction/Architecture decision defining a stronger control-plane trust model.

## 12. Migration, compatibility, and rollback

### 12.1 Frozen phases

```text
PHASE_0 = prepare implementation, denial tests, audit sink, and rollout rehearsal
PHASE_1 = deny admission to retired surfaces and drain/cancel in-flight responses
PHASE_2 = execute one-transaction disable + activation prohibition migration
PHASE_3 = deploy enforcement binaries everywhere; old binaries fail readiness
PHASE_4 = publish capability-free OpenAPI, HTTP bundle, SDK, tests, config, and docs
PHASE_5 = observe zero-success/zero-write evidence for 30 continuous days
PHASE_6 = retire old artifacts and physically drop tombstone no later than day 90
```

No phase may be reordered. Phase transition evidence is part of later COMPLIANCE.

### 12.2 Existing binding disposition

- Existing rows are not accepted role assignments.
- Every row is disabled in the containment transaction.
- The disable time and batch audit preserve historical provenance temporarily.
- Rows cannot be re-enabled.
- No product request may read them for authorization after containment.
- Table and rows are physically deleted under `CTR-XDOMAIN-MIGRATION-003`.

### 12.3 Migration 0020 disposition

`0020_global_role_bindings.sql` remains an immutable historical migration. The implementation creates new forward migration(s); it never rewrites `0020`. Fresh installs must end in the same deny-only then deleted state as upgraded installs.

### 12.4 API and SDK compatibility

```text
AUTHORIZATION_COMPATIBILITY_WINDOW = NONE
FEATURE_FLAG_THAT_RETURNS_DATA = FORBIDDEN
OLD_CLIENT_AFTER_ENFORCEMENT = 404 route_not_found on retired paths
CROSS_DOMAIN_OBJECT_DENIAL = 404 not-found-or-not-visible
STEADY_STATE_410 = FORBIDDEN
STEADY_STATE_501 = FORBIDDEN
SDK_COMPATIBILITY_SHIM = FORBIDDEN if it calls or models retired capability
```

Removing methods/types is intentionally breaking. Security containment takes precedence over old-client source or binary compatibility.

### 12.5 Rollback

Rollback may restore availability only within the deny-only boundary. It may not restore the role, bindings, routes, SDK behavior, or any global projection. A release without a safe deny-only rollback artifact must use roll-forward, shutdown, or isolation. Migration rollback must preserve the database activation prohibition.

### 12.6 Data cleanup and audit evidence

The implementation and production rollout must persist a conformance report containing migration identity, affected-row count and digest, before/after binding state, in-flight drain proof, deployed artifact identities, retired-route attempts, blocked writes, rollback rehearsal, 30-day observation, and final schema deletion. Raw production data and sensitive payload are not copied into the report.

## 13. Open questions

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
IMPLEMENTATION_AUTHORITY = contracts
INDEPENDENT_REVIEW_PENDING = YES
AUTHORIZED_ACCEPTANCE_PENDING = YES
CONFORMANCE_PENDING = YES
AUTHORING_READY_FOR_REVIEW = YES
```

Remaining work is procedural and implementation-bound:

1. independently review the exact proposed Spec commit;
2. obtain authorized acceptance of an exact final head and merge it to the authority branch;
3. create a separate product implementation task based on that accepted revision;
4. remove/hard-disable every unauthorized surface under these Contracts;
5. run Contract-by-Contract COMPLIANCE against the exact accepted Spec and exact implementation commit.

Until step 5 has sufficient Evidence, affected Contracts and aggregate conformance MUST be reported `DRIFTED` where an implemented conflict is directly established, otherwise `UNKNOWN`; they MUST NOT be reported `VERIFIED`.
