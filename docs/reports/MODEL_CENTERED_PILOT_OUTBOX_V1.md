# Model-centered pilot: three-channel durable outbox

**Status:** requirements/model/code mapping prepared; Lean kernel verification and consumer PR CI pending.
**Scope:** read-only pilot of the accepted three-channel outbox semantics. This report is not a Product Contract, production migration authorization, deployment authorization, configuration change, or backlog activation.

## 1. Exact source and authority

- Consumer source snapshot inspected: `main@c50ab2ac98decceaa26138a282c34dd92ede0ddc`.
- Governing accepted authority: `SVC_WORKFLOW_EXECUTION_CONTROL_V3` (accepted technical semantics; production apply remains none).
- Existing acceptance criteria: `ACC-P0B-001..010` in that Spec.
- Implementation: `src/forum_sync.rs`, `src/store/postgres/outbox.rs`, `migrations/0032_outbox_dead_letter.sql`.
- Test surfaces: `tests/48_outbox_three_channels.rs` and focused unit tests in `src/store/postgres/outbox.rs`.
- Shared model candidate: `mayf3/primitives` PR #12, module `AgentWorkflow.Outbox`. Kernel verification remains pending until the pinned Lean workflow passes.
- Historical evidence: `RECEIPT-1359-OUTBOX.md` and `REVIEW-1359-OUTBOX.md` are historical R5 evidence with explicitly limited scope; this pilot does not reinterpret their delivery or merge status.

No accepted Spec is changed. No production DB, migration, runtime, config, credentials, backlog, or service state was accessed or changed.

## 2. Requirements → model → Lean → code → evidence

| Requirement / accepted clause | Model property | Lean declaration | Implementation mapping | Existing evidence surface |
|---|---|---|---|---|
| Disabled channel never reads/claims/retries/settles its rows (V3 Delta 1; ACC-P0B-001/007) | No row transition is possible when its channel gate is false | `Outbox.disabled_channel_has_no_transition` | `forum_sync.rs:250-340` builds independent channel configuration; `run_once` / `drain_channel` only drain configured kinds | `tests/48_outbox_three_channels.rs:882,911`; kick-only and all-three-channel cases |
| Event identity remains stable across attempts (Delta 2; ACC-P0B-008) | Transition preserves the event key | `Outbox.step_preserves_event_key` | `outbox.rs:55-120` writers use stable activation / assistance keys; kick request carries the event key as Idempotency-Key | `transport_unknown_result_schedules_retry_and_carries_stable_request_id` at test line 1213 |
| Concurrent workers cannot hold the same active claim simultaneously (Delta 4; ACC-P0B-002/009) | A claimed row cannot be claimed again before release | `Outbox.claimed_row_cannot_be_claimed_again` | `outbox.rs:195-212` uses `FOR UPDATE OF o SKIP LOCKED`; settlement occurs under the caller transaction | `concurrent_workers_never_double_deliver` line 1127; crashed-worker lock-release test line 1178; one-connection pool tests |
| UNKNOWN is retry responsibility, not successful settlement (Delta 2; ACC-P0B-008) | UNKNOWN returns the row to pending, never delivered | `Outbox.unknown_returns_to_pending` | `mark_attempt_failed` increments attempts and schedules backoff; delivered is a separate guarded update | UNKNOWN stable-request test line 1213; restart preserves pending rows line 1267 |
| Only latency-only EXECUTION_KICK can dead-letter (Delta 3; ACC-P0B-004/005) | A transition to dead-letter requires channel = executionKick | `Outbox.dead_letter_is_kick_only` | `outbox.rs:235-297` applies bounded retry / explicit dead-letter; migration 0032 retains payload and provides operator requeue statement | retry-cap/requeue line 1027; permanent 400 line 1092; payload retention unit test in outbox.rs line 690 |
| One channel's slow/failing work does not stall other channels (Delta 1/ACC-P0B-006) | Per-row model does not establish cross-channel scheduling fairness | No Lean theorem claimed | `forum_sync.rs:1214-1308` starts channel-local workers; each worker owns its own pass/tick | sustained slow-channel tests lines 668-679 and same-pass outage test line 843 |
| Forum events remain oldest-first per workflow instance (Delta 4; ACC-P0B-010) | Not represented in the per-row Lean slice | No Lean theorem claimed | `outbox.rs:162-190` candidate query excludes a Forum row while any earlier undelivered Forum row exists for the same instance | unit test `backed_off_forum_event_holds_later_event_for_same_instance` line 401; two-worker ordering test line 1715; backoff hold-back test line 1802 |
| Unknown outbox kinds remain visible and untouched (Delta 5) | Not represented in the per-row transition slice | No Lean theorem claimed | Kind-scoped batch selects only the requested known kind; migration check constrains current writers | unit test `unsupported_kind_stays_pending_and_retryable` in forum_sync.rs line 1564 |

## 3. Abstract row model

A row is modeled as:

```text
(eventKey, channel, status, claimant, attempts)

pending --claim(worker, channel enabled)--> claimed(worker)
claimed(worker) --settle--> delivered
claimed(worker) --retry / UNKNOWN below kick cap--> pending, attempts + 1
claimed(worker) --UNKNOWN at kick cap or permanent rejection, EXECUTION_KICK only--> deadLetter
```

The abstract model intentionally does not claim:
- exactly-once external side effects; the accepted contract is at-least-once with stable idempotency identities;
- PostgreSQL's actual lock / MVCC behavior;
- SQL transaction atomicity or crash recovery;
- HTTP receiver truthfulness or remote idempotency;
- global channel fairness or the Forum per-instance anti-overtake property.

## 4. Implementation mapping details

- **Transactional writers:** `outbox.rs:19-123` queues Forum, kick and Owner Assistance facts in the caller's business transaction. The outbox is a delivery projection, not the authority for workflow business state.
- **Eligibility/order:** `outbox.rs:162-190` selects one kind, due, not delivered, not dead-lettered; the Forum-only NOT EXISTS predicate prevents an earlier undelivered event for the same instance from being overtaken.
- **Exclusive claim:** `outbox.rs:195-212` performs `FOR UPDATE OF o SKIP LOCKED` and rechecks due predicates under lock.
- **Settlement:** `outbox.rs:215-233` marks delivered only under `delivered_at IS NULL`; `235-264` records retry/backoff and optional cap; `266-297` handles immediate dead-letter.
- **Channel isolation:** `forum_sync.rs:870-984` drains a kind; `1214-1308` constructs independent workers for EXECUTION_KICK, FORUM_EVENT and OWNER_ASSISTANCE_WAKE. Each worker reuses the same bounded ordered pass semantics with only its own channel enabled.
- **Recovery boundary:** migration `0032_outbox_dead_letter.sql` retains dead-lettered row payload and error; the requeue statement is an operator action after cause repair, not automatic delivery machinery.

## 5. Counterexamples and existing test evidence

The current source contains a substantial integration suite. This pilot inspected the test definitions but did not execute them:

- `tests/48_outbox_three_channels.rs:785-910`: kick-only operation, Forum outage isolation, disabled kick/wake non-interference.
- `tests/48_outbox_three_channels.rs:945-1092`: retryable 5xx, auth 401/403, retryable 409, cap dead-letter/requeue, permanent 400.
- `tests/48_outbox_three_channels.rs:1127-1267`: concurrent no-double-delivery, crash releasing row lock, UNKNOWN stable request ID, restart retention. The new test `kick_unknown_at_retry_cap_dead_letters_with_payload_retained` seeds attempt_count=11, induces a transport UNKNOWN and checks attempt 12 dead-lettering with original payload retained.
- `tests/48_outbox_three_channels.rs:1361-1476`: one-connection pool and all-three-channel delivery.
- `tests/48_outbox_three_channels.rs:1715-1866`: concurrent Forum ordering and backed-off earlier event blocking later events.
- `src/store/postgres/outbox.rs:400-780`: focused SQL-level ordering, kind scoping, claim recheck, retry-cap, payload retention and concurrent-backend tests.

**Execution status:** no Rust test command was run in this GitHub-only pilot session. The consumer PR CI must run the focused test target and any required repository checks at its exact head before these are marked executed PASS.

## 6. Proof boundary and gaps

| Claim | Current evidence class | Remaining obligation |
|---|---|---|
| Disabled row cannot transition; event key is invariant; only kick dead-letters; UNKNOWN is not delivered; claimed row cannot be claimed again in the abstract model | Lean model source; pending kernel run | Pinned Lean 4.19.0 build + compiled theorem/axiom audit in primitives PR #12 |
| SQL claim is exclusive and settlement is guarded | Source mapping + integration/unit test definitions | Run the exact Rust tests at the candidate head; inspect transaction commit/rollback and lock release evidence |
| Forum ordering is preserved under concurrent workers and backoff | SQL anti-overtake predicate + test definitions | Execute two-worker ordering and backoff tests against isolated PostgreSQL |
| Slow channel does not block other channels | Separate worker implementation + sustained tests | Execute the three sustained slow-channel tests and retain timing output |
| External delivery is at-least-once and idempotently reconciled | Stable event key + retry model + source tests | Receiver-side idempotency and crash-after-remote-effect-before-local-settlement remain external assumptions |
| Production behavior / backlog disposition | Not established | Separate authorized deployment, migration, activation and real operational evidence remain required |

Important semantic distinction: the Lean row model's exclusive claim theorem is a property of its transition relation. It does not prove that PostgreSQL implements that relation. Likewise, the SQL lock and tests do not establish exactly-once external effects; the contract remains at-least-once, and stable idempotency keys / receiver semantics are essential.

## 7. Pilot conclusion

**Model fit:** PASS for the narrow per-row channel / claim / UNKNOWN / dead-letter slice, subject to Lean CI.
**Implementation mapping:** identified for channel config, SQL selection, claim, settlement, retry, dead-letter and forum ordering.
**SQL/PostgreSQL refinement:** NOT PROVED by Lean; test execution pending.
**Production / migration / backlog / unattended acceptance:** NOT RUN / NOT AUTHORIZED / NOT CLAIMED.

The highest-value result is a clean separation between properties that are generic state-machine invariants and properties that are specific to PostgreSQL concurrency, cross-channel scheduling, and remote HTTP effects. The latter must not be mislabeled as Lean-proven merely because the abstract row model compiles.
