# #683 release realm repair: current-main revalidation

The requested source repair is already present in main. [PR #77](https://github.com/mayf3/svc-workflow/pull/77) merged on 2026-10-08 at `2b746b5d478d8f98db9d7fc6658dd1460706d6bf`, an ancestor of the freshly fetched main `3b81d8f99a286caeb04ea2c349966b0f6662275d`. Both affected scripts are byte-identical to its final head `4a7e8191d6bca800786f32848453a8c094eac756` (`git diff --exit-code` = 0). This delivery preserves that implementation and records fresh isolated evidence on the requested branch `ac-task/683-release-realm-fix`.

No source, behavior-defining test, service configuration, binary, migration, production ledger, #682/#668 maintainer contract, Broker, or B7 change is included. No production command, sudo, deployment, restart, merge, or issue closure was performed in this revalidation.

## Route and authority

- Mandate: user's bounded #683 brief and [mayf3/agent-control#683](https://github.com/mayf3/agent-control/issues/683); persist source conformance evidence, open a PR, comment delivery, stop before merge.
- Target/base: `mayf3/svc-workflow`, `3b81d8f99a286caeb04ea2c349966b0f6662275d`; isolated temporary clone `/tmp/sw-367`, named branch above, initially clean, one intended tree.
- Governance: `.agents/governance.lock.json` is accepted on main; `python3 .agents/tools/verify_governance.py --target . --require-accepted` exited 0. Product V13, Architecture V0.4.7 and accepted governance adoption were read. Scoped discovery found no accepted release-specific implementation-authorizing Spec; this report creates no Product Contract and authorizes no new semantic implementation.
- `SPEC_GOVERNANCE_MODE=PREFLIGHT`, `AUTHORITY_ACTION=REUSE` for the existing governance/evidence route, `PLAN_LEVEL=BRIEF`, `ASSURANCE_LEVEL=DURABLE`, `ROUTE_STAGE=IMPLEMENTATION` (existing source inspection only), `AUTHORITY_ACCEPTED_IN_BASE=YES` for governance, `IMPLEMENTATION_AUTHORITY=not_applicable`, `IMPLEMENTATION_ALLOWED=NO` for new behavior, `OPERATION_ALLOWED=NO`, `MUTATION_AUTHORIZATION=VALID` for this evidence-only delivery. No implementation-authority claim is made for release behavior.
- Done when: evidence and non-author review are reviewable, PR is open, #683 delivery is commented; stop without merge. A new behavioral gap would require re-PREFLIGHT and an accepted governing implementation Spec in base before code/test changes.

## Exact existing code

Coordinates below refer to main `3b81d8f`, with unchanged source bytes in this PR.

| Surface | Code | Observed behavior |
| --- | --- | --- |
| Realm resolution | `scripts/release.sh:151-204` | Probe both `gui/$EXPECTED_SERVICE_UID/com.svc-workflow` and `system/com.svc-workflow`; reject unknown/denied, missing or multiple units before installation. |
| Identity and install target | `scripts/release.sh:206-236` | Check unit program, running UID and actual executable path; stopped units require configured UID. System control requires root; root cannot substitute for service UID. |
| Persistent binding and preimage | `scripts/release.sh:377-442` | Invoke resolver, validate active-ledger/fixed target and mandatory existing binary digest before backup/install. First update without active ledger needs an approved explicit target pin. |
| First service-tree writes | `scripts/release.sh:451-466` | Backup and installation occur after admission. |
| Restart and verify | `scripts/release.sh:496-500`, `518-543` | Restart uses resolved target; verify reads the recorded binding and rejects absent/substitute realm. |
| Rollback | `deploy <oldSha>`, same lines above | The script has no separate rollback subcommand. Restoring the old immutable release uses the same provenance, realm, UID, binding and preimage gates. |

SHA-256:

```text
7fec7e8c3476779ed773a42c7e03a14bc6c885b75eba2217b4545623f2e3fc68  scripts/release.sh
6dd4a4f6f5509fb431817ee4be60b4afef8f11f71b4a6a967a1f2f648226acbb  scripts/test-release-target-resolution.sh
```

## Fresh executed evidence

[Executed receipt](evidence/ac683-release-realm-20261031/executed-receipt.json) retains source coordinates, source digests, original subprocess outputs, per-case exit files and fingerprint digests, and the exact paired driver tail derived from the unchanged existing fixture. All launchctl, id, ps, lsof and curl calls use fixture commands; all binary/migration/ledger writes use temporary service roots. UID0 in the table is simulated by the fake `id`, without real privilege elevation.

| Source / synthetic system-only UID502 scenario | Release exit | Service-tree fingerprint | Fake service control |
| --- | --- | --- | --- |
| Original `18fb2f8b`, caller502, deploy new artifact | 1 | Changed after binary/migrations install and ledger append | None; missing GUI check fails after writes |
| Original `18fb2f8b`, caller502, restore old artifact | 1 | Changed after binary/migrations install and ledger append | None; same late GUI failure |
| Current main, caller502, deploy / restore (2 executions) | 1 each | Unchanged in each execution | None; system/root admission rejects before writes |
| Current main, caller0 + expected UID502, deploy / restore (2 executions) | 0 each | Expected staged installation changes | Exactly one system kickstart per execution; no GUI kickstart |

These are fresh paired RED→GREEN observations of the requested failure ordering, not the historical author's RED counts. The old release has different installed and target binary contents in each case; restore uses the real old source SHA and release directory. The paired orchestrator separately checked release rc files, nonempty before/after fingerprints, and every fake kickstart entry. All six execution expectations passed.

Existing relevant verification:

- `bash -n scripts/release.sh scripts/test-release-target-resolution.sh`: exit 0.
- `KEEP_WORK=1 bash scripts/test-release-target-resolution.sh`: exit 0, **80 passed, 0 failed**. Covers system routing, two realms, missing/denied units, UID/program/executable/preimage mismatch, fixed-target drift, stopped-unit restore, distinct old/new artifacts, ledger/verification and archive/switch cases. No existing relevant failure observed. Rust/HTTP/database suites were not run for this evidence-only delta.
- Governance distribution/accepted-adoption check above: exit 0.

Fixture limit: most negative cases fingerprint only the staged service tree; the fake kickstart log is outside that tree, so those assertions alone do not prove zero service-control calls. Static ordering and the additional paired log checks support that narrower claim. Z1/Z2 deliberately invalidate fingerprint files and Z3 removes its rc file to test the assertion helpers; their resulting raw-file state must not be counted as application failures.

## Independent review

Non-author agent `/root/independent_realm_review` independently read the governance entrypoints and REVIEW mode, inspected exact main `3b81d8f` and the two digests above, ran shell syntax validation, and independently executed the inspected fixture: exit 0, **80 passed, 0 failed**. It found no blocking defect in the requested source scope, and identified the fingerprint/control-log coverage limit stated above. Its retained local fixture root is `/var/folders/jh/5k4x44w12ps1kbrch8__f6b80000gp/T/release-fix-test.OZLBu9`. This Agent review is not an Owner acceptance or GitHub APPROVED state. Final report-head review is recorded on the delivery PR.

Historical independent review is separately attributable to [#697's terminal exact-head receipt](https://github.com/mayf3/agent-control/issues/697#issuecomment-6071109612) and [final reconciliation](https://github.com/mayf3/agent-control/issues/697#issuecomment-6071429126). Those records report full-delta review and RED65 against the old base; [#696](https://github.com/mayf3/agent-control/issues/696#issuecomment-6070819672) reports final-delta RED16 and GREEN80. They are historical linked statements, not newly executed raw evidence here. PR77's old description still mentions an earlier 11-test stage; the source hashes and fresh receipts above identify the actual reviewed version.

## REALM-MISMATCH scenario and stop boundary

The issue's original scenario is system/com.svc-workflow, service UID502, with gui/502 absent (113). The current source and synthetic fixtures close its source failure: the ordinary UID502 caller cannot write before failing system-control admission; an authorized root caller with explicit service UID, approved fixed system binding and matching preimage routes to system for deploy and restore. Ambiguous/unknown targets remain pre-write refusals.

The original `task-2/workflow-release-readonly/REALM-MISMATCH.json` was not found in available relevant workspace/task inventories. Its raw bytes/hash and the original production artifact hashes were not independently revalidated. Scenario linkage here is to the issue description, existing fixture shape, exact source, and fresh synthetic observation. Live service state, actual installed artifacts, original business graph/Broker/shopping/blog acceptance and actual production rollback remain **UNKNOWN in this session**. Historical issue claims do not become fresh runtime evidence.

Deliver this report PR and comment to **mayf3/agent-control#683**; retain that issue OPEN for its broader existing runtime/business closure. Stop before merging or production action.
