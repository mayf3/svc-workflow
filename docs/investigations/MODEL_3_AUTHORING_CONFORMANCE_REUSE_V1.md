# Model 3 authoring conformance repair — REUSE

```text
DEVELOPMENT_PREFLIGHT = PASS
SPEC_GOVERNANCE_MODE = PREFLIGHT
PREFLIGHT_MODE = REUSE
CHANGE_CLASS = NON_MECHANICAL (behavioral repair; no exemption needed)
OWNER_CLASSIFICATION = CASE_A_MECHANICAL_CONFORMANCE_FIX
GOVERNANCE_ADOPTION_STATUS = accepted
PRIMARY_GOVERNING_SPEC = SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
RELATED_ACCEPTED_AUTHORITIES = SVC_WORKFLOW_PRODUCT_BOUNDARY_V6; SVC_WORKFLOW_ARCHITECTURE_V0_4_0
GOVERNING_SPEC_REVISION = 22e862af8e47050ae1bf9e7c5db7eb22a4d81ee7
BASE_COMMIT = 22e862af8e47050ae1bf9e7c5db7eb22a4d81ee7
SPEC_PRESENT_IN_BASE = YES
SPEC_STATUS_IN_BASE = accepted
IMPLEMENTATION_AUTHORITY = contracts
AUTHORITY_CONFLICT = NONE
IMPLEMENTATION_ALLOWED = YES
NEXT_ACTION = restore canonical model 3 decode and existing HTTP admission; focused tests; one independent audit
```

CTR-VAI-001 explicitly defines persisted `3 = VISIT_ACTIVATION_V1` and preserves 1/2; CTR-VAI-011 requires draft-time and publish-time Visit Activation validation. Missing `TryFrom(3)` violates these accepted obligations: the repository currently falls back to Legacy when decoding stored 3. HTTP create-draft admission also omits the accepted value. Owner's CASE A ruling permits bounded correction of these omissions without changing any accepted meaning. This repair uses accepted implementation authority, not an exemption from behavioral review.

Only `model.rs` conversion/documentation, `definitions.rs` admission/documentation/error message, and focused tests are changed. No persisted data semantics, migrations, graph validators, publication lifecycle, runtime execution, Scheduler, Auth, Broker, or accepted authority change. Existing 20/20 runtime evidence is reused within its original scope; focused HTTP -> real repository -> graph replace -> publish evidence must close this authoring-specific gap.

Unknown integers remain rejected by conversion and HTTP; DB constraints continue rejecting model 4. Existing repository fallback for corrupted unknown database values is unchanged and outside this bounded repair; valid persisted 3 must now decode exactly to VisitActivation.

## Qualified implementation evidence

Source base and governing Spec revision: `22e862af8e47050ae1bf9e7c5db7eb22a4d81ee7`; governing Spec blob unchanged: `e1504d00ca587e4d193d5b5e46653cd0c54204b2`. Candidate is the commit containing this record.

- CTR-VAI-001: model conversion and serde roundtrip test covers 1/2/3 and unknown 0/4 rejection, omitted Legacy compatibility; HTTP test covers omitted/1/2 repository readback, 3 exact get/lock/published readback, unknown -1/0/4/32767 rejection without a new version, unchanged DB constraint rejection of 4.
- CTR-VAI-011: one focused test creates Definition and model 3 draft over authenticated HTTP, uses the actual PostgreSQL Definition repository, rejects a NORMAL graph with zero persisted nodes, accepts TASK→TERMINAL via existing draft route, publishes via existing HTTP route and reads model 3/PUBLISHED back. No Definition/Version/graph SQL seeding bypasses authoring. The only direct inserts are existing test principal/domain seeds and the deliberate model 4 DB constraint negative.
- CTR-VAI-012/013: production change excludes validators, transactions, migration, runtime execution, and lifecycle. Historical Phase 4 20/20 remains unchanged and is not rerun. This focused test is not production E2E and does not claim deploy readiness.

Commands use `CARGO_TARGET_DIR=/Users/yanfenma/workspace/deployment-artifacts/visit-activation-dispatch-v1/recovery-01a07001/model3-svc-target`:

```sh
TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/model3_conformance_01a07001 cargo test --locked --test 29_model3_authoring_conformance -- --nocapture
cargo test --locked --lib domain::definition::model::tests -- --nocapture
git diff --check
```

Environment: disposable local PostgreSQL at 127.0.0.1:55432, SQLx official migrations through 0023, runtime-generated RSA keys and local JWKS, in-process actual Axum router. Production untouched. Logs: deployment-artifacts `visit-activation-dispatch-v1/recovery-01a07001/model3-downstream-preflight/{AUTHORING_TEST,MODEL_TEST}.log`.

Initial test expected invalid graph HTTP 422 but existing error mapping produces 500 `internal_consistency_error` with `graph validation failed`. The final test asserts rejection plus zero persisted nodes, without changing the existing error mapping or expanding this repair. This is an evidence limit, not authority to alter error contracts.

Independent implementation audit and integration are pending; author does not self-accept or merge.

Executed result: formal HTTP/repository authoring 1/1 PASS; focused domain conversion/serde 2/2 PASS; `git diff --check` PASS. Scope verification: only the two named source files, this REUSE record, and one new focused integration test. `SPEC_COMPLIANCE = PASS` is the author's scoped self-check; independent verdict remains pending.
