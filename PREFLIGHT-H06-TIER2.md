# H06 Tier-2 isolated conformance preflight and one-attempt runbook

Recorded 2026-11-06. Persistent attributable execution mandate:
`BRIEF-H06-TIER2.md`, supplied verbatim by the user. The Owner explicitly allows
an equivalent isolated PostgreSQL environment and minimal runner adaptation,
with unchanged test semantics/assertions, no production DB/service writes,
stop on failure, and shutdown with data retained. This supersedes only the
earlier H06 R2 turn's prohibition on running Tier-2; its evidence stays intact.

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
TARGET_REPOSITORY = mayf3/svc-workflow
REVIEW_TARGET_HEAD = 5a7dc9ee2dde1dd730dae94afb39f116160aec2c
BASE_HEAD = 59daec042786f0416a6a9061c0b638ba56c0d83e
CURRENT_BASE_HEAD = 59daec042786f0416a6a9061c0b638ba56c0d83e
ROUTE_STAGE = OPERATION (isolated candidate validation only)
AUTHORITY_ACCEPTED_IN_BASE = YES (governance and unchanged invariants)
GOAL_OR_TARGET = run the existing Tier-2 suite once on fresh isolated PG16
CURRENT_GAP = prior Tier-2 NOT_RUN because Docker was unavailable
AUTHORITY_ACTION = REUSE
PRIMARY_AUTHORITY = SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1@BASE_HEAD
RELATED_AUTHORITIES = Product V13 CTR-V6-035..042; Architecture V0.4.7 CTR-ARCH-026
IMPLEMENTATION_AUTHORITY = not_applicable (no product behavior implementation)
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = NO
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = CONTROLLED (DB writes confined to disposable test cluster)
EXECUTION_MANDATE = VALID (Owner/user, 2026-11-06, one local attempt)
MUTATION_AUTHORIZATION = VALID
ISOLATED_WRITE_SURFACE = YES
CONTROLLED_RUNBOOK_REQUIRED = YES (below)
SPEC_GAP_DEPENDENCY = NONE for executing unchanged candidate tests
EVIDENCE_REVIEWABILITY = PASS (raw output, hashes, diff and receipt retained)
LIVE_AUTHORITY_GAP = NONE claimed (no production state inspected)
OWNER_DECISION_REQUIRED = NO
EMERGENCY_STATE = NONE
BASE_IMPACT = NONE (GitHub main and candidate freshly matched local refs)
IMPLEMENTATION_ALLOWED = NO (product semantics)
MERGE_READY = NO (pending results, independent review and maintainer acceptance)
OPERATION_ALLOWED = YES (only this isolated conformance attempt)
NEXT_ACTION = CONTINUE
```

Accepted adoption commit `f4a591af81c2f40f330d8254fc2aaa9700270eb7` is an
ancestor of main; `verify_governance.py --target . --require-accepted` rc0.
Relevant authority bytes are identical between base and candidate. The cutover
child Spec is still `proposed / implementation_authority: none`; no acceptance,
product implementation, deployment or production apply authority is inferred.
This operation validates the already existing mechanical candidate under the
explicit Owner mandate; it does not change any long-lived Product Contract.

Writer: root agent, existing isolated worktree `/private/tmp/svc-h06-manifest`,
branch `ac-h06-migration-manifest`; reviewers are read-only. Existing processes
on TCP 55432 and 55439 were observed without DB connections and will be left
untouched. PostgreSQL Homebrew 16.15 tools exist. Both fixture paths were read
and hash to `0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606`
(540472 bytes). Only fixture file reads occur in the shared source checkout.

One-attempt runbook:

1. Require `/private/tmp/h06-tier2-pg` and `/private/tmp/h06-tier2-evidence`
   absent; create evidence dir 0700. Preserve exact pre-run runner diff, source
   hashes, Git coordinates and tool versions. Use a clean explicit subprocess
   environment, with no inherited DB URL, PG service, credential or dotenv.
2. `initdb -D /private/tmp/h06-tier2-pg -U postgres --auth-local=trust
   --auth-host=reject --encoding=UTF8 --locale=en_US.UTF-8` using the explicit
   `/opt/homebrew/opt/postgresql@16/bin` toolchain. No system service registration.
3. Start with `pg_ctl -D /private/tmp/h06-tier2-pg -l
   /private/tmp/h06-tier2-evidence/postgres.log -w -t 30 -o
   "-p 55487 -h '' -k /private/tmp/h06-tier2-pg -c unix_socket_permissions=0700 -c log_connections=on -c log_disconnections=on" start`.
   This enables no TCP listener. Read back data directory, version, port,
   socket directory, permissions, listen address, DB/user and null server IP
   using the explicit local socket before any test DB writes.
4. Execute exactly `scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh`
   with `TRUSTED_FLEET_LOCAL_PG=1`. The bounded opt-in changes only Docker
   lifecycle/DB creation and socket URLs. Its identity check precedes writes.
   Keep original migration loop, Rust cfg, locked Cargo command, test thread
   count, test source, frozen fixture and every assertion unchanged.
5. Capture complete merged stdout/stderr and exact rc. A nonzero rc is terminal:
   do not retry or repair assertions; write `STOPPED-H06-TIER2.md`.
6. In a finally path run explicit `pg_ctl ... -m fast -w -t 30 stop` on this
   data directory only. Retain cluster and all logs; verify status reports no
   server, postmaster PID/socket are absent. Independently review diff/results.
7. Write `RECEIPT-H06-TIER2.md`, commit, push only GitHub origin branch and
   update PR #129 plus agent-control #1360. If a gh submission times out,
   preserve commit, stop further writes/submissions and report UNKNOWN outcome.

Secrets: none required; trust auth is confined to a 0700 socket/data directory
owned by the local user. No .env read or production credentials passed.
EVIDENCE_NEEDED: exact diff/fixture hashes, PG identity/start/stop, complete suite
output with rc, assertion/test count, independent post-state verification.
DONE_WHEN: one successful suite, stopped retained cluster, reviewable committed
receipt and requested PR/issue update (or explicit timeout stop).
EXPANSION_TRIGGER: fixture/source/assertion drift, identity mismatch, required
gate failure, attempted non-isolated DB/service access or unknown gh write.
Convergence: RETAIN both existing consumers and historical frozen mappings;
no new identity model, migration, replay implementation or permanent fallback.
Actual suite coverage is three tests, including one canary DB scenario; it
does not establish full fleet apply, all recovery integrations or production
business acceptance.
