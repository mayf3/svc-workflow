# PREFLIGHT mode

## Goal

Determine whether implementation may start and which authority action is required.

## Procedure

### 1. Fix coordinates

```text
TARGET_REPOSITORY
CURRENT_HEAD
TARGET_BASE_BRANCH
BASE_COMMIT
REQUESTED_CHANGE
```

### 2. Read authority in precedence order

Read the local adoption lock and local authority map, then Product Direction, Architecture/invariants, overlapping accepted Specs, and exact external authority revisions. Record whether the shared governance adoption is `accepted`, still `proposed`, absent, or not applicable because this is the source repository.

### 3. Classify mechanicality

```text
UNCERTAIN = NON_MECHANICAL
```

Potentially mechanical: spelling, formatting with no interpretation change, deterministic generated refresh already required by an accepted Contract, or a pure move with machine proof of semantic identity.

Default non-mechanical: dependency, schema, API, permission, scope, timeout, retry, lifecycle, test-expectation, deletion, module-boundary, trust-boundary, and unproven “refactor-only” changes.

A mechanical exemption requires independent review and a persistent record.

### 4. Classify authority handling

Choose exactly one:

```text
REUSE       accepted authority already covers the work
AMEND       proposed authority changes, or accepted editorial / strictly additive change
SUPERSEDE   accepted normative meaning changes
NEW         no existing authority owns the bounded decision
```

An accepted-Spec addition is `AMEND` only when it adds new stable IDs within unchanged Goal, scope, authority, and accepted Decisions; otherwise it is `NEW` or `SUPERSEDE`.

### 5. Select governing authority

Record one primary Spec and related authorities. Confirm:

- lower authority does not override parent authority;
- external authorities are reference-only;
- no partial supersession is implied;
- Program acceptance is not mistaken for child implementation permission;
- implementation-authorizing Contracts cover the request.

### 6. Check base rule

For non-mechanical implementation, all must hold:

```text
SPEC_PRESENT_IN_BASE = YES
SPEC_STATUS_IN_BASE = accepted
IMPLEMENTATION_AUTHORITY = contracts
REQUEST_WITHIN_CONTRACT_SCOPE = YES
```

## Output

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
PREFLIGHT_MODE = REUSE | AMEND | SUPERSEDE | NEW
CHANGE_CLASS = MECHANICAL | NON_MECHANICAL
MECHANICAL_EXEMPTION_REVIEW = ACCEPT | REJECT | NOT_APPLICABLE
GOVERNANCE_ADOPTION_STATUS = accepted | proposed | absent | source_repository
PRIMARY_GOVERNING_SPEC = <ID | NONE>
RELATED_ACCEPTED_AUTHORITIES = <IDs | NONE>
GOVERNING_SPEC_REVISION = <commit/blob | NONE>
BASE_COMMIT = <sha>
SPEC_PRESENT_IN_BASE = YES | NO | NOT_APPLICABLE
SPEC_STATUS_IN_BASE = accepted | proposed | superseded | NONE
IMPLEMENTATION_AUTHORITY = contracts | none | UNKNOWN
AUTHORITY_CONFLICT = NONE | <description>
IMPLEMENTATION_ALLOWED = YES | NO
NEXT_ACTION = <bounded action>
```

When `IMPLEMENTATION_ALLOWED = NO`, do not begin semantic implementation.
