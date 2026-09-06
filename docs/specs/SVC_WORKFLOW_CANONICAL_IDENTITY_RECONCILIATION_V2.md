---
spec_id: SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: conditional_controlled_operation
scope:
  - mayf3/svc-workflow
  - frozen identity-only source and active-work reconciliation
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V7
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_1
external_authorities:
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_EXACT_PRINCIPAL_AGENT_RESOLUTION_V2
    revision: a95410e6c771c11f786a2f9a024b18931fecfdb4
    relation: constrained_by
  - repository: mayf3/auth-service
    authority_id: AUTH_SERVICE_INTERNAL_IDENTITY_DIRECTORY_V1
    revision: e8ae1949ef330bd44b0e0d185053b39e18cab3f3
    relation: depends_on
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_INTERNAL_AGENT_DIRECTORY_V1
    revision: a7dbaafde211dd67381bcb623fe91b6339cb917e
    relation: depends_on
supersedes:
  - SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V1
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2

## 1. Goal

Repair exact Workflow assignee identity and its generators while retaining prior business behavior. Owner clarified on 2026-09-06: keep personnel selection as before. Source topology, ADVANCE/RETURN choices, draft/revise behavior and per-creation owner inputs are not incidental details to remove during identity repair.

This is a whole-authority successor of the accepted V1 (2026-09-06, merged via PR #29). Successor reason: Owner product direction 2026-09-06 changed the admission dependency model — canonical identity lookup is foundational internal directory information available to any authenticated canonical internal Agent/SERVICE caller, and the dedicated Workflow admission SERVICE Principal/Client/Grant model pinned by V1's CTR-CIR-003 is SUPERSEDED_DIRECTION_PENDING (its objects were never provisioned; nothing is deleted; the implemented exact-lookup logic is preserved for reuse). Except for the successor delta recorded in section 14, every V1 contract, decision and acceptance row is carried forward verbatim.

This candidate becomes implementation authority only after independently reviewed exact-head Owner acceptance and merge into the implementation base. No production mutation is performed by authoring or acceptance.

## 2. Scope and non-goals

Closed input scope is `docs/evidence/workflow-canonical-identity-reconciliation-v1/scope.json`, SHA256 `99e35b9b64275787e8516d7df4cefa7e147e6a940b68e3b69c4c89652b1338c0`. It lists 203 observed current Instance/Visit/version tuples, 55 affected published source versions and 69 total inspection versions. It is an upper bound, not a requirement to migrate all rows. Every current/future reachable owner of each considered instance is inspected; a currently canonical owner does not excuse a stale subsequent source.

No arbitrary UUID arguments, dynamic fleet enumeration, generic migration framework, fuzzy/normalized mapping, new identity authority, Agent/Client/credential/Grant changes, completed-work reactivation, ordinary reassignment API, or model3 conversion. Proven invalid assignments remain classified and unsent; classification does not authorize cancellation, archive, deletion or fixture labeling. New discoveries outside the frozen scope cannot silently enter apply.

## 3. Authority and dependencies

Product V7 and Architecture0.4.1 fully replace their parents only after acceptance; their WACI exception permits identity-corrected model1 sources and exact same-model successor Instances. V6/0.4.0 remain active until then. Original CTO and fleet authorities are not expanded or replayed: CTO history drift112 versus111 and current fleet Visit/version drift are recorded blockers to their reuse.

Auth remains sole Principal mapping owner. The dsh exact resolver V2 owns the composed read and canonical Agent enabled check. This child consumes qualified exact identity evidence; it does not redefine that resolver or grant its permission. Normal Workflow Domain/actor checks remain in force. The actual caller/repair actor identity, credential access and production endpoint must be freshly proven before apply. An absent Broker tool in a Codex task is not permission to impersonate HR.

## 4. Current State

STATE-WACI-001: production at 2026-09-06 census, not a live lease:203 current assigned model1 items;81 structurally canonical,29 legacy Agent identities,93 Auth-missing;3 disabled-Domain items. All55 problematic published source versions are model1;9 use instance-input owners and2 have multiple ADVANCE choices. Qualified source evidence is the investigation at `a4a5900` and restricted evidence directory identified there. Source main is `e297ff1f3913133058d97bb30bcf8f63b3e137f9`; no equality with deployed binary is implied.

## 5. Observations

OBS-CIR-001: exact census/registry/source graphs enumerate the state above. OBS-CIR-002: current writer/CTO Instance IDs overlap historical plans but current Visit/version and responsibility differ. OBS-CIR-003: Owner requires original personnel flexibility; model3 owner/topology constraints would change11 affected source versions. OBS-CIR-004: exact UUID source search cannot establish fixture provenance for118 noncanonical items; names are insufficient. Provenance: investigation and `AUTHORITY_ROUTE_DECISION_PACKET.md`, `source-graph-conservation-matrix.json`, `parameterized-source-decision-scope.json`, `cto-exception-eligibility-counts.json` in the restricted evidence directory.

## 6. Claims and assumptions

CLM-CIR-001 SUPPORTED: a current-Visit-only repair leaves generators/future transitions capable of recreating drift (OBS-001/002). CLM-CIR-002 SUPPORTED: an identity-only Legacy exception is required to preserve these graphs without workflow redesign (OBS-003 and Owner clarification). CLM-CIR-003 OPEN_ASSUMPTION: every genuinely valid affected identity/source can be reconciled automatically. Missing equivalence or business provenance remains explicitly unresolved; implementation cannot force this assumption true.

## 7. Evidence relations

EVD-CIR-001 links OBS-001/002 to CLM-001 at the frozen source/production coordinates: sufficient for source drift and no-replay, not mutation authority. EVD-CIR-002 links OBS-003 to CLM-002: sufficient to reject silent model conversion, not to accept proposed parents. EVD-CIR-003 links OBS-004 against assuming all invalid rows are fixtures. No test or plan substitutes for fresh runtime acceptance.

## 8. Decisions

DEC-CIR-001: perform one bounded exact identity reconciliation; freeze source and instance plans before apply. DEC-CIR-002: publish identity-only corrected model1 source versions under the finite parent exception, retaining owner inputs and all branch choices. DEC-CIR-003: use same-model successor Instances with immutable source lineage, never rebind the original Instance or edit history. DEC-CIR-004: retain source history for read/RETURN/revise semantics through exact lineage, with explicit no-write source closure. DEC-CIR-005: no automated identity-equivalence or business-semantic guesses; batch genuine ambiguities for Owner. These are decisions owned by mayf3 (V1 accepted; carried forward).

## 9. Contracts

### CTR-CIR-001 — Closed read-only plan

The one-time operator has `plan`, `apply --plan <path> --plan-sha256 <digest>`, and `verify --plan <path> --plan-sha256 <digest>`. No OLD/NEW/scope/target/database-discovery arguments. Source scope digest is compiled from the accepted artifact. Plan output lives outside Git and requires exact full code SHA plus a clean checkout. Plan opens read-only consistent Workflow transactions, acquires no mutation lock and emits no audit/receipt/database writes. Read Auth/Agent evidence separately with explicit observation timestamps; no claim of a cross-service atomic snapshot.

Plan must enumerate all current/future reachable owner references in scope, source eligibility, valid/invalid classification and reasons, exact successor equivalence chains, source Domain/version/node/transition/schema/Context digests, target graph/version/Instance/Visit/Context UUIDs, old/current state versions, intended source closure and Domain tuples, historical row identity arrays/digests, and every write/readback/invariant. Canonical JSON bytes and SHA256 are verified before apply. Changed current tuples are conflict; never update the frozen plan during apply. Invalid or ambiguous rows have explicit no-write disposition. Whole-goal success is not inferred from an eligible subset.

### CTR-CIR-002 — Exact identity and permissions

OLD/NEW Principal UUIDs are fixed per plan. Each successor chain must use exact Auth UUID/type/status/agent_id, documented historical identity/binding facts and current exact enabled Agent Definition. Every non-null identity assertion must agree; missing optional old external_ref alone is not ambiguity if the remaining exact chain is sufficient and independently reviewed. Reject display names, prefixes, case normalization, nearest matches, duplicate/corrupt mappings and inferred clients. Require NEW active AGENT, exact canonical agent_id grammar, cardinality one and one enabled Agent.

Do not modify Auth Principal/Agent mappings or copy permissions. Any required Workflow Principal projection and Domain owner/member successor tuple must be explicit in the reviewed plan, mechanically tied to the same business identity, use existing provisioning/unique-owner invariants, and transfer only necessary existing Domain authority. No global role or privilege broadening. Missing authority is a fail-closed plan blocker. Apply actor is the actual independently authorized caller, never OLD or NEW merely because it is a target.

### CTR-CIR-003 — Source graph conservation and future input

For each eligible source, construct an exact new version using ordinary draft/publish validation plus the finite Legacy exception. Compare complete normalized before/after graphs: topology, node/transition keys/types/order, ADVANCE/RETURN edges, schema semantics, context/revise behavior, instructions, artifacts and owner-reference kind must remain identical. Permitted delta is only exact identity-bearing values, new storage UUIDs/version number and named reconciliation provenance. Do not change INSTANCE_INPUT_PRINCIPAL into FIXED_PRINCIPAL, remove a choice, fix a default person, or introduce a new rule about who may choose personnel.

Identity literals inside input defaults/enums/examples and source configuration must be enumerated by exact schema/field path and corrected only with proved equivalence; do not recursively replace arbitrary matching business text. Dynamic owner inputs remain per creation, using exact canonical Principals selected by the ordinary authoring caller. The repair must prove new-instance examples with different canonical owner combinations, not one hard-coded happy path. Invalid/stale input must be rejected before persisting an assignment or identity-bearing Context value. A dispatch-only check is insufficient. Apply this admission rule to corrected-source publish/defaults/enums, direct creation, revision, revise-and-transition, ordinary future transitions, admin moves and the bounded repair operator. Validate all supplied role identities and all identity-bearing values reachable in the resulting Context/configuration, not just the current node owner. A local Workflow projection alone does not prove current Auth/Agent status. Obtain exact authoritative Auth relation/type/status/cardinality plus Agent Definition existence/enabled for each distinct Agent Principal within the command, after request identity/schema authorization and before its database commit. No cached positive result from an earlier command, caller-provided success assertion, or incoming Workflow-audience credential forwarded to another audience is acceptable. The owning dependencies are Auth AUTH_SERVICE_INTERNAL_IDENTITY_DIRECTORY_V1 and dsh AGENT_CORE_INTERNAL_AGENT_DIRECTORY_V1 at their individually accepted exact heads. Workflow authenticates as a normal authenticated canonical internal SERVICE caller: one general backend SERVICE principal and client for svc-workflow (its own service identity, not a per-purpose lookup identity), provisioned through the existing accepted provisioning surface and holding exactly the two generic directory grants (identity-directory / auth.directory.read) and (agent-directory / agent.directory.read); the V1-pinned dedicated admission identity (cedb954a-3d99-4e5a-b568-d312441bcc56 / svc-workflow-canonical-admission-v1 and the two workflow-*-admission audiences) is retired without ever having been provisioned. With a fresh token for each exact read audience per command, call the pinned Auth directory route (GET /api/v1/directory/principals/{principalId}/agent), then the pinned dsh directory route (GET /v1/directory/agents/{agentId}) using that returned exact ID. Admission acceptance requires, for every distinct Agent Principal in the command: the Auth observation with principalStatus == active and its exact agentId, then the agent-core observation with exists == true and enabled == true; a missing/disabled/noncanonical/duplicate identity, timeout or unavailable validator rejects the entire business write. Service URLs are fixed reviewed backend configuration, never supplied by request fields; plain HTTP is allowed only on exact loopback endpoints in the reviewed same-host deployment, redirects are forbidden, and non-loopback configuration requires authenticated TLS. Responses must bind the requested exact UUID/ID; reject unknown fields inconsistent with the pinned response contract. Total admission through commit is bounded to5seconds from the first admission request start using a monotonic clock, with a database statement/transaction deadline no later than that bound. Per-command identical IDs may share the same still-current observation; no reuse across commands. At most8 in-flight read requests; no automatic retry. Lock/check the DefinitionVersion, Domain and relevant Instance/Visit/Context preconditions in the committing transaction and discard observations on any tuple drift. Timeout/unavailable/identity mismatch or transaction failure yields zero business delta, not a deferred invalid assignment. Database commit-response loss is outcome_unknown: inspect the original command Receipt before resubmit. The external Specs are proposed and require independent review/explicit Owner acceptance of the additional directory-read authority; current Goal permission alone is not a grant. No implementation may assume either external dependency is accepted. Normal business actor identity/Domain authorization remains separate from this backend service credential. No display-name fallback is introduced.

Every affected published old version is either paired with an approved corrected version or held as an unresolved source. For exactly the two versions in parent CTR-WACI-010, the reviewed plan may deprecate without publishing a replacement, atomically recording SOURCE_IDENTITY_UNRESOLVED and preserving every task/history fact. No cancel/archive/assignee rewrite or implicit redirect follows. Other unresolved versions fail planning and cannot be silently added to that exception. New intake into a held version fails the existing deprecated-version rule. Its real existing work remains unresolved, not excluded from the final census and not relabeled invalid business merely because identity proof is missing. Successful source activation and predecessor deprecation are one database transaction; old published graphs remain untouched. All authoritative caller source-version references found by census are updated through their owning accepted surface. A deprecated old UUID must fail new creation, never redirect invisibly or fall back to another Legacy source. No current-generation readiness claim while an unresolved bad source remains available to normal creation.

### CTR-CIR-004 — Same-model successor and preserved lineage

For an eligible active model1 Instance, preassign one successor Instance on the approved corrected model1 version and the mapped same logical current node. Preserve source original Instance Definition/model/creator, Context rows, Visits, Submissions, Events, receipts and audit facts byte-for-byte. Create new successor facts; never overwrite or duplicate old historical facts as if they were newly authored.

Create exactly one immutable lineage/closure row keyed by source Instance and migration ID, recording successor, source expected Visit/version, node correspondence, approved identity-valued Context paths, plan digest and command identity. It is Workflow migration provenance, not a Principal resolver or reusable identity map. Source closure blocks all further business writes/dispatch from that source; reads expose its preserved history and exact successor link. Do not report it as ordinary task completion/cancellation or merely archive it while leaving transitions executable.

Successor current Context is a new immutable fact preserving all business values; only reviewed identity-valued fields may differ, with old/new values recorded. Preserve the original artifact binding and its original object identity. The original row retains its external_reference, including the existing unique (Domain, external_reference) reservation. The successor stores external_reference=NULL, not a copied value or generated suffix. Its read projection returns inheritedExternalReference={value, originWorkflowInstanceId, successorWorkflowInstanceId}; the value is derived from the exact source row, never a second mutable field. Existing reference lookup still identifies the source and adds explicit closed-for-reconciliation and successor linkage; commands addressed to the old ID fail with that linkage and never execute by implicit redirect. Creating another Instance with the same Domain/reference still fails uniqueness. Clients must use the explicit successor ID for ongoing work and may display the inherited reference as the same business reference, with origin preserved. All seven observed non-null references require before/after lookup and uniqueness proof in the plan.

For every successor, set created_by_principal_id to the existing valid original creator if unchanged, or to its exact proved canonical Agent successor in the individually approved plan. A missing creator, missing equivalence, disabled canonical creator or unapproved stewardship transfer blocks that row even when the current assignee is repairable. Record originalCreatorPrincipalId from the immutable source, successorCreatorPrincipalId and actualMigrationActorPrincipalId separately. The operator obtains no creator rights. Revision and revise-and-transition require actual authenticated actor == successor.created_by_principal_id plus all ordinary enabled-Domain, DRAFT/current-Visit, expected-version and command authorization predicates. The same predicate applies after a later RETURN to DRAFT. An old creator UUID gains no alias login/authority. The new parent CTR-WACI-008 owns this finite stewardship exception, including individually approved overlaps with the historical99 draft restrictions; the historical fleet operator and its accepted tuple set are not replayed or modified.

### CTR-CIR-005 — History-dependent behavior

A successor retains required upstream evidence through exactly one immutable source link. The apply plan seals the exact source Instance ID, Visit IDs, Submission IDs, Context revision IDs and their row hashes before closure. Eligible source facts must be in this sealed membership, belong to that source and Domain, and have an exact source-node to successor-node key correspondence. Submission membership additionally proves its source Visit, Context and transition belong to the original source graph. No recursive ancestry, caller-supplied ancestry path, arbitrary linked Instance, post-closure source append or copied historical participant grants admission. Any source that is already a WACI successor is ineligible.

The read/RETURN evidence predicate is conjunctive: (1) the actual authenticated actor has current full successor visibility as an enabled Domain Owner, exact current assignee on a consistent nonterminal Visit, or exact successor creator while current node is DRAFT; (2) the Domain is enabled; (3) the evidence satisfies sealed membership and exact structural relations above. These are the only new access paths to source evidence via the successor. Being a historical participant, being a migration operator, knowing the lineage UUID, or being an equivalent identity does not itself satisfy the predicate. Ordinary direct source reads retain their prior authorization and restricted filtering. A restricted historical participant does not become a full successor reader.

Client evidence identifiers preserve true origin: source entries expose originWorkflowInstanceId plus the original nodeVisitId/submissionId/contextRevisionId and an explicit WACI_SOURCE_EVIDENCE tag. Preserve existing RETURN submissionPayload fields rootCauseNodeVisitId and relatedSubmissionIds as UUID references; no new top-level Broker argument or graph submission-schema field is introduced. Under parent CTR-WACI-007 only, a reference may resolve to a local successor fact or the uniquely sealed source fact. This is an explicit exception to their ordinary same-Instance reference meaning, not a claim that the source UUID belongs to the successor. The service derives the true origin from the immutable sealed membership, never from caller-supplied ancestry. Validate both eligibility and actor visibility again inside the command transaction before RETURN, and persist a typed sourceEvidence provenance payload in the new Event separately from ordinary Event foreign keys. Existing Broker JSON submissionPayload pass-through must preserve these UUIDs; its body allowlist needs no additional field. Mixed unrelated facts, forged origin, repeated migration, unmapped node or inaccessible evidence is a zero-write denial.

New successor Events use only successor Instance/Visit/Context/Submission IDs in their ordinary reference columns. Validated source evidence is stored as a separately typed immutable provenance payload; old Events retain their original references. Replay/recovery validates the accepted migration identity, sealed membership/digests, same-Domain and node relations, closure and local Event reference invariants. It does not reassign old facts to the successor or recalculate historical acceptance using a later actor's permissions. Fresh command authorization is still required for new actions; replay is not a way to submit an action again.

RETURN creates a new successor Visit at the corresponding node with the original payload/evidence requirements. Revision appends only successor Context facts and uses the exact creator predicate in CTR-004. The migration Context is revision1 of the successor with an explicit originContextRevisionId in lineage; subsequent revisions use ordinary successor-local revision order and previous Context references. Required source Context history is read-only origin evidence. Preexisting source row membership/value hashes must remain unchanged; explicitly appended migration Events/audit/receipt are checked separately rather than falsely requiring whole-table equality.

Acceptance must include an actual scoped graph capable of RETURN to DRAFT, with original fact UUIDs, preassigned successor/Visit/Context UUIDs, exact original/canonical creator and current actor, and the sequence successor-create -> RETURN -> creator revise -> ADVANCE. The highlighted writer and CTO rows have zero RETURN-to-DRAFT edges and cannot alone supply this proof. The matching same-lineage trace by a historical-participant-only actor must fail before reading the source evidence or writing any fact. The selected real-source mapping and fully preassigned design fixture are in docs/evidence/workflow-canonical-identity-reconciliation-v1/return-revise-design-trace.json: actual source44e3e53e-8d1c-4a23-a36a-820deb9903d6 starts at propose, advances to efficiency_check, returns using its sealed original Visit, revises under its canonical creator and advances again. The artifact is a test definition, not executed/live acceptance. Migration Context created_by is the actual migration actor, while successor Instance creator is the explicitly authorized canonical business creator; normal later Context revisions attribute their actual creator. The historical99 overlap is explicit. Source Context ancestry lives in immutable lineage, not a cross-Instance previous_revision_id foreign key.

### CTR-CIR-006 — Atomic apply and concurrency

Apply verifies exact code/authority/scope/plan hashes, target database identity and reviewed endpoint, current runtime/deployed preimage and actual actor before any mutation. It acquires a bounded goal-specific exclusive repair lock; production mutation concurrency is one. One SERIALIZABLE transaction covers the complete reviewed connected source/instance/Domain group (or one of the exact source-only holds under parent CTR-WACI-010): deterministic locks on identities, Domains, versions, current Instances/Visits, relevant assistance/receipt/lineage rows; all fresh preconditions; target source publication and old deprecation; any explicitly governed Domain delta; source closure and successor creation; source migration Event/version and successor initial Event/version; completed Receipt; audit; postcondition verification; one commit. No successful partial source/instance visibility.

Existing active Assistance is conflict until an accepted exact preservation path is included; no automatic voiding. Unexpected row/count/version, source transition during plan, privilege failure, missing target, incompatible replay, timeout before commit or postcondition failure rolls back all writes. No automatic serialization retry or plan refresh. Each target graph/instance UUID is preassigned. Any schema needed for immutable lineage/closure must be separately included in the audited release and the ordinary migration/release path; never hand-edit production rows.

### CTR-CIR-007 — Receipt, idempotency and unknown outcome

Use one fixed command family WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V1. Request hash covers accepted authority refs, operator SHA, scope digest, plan digest, database identity, actor and exact group identity. Unique source closure and preassigned target identities prohibit two successors. Exact rerun returns the existing completed Receipt only when receipt/event/audit/lineage and complete planned poststate agree; any mismatch is conflict with zero writes. Partial or unknown outcome is not failure permission to replay: inspect the same command/transaction and authoritative durable chain until outcome is known. New plan, new UUID, repeated submit or guessed success is forbidden.

### CTR-CIR-008 — Verification, rollback and release

Verify covers full approved source and instance scope, exact original history digests, canonical target identities, no executable closed source, old-source new-create denial, source graph conservation and all lineage read/RETURN/revise/recovery behavior. Before facts exist rollback may restore the reviewed code/config preimage. After committed successor facts, rollback is containment of affected intake/dispatch while preserving all committed lineage/history; never delete successor facts, reopen a closed source, or restore a fallback that can duplicate dispatch. Controlled repair resumes only from observed known state under accepted authority.

Implementation scope is confined to one offline bounded operator, lineage/closure schema and associated repository/query/command/recovery integration, source lifecycle exception enforcement, focused conformance fixtures and release/readback artifacts. No standalone identity service or general workflow engine redesign. Before implementation, freeze the exact changed-file closure and prove each file is required by these Contracts; expand only through renewed independent scope review, not through convenience refactoring. Existing structure gates still apply.

### CTR-CIR-009 — Actual business acceptance

Fresh full census classifies every currently actionable assigned item, including changes since the frozen initial snapshot. Every valid Agent assignment resolves via actual agent_resolve_principal to one enabled canonical Agent; invalid/stale work remains explicitly classified/no-send. Both corrected source creation with different selected canonical personnel and existing-work successor transitions must pass. Current-source closure is not proven by one version or one instance.

Normal HR Feishu operation selects at least3 low-risk real actionable items across at least2 Domains and2 Agents. HR resolves and dispatches normally; targets self-read and self-transition. Prove exact items/Principals/Agent IDs, receipts, canonical target session, no duplicate send/display fallback/permission broadening/HR proxy transition, and runtime health. Synthetic fixtures, an admin impersonation or a direct SQL observation do not substitute for normal HR proof. Only all requested evidence permits WORKFLOW_ASSIGNEE_CANONICAL_IDENTITY_READY=YES and Goal completion.

### CTR-CIR-010 — Governance and remaining decisions

Author and independent Reviewer are separate. Obtain exact-head semantic review and Owner acceptance before implementation. Then focused tests, fresh independent implementation/data audit, one frozen blocker union, one concentrated repair and one fresh independent re-audit as required by the Goal. Code, schema, operator and plan are reviewed together before controlled apply; native Owner authentication and real-user-only E2E remain Owner gates. Do not ask Owner to dispatch auditors or repeat routine approvals already supplied by the Goal.

The business choice to preserve personnel selection is settled; this draft does not request it again. Exact identity equivalence or business-provenance gaps must be batched with the affected rows and proposed no-write disposition. This child authorizes no guessed mapping. Remaining semantic details uncovered by the required graph/history proof must be resolved in the candidate before it is used as implementation authority; do not call an incomplete preservation model an implementation detail.

## 10. Acceptance mapping

| Acceptance | Contracts | Discriminating evidence |
|---|---|---|
| ACC-CIR-001 |001,002| read-only DB trace; exact scope/hash; wrong UUID/type/status/cardinality/alias/endpoint fail; target/caller separated |
| ACC-CIR-002 |003| exact two-source hold and denied new intake with preserved current work; all source graph/schema deltas; both multi-ADVANCE definitions; all9 parameterized versions; different personnel combinations; old version deny/no fallback |
| ACC-CIR-003 |004,005| original history hashes; exact successor position/context; prior submission visibility; actual RETURN/revise before-after traces; cross-instance and forged-lineage deny |
| ACC-CIR-004 |006| concurrent source transition; every premutation and mutation-stage fault; all-or-nothing source/instance/Domain/receipt visibility |
| ACC-CIR-005 |007| exact replay no writes; changed code/plan/actor/database conflict; unknown commit readback; duplicate successor impossible |
| ACC-CIR-006 |008| recovery/rebuild parity; contained postcommit rollback; structure checks and affected existing suites |
| ACC-CIR-007 |009| fresh whole census/source/new-instance proof; normal HR3 items2Domains2Agents; exact resolver/target self-transition/health |
| ACC-CIR-008 |010| pinned authorities, independent reviews, Owner exact-head acceptance, blocker union/one re-audit, no premature completion |

All10 Contracts have acceptance coverage; coverage definitions are not executed evidence.

## 11. Alternatives and disposition

Rejected: simple same-Visit/Instance rebind, current-Visit-only fix, old fleet replay, automatic model3 graph conversion, fixed-owner specialization after Owner preservation instruction, generic migration framework, Auth identity merging, rewriting historical events, uncontrolled old-source creation, fixture guessing. Existing model3 ONE_TIME_MIGRATE remains valid for a future explicitly semantics-preserving scope; it is not selected for these changed-behavior graphs.

## 12. Migration, compatibility and rollback

CTR-003..008 govern source and current-work migration together. No schema/graph/file/database mutation is authorized by this proposed document. Existing CTO/fleet operators and their historical receipts retain their meanings. Actual release must use the repository's controlled release path with reviewed schema/source changes, preimage, health, independent readback and durable outcome. Native authentication is never interpreted as apply success.

## 13. Open questions and author status

Owner business preference is settled: preserve prior behavior. The graph/history preservation model in CTR-004/005 is carried forward accepted from V1. Actual per-row equivalence and source/instance plan remain proof obligations, not authorization to guess. AUTHORING_READY_FOR_INDEPENDENT_REVIEW=YES; IMPLEMENTATION_ALLOWED_NOW=NO; PRODUCTION_READY=NO. First semantic audit of the successor-delta set returned ACCEPT_WITH_BLOCKERS (1 governance blocker: partial-supersession vehicle unlawful under SPEC_GOVERNANCE_V0; plus mechanical text repairs); the blocker union was frozen once and closed by this whole-authority reissue plus dependency-head repinning (auth successor e8ae194, dsh successor a7dbaaf) in the same bounded repair; ONE fresh independent re-audit follows before the exact-head Owner acceptance gate.

## 14. Successor delta versus V1 (exhaustive)

1. Frontmatter: spec_id V2; status proposed; supersedes [V1]; the two depends_on external_authorities pins replaced by AUTH_SERVICE_INTERNAL_IDENTITY_DIRECTORY_V1 @ e8ae1949ef330bd44b0e0d185053b39e18cab3f3 and AGENT_CORE_INTERNAL_AGENT_DIRECTORY_V1 @ a7dbaafde211dd67381bcb623fe91b6339cb917e (to be re-pinned to their individually accepted exact heads at acceptance); section 1 and 13 updated for successor state; this section added. The V1 frontmatter's pins onto the retired workflow-admission specs are historical record only.
2. CTR-CIR-003: exactly the sentence cluster "The proposed owning dependencies are Auth AUTH_SERVICE_WORKFLOW_CANONICAL_ADMISSION_V1 at2af21f8... through ...Call the pinned Auth exact-Principal route, then the pinned dsh exact-Agent route using that returned ID." is displaced and replaced by the directory sentences now present (dependency pins, generic backend SERVICE identity with exactly the two generic directory grants, retired dedicated identity, directory route paths, principalStatus/exists/enabled acceptance predicates, fresh-token-per-audience-per-command). The neighboring sentences — including "No cached positive result from an earlier command, caller-provided success assertion, or incoming Workflow-audience credential forwarded to another audience is acceptable.", "Service URLs are fixed reviewed backend configuration...", "Responses must bind the requested exact UUID/ID; reject unknown fields inconsistent with the pinned response contract.", "Total admission through commit is bounded to5seconds from the first admission request start using a monotonic clock, with a database statement/transaction deadline no later than that bound.", the external-Spec acceptance sentence, and "No display-name fallback is introduced." — are carried verbatim.
3. Acceptance obligation: the acceptance transaction must atomically flip V1 to superseded_by this Spec's accepted head in the same docs-only change, and re-pin this Spec's external_authorities revisions to the individually accepted heads of both successor dependencies in that same change.

Every other line of V1 is carried forward verbatim.
