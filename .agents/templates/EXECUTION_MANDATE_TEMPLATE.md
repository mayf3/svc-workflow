# Execution Mandate

This authorizes and constrains one task or operation. It does not create Product Authority.

```text
MANDATE_ID =
ROUTE_STAGE = AUTHORITY_AUTHORING | IMPLEMENTATION | OPERATION
AUTHORITY_ACCEPTED_IN_BASE = YES | NO | NOT_APPLICABLE
ISSUER =
ISSUER_AUTHORITY =
ISSUED_AT =
VALID_FROM =
EXPIRES_AT =
MAX_ATTEMPTS =
ACTOR_OR_ALLOWED_ROLE =
TARGET_REPOSITORY_OR_SYSTEM =
ISOLATED_WRITE_SURFACE = <worktree or exact parent/ref/tree | NOT_APPLICABLE>
ENVIRONMENT =
PURPOSE =
ALLOWED_EFFECTS =
FORBIDDEN_EFFECTS =
EXACT_OPERATION_OR_CLASS =
DONE_WHEN =
ABORT_CONDITIONS =
SECRET_HANDLING =
LOGGING_AND_REDACTION =
RECEIPT_REQUIRED = YES | NO
INDEPENDENT_POST_STATE_VERIFICATION = YES | NO
UNKNOWN_OUTCOME_HANDLING =
EMERGENCY_STATE = NONE | ACTIVE
EMERGENCY_ACTION = NONE | ROLLBACK | DISABLEMENT | SHUTDOWN |
                   REVOCATION | ISOLATION | CONTAINMENT
INCIDENT_REFERENCE = <reference | NOT_APPLICABLE>
DURABLE_NEW_BEHAVIOR = NO
POST_INCIDENT_AUTHORITY_RECONCILIATION_REQUIRED = YES | NO | NOT_APPLICABLE
PRODUCT_AUTHORITY_REFERENCES =
MAY_CHANGE_PRODUCT_CONTRACTS = NO
MAY_EXPAND_SCOPE = NO
SELF_ISSUED_BY_ACTING_AGENT = NO
```

An unattributed “Owner approved” statement is invalid. Post-hoc text cannot fabricate prior authorization.

For every mutation, issuer, target, scope, allowed/forbidden effects, Done When, and isolated write surface are mandatory. Controlled mutation additionally requires actor/role, environment, exact operation or class, abort, Secret, receipt, and validity/attempt bounds. Emergency use also requires an incident reference, containment-only action, no durable new behavior, and later normal authority reconciliation.
