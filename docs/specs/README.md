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
| `SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V1` | superseded → `SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2` | implementation | contracts | frozen identity-only source and active-work reconciliation | none |
| `SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2` | accepted | implementation | contracts | frozen identity-only source and active-work reconciliation | `SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V1` |
| `SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1` | accepted | implementation | contracts | GLOBAL_WORKFLOW_COORDINATOR governance control plane (server-side role gates only) | none |
| `SVC_WORKFLOW_DEFINITION_GRAPH_DIAGNOSTICS_V1` | accepted | implementation | contracts | definition governance graph validation diagnostics | none |
| `SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1` | accepted | invariant | none | repository development process | none |
| `SVC_WORKFLOW_DISPATCH_INTENT_KEYSET_CONTINUATION_V1` | accepted | implementation | contracts | due Dispatch Intent read (GET /internal/v1/dispatch-intents); keyset continuation only | none |
| `SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1` | accepted |  | contracts | Global Workflow Reader role (dual grant: HR main + dedicated dispatcher) | none |
| `SVC_WORKFLOW_INVALID_RETURN_REFERENCES_HTTP_422_V1` | accepted | implementation | contracts | workflow transition execute API invalid-return-references error surface | none |
| `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` | accepted | implementation | contracts | one-time Principal successor migration | none |
| `SVC_WORKFLOW_TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1` | proposed | implementation | none | trusted-fleet principal cutover | none |
| `SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1` | accepted | implementation | contracts | VISIT_ACTIVATION_V1 runtime core (canonical activation facts, DISPATCH_INTENT, wake, due-intent read) | none |
| `SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1` | accepted | implementation | contracts | canonical work eligibility read projection (reads VISIT_ACTIVATION facts; consumed under the READER authorization) | none |
| `SVC_WORKFLOW_WORK_EXECUTION_CLASS_V1` | accepted | implementation | contracts | workflow_instances execution-class fact, create marking, due-feed predicate, domain/global summary projection | none |

Update this index when a governing Spec is accepted or superseded. The table is a discovery aid; each Spec's exact frontmatter and revision remain authoritative.

Regenerated 2026-09-13 from current main `cc006d9` (13 spec files = 11 accepted + 1 proposed + 1 superseded), fields read mechanically from each Spec's frontmatter; blank cells are absent frontmatter fields, not omissions.
