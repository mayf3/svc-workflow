# Existing graph diagnostics Spec: exact compile-input recovery

## Scope and authorization

During the owner's 2026-11-27 continuation of the Wake modeling/repair task,
`cargo test --lib wake_transaction::tests` could not compile the existing test
module in `src/application/definition_governance/diagnostics.rs`: its two
`include_str!` calls reference the absent
`docs/specs/SVC_WORKFLOW_DEFINITION_GRAPH_DIAGNOSTICS_V1.md`.

This recovery restores that **existing accepted Spec verbatim** from its trusted
original merged Git coordinate. It changes no Spec semantics, test expectation,
diagnostic catalog, Rust implementation, or production state. It is a bounded
mechanical recovery of a required compile input, not a new acceptance or authority.

## Exact provenance

| Coordinate | Value |
|---|---|
| Historical reviewed semantic head | `78323394c6c6d82a14657bdfd6589419fdbb6dff` |
| Historical accepted lifecycle head | `424022741a7ec863fbb91f5afdee836f7c7ade6c` |
| Original accepted merge / restored source | `39279e34ff3124f8ca6f4269f9dab737a28aed75` |
| Original implementation head with identical accepted bytes | `6f8b743bab130e1599c8a4b6b3358814f1ad7c91` |
| Restored Git blob | `f49dba0344bac793f1e84faec0bb3259a5e3ef94` |
| Restored SHA-256 | `81a76db7948b9388b8a489cea07aabde04c9853981d033eeacd5f8a41cb02c7e` |
| Exact size | 17,559 bytes; 345 LF-terminated lines |
| Audited main baseline where input is absent | `c50ab2ac98decceaa26138a282c34dd92ede0ddc` |

[PR #27](https://github.com/mayf3/svc-workflow/pull/27) is recorded as merged on
2026-09-06. Its lifecycle record identifies Owner acceptance of the reviewed
semantic head, an independent final-head recheck with no blockers, lifecycle
head `4240227…`, and merge `39279e3…`. The local immutable objects match these
coordinates. Comparing the proposed semantic file to the accepted file changes
only frontmatter: proposed → accepted, implementation authority none → contracts,
and acceptance/review provenance. The semantic body is byte-identical.

[PR #28](https://github.com/mayf3/svc-workflow/pull/28), merged later that day,
identifies `39279e3…` as the accepted implementation base. The current
[`WORKFLOW_AUTHORING_DIAGNOSTICS_IMPLEMENTATION_V1.md`](WORKFLOW_AUTHORING_DIAGNOSTICS_IMPLEMENTATION_V1.md)
independently retains the same base and reviewed semantic-head references.
The accepted file at lifecycle head, original merge, and original implementation
head has exactly the blob and SHA-256 above. Current accepted
`SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3` and
`SVC_WORKFLOW_OWNER_OBLIGATION_WORKFLOWS_V3` still cite this Spec.

The original merge SHA is **not an ancestor of the rewritten current main**.
No present ancestry claim is made. The current main tree lacks the file while
retaining its implementation, include paths, and accepted-authority references.
The earlier SHA and the current tree are therefore explicitly distinguished.
PR #78 concerns later version-detail logging; it did not introduce this authoring
diagnostics Spec and is not the source for recovery.

## Recovery and verification

The exact bytes were obtained with:

```bash
git show 39279e34ff3124f8ca6f4269f9dab737a28aed75:docs/specs/SVC_WORKFLOW_DEFINITION_GRAPH_DIAGNOSTICS_V1.md
```

Before and after writing the original path, SHA-256, byte count, line count and
byte-for-byte equality were checked. The restored Appendix A contains 70 rule
codes; all 70 still occur in the current diagnostic catalog. Later additional
catalog rules are preserved unchanged; the old accepted text is not rewritten
to match today's implementation.

This record proves provenance and byte identity of the recovery. It does not
claim a Rust test passed; actual build/test results are recorded in the bounded
Wake regression evidence. Other absent product/architecture documents are
outside this recovery and remain separate traceability gaps.
