# Workflow authoring diagnostics implementation record

## DEVELOPMENT_PREFLIGHT

- Repository: mayf3/svc-workflow; isolated branch codex/workflow-authoring-diagnostics-impl-v1.
- Base: 39279e34ff3124f8ca6f4269f9dab737a28aed75 (github/main).
- Change: NON_MECHANICAL; authority handling REUSE.
- Governance adoption: accepted, byte integrity checked during preceding authority work.
- Primary: SVC_WORKFLOW_DEFINITION_GRAPH_DIAGNOSTICS_V1, accepted in base,
  implementation_authority=contracts; reviewed semantic head 78323394c6c6d82a14657bdfd6589419fdbb6dff.
- Related: SVC_WORKFLOW_PRODUCT_BOUNDARY_V6; SVC_WORKFLOW_ARCHITECTURE_V0_4_0;
  SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1. No parent changes.
- SPEC_PRESENT_IN_BASE=YES; REQUEST_WITHIN_CONTRACT_SCOPE=YES;
  AUTHORITY_CONFLICT=NONE; IMPLEMENTATION_ALLOWED=YES.
- Scope: typed diagnostic projection, governance HTTP and safe receipt replay,
  affected contract artifacts and discriminating local tests. Canonical predicates unchanged.
- Production mutation: NONE. Independent audit belongs to the primary Goal agent.

## Supplemental DEVELOPMENT_PREFLIGHT: model 3 detail read conformance

- Before this one-line repair, composed E2E author reported actual model 3 instance
  create succeeded, then detail read returned 503; SQLx reported activation_kind
  PostgreSQL enum incompatible with Option<String>.
- Governing accepted authority: SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1 §3,
  blob 5f7a7ee9a6f9fbb1f67f01583776b12bd09902d6 at implementation base; accepted
  implementation_authority=contracts. SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
  CTR-VAI-013 preserves working details. This is not graph diagnostic authority.
- REUSE / conformance repair; requested source closure exactly one SQL projection
  cast a_open.activation_kind::TEXT in query_visibility.rs load_base. No eligibility
  predicate, graph rule, identity or authorization changes. IMPLEMENTATION_ALLOWED=YES.
- Production mutation NONE. Final composed regression evidence is recorded by its
  independent harness author under the same parent Goal.

## Implemented closure and qualified local evidence (2026-09-06)

The code in this implementation commit projects only canonical GraphValidationFailed
through a closed static catalog, preserves exact codes in details, and exposes the
first encoded rule in the top-level message. Canonical validator/application graph
predicates have zero diff. Corrupt typed receipts receive opaque 500 without details.

Receipt compatibility is deliberately narrow: the original version-only replacement
request hash remains unchanged. Only NEW completed graph diagnostic receipts store
an additional full-input hash, checked on diagnostic replay. Thus corrected input
with the same failed-attempt key conflicts while historical success/error replay
remains unchanged. Existing successful-write payload binding is FOLLOW_UP_DEBT;
this change does not redesign it or claim it was repaired. Internal input hashes
are not returned in HTTP diagnostics. Unknown historical PROCESSING attempts are
not resumed or converted.

Executed in the isolated worktree against disposable PostgreSQL at loopback 55449,
DB svc_diagnostics, principal postgres. `current_database/current_user` readback
returned `svc_diagnostics|postgres` before tests. This cluster was created specifically
for this Goal under `/tmp/svc-diagnostics-pg-20260906`; no production database used.

| Observation | Command / provenance | Actual result |
|---|---|---|
| OBS-IMPL-GD-001 | `TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55449/svc_diagnostics cargo test --lib --test 19_domain_owner_definition_governance --test 29_model3_authoring_conformance --test 30_definition_graph_diagnostics -- --test-threads=1`; evidence/authoring-diagnostics-v1/focused.log | exit 0; 175 library, 25 governance, 1 model3 authoring, 1 TCP integration passed |
| OBS-IMPL-GD-002 | After final receipt decode ordering/status hardening: `cargo test --lib definition_governance` and same disposable DB `cargo test --test 30_definition_graph_diagnostics`; evidence/authoring-diagnostics-v1/receipt-final.log | exit 0; 4 focused library + 1 TCP integration passed |
| OBS-IMPL-GD-003 | `cargo build --bin svc-workflow -q` | exit 0; binary SHA256 5603e142445e23060c9fc9c7dfec24f5dfeb2c5555c4d00df72a22b2bb77e75a |
| OBS-IMPL-GD-004 | `bash contracts/workflow-http/v1/verify-digests.sh` | exit 0; schema 1628c204b083d30771d9d7b2badc6e1b89e09b7c89a16239227401718bd1328c; bundle ece23f5cf78cdb1d17a64b621024b79fd623f7787ba82ed99bbda96963b69332 |
| OBS-IMPL-GD-005 | Python PyYAML parse and recursive resolution of every local OpenAPI `$ref`; `git diff --check`; accepted governance byte verifier | all exit 0 |

EVD-IMPL-GD-001: OBS-IMPL-GD-001/002 SATISFIES CTR-GD-001..004 for this tested local
source snapshot and disposable DB, including 14 canonical negative graph cases
across model1/model3, empty-draft publication failure and replay, historical format
and historical success-hash replay, changed graph conflict, corrupt receipt opacity,
unchanged stored graph/version readbacks, valid publication and visibility denial.
The static catalog tests enumerate every accepted catalog rule, require exact code,
static bounded ASCII messages, sanitizer-safe top-level text, and test overflow,
unknown/empty/duplicate/reordered vectors. This is local evidence, not production.

EVD-IMPL-GD-002: OBS-IMPL-GD-003/004/005 SATISFIES the local build/contract part of
CTR-GD-005 at this source snapshot; production portion remains INCONCLUSIVE until
separate serialized deployment and normal Agent E2E evidence under the owning Goal.
The historical broad shell conformance runner hardcodes PostgreSQL 5432 and is not
run against that shared default. New real TCP tests use the actual router, RS256/
JWKS authentication, canonical validators and disposable PostgreSQL; the composed
Broker E2E is separately owned by the parent Goal.

Observed limitation preserved: nonexistent fixed-principal UUID insertion hits the
existing DB foreign key and returns service_unavailable/503; a disabled existing
principal reaches canonical publication validation and returns opaque404. Both
remain fail-closed. No validator or existing error mapping is rewritten to hide
this baseline behavior. Initial test expected the nonexistent UUID to survive
replacement; actual source/DB evidence corrected that test assumption.

The one-line model3 detail-read cast has separate accepted eligibility authority
above. Composed E2E supplies its post-fix runtime regression; this report does not
claim a self-run model3 detail regression before that evidence arrives.

Independent audit = pending parent orchestration. Production conformance = UNKNOWN.
