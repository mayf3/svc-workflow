# Spec Governance Protocol V0 — historical compatibility record

```text
PROTOCOL_VERSION = 1.0.0
STATUS = superseded
SUPERSEDED_BY = .agents/protocol/SPEC_GOVERNANCE_V1.md
```

This path is retained for exact-pinned history and link compatibility. It is not the active workflow in a source revision that contains Governance V1.

The active protocol is `.agents/protocol/SPEC_GOVERNANCE_V1.md`.

Historical V0 used one mechanical/non-mechanical switch. Governance V1 preserves authority, exact-revision review, immutable accepted meaning, qualified Evidence, local adoption, whole-authority supersession, and emergency containment while separating:

```text
long-lived obligation -> Authority
execution complexity  -> Plan
failure consequence   -> Assurance
```

Consumers pinned to an older commit keep the old V0 bytes. A consumer adopting this revision follows V1. Do not combine V0 and V1 routes to invent another workflow.
