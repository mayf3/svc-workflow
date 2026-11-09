# Independent affected-scope review

Reviewer: `/root/semantic_review`. Method: read-only source and recorded evidence inspection; no tests executed by this reviewer, no file edits and no production access. Root transcribed this review from the reviewer's returned observations.

`REVIEW_TARGET_HEAD=BASE_HEAD=f51d8724fe0dc80053a82c37f19862b059ba6a37`.

Three implementation files match main. #128 attribution initialization, unattributed warning and invalid UUID rejection remain at `scripts/release.sh:92-112`; provenance/ledger fields at `431-441,550-564`; verify preserves prior attribution at `742-752`. #71 kind-scoped batch remains at `src/store/postgres/outbox.rs:157-188`, independent wake-first drain at `src/forum_sync.rs:870-900`, and fail-closed verification plus byte-preserved history at `scripts/release.sh:628-724`. #132 Forum Option, per-channel switches and worker isolation remain at `src/forum_sync.rs:103-111,249-345,884-899,1213-1268`.

The new `scripts/test_release.sh:194-223` correctly tests attribution × receipt history: escaped and Chinese historical prefix bytes retained; four attribution fields preserved across successful verification; full ledger preserved on a subsequent failed verification. Existing outbox batch × channel tests at `src/forum_sync.rs:1429,1465-1560,1591` and `tests/48_outbox_three_channels.rs:286,669-679,785-938,1476` cover the other interaction, so no further test is needed.

Independent raw shell count: **114 PASS / 0 FAIL** (26 core plus 88 upstream); the new case is PASS at log line 3. Metadata records rc=0 and the exact candidate. This is +1 over the 113 integrated baseline; the terminal 88/0 summary covers only the upstream suite.

`resume-validation.py` fixes task PG host/port/database/role/data-directory and checks all five before cargo. It clears inherited PG/database/Workflow/attribution/proxy environment, runs only locked/offline lib and three-channel binaries, records commands/times/rc/counts/raw logs, and compares before/after source hashes. Its final assertion checks failure status, not historical count lower bounds; the receipt and test-comparison.json must make those comparisons explicitly.

Recorded library result read by reviewer: **276 passed / 0 failed / 1 ignored, rc=0**. Recorded channel log: **22 passed / 0 failed**. The existing explicitly ignored owner-wake recovery test at `src/forum_sync.rs:2587-2591` was not executed and must remain visible. Parent verifies channel rc and the full acceptance comparison.

Affected semantic review: **PASS**, no new interaction gap found. Runtime/production acceptance is outside this review. Read authorities: repository AGENTS/shared/local governance and REVIEW mode, accepted Product V13 / Architecture V0.4.7 / Execution Control V3. V3 authorizes source contracts and grants no production apply authority.
