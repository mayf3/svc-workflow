# Workflow assignee canonical identity reconciliation investigation

Disposition: evidence gathering / authority gap; no implementation or production apply.
Goal: WORKFLOW_ASSIGNEE_CANONICAL_IDENTITY_RECONCILIATION_V1.
Source base: svc e297ff1f3913133058d97bb30bcf8f63b3e137f9; dsh a95410e6c771c11f786a2f9a024b18931fecfdb4.
Observed: 2026-09-06. Production reads are point-in-time observations, not durable readiness.
Raw restricted evidence: /Users/yanfenma/workspace/deployment-artifacts/workflow-assignee-reconciliation-01a07639/.

## Observations

OBS-WACI-001: deployed Workflow database read-only snapshot contains 203 current nonterminal, noncancelled, unarchived assigned work rows, all Workflow AGENT type and ACTIONABLE_NOW under the shared eligibility formula. 3 belong to disabled Domains. census-classified.json enumerates exact Instance/Visit/Principal/version/source coordinates. Auth and registry snapshots are sequential, not atomic cross-service observations.

OBS-WACI-002: exact Auth UUID/type/status/agent_id/cardinality and production Agent Definition id/disabled composition classifies 81 structurally canonical, 29 legacy Agent identities, 93 Auth-missing Principal assignments. This is not actual agent_resolve_principal invocation evidence. No display-name matching was used.

OBS-WACI-003: 366 published, enabled-Domain, nonarchived Definition node sources (328 FIXED_PRINCIPAL, 38 DOMAIN_OWNER) are noncanonical; they span 55 version IDs and 39 distinct source Principal IDs. All published versions are included, not only latest. Creator/input sources remain separate. Source corpus and per-Principal successor coverage are in definition-sources.json and source-successor-coverage.json. No definition was changed.

OBS-WACI-004: the old fleet plan bytes still match SHA256 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606, size 540472. Historical/local exact workspace plus Auth external_ref readback validates 76 of its 86 pairs under the strict conjunction recorded in successor-equivalence-readback.json. The remaining ten fail that conjunction: nine have null old Auth external_ref and one efficiency pair uses a documented external_ref alias instead of old agent_id as the historical registry key. These are evidence gaps under this check, not proof of inequivalence. No new mapping may be inferred by prefix or display name.

OBS-WACI-005: current writer and CTO instances occur in the old fleet responsibility plan but do not match its currentVisitId or expected state version. The original planned responsibility belonged to a different node/Agent. Three current old-HR items do not occur in its frozen responsibility set. legacy-plan-scope-comparison.json records each comparison. Matching Instance ID alone cannot authorize a successor.

OBS-WACI-006: independent exact UUID search in source tests/scripts/migrations/docs found source references for CTO 1 item and old HR 3 items, but no source provenance for the other 118 noncanonical items. Random UUID generation or e2e-like names do not prove fixture provenance. invalid-provenance-report.md records limits. No item is deleted, cancelled, archived, hidden from census, or treated as repaired.

## Claims and evidence relations

CLM-WACI-001 (SUPPORTED): this is a source plus stored-assignment problem; changing HR name resolution is not a valid repair. EVD-WACI-001 links OBS-001/002/003 to this claim at the pinned source and production snapshot; sufficient for classification and source drift, insufficient for actual Broker success.

CLM-WACI-002 (SUPPORTED): the historical fleet responsibility plan cannot be replayed on the current affected rows. EVD-WACI-002 links OBS-004/005 to this claim; exact current Visit/version mismatches and proposed child authority rule out automatic apply.

CLM-WACI-003 (OPEN_ASSUMPTION): ordinary published-version replacement plus a bounded new active-responsibility exception can close all genuinely valid work. Current ownership, graph model compatibility, exact successor equivalence, invalid-source treatment and parent authority require closure first. No implementation is authorized by this claim.

## Authority inventory

- Product Boundary V6 and Architecture 0.4.0 are accepted upper authorities; their exact CTO/fleet exceptions cannot be generalized. Architecture section 5.15 retains Legacy-only exceptions and forbids general/new-model same-Visit reassignment.
- SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1 is accepted for the exact CTO pair 3e2439d2-fb54-44f5-afee-77aa17c40d22 -> 4e5a4578-0645-4133-bd35-b80e453dfee9 and original 9 Domain/1 responsibility facts. The expected one-time binary is absent at this base. Current eligibility and exact history invariants still require revalidation.
- SVC_WORKFLOW_TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1 is proposed with implementation_authority:none. Binary presence and old plan are not acceptance.
- dsh exact resolver V2 and Definition authoring V2 are accepted for their limited read/lifecycle capabilities. Auth owns Principal-to-agent_id; Agent Definition owns exact Agent existence/enabled.
- Definition service contract permits draft replacement/new version/publish and deprecation, preserves published graph immutability and existing Instance version identity. Broker has no deprecate/revoke operation.
- Admin MOVE_TO_NODE resolves the target under the existing definition and cannot inject arbitrary NEW_ASSIGNEE. Required-input repair cannot overwrite already set inputs.

## DEVELOPMENT_PREFLIGHT

SPEC_GOVERNANCE_MODE = PREFLIGHT
PREFLIGHT_MODE = SUPERSEDE
CHANGE_CLASS = NON_MECHANICAL
GOVERNANCE_ADOPTION_STATUS = accepted
BASE_COMMIT = e297ff1f3913133058d97bb30bcf8f63b3e137f9
PRIMARY_GOVERNING_SPEC = NONE for current cross-pair responsibility repair
RELATED_ACCEPTED_AUTHORITIES = Product V6; Architecture 0.4.0; exact CTO successor; Definition service; dsh resolver/authoring V2
IMPLEMENTATION_AUTHORITY = none for the new repair set
AUTHORITY_CONFLICT = proposed repair extends digest-frozen responsibility scope and may extend parent reassignment exceptions
IMPLEMENTATION_ALLOWED = NO
NEXT_ACTION = resolve exact source/equivalence and row/model plan; independently assess parent successor versus existing exception reuse; author only necessary docs-only authority after that assessment

This preflight classification is the author's assessment pending independent semantic review. No accepted Contract has been edited.

## Frozen prohibitions and next evidence

No display-name/normalized-prefix mapping, no rewriting old Principal agent_id to merge identities, no overwriting published graph/old Visit/context/receipt, no swapping Instance DefinitionVersion, no arbitrary reassignment API, no generic migration framework, no proposed fleet operator replay. Preserve invalid rows as explicit no-send classifications until governed disposition is known.

Next: obtain exact runtime workspace binding and qualified equivalence where old external_ref is null; verify CTO accepted pair current history/gates; distinguish all source semantic model versions and future creation validation; build a reviewed exact source/active-instance plan with atomic append-only facts. Only then request the irreducible exact-head semantic acceptance. Actual HR >=3 real items across >=2 Domains and >=2 Agents remains entirely unverified.
