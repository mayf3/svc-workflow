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


## Follow-up observations and independent preflight result

OBS-WACI-007: current-model-and-input-facts.json confirms all 203 current items are model1; definition-version-models.json confirms all 55 bad published source versions are model1 (all published:99 model1,1 model3). Source repair cannot silently create new Legacy intake through cloned replacement model1 versions.

OBS-WACI-008: fresh protected runtime-workspace-snapshot.json has 88 mappings, SHA256315ac692a7b4c0074ec092a5d961d7ef818e5a9189d78dba2e86cd5cb49411dc. HR/writer exact historical paths match production. CTO has no override, so falls back to its runtime-owned workspace. This does not negate the separately accepted exact CTO identity pair.

OBS-WACI-009: 2026-09-06T18:36:08+08 CTO exception readback: OLD enabled role bindings9, NEW0, ever-assigned instances58, OLD historical visits112. Original accepted plan requires111 visits. Therefore original count eligibility fails before apply; do not weaken its gate or replace its frozen history silently. Provenance cto-exception-eligibility-counts.json.

Independent semantic preflight at74ec339166969d699cbec74e9edc3c159a19e757 is recorded in semantic-preflight-review.md. It confirms no REUSE for the new responsibility tuples; a model1 exact-plan same-node successor exception would need Product/Architecture successor plus implementation child. Null old external_ref alone is not ambiguity: qualified exact Auth agent_id, historical registry/workspace, recovery successor binding and current canonical Agent can establish equivalence. Review is not Spec acceptance or implementation audit.

Next authoring must jointly close current responsibility and source behavior. Do not take a same-node visit repair as sufficient if immutable source assignments can recreate drift on later transitions. Evaluate the already designed explicit model3 successor-instance migration against the minimal exact affected graphs; if a new bounded Legacy exception is necessary, expose that semantic delta for Owner exact-head acceptance. Do not build a generic migration engine or silently widen identity reconciliation into a mass workflow-model conversion. Canonical future sources, preserved existing business semantics and immutable history must all be shown in the concrete plan.
