# Issue #633: published input-contract regression coverage

## Change brief and preflight

```text
TASK_ID = mayf3/agent-control#633
TARGET_REPOSITORY = mayf3/svc-workflow
BASE_HEAD = 88c73bcbba624da8c2557a94cbd2f3802239cdff
ROUTE_STAGE = IMPLEMENTATION
AUTHORITY_ACCEPTED_IN_BASE = YES
GOAL_OR_TARGET = verify ordinary same-domain instance initiators can read the exact published input contract without gaining definition-management authority
CURRENT_GAP = implementation already merged; HTTP input-contract regression coverage is missing
AUTHORITY_ACTION = REUSE
PRIMARY_AUTHORITY = SVC_WORKFLOW_DEFINITION_INPUT_CONTRACT_MEMBER_READ_V1 at BASE_HEAD
RELATED_AUTHORITIES = Product V13; Architecture V0.4.7 CTR-ARCH-004/005; DEFINITION_SERVICE_CONTRACT_V0_1 section 6
IMPLEMENTATION_AUTHORITY = accepted Spec implementation_authorized_now: true
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = CONTROLLED
MUTATION_AUTHORIZATION = VALID (user task: isolated branch, regression coverage, independent review, PR and issue delivery; no merge)
ISOLATED_WRITE_SURFACE = user-provided temporary clone / branch ac-task/633-schema-member-read / exact BASE_HEAD
SPEC_GAP_DEPENDENCY = NONE for the bounded ordinary-member read
LIVE_AUTHORITY_GAP = NONE (no live state inspected)
OWNER_DECISION_REQUIRED = NO
MODEL_CONVERGENCE_APPLICABLE = NO
```

Governance adoption is accepted on `main`; the locked distribution verifier
passes. The exact adoption commit and both input-contract merge commits are
ancestors of the base. PR [#75](https://github.com/mayf3/svc-workflow/pull/75)
already delivered the minimal endpoint and SDK at merge commit
`bcfdee5c9d39d9c1a1ca88d7dfbea30bdadce6da`; PR
[#79](https://github.com/mayf3/svc-workflow/pull/79) added snapshot consistency at
`bfc28c15320acab8cf7d7a36128cd5c9db9a1b1a`. This change preserves that production
implementation rather than duplicating it or opening the H-5 reads.

The read applies the ordinary membership admission conditions: enabled
principal, exact stored version/domain, enabled binding (any role key), enabled
domain, and PUBLISHED version. Later managed Owner-obligation policy creation
has a separately authorized internal actor path; this task neither broadens
the member read to that path nor claims equality with every specialized create
operation or error order.

## Plan and controlled test runbook

1. Bind the branch/base; verify governance and accepted Spec ancestry.
2. Run suites 16, 19, 39 and 40 unchanged as the baseline.
3. Add HTTP coverage for minimal exact-version schema read followed by real
   ordinary instance creation, opaque denials, non-PUBLISHED states and scope;
   complete the existing all-four H-5 read assertions.
4. Run focused and related regressions; record actual results and limitations.
5. Obtain independent exact-head review, create the PR, comment on #633 and stop.

All database writes are limited to a newly initialized, disposable PostgreSQL
cluster with an explicit loopback test DSN. Authentication uses newly generated
test keys and a loopback mock JWKS server. No production configuration,
credentials, services, instances or identities are read or changed. Abort if
the explicit isolated DSN is absent, any production dependency is needed, a
production behavior change is required, or the affected authority/base changes.
Logs and exit codes qualify test evidence; runtime deployment and original
Agent self-read acceptance remain outside this task. Stop the disposable
cluster after verification. No merge, install, restart, grant, role change or
new role system is authorized.

## Verification and review

Executed on 2026-10-31 with PostgreSQL 16.15, Rust/Cargo 1.98.1 and an explicit
disposable loopback database. Both integration commands used:

```sh
TEST_DATABASE_URL=<isolated-test-dsn> cargo test --locked \
  --test 40_definition_input_contract_member_read_tests \
  --test 39_definition_version_detail_read \
  --test 16_definition_service_audit_fix \
  --test 19_domain_owner_definition_governance -- --test-threads=1
cargo test --locked --lib input_contract_error_mapping_tests -- --test-threads=1
```

| Suite | Unchanged base | Candidate |
| --- | ---: | ---: |
| 16: Definition audit/H-5/lifecycle | 36 passed | 36 passed |
| 19: Domain Owner definition governance | 25 passed | 25 passed |
| 39: owner version HTTP reads | 6 passed | 6 passed |
| 40: member input contract and snapshots | 17 passed | 20 passed |
| HTTP error mapper unit tests | not rerun for baseline | 3 passed |

Baseline and candidate integration exit codes are both 0; mapper exit code is
0. All existing tests were retained, with P2/P3 moved into the H-5 test module
and expanded to exercise all four reads. The three added HTTP cases verify:

- ordinary same-domain AGENT reads exactly the five public fields for an older
  exact PUBLISHED version despite a different newer PUBLISHED schema, then
  creates a real isolated Instance from that exact version with its own token;
- non-member, disabled-binding and cross-domain Agents are rejected by create
  and get identical opaque input-contract 404 errors for existing/missing IDs;
- DRAFT/DEPRECATED/REVOKED versions return 409 without schema, and a token
  without `workflow.read` returns 403 even for a PUBLISHED version.

`rustfmt --edition 2021 --check` on the three affected Rust files and
`git diff --check` passed. `git diff --quiet -- src sdk contracts docs/specs
.agents` passed: production code, contracts and authority are unchanged.
Existing unused-import warnings in shared test helpers and suites were not
changed. The full unrelated repository suite was not run. These are regression
observations against already implemented behavior, not a new fix/RED claim.

Execution-log SHA-256 receipts (local logs; semantic independent review is
recorded separately against the exact committed candidate in the PR):

```text
baseline  79f17cb81674f3633cf2aaf72bcab1b9372c38196c65658e0bb28585bf72bb27
candidate 1b24b34bec3c4fb58b8ddc5946178e8edd0921633e086eb01fa2f76b8972c411
mapper    cf59f8bdc039a5bc2f7583582ccab848f92e07f595c2be1da9f783a7d2126ee8
```

Deployment, loaded-service behavior and original Agent self-read/natural daily
entry acceptance were not tested. Nothing here claims those runtime outcomes.

`DONE_WHEN`: regression PR opened and #633 delivery comment posted, with exact
implementation/test coordinates, review and read-only scope boundaries.
`EXPANSION_TRIGGER`: a concrete behavior/authority conflict needed to satisfy
this bounded read contract. `NEXT_ACTION`: continue within this brief.
