---
spec_id: SVC_WORKFLOW_EXECUTION_CONTROL_V2_AMENDMENT_1_OUTBOX_CHANNELS_P0B
title: Outbox three-channel independence, kick failure classification and bounded dead-letter (P0-B)
status: proposed
spec_kind: amendment
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
repo: mayf3/svc-workflow
amends: SVC_WORKFLOW_EXECUTION_CONTROL_V2
date: 2026-11-03
revision: 1
candidate_base: origin/main 4ea514b
owners:
  - mayf3
acceptance_authority_basis: >-
  Pending Owner acceptance. Task authorization: P0-B user-direct task brief
  (2026-11-03, ZERO_HUMAN_OPERATIONS_V1) ordering the outbox three-channel
  split with explicit reliability requirements (§5 reliability: atomic claim,
  lease, bounded retry with backoff, failure classification separating auth
  errors from business errors, dead letter preserving the original request,
  observability). This document supplies the spec-side authority the
  implementation needs; acceptance is part of review, not assumed.
production_activation: out_of_scope
---

# SVC_WORKFLOW_EXECUTION_CONTROL_V2_AMENDMENT_1 — outbox channels (P0-B)

Bounded amendment to `SVC_WORKFLOW_EXECUTION_CONTROL_V2` (accepted). It does
not change any business-table semantics, any Workflow command, or the Agent
Core contract. It changes only how the svc-side outbox reconciler starts,
claims, classifies failures, and retires latency-only rows — plus one
additive storage column. PRODUCTION_APPLY_AUTHORITY remains none; activation
(reconciler start with kick credentials) is a later, separately authorized
operations round.

## A. Contract delta

### A.1 Channel independence (amends §3/§4 enablement)

The reconciler's channels are configured independently:

- `FORUM_EVENT` — unchanged env (`WORKFLOW_FORUM_SYNC_ENABLED` + credential
  trio). Absent/blank forum env with the master flag unset leaves the forum
  channel dormant.
- `EXECUTION_KICK` — new explicit flag `WORKFLOW_EXECUTION_KICK_ENABLED`
  (`1|true|TRUE` = on, other non-empty = off, absent = legacy nesting: the
  forum master switch enables the channel when `WORKFLOW_EXECUTION_KICK_URL`
  + `WORKFLOW_EXECUTION_KICK_TOKEN` are present, byte-compatible with V2).
- `OWNER_ASSISTANCE_WAKE` — new explicit flag
  `WORKFLOW_OWNER_ASSISTANCE_WAKE_ENABLED` with the same precedence rules;
  V2's wake env semantics otherwise unchanged (partial config retained as a
  sanitized fail-closed error, never a silent channel).

The loop starts when ANY channel is active and stays fully dormant (honest
startup line) when none is. A disabled channel never reads, claims, retries,
or settles its rows. A kick-only deployment therefore starts and delivers
kicks with no forum credentials present — this is the intended F1 fix
(production 2026-11: 1277 pending rows, attempt_count = 0).

### A.2 Kick failure classification (amends §3 `EXECUTION_KICK` bullet)

V2 §3 said: "mark `delivered_at` on 2xx/4xx (4xx = permanent …; only
5xx/network retry)". Amended classes for `POST /workflow-execution/kicks`
(Bearer token with `workflow.execute`):

- 2xx (`kicked`/`coalesced`) — delivered.
- 401/403 — authentication/authorization failure: retryable with the V2
  backoff. NEVER marked delivered (a token misconfiguration must not be
  confused with a successful delivery; the poll loop still owns correctness).
- 408/409/429/5xx — retryable with the V2 backoff. (The kicks contract has no
  409 semantics; a 409 is an unexpected condition, retryable and bounded by
  the cap below.)
- Transport failure/timeout — the delivery outcome is UNKNOWN; retryable.
  Replay is safe by the V2 kicks contract (a kick triggers one coalesced
  poll; kicks never stack; the one-attempt fence makes double triggers
  harmless).
- Every other 4xx (e.g. 400 `invalid_arguments`) — the receiver
  deterministically rejected the payload: permanent.

Every request carries the stable `Idempotency-Key` header (the outbox
`event_key`) for audit/replay correlation.

### A.3 Bounded dead-letter for the latency-only channel (amends §1 storage
and ACC-SWEC-009-004's general never-dead-lettered statement)

`workflow_outbox` gains an additive nullable column `dead_lettered_at`
(migration `0032_outbox_dead_letter.sql`; rows are still never deleted). Only
`EXECUTION_KICK` rows can be dead-lettered, in exactly two cases:

- a permanent 4xx per A.2 (immediately), or
- a retryable failure reaching `EXECUTION_KICK_MAX_ATTEMPTS = 12`.

A dead-lettered row retains `payload`, `attempt_count` and `last_error`,
keeps `delivered_at IS NULL`, and is excluded from the due queries. The
recovery entry is the documented requeue statement in the migration header
(`dead_lettered_at = NULL, attempt_count = 0, next_attempt_at = now()` after
the underlying cause is repaired). Requeue is an operator action, never an
automatic bulk replay; backlog isolation rules (no batch requeue of
historical rows) are inherited from the activation round's authority.

`FORUM_EVENT` and `OWNER_ASSISTANCE_WAKE` keep the V2 never-dead-lettered
correctness-path discipline (the marker can never become set for them by the
reconciler).

### A.4 Per-kind atomic claim (amends §3 batch)

Candidate batches are per-kind and rows are claimed with
`FOR UPDATE SKIP LOCKED` inside the delivery/ settle transaction: concurrent
workers cannot double-deliver, and the claim lock doubles as the lease (a
crashed worker's transaction dies with its connection — recovery is
immediate, not timeout-bound). Delivery failures settle in the same
transaction; every settlement statement carries a `delivered_at IS NULL`
guard so a concurrently retired row is never rewritten. Governance review
wakes (payload `phase` rows) run their pre-existing coordination transaction
BEFORE claiming, because it settles the same outbox row on retirement.

### A.5 Unknown kinds

Batches select only the three known kinds. A row of an unknown kind (written
by a newer binary, read by a downgraded one) stays visible in SQL but is not
drained (V2 marked such rows delivered; with per-kind batches that path no
longer exists).

## B. Acceptance criteria (this amendment)

- ACC-P0B-001: kick-only env starts the reconciler and delivers a queued kick
  with no forum configuration; forum facts stay untouched (tests/48
  `kick_only_delivers_without_any_forum_dependency`).
- ACC-P0B-002: concurrent workers deliver each event exactly once
  (tests/48 `concurrent_workers_never_double_deliver`).
- ACC-P0B-003: 401/403 are classified as auth failures and never delivered
  (tests/48 `kick_401_and_403_are_auth_failures_never_delivered`).
- ACC-P0B-004: the cap dead-letters at attempt 12 with payload retained, the
  documented requeue recovers the row, and dead-lettered rows are invisible
  to the drain (tests/48 `kick_dead_letters_at_the_retry_cap_and_recovers_via_requeue`).
- ACC-P0B-005: a permanent 4xx dead-letters immediately with the original
  payload retained (tests/48 `kick_400_dead_letters_immediately_with_payload_retained`).
- ACC-P0B-006: a forum outage does not block the kick channel in the same
  pass (tests/48 `forum_api_outage_does_not_block_kick_channel`).
- ACC-P0B-007: legacy env (master switch + nested URLs) behaves
  byte-compatibly with V2 (forum_sync unit tests
  `legacy_forum_master_switch_nesting_still_enables_kick_and_wake`,
  `explicit_kick_disable_wins_over_legacy_nesting`).
- ACC-P0B-008: an UNKNOWN transport outcome schedules a retry with a stable
  request identity instead of being marked delivered
  (tests/48 `transport_unknown_result_schedules_retry_and_carries_stable_request_id`).

## C. Out of scope

Production activation, credentials, `.env` changes, historical backlog
disposition, Agent Core changes. Business tables, commands, receipts, and
events are unchanged.
