---
authority_id: POSTGRES_STORAGE_CONTRACT_V0_3
status: accepted
authority_kind: storage_contract
owning_repository: mayf3/svc-workflow
implementation_authority: none
production_apply_authority: none
supersedes:
  - POSTGRES_STORAGE_CONTRACT_V0_2
superseded_by: null
owners:
  - mayf3
date: 2026-10-29
revision: draft-2
candidate_base: f5e7f328ccba5f6e0d752abf74e68558ef38ca0f
acceptance_authority_basis: >-
  Root acceptance under user-delegated authority; ROOT_OWNER_ACCEPTANCE.json SHA-256 43eef302fb86cf131106fd6d39c0e338abc0acf9f14fb0210a5e51918d91448e; reviewed semantic Svc b115c0641e4c2b6b965a6a3cd24252734754c14f and Core 0629858c08029ebcaed39b012b4c4a2a2a8078fb. Lifecycle transaction only; no production apply authority.
---

# POSTGRES_STORAGE_CONTRACT_V0_3 — accepted whole-authority successor

Accepted whole-authority storage successor. Root acceptance under the user-delegated authority, recorded in `ROOT_OWNER_ACCEPTANCE.json` (SHA-256 `43eef302fb86cf131106fd6d39c0e338abc0acf9f14fb0210a5e51918d91448e`), binds the independently reviewed semantic heads Svc `b115c0641e4c2b6b965a6a3cd24252734754c14f` and Core `0629858c08029ebcaed39b012b4c4a2a2a8078fb`. This lifecycle-only transaction records acceptance and reciprocal predecessor links. Accepted governing Specs with `implementation_authority: contracts` authorize later source implementation within their Contracts once present in its base; Product, Architecture, and Storage authority retain `implementation_authority: none`. Production apply, migration execution, deployment, and A75 mutation remain unauthorized. The storage contract specifies source-level schema and transaction requirements only; it does not authorize migration apply or production changes.

## Bounded contract delta

V0.3 retains all V0.2 rules and adds exactly the five relations below, one forward integrity migration, and the named command-type additions below. It does not rewrite/delete historical facts, broaden ordinary Event uniqueness, or add a ledger for handoff, plan, recovery, or admission counts. FKs prove actual DB identity; transactional semantic guards must also prove receipt type/status/actor, digest, business facts, and cross-row lineage.

### Five new relations and immutable identities

1. `workflow_owner_policy_approvals`: `source_definition_version_id` PK/FK to `workflow_definition_versions`; `policy_digest` SHA-256; `publish_command_id` UNIQUE FK to `workflow_command_receipts`; `approved_by_principal_id` and `configured_policy_actor_principal_id` FKs to principals; `domain_id` FK domains; `source_definition_digest`; intake/review target DefinitionVersion FKs; acceptance/completion transition FKs; intake/review target Version, graph and schema digests; nullable `owner_replacement_command_id` FK receipts only for an actual replacement; `published_at`. Guard `DEFINITION_PUBLISH`, COMPLETED 2xx, receipt Principal equals approver, actual current-Owner authorization, and exact published Version/Domain/digests/configured-actor snapshot. Freeze approval and referenced semantic identity after publish. No caller-supplied approval UUID.
2. `workflow_owner_obligation_intents`: `intent_id` PK; `kind` INTAKE|REVIEW|STOP; `case_id` FK cases; `source_instance_id` FK instances; `source_node_visit_id` FK visits; `request_event_id` FK Events; `request_context_revision_id` FK Context revisions; `source_definition_version_id` FK versions; `request_digest`; `policy_digest`; `policy_source_version_id` FK approvals; `selected_definition_version_id` FK versions; `selected_transition_id` FK transitions where applicable; nullable `binding_id` FK GCR bindings until INTAKE materializes; nullable `child_terminal_event_id` FK Events for REVIEW; nullable `managed_instance_id` FK instances until materialized; `state`, `outcome`, nullable `last_command_id` FK receipts; and actual STOP origin fields `owner_replacement_command_id`, `node_visit_id`, `last_attempt_id`, `origin_event_id`, `stop_code`. UNIQUE lifetime INTAKE `(case_id)` with matching immutable request/policy refs; UNIQUE REVIEW `(binding_id,child_terminal_event_id)`. A pre-materialization Owner-replacement stop updates that same lifetime-unique INTAKE row even when `managed_instance_id` and `binding_id` are absent: keep `kind=INTAKE`, record actual `owner_replacement_command_id`, `last_command_id`, `stop_code=OWNER_CHANGED_UNSUPPORTED`, and `state=STOPPED`, without fabricating absent managed/binding coordinates or creating a second STOP row. Preserve the existing managed-instance STOP uniqueness origins `(managed_instance_id,owner_replacement_command_id,stop_code)`, `(managed_instance_id,node_visit_id,last_attempt_id,stop_code)`, or `(managed_instance_id,origin_event_id,stop_code)` with FKs for real facts. The same Domain/Owner fence serializes replacement with materialization; STOPPED is absorbing across Owner ABA. Existing trusted WAE attempt IDs are not receipt FKs and may not be fabricated.
3. `workflow_owner_managed_instances`: `workflow_instance_id` PK/FK instances; `kind` INTAKE|REVIEW; `case_id` FK cases; `policy_source_version_id` FK approvals; `policy_digest`; `binding_id` FK GCR bindings; nullable `child_terminal_event_id` FK Events (required for REVIEW, absent for INTAKE); `creation_command_id` UNIQUE FK receipts; `created_at`. Partial UNIQUE INTAKE `(case_id)` and REVIEW `(binding_id,child_terminal_event_id)`. Validate creation receipt and actual DefinitionVersion/Domain/role. Review is not a GCR child. Recorded instance identity survives retries and Owner changes.
4. `workflow_operation_effects`: `effect_id` PK; `case_id` FK cases; `managed_instance_id` FK managed instances; `definition_version_id` FK versions; `node_definition_id` FK node definitions; `slot_key`; `logical_occurrence=1`; `descriptor_key`, `descriptor_version`, `descriptor_hash`; `definition_graph_digest`; `first_authorizing_visit_id` FK visits; `actor_principal_id` FK principals; `business_intent_digest`; `concrete_request_digest`; `state`; nullable actual `command_id` FK only until truly known; actual `result`/`result_digest` or actual failure; `last_outcome_code`. UNIQUE `(case_id,managed_instance_id,definition_version_id,node_definition_id,slot_key,logical_occurrence)` and UNIQUE non-null command ID. Guard actual command type/Principal/result/facts. Do not include Owner, visit, attempt, CAS, or input digest in identity. SUCCEEDED is immutable. Do not invent PREPARED/STARTED rows; V1 does not retry UNKNOWN or deterministic rejected actions.
5. `workflow_owner_dependency_origins`: `created_instance_id` PK/FK instances; UNIQUE `parent_effect_id` FK effects; `parent_managed_instance_id` FK managed instances; `case_id` FK cases; UNIQUE `create_command_id` FK receipts; `target_definition_version_id` FK versions; `created_at`. Insert in the same successful ordinary effect-create transaction and validate against exact INSTANCE_CREATED Event, result, Definition/Domain and completed receipt. One effect creates at most one dependency leaf; the leaf cannot have INTAKE/REVIEW role or auto-intake policy.

### Exact receipt/Event command matrix

- Existing `DEFINITION_PUBLISH`: opted-in publication success creates **zero** runtime `workflow_events`, plus its existing security-audit behavior and one actual publish receipt. Publication, approval relation, and receipt completion must be one transaction; implementation must explicitly compose the actual owned receipt/transaction, not assume the current outer wrapper and inner pool transaction already do so. Preserve existing opaque authorization/existence behavior (`definition_not_found` 404 for the mapped permission/disabled/not-found class; direct `NotDomainOwner` 403). Rejection creates no approval/published-policy relation and zero runtime Events.
- Existing `REQUEST_WORKFLOW_ASSISTANCE`: retain its exact existing request Event/cardinality. The opted-in request writes INTAKE responsibility in that same transaction and adds zero Events.
- New `AUTO_CREATE_AND_BIND_OWNER_INTAKE`: exactly two Events on success: child `INSTANCE_CREATED` and source `GOVERNANCE_CHILD_BOUND`, same real command ID/actor and existing exact GCR pair lineage; one receipt. Its deterministic completed non-2xx outcome has zero business Events/facts, while retaining the valid failure receipt/diagnostic behavior. Extend only the named composite integrity guard for this command using the GCR pair check. Never include the request Event.
- New `AUTO_CREATE_OWNER_REVIEW`: exactly one `INSTANCE_CREATED` on successful review birth under one receipt; deterministic completed non-2xx has zero business Events/facts and may retain failure receipt/diagnostic. No source bind Event or GCR child binding. Replay returns original birth result.
- Governed ordinary create reuses existing `CREATE_WORKFLOW_INSTANCE`: exactly one `INSTANCE_CREATED` Event and its original actual receipt/result/`response_body`. No effect-wrapper receipt or Event. Existing assistance resolution reuses `RESOLVE_WORKFLOW_ASSISTANCE` and its one `ASSISTANCE_RESOLVED` Event; escalation reuses `ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN` and its existing corresponding Event. Preserve actual receipt body and existing semantics.
- Existing transition/revise/RETURN command types retain their current Event cardinality; required-effect and node-ready checks add zero Events. A RETURN-limit STOP is a sidecar fact in the actual RETURN transaction. Existing `SYSTEM_EXECUTION_ESCALATION` retains current 0-or-1 Event behavior; its STOP sidecar adds zero Events. Owner replacement adds its typed unsupported STOP with zero Events. Cancellation uses one existing `CANCEL_WORKFLOW_INSTANCE` receipt per managed Instance and the existing state-change/no-op cardinality, never a batch Event.
- Intent/effect lookup, responsibility reconciliation/projection, causal origin, and STOP sidecars add no phantom business Event or independent receipt. Completed deterministic non-2xx receipts remain valid; no successful observation may survive rollback. UNKNOWN commit is not a known-zero result. The only new business command type literals are the two `AUTO_*` commands above. Existing `ONE_TIME_MIGRATE` behavior is untouched.

A successful request transaction persists the ordinary request facts plus INTAKE responsibility; a separate actual create-bind command creates the GCR pair. A child terminal transaction stores its actual proof and REVIEW responsibility; a separate create command births one review Event. The two commands are not combined. Ordinary effect response envelopes are not written over original command `response_body`. Preserve all old receipt/Event bytes and direct-reader behavior, explicitly returning the two named GCR Events where appropriate.

The forward migration validates old data before installing guards, preserves all old rows/IDs, fails closed on invalid history, and supplies transactional/concurrency proof for new uniqueness and semantic guards. Rollback restores the old global Event-command index only if it installs without conflict; valid multi-Event GCR/intake pair history must never be deleted or rewritten to make rollback pass. No production DDL is authorized by this proposal.

Paired authority: `SVC_WORKFLOW_OWNER_OBLIGATION_WORKFLOWS_V1 §§3–4, 8, 11–12`; exact field/guard/cardinality source is the immutable author storage/stage correction artifact SHA-256 `8eab0f233793e1704c3612e1659f0c69c0e4323b45b0f1d395c8f9fec8a8ab06`, paired with design `db3c63991d816e11e31d94d24e312a60c9204d0cec0e160cd6dbbef0db4dd717`, Broker mapping `34cf1f937d50c6f5bb45b4fba3d008e320b283507778552194617663104f955f`, producer/discovery `874e36341a4ef0e390aafebae090f482a986def3faea665ca0d99cf108c55823`, and review-gap closure `79e47fa08a3be5068c21f0b08f7a0b090097d804fc8894f0f92d25174105a6ce`. All are design inputs, not implementation evidence.

The predecessor contracts are carried forward below with only the bounded successor changes stated above; the predecessor source remains unchanged. This successor is accepted in the same paired lifecycle transaction that records the reciprocal predecessor backlink. Acceptance is not implementation or conformance evidence.

Predecessor source: `docs/contracts/POSTGRES_STORAGE_CONTRACT_V0_2.md` at `f5e7f328ccba5f6e0d752abf74e68558ef38ca0f`; original file SHA-256 `6642a5addd9dfc0f9da025b76923c2f577035b6706c0407fd119b1eec6647eb3`.

## Complete predecessor body carried forward verbatim

# PostgreSQL Storage Contract v0.2 — accepted exact GCR command/Event pair

```text
Status: ACCEPTED_STORAGE_CONTRACT_SUCCESSOR
Version: v0.2
Predecessor: POSTGRES_STORAGE_CONTRACT_V0_1.md
Supersedes: complete V0.1 storage authority
Candidate base: afb73d83c4804c894582a81af6907f48c13fa0bb
Accepted by: Codex root (explicitly delegated by mayf3)
Accepted date: 2026-10-29
Accepted reviewed head: 1ac73756e22c58fd6ca3c43d1ff4c4292881da26
Acceptance review verdict: PASS
Acceptance record: docs/reports/WORKFLOW_PREREQUISITE_COMPOSITE_ACCEPTANCE_20261029.md
```

This is the accepted whole successor. The complete V0.1 storage contract is
preserved below as the historical baseline. The V0.1 Event uniqueness row and
its Section 8 DDL guarantee are superseded only by the bounded current rules
below. V0.1 is retained intact as historical contract text.

## V0.2 current rules — exact CREATE_AND_BIND_GOVERNANCE_CHILD pair

1. Keep the immutable `workflow_events` primary identity and per-Instance
   `(workflow_instance_id, event_sequence)` uniqueness unchanged.
2. For the exact command type `CREATE_AND_BIND_GOVERNANCE_CHILD`, enforce this
   completed-receipt matrix at commit: a completed successful 2xx receipt has
   exactly two Events with its actual non-null `command_id`—one
   `INSTANCE_CREATED` for the new child and one `GOVERNANCE_CHILD_BOUND` for
   its same-Domain source—and all child, source, activation, binding, response,
   and Event facts commit atomically; a completed deterministic non-2xx receipt
   has zero business Events and zero child, source, activation, or binding
   facts. A failed command retains the existing deterministic failure-receipt
   and same-key replay semantics. For success, the two Event Instance IDs are
   distinct, each contributes exactly one Event for this command, the child
   starts at version 1, and the source advances exactly one version. Any
   missing, extra, wrong-type, wrong-Instance, duplicate-per-Instance, or
   receipt-mismatched Event, or any completed success without the exact pair,
   fails closed. Inflight/PROCESSING and transaction abort behavior otherwise
   retain existing receipt semantics; no partial pair or business facts may
   commit as a successful outcome.
3. Replace the former global partial unique index on non-null `command_id` with
   database-enforced per-Instance uniqueness plus a command-type-aware,
   commit-time integrity rule. The guard MUST serialize by locking the actual
   `workflow_command_receipts` row identified by the Event/receipt's real
   `command_id` before reading or validating that command's Event cardinality;
   a deferred COUNT check without this command-identity lock is insufficient
   because concurrent transactions could both pass on stale snapshots. The
   receipt-completion transition and Event mutations MUST both invoke the
   invariant, so a completed 2xx receipt cannot commit with zero Events and a
   completed deterministic non-2xx receipt cannot commit with business facts.
   The rule MUST preserve the old global at-most-one Event behavior for every
   other command type under concurrency and MUST preserve the exact existing
   `ONE_TIME_MIGRATE` source/successor pair. Merely dropping the global
   index or adding only `(command_id, workflow_instance_id)` uniqueness is
   insufficient: the exact two-event type/instance/receipt shape is mandatory.
   No component receipt, NULL command ID, fabricated provenance, omitted child
   birth Event, or additional command type is permitted.
4. Any direct command-ID reader MUST continue returning its existing one-Event
   result for old command types and explicitly represent the two named Events
   for `CREATE_AND_BIND_GOVERNANCE_CHILD`; it MUST NOT silently pick one,
   duplicate either result, or generalize multi-Event behavior to other types.
5. The single required forward migration MUST validate existing rows before
   enforcement changes, preserve every Event/receipt/history row and ID, and
   fail closed if old data violates the target invariants. The migration must
   not rewrite or delete historical Events. Rollback may restore the former
   global index only if that index installs without error; if any valid pair
   exists, restoration fails closed and cannot be made to pass by deleting or
   changing immutable history. Forward repair or continued pair-aware operation
   is required.

This contract does not authorize production DDL. The migration and implementation
remain subject to their separately accepted source contracts and exact mandate.

---

# POSTGRES_STORAGE_CONTRACT_V0_1 — complete preserved predecessor body

# PostgreSQL Storage Contract v0.1

```text
Status: STORAGE_CONTRACT
Version: v0.1
Architecture: SVC_WORKFLOW_ARCHITECTURE_V0_3_1 (ARCHITECTURE_FROZEN)
```

> 本文档记录 `svc-workflow` PostgreSQL 存储骨架的表设计、约束体系和不可变规则。
> 它是 PR 1（仓储骨架与不可变核心事实表）的实施合同。
> 本文档不改变冻结架构的核心含义。

---

## 1. 表清单与所有权

所有表位于 `public` schema（推荐独立 `workflow` schema，本 PR 暂用 public）。

| # | 表名 | 分类 | 写入者 |
|---|------|------|--------|
| 1 | `principals` | 身份 | svc-workflow 独占 |
| 2 | `domains` | 域 | svc-workflow 独占 |
| 3 | `domain_role_bindings` | 域权限 | svc-workflow 独占 |
| 4 | `workflow_definitions` | 定义 | svc-workflow 独占 |
| 5 | `workflow_definition_versions` | 定义版本 | svc-workflow 独占 |
| 6 | `workflow_node_definitions` | 节点定义 | svc-workflow 独占 |
| 7 | `workflow_transition_definitions` | 转换定义 | svc-workflow 独占 |
| 8 | `workflow_instances` | 运行时 | svc-workflow 独占 |
| 9 | `workflow_context_revisions` | 运行时（不可变事实） | svc-workflow 独占 |
| 10 | `workflow_node_visits` | 运行时（不可变事实） | svc-workflow 独占 |
| 11 | `workflow_submissions` | 运行时（不可变事实） | svc-workflow 独占 |
| 12 | `workflow_events` | 运行时（不可变事实） | svc-workflow 独占 |
| 13 | `workflow_command_receipts` | 命令 | svc-workflow 独占 |
| 14 | `workflow_command_attempt_audits` | 审计 | svc-workflow 独占 |
| 15 | `workflow_security_audits` | 审计 | svc-workflow 独占 |

上层产品即使能连接同一 PostgreSQL 集群，也不得直接写入这些表。

---

## 2. Enum 类型

| PostgreSQL 类型名 | 值 |
|---|---|
| `principal_type` | `HUMAN`, `AGENT`, `SERVICE` |
| `definition_version_status` | `DRAFT`, `PUBLISHED`, `DEPRECATED`, `REVOKED` |
| `node_type` | `DRAFT`, `NORMAL`, `TERMINAL` |
| `assignee_ref_type` | `WORKFLOW_CREATOR`, `DOMAIN_OWNER`, `FIXED_PRINCIPAL` |
| `transition_effect` | `ADVANCE`, `RETURN`, `TERMINATE` |
| `receipt_status` | `PROCESSING`, `COMPLETED` |

---

## 3. 主键与复合唯一键

### 主键

所有实体使用 UUID 主键，由应用层生成（`gen_random_uuid()` 或 Rust 的 `Uuid::new_v4()`）。

### 复合唯一键

| 表 | 唯一键 | 说明 |
|---|---|---|
| `domain_role_bindings` | `(domain_id, principal_id, role_key)` | 同一域同一角色每人最多一个 binding |
| `domain_role_bindings` | `(domain_id, role_key)` WHERE `enabled=true AND role_key='DOMAIN_OWNER'` | 部分唯一索引：最多一个有效 Owner |
| `workflow_definitions` | `(domain_id, definition_key)` | 域内 definition_key 唯一 |
| `workflow_definition_versions` | `(workflow_definition_id, version_number)` | 定义内版本号唯一 |
| `workflow_node_definitions` | `(definition_version_id, node_key)` | 版本内 node_key 唯一 |
| `workflow_node_definitions` | `(definition_version_id, order_index)` | 版本内 order_index 唯一 |
| `workflow_transition_definitions` | `(definition_version_id, transition_key)` | 版本内 transition_key 唯一 |
| `workflow_context_revisions` | `(workflow_instance_id, revision_number)` | 实例内 revision_number 唯一 |
| `workflow_node_visits` | `(workflow_instance_id, node_id, visit_number)` | 实例内同一节点的 visit 编号唯一 |
| `workflow_submissions` | `(source_node_visit_id)` | 每个 visit 最多一个 committed submission |
| `workflow_events` | `(workflow_instance_id, event_sequence)` | 实例内 event_sequence 唯一 |
| `workflow_events` | `(command_id)` WHERE `command_id IS NOT NULL` | 每个 command 最多对应一个 event |
| `workflow_command_receipts` | `(principal_id, idempotency_key)` | 幂等键唯一 |

---

## 4. 复合外键

确保跨表引用属于同一 Workflow Instance。

| 源表 | 源列 | 目标表 | 目标列 | 延迟 |
|---|---|---|---|---|
| `workflow_submissions` | `(source_node_visit_id, workflow_instance_id)` | `workflow_node_visits` | `(node_visit_id, workflow_instance_id)` | DEFERRABLE INITIALLY DEFERRED |
| `workflow_submissions` | `(context_revision_id, workflow_instance_id)` | `workflow_context_revisions` | `(context_revision_id, workflow_instance_id)` | DEFERRABLE INITIALLY DEFERRED |
| `workflow_events` | `(source_node_visit_id, workflow_instance_id)` | `workflow_node_visits` | `(node_visit_id, workflow_instance_id)` | DEFERRABLE INITIALLY DEFERRED |
| `workflow_events` | `(target_node_visit_id, workflow_instance_id)` | `workflow_node_visits` | `(node_visit_id, workflow_instance_id)` | DEFERRABLE INITIALLY DEFERRED |
| `workflow_events` | `(context_revision_id, workflow_instance_id)` | `workflow_context_revisions` | `(context_revision_id, workflow_instance_id)` | DEFERRABLE INITIALLY DEFERRED |
| `workflow_events` | `(submission_id, workflow_instance_id)` | `workflow_submissions` | `(submission_id, workflow_instance_id)` | DEFERRABLE INITIALLY DEFERRED |
| `workflow_context_revisions` | `(previous_revision_id, workflow_instance_id)` | `workflow_context_revisions` | `(context_revision_id, workflow_instance_id)` | DEFERRABLE INITIALLY DEFERRED（自引用） |

### 循环外键（创建实例时）

```sql
workflow_instances (current_context_revision_id, workflow_instance_id)
  → workflow_context_revisions (context_revision_id, workflow_instance_id)
  DEFERRABLE INITIALLY DEFERRED

workflow_instances (current_node_visit_id, workflow_instance_id)
  → workflow_node_visits (node_visit_id, workflow_instance_id)
  DEFERRABLE INITIALLY DEFERRED
```

创建实例时，预生成 `instance_id`、`context_revision_id`、`node_visit_id` 并在同一事务中依次插入：
1. `workflow_instances`（引用尚不存在的 ctx 和 visit）
2. `workflow_context_revisions`（引用 instance_id）
3. `workflow_node_visits`（引用 instance_id）
4. Commit 时所有 FK 验证通过。

---

## 5. 不可变触发器

### 绝对不可变表（禁止 UPDATE 和 DELETE）

| 表 | 触发器 | 效果 |
|---|---|---|
| `workflow_context_revisions` | `trg_context_revisions_immutable` | UPDATE 和 DELETE 均拒绝 |
| `workflow_node_visits` | `trg_node_visits_immutable` | UPDATE 和 DELETE 均拒绝 |
| `workflow_submissions` | `trg_submissions_immutable` | UPDATE 和 DELETE 均拒绝 |
| `workflow_events` | `trg_events_immutable` | UPDATE 和 DELETE 均拒绝 |
| `workflow_command_receipts`（COMPLETED 后） | `trg_command_receipts_completed_immutable` | COMPLETED 后 UPDATE 和 DELETE 均拒绝 |

### 条件不可变表

**workflow_instances**：`trg_instance_immutable_fields` 阻止修改以下字段：
- `domain_id`
- `definition_version_id`
- `created_by_principal_id`
- `created_at`

其他投影字段（`current_context_revision_id`、`current_node_visit_id`、`workflow_state_version`）允许后续 Command Service 更新。

**workflow_definition_versions**：`trg_definition_version_immutable` 阻止 PUBLISHED/DEPRECATED/REVOKED 状态下修改业务字段：
- `definition_digest`
- `json_schema_dialect`
- `validator_version`
- `context_schema`
- `submission_schema`
- `metadata`

### 状态生命周期触发器

**Receipt 状态**：`trg_command_receipts_status_check` 只允许 `PROCESSING → COMPLETED`。

**Definition Version 状态**：`trg_definition_version_status_transition` 只允许：
- `DRAFT → PUBLISHED`
- `PUBLISHED → DEPRECATED`
- `PUBLISHED → REVOKED`
- `DEPRECATED → REVOKED`

禁止：
- `PUBLISHED → DRAFT`
- `DEPRECATED → PUBLISHED`
- `REVOKED → anything`
- `DEPRECATED → DRAFT`

**Completed 时间**：`trg_receipt_set_completed_at` 自动设置 `completed_at`。

**COMPLETED 必填字段**：`trg_receipt_check_completed_fields` 要求 `response_status` 非空。

---

## 6. 大小限制

| 列 | 限制 | DDL 检查 | 说明 |
|---|---|---|---|
| `workflow_context_revisions.payload` | ≤ 1 MiB | `CHECK (pg_column_size(payload) <= 1048576)` | pg_column_size 返回 JSONB 二进制存储大小 |
| `workflow_submissions.payload` | ≤ 1 MiB | `CHECK (pg_column_size(payload) <= 1048576)` | 同上 |
| `workflow_instances.metadata` | ≤ 64 KiB | `CHECK (metadata IS NULL OR pg_column_size(metadata) <= 65536)` | 同上 |
| `workflow_definitions.metadata` | ≤ 64 KiB | `CHECK (metadata IS NULL OR pg_column_size(metadata) <= 65536)` | 同上 |
| `workflow_definition_versions.metadata` | ≤ 64 KiB | `CHECK (metadata IS NULL OR pg_column_size(metadata) <= 65536)` | 同上 |
| `workflow_command_receipts.response_body` | ≤ 1 MiB | `CHECK (response_body IS NULL OR pg_column_size(response_body) <= 1048576)` | 同上 |
| `workflow_events.event_data` | ≤ 256 KiB | `CHECK (event_data IS NULL OR pg_column_size(event_data) <= 262144)` | 同上 |

### 关于 pg_column_size 的说明

`pg_column_size()` 计算的是 JSONB 二进制存储格式的大小，不是输入 JSON 字符串的大小。JSONB 二进制格式包含类型标签、键去重和压缩等开销，因此：
- 实际输入 JSON 字符串可能略大于或略小于 pg_column_size 的值
- 对于绝大多数业务数据，95 分位偏差在 ±10% 以内

**分层覆盖策略**：
- **数据库层**（DDL CHECK）：提供防御性硬限制，防止极端超大 payload 进入数据库
- **服务层**（Rust 验证）：后续 PR 在 Command Service 中实现精确的大小校验，在读取和解码前拒绝超限请求

---

## 7. Transaction 假设

所有工作流状态命令假设以下 PostgreSQL 配置：

```sql
SET transaction_isolation = 'read committed';
```

默认锁顺序：
1. `workflow_command_receipts`（幂等键）
2. `workflow_instances`（行锁）
3. 其他必要只读引用

所有命令事务使用 `ON CONFLICT DO NOTHING RETURNING` 进行乐观幂等插入。

DEFERRABLE INITIALLY DEFERRED 约束在事务提交时一次性验证，允许循环 FK 在单事务中满足。

---

## 8. DDL 覆盖 vs 后续 Command Service 覆盖

| 规则 | DDL 保证 | Command Service 保证 |
|---|---|---|
| Domain Owner 唯一（最多一个 enabled） | ✅ 部分唯一索引 | ✅ 管理事务校验"至少一个" |
| Definition 属于一个 Domain | ✅ FK + 唯一键 | ✅ 路由校验 |
| Definition Version 不可变（发布后） | ✅ 触发器 | ✅ 业务层校验 |
| Context Revision 编号唯一 | ✅ 唯一键 | ✅ |
| Context Revision 禁止跨 Instance 引用 | ✅ 复合 FK | ✅ |
| Context Revision 不可修改/删除 | ✅ 触发器 | ✅ |
| Node Visit 编号唯一 | ✅ 唯一键 | ✅ |
| Node Visit 不可修改/删除 | ✅ 触发器 | ✅ |
| 每个 Visit 最多一个 Submission | ✅ 唯一约束 | ✅ |
| Submission 不能混用其他 Instance | ✅ 复合 FK | ✅ |
| Event 实体引用不能跨 Instance | ✅ 复合 FK | ✅ |
| Event Sequence 唯一 | ✅ 唯一键 | ✅ |
| 一个 Command 最多一个 Event | ✅ 部分唯一索引 | ✅ |
| COMPLETED Receipt 不可修改/删除 | ✅ 触发器 | ✅ |
| Instance 不可变字段 | ✅ 触发器 | ✅ |
| 初始循环外键 | ✅ DEFERRABLE | ✅ 预生成 ID |
| Definition Version 状态回退禁止 | ✅ 触发器 | ✅ |
| 大小限制 | ✅ CHECK 约束 | ✅ 服务层精确校验 |
| Receipt PROCESSING→COMPLETED | ✅ 触发器 | ✅ 业务层校验 |
| 数据 digest 一致性 | ❌ | ✅ 服务层比较 |
| Schema 校验 | ❌ | ✅ Command Service |
| 负责人解析 | ❌ | ✅ Transition 引擎 |
| 幂等请求去重 | ❌ ON CONFLICT (部分) | ✅ 完整 ON CONFLICT 流程 |
| Context 和 Submission JSON Schema 校验 | ❌ | ✅ Command Service |
| Definition 图结构校验 | ❌ | ✅ Definition Service |

---

## 9. Migration 文件

| 文件 | 内容 |
|---|---|
| `0001_identity_domain.sql` | Enums（6 个）、principals、domains、domain_role_bindings |
| `0002_workflow_definition.sql` | workflow_definitions、workflow_definition_versions、workflow_node_definitions、workflow_transition_definitions |
| `0003_runtime.sql` | workflow_instances、workflow_context_revisions、workflow_node_visits、workflow_submissions |
| `0004_workflow_events.sql` | workflow_events |
| `0005_command_audit.sql` | workflow_command_receipts、workflow_command_attempt_audits、workflow_security_audits |
| `0006_triggers_constraints.sql` | 所有触发器（不可变保护、状态生命周期、大小检查）、deferred FK、剩余约束 |

---

## 10. 发布后 Definition 图不可变（审计修复）

| 表 | 触发器 | 效果 |
|---|---|---|
| `workflow_node_definitions` | `trg_node_definitions_graph_immutable` | INSERT/UPDATE/DELETE 仅当父版本为 DRAFT 时允许 |
| `workflow_transition_definitions` | `trg_transition_definitions_graph_immutable` | INSERT/UPDATE/DELETE 仅当父版本为 DRAFT 时允许 |

触发器函数 `fn_check_definition_graph_immutable()` 查询父级 `workflow_definition_versions.version_status`。如果父版本为 PUBLISHED、DEPRECATED 或 REVOKED，拒绝所有修改。

错误信息以 `graph_immutable:` 开头，便于测试断言。

## 11. 审计修复汇总

以下保护由 Migration `0007_definition_graph_immutability.sql` 添加：

### 11.1 Definition 子表保护（Blocker 修复）

见第 10 节。

### 11.2 Workflow Instance 额外不可变字段

`fn_check_instance_immutable_fields()`（重写在 0007 中）新增保护以下字段：

- `external_url` — 创建后不可修改
- `metadata` — 创建后不可修改

### 11.3 PROCESSING Receipt 身份字段保护

新触发器 `trg_receipt_identity_immutable`（0007 创建）保护 `workflow_command_receipts` 在任意状态下：

| 保护字段 | 说明 |
|---|---|
| `command_id` | 命令唯一标识 |
| `principal_id` | 发起者身份 |
| `idempotency_key` | 幂等键 |
| `command_type` | 命令类型 |
| `request_hash` | 请求哈希 |
| `created_at` | 创建时间 |

允许修改的字段：`receipt_status`, `response_status`, `response_body`, `response_digest`, `completed_at`。

### 11.4 Size 约束测试覆盖（7/7）

所有 7 个 size CHECK 约束现在都有测试覆盖：

| 约束名 | 表 | 列 | 限制 | 测试 |
|---|---|---|---|---|
| `chk_ctx_payload_size` | workflow_context_revisions | payload | ≤ 1 MiB | ✅ `test_context_payload_size_limit` |
| `chk_submission_payload_size` | workflow_submissions | payload | ≤ 1 MiB | ✅ `test_submission_payload_size_limit` |
| `chk_instance_metadata_size` | workflow_instances | metadata | ≤ 64 KiB | ✅ `test_instance_metadata_size_limit` |
| `chk_def_metadata_size` | workflow_definitions | metadata | ≤ 64 KiB | ✅ `test_definition_metadata_size_limit` |
| `chk_def_ver_metadata_size` | workflow_definition_versions | metadata | ≤ 64 KiB | ✅ `test_definition_version_metadata_size_limit` |
| `chk_receipt_response_size` | workflow_command_receipts | response_body | ≤ 1 MiB | ✅ `test_receipt_response_body_size_limit` |
| `chk_event_data_size` | workflow_events | event_data | ≤ 256 KiB | ✅ `test_event_data_size_limit` |

### 11.5 Deferred FK 当前策略

所有复合外键目前使用 `DEFERRABLE INITIALLY DEFERRED`。真正需要 deferral 的 FK：

1. `fk_instance_current_ctx` — Instance ↔ ContextRevision 循环引用
2. `fk_instance_current_visit` — Instance ↔ NodeVisit 循环引用
3. `fk_previous_revision` — ContextRevision 自引用循环
4. `fk_primary_advance_transition` — Node ↔ Transition 循环（同一版本内）

以下 FK 没有循环依赖，但当前保持 deferred 以统一开发模式：

- `fk_submission_visit_same_instance`
- `fk_submission_ctx_same_instance`
- `fk_event_source_visit_same_instance`
- `fk_event_target_visit_same_instance`
- `fk_event_ctx_same_instance`
- `fk_event_submission_same_instance`
- `fk_event_command`

**后续收窄点**：当 Command Service 稳定后，可将非循环 FK 改为 `NOT DEFERRABLE`，前提是不破坏现有事务流程。

## 12. 已知限制与后续任务

1. **大小限制**：DDL 中的 `pg_column_size` 检查是防御性硬限制。精确的输入字符串大小校验应在 Rust 服务层实现。
2. **幂等 ON CONFLICT 完整流程**：本 PR 已建立唯一约束保护，但完整的 `INSERT ... ON CONFLICT DO NOTHING RETURNING` 命令执行流程在后续 Command Service PR 实现。
3. **Domain Owner "至少一个"**：数据库保证"最多一个"，"至少一个"由后续 Domain 管理事务保证。
4. **Context Revision 单链**：第一条 Revision 的 `previous_revision_id = null` 及后续链规则由 Command Service 实现，数据库只保证同实例引用完整性。
5. **Definition 图结构校验**：发布前校验（恰好一个 Draft 入口、主干无环、Terminal 无出边等）在后续 Definition Service PR 实现。
6. **eventSchemaVersion 版本管理**：本 PR 存储字符串字段，版本模式由后续 PR 定义。
7. **没有全局 event cursor**：符合 v0.3.1 冻结决策。
8. **OperationalTelemetry**：本 PR 不落领域表，后续使用运行监控系统处理。
