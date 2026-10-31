# AC #367 distribution loop: implementation preflight and existing-path evidence

This is an investigation, not Product Authority, an accepted implementation
Spec, or a completed bug fix. The requested three-direction implementation is
**not delivered** by this change. No production Definition, Context, Instance,
role, runtime, or dispatch state was read or changed in this investigation.

## Coordinates and mandate

- Target: `mayf3/svc-workflow`.
- Source and authority base: `3b81d8f99a286caeb04ea2c349966b0f6662275d`.
- Local `main`, `origin/main`, and GitHub main API agreed at that revision on
  2026-10-31. It includes disabled-admission amendment PR #108.
- Isolated write surface: task-specific clone `/tmp/sw-367`, branch
  `ac-task/367-dispatch-loop-fix`, created from that exact base. No other
  checkout was changed.
- Mandate: Owner task for private `mayf3/agent-control#367`: three-direction
  source repair and regressions, independent review, PR and issue delivery;
  zero production writes, no merge, no FSM/scheduler rewrite, no overlap with
  #724/#725/#726 without declaration. Investigation and testing support that
  mandate; the task does not replace accepted Product Authority.
- Evidence input: issue body and all nine pre-existing comments. Later
  corrections qualify earlier claims. Full business identifiers remain in the
  private issue; this public repository uses case A/B.

## Result and route

Existing source already supplies Definition-declared input validation and
Assistance-based due-feed exclusion. The local tests below pass on the
unchanged base. They do not prove the reported production Definition was
corrected, the runtime loaded those implementations, or the execution runtime
reported exhaustion. A new Svc implementation defect has not been established.

The remaining requests cannot be implemented by silently converting business
inputs into engine rules, adding a second attempt ledger, broadening repair or
global-read permissions, or treating a time projection as full admission.

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
GOAL_OR_TARGET = reject invalid distribution inputs and stop unchanged-blocker dispatch
CURRENT_GAP = existing source paths verified; live Definition/runtime binding unknown;
              new universal admission/repair/detection Contracts absent
REVIEW_TARGET_HEAD = recorded in the investigation PR review
BASE_HEAD = 3b81d8f99a286caeb04ea2c349966b0f6662275d
CURRENT_BASE_HEAD = 3b81d8f99a286caeb04ea2c349966b0f6662275d
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO for the remaining new semantics
AUTHORITY_ACTION = NEW for independent missing decisions
PRIMARY_AUTHORITY = NONE for the remaining new semantics
RELATED_AUTHORITIES = Product V13; Architecture V0.4.7; Execution Control V2;
                      Work Eligibility Projection V2; existing create Contract
IMPLEMENTATION_AUTHORITY = none for the missing Contracts
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = NO
PLAN_LEVEL = EXEC_PLAN for the requested cross-component delivery
ASSURANCE_LEVEL = DURABLE; any permission change is separately CONTROLLED
EXECUTION_MANDATE = VALID within the Owner task's source/test/PR boundary
MUTATION_AUTHORIZATION = VALID for this investigation and isolated local tests
ISOLATED_WRITE_SURFACE = YES
CONTROLLED_RUNBOOK_REQUIRED = NO for this investigation
SPEC_GAP_DEPENDENCY = LOAD_BEARING for new semantics
EVIDENCE_REVIEWABILITY = PASS for checked-in source/test observations;
                         production binding unavailable
LIVE_AUTHORITY_GAP = NONE (none established; production state not sampled)
OWNER_DECISION_REQUIRED = YES for missing Product decisions, not routine source work
EMERGENCY_STATE = NONE
EMERGENCY_ACTION = NONE
INCIDENT_REFERENCE = NOT_APPLICABLE
BASE_IMPACT = BOUNDED
IMPLEMENTATION_ALLOWED = NO for the new semantics
MERGE_READY = NO for the requested implementation deliverable
OPERATION_ALLOWED = NO
EVIDENCE_NEEDED = accepted-in-base affected Contracts or a reproducible failure
                  of an existing accepted Contract; exact Definition/runtime binding
DONE_WHEN = three authorized directions implemented or proven already covered,
            regressions and independent review, implementation PR and issue receipt
EXPANSION_TRIGGER = new permission, general Context editing, new admission state,
                    Core attempt-policy change, production mutation, shared-file conflict
NEXT_REAL_ACTION = bind the actual Definition/runtime failure to existing Contracts;
                   author and accept only genuinely missing decisions before implementation
NEXT_ACTION = RE_PREFLIGHT
```

Governance adoption is accepted on this base. The lock records Owner
acceptance, and `python3 .agents/tools/verify_governance.py --target .
--require-accepted` exits 0. The blocker is coverage of the requested new
semantics, rather than an unaccepted bootstrap.

## Three requested directions

All code locations below refer to the exact source base above.

| Direction | Existing implementation and authority | Proven limit / missing evidence |
| --- | --- | --- |
| Creation admission | `src/application/definition/lifecycle/validation.rs:173` checks node metadata `requiredContextInputs`: schema-required key, explicit type, array `minItems >= 1`, nonempty closed `items.enum`. `create_transaction/facts.rs:159` calls the ordinary Context schema validator before runtime facts commit. The existing create Contract §7 requires validation against the pinned schema. PR #72 merged this declared-input implementation as `7c533cfc2676ea970d502969662dff3bbdd3e66c`. | The declaration is optional and old Published versions remain immutable. An incomplete old schema is not retroactively strengthened. The test registry is fixture/Definition data, not a live authoritative platform service. No accepted Contract defines a universal `targetPlatforms` rule, registry owner, freshness, or lookup-failure behavior. |
| Existing missing input | `src/http/handlers/assistance.rs` and the existing Assistance application/store paths let the current assignee request Owner help; the current Domain Owner explicitly escalates to HUMAN_REQUIRED. Execution Control V2 CTR-SWEC-006 excludes both OWNER_PENDING and HUMAN_REQUIRED from due dispatch. | Assistance preserves Context. Creator+DRAFT legacy revision is not non-DRAFT correction. `repair_transaction.rs` only adds missing required assignee keys and rejects changes to existing values or extra business keys. It cannot add arbitrary `targetPlatforms`, replace `[]`, or correct a frozen evidence hash. Product V13 and Architecture CTR-ARCH-008 permit only the exact managed-INTAKE `workflowPrerequisite` exception for model3 Context. |
| Repeated no progress | `src/http/handlers/escalations.rs` and `execution_escalation.rs` implement exact `ATTEMPTS_EXHAUSTED` / `STALE_LOOP_EXHAUSTED` ingress; current-Visit escalation opens OWNER_PENDING idempotently. `query_dispatch_intents.rs:129` and `:181` exclude open cases in both keyset branches. Execution Control V2 CTR-SWEC-004/006 authorizes this path. | Core owns attempt detection and the existing three-attempt cap; Svc stores no second attempt ledger. No new identical-result fingerprint, threshold, reason, or cross-Visit counter is authorized here. The source test exercises a genuine Svc escalation request, not Core detecting exhaustion unattended. |

`src/application/workflow_instance/eligibility.rs` deliberately retains
`ACTIONABLE_NOW` as the time dimension. WEP V2's selected exception explicitly
says time eligibility is not full admission and forbids a new WEP `BLOCKED`
state. Open Assistance suppresses due-feed dispatch even if a read projection
still says `ACTIONABLE_NOW`. A consumer selecting all such summaries without
the other admission predicates is not repaired by inventing a new classifier
value in Svc.

The existing paths are retained because they have distinct accepted
responsibilities. No mapping, fallback, second repair path, parallel counter,
new state, schema migration, endpoint, or compatibility bridge is introduced.

## Independent 403 disposition

The reported `global_read_role_required` belongs to global-list admission
(`http/handlers/instances.rs` -> query service -> global-read role predicate).
Ordinary instance creation uses `workflow.execute` and target-domain membership;
it does not require the global-reader role. A denied global discovery call is
not evidence that the create transaction itself denied an authorized member.

The accepted member-readable Published input-contract endpoint already uses
the create-authorized set, while the other four Definition-management reads
remain Owner-only. Its 17-test matrix passes below. That endpoint does not
grant arbitrary global instance discovery or canonical-principal resolution.

Therefore the 403 is independent of the schema validator and Svc's existing
Assistance exclusion. No permission patch is implemented. Any follow-up must
identify the precise missing scoped discovery/handoff predicate and its owning
Svc/Core/Auth authority; preserve global-read and scheduler-read gates. The
issue's later `scheduler_read_role_required` wake refusal is a distinct gate.
Wake's existing request uses instance, Visit, and expected version; a required
intent ID was an earlier evidence error, corrected in the issue.

Issue expected behavior also mentions general parent/child deduplication.
Ordinary create's receipt idempotency and specialized governance-child binding
must not be relabeled as a general distribution-child audit implementation.
The Owner's bounded three-direction task does not authorize building that
additional subsystem.

## Executed evidence

Environment: macOS arm64, repository base above, task-owned PostgreSQL cluster
at `/private/tmp/sw367-pgdata`, loopback port `55467`, database
`sw367_regression`, local test role `sw367_test`. The pre-existing port-5432
cluster rejected the fixture credentials; it was not changed. All test writes
went to the separate task cluster. Test outcomes and initial failures are
preserved in `../reports/ac-367-evidence/`.

| Evidence | Command / observation | Result and scope |
| --- | --- | --- |
| EVD-367-001 | `cargo test --test 38_declared_context_inputs_v5_distribution --test 36_execution_control_v1 --test 22_repair_context --test 40_definition_input_contract_member_read_tests` with task `TEST_DATABASE_URL` | Initial parallel baseline exited 101: repair 10/10; execution control 7/8. `acc06_outbox_event_key_is_unique` failed at instance creation with PostgreSQL serialization failure. Cargo stopped before the two later binaries. This is an unchanged-base failure, not a product RED assertion. |
| EVD-367-002 | Same command plus `--test 26_workflow_assistance_http -- --test-threads=1`, same source and database | Exit 0: repair 10, Assistance HTTP 1, execution control 8, declared-input distribution 7, member input contract 17: **43 passed, 0 failed**. The initial serialization failure did not recur serially. |
| EVD-367-003 | `cargo test --lib` | Exit 0: **252 passed, 0 failed, 1 ignored**. The ignored test requires the explicit task-owned Owner-wake database variable; separate execution is recorded below. |
| EVD-367-004 | Governance verifier with `--require-accepted` | Exit 0; accepted adoption and vendored digests match. |
| EVD-367-005 | `cargo test --lib owner_wake_outbox_stays_pending_then_recovers_on_later_pass -- --ignored --test-threads=1` | First invocation with only `TEST_DATABASE_URL` exited 101 because this test actually requires `OWNER_WAKE_TEST_DATABASE_URL`. Corrected invocation with the latter bound to the same task-owned loopback database exited 0: **1 passed**. The setup error is retained and is not product RED evidence. |

The distribution test proves missing, empty, and unknown platform inputs are
refused with the corrected fixture Definition; it does not prove admission
against the reported old production version. Execution-control `acc05` proves
due before escalation -> persisted OWNER_PENDING -> absent from due feed ->
same-case replay, including authorization denial. Assistance HTTP proves
assignee request / Owner handling / Human-query and resume semantics. Repair
tests pin refusal of altered existing values and non-assignee business keys.

No new regression was made RED and no product source changed; these results
must not be presented as a new RED-to-GREEN fix. No full integration-suite or
production/runtime acceptance claim is made.

## Case A/B disposition for the authorized recovery actor

The original two Instance coordinates are recorded in private #367. Their
current Visit, version, assignee, semantic model, open Assistance, runtime
binding, and effect outcomes were not freshly sampled here and remain UNKNOWN.
The following is a handoff, not an executed recovery or permission grant:

1. Through existing authorized scoped reads, bind the actual Instance/current
   Visit/CAS, model version, Published Definition digest/schema, assignee, open
   case, and current runtime capability surface. Do not infer model3 from a
   Definition's business name containing “v5”. Reconcile successful/unknown
   child effects before any new attempt.
2. If a case is already open, use that case; do not invent a case ID. Otherwise
   the actual current assignee may use the existing Assistance request route
   with real Visit/CAS/idempotency coordinates to describe the blocker. A
   Domain Owner cannot impersonate the assignee. A legitimate exhausted
   execution runtime may use the existing system ingress only with its actual
   exhaustion evidence and existing role.
3. The current Domain Owner may explicitly escalate the existing case to
   HUMAN_REQUIRED. For model3 dispatch, verify the case is durable and the
   Visit absent from the canonical due feed, including pagination. This
   contains repeated dispatch; it does not repair Context or finish business.
   For legacy work, do not invent an activation; verify the actual runtime
   consumer respects the existing Assistance stop instead of the time label
   alone. No such runtime verification was performed here.
4. Do not use the operator repair CLI, which is limited to missing required
   assignee keys, to add/replace `targetPlatforms`. That CLI is a trusted-host
   maintenance surface; its operator ID is audit attribution, not authenticated
   current-assignee authority.
   A valid existing Legacy Creator+DRAFT route applies only if its actual
   actor/node/model preconditions hold. General non-DRAFT/model3 correction
   needs accepted authority; preserving the old immutable evidence is required.
   Do not resolve the case merely to re-enable the unchanged failing work.
5. For future traffic, an authorized Definition Owner must separately author,
   validate and publish a corrected version with real registry-backed schema
   values through the existing channel, then bind future creates to it. This
   task performed no production publication, successor-instance creation,
   replay, wake, transition, permission change, or data patch.

The issue comments explicitly leave current loaded build/profile UNKNOWN;
historical stamp or tool-name matching was invalidated as provenance. Preserve
that qualification until an authorized same-time runtime binding is available.

## Boundaries and next acceptance

#724 owns Core submission-gap diagnostics and their consumers/tests; #725 owns
fixed-operation async prototype work; #726 owns the existing closed-loop
fixture/pilot and Forum projection. No files in those surfaces are changed,
and their completed source work is not reopened.

Before a source implementation PR resumes, either establish a failure of an
already accepted schema/Assistance/cap Contract and fix only that drift, or
accept and merge the exact missing authority first. The minimum new decisions,
if still requested, are registry ownership/admission semantics, precise scoped
non-DRAFT correction semantics with immutable provenance, and the ownership and
evidence rule for any new identical-no-progress detector. Any change to existing
Context immutability or WEP meaning requires a whole-authority successor, not a
test or an implementation PR silently overriding it. The independently
reviewed preflight is evidence for that next decision; it cannot accept itself.
