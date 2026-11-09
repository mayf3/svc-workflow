# #1359 outbox channel isolation — R3 receipt

STATUS: STOPPED / INCOMPLETE / REQUIRED_GATE_FAILURE. Attribution and
focused gate complete; full cargo returned rc101. No R3 commit, push or PR.
This records an incomplete delivery, not a completion PASS.

## Coordinates and authority

- Repository/worktree: `mayf3/svc-workflow`, `/private/tmp/h1359-outbox`.
- Branch: `fix/outbox-channel-isolation`; PR target: `main`; never self-merge.
- Integration base: `8da8dbcb70e700d9e35970e8b662b1be2c29afdc`.
- RED fixture: `0cbb7f6cc2c68b114cc493712eb98699ad660393`.
- Product implementation: `062fe82301eadbf23b5b6279643355c20ca4218c`.
- R2 checkpoint preserved: `8aa864ab9ff70107faf8e016a04d32f4e78ad141`.
- R3 tested product SHA-256: `d78eb6f6c09c4a7849f2fc218c48543bddf476e767d3623fd7df04c54a56ab7a`.
- R3 tested fixture SHA-256: `3328d6710e80cc404d28fc902cd7983c6747752ed3a7da7bbe60b6adffb252c7`.
- Unchanged outbox store SHA-256 on base/candidate: `4531f1e2b9cd57a19236c7f27cc3632e5ea234b0a1cc3af0ffa979f53a23d7b3`.

Accepted authority remains Execution Control V3 (`implementation_authority:
contracts`), Product V13 and Architecture V0.4.7, already in the integration
base. Vendored governance integrity/adoption check returned rc0. R3 is
REUSE / BRIEF / DURABLE under the current user's continuation mandate.
See [R2 brief](BRIEF-1359-OUTBOX.md) and [R3 mandate/preflight](docs/evidence/1359-r3/BRIEF-R3.md).

## Change and preserved contracts

`src/forum_sync.rs:1206` projects the existing reconciler into at most three
channel-only workers; `:1263` supervises each channel's single in-flight
pass. A slow binding ensure or HTTP delivery blocks only that channel's
next tick. Enabled-kind ordering within the existing pass, atomic
`FOR UPDATE SKIP LOCKED` claim, same-instance anti-overtake predicate,
HTTP timeouts, settlement transaction, stable identities, retry and
dead-letter SQL remain unchanged. No whole-pass time-budget cancellation,
new scheduler or durable scheduling truth is introduced. Shared pool
capacity remains a physical constraint.

R3 makes **no further product-source change**. It corrects the sustained
test's unsupported post-timeout future-window assumption at
`tests/48_outbox_three_channels.rs:536,650,663`. The retry timestamp must
advance at least the first 2s backoff from its original due time. Both
healthy-channel 2s bounds and the existing fast-error future-backoff tests
are retained. Adjacent rustfmt changes are formatting only.

## Failure attribution

The starting tree was clean, with all R2 work committed. Instead of creating
an empty stash, R3 preserved that branch and used detached exact-base and
exact-candidate scratch worktrees. Only the test fixture was transplanted
onto base. Original raw failures and R2 stop/review remain historical.

Original tests on clean base were 0 passed / 3 failed, rc101. Forum/Kick
failed the initial healthy wait at line 601; Wake failed the later healthy
wait at line 628. Those latency failures do not attribute the backoff
assertion. The common diagnostic-only single-channel/single-`run_once`
probe then reached the **same original backoff assertion** on both product
revisions, using identical test SHA `03ee3de4e24c43369d5efd1a77a39ae2ec44d527a003a36502d180b321b80bef`.

| R2 failing case | Clean base: real timeout / retry timestamp remaining | Candidate: real timeout / retry timestamp remaining | Attribution |
|---|---|---|---|
| sustained_forum_timeout_keeps_other_channels_polling | 10011ms / -8.008s | 10008ms / -8.006s | Pre-existing timestamp behavior; new test expectation corrected |
| sustained_kick_timeout_keeps_other_channels_polling | 5007ms / -3.007s | 5004ms / -3.004s | Pre-existing timestamp behavior; new test expectation corrected |
| sustained_wake_timeout_keeps_other_channels_polling | 5004ms / -3.004s | 5004ms / -3.004s | Pre-existing timestamp behavior; new test expectation corrected |

Both diagnostic commands intentionally returned rc101 with all three rows
pending, attempt_count=1, an error retained and no dead letter. PostgreSQL
`now()` is the delivery transaction's start time: its first 2s retry can
already be past after a 5s/10s HTTP timeout. V3 CTR-SWEC-002 and Delta 4 do
not establish the stronger settlement-relative future window. There is no
demonstrated candidate backoff regression. These were candidate-added
unsupported assertions, not three pre-existing repository test failures;
the pre-existing behavior is documented and not turned into a new delivery
gate. No test exclusion or product retry repair was used.

Full attribution method, exact source/test hashes and raw output:
[attribution](docs/evidence/1359-r3/ATTRIBUTION.md),
[base original](docs/evidence/1359-r3/base-original.log),
[base same-assertion](docs/evidence/1359-r3/base-backoff.log),
[candidate same-assertion](docs/evidence/1359-r3/candidate-backoff.log).
Each log has a same-stem JSON command/time/rc/coordinate record.

## RED/GREEN observations

Original raw timing lines (RED, then GREEN), preserved without re-running passed evidence:

```text
H1359_TIMING pending_bindings=20 forum_http_timeout_ms=10000 lookup_calls=20 kick_received_ms=Some(200052) wake_received_ms=Some(200068) kick_settled_ms=Some(200063) wake_settled_ms=Some(200086)
H1359_TIMING pending_bindings=20 forum_http_timeout_ms=10000 lookup_calls=1 kick_received_ms=Some(5) wake_received_ms=Some(5) kick_settled_ms=Some(23) wake_settled_ms=Some(23)
```

See actual unabridged [RED log](docs/evidence/1359-r2/red.log) and
[GREEN log](docs/evidence/1359-r2/green.log), with JSON coordinates beside
them. RED was the explicitly authorized failure, rc101; GREEN rc0. These
measure this fixture, not a universal worst-case bound. Human/business or
production runtime acceptance is outside this source-only delivery.

R3 focused result: **22 passed, 0 failed, rc0**. All three corrected
sustained cases pass with real timeouts: healthy first/later settlement
25/22ms (Forum peer), 24/22ms (Kick peer), 25/22ms (Wake peer). Retry
timestamp advances are 2019/2011/2005ms. Existing claim, concurrency,
single-connection, same-instance order, backoff hold-back, disabled-channel,
unknown transport, retry/dead-letter/requeue and restart cases also pass.
[Focused raw log](docs/evidence/1359-r3/focused.log),
[command/rc/hash record](docs/evidence/1359-r3/focused.json).

## Full-suite, review and delivery gates

Full `cargo test --locked --jobs 4 -- --test-threads=1 --nocapture` returned
**rc101**, elapsed 1111.197s: raw aggregate **1221 passed / 18 failed /
1 ignored**, across 61 binary results (60 rc0, one rc101). Cargo stopped
at `45_workflow_prerequisites` and did not execute later binaries.
[Full log](docs/evidence/1359-r3/full.log),
[command/hash/rc](docs/evidence/1359-r3/full.json),
[binary results](docs/evidence/1359-r3/full-binary-results.jsonl),
[summary](docs/evidence/1359-r3/summary.json).

The failing binary had **10 passed / 18 failed**. All 18 failed before a
database connection at existing `tests/prerequisites/support.rs:6`, which
requires `TEST_DATABASE_URL` to end with `/svc_prereq_20261029_impl`.
The R3 runner omitted that route and supplied
`postgres://postgres@127.0.0.1:56589/h1359_r3_45_workflow_prerequisites`.
This is a confirmed runner database-name mismatch. It is not evidence of
an outbox regression, and no base comparison was executed to classify it
as a pre-existing suite failure. The runner was not repaired or rerun
after this unexpected failure; the user's final “失败即停” controls.

The complete command and isolated routing as actually executed are in
[reproduction](docs/evidence/1359-r3/REPRODUCTION.md), with the omission
explicitly recorded. The ignored owner-wake test and optional old-binary
`NOT_RUN` / canonical-principal skip remain coverage limits. The tests/42
guard panics are expected caught negative tests; that binary actually
returned rc0 (3 passed), and is not a second failure.

Independent affected-contract and failure review by
`/root/outbox_attribution_review` found no demonstrated source/test
regression at the recorded hashes, confirmed the exact full-gate failure,
and returned **ACCEPTANCE_COVERAGE_REVIEW=FAIL / NEXT_ACTION=STOP**.
See [R3 review](docs/evidence/1359-r3/REVIEW-R3.md).
No committed-final-Head or overall delivery PASS is issued.

| DONE_WHEN item | State |
|---|---|
| Real ~200s RED and same-probe 23ms GREEN | Complete; retained R2 raw evidence |
| Minimal source isolation / unchanged claim/order/idempotency | Implemented at 062fe82; independent source review |
| Attribute the three backoff failures | Complete, matching same-assertion controls |
| Outbox suite rc0 | Complete, 22/22 |
| Full cargo suite rc0 | FAILED, rc101; fixture-name guard in binary45 |
| Commit / independent exact-Head review | NOT DONE; R3 changes remain uncommitted |
| Push / main-base PR / no merge | NOT DONE; no push/PR/merge |
| Owned fixture shutdown | Complete, all three identity-checked pg_ctl stops rc0 |

All three task-owned PG data directories remain available and were stopped
without touching existing services; [cleanup record](docs/evidence/1359-r3/cleanup.json).
The branch remains at R2 checkpoint `8aa864ab9ff70107faf8e016a04d32f4e78ad141`,
with R3 tests/records preserved uncommitted. The next continuation must
explicitly reopen the stopped gate, correct the runner's database-name
route, and complete the remaining validation/delivery steps. No new
Product Authority or product backoff repair is indicated by these results.

No rebase, governance changes, production writes, deployment, configuration
changes, backlog consumption, or merge were performed.
