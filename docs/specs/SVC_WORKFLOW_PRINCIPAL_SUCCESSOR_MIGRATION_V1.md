---
spec_id: SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1
status: accepted
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
scope:
  - mayf3/svc-workflow
  - one-time-principal-successor-migration
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V2
authority_chain:
  authority_id: SVC_WORKFLOW_PRODUCT_BOUNDARY_V2
  amendment_id: PBV2-ONE-TIME-SUCCESSOR-001
  accepted_main_revision: 6d4e117bfe8b41b82cf74d4e839125ffc4ee7261
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
title: One-Time Principal Successor Migration V1
repo: mayf3/svc-workflow
base_head: 6d4e117bfe8b41b82cf74d4e839125ffc4ee7261
source_spec_revision: 6f1f546787bd5fb1644ec91327d3e7374dc28165
source_spec_authoring_base: 8cda3d05e1c22814b7aeaace97d317380df83836
semantic_delta_from_source: AUTHORITY_ALIGNMENT_ONLY
migration_kind: ONE_TIME_SUCCESSOR
implementation_authority_activation: accepted_on_main
production_apply_authorized_now: false
merge_required_for_activation: true
---

# SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1

## 0. Authority alignment and provenance

This accepted child implementation Spec is governed by the active Product Direction chain present on `github/main`:

```text
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V2
BOUND_AMENDMENT = PBV2-ONE-TIME-SUCCESSOR-001
PARENT_ACCEPTED_MAIN_REVISION = 6d4e117bfe8b41b82cf74d4e839125ffc4ee7261
OWNER_DECISION = ALLOW_BOUNDED_ONE_TIME_SUCCESSOR_MIGRATION
AUTHORITY_COMPATIBILITY = PASS
```

The parent amendment authorizes only an upper-level, bounded exception for this exact one-time offline migration. It keeps ordinary reassignment, handoff, delegation, arbitrary Principal-pair migration, implementation, and production apply unauthorized unless and until their own required gates are satisfied.

Provenance for this aligned proposal:

```text
SOURCE_SPEC_REVISION = 6f1f546787bd5fb1644ec91327d3e7374dc28165
SOURCE_SPEC_AUTHORING_BASE = 8cda3d05e1c22814b7aeaace97d317380df83836
CURRENT_AUTHORING_BASE = 6d4e117bfe8b41b82cf74d4e839125ffc4ee7261
SEMANTIC_DELTA_FROM_6F1F546 = AUTHORITY_ALIGNMENT_ONLY
```

All migration behavior frozen by the source proposal is preserved. The reviewed-to-finalized change is lifecycle-only: `status: accepted` and the corresponding activation wording. Under repository lifecycle, the declared `implementation_authority: contracts` becomes active only when this exact accepted head is present on `main`. Acceptance and merge do not themselves perform implementation; production apply remains a separate later execution gate.

## 1. Decision summary

This Spec proposes one extremely narrow, offline successor migration for exactly:

```text
OLD_PRINCIPAL = 3e2439d2-fb54-44f5-afee-77aa17c40d22
NEW_PRINCIPAL = 4e5a4578-0645-4133-bd35-b80e453dfee9
```

It transfers only the OLD principal's responsibility that is current and effective at execution time, provided that responsibility still exactly matches the reviewed plan. It does not rewrite historical ownership or assignment provenance.

```text
ONE_TIME_OFFLINE_SUCCESSOR_MIGRATION = YES
GENERAL_PRINCIPAL_REASSIGNMENT_API = NO
HTTP_API_ADDED = NO
PRODUCT_CODE_CHANGED_BY_THIS_SPEC_PR = NO
DATABASE_SCHEMA_CHANGED = NO
PRODUCTION_APPLY_AUTHORIZED_BY_THIS_SPEC = NO
```

The frozen executable shape is:

```text
plan
--apply <reviewed-plan>
exact metadata rerun => NOOP
```

This independently reviewed and accepted Spec authorizes only the bounded implementation described by its Contracts once the exact accepted head is present on `main`. Acceptance and merge do not perform that implementation and do not authorize production execution. Production execution requires a separate explicit owner authorization over an exact implementation Git SHA and reviewed plan digest.

## 2. Scope and frozen production facts

Facts supplied for this Spec:

| Fact | Frozen count |
|---|---:|
| OLD enabled Domain bindings | 9 |
| NEW enabled Domain bindings | 0 |
| OLD active assigned worklist responsibility | 1 |
| NEW active assigned worklist responsibility | 0 |
| Instances ever assigned to OLD | 58 |
| OLD-assigned node visits in history | 111 |

The nine Domain authorities contain exactly one `DOMAIN_OWNER` binding and exactly eight `DOMAIN_MEMBER` bindings. V1 recognizes no other transferable role key. Any enabled OLD role outside this exact 1+8 shape is conflict; the tool must not normalize or invent role keys.

The counts above are review inputs, not permission to select rows by count alone. The plan output must enumerate the exact identities described in section 5 and be reviewed as an immutable canonical snapshot.

## 3. Normative repository findings

This proposal follows existing repository semantics rather than introducing a general service surface:

1. `domain_role_bindings` has unique `(domain_id, principal_id, role_key)` identity and a partial unique index allowing at most one enabled `DOMAIN_OWNER` per Domain (`migrations/0001_identity_domain.sql`).
2. Existing owner replacement disables the previous enabled owner and enables/re-enables the replacement while holding the Domain row as the serialization point (`src/store/postgres/provisioning_repository/mod.rs::replace_domain_owner`).
3. `workflow_node_visits` is immutable: UPDATE and DELETE are rejected (`migrations/0006_triggers_constraints.sql`). Therefore successor assignment must append a visit, never edit the old visit.
4. Current assigned-worklist semantics join `workflow_instances.current_node_visit_id`, require the current visit assignee, require a non-`TERMINAL` node, require an enabled Domain, and exclude cancelled instances (`src/store/postgres/workflow_instance_repository/query_worklists.rs::list_assigned_to_me`).
5. Existing transition-like and emergency-move commands append a visit and event, CAS-update `workflow_instances.current_node_visit_id` and `workflow_state_version`, complete a command receipt, and write audit in one transaction. This migration uses that relevant storage shape but uses a dedicated successor command/event; it must not use `ADMIN_EMERGENCY_OVERRIDE`.
6. Existing conformance convention creates a disposable PostgreSQL database, applies repository migrations, runs checks, and drops it (`contracts/workflow-http/v1/conformance/run.sh`). Release convention requires a full source SHA and clean detached checkout (`scripts/release.sh`).

## 4. Explicit non-goals and prohibitions

The implementation and any production run MUST NOT:

- add a general, reusable, or long-lived HTTP principal reassignment API;
- accept arbitrary old/new principal IDs; both UUID constants must be compiled into the one-time tool;
- update or delete any pre-existing `workflow_node_visits`, `workflow_events`, `workflow_submissions`, context revisions, receipts, or historical audit rows;
- change `workflow_instances.created_by_principal_id`;
- change a historical visit's `assignee_principal_id` from OLD to NEW;
- represent this operation as `ADMIN_EMERGENCY_OVERRIDE` or emit `ADMIN_EMERGENCY_OVERRIDE_COMMITTED`;
- import, copy, reconcile, or otherwise touch the legacy 5,583 `dlist` archive;
- select another database, rewrite `DATABASE_URL`, or introduce fallback database discovery;
- delete or disable OLD;
- create, delete, or modify Auth identity, client, credential, token, or authorization mappings;
- add a SQL migration or change database schema;
- perform production apply as part of implementation or Spec review.

Required historical invariants:

```text
EVER_ASSIGNED_58_HISTORY_REWRITTEN = 0
NODE_VISIT_111_HISTORY_REWRITTEN = 0
HISTORY_REWRITE_ALLOWED = NO
LEGACY_5583_IN_SCOPE = NO
```

The pre-existing 111 OLD-assigned visit rows remain byte-for-byte unchanged. The successor operation appends one NEW-assigned visit, so it does not convert the 111 into NEW history.

## 5. Plan contract

### 5.1 Invocation and write safety

Running the tool without `--apply` is plan mode. Plan mode:

- requires a supplied full source Git SHA, exact HEAD equality, and a clean checkout before database access;
- requires the exact compiled OLD and NEW constants;
- connects only to the already supplied svc-workflow `DATABASE_URL` and must never set or replace it;
- opens a read-only, consistent PostgreSQL transaction;
- performs no INSERT, UPDATE, DELETE, DDL, migration, audit, or receipt write;
- exits non-zero on an incomplete, ambiguous, or ineligible snapshot;
- requires the output plan path to be outside the Git checkout, so a reviewed plan cannot make the checkout dirty;
- serializes the plan with the repository's existing RFC 8785/JCS dependency; the plan file bytes are exactly the JCS UTF-8 bytes with no BOM or trailing newline, and the printed SHA-256 is computed over those exact file bytes;
- reports `WRITES = 0`.

### 5.2 Canonical reviewed snapshot

The canonical plan must contain at least:

- `specId`, fixed `migrationId`, `migrationKind`, tool source Git SHA, and clean-tree assertion;
- exact OLD and NEW UUIDs;
- the exact `databaseIdentity` object defined below, without credentials;
- both principal rows' relevant state: existence, `principal_type`, and `enabled`;
- exactly nine objects sorted by `(domain_id, role_key)`, each containing `domain_id`, `domain_key`, Domain enabled state, `role_key`, OLD `binding_id`, and any existing NEW binding identity/state;
- exactly one `DOMAIN_OWNER` binding and exactly eight `DOMAIN_MEMBER` bindings; any other or duplicate authority shape is conflict, not an invitation to normalize it;
- exactly one active-responsibility object containing `workflow_instance_id`, `domain_id`, `definition_version_id`, current `node_visit_id`, `node_id`, `visit_number`, current assignee, `entered_by_transition_id`, visit `created_at`, current context revision, `workflow_state_version`, cancelled/archived state, node type, Domain enabled state, and count of open visit-scoped assistance cases;
- OLD and NEW current worklist counts under the actual assigned-worklist predicate;
- the exact historical arrays and digests defined below, with evidence counts `oldEverAssignedInstanceCount = 58` and `oldAssignedNodeVisitCount = 111`;
- `domainSnapshotCount = 9`, `activeResponsibilitySnapshotCount = 1`;
- `snapshotDigest`, computed as SHA-256 over the RFC 8785/JCS UTF-8 bytes of the plan object with `snapshotDigest` omitted; the separately printed `planFileSha256` is SHA-256 over the final full plan file bytes.

`databaseIdentity` must be produced by one query on the same connection:

```sql
SELECT current_database() AS database_name,
       (SELECT oid::text FROM pg_database WHERE datname = current_database()) AS database_oid,
       (pg_control_system()).system_identifier::text AS system_identifier,
       inet_server_addr()::text AS server_address,
       inet_server_port() AS server_port;
```

Its canonical object shape is exactly `{databaseName, databaseOid, systemIdentifier, serverAddress, serverPort}`. The first three values are JSON strings; `serverAddress` is a JSON string or `null` for a Unix socket; `serverPort` is a JSON integer or `null`. Failure or lack of permission to obtain any non-null-required field is conflict. This tuple deliberately combines cluster, database, and endpoint evidence. It is an accepted operational identity boundary, not a cryptographic defense against a byte-for-byte physical clone deliberately presented at the same endpoint; the separate production authorization must verify the endpoint.

Historical evidence is exact and deterministic:

- `oldAssignedVisits` contains all rows whose `assignee_principal_id = OLD`, projected as exactly `{nodeVisitId, workflowInstanceId, nodeId, visitNumber, assigneePrincipalId, enteredByTransitionId, createdAt}`; UUIDs are lowercase hyphenated strings, nullable transition IDs are JSON null, and `createdAt` is UTC RFC 3339 with exactly six fractional digits (`YYYY-MM-DDTHH:MM:SS.ffffffZ`, preserving PostgreSQL microsecond precision);
- sort `oldAssignedVisits` ascending by `nodeVisitId`; duplicates are conflict; `oldAssignedVisitsDigest = SHA-256(JCS(oldAssignedVisits))`;
- `oldEverAssignedInstanceIds` is the deduplicated set of `workflowInstanceId` values from that array, sorted ascending as lowercase UUID strings; `oldEverAssignedInstanceIdsDigest = SHA-256(JCS(oldEverAssignedInstanceIds))`;
- counts are the respective array lengths and must be exactly 111 and 58.

Plan returns conflict with zero writes if either supplied history fact is not exactly 58/111. The exact nine `(domain_id, role_key, binding_id)` tuples, exact active `(workflow_instance_id, current_node_visit_id, workflow_state_version)` tuple, complete historical arrays, and their digests become frozen only when the plan file and digest are independently reviewed. Count-only approval is invalid.

### 5.3 Eligibility

The one current responsibility is eligible only if all of the following hold in the same snapshot:

- the instance points to the visit through `current_node_visit_id`;
- the visit assignee is exactly OLD;
- the node is non-terminal;
- `workflow_instances.cancelled = FALSE`;
- the Domain is enabled;
- it is visible under the repository's assigned-worklist semantics;
- it belongs to the exact reviewed plan;
- it has zero open visit-scoped assistance cases. A non-zero value is `conflict`; V1 does not silently void or migrate side-band assistance.

`archived_at` is recorded and frozen as projection evidence but is not an additional eligibility predicate because the production `list_assigned_to_me` query does not filter it. Plan must fail if the actual eligible set is not exactly one, even when another non-authoritative query happens to return one user-visible item.

## 6. Apply gates and conflict model

Apply requires all of the following operator inputs:

- `--apply`;
- the exact reviewed plan file at a path outside the Git checkout;
- the exact reviewed plan SHA-256;
- the full implementation Git SHA expected to be running;
- a fixed migration actor principal UUID recorded in the reviewed execution authorization.

The tool must fail before database access unless:

- HEAD equals the supplied full implementation SHA;
- the checkout is clean;
- the plan path is outside the checkout;
- OLD and NEW in the plan equal the compiled constants;
- the plan `specId`, `migrationId`, and `migrationKind` equal the compiled constants;
- the plan file bytes are canonical JCS bytes, their full-file SHA-256 equals the operator-supplied `planFileSha256`, and the embedded `snapshotDigest` equals SHA-256 over the JCS bytes of the same object with `snapshotDigest` omitted.

After connecting but before any write, the tool must query database identity and require an exact match with the plan. Inside the apply transaction, it must then re-read and lock the relevant facts. For a first execution, all of these gates are mandatory:

```text
OLD principal exists and has reviewed type/status
NEW principal exists, is enabled, and has reviewed type/status
OLD enabled Domain bindings = 9
NEW enabled Domain bindings = 0
OLD active assigned-worklist responsibility = 1
NEW active assigned-worklist responsibility = 0
exact Domain/role/binding set = reviewed plan
exact current instance/visit/state-version set = reviewed plan
exact pre-existing OLD history identity/digest set = reviewed 58/111 plan evidence
open assistance cases for target visit = 0
```

The migration actor must exist and be enabled; it is audit identity only and does not become OLD or NEW. Its authorization is an execution-approval concern and must be recorded in the reviewed run authorization.

Any missing row, extra row, state/version change, role change, eligibility change, duplicate audit, database mismatch, serialization failure, unexpected affected-row count, or failed postcondition returns:

```text
RESULT = conflict
WRITES = 0
```

`WRITES = 0` means zero committed writes. Every error path rolls back the transaction. The tool must never widen the selection to make the frozen counts match.

## 7. Transaction and locking model

### 7.1 Frozen model

```text
TRANSACTION_MODEL = ONE_POSTGRESQL_SERIALIZABLE_TRANSACTION
```

The repository facts required by this migration are in one PostgreSQL store, so no prepare/commit protocol is needed or allowed in V1. If implementation investigation proves any required business mutation or durable audit cannot participate in the same PostgreSQL transaction, implementation stops with:

```text
OWNER_DECISION_REQUIRED = YES
IMPLEMENTATION_BLOCKED = ATOMIC_STORE_BOUNDARY_NOT_PROVEN
```

It must not degrade to best-effort sequencing or compensating writes.

### 7.2 Required transaction order

One `SERIALIZABLE` transaction must cover all Domain changes, the current-responsibility successor append/projection change, the completed receipt, and the single durable migration audit:

1. acquire a migration-specific PostgreSQL transaction advisory lock;
2. execute the exact-rerun check in section 10;
3. lock OLD, NEW, and migration actor principal rows in deterministic UUID order;
4. lock the nine Domain rows in sorted UUID order;
5. lock the relevant Domain bindings and reject any snapshot drift;
6. lock the frozen workflow instance, current visit, node, and relevant open assistance rows; re-evaluate exact worklist eligibility and version;
7. perform Domain successor changes;
8. insert one `PROCESSING` command receipt with command type `PRINCIPAL_SUCCESSOR_MIGRATION_V1` and an idempotency key derived from the fixed migration ID;
9. append one successor visit and CAS-update the instance projection/version;
10. append one dedicated successor event;
11. complete the receipt with the canonical result;
12. insert one durable migration security audit;
13. verify all postconditions inside the transaction;
14. commit once.

No step may commit independently. A PostgreSQL serialization failure is a fail-loud conflict; the tool does not auto-retry against a potentially changed scope.

## 8. Domain successor semantics

### 8.1 Owner

For the one exact `DOMAIN_OWNER` snapshot row, use the existing owner-replacement semantics inside the outer transaction:

- lock and verify the enabled Domain;
- verify OLD is the one enabled owner;
- disable OLD's enabled owner binding;
- insert or re-enable NEW's owner binding using the existing `(domain_id, principal_id, role_key)` identity;
- rely on and verify the partial unique index preserving at most one enabled owner.

The tool may duplicate the repository's exact SQL inside the offline binary because existing helper visibility/transaction ownership does not permit opening a nested transaction. It must not call the public HTTP/application command nine times.

### 8.2 Member-class bindings

For each of the remaining eight exact reviewed `DOMAIN_MEMBER` rows:

- require and preserve `role_key = 'DOMAIN_MEMBER'`;
- insert or re-enable NEW's corresponding binding;
- disable OLD's corresponding binding and set `disabled_at`;
- require exact affected-row counts;
- verify after mutation that NEW is enabled and OLD is disabled for that tuple.

All enable/disable operations remain invisible until the single transaction commits. On commit there must be no migrated `(domain_id, role_key)` for which OLD and NEW are both enabled. There is never a committed long-lived dual-authority state.

`DOMAIN_SUCCESSOR_CHANGES = 9` counts nine transferred authority tuples, not the number of physical SQL row writes.

## 9. Current-responsibility successor semantics

The migration must not update the OLD visit. For the one exact frozen current responsibility it must:

1. create one new `workflow_node_visits` row for the same instance and same node;
2. assign the new visit to NEW;
3. allocate `visit_number = MAX(visit_number for instance/node) + 1` under the locked instance;
4. preserve `entered_by_transition_id = NULL`, because this is a successor transfer rather than a workflow-definition transition;
5. CAS-update only `workflow_instances.current_node_visit_id`, `workflow_state_version = old + 1`, and `updated_at`, matching the frozen old visit and state version;
6. append one `PRINCIPAL_SUCCESSOR_MIGRATION_COMMITTED` event with:
   - source visit = immutable OLD-assigned current visit;
   - target visit = new NEW-assigned successor visit;
   - from/to node = the same current node;
   - `transition_effect = NULL`;
   - unchanged current context revision;
   - event sequence/new state version equal to the incremented state version;
   - actor = reviewed migration actor;
   - event data containing fixed migration ID, `snapshotDigest`, `planFileSha256`, OLD, NEW, reason `PRINCIPAL_SUCCESSOR`, and source/target visit IDs;
   - `event_data_digest` computed from the repository's canonical JSON digest convention;
7. complete the one dedicated receipt, including its canonical response digest, in the same transaction.

The receipt/event identity is frozen as follows:

- receipt `principal_id = <reviewed migration actor>`;
- receipt `idempotency_key = "principal-successor-v1:" + <fixed migrationId>`;
- receipt `command_type = PRINCIPAL_SUCCESSOR_MIGRATION_V1`;
- receipt `request_hash = SHA-256(JCS({specId,migrationId,migrationKind,oldPrincipal,newPrincipal,sourceGitSha,snapshotDigest,planFileSha256,databaseIdentity,migrationActor}))`;
- completed receipt `response_status = 200`; `response_body` is a fixed object containing command ID, source/target visit IDs, old/new workflow state versions, all result counters, both plan digests, and exact OLD/NEW successor binding IDs; `response_digest` is the repository canonical digest of that body;
- event `event_schema_version = v1`, `command_id = receipt.command_id`, `causation_id = NULL`, and `correlation_id = NULL`;
- event `event_data_digest` is the repository canonical digest of the fixed event-data object.

This is a new successor event/visit/receipt chain. It is not an emergency override and not a business transition submission.

After the projection changes, the OLD visit remains historical provenance. All earlier visits, events, receipts, submissions, and context revisions remain unchanged.

## 10. Durable audit and exact-rerun NOOP

### 10.1 First successful execution

Insert exactly one `workflow_security_audits` row:

```text
action        = PRINCIPAL_SUCCESSOR_MIGRATION_V1_COMMITTED
resource_type = PRINCIPAL_SUCCESSOR_MIGRATION
resource_id   = <fixed migrationId>
principal_id  = <reviewed migration actor>
```

Its `details` must contain the spec ID, migration kind, OLD, NEW, source Git SHA, clean-tree assertion, database identity evidence, `snapshotDigest`, `planFileSha256`, exact nine OLD and NEW Domain tuple/binding identities (including every successor `binding_id`), exact source/target visit identities, receipt/command ID, before/after counts, and committed result counters. `workflow_security_audits` is append-only by repository convention but not protected by an immutability trigger; the immutable completed receipt and immutable event are therefore the primary mechanical rerun anchors.

First success must return exactly:

```text
RESULT = applied
DOMAIN_SUCCESSOR_CHANGES = 9
CURRENT_RESPONSIBILITY_SUCCESSOR_CHANGES = 1
MIGRATION_AUDITS_CREATED = 1
```

### 10.2 Rerun decision order

The exact-rerun check intentionally precedes first-run gates because a successful migration changes the required OLD/NEW counts.

After acquiring the advisory lock, apply first queries the completed receipt by the fixed migration actor/idempotency key and verifies its immutable identity, request hash, status, body, and response digest; it then verifies the unique immutable event by `command_id`. The audit is checked afterward by fixed action/resource identity:

- no receipt and no audit: continue through all first-run gates;
- receipt/audit presence is asymmetric, receipt is not completed, event is absent/duplicated, or any canonical metadata differs: conflict, zero committed writes;
- more than one matching audit: conflict, zero committed writes;
- one exact receipt/event/audit chain: verify the new visit, all nine exact successor binding IDs and post-migration Domain states, the current projection, and historical non-rewrite invariants.

The audit itself is not described as immutable. Any later audit alteration is detected as a mismatch against the immutable receipt/event metadata and returns conflict rather than being repaired.

Only when the audit metadata and all postconditions match exactly may the second run return:

```text
RESULT = NOOP
WRITES = 0
AUDITS_CREATED = 0
```

NOOP performs no audit-attempt write, no receipt replay write, and no timestamp update. Missing or inconsistent post-state is conflict, never self-healing.

## 11. Postconditions and historical proof

Before commit, first execution must verify:

- NEW has exactly the reviewed nine enabled Domain authority tuples;
- OLD has none of those nine enabled tuples;
- each migrated tuple has exactly one enabled successor authority according to its role semantics;
- the exact instance points to the new NEW-assigned visit;
- the source OLD visit still exists and every column equals the plan-time source row;
- no pre-existing OLD-assigned visit was updated or deleted;
- no pre-existing event, receipt, submission, or context revision was updated or deleted;
- OLD's pre-existing 111 assigned visit rows remain OLD-assigned;
- the set of 58 historically ever-assigned instances has not been rewritten;
- the legacy `dlist` archive was not queried or written by the tool;
- exactly one successor event, one completed successor receipt, and one migration audit exist for the fixed migration ID.

Database immutability triggers provide an additional mechanical guard for visits/events/submissions; tests must also compare frozen row projections before and after.

## 12. Minimal implementation closure

Implementation is frozen to three files:

1. `src/bin/one_time_principal_successor_migration_v1.rs`: offline plan/apply tool with compiled OLD/NEW/spec/migration constants, canonical plan validation, serializable transaction, successor writes, audit, and exact NOOP.
2. `scripts/run_principal_successor_migration_v1_conformance.sh`: disposable PostgreSQL runner that creates an isolated database, applies existing migrations, executes the focused tests/tool scenarios from a clean fixed SHA, and always drops the database.
3. `tests/26_principal_successor_migration_v1.rs`: focused integration/conformance tests that invoke the actual binary and inspect PostgreSQL facts.

Cargo automatically discovers `src/bin/*.rs`; integration tests invoke the actual target through `CARGO_BIN_EXE_one_time_principal_successor_migration_v1`, so no `Cargo.toml` edit is expected. Plan fixtures are created only in disposable paths outside the checkout. No migration file is permitted.

Fault injection is compiled only when the runner supplies a dedicated custom Rust cfg (for example `RUSTFLAGS="--cfg successor_migration_conformance"`). The binary must compile all fault hooks out of ordinary/release builds; under the conformance cfg it accepts only enumerated phase names, requires the disposable test database identity, and aborts the current transaction at that phase. A Cargo feature is forbidden because it would require a fourth `Cargo.toml` change.

```text
IMPLEMENTATION_FILES = 3
OWNER_DECISION_REQUIRED = NO
```

If implementation cannot be completed in these three files, work stops before creating a fourth file and reports:

```text
OWNER_DECISION_REQUIRED = YES
PROPOSED_FOURTH_FILE = <exact path>
WHY_UNAVOIDABLE = <specific reason>
SCOPE_IMPACT = <specific impact>
```

No implementation agent may self-authorize that expansion.

## 13. Required tests and acceptance matrix

The disposable PostgreSQL suite must cover at least:

1. binary rejects any OLD/NEW other than the two compiled UUIDs and exposes no arbitrary-principal flags;
2. plan emits zero writes and freezes the exact nine Domain tuples;
3. plan freezes exactly one active responsibility using current-visit/non-terminal/worklist semantics;
4. one owner is replaced and the single-owner unique invariant remains true;
5. all eight exact `DOMAIN_MEMBER` bindings transfer by NEW-enable plus OLD-disable;
6. no committed state has OLD and NEW simultaneously enabled for a migrated tuple;
7. successor creates a dedicated receipt, new same-node visit, event, and projection/version increment;
8. all 111 pre-existing OLD-assigned visits and the 58-instance ever-assigned provenance set remain unchanged;
9. injected failure at every mutation phase rolls back Domains, visit/projection/event/receipt, and audit together;
10. any principal, count, Domain tuple, role, binding identity, instance, current visit, state version, terminal/cancel/archive state, assistance state, or database identity drift returns conflict with zero committed writes;
11. exact metadata rerun is NOOP with zero writes and zero audits; altered metadata or damaged post-state is conflict;
12. test fixture representing the legacy 5,583 archive remains untouched and the tool contains no import/database-switch path;
13. source SHA mismatch or dirty checkout fails before database access;
14. concurrent apply attempts serialize to one applied result plus one exact NOOP (or one explicit serialization conflict followed by an exact NOOP), with one audit total;
15. post-implementation production acceptance uses the real Feishu `agt_cto-agent` identity and the capabilities below.

No production test or apply runs during implementation review.

## 14. Production run authorization and acceptance

A later production run requires a separate reviewed run record containing:

- accepted Spec commit;
- exact implementation commit and full Git SHA;
- clean checkout evidence;
- exact plan file and SHA-256;
- exact database identity evidence;
- migration actor and authorization evidence;
- backup/recovery readiness evidence that does not involve changing databases;
- operator and independent reviewer approval;
- command transcript and result JSON.

After a successful authorized apply, acceptance must be performed through real Feishu as `agt_cto-agent`:

```text
agt_cto-agent
  -> workflow_my_domains
  -> workflow_my_tasks
```

Expected:

```text
WORKFLOW_MY_DOMAINS_COUNT = 9
WORKFLOW_MY_TASKS_COUNT = current active count visible under interface semantics
EXPECTED_FROM_REVIEWED_SNAPSHOT = 1
```

The acceptance does not request or expect 58 historical tasks. If current interface-visible task count differs from the committed migration result, acceptance fails and investigates without rewriting history or broadening migration scope.

## 15. Review checklist

Independent review must explicitly confirm:

- exact OLD and NEW constants;
- exact reviewed nine-row Domain snapshot mechanism;
- exact reviewed one-row active-responsibility snapshot mechanism;
- existing owner replacement semantics;
- member successor enable/disable semantics;
- no persistent dual authority;
- dedicated current-responsibility successor event/visit/receipt;
- no historical visit/assignment rewrite;
- one serializable transaction and fault-injection proof;
- drift conflict is fail-closed with zero committed writes;
- exact rerun NOOP;
- legacy 5,583 archive exclusion;
- real Feishu Domains/Tasks acceptance;
- three-file implementation closure;
- lifecycle activation authorizes only the bounded implementation Contracts, performs no implementation, and leaves production apply unauthorized.

## 16. Final frozen fields

```text
TASK_NAME = 过户 执行

SPEC_ID = SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1
SPEC_FILE = docs/specs/SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1.md
BASE_HEAD = 6d4e117bfe8b41b82cf74d4e839125ffc4ee7261
SOURCE_SPEC_REVISION = 6f1f546787bd5fb1644ec91327d3e7374dc28165
SOURCE_SPEC_AUTHORING_BASE = 8cda3d05e1c22814b7aeaace97d317380df83836
COMMIT = REPORTED_EXTERNALLY_AFTER_DOCS_ONLY_COMMIT (not self-embedded)

PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V2
BOUND_AMENDMENT = PBV2-ONE-TIME-SUCCESSOR-001
PARENT_ACCEPTED_MAIN_REVISION = 6d4e117bfe8b41b82cf74d4e839125ffc4ee7261
AUTHORITY_COMPATIBILITY = PASS
SEMANTIC_DELTA_FROM_6F1F546 = AUTHORITY_ALIGNMENT_ONLY

MIGRATION_KIND = ONE_TIME_SUCCESSOR
OLD_PRINCIPAL = 3e2439d2-fb54-44f5-afee-77aa17c40d22
NEW_PRINCIPAL = 4e5a4578-0645-4133-bd35-b80e453dfee9

DOMAIN_SNAPSHOT_COUNT = 9
ACTIVE_RESPONSIBILITY_SNAPSHOT_COUNT = 1

HISTORY_REWRITE_ALLOWED = NO
LEGACY_5583_IN_SCOPE = NO

TRANSACTION_MODEL = ONE_POSTGRESQL_SERIALIZABLE_TRANSACTION
NOOP_MODEL = EXACT_RECEIPT_EVENT_AUDIT_CHAIN_AND_POSTSTATE_MATCH
IMPLEMENTATION_FILES = 3

STATUS = accepted
INDEPENDENT_REVIEW_RESULT = PASS
REQUIRED_FIXES = NONE
SEMANTIC_DELTA_AFTER_REVIEW = LIFECYCLE_ONLY
IMPLEMENTATION_AUTHORITY = contracts
IMPLEMENTATION_AUTHORITY_ACTIVATION = ACCEPTED_ON_MAIN
IMPLEMENTATION_PERFORMED = NO
PRODUCTION_APPLY_AUTHORIZED_NOW = NO
MERGE_REQUIRED_FOR_ACTIVATION = YES
PRODUCTION_CHANGE = NONE
```

End of accepted Spec. Merge of this exact accepted head activates only its bounded implementation Contracts; it performs no implementation or database write and does not authorize production apply.
