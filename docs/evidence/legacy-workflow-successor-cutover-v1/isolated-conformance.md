# LEGACY_WORKFLOW_SUCCESSOR_CUTOVER_OPERATOR_V1 isolated conformance

No production mutation was performed. The blocker-union revalidation runs
through the checked-in disposable PostgreSQL harness
`scripts/run_legacy_successor_cutover_v1_tests.sh`; it creates a fresh database
named exactly like the reviewed target and migrates it through schema 0027.

Plan SHA-256: `dd994cbc1fed7cb8c523eb1cc31e5cd5ca6a27a71a9e7bf3c50337dad7ea6f17`.

| Executable case | Isolated result |
|---|---|
| Exact source preimage | PASS; group plan succeeds against four exact source fixtures |
| stateVersion / current Visit drift | PASS; group rejects with zero closure/successor |
| wrong model / DefinitionVersion | PASS; group rejects with zero closure/successor |
| non-canonical successor Context | PASS; closed-plan gate rejects |
| successor UUID collision | PASS; group rejects with zero closure |
| connected-group atomicity | PASS; one SERIALIZABLE transaction and one commit creates all four closures/successors |
| injected failure rollback | PASS; all four successors, closures and receipts remain absent |
| historical Visits / Events / Context | PASS; every preexisting row value remains identical; only the required source closure Event is appended |
| exact replay | PASS; fresh durable group verification returns the existing Receipt without duplicates |
| incompatible replay | PASS; successor poststate drift rejects instead of returning the stored Receipt |
| exact verify | PASS; one group verify proves all four closures, successors, canonical Contexts, Visits, Events, Receipts and audits |

The four rows are one connected group, not four independently committable
operations. The CLI has no `--row` apply or verify surface. Apply also requires
fresh directory admission for every persisted canonical Agent identity.
