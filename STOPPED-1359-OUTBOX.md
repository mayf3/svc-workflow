# STOPPED-1359-OUTBOX — R2

**STOPPED / REQUIRED_GATE_FAILURE. Task incomplete; do not merge or activate.**

Stop source is the user's original mandate (`/private/tmp/h1359-worker.log:20,32`), renewed by the R2 request: “失败即停写STOPPED-1359-OUTBOX.md” and “失败即停; 不动治理文档; 不rebase; PR禁自合并”. The expected RED was explicitly authorized. The later focused-suite rc101 was unexpected and activates this stop. No attempt was made to repair, remove, weaken or rerun the failing tests after that result.

## Bound coordinates

| Coordinate | Value |
|---|---|
| Repository / surface | `mayf3/svc-workflow` / `/private/tmp/h1359-outbox` |
| Integration base | `8da8dbcb70e700d9e35970e8b662b1be2c29afdc` |
| Branch | `fix/outbox-channel-isolation` |
| RED fixture commit | `0cbb7f6cc2c68b114cc493712eb98699ad660393` |
| Implementation and failed-suite target | `062fe82301eadbf23b5b6279643355c20ca4218c` |
| Product source SHA-256 | `d78eb6f6c09c4a7849f2fc218c48543bddf476e767d3623fd7df04c54a56ab7a` |
| Final test file SHA-256 | `9f39ea56e7c65d0f79829cc739980d93f98dbe265dca69ff78588773c1460941` |
| Outbox store SHA-256, unchanged from base | `4531f1e2b9cd57a19236c7f27cc3632e5ea234b0a1cc3af0ffa979f53a23d7b3` |
| Remote main at stop recheck | `8da8dbcb70e700d9e35970e8b662b1be2c29afdc` |
| Remote fix branch / PR | Not pushed / not created |

The following documentation/evidence commit does not modify product source or tests. R1's wrong-repository stop was superseded by the R2 corrected work surface; it is not the current stop reason.

## Executed evidence

Raw output is preserved byte-for-byte under [docs/evidence/1359-r2](docs/evidence/1359-r2). JSON receipts bind command, time, source/test hashes, return code and the local disposable database. The ordinary `git diff --check` warning about the raw GREEN log's final blank line is a raw-output formatting observation; the separate source/test diff check passed. The actual stop gate below is the test return code.

| Evidence | Exact result | Limit |
|---|---|---|
| [RED](docs/evidence/1359-r2/red.log), [coordinates](docs/evidence/1359-r2/red.json) | Real background loop, 20 pending bindings, 20 original 10s Forum HTTP timeouts. Kick received 200052ms / settled 200063ms; wake received 200068ms / settled 200086ms. rc101, expected timing assertion failure. | This fixture's measurement, not an absolute system worst-case bound. Original product source SHA `cad3a653abe40ca0fac122112891230a12abf22a5fba9e24dd1f6786ca8e9f95`. |
| [Initial GREEN](docs/evidence/1359-r2/green.log), [coordinates](docs/evidence/1359-r2/green.json) | Same test bytes as RED, SHA `2fc75625cc81fca95e62cd5e5548793d8915a2b1497cb01786ad26773a0d15f7`: both healthy channels received at 5ms and settled at 23ms, rc0. | Receiver released after fast progress; does not alone prove sustained timeout behavior. Before formatting and the sustained-test additions. |
| [Focused suite](docs/evidence/1359-r2/focused.log), [coordinates](docs/evidence/1359-r2/focused.json) | `cargo test --locked --test 48_outbox_three_channels -- --test-threads=1 --nocapture`: **19 passed, 3 failed, rc101**. All 18 previously existing tests and the timing probe passed. Final timing probe: both channels settled at 26ms. | Entire focused suite FAIL. The planned lib command did not execute. |
| Sustained Forum timeout | First healthy rows 23ms; slow attempt settled at 10017ms; later healthy rows 22ms. Fails `tests/48_outbox_three_channels.rs:652`, `original backoff is retained`. | Retained failed result, not a passing sustained acceptance. |
| Sustained kick timeout | First healthy rows 23ms; slow attempt settled at 5013ms; later healthy rows 24ms. Same failure. | Same limitation. |
| Sustained wake timeout | First healthy rows 23ms; slow attempt settled at 5023ms; later healthy rows 23ms. Same failure. | Same limitation. |

## Failure explanation and authority boundary

**OBSERVED:** all three new tests reach the final `next_attempt_future` assertion and fail there. The source updates `next_attempt_at = now() + make_interval(secs => LEAST(POWER(2, attempt_count + 1), 600))` in `src/store/postgres/outbox.rs:249`, unchanged from the base. `deliver_claimed` begins its transaction before HTTP and settles after HTTP. The new helper computes `next_attempt_future` against `Utc::now()` and demands it remain true after the 5s/10s timeout. At first failure the stored increment schedules two seconds from the transaction timestamp; the HTTP duration can outlast that value.

**INFERRED, independently reviewed:** the new assertion assumes a stronger after-settlement wall-clock delay than the existing implementation supplies. This is not evidence that the isolation patch changed backoff, and the second sequential HTTP start in each trace is not evidence of concurrent double claiming. The existing transaction-timestamp behavior may require a separate conformance interpretation against the accepted `now()`/backoff text. No load-bearing SPEC_GAP has been conclusively established, and no governing text was changed to resolve the ambiguity.

The source change only creates at most three channel-projected workers within the existing reconciler; each still invokes the original ordered pass. `run_once`, `drain_channel`, `deliver_claimed`, the outbox store, claim predicate, anti-overtake query, retry/dead-letter SQL, HTTP timeout values and stable idempotency keys were not changed. Independent source review found no demonstrated change to those obligations; **the failed validation gate still prevents overall PASS**.

### Blocker

- **Class:** `REQUIRED_GATE_FAILURE`.
- **SOURCE:** attributable execution mandate's unexpected-failure stop boundary; accepted local governance's evidence/readiness rules.
- **COUNTEREXAMPLE:** [focused.log:26](docs/evidence/1359-r2/focused.log), through the three failure traces and `COMMAND_RC 101`.
- **IMPACT:** focused acceptance FAIL; broader lib/full cargo suite unexecuted; merge readiness NO; production operation NO; push/PR delivery stopped.
- **MINIMAL_CLOSURE:** a later explicit resume may first resolve whether the test is an unsupported expectation or exposes separately scoped existing drift. Preserve raw evidence and accepted claim/lock/ordering semantics; reroute any proposed behavior change before implementation. This R2 does not silently weaken the assertion or expand into retry repair.

The applied [spec-governance skill](.agents/skills/spec-governance/SKILL.md) and [selected PREFLIGHT mode](.agents/skills/spec-governance/modes/PREFLIGHT.md) require: “Do not start semantic implementation or mutation when the relevant readiness flag is `NO`.” The immediate stop here is required by the user mandate; the skill does not create a new approval requirement.

## Task / DONE_WHEN accounting

| Original task | State |
|---|---|
| 1. RED reproduction on corrected Svc main | Complete, measured approximately 200s with raw evidence |
| 2. Minimal channel isolation | Candidate source committed locally; not fully validated |
| 3. Accepted-lock SPEC_GAP check | Bounded worker route independently reviewed as REUSE; no claim/lock change; retry-time interpretation unresolved |
| 4. Same-probe GREEN and existing suites rc0 | Probe GREEN; focused suite FAIL; broader suites NOT RUN |
| 5. Push / main-base PR / completion receipt | NOT DONE due mandatory stop; no completion receipt issued |
| No governance changes / no rebase / no self-merge | Preserved |
| Production writes, deployment, config or backlog consumption | None |

## Owned environment closure

The three newly created test clusters were identity-checked by data directory, port and loopback listen address before shutdown. All three `pg_ctl -m fast -w stop` calls returned 0; their data and logs remain preserved:

- `/private/tmp/h1359-r2-pg/data`, port 56485;
- `/private/tmp/h1359-r2-pg-55432/data`, port 55432;
- `/private/tmp/h1359-r2-pg-55483/data`, port 55483 (prepared but no suite executed there).

See [cleanup.json](docs/evidence/1359-r2/cleanup.json) and the corresponding `pg-stop-*.log` files. No existing PostgreSQL cluster, service or other checkout was stopped or changed. Source/tests and failed evidence remain on the local branch as a resumable checkpoint.
