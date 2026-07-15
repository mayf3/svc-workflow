# PR 5 Independent Final Audit

```text
Verdict: PASS_WITH_NOTES
Audit mode: independent, read-only source review plus fresh execution
Architecture: svc-workflow v0.3.1 (frozen)
Slice: PR 5 — Admin Emergency Recovery and Projection Rebuild
Branch: codex/admin-emergency-repair-v0
Base: d2f5636ca4ac6484850d8b3d2766796d7d46469f
Audited HEAD: 78df30af95f4e4201c81b8e66086c18ab88c5e82
Audited tree: 96c588ad5b0f315807d2df9d89bc46a8a37a86ce
Audit date: 2026-07-15 (Asia/Shanghai)
```

## 1. Result

The repaired PR 5 head is fit for fast-forward integration into `main`.

```text
Blocker: 0
High:    0
Medium:  0
Low:     2
```

The first audit's four High findings are closed. The implementation now derives the projection by strict event-by-event replay, requires a matching replay before emergency override, reauthorizes every Receipt outcome before exposing it, and redacts sensitive admin EventData from restricted timeline readers. Migration 0010 also enforces both terminal-assignee shape and Visit/Instance definition-version alignment for new rows while preserving legacy rows.

The two Low notes are contract-hardening follow-ups, not correctness, authorization, data-loss, transactionality, or migration blockers for PR 5.

## 2. Audited change set

The fixed range is:

```text
d2f5636ca4ac6484850d8b3d2766796d7d46469f
  ..78df30af95f4e4201c81b8e66086c18ab88c5e82
```

It contains the implementation commit `c39ff88` and audit-fix commit `78df30a`. The range changes 63 files, adding the Admin Recovery contract, application/domain types, PostgreSQL transactions and replay state machine, migration 0010, query redaction, and PR 5 tests.

Migration review established:

```text
changed migration files: migrations/0010_terminal_assignee_nullable.sql only
0001..0009 byte content compared with base: unchanged
```

The audit did not modify source, tests, contracts, migrations, or Git history. This report is the sole audit artifact and is intentionally left untracked for a separate evidence commit.

## 3. First-audit findings and closure

### H1 — Override did not prove projection/fact consistency: CLOSED

`ADMIN_EMERGENCY_OVERRIDE` now:

1. acquires Receipt, Instance, and DefinitionVersion locks;
2. replays all immutable Context/Visit/Submission/Event facts;
3. compares all three replayed projection fields with the locked Instance projection;
4. completes a deterministic failure Receipt if any field differs;
5. creates no Visit/Event until that equality is proven.

Relevant implementation: `override_transaction.rs:240-258`, followed by target resolution and fact creation only at `:290-440`.

Independent regression evidence includes both a corrupted state-version projection and a stale current-Visit projection; both return `InvalidImmutableFacts`, preserve fact counts, and complete only the failure Receipt.

### H2 — Rebuild did not strictly replay Events or bind EventData/facts: CLOSED

The new replay engine is a per-Event state machine, not a latest-row heuristic. It maintains current Context, current Visit, state version, introduced fact sets, and per-node Visit counters.

It verifies:

- canonical creation and architecture creation alias;
- canonical context revision and architecture context alias;
- transition-only events;
- combined context+transition events;
- admin override events and their before-snapshot digest;
- the PR 6 imported initial shape;
- event sequence, old/new version, schema version, and unknown event rejection;
- exact source/target Visit, Context, Submission, Transition definition, node fields, transition key/effect, and EventData digest relationships;
- Context chain continuity and payload digests;
- Submission relationships and payload digests;
- every Context, Visit, and Submission is introduced exactly once, with no orphan fact;
- final projection comes from replay state, so a stale stored current pointer cannot be accepted.

Relevant implementation: `event_replay.rs:18-490`, `event_fields.rs:1-81`, and `snapshot.rs:42-282`.

The replay was exercised against creation, revise, transition, combined, admin, documented aliases, imported initialization, event gaps, old-context branching, field-matrix corruption, projection drift, and orphan facts.

### H3 — Receipt replay/conflict/PROCESSING skipped current authorization: CLOSED

Both commands now use the same ordering before handling any acquired Receipt variant:

```text
Receipt -> requested WorkflowInstance -> DefinitionVersion -> current actor/admin checks
```

Only after those checks do they return a stored success/failure, report an opaque hash conflict, or report PROCESSING. Revoking the actor or `WORKFLOW_ADMIN` binding therefore prevents an old response from being returned. Existing Receipt content remains unchanged; denial audits contain no old command ID, response digest, original request hash, or processing metadata.

`PrincipalDisabled` and `PrincipalTypeNotAllowed` remain stable for existing requested instances in all Receipt states. Missing and unauthorized instances remain indistinguishable as `PermissionDenied`.

Relevant implementation: `rebuild_transaction.rs:92-180`, `override_transaction.rs:147-231`, `authorization.rs:12-86`, and `receipt.rs:8-264`.

Independent tests cover completed success replay, completed failure replay, conflict, PROCESSING, binding revocation, disabled Principal, SERVICE Principal, cross-instance key conflict opacity, and original Receipt immutability.

### H4 — Restricted terminal timeline exposed admin reason/references: CLOSED

For a restricted historical participant, `ADMIN_EMERGENCY_OVERRIDE_COMMITTED` remains a terminal outcome skeleton but both `event_data` and `event_data_digest` are cleared before DTO construction. Full visibility still returns the complete event.

Relevant implementation: `query_detail.rs:238-247`.

The independent regression verifies that a restricted participant cannot observe the reason while a full-scope creator can.

## 4. First-audit Medium items and closure

All original Medium observations are closed:

- reason is validated on the original value, rejects surrounding whitespace/control characters, and is bounded to 1..2000 characters;
- related references are count- and byte-bounded;
- `workflowStateVersion + 1` and per-node `visitNumber + 1` use checked arithmetic;
- both rebuild and override lock DefinitionVersion after Instance;
- creator, fixed assignee, Domain Owner binding, resolved assignee, actor, and admin binding are read with row locks and enabled checks;
- migration 0010 checks Visit node/Instance definition-version equality and fires on relevant INSERT/UPDATE columns;
- PR 5 fault coverage includes Visit insert, Event insert, Instance projection update, Receipt completion, and SecurityAudit insertion, using UUID-scoped functions/triggers and RAII cleanup through a fresh connection.

## 5. Transaction and invariant assessment

### Projection rebuild

- `expectedWorkflowStateVersion` is intentionally absent.
- Optional before-snapshot digest is checked under the Instance lock.
- The result is derived from immutable facts and Events.
- Only the three projection columns plus `updated_at` can change.
- It creates no Context, Visit, Submission, or WorkflowEvent and does not increment state version.
- Projection update, completed Receipt, and SecurityAudit commit atomically.

### Emergency override

- It is restricted to enabled HUMAN/AGENT Principals with an enabled Domain-scoped `WORKFLOW_ADMIN` binding; Domain Owner, creator, assignee, and SERVICE do not inherit this authority.
- DRAFT DefinitionVersion is rejected; PUBLISHED, DEPRECATED, and REVOKED are supported.
- Full immutable replay and exact projection equality precede runtime fact writes.
- `MOVE_TO_NODE` requires a non-terminal target and resolves an enabled normal assignee.
- `TERMINATE_INSTANCE` requires a terminal target and writes a null assignee.
- A successful command creates one new Visit, updates only the current Visit/state projection, and creates exactly one admin Event with contiguous state/event version.
- Visit, projection, Event, Receipt, and SecurityAudit are in one PostgreSQL transaction.
- Injected infrastructure failures roll back all PR 5 effects, including a first PROCESSING Receipt.

### Request hash and idempotency

- Both request envelopes use JCS plus SHA-256 and cover actor, instance, schema version, operation-specific fields, optional digest, reason, and related references.
- `(principal_id, idempotency_key)` remains the database uniqueness boundary.
- Same hash + COMPLETED replays without a second mutation.
- Different hash is opaque and does not mutate the original Receipt.
- PROCESSING is not taken over.
- Concurrent same-key and different-key tests establish one mutation and Instance serialization.

## 6. Migration 0010 assessment

Fresh-database and legacy-upgrade checks were run against PostgreSQL 16.14.

Fresh `0001..0010` result:

```text
both assignee columns nullable: yes (2/2)
chk_node_assignee_shape installed NOT VALID: yes
trg_node_visit_assignee installed exactly once: yes
```

Legacy `0001..0009 -> 0010` result:

```text
legacy terminal Definition/Visit with non-null assignee preserved: yes
new terminal Visit with non-null assignee rejected: yes
new terminal Visit with null assignee accepted: yes
cross-definition-version Visit rejected: yes
```

The `NOT VALID` definition constraint preserves historical published rows but enforces the canonical shape on every new or updated row. The Visit trigger additionally checks target node existence, instance existence, fixed DefinitionVersion equality, terminal null assignee, and non-terminal non-null assignee.

## 7. Commands executed from zero

All commands were run by the audit agent at the fixed HEAD/tree.

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo build --all-targets` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --test 17_workflow_runtime admin_recovery -- --test-threads=1` | PASS — 45/45 |
| `cargo test --test 17_workflow_runtime -- --test-threads=1` | PASS — 226/226 |
| `cargo test -- --test-threads=1` | PASS — 398/398 |
| `cargo test -- --list` count | PASS — 398 |
| default `cargo test --quiet`, consecutive run 1 | PASS — 398/398 |
| default `cargo test --quiet`, consecutive run 2 | PASS — 398/398 |
| default `cargo test --quiet`, consecutive run 3 | PASS — 398/398 |
| default `cargo test --quiet`, consecutive run 4 | PASS — 398/398 |
| default `cargo test --quiet`, consecutive run 5 | PASS — 398/398 |
| `git diff --check base..HEAD` | PASS |
| PostgreSQL server version | 16.14 (Homebrew) |
| empty DB migrations `0001..0010` | PASS |
| legacy DB migrations `0001..0009`, legacy seed, then `0010` | PASS |
| residual test triggers before/after | 0 / 0 |
| residual test functions before/after | 0 / 0 |
| residual audit temporary databases | 0 |

Test inventory:

```text
library tests:     54
integration tests: 344
total:             398
```

## 8. Structure and repository hygiene

```text
largest handwritten Rust file: 493 lines
new replay file:               490 lines
new override transaction:      476 lines
maximum direct children:       20
maximum source/test directory nesting below src|tests: 3
tracked Markdown at audited HEAD: 12
```

All structure gates pass:

- handwritten Rust file <= 500 physical lines;
- direct directory children <= 20;
- directory depth <= 4;
- `tests/` top level remains at 20 direct entries;
- no obsolete audit snapshot is tracked in this audited tree;
- the one added current document is the PR 5 contract linked from README.

Before creating this report, `git status --short --branch` showed only the branch header and no worktree changes.

## 9. Remaining Low notes

### L1 — Clarify actor-status precedence for a nonexistent requested instance

Section 1 of the PR 5 contract says disabled/type checks occur before reading the Instance, while Section 2 and the implementation use the fixed Receipt -> Instance -> DefinitionVersion order before current actor/admin checks. Consequently, a disabled or SERVICE Principal receives its stable typed error for an existing requested instance, but a nonexistent requested instance is normalized to `PermissionDenied` first.

This is fail-closed and preserves instance non-enumeration; it does not bypass authorization or reveal Receipt metadata. Before exposing an HTTP endpoint, the contract should state which precedence is canonical so client error semantics are unambiguous.

### L2 — Freeze stronger PR 6 imported EventData value constraints with the importer

The PR 5 reader strictly requires the six imported EventData keys, a UUID `importedNodeId`, and non-empty string values for the other five fields. The still-unimplemented PR 6 writer contract has not yet frozen stronger formats for `legacySnapshotDigest`, `importedAt`, or the complete `creatorResolution` vocabulary.

This does not affect PR 5 projection correctness: instance/definition/Context/Visit ownership, fact digests, revision/Visit numbering, Event sequence, and projection state are still validated. PR 6 should freeze and test the exact digest, timestamp, and resolution formats, then tighten the reader in the same slice if required.

## 10. Final verdict

```text
PASS_WITH_NOTES
Blocker = 0
High    = 0
Medium  = 0
Low     = 2
```

PR 5 may proceed to a separate audit-report commit and `git merge --ff-only` into `main`. No push, tag, or branch deletion was performed by this audit.
