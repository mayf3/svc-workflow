---
record_id: A75_OWNER_OBLIGATION_NEW_EVIDENCE_20261029
status: investigation
owning_repository: CROSS_REPOSITORY
observed_run: trial-20261027-a75
source_revisions:
  core: 45268be276276462bd71040f0cd51b28a7764fe0
  svc: f5e7f328ccba5f6e0d752abf74e68558ef38ca0f
---

# A75 Owner obligation — new evidence and limits

## Observation

The read-only native audit for `trial-20261027-a75` records a normally completed Owner turn after supplier creation, with no ordinary consumer create and no governance-child create-and-bind attempt. The supplier reached its terminal node with a distinct QA submission. The source remained `source_blocked`; its Assistance case remained `OWNER_PENDING`. The final business verdict is `PARTIAL_UNATTENDED_NONPROGRESS`, while the runtime naturally exited with actual code 0. The actual exit of the outer infrastructure supervisor remains `UNKNOWN_NO_EXTERNAL_WAITER`.

The read-only Svc snapshot records the delivered `owner-assistance:<case>` wake and the still-pending case. This confirms the distinction `wake delivered != owner turn quiescent != business obligation satisfied`. It does not establish why the model stopped, and it does not support a turn-call cap hypothesis.

Neither dependent readiness observation was exercised: there is no consumer create receipt or governance-child create-and-bind receipt. `WAITING`, a zero-attempt interval, the later paired poller `READY` row, and a first dependent attempt are all `NOT_EXERCISED / UNKNOWN`. An empty readiness list is not evidence of `WAITING`.

## Evidence pins

- Retained A75 isolated DB read-only preflight: `ART/a75-owner-wake-mechanism-diagnosis-20261029/owner-obligation-preimplementation-readonly-db.json`, SHA-256 `eb8823f10417889782069002f79ab4d76ae04482915486bf8153be03d2ae11c6`. In that sampled A75 DB, existing `pm` was an enabled AGENT with `GLOBAL_SCHEDULER_READ`, no Domain membership; the Domain was enabled and its single enabled Owner matched the public identity. This is retained-fixture evidence only, not production or credential-isolation evidence.

- Independent design counter-review: `ART/a75-owner-wake-mechanism-diagnosis-20261029/independent-owner-obligation-design-review.md`, SHA-256 `40c662f1a6d5fe3cea419a39684d2e5a2e8722acd124858379b1759e1cb60c1f`.
- Native audit evidence: `ART/g6-native-audit-a75-20261029/evidence.json`, SHA-256 `cc053d9934847dbbf35a6e44a78463af2f9548f69145f63bd1fdd49576d4de41`.
- Native audit report: `ART/g6-native-audit-a75-20261029/final-audit.md`, SHA-256 `afa991636499f80394745fb5877a810f5d87288ded0d8b14745d9321afb170f0`.
- Read-only Svc case/outbox snapshot: `ART/a75-owner-wake-mechanism-diagnosis-20261029/root-readonly-owner-wake-snapshot.json`, SHA-256 `c167561b72c69b656b673d0218fa88e0881376654f1dce84494d72af71003210`.
- Natural runtime lifecycle receipt: `ART/a75-prepare-preflight-20261029/a75-final-lifecycle.json`, SHA-256 `6ad150c3742a4f9f66891d3e6b700a5fb01ed7988b3bb8730dafff225fa50b75`.
- Read-only preimplementation source/identity cross-check: `ART/a75-owner-wake-mechanism-diagnosis-20261029/owner-obligation-preimplementation-readonly.json`, SHA-256 `da360ea7d9853977eab830289dba3febdf79f923ee904018420b349abc82c548`; report `owner-obligation-preimplementation-readonly.md`, SHA-256 `735fafa122620cfec384da4faef98daf85ad1013968238bde65eabbaae6e7657`. These pin exact Core/native source bytes and the retained identity/Owner sample; they are not runtime activation evidence.
- Accepted native authority: `AGENT_CORE_TRUSTED_NATIVE_TOOL_POLICY_V1` at accepted Core base `45268be276276462bd71040f0cd51b28a7764fe0`, §§2–4; exact authority file SHA-256 `3173692fb7d7fb8580fc09367c1f7921f02e6a0369354b7736603bf2ff9024bf`.
- Frozen Native implementation source coordinate: `43ec34b584bf9f11920bb083383a63ecd4f115bd`; exact implementation-note, final-dispatch guard, and selector source hashes are listed below from the read-only sourcePins.
- Pinned policy-source bytes from the read-only preimplementation sourcePins: Core `packages/agent-provisioning/src/trusted-native-tool-policy.js` SHA-256 `474b618c2e13b5171dc6bac5ecc560e436dd1cf868b0917712d438d5be892c08` and `packages/agent-router/src/process/agent-process.js` SHA-256 `11b60fb0dec1c74d7ede2286978f73964a9d699f01d164d2243d4363771727e4`; Native implemented note SHA-256 `aa5cef86bd748d8f8656acec0802e0f5cd48d2f2a80daad896ba5ddb655e6a61`; final dispatch guard `packages/core/tools/src/index.ts` SHA-256 `0b77dd507b2dad9432be57f9350b96c1c203baa74bc5b75569dacc5b19b53c38` and exact selector `packages/core/tools/src/trusted-native-policy.ts` SHA-256 `8fe67f34cf90f1e26ed52da3fa1bbe0743b0dc050eea34d14300cc34cf23f2d3`.

## Interpretation and limits

This is new evidence of an unmet Owner obligation after a normally completed turn, not evidence of an API refusal or of an automatic retry defect. It supports proposing a system-owned bootstrap responsibility that creates one exact published governance intake Workflow, plus a distinct real review Workflow after the intake child reaches its proven terminal transition. Ordinary multi-step business work remains expressed in the existing published Workflow graph and existing QA/RETURN machinery.

The evidence does not authorize replay, reclassification, repair, or mutation of A75, its database, its source Instances, or its existing receipts. It does not prove `WAITING` or `READY` for either missing dependent. The outer supervisor exit remains unknown even though runtime, worker, and service child exits were observed as zero.

## Proposed investigation disposition

Use this evidence as `NEW_EVIDENCE` for a docs-first paired authority proposal. Keep the design recommendation non-authoritative until exact proposed Contracts receive independent review and Owner acceptance. The proposal must not add a parallel flat catalog/template/materializer/completeness language or a Core phase-specific prompt/continuation engine. The exact persisted fields, effect keys, service/API mapping, producer/discovery rules, and storage/cardinality contract are now fixed by the pinned design plus three author addenda; independent paired review and Owner acceptance remain pending. A75's observed WAIT/READY and unattended business completion remain unexercised.
