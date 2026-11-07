# #1359 outbox channel isolation — R5 receipt

STATUS: **CANDIDATE_VALIDATED / PUBLICATION_PENDING**. R5 resumes the prior
stop under the current user's exact clean-control/pre-existing disposition.
No R5 unexpected validation failure occurred. Commit, push and unmerged PR
publication remain pending at this prepublication checkpoint.

## Coordinates and authorized delivery gate

- Repository/worktree: `mayf3/svc-workflow`, `/private/tmp/h1359-outbox`.
- Branch: `fix/outbox-channel-isolation`; PR target `main`; merge forbidden.
- Integration and fresh remote main: `8da8dbcb70e700d9e35970e8b662b1be2c29afdc`.
- Start/clean-control Head: `8aa864ab9ff70107faf8e016a04d32f4e78ad141`.
- Original implementation commit: `062fe82301eadbf23b5b6279643355c20ca4218c`.
- Product SHA256: `d78eb6f6c09c4a7849f2fc218c48543bddf476e767d3623fd7df04c54a56ab7a`.
- Corrected test48 SHA256: `3328d6710e80cc404d28fc902cd7983c6747752ed3a7da7bbe60b6adffb252c7`.
- Unchanged outbox store SHA256: `4531f1e2b9cd57a19236c7f27cc3632e5ea234b0a1cc3af0ffa979f53a23d7b3`.

R5 execution mandate: stash candidate -> clean8aa group67 control -> exact
stash pop -> if clean also red, record pre-existing and use candidate surface
gates, then commit/push/open main-base PR without merge and complete receipt.
This explicitly supersedes R4's full-rc0 delivery requirement only for the
attributed67 failure set. It does not create Product Authority or waive a new
failure. [R5 Brief](docs/evidence/1359-r5/BRIEF-R5.md) binds the scope.
Route remains REUSE / BRIEF / DURABLE under accepted ExecutionControlV3,
ProductV13 and ArchitectureV0.4.7 in the integration base. Accepted adoption
and vendored-byte verification pass; no governance delta or proven Spec gap.

## Clean-control attribution and restoration

[Full attribution](docs/evidence/1359-r5/ATTRIBUTION.md),
[raw clean67](docs/evidence/1359-r5/clean67.log),
[actual command/rc](docs/evidence/1359-r5/clean67.json),
[individual comparison](docs/evidence/1359-r5/attribution-comparison.json).

```text
cargo test --locked --jobs 4 --test 67_authorized_actions_r4 -- --test-threads=1 --nocapture
test result: FAILED. 13 passed; 10 failed; 0 ignored; 0 measured; 0 filtered out; finished in 62.88s
```

Actual cargo **rc101**, expected diagnostic RED. The exact same ten names,
panic source coordinates and errors recur on clean8aa: **nine
AdmissionFailed(Unavailable), one HTTP422 admission_legacy_input vs200**.
Pre-existing relative to the restored uncommitted R3/R4 delta, as specified
by R5. These ten are excluded from this delivery gate by the attributable
user mandate, not removed, ignored or weakened in code.

Clean8aa already contains062fe82; this is not an original-main runtime
comparison. Supplemental static evidence: all67/common fixture files match
integration main;67 never invokes the patched background loop. Unavailable
is explained by fixture publication passing `AdmissionGate::disabled()`
for AGENT identity literals, whose existing no-client path fails closed.
No port conflict or unavailable production service is demonstrated. The
successor legacy-input cause remains a source-based inference with an actual
422 response; no unrelated admission repair occurred.

Task stash `e152fd6acbd0279c2f0ee2132cd67848e0469853` was pushed including
untracked files, then popped rc0. [Clean state](docs/evidence/1359-r5/clean-state.json)
proves empty status at exact8aa; [restoration](docs/evidence/1359-r5/restoration.json)
proves **814/814 files restored, zero hash mismatch**. Other stashes untouched.

## Candidate validation and preserved full-suite result

| Evidence | Actual result | Disposition |
|---|---|---|
| R5 candidate48, fresh restored bytes | **22 passed /0 failed /rc0** | Current affected delivery gate PASS |
| R5 library, fresh | **269 passed /0 failed /1 existing ignored /rc0** | Current affected delivery gate PASS |
| R4 binary45 | **28/28 rc0** | Reused; fixed database-name route preserved |
| R4 full cargo | **1324 passed /10 failed /1 ignored /rc101** | Preserved FAIL; all ten attributed above; never called full-green |
| R4 build inputs | **468/468 SHA match** | Reuse passed binaries without rerun |
| Historical R2/R3/R4 evidence | **70/70 SHA match** | Original failures and reviews retained |

[Fresh outbox raw](docs/evidence/1359-r5/candidate48.log),
[command/time/rc/hashes](docs/evidence/1359-r5/candidate48.json),
[lib raw](docs/evidence/1359-r5/candidate-lib.log),
[lib metadata](docs/evidence/1359-r5/candidate-lib.json),
[reuse manifest](docs/evidence/1359-r5/validation-reuse.json),
[R4 full raw](docs/evidence/1359-r4/full.log),
[R4 per-binary rc](docs/evidence/1359-r4/full-binary-results.jsonl).

New executed sustained measurements, unchanged real HTTP timeouts:

```text
H1359_SUSTAINED slow=FORUM_EVENT first_healthy_ms=23 slow_timeout_settled_ms=10026 later_healthy_ms=22 slow_attempts=1 slow_http_started=2
H1359_SUSTAINED slow=EXECUTION_KICK first_healthy_ms=24 slow_timeout_settled_ms=5020 later_healthy_ms=42 slow_attempts=1 slow_http_started=2
H1359_SUSTAINED slow=OWNER_ASSISTANCE_WAKE first_healthy_ms=23 slow_timeout_settled_ms=5013 later_healthy_ms=24 slow_attempts=1 slow_http_started=2
```

Original same-fixture RED/GREEN remains unchanged: R2 RED healthy kick settled
200063ms / wake200086ms, rc101 expected; initial GREEN both settled23ms rc0.
[RED original](docs/evidence/1359-r2/red.log), [RED binding](docs/evidence/1359-r2/red.json),
[GREEN original](docs/evidence/1359-r2/green.log), [GREEN binding](docs/evidence/1359-r2/green.json).
R3 separately proved transaction-start backoff behavior on base/candidate
with the same diagnostic bytes, then corrected only unsupported new sustained
assertions. Both2s healthy gates, real5s/10s timeouts, retention and existing
quick-error future-backoff assertions remain. [R3 attribution](docs/evidence/1359-r3/ATTRIBUTION.md).
The existing ignored lib test and optional old-binary/canonical-principal
NOT_RUN legs remain explicit coverage limits.

## Implementation, review and closure

Product change remains only `src/forum_sync.rs` background worker structure:
`run_loop` projects the existing config into at most three channel workers;
each uses unchanged ordered `run_once`, original interval and panic supervision.
Claim/settle transaction, SQL lock, per-instance anti-overtake order,
idempotency keys, HTTP limits, retry and dead-letter paths are unchanged.
R3 test changes are committed as part of this delivery; R4 changes only its
separate evidence runner's binary45 database-name route. R5 changes evidence
and records only. No new product/admission/config semantics are introduced.

[Independent affected review](docs/evidence/1359-r5/REVIEW-AFFECTED.md) by
`/root/r5_review`: authority, Contracts, exact ten-failure attribution,
restoration, input reuse and R5 bounded acceptance coverage PASS; blockers0,
proven Spec gaps0. Final committed-Head/publication review is still pending
at this checkpoint. [Historical R4 receipt](docs/evidence/1359-r5/RECEIPT-R4-CHECKPOINT.md)
preserves the previous incomplete state verbatim.

Both new task PG16 clusters were checked against exact data-directory/port/
127.0.0.1 identities, then stopped with actualrc0. PID files absent and ports
closed; retained data/server logs remain under `/private/tmp/h1359-r5-pg-*`.
[Cleanup and readbacks](docs/evidence/1359-r5/cleanup.json).
No pre-existing service restart, production write, deployment, loaded config
change, backlog operation, governance edit, rebase or merge occurred.
Source/isolated validation does not establish production or business acceptance.

## DONE_WHEN

| Requirement | State |
|---|---|
| Clean8aa exact67 attribution | DONE, same ten; pre-existing with coordinate limit |
| Exact stash pop / preserve original evidence | DONE,814 restored/70 evidence files match |
| Candidate gates and R4 passed-results reuse | DONE,22/22 +269/1ignored rc0;468 inputs match |
| Independent affected review | DONE, bounded PASS |
| Commit / push / base-main PR, no merge | PENDING publication |
| Completed publication receipt/final-head review | PENDING actual publication |

`PR_DELIVERY_READY=YES` for publication preparation; `MERGE_READY=NO`
(self-merge forbidden); `OPERATION_ALLOWED=NO`. The containing commit hash,
ordinary push and fresh open/unmerged PR readback will be recorded after
those actions occur, avoiding self-referential commit claims.
