---
spec_id: SVC_WORKFLOW_INSTANCE_COLLABORATION_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: none
scope:
  - mayf3/svc-workflow
  - workflow-instance-collaboration-side-band-v1
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V9
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_2
  - SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
title: Workflow Instance Collaboration V1 — append-only side-band entries and unified feed
repo: mayf3/svc-workflow
base_head: ed99fa06a3067fe1d230699e9fed2b19542ab190
date: 2026-09-15
production_apply_authority: none
product_code_changed_by_this_spec_pr: false
owner_ruling_input: ACCEPT_WITH_SIMPLIFICATION_WORKFLOW_INSTANCE_COLLABORATION_V1_2026_09_15
---

# SVC_WORKFLOW_INSTANCE_COLLABORATION_V1

> **Proposed-stage metadata note:** this Spec is a candidate on a
> docs-only branch. It carries `implementation_authority: none`; the
> acceptance transaction (after independent semantic review and Owner
> exact-head acceptance) flips it to `contracts` and adds the
> acceptance-transaction fields, per repository precedent
> (`SVC_WORKFLOW_INVALID_RETURN_REFERENCES_HTTP_422_V1` revision note,
> `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1`).

## 0. Problem and Owner ruling

Workflows experience RETURNs, repeated rejections, role handover, and
Agent Session restarts. Today a fresh assignee recovers context only by
re-assembling instance detail, timeline, submission history, and
assistance reads across several surfaces, and has no place to read or
leave durable clarifications. The Owner ruling
`ACCEPT_WITH_SIMPLIFICATION_WORKFLOW_INSTANCE_COLLABORATION_V1`
(2026-09-15) selects an independent, kind-free, append-only side-band
Collaboration model with a unified feed, and fixes the product decisions
PD-1..PD-6 exactly as restated in §2 of the parent Product Direction
exception (`SVC_WORKFLOW_PRODUCT_BOUNDARY_V9` §V9.1 OBS-V9-003). This
Spec is the implementation contract layer for that ruling. It changes no
existing workflow semantics.

Fresh-base facts this Spec builds on (all at `ed99fa0`):
`workflow_command_receipts` idempotency machinery is command-type
generic; `workflow_events` enforces `event_sequence =
new_workflow_state_version` and `new = old + 1`, structurally binding
events to state progression; the query path classifies visibility into
`DomainOwnerFull | CurrentAssigneeFull | CreatorDraftFull |
HistoricalParticipantRestricted`; every cross-family reference in the
runtime schema uses the `(referenced_id, workflow_instance_id)`
composite-key discipline.

## 1. Authority audit

- `SVC_WORKFLOW_PRODUCT_BOUNDARY_V9` (proposed parent): product
  semantics, authorization rulings, non-goals.
- `SVC_WORKFLOW_ARCHITECTURE_V0_4_2` (proposed parent): side-band fact
  family, invariants, projection semantics; explicitly delegates names,
  DDL, and wire to this layer.
- Legacy `docs/contracts/WORKFLOW_QUERY_CONTRACT_V0_1.md`: visibility
  class wire names; unchanged by this Spec.
- `contracts/workflow-http/v1` bundle (1.7.0 at base): envelope, error,
  cursor, idempotency conventions extended additively by this Spec.
- No accepted authority currently names any collaboration concept; the
  two proposed parents above are the required authority for this Spec.

This Spec owns exactly: storage names and constraints, HTTP surface,
DTOs, error codes, feed projection shape, pagination, idempotency
envelope, and acceptance tests for the Collaboration scope.

## 2. Normative contracts (authorized upon acceptance)

### CTR-1 — Side-band storage family
A single table `workflow_collaboration_entries` stores immutable,
kind-free, instance-scoped entries. Columns:

```text
collaboration_entry_id        UUID PK
workflow_instance_id          UUID NOT NULL → workflow_instances
author_principal_id           UUID NOT NULL → principals
body                          TEXT NOT NULL, CHECK char_length 1..16384
reply_to_entry_id             UUID NULL (same-Instance self reference)
observed_node_visit_id        UUID NOT NULL (same-Instance Visit snapshot)
observed_workflow_state_version INTEGER NOT NULL CHECK (>= 1)
related_event_id              UUID NULL (same-Instance Event)
related_submission_id         UUID NULL (same-Instance Submission)
related_assistance_case_id    UUID NULL (same-Instance Assistance Case)
command_id                    UUID NOT NULL UNIQUE → workflow_command_receipts
created_at                    TIMESTAMPTZ NOT NULL DEFAULT now()
UNIQUE (collaboration_entry_id, workflow_instance_id)
INDEX (workflow_instance_id, created_at, collaboration_entry_id)
```

No `kind` column exists. `observed_*` and `author_*` are server-authored
only.

### CTR-2 — Storage-level same-Instance integrity and immutability
Every optional reference uses the composite-key discipline via foreign
keys on `(reply_to_entry_id, workflow_instance_id)` →
`workflow_collaboration_entries`, `(observed_node_visit_id,
workflow_instance_id)` → `workflow_node_visits`, `(related_event_id,
workflow_instance_id)` → `workflow_events`, `(related_submission_id,
workflow_instance_id)` → `workflow_submissions`,
`(related_assistance_case_id, workflow_instance_id)` →
`workflow_assistance_cases` (each referencing the family's
`(id, workflow_instance_id)` unique key; DEFERRABLE INITIALLY DEFERRED
where the referenced family requires it). A `BEFORE UPDATE OR DELETE`
trigger raises `23000` (house immutability rule). A `BEFORE INSERT`
trigger fails closed when the Instance is archived or the author
principal is disabled. Cross-Instance references are structurally
impossible.

### CTR-3 — Append is a non-state idempotent command
`command_type = APPEND_COLLABORATION_ENTRY`. Request hash = JCS
canonicalization of `{commandSchemaVersion, commandType,
routeParameters: {workflowInstanceId}, requestBody}` with SHA-256
(excluding the idempotency key). Same `(principal_id, idempotency_key)`
+ same hash → replay original outcome; different hash → 409
`idempotency_conflict`, zero mutation; in-flight → 425
`command_still_processing`. Deterministic failures complete their
receipts for stable replay. `UNIQUE (command_id)` makes one command at
most one Entry. The transaction takes no expected-version input, does
not lock the Instance row in the state-command serialization position,
and writes no row in `workflow_instances` or `workflow_events`. The
append MUST fail closed against a concurrently committed archive (a
non-serialization re-check such as a shared lock or an insert-time
trigger recheck is permitted and required for this; it does not occupy
the state-command serialization position). Audit coverage follows the
carried architecture CTR-ARCH-039 discipline: the append is a protected
write whose receipt and attempt audits commit atomically with it, and
unauthorized feed reads take the existing unauthorized-read audit path
used by instance reads.

### CTR-4 — Server-authored snapshot
Within the append transaction the server reads the Instance's current
`current_node_visit_id` and `workflow_state_version` and stores them as
`observed_node_visit_id` / `observed_workflow_state_version`. These are
advisory observation records; no command validates against them as
concurrency predicates and no projection consumes them as state. If the
Instance structurally has no current Visit at append time (not
reachable through any current create path), the append fails closed
with a deterministic 409 rather than writing a null snapshot.

### CTR-5 — Write authorization (PD-2/PD-3/PD-4)
Append is permitted iff the authenticated Principal is enabled and holds
at least one of: enabled Domain Owner binding for the Instance's
Domain; `created_by_principal_id` equality; equality with the current
Visit's assignee; Historical Participant (the existing classification:
creator, past Visit assignee, or past Submission author of this
Instance). The Instance must not be archived. TERMINAL and CANCELLED
Instances that are not archived remain writable. Violations:
not-visible → 404 `workflow_instance_not_found_or_not_visible`;
visible but no write relation → 403
`collaboration_write_forbidden`; archived → 409 `instance_archived`;
disabled principal → 403 `principal_disabled`. Evaluation precedence
after DTO validation and the receipt gate: 404 visibility, then 403
`principal_disabled`, then 409 `instance_archived`, then 403
`collaboration_write_forbidden`, then 422 reference validation.

### CTR-6 — Reference validation (HTTP layer)
`replyToEntryId`, `relatedEventId`, `relatedSubmissionId`,
`relatedAssistanceCaseId` must each resolve to an existing row of the
same Instance. Violations aggregate into one 422
`invalid_collaboration_references` response whose `details.detail`
names every failing field and reason (pattern of accepted
`SVC_WORKFLOW_INVALID_RETURN_REFERENCES_HTTP_422_V1`), replaying
idempotently like any deterministic failure. DB composite keys remain
the second, structural enforcement layer.

### CTR-7 — Unified Collaboration Feed projection
`GET /internal/v1/workflow-instances/{workflowInstanceId}/collaboration`
returns one stable, chronologically ascending stream of items:

```text
itemType = COLLABORATION_ENTRY | WORKFLOW_FACT
```

`COLLABORATION_ENTRY` items expose: `entryId`, `authorPrincipalId`,
`body`, `replyToEntryId?`, `observedNodeVisitId`,
`observedWorkflowStateVersion`, `relatedEventId?`,
`relatedSubmissionId?`, `relatedAssistanceCaseId?`, `commandId`,
`createdAt` (camelCase DTO).

`WORKFLOW_FACT` items expose the minimal collaboration-relevant subset,
projected directly from canonical rows — never a copy:

```text
factType ∈
  SUBMISSION_COMMITTED     (submission_id, source_visit, author,
                            payload per CTR-8 visibility)
  RETURN                   (event_sequence, root_cause_visit,
                            reason_code, reason, related_submission_ids
                            — derived from the canonical Submission
                            payload of the RETURN transition)
  TRANSITION_TERMINAL      (TERMINAL-entry transition facts)
  INSTANCE_CANCELLED       (canonical cancel metadata)
  ASSISTANCE_LIFECYCLE     (case id, status change, timestamp; no
                            assistance payload bodies)
```

Fact-item mapping is strictly row-per-row: event-derived facts
(`RETURN`, `TRANSITION_TERMINAL`, `INSTANCE_CANCELLED`,
`ASSISTANCE_LIFECYCLE`) emit one item per canonical `workflow_events`
row with `created_at` = that row's timestamp and `item_id` = its
`event_id`; `ASSISTANCE_LIFECYCLE` covers exactly the
`ASSISTANCE_REQUESTED` / `ASSISTANCE_ESCALATED_TO_HUMAN` /
`ASSISTANCE_RESOLVED` / assistance-void event rows;
`SUBMISSION_COMMITTED` emits one item per canonical Submission row with
`created_at` = the submission's timestamp and `item_id` = its
`submission_id`, excluding Submissions already surfaced as `RETURN`
facts (one commit never yields two items).

Every fact item's payload visibility follows that fact's own read
authority (existing timeline/submission/assistance row rules). Entries
are visible to every Principal holding instance visibility. Failed or
rolled-back commands never appear. `INSTANCE_CREATED`, `WAKE_*`,
`CONTEXT_REVISED`, archive events, and administrative events are
excluded (minimal-subset rule).

### CTR-8 — Feed read authorization (PD-1/PD-5)
Feed access requires legal instance visibility under the existing
classification; the response carries `visibility:
"full" | "historical_participant"` exactly like instance detail.
Historical Participants receive complete `COLLABORATION_ENTRY` items;
their `WORKFLOW_FACT` payload visibility is identical to what the
existing timeline/submissions/assistance surfaces already grant them.
Referencing a fact never grants its payload
(`REFERENCE_DOES_NOT_GRANT_ACCESS`). No visibility → 404
`workflow_instance_not_found_or_not_visible`.

### CTR-9 — Pagination and ordering
Total order: `(created_at, source_rank, item_id)` ascending, where
`source_rank(WORKFLOW_FACT) = 0 < source_rank(COLLABORATION_ENTRY) = 1`
and `item_id` is the fact's `event_id` / entry's
`collaboration_entry_id`. Keyset continuation: paired query parameters
`afterCreatedAt` (RFC 3339) + `afterItemType` + `afterId`, all present
or all absent; half-present or malformed → 422 `invalid_cursor`.
`limit` default 50, maximum 100 (larger or non-positive → 422
`invalid_pagination`, matching the accepted keyset-continuation
precedent).
Response envelope: `{visibility, latestWorkflowStateVersion, items,
nextCursor}` with `nextCursor = {createdAt, itemType, id} | null`.

### CTR-10 — HTTP surface and wire
Routes (under the existing `/internal/v1` prefix, JWT + scope rules):

```text
POST /internal/v1/workflow-instances/{workflowInstanceId}/collaboration/entries
     scope workflow.execute, Idempotency-Key required (house rules),
     canary write guard like other writes
GET  /internal/v1/workflow-instances/{workflowInstanceId}/collaboration
     scope workflow.read
```

POST body (camelCase, `deny_unknown_fields`, unknown field → 422
`invalid_input`): `body` (required), `replyToEntryId?`,
`relatedEventId?`, `relatedSubmissionId?`,
`relatedAssistanceCaseId?`. The body MUST NOT contain `kind`, `author`,
`authoredNodeVisitId`, `observedNodeVisitId`, or
`observedWorkflowStateVersion` fields; any attempt is an unknown-field
rejection. Success: 201 with the full entry DTO (CTR-7 fields) plus
`replayed` flag on idempotent replay. The name `context-entries` is
forbidden (Product Direction ruling).

### CTR-11 — Single source of truth for formal facts
RETURN `reasonCode`/`reason`/`relatedSubmissionIds` are read from the
canonical Submission payload at feed time; no Entry or feed field
duplicates them as authority. Assistance payloads are never copied into
Entries or the feed. A rejected RETURN (422
`invalid_return_references`), a rolled-back transition, or any
uncommitted command never produces any Collaboration item.

### CTR-12 — Migration and rollout
One additive migration creates the table, constraints, triggers, and
indexes; the number is chosen at implementation time from the then-fresh
migration ledger (no number is pre-allocated by this Spec). No backfill,
no change to existing tables, `EXPECTED_MIGRATION_VERSION` and
`/version` `schemaVersion` advance accordingly; readiness gating
(`migration_version_mismatch`) applies. The HTTP contract bundle gains
a minor entry (changelog, `openapi.yaml` paths/schemas,
`errors.json` codes, fixtures) with digest re-verification
(`verify-digests.sh`); existing routes, error codes, and envelopes are
unchanged. Production apply remains a separate gate.

### CTR-13 — Agent handoff protocol (consuming surface)
The supported recovery sequence for a fresh Agent Session is:
(1) read instance detail; (2) read the Collaboration Feed from the
beginning (or from a persisted cursor); (3) derive prior RETURN causes,
clarifications, prior submission content, and remaining work; (4)
re-read feed increments after finishing work and before submitting;
(5) execute the formal transition with
`expectedWorkflowStateVersion` under existing rules. Collaboration
participates in no concurrency control and triggers no wake, dispatch,
or transition. This protocol is an Agent-side convention documented by
this Spec; it creates no scheduler obligation in svc-workflow.

## 3. Error catalogue (Collaboration scope)

| HTTP | code | condition |
|---|---|---|
| 404 | `workflow_instance_not_found_or_not_visible` | no instance visibility |
| 403 | `forbidden` | missing scope |
| 403 | `principal_disabled` | disabled author |
| 403 | `collaboration_write_forbidden` | visible, no write relation |
| 409 | `instance_archived` | append on archived instance |
| 409 | `idempotency_conflict` | same key, different request |
| 422 | `invalid_input` | body bounds, bad UUIDs, unknown fields |
| 422 | `invalid_pagination` | limit misuse on the feed |
| 422 | `invalid_collaboration_references` | same-Instance reference violations, aggregated `details.detail` |
| 422 | `invalid_cursor` | half-present/malformed cursor triple |
| 425 | `command_still_processing` | receipt PROCESSING |

`collaboration_write_forbidden` and `invalid_collaboration_references`
are new registry entries; all others are existing codes reused
unchanged.

## 4. Acceptance scenarios

### 4.1 Handoff scenario (end-to-end, real TCP)

1. Writer Agent submits S1.
2. Reviewer executes RETURN #1 (`reasonCode`/`reason` = missing
   original source).
3. Writer appends Entry: "官方博客是否可以？"
4. Reviewer replies (replyToEntryId): "可以，但需要发布日期。"
5. Writer submits S2.
6. Reviewer executes RETURN #2 (second evidence insufficient).

Then the Writer Agent Session is completely replaced. The new Session,
using only svc-workflow APIs, must recover: why RETURN #1 happened;
what both sides clarified; what S2 delivered; why RETURN #2 happened;
what remains to be done. All five must be derivable from instance
detail + Collaboration Feed alone, with the feed's RETURN reasons
byte-derived from canonical Submission payloads.

### 4.2 Mechanical invariants

```text
M1: 100 Entries appended → workflow_state_version unchanged AND
    workflow_events row count unchanged.
M2: same idempotency key + same request retried (including after
    server restart) → exactly one Entry; replay returns the original
    201 response.
M3: same key + changed request → 409 idempotency_conflict, zero
    mutation.
M4: reply/reference targeting another Instance's entry/event/
    submission/assistance → 422 invalid_collaboration_references
    (aggregated detail) AND storage-level composite keys reject any
    bypass.
M5: request bodies containing kind/author/observed-* fields →
    422 invalid_input (unknown field); author is always token.sub.
M6: Historical Participant reads all Entries but cannot read a
    Submission payload it could not already read (feed fact item
    omits payload; direct submission read still filtered).
M7: rejected RETURN (422 invalid_return_references) produces no
    Collaboration item of any kind.
M8: append on TERMINAL-not-archived and CANCELLED-not-archived →
    201; after archive → 409 instance_archived; feed remains
    readable after archive.
M9: feed pagination walks the full merged stream with the cursor
    triple without skips or duplicates, including across timestamp
    ties between facts and entries.
M10: WAKE/CONTEXT_REVISED/INSTANCE_CREATED events never appear as
    feed items.
```

### 4.3 Test placement

Integration tests against real PostgreSQL for M1-M8 (asserting version
and event counts), plus real-TCP e2e for §4.1 and for 422 detail
exposure (pattern of the return-422 e2e). All tests are additive; no
existing test expectation changes.

## 5. Explicit non-goals

No edit/delete/void/reaction/mention/search/realtime/attachment/
cross-instance-thread/AI-summary-authority/auto-wake/auto-transition/
scheduler construct; no `kind` taxonomy; no Assistance semantic change;
no Context semantic change; no visibility escalation; no UI delivery;
no pre-allocated migration number.

## 6. Conformance and review hooks

Implementation PR must record Contract-by-Contract evidence against
CTR-1..CTR-13, cite test names for M1-M10 and §4.1, re-run the bundle
digest verification, and update the goal coordination record. Any drift
between this Spec and the implemented behavior is reported as drift,
not excused by editing this Spec after acceptance.
