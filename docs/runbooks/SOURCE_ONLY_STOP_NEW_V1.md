# Runbook: Source-only stop-new deprecation (SOURCE_IDENTITY_UNRESOLVED) — V1

Authority: `docs/specs/SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2.md`
(accepted), CTR-CIR-003 final paragraph — for exactly the two versions of
parent CTR-WACI-010, "the reviewed plan may deprecate without publishing a
replacement, atomically recording SOURCE_IDENTITY_UNRESOLVED and preserving
every task/history fact. No cancel/archive/assignee rewrite or implicit
redirect follows." Storage support: migration
`migrations/0024_deprecation_reason.sql` (nullable
`workflow_definition_versions.deprecation_reason`).

## 1. The two eligible versions (frozen; closed set)

| workflow_definition_version_id         | version | definition_key       |
|----------------------------------------|---------|----------------------|
| `9b07afc4-d3a2-456d-8b96-13fdffbaf995` | 1       | `agent_self_task_v1` |
| `e01d1f3a-661b-468f-9eda-0506abaa5c0b` | 2       | `idea_pool_v1`       |

No other version may be deprecated under this runbook. Other unresolved
versions fail planning and cannot be silently added to this exception
(CTR-CIR-003).

## 2. Preconditions (all must hold, verified in the same command)

1. The operator acts as an authenticated, **enabled** Principal that holds an
   **enabled `DOMAIN_OWNER`** binding on the definition's owning domain. The
   deprecation command re-verifies principal enabled status, domain enabled
   status and the owner binding inside its transaction — a missing or
   disabled role fails the command with zero writes.
2. The version is currently `PUBLISHED`. Deprecating a DRAFT or already
   DEPRECATED/REVOKED version is refused (`InvalidLifecycleTransition`).
3. The reviewed reconciliation plan names this exact version UUID. Never
   operate from a display name, definition key or memory alone.

## 3. Procedure

Invoke the definition lifecycle write
(`DefinitionService::deprecate_version`, the same surface used by the
lifecycle test suites) with the reason token exported by the lifecycle
module:

```rust
use svc_workflow::application::definition::{SOURCE_IDENTITY_UNRESOLVED, DefinitionService, commands::DeprecateVersion};

let deprecated = service
    .deprecate_version(DeprecateVersion {
        actor_principal_id: operator_principal_uuid,   // the actual actor
        definition_version_id: /* exact UUID from section 1 */,
        deprecation_reason: Some(SOURCE_IDENTITY_UNRESOLVED.to_string()),
    })
    .await?;
```

- The reason token is exactly `SOURCE_IDENTITY_UNRESOLVED` (constant
  `svc_workflow::application::definition::SOURCE_IDENTITY_UNRESOLVED`);
  arbitrary prose reasons are not part of this runbook.
- `deprecation_reason: None` remains valid for ORDINARY deprecations (it
  writes NULL) but MUST NOT be used for a stop-new deprecation under this
  runbook: recording the reason is the whole point of the procedure.
- The reason (1–256 chars, checked by the service and by a DB CHECK
  constraint) is written in the **same transaction** as the status flip: the
  version row is locked `FOR UPDATE`, re-verified `PUBLISHED`, then one
  `UPDATE` sets `version_status='DEPRECATED'`, `deprecated_at`,
  `deprecated_by_principal_id` and `deprecation_reason` atomically. There is
  no window where the version is deprecated without its provenance or vice
  versa.

## 4. Effect (expected, no further action)

- **Stop-new:** creating a new instance on a DEPRECATED version already fails
  with `409 version_not_published`
  (`create_transaction::lock_and_validate_version`: status must be
  `PUBLISHED`). This is the "deprecated-version rule" CTR-CIR-003 relies on;
  a deprecated old UUID never redirects or falls back to another source.
- **Existing work continues:** instances already created on the version are
  unaffected — transition validation permits `DEPRECATED` versions, so real
  existing work runs to completion and remains unresolved business, not
  invalid business (identity proof is missing, not the work).

## 5. Verification after execution

```sql
SELECT version_status, deprecated_at, deprecated_by_principal_id,
       deprecation_reason
FROM workflow_definition_versions
WHERE definition_version_id IN (
    '9b07afc4-d3a2-456d-8b96-13fdffbaf995',
    'e01d1f3a-661b-468f-9eda-0506abaa5c0b'
);
```

Expect `version_status='DEPRECATED'` and
`deprecation_reason='SOURCE_IDENTITY_UNRESOLVED'` for each. (The reason is
deliberately not projected through the definition read API; it is operator
provenance read at the database.)

## 6. Strictly forbidden

- No delete, cancel, archive, revoke or assignee rewrite of the held version
  or of any instance/visit/context/event fact created from it.
- No history rewrite and no relabeling of existing work as invalid.
- No implicit redirect: never point callers of a held version at another
  version as part of this procedure.
- No deprecation of any version outside section 1 under this runbook.
- No revision of the version's graph/schema after deprecation (immutable by
  constraint).

## 7. Reference test

`tests/33_source_only_stop_new_v1.rs` proves the full contract: deprecation
with the reason persisted atomically, new-instance creation refused with the
`VersionNotPublished` (409) rule, and an existing instance still transitioning.
