# R3 backoff failure attribution

The starting worktree was clean; R2 changes were already committed. They
were preserved rather than moved into a new stash. A detached worktree at
exact base 8da8dbcb70e700d9e35970e8b662b1be2c29afdc carried only the R2
test fixture. Its product source was unchanged. The candidate comparison
used detached implementation 062fe82301eadbf23b5b6279643355c20ca4218c.
Both used the same owned PostgreSQL 16 server and loopback-only fake HTTP.

## Original-test control

`base-original.{json,log}`: original test SHA
9f39ea56e7c65d0f79829cc739980d93f98dbe265dca69ff78588773c1460941;
0 passed / 3 failed, rc101. Forum/Kick failed the initial 2s healthy-channel
wait at line 601; Wake failed the later 2s healthy-channel wait at line 628.
These failures establish serial coupling, **not** backoff attribution.

## Same-assertion control

`prepare-attribution.py` pins the R2 fixture and creates the retained
`attribution-only.patch`. Only its explicitly selected diagnostic branch
enables the slow channel alone, calls the unchanged public `run_once`
exactly once, waits the original HTTP timeout, and evaluates the original
`next_attempt_future` assertion. Neither product source nor retry/claim SQL
is changed. This probe does not count as isolation acceptance.

Both runs used diagnostic test SHA
03ee3de4e24c43369d5efd1a77a39ae2ec44d527a003a36502d180b321b80bef,
each 0 passed / 3 failed at the **same original backoff assertion**, rc101.
Every slow row has attempt_count=1, remains undelivered, is not dead-lettered,
and retains an error. Raw output and timestamp coordinates are in
`base-backoff.{json,log}` and `candidate-backoff.{json,log}`.

| Original failing case | Clean base: timeout / remaining retry time | Candidate: timeout / remaining retry time | Attribution / disposition |
|---|---|---|---|
| sustained_forum_timeout_keeps_other_channels_polling | 10011ms / -8.008s | 10008ms / -8.006s | Same pre-existing timestamp behavior; new overstrong test assertion corrected |
| sustained_kick_timeout_keeps_other_channels_polling | 5007ms / -3.007s | 5004ms / -3.004s | Same pre-existing timestamp behavior; new overstrong test assertion corrected |
| sustained_wake_timeout_keeps_other_channels_polling | 5004ms / -3.004s | 5004ms / -3.004s | Same pre-existing timestamp behavior; new overstrong test assertion corrected |

The unchanged `mark_attempt_failed` SQL uses PostgreSQL transaction-start
`now() + 2s` on the first failure. `deliver_claimed` opens that transaction
before the real 5s/10s HTTP attempt. Thus its timestamp can be past after
settlement on either revision. Accepted V3 CTR-SWEC-002 specifies `now()`
backoff; Delta 4 preserves HTTP and settlement in one transaction. A future
window measured from settlement is not an authorized new requirement.

Conclusion: **no candidate backoff regression demonstrated**. The three
R2 failures were unsupported candidate-added assertions against existing
timestamp behavior, not three pre-existing repository test failures. They
are recorded as pre-existing behavior and are not new delivery gates. No
retry implementation repair, exclusion/ignore, timing budget relaxation,
governance change, or product contract change is made.

The permanent test correction captures the original due timestamp before
the loop and checks persisted retry advancement by at least the first 2s
backoff. Both healthy-channel 2s limits, real HTTP timeouts, pending/error/
count/dead-letter checks, and existing fast-error future-backoff assertions
remain. This advancement check does not independently prove the complete
exponential formula or a settlement-relative future window. The full
outbox suite, including all three corrected sustained tests, remains a
required rc0 gate.

Independent read-only attribution review: /root/outbox_attribution_review
confirmed the contract-preserving correction and warned against conflating
the earlier latency failures with the backoff failure. Final affected-Head
review and suite results are recorded separately.
