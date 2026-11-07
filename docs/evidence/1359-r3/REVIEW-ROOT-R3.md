> R3 current state: full-suite gate failed; see [R3 review](docs/evidence/1359-r3/REVIEW-R3.md) and [current stop](STOPPED-1359-OUTBOX.md). The R2 review below is preserved as history.

# Independent affected-contract review — #1359 R2

Recorded from the final read-only review delivered by `/root/authority_check`. The implementation author recorded this result without changing its decision. The reviewer performed no writes, test executions or production access.

```text
SPEC_GOVERNANCE_MODE = REVIEW
REVIEW_KIND = AFFECTED_CONTRACT
REVIEW_TARGET_HEAD = 062fe82301eadbf23b5b6279643355c20ca4218c
BASE_HEAD = 8da8dbcb70e700d9e35970e8b662b1be2c29afdc
CURRENT_BASE_HEAD = 8da8dbcb70e700d9e35970e8b662b1be2c29afdc
REVIEWER_ID = /root/authority_check
AUTHOR_INDEPENDENCE = PASS
AUTHORITY_REVIEW = PASS
CONTRACT_REVIEW = PASS (source inspection only)
ACCEPTANCE_COVERAGE_REVIEW = FAIL
EVIDENCE_REVIEWABILITY = PASS
BLOCKERS = 1
SPEC_GAPS = 0 proven
IMPLEMENTATION_ALLOWED = NO (mandated stop)
MERGE_READY = NO
OPERATION_ALLOWED = NO
NEXT_ACTION = STOP
```

## Source findings

- `src/forum_sync.rs:1212-1245` moves the original configuration into three channel-only projections. Disabled channel configuration stays disabled. Wake configuration `Err` stays active and fail-closed under lines 339-340.
- Lines 1250-1288 use two levels of `JoinSet` ownership, at most one in-flight pass per channel, no ordinary whole-pass timeout cancellation, and continuation after a pass panic. Cancelling the reconciler requests cascading cancellation of its child tasks.
- The original `run_once` order and preparation/claim/HTTP/settlement boundaries remain unchanged, as do the candidate query, same-instance anti-overtake guard, idempotency identities and HTTP timeout values. Base and candidate `src/store/postgres/outbox.rs` both hash to `4531f1e2b9cd57a19236c7f27cc3632e5ea234b0a1cc3af0ffa979f53a23d7b3`.
- The sustained tests reach the original 10s/5s HTTP timeout and measure healthy rows at 23ms initially and 22-24ms subsequently. They fail their later backoff assertion and must not be counted as passing acceptance.

## Blocker: REQUIRED_GATE_FAILURE

**SOURCE:** original execution mandate `/private/tmp/h1359-worker.log:20,32`, “失败即停”, also persisted in the local Brief's unexpected-validation-failure stop boundary.

**COUNTEREXAMPLE:** `docs/evidence/1359-r2/focused.log:26-41,51-54`: 19 passed, 3 failed, rc101. All three fail at `tests/48_outbox_three_channels.rs:652`. `focused.json` contains only the first command result; `--lib forum_sync` was not run.

**IMPACT:** full-suite, completion-receipt and repair-PR delivery gates are not met. Overall PASS is forbidden.

**MINIMAL_CLOSURE:** preserve failure evidence and the stop record. Later explicit continuation must resolve the retry-time interpretation or unsupported test expectation before verification. Do not close this round by deleting the assertion, self-authorizing another repair, or rerunning.

## Bounded failure attribution

The reviewer found a new test assumption incompatible with the existing timestamp implementation, rather than a proven candidate regression. `deliver_claimed:1012` opens the transaction before HTTP; `outbox.rs:249` computes the first retry using transaction-start `now()+2s`. After 5s/10s HTTP, that timestamp can already be past. The helper's `row_state:267` compares against current wall clock and the new assertion requires it remain future. `slow_http_started=2` is compatible with a sequential second attempt after the first settlement; it is not proof of simultaneously duplicate claims. Whether the existing behavior is contract drift remains unresolved.

Decision: **source structure reviewed; execution gate FAIL; STOP**. See [STOPPED-1359-OUTBOX.md](STOPPED-1359-OUTBOX.md).
