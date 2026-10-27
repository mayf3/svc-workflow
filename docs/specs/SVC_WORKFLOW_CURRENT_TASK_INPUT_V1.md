---
spec_id: SVC_WORKFLOW_CURRENT_TASK_INPUT_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
date: 2026-10-27
scope:
  - svc-workflow target-specific authorized current-task read projection and bounded exact body reads
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V8
architecture_authority: SVC_WORKFLOW_ARCHITECTURE_V0_4_3
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
base_head: 371ea8155d8fa3025ef26af75d128f3047736d6d
---

# SVC_WORKFLOW_CURRENT_TASK_INPUT_V1

## 1. Goal

Give the authorized executor of a known WorkflowInstance a small, deterministic
read of the **current task**, its exact incoming Submission, recent feedback and
explicitly requested originals. The executor must not have to choose an oldest
history page and guess whether it represents the current work.

This is a new read projection over existing authoritative facts. It does not
decide whether a review is correct or whether business evidence is sufficient.

## 2. Scope and non-goals

One HTTP path, with `input` and `body` views, in svc-workflow. Reuse existing
Principal/visibility checks, authorized repeatable-read snapshots, `build_full`,
immutable Context/Submission facts and existing RETURN references. Support
existing Legacy DRAFT/NORMAL and model-3 TASK instances without changing their
lifecycle or eligibility. A read also remains possible for an authorized full
viewer of terminal, disabled-Domain or revoked-version state under existing rules.

No new table, cache, ledger, persisted input packet, evidence store, scheduler,
Workflow engine, activation, transition gate, semantic supersession rule, ACL,
grant, instance migration, production deployment or legacy cutover is introduced.
Existing endpoints and their wire/order/visibility contracts are unchanged.
The endpoint does not fetch filesystem paths, URLs, Spec bodies or external
artifacts mentioned inside a Submission. A returned reference is not proof that
its target was read. Source proof propagation and durable delivery are separate.

## 3. Authority and dependencies

Owning repository: `mayf3/svc-workflow`; base: the exact `base_head` above.
Product and Architecture parents retain their existing meanings, including
Domain isolation, full/restricted content visibility and the separation of
read access from dispatch or command authority.

`docs/contracts/WORKFLOW_QUERY_CONTRACT_V0_1.md` has `Status: CURRENT` and freezes
the PR-4 query slice. Its §2/3/5/8 snapshot, visibility, history and consistency
rules are retained by the current Architecture. This independent new projection
does not add parameters or change the frozen behavior/error sets of those
existing queries. It reuses their implementation and defines its own additive
query, response and failure surface below. No predecessor is superseded.

`SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1` remains unchanged: this read neither
adds semantic evidence validation nor changes its selected ADVANCE gate or
RETURN behavior. WACI lineage/source-evidence permissions are not extended;
this projection exposes ordinary same-instance facts only and never relabels
ancestor facts as current-instance facts.

The DSH Broker consumer must receive its own repository-owned authority for a
transparent tool mapping. This Spec authorizes svc-workflow only; it cannot
accept or govern DSH code. A consumer may use a separately accepted companion
Spec, but its exact revision must be recorded before integration conformance.

Route: `NEW / BRIEF / CONTROLLED / AUTHORITY_AUTHORING`, docs first.
This candidate is proposed. Independent review and an attributable acceptance
act are separate; the author does not perform either by writing this file.
Implementation is forbidden until acceptance is active in the owning base.

## 4. Current State

`STATE-CTI-001`: at svc-workflow `371ea8155d8fa3025ef26af75d128f3047736d6d`,
source inspected 2026-10-27, the new endpoint does not exist. `ListAssignedToMe`
already returns full detail, the latest 50 Submissions and latest 50 RETURN
events with truncation flags. This is a multi-instance worklist, not exact
current-visit input. Basis: `OBS-CTI-001`, `EVD-CTI-001`.

`STATE-CTI-002`: this candidate has no implementation, executed acceptance or
production adoption evidence. All acceptance cases below are future tests.

## 5. Observations

- `OBS-CTI-001`: read `query_worklists.rs::list_assigned_to_me`,
  `query_detail.rs::{build_full,fetch_submissions,list_submission_history}` and
  `query_visibility.rs::authorized_snapshot` at the base above. Worklist uses
  one authorized snapshot and descending recent Submissions; ordinary history
  uses ascending `(created_at,id)` pagination. Neither exact-entry selection nor
  this small-response contract exists. Environment: local source, no DB calls.
- `OBS-CTI-002`: read the same base's transition/create/revise writers and
  recovery event checks. A transition event identifies its target visit and
  optional Submission. `CONTEXT_REVISED` keeps the visit; its current Context
  can therefore differ from the entry event's Context. NULL entered transition
  and visit number 1 do not uniquely mean instance creation.
- `OBS-CTI-003`: `compute_json_digest` uses existing JCS canonicalization plus
  SHA-256. Stored Submission/Context digests are not ordinary display-JSON
  hashes. Query reads currently do not recompute those digests. RETURN
  `relatedSubmissionIds` establishes same-instance references, not resolution
  or supersession of earlier findings.
- `OBS-CTI-004`: the sampled ACB run returned ten history records including
  later corrections, then active-surface replacement removed middle records;
  a later limit-6 read returned the oldest six ending at an old RETURN. The
  tester repeated the old reasons and withdrew them after direct original
  reads. Provenance: external non-authoritative investigation
  `unattended-workflow-reuse-assessment.json`, SHA-256
  `0b0069aa239da8a610edb2d5de6b446346f1f6640e8c1d6b92032833c9fc7fcb`,
  held with the change Brief. This is sampled behavior, not a general model
  reliability result or proof of all causal reasoning.

## 6. Claims and assumptions

- `CLM-CTI-001` — SUPPORTED: exact target/entry selection and bounded original
  retrieval are a separate missing read contract, although the underlying
  facts and authorized snapshot already exist. Supported by `EVD-CTI-001`.
- `CLM-CTI-002` — INFERRED: deterministic current input reduces the observed
  stale-page failure opportunity. It does not guarantee that an executor reads
  every required body or reasons correctly. Supported by `EVD-CTI-002`.
- No assumption grants an actor new content access, makes a referenced finding
  resolved, or treats a stored UUID/digest as semantic proof.

## 7. Evidence relations

- `EVD-CTI-001`: `OBS-CTI-001/002/003` SUPPORT `CLM-CTI-001` and
  `STATE-CTI-001`; coordinates are the source base above. Sufficient for source
  shape and reuse, not installed behavior. Provenance: named source functions.
- `EVD-CTI-002`: `OBS-CTI-004` SUPPORTS `CLM-CTI-002`; bound to the sampled
  public run and investigation hash. It motivates deterministic selection but
  does not prove unattended closure or future semantic correctness.

## 8. Decisions

- `DEC-CTI-001` — Proposed by the author for delegated acceptance: use one new
  target-specific query and one body view on the same path. Do not make a model
  traverse general history, and do not build a second state store.
- `DEC-CTI-002` — Select entry facts and fixed recent windows on the server.
  Explicit IDs can retrieve older originals; references never decide truth.
- `DEC-CTI-003` — Keep each HTTP response in one authorized snapshot. Cross-call
  body assembly requires both current coordinates and expected content digest;
  it never claims to hold a transaction open across calls.
- `DEC-CTI-004` — Keep output bounded and honest. Partial content and limited
  history coverage are explicit, with exact body retrieval. No truncated text
  is labeled a complete original.

## 9. Contracts

### CTR-CTI-001 — Endpoint, caller and request types

The endpoint MUST be:

```text
GET /internal/v1/workflow-instances/{workflowInstanceId}/current-task-input
scope: workflow.read
query:
  view: input | body                         # default input
  expectedNodeVisitId?: UUID
  expectedWorkflowStateVersion?: integer >= 1
  expectedContextRevisionId?: UUID
  submissionIds?: comma-separated 1..8 distinct UUIDs   # input only
  bodyKind?: CURRENT_DETAIL | SUBMISSION_PAYLOAD       # body only
  bodyId?: UUID                                       # body only
  expectedBodySha256?: [0-9a-f]{64}                     # body only
  offset?: integer >= 0                               # body only, default 0
```

Expected coordinate fields MUST be all present or all absent. `body` requires
all three and bodyKind/bodyId/expectedBodySha256. Unknown fields, duplicate
query keys, malformed/partial combinations, invalid enums, duplicate IDs and
out-of-range values MUST return 422 `invalid_current_task_input` without
returning content. UUID identity uses parsed UUID equality, not textual case.
No actor, Principal, grant, semantic-model or permission override is accepted.

Both views MUST reuse the existing actor validation and instance visibility.
Only full visibility may receive this full-content projection. Restricted
historical participants receive the existing 403
`restricted_history_not_visible` and existing rejection audit; they may still
use their existing restricted history endpoint unchanged. Missing/hidden
instances retain the same masked 404. Global reader or scheduler grants alone
MUST NOT grant current-task content. Domain-disabled read behavior remains the
existing query behavior. UUID possession and references grant no access.

### CTR-CTI-002 — One snapshot, exact current binding, no business mutation

Each call MUST authorize, validate and select within one PostgreSQL
`REPEATABLE READ` transaction established before the first query, reusing
`authorized_snapshot` and its existing base/fact consistency checks. No
`FOR UPDATE`, business write, Event, Receipt, input record or success Audit is
allowed. Existing denial SecurityAudit is retained; audit failure fails closed.

The response MUST carry this server-derived binding, with snake_case wire keys:

```text
Binding = {
  workflow_instance_id: UUID, definition_version_id: UUID,
  node_visit_id: UUID, workflow_state_version: integer,
  context_revision_id: UUID
}
```

After authorization, a supplied expected triple differing in any member MUST
return 409 `current_task_input_stale` without body bytes. Omitted expected fields
in `input` mean discover a fresh binding, not authority to mutate it. The
response is a snapshot observation, not a lease or future-current guarantee;
subsequent commands still use their existing fresh-read/CAS/actor checks.

`CURRENT_DETAIL` MUST be the exact existing `build_full` JSON value at that
snapshot, including current Context, instructions, eligibility and outgoing
schemas. It is a transient body, not a new durable fact. Its bodyId is the
current node_visit_id; its identity also includes the whole binding and digest.

### CTR-CTI-003 — Deterministic entry, windows and explicit references

`input` MUST return `entry`, `recent_submission_ids`, `return_feedback`,
`requested_submissions`, `submissions` and `detail_body` with one binding.

Entry selection MUST use the event that introduced the current Visit, not the
latest Submission, timestamp, node key, visit number, or NULL transition alone:

1. Select introducing events whose target is current node_visit_id and whose
   source is NULL or a different Visit. A same-Visit Context revision is not
   an introducing event. More than one introducing event is an internal
   consistency failure; absence/unknown introducing event semantics yields
   `entry.status=UNSUPPORTED_ENTRY`, never a fabricated initial visit.
2. `INSTANCE_CREATED` / `WORKFLOW_INSTANCE_IMPORTED`: `INITIAL`, with null
   source and null Submission; preserve the original Context reference.
3. `WORKFLOW_TRANSITION_COMMITTED` or
   `WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED`: `TRANSITION`;
   event source/target/Context,
   transition definition and target Visit.entered_by_transition_id MUST agree.
   The optional Submission is joined by event.submission_id. If present, its
   instance, source Visit, Context, transition and payload digest MUST agree
   with those event facts; missing/mismatched required facts fail internally.
   Null Submission is legal and returns `submission_status=NONE_RECORDED`.
4. `ADMIN_EMERGENCY_OVERRIDE_COMMITTED`, `HUMAN_EXECUTOR_NORMALIZED` and
   `PRINCIPAL_SUCCESSOR_MIGRATION_COMMITTED` introducing events return
   `NON_TRANSITION_ENTRY` with original event references and no invented
   transition/Submission. Their source/target and same-instance facts are
   validated. An unrecognized introducing type returns `UNSUPPORTED_ENTRY`.

Entry event context_revision_id and current binding.context_revision_id MUST
remain separate. Legitimate later Context revisions MUST NOT fail merely
because these IDs differ. Event data is a reference cross-check; the query
MUST NOT rebuild Context/Visit/Submission facts by replaying Event payloads.

The server MUST select, independent of model preference:

- exact entry Submission, when one is recorded;
- latest 8 Submissions by `(created_at DESC, submission_id DESC)`;
- latest 4 RETURN events by `event_sequence DESC`, and their recorded
  Submission bodies when present;
- each explicitly requested submissionId, ordered by parsed UUID bytes.

Union bodies MUST be deduplicated by Submission ID, ordered entry first,
explicit requests next, recent Submissions next, then RETURN Submissions;
the maximum union is 21. `recent_submissions_truncated` and
`return_feedback_truncated` MUST use limit+1 evidence; they cannot be inferred
from a full page alone. These windows are not an exhaustive evidence set.

Each selected Submission carries its existing ID, instance/sourceVisit/Context,
author/transition/effect, created_at, stored payload_digest and a body descriptor.
Each RETURN entry carries eventId/sequence/sourceVisit/targetVisit/Context and
optional submissionId. Its `related_submission_ids` are at most the first 16
distinct parsed UUIDs in original array order, with an explicit truncation flag.
The complete array remains in the exact RETURN body. References are not
automatically followed recursively and have no supersedes/resolved semantics.
Malformed persisted RETURN reference shape or cross-instance required
references are internal inconsistency, not a new judgment about its prose.

Each explicit requested ID has `FOUND` or `NOT_FOUND_OR_NOT_VISIBLE`. Missing or
foreign-instance IDs MUST share the latter result and reveal no foreign data.
Absence of an optional requested ID is not a corrupt required entry reference.
The caller can request older correction IDs in a new bounded input call; it
need not traverse a generic history cursor to retrieve a known original.

The metadata wire types are closed objects with these snake_case keys:

```text
SubmissionMetadataWithBody = {
  submission_id: UUID, workflow_instance_id: UUID,
  source_node_visit_id: UUID, context_revision_id: UUID,
  author_principal_id: UUID, transition_id: UUID,
  transition_effect: ADVANCE|RETURN|TERMINATE,
  created_at: RFC3339 string, payload_digest: lower-hex SHA256, body: Body
}
ReturnReference = {
  event_id: UUID, event_sequence: integer,
  source_node_visit_id: UUID|null, target_node_visit_id: UUID|null,
  context_revision_id: UUID|null, submission_id: UUID|null,
  related_submission_ids: UUID[], related_submission_ids_truncated: boolean
}
```

If a RETURN event has no recorded Submission, its related list is empty and
not truncated; this says no reference payload was recorded, not that no
business feedback exists. Nullable fields retain actual stored nulls.

### CTR-CTI-004 — Bounded exact bodies and digest-bound continuation

Every body descriptor MUST have:

```text
Body = {
  kind: CURRENT_DETAIL | SUBMISSION_PAYLOAD, id: UUID,
  sha256: lower-hex SHA256, encoding: "utf-8",
  serialization: "JCS", total_bytes: nonnegative integer,
  offset: integer, text: string, next_offset: integer | null,
  content_status: COMPLETE | TRUNCATED | OMITTED
}
```

The server MUST use the repository's existing JCS canonicalization to obtain
the exact UTF-8 bytes and compute SHA-256 over those bytes. Consumers need only
concatenate the returned UTF-8 bytes and verify sha256; they MUST NOT have to
independently canonicalize JSON. For Submission payloads the recomputed digest
MUST equal stored payload_digest, otherwise return internal consistency error.
For CURRENT_DETAIL, recomputation creates only a transient response digest.
Descriptor SHA is about content bytes; it is not a signature or evidence of
semantic correctness. No claim of original HTTP formatting preservation is made.

Input inline bodies share at most 8192 content bytes: up to 4096 for detail
first, then remaining budget for selected bodies in CTR-CTI-003 order. Prefixes
MUST end at a UTF-8 codepoint boundary. A zero-byte omitted body has offset=0,
text="", next_offset=0 and OMITTED. COMPLETE requires all bytes returned;
otherwise TRUNCATED carries the precise next byte offset. Prefixes are text
fragments of canonical JSON, not independently parseable original JSON objects.

For `view=body`, the service MUST authorize and validate the current binding,
resolve that exact bodyId, recompute the body bytes/digest and compare mandatory
expectedBodySha256 before emitting any bytes. A different digest, even with an
unchanged coordinate triple (for example changed target eligibility), MUST
return 409 `current_task_body_changed`, with no chunk. `CURRENT_DETAIL` only
accepts current node_visit_id; `SUBMISSION_PAYLOAD` accepts an exact visible
same-instance Submission. Missing/foreign body IDs return masked 404
`current_task_body_not_found_or_not_visible`.

Offset MUST be <= total_bytes and a UTF-8 boundary; otherwise 422
`invalid_current_task_input`. A body call returns at most 8192 content bytes,
cut before the next codepoint if necessary; next_offset is the exact next byte
position or null at EOF. Offset=total_bytes yields an empty COMPLETE suffix.
Nonterminal chunks MUST make positive progress. The consumer MUST verify the
full assembled body SHA before claiming a complete original; individual chunks
and a terminal suffix alone do not prove preceding bytes were read.

Every serialized successful HTTP response MUST be <=65536 UTF-8 wire bytes.
If JSON escaping would exceed this bound, reduce inline text prefixes in reverse
allocation order, retaining all selected descriptors/references and truthful
offset/status. UUID/enum/count/time-only metadata is fixed and bounded above;
arbitrary titles, prose or payload fields belong inside bodies, not metadata.
An unknown introducing event type is represented by UNSUPPORTED_ENTRY and
event_type=null rather than copying unbounded unknown text into this projection;
its event_id still identifies the original in the unchanged timeline query.
The response MUST expose `content_complete` (all selected bodies fully inline)
and `history_window_complete` (neither recent window truncated) separately.
Neither flag means business evidence is sufficient. `UNSUPPORTED_ENTRY`, an
unresolved requested ID, or incomplete bodies MUST NOT be labeled complete input.

### CTR-CTI-005 — Explicit results, privacy and compatibility

Successful `input` wire shape MUST be:

```text
{
  view: "input", binding: Binding,
  input_status: "COMPLETE_FOR_SELECTION" | "INCOMPLETE",
  detail_body: Body,
  entry: {status: INITIAL | TRANSITION | NON_TRANSITION_ENTRY | UNSUPPORTED_ENTRY,
          event_id: UUID|null, event_type: string|null, event_sequence: integer|null,
          source_node_visit_id: UUID|null, target_node_visit_id: UUID,
          context_revision_id: UUID|null, transition_id: UUID|null,
          submission_id: UUID|null,
          submission_status: RECORDED | NONE_RECORDED | UNKNOWN},
  recent_submission_ids: UUID[], recent_submissions_truncated: boolean,
  return_feedback: ReturnReference[], return_feedback_truncated: boolean,
  requested_submissions: {id: UUID, status: FOUND|NOT_FOUND_OR_NOT_VISIBLE}[],
  submissions: SubmissionMetadataWithBody[],
  content_complete: boolean, history_window_complete: boolean
}
```

`COMPLETE_FOR_SELECTION` requires supported entry semantics, no unresolved
requested ID, and content_complete=true. It does not require the fixed recent
windows to cover all historical facts; history_window_complete remains explicit.
`body` success is `{view:"body", binding:Binding, body:Body}`. Existing HTTP
error envelope conventions apply; no command Receipt or idempotency key is used.

Existing actor/auth failures remain unchanged. New 409 errors and new 422
validation are as specified above. Required fact/digest corruption returns
500 `internal_consistency_error`; storage failure returns 503
`service_unavailable`, with existing public redaction. An unauthorized or
unavailable read MUST NOT be transformed into an empty complete input, a
business rejection, proof that no correction exists, or a valid mutation.

New views MUST NOT alter old worklist/history/detail results, include another
instance's facts, grant a restricted participant full access, change a frozen
Definition snapshot, generate activation, or invalidate existing CAS behavior.
NORMAL and TASK support is a read compatibility choice, not a declaration that
the production model3 cutover barrier is active.

### CTR-CTI-006 — Consumer boundary and honest closure claims

The svc implementation and tests MUST make no network call to read external
evidence or dispatch the actor. Broker integration MUST use the same endpoint
and transparent server-owned selection, statuses, digests and bodies; it cannot
replace them with a model-written summary, an inferred latest verdict or a
local permissions rule. Acceptance of this svc Spec alone grants no DSH code
or production authority; the separately owned consumer must be reviewed against
this exact wire contract before integrated use is reported passing.

A delivered current-input bundle, matching ACB catalog, accepted Submission,
or advancing Visit MUST NOT be reported as proof of correct semantic review or
unattended end-to-end completion. Technical acceptance is scoped to these reads.
Any broader unattended observation must record the external-control cutoff,
actual existing dispatch/ingress/read/Submission chain and each intervention.
Manual correction after cutoff cannot be counted as an unattended success.

## 10. Acceptance

All six cases require the exact implementation and Spec commits, service build,
isolated database identity, actual command/exit status and request/response
originals. Test definitions below are not executed evidence. No production DB,
principal grants, schedules or old business instances are needed.

### ACC-CTI-001 — Visibility, validation and zero business effects

- Contracts: CTR-CTI-001, CTR-CTI-002, CTR-CTI-005.
- Method/environment: real HTTP service + isolated PostgreSQL; full Owner,
  current assignee, creator-on-DRAFT, restricted historical participant,
  unrelated/disabled Principal and global-reader-only actors.
- Required evidence: full positive/negative matrix, invalid query combinations,
  business-table before/after counts and denial-audit results.
- Expected result: authorized full reads only; masked existence errors and
  exact existing restricted denial; malformed input rejects; zero business
  writes, with only specified denial audits permitted.
- Failure condition: reference/role bypass, content leak, success audit/Receipt,
  changed old endpoint behavior or false COMPLETE on denial.

### ACC-CTI-002 — Exact entry, Context revision and both semantic models

- Contracts: CTR-CTI-002, CTR-CTI-003, CTR-CTI-005.
- Method/environment: isolated real create/transition/revise flows covering
  Legacy NORMAL/DRAFT and model3 TASK, initial import/create, optional null
  Submission, repeated visits to one node and combined revision+transition.
- Required evidence: exact event/visit/Submission relation readbacks; later
  same-visit Context revision; bounded fixtures for supported administrative
  entries and an unsupported introduction type. Existing constraints remain on.
- Expected result: exact entry despite newer unrelated Submissions; separate
  entry/current Context; null Submission legal; unsupported not mislabeled
  initial; required broken relation produces internal failure when a legal
  isolated fault seam can produce it.
- Failure condition: latest-row heuristic, visit-number/NULL heuristic,
  false mismatch on legal revision, or fabricated/replayed facts.

### ACC-CTI-003 — Old RETURN, later correction and exact older references

- Contracts: CTR-CTI-003, CTR-CTI-005, CTR-CTI-006.
- Method/environment: actual same-instance submissions with old RETURN followed
  by correction/QA/new-stage records; >8 submissions, >4 RETURN events, equal
  timestamps and a known correction outside the fixed window.
- Required evidence: deterministic selected IDs/order, truncation flags,
  explicit-ID retrieval, >16 related IDs and complete RETURN-body retrieval;
  foreign/missing UUID results and unchanged underlying facts.
- Expected result: recent correction is not replaced with oldest six; older
  known original is directly retrievable; windows and references remain honest.
  Reuse of IDs with wrong prose is still semantically wrong but not automatically
  judged by this read. New valid contrary evidence remains possible.
- Failure condition: silent history loss, cross-instance disclosure, implicit
  supersession/truth classification or claim that this case proves QA judgment.

### ACC-CTI-004 — Bounded bodies, UTF-8 and exact reconstruction

- Contracts: CTR-CTI-004, CTR-CTI-005.
- Method/environment: service HTTP with long current Context, long instructions,
  outgoing schemas, large Submission, escaping, multibyte Unicode and empty JSON
  values using the repository's existing canonicalizer.
- Required evidence: each wire response length, descriptor/offset sequence,
  concatenated exact bytes and SHA; comparisons with stored Submission digest;
  malformed offset and digest-corruption negative controls.
- Expected result: <=65536 wire bytes/call, <=8192 content bytes/body chunk,
  no split codepoint, truthful omitted/truncated flags, exact reconstruction;
  no client-side canonicalizer required.
- Failure condition: output truncation without metadata, hash mismatch accepted,
  offset loop, corruption hidden, or partial bytes called complete original.

### ACC-CTI-005 — Snapshot races, content drift and read failure

- Contracts: CTR-CTI-001, CTR-CTI-002, CTR-CTI-004, CTR-CTI-005.
- Method/environment: controlled concurrent isolated service reads with actual
  transition/revision, and a legitimate outgoing-target eligibility change
  between body chunks that leaves the expected triple unchanged.
- Required evidence: one-call snapshot coherence; stale-coordinate 409;
  same-coordinate/different-body 409; missing/full-view body errors; database
  interruption and recovery; no Receipt or business mutation on read failure.
- Expected result: no mixed chunks or stale-current claim; permissions checked
  again each call; storage failure remains unavailable, never empty success.
- Failure condition: multi-request atomicity claimed, stale data relabeled
  current, digest omitted/optional for body, or auth check bypass after first read.

### ACC-CTI-006 — Consumer parity and unattended-claim negative control

- Contracts: CTR-CTI-001 through CTR-CTI-006.
- Method/environment: after separately accepted consumer authority, actual DSH
  tool to isolated real service, two legal successor stages using the existing
  activation/dispatcher appropriate to each tested model; no new scheduler.
- Required evidence: exact consumer revision, tool input/output parity,
  dispatch/ingress and read provenance, returned current binding and real stage
  Submission, external-control cutoff/intervention ledger in test report only.
- Expected result: model does not choose a history window or invent returned
  coordinates; truncation/ref errors survive the tool boundary. Technical read
  conformance may pass independently of an honest unattended run failure.
- Failure condition: tool fabricates completion/semantic validity, manual rescue
  is counted as unattended PASS, a missing consumer is called integrated PASS,
  or this endpoint is substituted for delivery/semantic correctness evidence.

## 11. Alternatives and disposition

- RETAIN: current my_tasks/history/detail endpoints for their existing callers.
  They are supported read surfaces, not competing execution engines.
- Rejected: expose only history cursor. It makes known originals reachable but
  leaves page traversal and current-entry selection to the model.
- Rejected: Broker scans history and calls it one snapshot. Individual HTTP
  snapshots do not make an atomic cross-call current-input selection.
- Rejected: persist a second input/evidence ledger, rewrite Submission history,
  auto-suppress older RETURN findings or add a Codex reminder loop.
- Selected: one service-owned read projection plus bounded body view, sharing
  existing data and authorization; consumer integration remains separately owned.

## 12. Migration, compatibility, and rollback

No migration, backfill, changed Definition Version, current Context revision,
grant or model identity is required. Old endpoints remain unchanged; old servers
may lack this new path and must be reported unsupported by consumers, never
silently substituted with an oldest-page response labeled current input.

Deployment and production adoption require their existing separate authority.
Removing this additive endpoint leaves old reads and workflow execution intact;
consumers must fail honestly rather than synthesize completeness. No stored
packet or new lifecycle needs rollback. No existing serving runtime is claimed
to implement this proposed contract.

## 13. Open questions

`NORMATIVE_TBD=NONE`; `PARTIAL_SUPERSESSION=NONE`. Decisions above are the author's
bounded proposal for independent review and later delegated acceptance, not an
acceptance act. Exact Broker companion revision and implementation observations
are future integration coordinates, not permission to invent different wire
semantics. `AUTHORING_READY_FOR_REVIEW=YES`, `IMPLEMENTATION_ALLOWED=NO` while
proposed; `NEXT_ACTION=REVIEW`.
