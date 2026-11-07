# R5 group67 controlled attribution

**PRE_EXISTING_AT_CLEAN_8AA864A / not a regression of the restored R3/R4
uncommitted delta.** R5 user explicitly permits these attributed failures to
be recorded outside this delivery gate. No admission repair, test removal,
skip, ignore, semantic assertion change or full-green claim follows.

## Executed control and restoration

1. Saved the complete candidate using `git stash push --include-untracked`.
   Exact stash `e152fd6acbd0279c2f0ee2132cd67848e0469853`; other stashes untouched.
2. Verified empty tracked/untracked status at exact
   `8aa864ab9ff70107faf8e016a04d32f4e78ad141`.
3. Created new PG16 clusters, exact loopback identities:
   `/private/tmp/h1359-r5-pg-55483/data|55483|127.0.0.1` and
   `/private/tmp/h1359-r5-pg-56591/data|56591|127.0.0.1`.
   Group67 received role `swaar4` and its existing required port55483;
   each test created its own UUID database. No reused fixture flags or old runner.
4. Executed the whole existing binary (23 cases), preserving R4 order/context:

   ```text
   cargo test --locked --jobs 4 --test 67_authorized_actions_r4 -- --test-threads=1 --nocapture
   test result: FAILED. 13 passed; 10 failed; 0 ignored; 0 measured; 0 filtered out; finished in 62.88s
   ```

   Actual cargo rc101; 72.848s including build. This is the expected diagnostic
   RED authorized by R5, not a new unexpected failure.
5. Parsed both original R4 and fresh clean-control panic records: all ten
   names, source coordinates and errors match individually, not merely totals.
6. Popped that exact task stash with rc0. All814 pre-stash files were
   rehashed and restored with zero mismatch, including original evidence.
   Both newly owned clusters were later identity-checked and stopped rc0;
   PID files absent, ports no longer accepting; data/logs retained.

## Exact failure matches

| Case | Coordinate | R4 candidate and R5 clean control |
|---|---|---|
| coverage_cleanup::retired_managed_cleanup_actor_is_projected_only_with_existing_full_visibility | managed_fixture.rs:194:57 | AdmissionFailed(Unavailable) |
| guards::disabled_target_and_disabled_caller_fail_closed | fixture.rs:39:170 | AdmissionFailed(Unavailable) |
| guards::replaced_owner_loses_cancel_while_new_owner_gets_current_authority | fixture.rs:39:170 | AdmissionFailed(Unavailable) |
| linked::linked_child_resolve_requires_actual_accepted_terminal_proof | linked.rs:11:140 | AdmissionFailed(Unavailable) |
| managed::managed_required_effect_and_real_waiting_producer_preserve_non_gate_return | managed_context.rs:43:57 | AdmissionFailed(Unavailable) |
| managed::opted_in_source_ordinary_owner_actions_are_excluded_and_refused | managed_fixture.rs:194:57 | AdmissionFailed(Unavailable) |
| matrix::scope_gate_priority_full_restricted_hidden_and_minimal2 | fixture.rs:39:170 | AdmissionFailed(Unavailable) |
| preservation::successor_transition_permission_does_not_grant_raw_assignee_assistance | support.rs:186:5 | HTTP422 admission_legacy_input vs200 |
| review::managed_review_nonterminal_and_terminal_preserve_effect_and_payload_boundaries | managed_fixture.rs:194:57 | AdmissionFailed(Unavailable) |
| snapshot::repeatable_read_authority_snapshot_is_pre_owner_replacement_then_rechecks | fixture.rs:39:170 | AdmissionFailed(Unavailable) |

Coordinates above are under `tests/authorized_actions_r4/`. Full panic/output
bytes and exact URLs/environment are in [clean67.log](clean67.log),
[clean67.json](clean67.json), and [comparison](attribution-comparison.json).
Raw synthetic router/database probes are committed in [aa-results](aa-results).
Clean log SHA256: `08a977bb68ac2f965721f513e14e31b03344b53053a74e9972fb8ba6ef804886`.

## Supported explanation and limits

The nine Unavailable paths use fixtures that publish AGENT identity literals
with `AdmissionGate::disabled()`: fixture.rs:39, managed_fixture.rs:143/194,
managed_context.rs:43, linked.rs:11. The existing
`src/store/postgres/admission_gate.rs:170` documents that this has no directory
client; `observe` at335-344 returns Unavailable for a nonempty AGENT set with
no client, and `definition_repository/lifecycle_transactions.rs:354` invokes
admission during real publication. This supports a fixture/configuration
explanation without an actual remote-directory request. It does not establish
a port conflict or an unavailable production service.

The tenth case inserts a synthetic successor relation then transitions an
existing legacy graph. The actual router response is HTTP422
`admission_legacy_input`; existing `resolve_local_target_on_tx` at
admission_gate.rs:88-109 can reject a nonhistorical legacy literal, and
`src/auth/admission.rs:224` maps it to that label. The exact original input
lineage causing the rejection is a source-based inference, not instrumented
runtime proof; no admission or successor behavior is repaired here.

**Coordinate limit:** clean8aa already contains product implementation062fe82.
This actual control proves pre-existence relative to the uncommitted R3/R4
candidate increment, as the R5 mandate specifies. Original integration-main
product behavior was not runtime-compared in R5. Supplementary static evidence:
all group67/common fixture bytes match original integration main8da8dbc;
the binary does not invoke `forum_sync`, `ForumClient`, or `run_loop`, while
the product patch changes only the background `run_loop` and new worker helper.
This supports separation from outbox polling but is not called an original-main
runtime comparison or a full-suite green.

All468 R4 build-input hashes still match. Reuse R4 binary45=28/28rc0 and
the other passed binaries; retain its original full1324/10/1rc101 visibly.
New R5 delivery gates are candidate48=22/22rc0 and lib=269pass/1existing
ignored/rc0, with independent affected/final-head review and publication.

## Evidence relations

- EVD-1359-R5-001: clean-state + clean67 raw/metadata -> same exact ten
  failures pre-exist at clean8aa; sufficient for the R5 mandate's gate
  disposition, limited by the clean8aa coordinate above.
- EVD-1359-R5-002: pre-stash814 manifest + restoration zero mismatch ->
  preserved candidate restored; sufficient for byte-level restoration.
- EVD-1359-R5-003: candidate48/lib raw +468 matching inputs -> affected
  validation and reuse of R4 passed results; full all-green is not established.
- EVD-1359-R5-004: identity/readback + actual stop commands -> owned fixture
  closure; does not imply production, deployment or live business acceptance.

Provenance: root executed the scripts; `/root/r5_review` independently read
and recomputed attribution, restoration, input reuse and gate results.
