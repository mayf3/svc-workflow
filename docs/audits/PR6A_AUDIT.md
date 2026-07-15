# PR 6A Final Independent Audit

```text
Verdict: PASS
Blocker: 0
High: 0
Medium: 0
Low: 0
Audit Target: codex/legacy-initial-import-v0
Base: ac8cc80640701286ec6567b1529d8279887dc9a3
HEAD: 4a1146e738bf0de7836f8cfaa3a8182f54515e4c
Tree: 9bd4790c09c9e6bee16a8b9a5ec0e29c11b6bcad
```

## 1. Scope, independence, and readiness boundary

The final audit fixed the branch, base, HEAD, and tree above before reviewing
the final delta or running gates. The auditor modified no implementation, test,
contract, migration, or external repository and made no commit. This report is
the only audit write.

PR 6A remains deliberately limited to the atomic initial import of an already
frozen, mapped, and normalized ADC snapshot:

- `LOCAL_IMPORT_READY` is accepted;
- `SHADOW_NOT_READY` and `CUTOVER_NOT_READY` remain mandatory;
- Relay, Outbox, worker, high-water mark, comparator, reverse projection, ADC
  polling/writes, and automatic `workflowId` mapping remain outside this PR.

No migration was added or changed relative to the base.

## 2. Commit boundary and audit-fix exception

The base-to-HEAD history contains three functional commits:

1. `6a97c95edb8fba7ebee9173e043756f2dcedaf17` — initial primitive;
2. `26e88fec69ec15aebad870fe9ead567a0d2e2470` — closes the first audit's
   H1/H2/M1/L1 findings;
3. `4a1146e738bf0de7836f8cfaa3a8182f54515e4c` — closes a new lifecycle replay
   High found while independently auditing commit 2.

This is an explicit exception to the normal maximum of two functional commits.
The second commit had already been fixed and audited at tree
`2447d6610477102497fc34857087d8ae37e1d913`; it was not amended, rebased, or
otherwise rewritten. Appending the narrowly scoped third commit was the only
way to close the audit finding while preserving the immutable audited SHA.
The third commit changes only:

- `src/store/postgres/legacy_import_repository/replay.rs`;
- `tests/17_workflow_runtime/legacy_import/idempotency.rs`.

The exception is process-visible and does not weaken any functional, security,
database, or test gate.

## 3. Finding closure

### Initial High 1 — migration authorization predicate race: CLOSED

Authorization now locks the target Domain `FOR UPDATE`, then locks every
existing binding in that Domain in deterministic binding-ID order before
filtering, then locks the actor and all referenced principals in deterministic
principal-ID order. This closes all parts of the predicate without a migration:

- a new binding's FK key-share check waits on the locked parent Domain;
- enabling a disabled migration binding waits on the locked binding row;
- retagging an enabled non-migration binding waits on the locked binding row;
- actor and referenced principal enable/type changes wait on principal locks.

`import_locks_the_complete_migration_authorization_predicate` runs a real import
in one PostgreSQL session, proves it has passed authorization and is waiting on
a separately locked Definition Version, then attempts insert, enable, and
role-key retag mutations from other transactions. All three receive PostgreSQL
statement timeout SQLSTATE `57014`; after the Definition lock is released, the
import commits and exactly one enabled migration binding remains.

### Initial High 2 — imported event not anchored to its Receipt: CLOSED

Strict projection rebuild now loads event `command_id`, loads the referenced
Receipt, and requires a completed HTTP-200
`IMPORT_LEGACY_WORKFLOW_INSTANCE` Receipt. Receipt principal, fixed key, command
type, response digest, response IDs, initial event actor, external reference,
snapshot digest, and creator resolution must agree with the immutable instance,
context, visit, and event facts.

Negative rebuild coverage rejects null command ID, wrong command type, wrong
Receipt principal, wrong event actor, failed Receipt, response/body ID mismatch,
snapshot-digest mismatch, creator-resolution mismatch, unknown response field,
and stored response-digest corruption.

### Initial Medium 1 — Receipt replay integrity/correlation: CLOSED

Replay recomputes and length-stably compares `response_digest`, requires an
exact success schema, validates fixed version/sequence/digest fields, and binds
the stored response to the Receipt-referenced instance, context revision 1,
node visit 1, event 1, actor, command, event matrix, event-data digest, expected
snapshot digest, and fixed external reference. Failure replay has exact
status/label/field-count validation, including exact digest-mismatch bodies.
Corrupted digest, unrelated fact ID, and unknown success-field tests fail closed
with `InternalConsistency`.

### Initial Low 1 — request-hash golden: CLOSED

The request-hash test freezes both the canonical camelCase JCS envelope and its
SHA-256 value. It covers the actor, legacy record, all route IDs, the complete
snapshot, expected digest, creator, URL, metadata, and explicit `null` Option
semantics, while excluding the derived idempotency key/external reference.

### Audit of `26e88fec` — post-import lifecycle replay regression: CLOSED

The independent second review found that commit `26e88fec` correctly validated
the initial facts but also required the instance's *current* projection and
global fact counts to remain at the initial `1/1/1/0` state. A valid revise or
transition would therefore cause the original exact import retry to return an
internal-consistency error instead of its stored response. This was classified
High and the SHA was not accepted for merge.

Commit `4a1146e` removes only those current-projection and total-count
requirements. Replay still requires the response-referenced context revision
1, node visit 1, event sequence 1, null initial transition/submission/source
fields, target/context links, `0 -> 1` state matrix, event type/schema,
command/actor, exact event-data shape/digest, external reference, and expected
snapshot digest. Legitimate later facts and a later current projection are now
allowed.

`exact_replay_survives_valid_post_import_lifecycle_changes` proves the complete
sequence: import a Draft instance, revise context to state version 2, execute a
valid transition to version 3 and a different current visit, then replay the
original import request. Replay returns the original command/instance/context/
visit/event IDs and the original `1/1` import result with `replayed=true`.

## 4. Functional, security, and atomicity review

- Missing actors are rejected before Receipt creation; the transaction repeats
  actor, Domain, binding, Definition, and Node authorization under locks.
- Only an enabled `SERVICE` with the unique enabled
  `WORKFLOW_MIGRATION` binding can import or replay.
- Snapshot schema, record/domain/node relationships, lowercase JCS digest,
  RFC3339 timestamp, payload bounds, recursive `roleUserMap`, pseudo-state,
  context schema, creator, and assignee rules fail closed.
- The server-derived key/reference is exactly
  `migration:adc:<lowercase UUID>:v1`; conflict, processing, exact replay,
  current reauthorization, global reference collision, and same-request
  concurrency behavior are covered.
- A success atomically writes instance, context revision 1, visit 1, event 1,
  completed Receipt, and Security Audit, with no Submission. All seven injected
  write-stage failures roll back Receipt and runtime facts.
- Imported Draft, Normal, and Terminal histories pass strict rebuild. Event data
  has exactly six keys, server whole-second UTC time, exact identity values, and
  a verified digest. Terminal visits are unassigned.
- Security and attempt audits contain labels, IDs, and digests only; imported
  snapshot/context/workflow payloads, metadata, URL, and detailed reason text
  are not copied into those audit channels.

No additional Blocker, High, Medium, or Low finding remains at the final tree.

## 5. Commands and observed results

### Git and diff

- Branch, HEAD, and tree matched the fixed target.
- `26e88fec` still resolves to audited tree
  `2447d6610477102497fc34857087d8ae37e1d913`.
- `git diff --check base...HEAD`: pass.
- Base-to-HEAD migration diff: empty; repository has exactly migrations
  `0001` through `0010`.
- Before this report update, the only worktree entry was the untracked audit
  directory containing this report.

### Rust gates

- `cargo fmt --check`: pass.
- `cargo build --locked`: pass.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: pass.
- New lifecycle replay regression: 1/1 pass.
- Targeted PR 6A serial: 31/31 pass.
- Full serial: 431/431 pass:
  - library unit: 56;
  - integration: 375;
  - runtime integration: 257;
  - PR 6A subset: 31.
- Full default-parallel suite, five consecutive runs: 431/431 each. Runtime
  portions completed in 5.74s, 5.65s, 5.37s, 6.38s, and 5.30s.
- `cargo test --locked -- --list`: 431 tests, including 31 PR 6A tests.

### PostgreSQL and migrations

- Server and client: PostgreSQL 16.14 (Homebrew).
- SQLx tracking DB: migration count 10, maximum version 10, failed rows 0.
- Fresh DB applying `0001` through `0010`: pass; 15 public base tables;
  Terminal visit assignee column nullable; one assignee enforcement trigger.
- Fresh legacy DB applying `0001` through `0009`, then `0010`: pass;
  assignee nullability changed `NO -> YES`; trigger count 1.
- No migration-only partial binding index is present.
- Residual injected fault triggers: 0; residual injected fault functions: 0;
  audit temporary databases: 0.

### Structure and documentation

- Maximum handwritten Rust file: 493 lines (`<=500`).
- Maximum direct directory entries: 20 (`<=20`).
- Maximum source/test file depth: 4 (`<=4`).
- Top-level test entries: 20 (`<=20`).
- Current Markdown before this audit report: 13 files (README plus current
  `docs/` material). PR 6A adds one current contract. This report should be
  committed as audit evidence and then removed from the current main tree under
  the repository's document policy; Git history remains the audit archive.

## 6. External read-only status

No external repository was written.

- During the long-running audit, the shared ADC checkout was moved by other
  work from the earlier detached `d55f9fa` evidence point to branch `develop`,
  HEAD `343afa49475e6504b61e0b6510bfae372c65027f`, with a pre-existing untracked
  `.zcode/`. At the final read-only check, startup still calls
  `ensureWorkflowTemplates()` and that function still updates catalog steps;
  many routes still write `currentStep` directly; no Relay/Outbox/high-water/
  comparator/reverse-projection implementation was found. The current schema
  also exposes no unique Domain Owner contract that would close the migration
  owner-selection requirement. It is not evidence for Shadow or Cutover
  readiness.
- `llm-todo` remained at
  `7cc746240ba15161a5350bbe4c6d8fb88f41f5c6` with its pre-existing deleted
  `data/llm-todo.db-shm` and `data/llm-todo.db-wal` entries.
- `auth-service` remained at
  `8ca5fcb48a40bbb4d6909d0499372959d26d0440` with its pre-existing modified and
  untracked files.

## 7. Merge decision

PR 6A is approved for fast-forward merge at
`4a1146e738bf0de7836f8cfaa3a8182f54515e4c` / tree
`9bd4790c09c9e6bee16a8b9a5ec0e29c11b6bcad`.

The accepted claim is strictly `LOCAL_IMPORT_READY`. This audit does not approve
Shadow or Cutover, and the known external write-path, template, Relay, and owner
normalization blockers remain outside PR 6A.
