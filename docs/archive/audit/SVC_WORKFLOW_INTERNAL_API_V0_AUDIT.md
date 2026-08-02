# SVC_WORKFLOW_INTERNAL_API_V0 Independent Audit

```text
Status: PASS
Stage: SVC_WORKFLOW_API_SMOKE_READY
Audit date: 2026-07-16
Mode: independent read-only implementation review followed by a report-only commit
Repository: svc-workflow
Branch: codex/internal-api-v0
Base: 8eb9e16e715b26a2c8b77d6c2a44045fb7ddc44f
Initial implementation: 84bbcbd5286c7f45b50fe94b86df46450721dee8
Audited fix head: 4fc40c38cd63246746691a1cd24fb809b9d59fbe
```

## Verdict

The Internal API V0 implementation and its follow-up fix satisfy the frozen Stage 1 smoke
contract. No Blocker or High finding remains.

```text
Blocker: 0
High:    0
Medium:  1
Low:     0
```

The branch may be treated as `SVC_WORKFLOW_API_SMOKE_READY`. This verdict is limited to the
isolated service API smoke boundary. It does not claim ADC client, Shadow, or Cutover readiness.

## Audit scope

The review covered:

- Axum server startup, routing, request limits, timeout envelope, and graceful shutdown wiring;
- strict HS256 JWT verification, required claims and scopes, and `JWT.sub` actor injection;
- create, detail, transition, and timeline adapters;
- health, readiness, version, migration-ledger, and build-SHA responses;
- strict request DTOs, actor-field rejection, idempotency headers, pagination, and error redaction;
- query not-found/not-visible indistinguishability;
- request-hash compatibility and HTTP receipt replay status/body semantics;
- deterministic failure receipt boundaries and the missing-principal foreign-key exception;
- isolated PostgreSQL and real TCP end-to-end evidence;
- file size, directory child-count, and directory-depth limits;
- repository hygiene and exclusion of pre-existing untracked reports.

## Closed High findings

### H1 — Deterministic failure receipts and replay stability: CLOSED

Create and transition now compute the request hash, resolve the persisted principal identity, own
the receipt, and complete all deterministic business validation before writing the first runtime
fact. A deterministic failure completes and commits the receipt with its original status and
semantic detail. Exact replay returns that persisted result even if the underlying business state
later changes.

Covered validations include disabled principals, domain and membership gates, definition and
assignee gates, optimistic state versions, transition applicability, schema validation, return
references, and application payload size limits.

An unknown `JWT.sub` is the single pre-receipt identity-mapping exception: the principal must
exist before a receipt can be inserted because `workflow_command_receipts.principal_id` has a
principal foreign key. It returns `404 principal_not_found` and creates no receipt. A known
disabled principal receives a stable completed failure receipt.

Evidence:

- `src/application/workflow_instance/create.rs`
- `src/application/workflow_instance/execute_transition.rs`
- `src/store/postgres/workflow_instance_repository/create_transaction.rs`
- `src/store/postgres/workflow_instance_repository/transition_transaction.rs`
- `src/store/postgres/workflow_instance_repository/validation_helpers.rs`
- `src/store/postgres/workflow_instance_repository/transition_validation.rs`
- `tests/17_workflow_runtime/receipt_stability/`

### H2 — Storage and internal-consistency HTTP classification: CLOSED

Create, transition, and query `StorageError` variants now return the redacted retryable envelope:

```text
503 service_unavailable
```

`InternalConsistency` remains non-retryable at the HTTP classification boundary:

```text
500 internal_consistency_error
```

Raw storage and consistency details are logged server-side and are not returned to clients.

Evidence:

- `src/http/error.rs`
- `http::error::tests::storage_is_retryable_but_consistency_is_not`

### H3 — Real TCP and isolated PostgreSQL E2E evidence: CLOSED

The E2E test creates a random `svc_workflow_e2e_<uuid>` PostgreSQL database, applies the exact
migration set, binds a real `TcpListener` on `127.0.0.1:0`, and uses an HTTP client across the TCP
boundary. Setup and scenario bodies run in joined tasks so panics are observed only after server
shutdown, database cleanup, and a zero-residue assertion.

The scenario verifies:

- readiness against the exact migration ledger;
- create, detail, transition, timeline, success replay, and opaque idempotency conflict;
- HTTP-to-domain request-hash preservation for create and transition;
- deterministic 413 failure replay for metadata and submission limits;
- identical responses for query not-found and not-visible states;
- `externalReference` longer than 512 Unicode characters returns `422 invalid_input`;
- Tower request-body overflow returns the standard `413 size_limit_exceeded` envelope;
- a migration-ledger mismatch returns `503 migration_version_mismatch`;
- cleanup leaves zero `svc_workflow_e2e_%` databases.

Evidence:

- `tests/17_workflow_runtime/http/e2e/database.rs`
- `tests/17_workflow_runtime/http/e2e/server.rs`
- `tests/17_workflow_runtime/http/e2e/scenario.rs`

## Remaining Medium

### M1 — Graceful shutdown has no explicit process-level total deadline

`src/main.rs` uses Axum graceful shutdown and request-level timeouts, but it does not wrap the
whole graceful-drain future in a separate process-level deadline. A pathological connection could
therefore delay process exit longer than the intended operational shutdown budget.

This does not block the isolated Stage 1 smoke boundary. Add an explicit total drain deadline
before production deployment semantics are frozen.

## Verification evidence

All commands were run against `codex/internal-api-v0` at audited head
`4fc40c38cd63246746691a1cd24fb809b9d59fbe`.

```text
git diff --check 8eb9e16..4fc40c3                         PASS
cargo fmt --all -- --check                                PASS
cargo check --all-targets --all-features                  PASS
cargo clippy --all-targets --all-features -- -D warnings  PASS
targeted receipt-stability tests                           13/13 PASS
targeted real-TCP isolated-PostgreSQL E2E                   1/1 PASS
full serial test suite                                   456/456 PASS
temporary E2E database residue                                  0
```

The real-TCP scenario also proved request-hash preservation, successful HTTP replay status/body,
failure replay status/body, anti-probing query responses, external-reference validation, 413
envelopes, and readiness migration mismatch handling.

## Repository hygiene

All reviewed implementation files stay within the frozen physical limits:

```text
handwritten file lines <= 500
direct directory children <= 20
directory depth <= 4
```

The following two pre-existing untracked investigation reports were preserved and were not added
to this audit commit:

```text
ADC_SVC_WORKFLOW_INTEGRATION_READINESS_REPORT.md
SVC_WORKFLOW_INTERNAL_API_V0_CONTRACT_INVESTIGATION.md
```

No deployment, push, tag, secret change, production database operation, or branch merge was
performed by this audit.

```text
SVC_WORKFLOW_INTERNAL_API_V0_AUDIT_PASS
SVC_WORKFLOW_API_SMOKE_READY
```
