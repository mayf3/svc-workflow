# STOPPED H06 Tier-2 R2

2026-11-06, one explicitly authorized R2 attempt; no retry or post-failure repair.

- Source parent: `bcd8950466091f4cb4e0133c431186d9267ac2c2`.
- Clean environment explicitly set `LC_ALL=C LANG=C` for all PG/script calls.
- PG start rc0, isolated identity rc0: existing PG16 directory, port 55432,
  restricted Unix socket, no TCP, actual postgres superuser.
- Conformance script ran once, **rc3** at
  `migrations/0030_composite_command_event_integrity.sql:5`:
  `ERROR: LOCK TABLE can only be used in transaction blocks`.
- Existing migration loop uses psql -f without a transaction. No repair applied.
- Cargo/tests/assertions: **NOT_RUN**, no PASS claimed.
- Explicit stop rc0; status rc3; no target PID/socket/PostgreSQL process.
- Data directory mode 0700 and partial test schema retained; R1 evidence and
  regulator start.log preserved. No additional DB query or teardown deletion.
- Source/test/migration/Cargo/fixture hashes unchanged. Runner only reuses R1
  opt-in patch with its two local port values changed to the mandated 55432.
- Production DB connections/writes and service writes: zero. No service registration.
- **MERGE_READY=NO**; PR stays draft and #1360 stays open.

[Complete receipt](RECEIPT-H06-TIER2-R2.md) and
[raw archive](docs/reports/h06-tier2-r2-20261106/).
Only evidence delivery and requested PR/issue status updates continue.
