# LEGACY_WORKFLOW_SUCCESSOR_CUTOVER_OPERATOR_V1 isolated conformance

No production mutation was performed. The production database was read only
for the fresh plan preimage check and `pg_dump`. All mutation tests ran in a
fresh temporary PostgreSQL cluster on `127.0.0.1:55439`, restored from that
read-only snapshot and migrated through schema 0027.

Plan SHA-256: `dd994cbc1fed7cb8c523eb1cc31e5cd5ca6a27a71a9e7bf3c50337dad7ea6f17`.

| Test | Isolated result |
|---|---|
| T1 exact source preimage | PASS; read-only production `plan` returned `PLAN_OK`, all four isolated applies succeeded |
| T2 stateVersion drift | PASS; `CONFLICT_ZERO_WRITE`, 0 closure and 0 receipt |
| T3 current Visit drift | PASS; `CONFLICT_ZERO_WRITE`, 0 closure and 0 receipt |
| T4 wrong model | PASS; closed-plan validation rejected model 2 |
| T5 non-canonical successor Context | PASS; frozen canonical-principal gate rejected legacy publisher target |
| T6 successor UUID collision | PASS; `CONFLICT_ZERO_WRITE`, 0 closure and 0 receipt |
| T7 closure + successor atomic | PASS; each of four committed rows has exactly one closure and one successor |
| T8 injected failure after create | PASS; successor, closure and command receipt counts all remained 0 |
| T9 historical Visits unchanged | PASS; complete row-value SHA-256 before/after matched |
| T10 historical Events unchanged | PASS; complete row-value SHA-256 before/after matched |
| T11 historical Context revisions unchanged | PASS; complete row-value SHA-256 before/after matched |
| T12 deterministic rerun | PASS; four replay receipts byte-matched and counts remained 4 closures/4 successors |
| T13 exact verify | PASS; four `VERIFY_OK`; all successors at publishing with canonical assignee and Context |

Additional closure/read proof: an UPDATE of a closed source was rejected by
the database trigger, and the assigned-work predicate returned 0 old sources
and all 4 canonical successors.
