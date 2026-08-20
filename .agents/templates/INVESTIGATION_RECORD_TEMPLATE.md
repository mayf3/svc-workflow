# Investigation Record

Investigation Records preserve durable knowledge when no new governing Spec becomes accepted. They are not governing authority and do not grant implementation permission.

## Identity

```text
INVESTIGATION_ID = INV-YYYY-NNN
REPOSITORY = owner/repository
SUBJECT = ...
OPENED_AT = <timestamp>
CLOSED_AT = <timestamp | OPEN>
OWNER = <identity or role>
DISPOSITION = open | adopted | rejected | no_change | reuse | deferred
```

## Goal

What question was investigated and why did it matter?

## Authority context

```text
RELATED_AUTHORITIES = <IDs and revisions>
AUTHORITY_CHANGE_PROPOSED = YES | NO
```

## Observations

### OBS-INV-001 — <direct result>

- Coordinates: ...
- Method: ...
- Result: ...
- Provenance: ...

## Claims

### CLM-INV-001 — <interpretation>

- Support state: SUPPORTED | INFERRED | OPEN_ASSUMPTION
- Supported by: ...
- Uncertainty: ...

## Alternatives considered

### ALT-INV-001 — <alternative>

- Benefits: ...
- Costs/risks: ...
- Evidence: ...

## Disposition

```text
DISPOSITION = rejected | no_change | reuse | deferred | adopted
REASON = ...
IMPLEMENTATION_ALLOWED = NO | governed by accepted Spec <ID>
```

## What would reopen the question

- new evidence: ...
- changed parent authority: ...
- changed operational constraint: ...

## Stable links

- issue / investigation PR: ...
- related Spec: ...
- evidence/report: ...
