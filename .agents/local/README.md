# Repository-local governance for svc-workflow

This file is owned by `mayf3/svc-workflow`. It is not part of the vendored distribution and MUST NOT be overwritten by a governance update.

## Repository identity

```text
REPOSITORY = mayf3/svc-workflow
AUTHORITY_BRANCH = main
GOVERNANCE_LOCK = .agents/governance.lock.json
GOVERNANCE_ADOPTION_EFFECTIVE = only when adoption.status=accepted and that exact snapshot is merged into main
```

A proposed or accepted-looking lock on an unmerged branch is not active repository authority.

## Authority precedence

The repository uses the following precedence for the domain each authority actually owns:

```text
SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
  path: docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V4.md
  kind: product_direction
  status: accepted

superseded history:
SVC_WORKFLOW_PRODUCT_BOUNDARY_V3
  path: docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V3.md
  kind: product_direction
  status: superseded
  superseded_by: SVC_WORKFLOW_PRODUCT_BOUNDARY_V4

SVC_WORKFLOW_PRODUCT_BOUNDARY_V2
  path: docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V2.md
  kind: product_direction
  status: superseded
  superseded_by: SVC_WORKFLOW_PRODUCT_BOUNDARY_V3

SVC_WORKFLOW_PRODUCT_BOUNDARY_V1
  path: PRODUCT-BOUNDARY.md
  kind: product_direction
  status: superseded
  superseded_by: SVC_WORKFLOW_PRODUCT_BOUNDARY_V2

> SVC_WORKFLOW_ARCHITECTURE_V0_3_1
  path: docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md
  kind: architecture

> SVC_WORKFLOW_CANCEL_ARCHIVE_GOVERNANCE_V0_3_2
  path: docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_2.md
  kind: architecture refinement for Cancel / Archive only

> other explicitly accepted, frozen, or effective local Architecture and long-lived invariant authorities in their declared scopes

> accepted governing Specs under docs/specs/ that explicitly name their parent authorities

> code, tests, migrations, schemas, configuration, deployment state, runtime observations, and operational records
```

The governance adoption Spec is a top-level authority only for repository development process. It does not outrank, replace, or redefine Product Direction or product Architecture.

Rules:

- scope is discovery metadata, not an implicit precedence algorithm;
- a newer file does not automatically win;
- a more specific file does not silently override a parent authority;
- code, tests, runtime, and operational records are descriptive and may conform or drift;
- conflicting accepted authorities block implementation until reconciled explicitly;
- external repositories may be referenced at exact revisions but cannot be governed or superseded here.

## Existing-authority bridge

This adoption is forward-only.

```text
NO_BULK_HISTORY_REWRITE = YES
NO_AUTOMATIC_RECLASSIFICATION_OF_LEGACY_DOCS = YES
```

Existing files under `docs/architecture/`, `docs/contracts/`, `contracts/`, and related historical locations retain only the authority and status already established by their own text and review history at adoption base `8cda3d05e1c22814b7aeaace97d317380df83836`.

The adoption does not make every legacy document accepted, does not demote a previously frozen/effective authority, and does not treat directory location alone as authority. When future work depends on a legacy document, PREFLIGHT MUST identify its exact path, revision, declared status, owning scope, and relationship to the new governing Spec.

New governing Specs use `docs/specs/<SPEC_ID>.md` and the vendored Spec format.

## Acceptance and exception actors

```text
SPEC_ACCEPTANCE_ACTORS = repository owner mayf3, or an explicitly authorized svc-workflow maintainer
GOVERNANCE_ADOPTION_ACCEPTANCE_ACTOR = repository owner mayf3
MECHANICAL_EXEMPTION_REVIEWERS = a reviewer independent from the change author, with final acceptance by an authorized maintainer
EMERGENCY_AUTHORIZATION_ACTORS = repository owner mayf3, or an explicitly designated incident commander / maintainer
```

Authoring, independent semantic review, and acceptance are distinct acts. A review recommendation does not perform acceptance. The exact final accepted head requires an independent final-head recheck.

## Governing and persistence locations

```text
PRODUCT_DIRECTION = docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V4.md
PRIMARY_ARCHITECTURE = docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md
ARCHITECTURE_REFINEMENTS = docs/architecture/
LEGACY_IMPLEMENTATION_CONTRACTS = docs/contracts/
EXTERNAL_HTTP_CONTRACT_BUNDLE = contracts/workflow-http/
GOVERNING_SPECS = docs/specs/
INVESTIGATIONS = GitHub Issues; use docs/investigations/ when a repository file is required
CONFORMANCE_REPORTS = implementation PR records; use docs/reports/ when evidence spans environments, migrations, restarts, canaries, or external services
REVIEW_RECORDS = persistent PR review or a repository report linked from the PR
```

Durable rejected, no-change, reuse, or deferred findings MUST NOT exist only in chat. Persist them as an Investigation Record, issue, or investigation PR.

## svc-workflow change classification

The following are non-mechanical by default and require PREFLIGHT:

- Rust behavior, module boundaries, domain semantics, or public/internal error behavior;
- PostgreSQL migrations, constraints, triggers, indexes, transaction or lock order;
- identity, authentication, authorization, Domain isolation, token, scope, or trust semantics;
- workflow Definition, Instance, Visit, Submission, Event, Assistance, cancellation, archive, recovery, replay, or idempotency semantics;
- OpenAPI, HTTP contract bundle, SDK behavior, wire compatibility, or generated contract artifacts;
- timeout, retry, unknown-outcome, readiness, rollout, release, rollback, or deployment behavior;
- tests whose expected results define or change system behavior;
- repository process, required verification, or CI policy.

Dependency upgrades, deletion of apparently unused behavior, and unproven “refactor-only” changes are not mechanical by default.

## Emergency boundary

Emergency action before a normal Spec cycle is limited to:

```text
rollback
shutdown or disablement
containment
credential revocation
isolation
```

It requires an incident reference, authorized approval, and `DURABLE_NEW_BEHAVIOR = NO`. Permanent repair follows the normal Spec-first process.

## Enforcement truth

For this bootstrap candidate:

```text
ENFORCEMENT_LEVEL = MANUAL_POLICY after accepted adoption is merged
DISTRIBUTION_INTEGRITY_CHECK = AVAILABLE via .agents/tools/verify_governance.py
SPEC_FRONTMATTER_SCHEMA = AVAILABLE
FULL_SPEC_SYNTAX_GATE = NOT_IMPLEMENTED
BASE_BRANCH_MERGE_GATE = NOT_IMPLEMENTED
SEMANTIC_REVIEW = independent human/Agent judgment, not CI
BRANCH_PROTECTION_REQUIRED_CHECK = NOT_CONFIGURED
PRODUCT_CONFORMANCE_AUTOMATION = NOT_IMPLEMENTED
```

Do not claim an unbypassable gate until the check, required status, and branch protection are actually active.

## Governance updates and rollback

An upstream change has no effect until a separate docs-only update PR pins a new exact source commit, updates the lock and vendored bytes, receives independent review and local acceptance, and is merged.

Rollback is a Git revert of the complete adoption or update commit. Do not hand-edit vendored files to approximate another revision.
