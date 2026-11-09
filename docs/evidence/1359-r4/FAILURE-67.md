# R4 unexpected binary67 failure — observation only

Required full-suite gate FAIL. These are real uncaught case failures, not
the expected caught-panic negative tests in binary42. No repair, retry,
base-control execution or product change follows this failure.

| Case | Actual failure coordinate | Observation |
|---|---|---|
| `coverage_cleanup::retired_managed_cleanup_actor_is_projected_only_with_existing_full_visibility` | `tests/authorized_actions_r4/managed_fixture.rs:194:57` | AdmissionFailed(Unavailable) |
| `guards::disabled_target_and_disabled_caller_fail_closed` | `tests/authorized_actions_r4/fixture.rs:39:170` | AdmissionFailed(Unavailable) |
| `guards::replaced_owner_loses_cancel_while_new_owner_gets_current_authority` | `tests/authorized_actions_r4/fixture.rs:39:170` | AdmissionFailed(Unavailable) |
| `linked::linked_child_resolve_requires_actual_accepted_terminal_proof` | `tests/authorized_actions_r4/linked.rs:11:140` | AdmissionFailed(Unavailable) |
| `managed::managed_required_effect_and_real_waiting_producer_preserve_non_gate_return` | `tests/authorized_actions_r4/managed_context.rs:43:57` | AdmissionFailed(Unavailable) |
| `managed::opted_in_source_ordinary_owner_actions_are_excluded_and_refused` | `tests/authorized_actions_r4/managed_fixture.rs:194:57` | AdmissionFailed(Unavailable) |
| `matrix::scope_gate_priority_full_restricted_hidden_and_minimal2` | `tests/authorized_actions_r4/fixture.rs:39:170` | AdmissionFailed(Unavailable) |
| `preservation::successor_transition_permission_does_not_grant_raw_assignee_assistance` | `tests/authorized_actions_r4/support.rs:186:5` | HTTP 422 admission_legacy_input, expected 200 |
| `review::managed_review_nonterminal_and_terminal_preserve_effect_and_payload_boundaries` | `tests/authorized_actions_r4/managed_fixture.rs:194:57` | AdmissionFailed(Unavailable) |
| `snapshot::repeatable_read_authority_snapshot_is_pre_owner_replacement_then_rechecks` | `tests/authorized_actions_r4/fixture.rs:39:170` | AdmissionFailed(Unavailable) |

All ten cases remain UNKNOWN for base/candidate causality. Nine fail during
an admission-related legitimate fixture operation with Unavailable. One
fails a real successor transition with HTTP422 admission_legacy_input rather
than expected200. The completed R3 attribution does not classify this newly
reached group: R3 stopped at45 and never executed67. Static source proximity
or unchanged admission files is not a same-byte base comparison.

See full.log:2486, full.json, full-binary-results.jsonl:65 and
failure67-observations.json. Any later investigation or repair requires an
explicit continuation under the same redlines; no continuation is implied
by this receipt.
