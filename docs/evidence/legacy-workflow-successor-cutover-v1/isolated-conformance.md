# WIKI_LEGACY_PRINCIPAL_BATCH_UNBLOCK_V0 conformance

Production mutation: **none**. The production database was accessed with
`default_transaction_read_only=on` for the fresh preimage and plan checks.
Mutation tests ran in a temporary PostgreSQL 16 container restored from that
read-only snapshot; the container was destroyed after the run.

Plan SHA-256:
`a036f16a102b353eaeaa12a45f5c93ecdcafa1c2c939f55394b2369889d85238`.

Fresh shared facts:

- all seven source Instances are active at `publish`, state version 4, pinned
  to DefinitionVersion v2 `1bf0d466-91aa-4cd0-8b51-b550523f4408`;
- v2 `publish-done -> publish_review` resolves to legacy Principal
  `6ccdae57-0a2c-4a2e-b3e4-e3f4ec1edf2a`;
- published corrected v3 `df59e327-43d3-462c-9cf3-f75271b8ca75`
  preserves the graph and resolves the same edge to canonical Principal
  `40daa67b-af5a-462d-b4ca-d40213147afb`;
- canonical successor is an enabled Workflow AGENT, has enabled
  `knowledge-curation` membership, and the legacy binding is disabled;
- the seven source Context objects contain only `slug`, `source_file`, and
  `title`; no identity-valued Context rewrite is needed.

| Test | Result |
|---|---|
| closed seven-row plan and constants | PASS (4 unit tests) |
| fresh production plan, read-only | PASS (`rows=7 unlockable=7`) |
| injected failure after successor creation | PASS; successor, closure, and receipt all rolled back |
| exact apply + verify | PASS, 7/7 |
| deterministic rerun | PASS, 7/7 receipts byte-identical |
| source history preservation | PASS; aggregate source counts remained `7:28:28:7` |
| successor current node | PASS; 7/7 at v3 `publish`, canonical knowledge-curator assignee |
| successor next assignee | PASS; 7/7 `publish-done` targets canonical learning-expert Principal |

The operator never accepts arbitrary IDs and never updates the old
DefinitionVersion, original Instance, Visit, Event, Submission, or Context
facts. Each apply is a SERIALIZABLE, plan-hash-bound transaction with a
preassigned successor identity, immutable closure row, completed receipt, and
exact replay behavior.
