---
spec_id: SVC_WORKFLOW_DEFINITION_INPUT_CONTRACT_MEMBER_READ_V1
title: Published Definition Version Input-Contract Member Read V1
status: accepted
repo: mayf3/svc-workflow
base_head: 88ff8145eb0842b4ce759117caf4b05f9012ebdc
acceptance_basis: >-
  Repo owner approved the narrow direction in-session on 2026-10-08
  ("没有问题啊。你觉得这个设计是合理的就行" / "没问题，你先改吧"); independent
  review remains the PR-time gate per central governance.
implementation_authorized_now: true
merge_performed: false
---

# SVC_WORKFLOW_DEFINITION_INPUT_CONTRACT_MEMBER_READ_V1

## 1. Problem (evidence-backed)

A domain-member agent principal holds an enabled `domain_role_bindings` row and
can successfully create instances against a domain's PUBLISHED definition
version via the formal create path (`POST /internal/v1/workflow-instances`,
admission predicate in `create_transaction.rs` + `validation_helpers.rs`).

The same principal cannot read that version's input contract: all four
definition read operations (`GetDefinition`, `GetDefinitionVersion`,
`ListDefinitionVersions`, `GetCompleteVersionGraph`) are gated by H-5
(`DEFINITION_SERVICE_CONTRACT_V0_1.md` §6) to `DOMAIN_OWNER` only, and every
denial maps to opaque `404 definition_not_found`
(`src/http/handlers/definitions.rs` `map_definition_error`).

Live evidence: principal `fd58881a-fdba-4ef2-9a80-b733671f24f1` (agt_blog-agent,
domain member) created instance `89346bcd…` via the formal create path, while
`get_definition` for version `fc9b0966-18f6-4821-8aad-057f37e257cd` returned
404. The read/write contract is asymmetric and forces a domain-owner human
relay for every schema delivery.

## 2. Decision summary

Add exactly ONE new read surface that reuses the EXISTING create-instance
admission predicate, so any principal already authorized to initiate instances
of a definition version in its domain can read that PUBLISHED version's input
contract. No roles are added, removed, or reinterpreted.

```text
NEW_READ_SURFACE              = GetPublishedVersionInputContract
ADMISSION_PREDICATE           = IDENTICAL to create-instance admission, applied in the same order:
                                1. principal exists and enabled
                                2. version exists (else 404)
                                3. version's domain exists; caller has an ACTIVE
                                   domain_role_bindings row in that domain
                                   (role_key unrestricted, enabled = TRUE — the
                                   exact `validate_domain_membership` SQL)
                                4. domain enabled
                                5. version_status == PUBLISHED (else 409
                                   version_not_published — mirrors create)
RETURNED_FIELDS               = { definitionVersionId, definitionId, versionNumber,
                                  versionStatus, contextSchema }
RETURNED_FIELDS_EXCLUDE       = nodes, transitions, instructions, assignee configs,
                                submission schemas, lifecycle operator fields,
                                digest, metadata, timestamps
HTTP                          = GET /internal/v1/definition-versions/{definitionVersionId}/input-contract
SCOPE                         = workflow.read (same as instance detail reads)
ERROR_MAPPING                 = reuses map_definition_error unchanged:
                                PermissionDenied | PrincipalDisabled | DomainNotFound |
                                DefinitionNotFound | DefinitionVersionNotFound -> opaque 404
                                DomainDisabled -> 403 domain_disabled
                                NEW DefinitionError::VersionNotPublished -> 409 version_not_published
```

`contextSchema` is the create-side input contract: instance creation validates
`context_payload` against exactly this schema
(`validate_context_schema(version.context_schema, …)`). Returning it — and
nothing else — is the minimum that removes the human relay.

## 3. Frozen boundaries (non-goals, unchanged by this Spec)

```text
H5_FOUR_READS_DOMAIN_OWNER_ONLY = UNCHANGED (GetDefinition / GetDefinitionVersion /
                                  ListDefinitionVersions / GetCompleteVersionGraph)
ROLE_MODEL                      = UNCHANGED (DOMAIN_OWNER / DOMAIN_MEMBER concepts stay;
                                  role simplification is explicitly deferred)
OWNER_PROMOTION                 = NO
GLOBAL_READ                     = NO
DRAFT_OR_MANAGEMENT_DATA        = NOT EXPOSED (PUBLISHED only; 409 otherwise)
NODE_ASSIGNEE_SUBMISSION_RIGHTS = UNCHANGED
CROSS_DOMAIN_VISIBILITY         = NO (domain derived server-side from the version row)
ANTI_ENUMERATION                = PRESERVED (all denials for non-members stay
                                  indistinguishable opaque 404s)
SPECIFIC_ID_HARDcoding          = NO (no blog/BIP/version IDs anywhere in source)
BROKER_SURFACE                  = NO CHANGE (first-batch broker capabilities do not
                                  carry definition tools; agents reach svc-workflow
                                  through the TypeScript SDK, which gains the
                                  matching read method beside client.create)
```

## 4. Precedent inside the contract

`DEFINITION_SERVICE_CONTRACT_V0_1.md` §6 closes with: "未来可通过扩展
`DomainRoleBinding` 角色实现更细粒度的 Definition 管理权限，当前不在 PR 2 实现。"
This Spec exercises exactly that reserved extension point, in the narrowest
possible form. §6 itself is NOT edited by this change (the four reads keep
their frozen semantics); this new Spec governs the added surface.

## 5. Test matrix (RED first, then GREEN)

```text
P1  member + active binding + PUBLISHED version -> 200, only contract fields
P2  characterization: member get_definition/get_definition_version still denied
    (PermissionDenied) — asymmetry documented, H-5 intact after the fix
P3  owner: new surface 200 AND all four H-5 reads unchanged (200)
P4  non-member (no binding): existing version -> PermissionDenied; unknown
    version -> DefinitionVersionNotFound; both in the same opaque-404 mapper
    bucket (handler-level unit assertion: identical ApiError)
P5  revoked membership (binding enabled = FALSE) -> PermissionDenied
P6  cross-domain member (binding in another domain only) -> PermissionDenied
P7  member + DRAFT version -> VersionNotPublished (409), schema bytes never returned
P8  member + DEPRECATED version -> VersionNotPublished
P9  disabled principal -> PrincipalDisabled (same bucket as existing reads)
P10 disabled domain -> DomainDisabled (403, mirrors create)
P11 wire shape: serialized result has no nodes/transitions/instructions/
    assignee/submissionSchema keys; contextSchema round-trips the seeded schema
```

## 6. Delivery

Normal path only: feature branch off `main@88ff814`, PR, independent review,
merge, then the existing deployment pipeline. Production rollout serializes
with any concurrent svc writer (admission-guard candidate authoring has not
started in this repo; this branch touches definition read surface only —
no file overlap with the create/admission guard scope beyond read-only reuse
of the same predicate semantics).

Acceptance in production (post-deploy, outside this PR): the blog agent
retries the schema read with its own token and succeeds without domain-owner
relay; its daily creation entry (`sdk.typescript` client `create`) is preceded
by the new read method against the same deployed svc.
