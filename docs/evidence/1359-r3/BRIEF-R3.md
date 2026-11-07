# #1359 R3 continuation and attribution mandate

Issuer: current user R3 instruction, 2026-11-08 (Asia/Shanghai): attribute
the three R2 backoff failures against clean base, fix candidate code/tests
as justified, then outbox rc0 -> full cargo test rc0 -> commit -> push PR
against main, never merge, and complete RECEIPT-1359-OUTBOX.md with attribution.
Expected attribution failures are diagnostic observations. Any unexpected
failure stops dependent work. No governance edits, rebase, production write,
deployment/config/backlog operation, or unrelated repair is authorized.

PREFLIGHT: REUSE / BRIEF / DURABLE; IMPLEMENTATION; authority accepted in
base YES; implementation_authority contracts; execution mandate VALID;
mutation authorization VALID; isolated surface YES; load-bearing SPEC_GAP
NONE; evidence reviewability PASS; live authority gap NONE; production
operation NO; merge readiness NO pending executed gates and independent
review. Authorities and unchanged boundaries remain those in the R2 Brief.

Coordinates: repository mayf3/svc-workflow; branch
fix/outbox-channel-isolation; start HEAD
8aa864ab9ff70107faf8e016a04d32f4e78ad141; implementation
062fe82301eadbf23b5b6279643355c20ca4218c; integration base and freshly
read remote main 8da8dbcb70e700d9e35970e8b662b1be2c29afdc.
Vendored governance integrity/adoption check rc0; authority paths have no
diff from integration base. R2 STOP remains historical, superseded only
for this explicitly authorized continuation.

The worktree starts clean and all R2 work is committed, so no dirty changes
exist to stash. Preserve that branch and all old evidence. Use detached
scratch worktrees at exact base and candidate, transplanting only the
candidate test fixture onto base: equivalent product-change isolation
without moving or rewriting the active branch or any existing stash.

Original tests on base may fail at healthy-channel latency before reaching
backoff. Record exact failure sites. Then an identical diagnostic-only
single-channel, single-run_once probe on both sources reaches the original
backoff assertion after the real timeout. Its test-only branch and hashes
are retained separately, and never count as channel-isolation acceptance.

Done when: attributable comparison; contract-preserving correction;
focused/full executed rc0; independent exact-head review; pushed unmerged
main-base PR and receipt. Unexpected failure or a changed accepted contract
stops dependent work and updates the stop record. No permission is needed
to execute the already requested bounded continuation.

## End checkpoint

STOPPED: full cargo rc101 at binary45 fixture-name guard. Outbox22/22rc0
and original three-failure attribution complete. No runner repair/retry or
commit/push/PR after failure. See root STOPPED/RECEIPT and REVIEW-R3.md.
