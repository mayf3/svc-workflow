# BRIEF-1359-OUTBOX — R2 corrected Svc work surface

Authorization: original user mandate preserved in `/private/tmp/h1359-worker.log:15-32`, renewed by the R2 user instruction to restart all tasks from task 1. Owner increment 3 permits #1359 on an independent Svc surface subject to capacity. No production writes. Deliver the scoped fix on `fix/outbox-channel-isolation`, push a PR against `main`, never merge or rebase. Unexpected failure or load-bearing SPEC_GAP stops dependent work and writes `STOPPED-1359-OUTBOX.md`. The explicitly requested RED is an expected test failure.

## PREFLIGHT

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
TARGET_REPOSITORY = mayf3/svc-workflow
BASE_HEAD = 8da8dbcb70e700d9e35970e8b662b1be2c29afdc
REVIEW_TARGET_HEAD = reported in receipt after implementation commit
CURRENT_BASE_HEAD = 8da8dbcb70e700d9e35970e8b662b1be2c29afdc (origin main read 2026-11-07 UTC)
ROUTE_STAGE = IMPLEMENTATION
AUTHORITY_ACCEPTED_IN_BASE = YES
GOAL_OR_TARGET = healthy outbox channels progress while another channel times out
CURRENT_GAP = global binding ensure and serial channel awaits couple delivery latency
AUTHORITY_ACTION = REUSE
PRIMARY_AUTHORITY = SVC_WORKFLOW_EXECUTION_CONTROL_V3 at BASE_HEAD
RELATED_AUTHORITIES = Product V13; Architecture V0_4_7; Owner Obligation V3 at BASE_HEAD
IMPLEMENTATION_AUTHORITY = contracts
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = NO
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = DURABLE
EXECUTION_MANDATE = VALID
MUTATION_AUTHORIZATION = VALID
ISOLATED_WRITE_SURFACE = YES (/private/tmp/h1359-outbox; exact base above; single writer)
CONTROLLED_RUNBOOK_REQUIRED = NO (source and disposable fixtures only)
SPEC_GAP_DEPENDENCY = NONE for the bounded worker structure below
EVIDENCE_REVIEWABILITY = PASS (repository evidence and reproducible disposable PG tests)
LIVE_AUTHORITY_GAP = NONE (no live-state access or changes)
OWNER_DECISION_REQUIRED = NO
EMERGENCY_STATE = NONE
BASE_IMPACT = BOUNDED
IMPLEMENTATION_ALLOWED = YES
MERGE_READY = NO (validation and independent review pending; self-merge forbidden)
OPERATION_ALLOWED = NO (production)
NEXT_ACTION = CONTINUE
```

Governance adoption is accepted, present in this main snapshot, and its vendored-byte check passed. Source coordinates from issue #1359 and the older matrix have been rebound: `src/forum_sync.rs` is byte-identical at `d4c510d9` and the R2 base, SHA-256 `cad3a653abe40ca0fac122112891230a12abf22a5fba9e24dd1f6786ca8e9f95`. Actual locations: HTTP limits 423-438, thread ensure 806-853, serial pass 865-898, drain/preparation 902-972, claim/delivery/settlement 1005-1067, supervised global loop 1206-1233. The issue's approximately 200 seconds is a conditional estimate until measured.

## Contract-preserving implementation boundary

V3 Delta 4, lines 165-178, requires each pass to process enabled kinds in KICK, FORUM, WAKE order; lines 163 and 168-196 expressly support concurrent workers. Keep `run_once`, `drain_channel`, the candidate query, claim, same-instance anti-overtake guard, HTTP timeouts, settlement transaction, stable keys, backoff and dead-letter behavior unchanged. Inside the existing reconciler, project configuration into at most three channel-only workers. Each worker uses the existing pass and interval: its enabled kinds remain an ordered subsequence. This adds no Scheduler, middleware, outbox or durable scheduling truth. A channel's finite HTTP/batch work is awaited only by that channel. No time-budget cancellation of a claimed delivery is introduced. Shared database capacity is still a physical constraint, not a promised isolation from pool exhaustion.

Independent read-only review by agent `/root/authority_check` confirmed this route before implementation. Implementation still requires independent affected-contract review at an exact candidate.

## Execution and stop boundary

1. Add a reproducible RED against unmodified product source: 20 pending Forum bindings, real 10-second HTTP timeouts, normal loopback kick/wake receivers, isolated temporary PG. Record actual endpoint and durable-settlement timing. Fixture Forum event rows are deferred to avoid adding a second 20-request phase.
2. Implement only channel-local polling/supervision within the existing reconciler.
3. Run the same probe GREEN, cover slow kick and slow wake, sustained healthy-channel progress, enabled/disabled behavior, concurrent claims and same-instance order through existing coverage.
4. Run the repository cargo suite with explicit isolated `TEST_DATABASE_URL`, retain raw commands/output/return codes and independent review.
5. Write `RECEIPT-1359-OUTBOX.md`, push scoped commits and open PR against main. No merge, rebase, governance changes, production access or backlog consumption.

`EVIDENCE_NEEDED`: RED/GREEN raw logs with source/test coordinates and durations; relevant existing suite rc0; full cargo suite rc0; independent review; PR and exact final head.

`DONE_WHEN`: scoped source fix, measurable healthy-channel progress under a slow/failing peer, preserved ordering/claim/idempotency evidence, existing tests rc0, review and pushed unmerged PR with receipt.

`EXPANSION_TRIGGER`: accepted claim/lock/order/identity semantics would change; unrelated repository or machine state is needed; any unexpected validation failure. Stop and record exact delta/failure. Do not rewrite governance or keep repairing unrelated gates.

`NEXT_REAL_ACTION`: execute the RED timing fixture on the unchanged base product source.
