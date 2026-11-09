# #1359 outbox channel isolation — R4 stopped receipt

STATUS: **STOPPED / INCOMPLETE / REQUIRED_GATE_FAILURE**. Fixed-name routing
and all original18 binary45 failures are resolved. Full cargo is **1324
passed /10 failed /1 ignored /rc101**. The newly reached binary67 fails.
No R4 commit, push, PR or merge was performed; no completion PASS is issued.
A full-suite all-green original does not exist for this attempt.

## Coordinates and scope

- Surface: mayf3/svc-workflow, `/private/tmp/h1359-outbox`.
- Branch: `fix/outbox-channel-isolation`; intended PR base `main`.
- Start and stopped HEAD: `8aa864ab9ff70107faf8e016a04d32f4e78ad141`.
- Integration base and fresh remote main: `8da8dbcb70e700d9e35970e8b662b1be2c29afdc`.
- Product implementation: `062fe82301eadbf23b5b6279643355c20ca4218c`.
- Product SHA256: `d78eb6f6c09c4a7849f2fc218c48543bddf476e767d3623fd7df04c54a56ab7a`.
- R3 corrected test SHA256: `3328d6710e80cc404d28fc902cd7983c6747752ed3a7da7bbe60b6adffb252c7`.
- Unchanged base/candidate outbox store SHA256: `4531f1e2b9cd57a19236c7f27cc3632e5ea234b0a1cc3af0ffa979f53a23d7b3`.
- R4 runner SHA256: `a4de83a85db39aa9ba3d56056fd3403367d894e84f17b43e17a5980cfa4e36e1`.

Current user explicitly reopened R3 only for the runner correction and
validation/delivery chain, with final “红线照旧; 失败即停”. Route remains
REUSE / BRIEF / DURABLE under accepted Execution ControlV3 contracts,
ProductV13/ArchitectureV0.4.7 in the base. Governance check with accepted
adoption returned rc0; no authority changes. See [R4 brief](docs/evidence/1359-r4/BRIEF-R4.md)
and [original brief](BRIEF-1359-OUTBOX.md). Existing R3 changes stay
uncommitted; R4 changes only the evidence runner/environment/records.
The reconciler implementation, `run_once`, claim/order/idempotency,
HTTP limits, backoff and dead-letter code have no new R4 changes.

## Fixed database-name route — implemented and verified

`docs/evidence/1359-r4/cargo-test-runner.py:19-20` adds the missing
`45_workflow_prerequisites -> svc_prereq_20261029_impl` route. Both
`TEST_DATABASE_URL` and `DATABASE_URL` receive
`postgres://postgres@127.0.0.1:56589/svc_prereq_20261029_impl`.
The existing `tests/prerequisites/support.rs:6-10` fixed-suffix guard stays
unchanged; each test then creates its own fresh UUID database as before.
R3's original runner and raw failures remain byte-preserved.

The actual route/rc is [binary record line61](docs/evidence/1359-r4/full-binary-results.jsonl),
and [full.log line2055](docs/evidence/1359-r4/full.log) records28/28rc0.
Independent review found all18 original failed names individually `ok` in
this run. [Runner correction diff](docs/evidence/1359-r4/runner-correction.diff),
[reproduction](docs/evidence/1359-r4/REPRODUCTION.md),
[source-input manifest](docs/evidence/1359-r4/validation-input-manifest.json).
All468 tracked build inputs still match their pre-run hashes.

## Attribution table

| Failure set | Qualified attribution | R4 disposition |
|---|---|---|
| R2 three new sustained-backoff assertions | Same-byte probes on base/candidate fail the same original assertion; pre-existing transaction-start timestamp behavior, unsupported new test expectation | Reuse completed R3 attribution/correction; focused22/22rc0 and fresh R4 full binary48=22/22rc0 |
| R3 binary45:18 connection guards | Runner omitted mandatory fixed database input suffix; no product regression shown | Fixed route;28/28rc0, all18 failures resolved without Rust fixture edits |
| R4 binary67:10 new failures | Nine admission Unavailable; one real HTTP422 admission_legacy_input vs200. Base/candidate cause UNKNOWN; R3 never reached this binary | STOP; no repair, retry, base comparison or delivery action |

### Preserved R3 same-assertion attribution


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


## Actual R4 validation originals

Exact command: `cargo test --locked --jobs 4 -- --test-threads=1 --nocapture`,
using the explicit disposable DB runner and conformance cfg in
[full.json](docs/evidence/1359-r4/full.json). Elapsed1224.678s;65 completed
binaries,64rc0 and only67rc101. Aggregate1324/10/1 is derived from all65
raw result lines, with zero filtered tests. The existing ignored test and
optional old-binary/canonical-principal NOT_RUN legs remain coverage limits;
no test was removed or newly ignored.

Original result lines, without alteration:

```text
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.80s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.75s
test result: FAILED. 13 passed; 10 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.49s
error: test failed, to rerun pass `--test 67_authorized_actions_r4`
```

Actual return-code field from full.json:

```json
  "rc": 101,
```

[Full unabridged log](docs/evidence/1359-r4/full.log),
[all raw result lines](docs/evidence/1359-r4/test-results-verbatim.txt),
[derived summary](docs/evidence/1359-r4/summary.json),
[all binary URLs and rc](docs/evidence/1359-r4/full-binary-results.jsonl).
Full log SHA256: `f697d768dc568fced0b04b9280f8ed81544768a14d2c325bf31abd724b587681`.
R2 ~200s RED/23ms GREEN and R3 focused evidence remain preserved and were not
replayed. [Historical R3 receipt](docs/evidence/1359-r3/RECEIPT-R3.md).

## New stop and independent review

The first newly reached failing group is67. [Ten failure observations](docs/evidence/1359-r4/FAILURE-67.md)
bind each case, source coordinate and actual error. These are uncaught
failures, unlike binary42's expected caught-panic negatives. No evidence
supports calling67 either pre-existing or an outbox regression. No new
Product Authority or admission behavior is inferred from the tests.

Independent reviewer `/root/r4_review` recomputed1324/10/1, confirmed all18
original45 cases individually pass, all468 build hashes match,65binary
return codes and full-log hash, and the three actual cleanup rc0s.
Decision: REQUIRED_GATE_FAILURE=1, ACCEPTANCE_COVERAGE_REVIEW=FAIL,
MERGE_READY=NO, DELIVERY_COMPLETE=NO, NEXT_ACTION=STOP.
[Review](docs/evidence/1359-r4/REVIEW-R4.md), [current stop](STOPPED-1359-OUTBOX.md).
No committed-final-Head review or overall PASS is issued.

## Checkpoint and closure

| Mandated item | Actual state |
|---|---|
| Reuse completed R3 two-part attribution | Preserved, not rerun |
| Fixed-name routing / original18 failures | Complete, binary45 28/28rc0 |
| Outbox suite | Fresh full-run22/22rc0 |
| Full cargo rc0 | FAIL:1324pass/10fail/1ignored,rc101 |
| Commit / push / main-base PR / completion receipt | Stopped; none performed; this is an incomplete receipt |
| Merge / rebase / governance edits / production writes/deploy/config/backlog | None |
| Owned fixture closure | All3 identity-checked PG stops rc0; data/logs retained |

[Cleanup](docs/evidence/1359-r4/cleanup.json) binds exact directories, ports,
loopback identities and actual stop commands. No existing PG service was
changed or stopped. Scratch attribution worktrees and all uncommitted R3/R4
changes remain resumable. A later explicitly authorized continuation must
first attribute the newly reached67 failures before any unrelated repair or
renewed validation/delivery. Current mandate requires stopping here.
