# AUTHOR mode

## Goal

Produce a docs-only governing Spec that can be implemented without hidden chat context.

## Procedure

### 1. Freeze authority and scope

Identify Goal, owning repository, parent authorities, exact external references, in-scope/out-of-scope behavior, Spec kind, and explicit implementation authority.

### 2. Build State from Observations and Evidence

Create stable `OBS-*` records, identify `CLM-*` propositions, then record explicit `EVD-*` source-to-target relations and build the State projection. Every load-bearing State statement cites Observation, Claim, Evidence, or direct provenance.

Distinguish source branch, deployed revision, configuration, data, environment, and user-visible behavior.

### 3. Preserve primitive types

```text
OBS-*    direct recorded result
CLM-*    interpretation with SUPPORTED / INFERRED / OPEN_ASSUMPTION
EVD-*    qualified Observation-to-Claim/State relation
STATE-*  time-indexed projection
DEC-*    selected normative direction
CTR-*    accepted obligation
ACC-*    verification mapping
```

### 4. Resolve decisions before implementation

For each Decision, record owner, selected direction, alternatives, rejection reasons, and remaining owner input. Identity, permission, lifecycle, failure, migration, compatibility, security, and public behavior are not hidden “implementation details.”

### 5. Write stable Contracts

Cover relevant positive, negative, identity, authorization, trust, transaction, lifecycle, deletion, timeout, retry, idempotency, unknown-outcome, migration, compatibility, rollback, audit, and security paths.

Contract IDs are never renumbered or reused after acceptance.

### 6. Map Acceptance both directions

Every active Contract has at least one Acceptance item or a reasoned runtime/manual evidence requirement. Every Acceptance reference resolves to a real Contract. A wrong implementation must fail at least one Acceptance item.

### 7. Run authority and lifecycle pass

Confirm parent precedence, external ownership, no partial supersession, valid backlinks, no accepted-ID repurposing, and no Program-to-child authorization leap.

### 8. Run syntax pass

Use `.agents/protocol/SPEC_FORMAT_V0.md` and the schema. Syntax pass is not semantic acceptance.

## Output

```text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = <ID>
SPEC_KIND = invariant | program | implementation
STATUS = proposed
AUTHORITY_LEVEL = governing_spec
IMPLEMENTATION_AUTHORITY = none | contracts
PRIMARY_PARENT_AUTHORITY = <ID | NONE>
EXTERNAL_AUTHORITIES = <qualified refs | NONE>
OPEN_OWNER_DECISIONS = NONE | <items>
NORMATIVE_TBD = NONE | <items>
PARTIAL_SUPERSESSION = NONE | DETECTED
CONTRACT_COUNT = <n>
CONTRACTS_WITH_ACCEPTANCE = <n>
AUTHORING_READY_FOR_REVIEW = YES | NO
```
