# PR 3C — Submission + Transition 原子命令调查报告

```text
Status: INVESTIGATION_COMPLETE
Base SHA: 4a06c66c25782e184a689e01c00c87b8b4f0db95
Current HEAD: 4a06c66c25782e184a689e01c00c87b8b4f0db95
Branch: main
Workspace: clean
Date: 2026-07-14
```

---

## 1. 仓库状态确认

| 检查项 | 结果 |
|--------|------|
| `git status --short` | (clean) |
| `git branch --show-current` | `main` |
| `git rev-parse HEAD` | `4a06c66c25782e184a689e01c00c87b8b4f0db95` |
| Workspace modified | No |
| Migration diff vs Base | Empty (no diff) |

---

## 2. 可复用通用模式（来自 PR 3A / 3B 审计和复审报告）

### 2.1 CommandReceipt 算法

PR 3A 和 PR 3B 一致实现：

```rust
// Step 1: INSERT ... ON CONFLICT DO NOTHING RETURNING
// Owns the request if row returned
// Step 2: If no row, SELECT ... FOR UPDATE to read existing receipt
//   Same hash + COMPLETED → replay stored response
//   Different hash → IdempotencyConflict + AttemptAudit
//   Still PROCESSING → CommandStillProcessing
```

PR 3C 完全复用此模式。

### 2.2 requestHash 协议

| 属性 | 规则 |
|------|------|
| 字段命名 | `snake_case`（`#[derive(Serialize)]`，无 rename） |
| 排除字段 | `idempotency_key` 不进入 Hash |
| Optional 字段 | `Option::None` → JSON `null` |
| 根结构 | `{ command_schema_version, command_type, route_parameters: {}, request_body: { ... } }` |
| Hash 函数 | `jcs_canonicalize::sha256_jcs_hex` (JCS + SHA-256) |
| 测试要求 | Golden test 固定 canonical JSON 和 SHA-256 hex |

### 2.3 确定性失败 vs 基础设施失败

| 类型 | 行为 |
|------|------|
| 确定性失败 | 完成 Receipt（COMPLETED with error），提交事务，留下可重放的失败记录 |
| 基础设施失败 | 事务整体回滚，包括 PROCESSING Receipt 也被回滚，无残留 |

确定性失败包括：Schema 校验失败、版本冲突、非当前负责人、非法 Transition 等。

### 2.4 锁顺序（已稳定）

```
1. CommandReceipt (INSERT ON CONFLICT / SELECT FOR UPDATE)
2. WorkflowInstance (SELECT ... FOR UPDATE)
3. DefinitionVersion (SELECT ... FOR UPDATE)
4. Domain / Principal / RoleBinding (只读，无锁)
```

### 2.5 stateVersion / eventSequence

旧 mode:
```
workflowStateVersion == N
eventSequence == N (newWorkflowStateVersion)
newWorkflowStateVersion = oldWorkflowStateVersion + 1
```

### 2.6 条件 Trigger 测试隔离模式

PR 3A 的 `TriggerGuard` RAII 模式：
- 唯一 UUID 后缀的 trigger/function 名称
- 条件表达式（`NEW.actor_principal_id = '{principal_id}'`）
- 禁止 `CREATE OR REPLACE`，使用 bare `CREATE FUNCTION / CREATE TRIGGER`
- Drop 时使用独立线程 + 独立连接清理

PR 3C 必须遵循此模式，不得回退到无条件 Trigger。

---

## 3. 现有 Schema 调查

### 3.1 `workflow_instances`

| 属性 | 值 |
|------|-----|
| 主键 | `workflow_instance_id UUID` |
| 不可变字段 | `domain_id`, `definition_version_id`, `created_by_principal_id`, `created_at`, `external_url`, `metadata` |
| 可更新字段（投影） | `current_context_revision_id`, `current_node_visit_id`, `workflow_state_version` |
| CHECK | `workflow_state_version >= 1` |
| 唯一索引 | 无额外（FK 引用由复合外键保护） |
| 与 PR 3C 关系 | Transition 事务必须更新 `current_node_visit_id` 和 `workflow_state_version`；如果在 Transition-only 场景，`current_context_revision_id` 不变 |

### 3.2 `workflow_node_visits`

| 属性 | 值 |
|------|-----|
| 主键 | `node_visit_id UUID` |
| 外键 | `workflow_instance_id` → instances, `node_id` → node_definitions, `assignee_principal_id` → principals |
| 复合唯一键 | `(workflow_instance_id, node_id, visit_number)` |
| 复合唯一键 | `(node_visit_id, workflow_instance_id)` — 用于复合 FK |
| 不可变 Trigger | `trg_node_visits_immutable` — 禁止 UPDATE/DELETE |
| 状态字段 | 无（无 `exited_at`, `OPEN`, `CLOSED`） |
| PR 3C 含义 | 创建目标 NodeVisit 后不可修改；旧 Visit 不更新（无 `exited_at`） |

**关键设计约束**：`workflow_node_visits` 是**完全不可变**的。系统不通过 UPDATE 旧 Visit 来表示"已离开"，而是通过 Instance 的 `current_node_visit_id` 指针和 Event 的 `source_node_visit_id` / `target_node_visit_id` 表示。

### 3.3 `workflow_submissions`

| 属性 | 值 |
|------|-----|
| 主键 | `submission_id UUID` |
| 外键 | `workflow_instance_id` → instances, `source_node_visit_id` → node_visits, `context_revision_id` → context_revisions, `author_principal_id` → principals, `transition_id` → transition_definitions |
| 复合唯一键 | `(source_node_visit_id)` — **强制一个 Visit 最多一个 Submission** |
| 复合唯一键 | `(submission_id, workflow_instance_id)` — 用于复合 FK |
| 复合外键 | `(source_node_visit_id, workflow_instance_id)` → `node_visits(node_visit_id, workflow_instance_id)` DEFERRABLE |
| 复合外键 | `(context_revision_id, workflow_instance_id)` → `context_revisions` DEFERRABLE |
| CHECK | `payload_digest ~ '^[0-9a-f]{64}$'` |
| CHECK | `pg_column_size(payload) <= 1048576` (1 MiB) |
| 不可变 Trigger | `trg_submissions_immutable` |
| schema_version | `TEXT NOT NULL CHECK (char_length(...) >= 1 AND ... <= 64)` |
| PR 3C 含义 | Schema 已完整支持 Submission；唯一约束强制**一个 Visit 最多一个 Submission**；复合 FK 确保 same-instance 引用完整性 |

### 3.4 `workflow_events`

| 属性 | 值 |
|------|-----|
| 主键 | `event_id UUID` |
| 外键 | `workflow_instance_id` → instances, `actor_principal_id` → principals, `command_id` → receipts, `from_node_id`/`to_node_id` → node_definitions |
| 复合唯一键 | `(workflow_instance_id, event_sequence)` |
| 部分唯一索引 | `(command_id) WHERE command_id IS NOT NULL` — 一个命令最多一个 Event |
| CHECK | `new_workflow_state_version = old_workflow_state_version + 1` |
| CHECK | `event_sequence = new_workflow_state_version` |
| 复合外键 | `source_node_visit_id`/`target_node_visit_id` → `node_visits` (same-instance) |
| 复合外键 | `context_revision_id` → `context_revisions` (same-instance) |
| 复合外键 | `submission_id` → `submissions` (same-instance) |
| 不可变 Trigger | `trg_events_immutable` |
| event_type | `TEXT NOT NULL CHECK (length 1-128)` — 无 PostgreSQL ENUM 约束 |
| transition_effect | `transition_effect` ENUM (`ADVANCE`, `RETURN`, `TERMINATE`) |
| PR 3C 含义 | Schema 已完整支持 Transition Event；`command_id` 唯一索引确保一个命令最多一个 Event；复合 FK 覆盖所有引用关系 |

### 3.5 `workflow_command_receipts`

| 属性 | 值 |
|------|-----|
| 主键 | `command_id UUID` |
| 唯一索引 | `(principal_id, idempotency_key)` |
| CHECK | `request_hash ~ '^[0-9a-f]{64}$'` |
| 不可变 Trigger（PROCESSING 身份字段） | `trg_receipt_identity_immutable` |
| 不可变 Trigger（COMPLETED） | `trg_command_receipts_completed_immutable` |
| PR 3C 含义 | `command_type` 字段是文本，需要定义新的 command type 字符串 |

### 3.6 `workflow_command_attempt_audits`

标准审计表，记录冲突和失败尝试。

### 3.7 `workflow_definition_versions`

| 属性 | 值 |
|------|-----|
| `submission_schema` | `JSONB` — 定义版本级别的全局 Submission Schema（被下文 Transition 级别的覆盖） |
| `version_status` | `DRAFT`, `PUBLISHED`, `DEPRECATED`, `REVOKED` |
| PR 3C 含义 | Transition 级别的 `submission_schema` 在 `workflow_transition_definitions` 表上，应按 Transition 级别优先于版本级别 |

### 3.8 `workflow_node_definitions`

| 属性 | 值 |
|------|-----|
| `node_type` | `DRAFT`, `NORMAL`, `TERMINAL` |
| `assignee_ref_type` | `WORKFLOW_CREATOR`, `DOMAIN_OWNER`, `FIXED_PRINCIPAL` |
| `fixed_principal_id` | 可选固定负责人 |
| `primary_advance_transition_id` | 非终态节点的唯一正常推进 Transition |
| PR 3C 含义 | 通过 Node 的 `node_type` 检查状态门禁；通过 `primary_advance_transition_id` 确认 ADVANCE 合法性 |

### 3.9 `workflow_transition_definitions`

| 属性 | 值 |
|------|-----|
| 主键 | `transition_id UUID` |
| 复合唯一键 | `(definition_version_id, transition_key)` |
| 复合唯一键 | `(transition_id, definition_version_id)` |
| `source_node_id` | FK → `node_definitions` |
| `target_node_id` | FK → `node_definitions` |
| `transition_effect` | ENUM: `ADVANCE`, `RETURN`, `TERMINATE` |
| `submission_schema` | `JSONB` — **每条 Transition 定义自己的 Submission Schema** |
| PR 3C 含义 | Transition Schema 是逐条定义的；架构冻结文档明确"每条 Transition 定义自己的 JSON Schema"（Section 13.2）；PR 3C 应优先使用 Transition 级别的 `submission_schema` |

### 3.10 `principals`, `domains`, `domain_role_bindings`

标准身份和域权限表，PR 3C 需要读取当前 assignee 的 enabled 状态，以及 Domain Owner 解析。

---

## 4. Submission 语义（冻结架构证据）

### 4.1 Submission Schema 来源

**冻结架构（Section 13.2）**：每条 Transition 定义自己的 JSON Schema。`workflow_transition_definitions.submission_schema` 是权威来源。

版本级别的 `workflow_definition_versions.submission_schema` 是全流程共享模式，但 Transition 级别的 Schema 在发布时已经过验证。

PR 3C 实施：**优先使用 Transition Definition 的 `submission_schema`** 进行校验。如果 `submission_schema` 为 NULL，表示该 Transition 不需要提交任何业务数据（空 `{}` 也被接受）。

### 4.2 空 Submission 是否允许

- ADVANCE（正常完成到 done）：允许空 Submission（`{}`）
- ADVANCE（非终态）：可能需要 Schema 约束的实际字段
- RETURN：必须包含 `rootCauseNodeVisitId`, `relatedSubmissionIds`, `reasonCode`, `reason`
- TERMINATE：必须包含 `reasonCode`, `reason`
- 管理员 TERMINATE：不创建普通业务 Submission

### 4.3 同一 Visit 最多一个 Submission

数据库唯一约束 `UNIQUE (source_node_visit_id)` 强制保证。

### 4.4 重放返回同一 Submission ID

确定性幂等重放通过 COMPLETED CommandReceipt 返回原始响应。响应中包含 `submissionId`，客户端获得同一 ID。

### 4.5 RETURN 后的新 Visit 有新 Submission

RETURN 创建全新 NodeVisit（`visit_number` 递增），新 Visit 可以有新 Submission。Submission 唯一性绑定 `source_node_visit_id`（Visit 级），不是 `node_id` 级。

---

## 5. Transition Effect 语义

### 5.1 ADVANCE

| 属性 | 结论 |
|------|------|
| 来源 | 当前 Node 的 `primaryAdvanceTransitionId` |
| Source 状态 | 当前非终态 Visit（DRAFT 或 NORMAL） |
| Target 类型 | NORMAL, DRAFT（RETURN 场景）, 或 TERMINAL |
| primary 用途 | 构成唯一正常主干 |
| 正常完成到 done | **仍为 ADVANCE**（Section 10.2：即使目标是成功 Terminal Node，仍为 ADVANCE） |
| DRAFT 节点能否 ADVANCE | 可以。DRAFT 是唯一入口，创建者完成内容后通过 primary ADVANCE 离开 |
| NORMAL 节点能否 ADVANCE | 可以，通过自己的 primary ADVANCE Transition |

### 5.2 RETURN

| 属性 | 结论 |
|------|------|
| 来源 | 非 primary 的 Transition，`transition_effect = RETURN` |
| Target 限制 | 必须是 `orderIndex` 更小的非终态 Node |
| 是否创建新 Visit | 是，创建全新 NodeVisit（`visit_number` 递增） |
| 旧 Visit 状态 | 不修改旧 Visit（NodeVisit 不可变），Instance 指针移到新 Visit |
| 旧 Submission | 保留，不可变 |
| Target assignee | 重新解析并快照 |
| 返回 DRAFT | 允许，这是标准返工流程 |
| Submission 要求 | 必须包含 `rootCauseNodeVisitId`, `relatedSubmissionIds`, `reasonCode`, `reason` |

### 5.3 TERMINATE

| 属性 | 结论 |
|------|------|
| 来源 | 非 primary 的 Transition，`transition_effect = TERMINATE` |
| Target 限制 | 必须为 TERMINAL Node |
| 是否创建 Target Visit | 是，创建目标 Terminal Node 的 Visit |
| 正常 TERMINATE 是否需要 Submission | **是**（Section 13.6），必须包含 `reasonCode`, `reason` |
| 管理员 TERMINATE | 不创建普通 Submission（唯一例外） |
| 权限 | 只能是当前 assignee（普通 TERMINATE） |
| 是否直接标记 Instance Terminal | 不。进入 Terminal Node 后仍然保持 Instance 记录，`currentNodeVisitId` 指向终态 Visit |

---

## 6. 命令形态选择

### 结论：采用候选 B — `ExecuteTransition`

命令名：**`ExecuteWorkflowTransition`**

Submission 作为可选字段：

```rust
pub struct ExecuteWorkflowTransitionCommand {
    pub principal_id: PrincipalId,
    pub idempotency_key: String,
    pub command_schema_version: String,
    pub workflow_instance_id: WorkflowInstanceId,
    pub expected_workflow_state_version: i32,
    pub transition_definition_id: TransitionId,
    pub submission_payload: Option<serde_json::Value>,
}
```

### 为什么不是候选 A（SubmitAndTransition）？

候选 A 将 Submission 和 Transition 捆绑为一个"原子操作名"。但架构冻结（Section 13）明确 Transition 可以不带 Submission（如简单 Todo 的 ADVANCE），也可以带 Submission（如正常完成、RETURN 或 TERMINATE）。将两者绑定为同一个名字会产生语义误导，暗示 Submission 总是必须的。

### 为什么不是候选 C（CreateSubmission + ExecuteTransition 拆分）？

架构冻结（Section 15）明确指出"提交即流转"——Submission 和 Transition 在同一个 PostgreSQL 事务中完成。拆分会导致：

1. **原子性违背**：`CreateSubmission` 成功后若 `ExecuteTransition` 失败，留下孤立 Submission
2. **一个 Visit 一个 Submission 约束**：如果先创建 Submission 再执行 Transition，中间状态是一个"已提交但未流转"的 Visit
3. **一个成功状态命令一个 Event**：拆分后可能产生两个 Event（一个 Submission 事件 + 一个 Transition 事件），违反冻结规则（Section 14）

### PR 3C / PR 3D 边界

| PR | 命令 | 功能 |
|----|------|------|
| PR 3C | `ExecuteWorkflowTransition` | Transition-only，不修改 Context |
| PR 3D | `ReviseContextAndTransition` | Context Revision + Transition 原子组合 |

**分界**：
- PR 3C 的 `ExecuteWorkflowTransition` 中 `submission.context_revision_id` 由服务端绑定锁内 `currentContextRevisionId`
- PR 3D 的组合命令在事务内先创建新 Context Revision，再创建 Submission（绑定新 Revision），然后 Transition
- PR 3D 使用 `WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED` Event 类型
- PR 3C 使用 `WORKFLOW_TRANSITION_COMMITTED` Event 类型

---

## 7. 命令输入边界

### 7.1 由客户端传入

| 字段 | 理由 |
|------|------|
| `principal_id` | 认证身份 |
| `idempotency_key` | 幂等键 |
| `command_schema_version` | 版本兼容 |
| `workflow_instance_id` | 目标实例 |
| `expected_workflow_state_version` | 乐观并发控制 |
| `transition_definition_id` | Transition 选择（权威来源，见下文） |
| `submission_payload` | 可选，Transition Schema 校验 |

### 7.2 为什么客户端传 transition_definition_id，而不是 transition_key 或 effect？

`transition_definition_id` 是 UUID 主键，是 Transition Definition Row 的唯一事实来源。架构冻结的幂等协议已使用 `workflow_transition_definitions.transition_id` 作为 Submission 的 `transitionId` 字段。

客户端传 `transition_key` 会增加一次查找（key → UUID），且 `(definition_version_id, transition_key)` 需要额外校验。直接传 UUID 更简单。

客户端**不能**传 `effect`, `target_node_id` 或 `transition_key` 让服务端自己选 Transition——这违反"服务端校验 Transition 合法性"的契约。

### 7.3 由服务端生成

| 字段 | 生成方式 |
|------|----------|
| `target_node_id` | 从 `transition_definition` 读取 |
| `new_node_visit_id` | 服务端 `Uuid::new_v4()` |
| `visit_number` | 服务端计算：`SELECT MAX(visit_number)+1 FROM node_visits WHERE instance_id = ... AND node_id = target_node_id` |
| `resolved_assignee` | 服务端事务内解析（同 PR 3A 的 `resolve_assignee`） |
| `new_state_version` | 服务端计算：`old_state_version + 1` |
| `event_sequence` | `= new_state_version` |
| `event_id` | 服务端 `Uuid::new_v4()` |
| `submission_id` | 服务端 `Uuid::new_v4()` |
| `command_id` | 服务端 `Uuid::new_v4()` |

---

## 8. 授权模型

### 8.1 唯一执行者：当前 NodeVisit 的 `assignee_principal_id`

**结论**：只有 `current NodeVisit.assignee_principal_id` 可以执行 Transition。

### 8.2 其他身份

| 身份 | 是否可以执行 Transition |
|------|------------------------|
| Workflow Creator（非当前 assignee） | **否** |
| Domain Owner（非当前 assignee） | **否** |
| 当前 assignee（disabled） | **否**（pre-validation check） |
| 管理员紧急修复 | 独立命令（`ADMIN_EMERGENCY_OVERRIDE`），不共用 Transition 路径 |

### 8.3 assignee 解析后统一比较

`FIXED_PRINCIPAL`、`DOMAIN_OWNER`、`WORKFLOW_CREATOR` 在**创建 NodeVisit 时已经解析为具体 `assignee_principal_id`** 并快照到 Visit 中。Transition 时只比较 `current_node_visit.assignee_principal_id`，不需要重新解析。

### 8.4 disabled assignee 处理

在事务内检查 principal 的 `enabled` 状态。disabled → 确定性失败（COMPLETED receipt with 403）。

---

## 9. 目标 NodeVisit 创建

### 9.1 字段来源

| 字段 | 值 | 来源 |
|------|-----|------|
| `node_visit_id` | 服务端 `Uuid::new_v4()` | 预生成 |
| `workflow_instance_id` | `current_instance.workflow_instance_id` | 从 Instance 行读取 |
| `node_id` | `transition.target_node_id` | 从 TransitionDefinition 读取 |
| `visit_number` | `SELECT COALESCE(MAX(visit_number), 0) + 1 FROM node_visits WHERE instance_id = $1 AND node_id = $2` | 事务内查询 |
| `assignee_principal_id` | 事务内重新解析目标 Node 的 assignee | 同 `resolve_assignee` 模式 |
| `entered_by_transition_id` | `transition_definition_id` | 本次 Transition 的 ID |
| `created_at` | `NOW()` | 事务时间 |

### 9.2 是否需要 `source_visit_id`？

NodeVisit 表上没有 `source_visit_id` 字段。事件矩阵中，`source_node_visit_id` 在 Event 上记录，不在 Visit 上。**不需要添加**。

### 9.3 是否需要 UPDATE 旧 Visit？

**不需要。** 架构冻结（Section 12）明确 NodeVisit 不可变。旧 Visit 不保存 `exited_at` 或 `left` 状态。Instance 的 `current_node_visit_id` 指针和 Event 的 `source_node_visit_id` 共同表达"已离开"。

### 9.4 唯一当前节点保证

Transition 事务更新 `instance.current_node_visit_id = new_node_visit_id`。这是唯一查询投影。

---

## 10. 目标 Assignee 解析

### 10.1 解析规则（复用 PR 3A 模式）

同一 `resolve_assignee` 逻辑：

```rust
match target_node.assignee_ref_type {
    WORKFLOW_CREATOR => instance.created_by_principal_id,
    DOMAIN_OWNER => SELECT enabled DOMAIN_OWNER FROM domain_role_bindings WHERE domain_id = $1,
    FIXED_PRINCIPAL => target_node.fixed_principal_id,
}
```

### 10.2 并发 Domain Owner 变更

Domain Owner 变更通过 `DOMAIN_OWNER` 管理事务（停用旧 Binding + 创建新 Binding）完成，锁住 `domain_role_bindings` 行。Transition 事务在锁住 Instance 后读取 Domain Owner。

**锁顺序分析**：
- Transition 锁：Receipt → Instance → DomainRoleBinding（只读，无锁）
- Domain Owner 变更锁：DomainRoleBinding（`FOR UPDATE`）

**不存在死锁环**，因为 Transition 不锁 DomainRoleBinding（只读），Owner 变更只锁 Binding。

### 10.3 解析结果快照

解析结果写入 `new_node_visit.assignee_principal_id`，后续 Domain Owner 变更不追溯修改。

---

## 11. Definition Version 生命周期和锁

### 11.1 状态门禁

| 状态 | Transition 允许 |
|------|----------------|
| `PUBLISHED` | ✅ 允许 |
| `DEPRECATED` | ✅ 允许（已有实例继续运行） |
| `REVOKED` | ❌ 拒绝（只允许管理员紧急修复） |
| `DRAFT` | ❌ 内部一致性错误（实例不应引用 DRAFT 版本） |

### 11.2 并发 Revoke 安全

Transition 事务使用 `SELECT ... FROM workflow_definition_versions WHERE id = $1 FOR UPDATE` 锁定版本行。如果 Revoke 先获得锁，Transition 等待后在锁内读到 `REVOKED` 状态并拒绝。反之亦然。

### 11.3 Lock 顺序

```
1. CommandReceipt (INSERT ON CONFLICT / SELECT FOR UPDATE)
2. WorkflowInstance (SELECT ... FOR UPDATE)
3. DefinitionVersion (SELECT ... FOR UPDATE)
4. 其他只读数据
```

**与 Context Revision (PR 3B) 的锁顺序完全一致**，不会出现死锁。

---

## 12. 原子事务步骤

### 推荐完整事务顺序

```
BEGIN
  1. 尝试 INSERT CommandReceipt (PROCESSING)
     ON CONFLICT DO NOTHING RETURNING
     → 已有 Receipt 则跳转到重放分支

  2. SELECT ... FROM workflow_instances WHERE id = $1 FOR UPDATE
     → 锁定 Instance

  3. 校验 expected_workflow_state_version
     → 不匹配则确定性失败

  4. 校验当前 assignee (principal_id = node_visit.assignee_principal_id)
     → 不匹配则确定性失败
     → 注意：re-audit 报告 M1 指出错误类型问题，建议使用正确的授权错误码（如 403），而非 PrincipalNotFound

  5. 校验 Principal 启用状态
     → disabled 则确定性失败

  6. SELECT ... FROM workflow_definition_versions WHERE id = $1 FOR UPDATE
     → 校验版本状态（PUBLISHED / DEPRECATED 允许，REVOKED 拒绝，DRAFT 内部错误）

  7. 确认当前 Node Visit 存在且属于 Instance
     → 读取 node_visit.node_id + node_definitions.node_type

  8. 读取 TransitionDefinition
     → 校验 source_node_id = current_node_visit.node_id
     → 校验 belongs to instance.definition_version_id
     → 读取 transition_effect, target_node_id, submission_schema

  9. 如果有 submission_payload:
     → 校验 TransitionSchema（transition.submission_schema）
     → 校验大小（≤ 1 MiB）
     → 计算 payload_digest

  10. 读取 target NodeDefinition
      → 校验 node_type（TERMINAL vs non-TERMINAL，匹配 effect）

  11. 解析 target assignee
      → 同 PR 3A resolve_assignee 模式

  12. 计算 visit_number
      → SELECT MAX(visit_number) + 1 FROM node_visits WHERE instance_id = $1 AND node_id = $2

  13. INSERT INTO workflow_submissions (若需要)

  14. INSERT INTO workflow_node_visits (新目标 Visit)

  15. UPDATE workflow_instances
      SET current_node_visit_id = $1, workflow_state_version = $2
      (如果在 Transition-only: current_context_revision_id 不变)

  16. INSERT INTO workflow_events (WORKFLOW_TRANSITION_COMMITTED)

  17. UPDATE CommandReceipt → COMPLETED (200)

COMMIT
```

### 确定性失败点

| 步骤 | 条件 | 状态码 | 错误码 |
|------|------|--------|--------|
| 3 | expectedVersion 不匹配 | 409 | `workflow_state_version_conflict` |
| 4 | 非当前 assignee | 403 | `principal_not_assignee` |
| 5 | Principal disabled | 403 | `principal_disabled` |
| 6 | Version REVOKED | 409 | `definition_version_revoked` |
| 7 | 当前 Visit 不存在 | 404 | `current_visit_not_found` |
| 8 | Transition 不属于当前 source | 409 | `transition_not_applicable` |
| 8 | Transition 不属于当前 Definition Version | 409 | `transition_not_applicable` |
| 9 | Submission Schema 校验失败 | 422 | `submission_validation_failed` |
| 9 | Submission 大小超限 | 413 | `size_limit_exceeded` |
| 10 | Target Node 类型与 effect 不匹配 | 500 | `internal_consistency_error` |
| 11 | Assignee 解析失败 | 422 | `assignee_resolution_failed` |

### 基础设施失败点

| 步骤 | 失败类型 | 行为 |
|------|----------|------|
| 13 | Submission INSERT 失败 | 事务回滚，无残留 |
| 14 | NodeVisit INSERT 失败 | 事务回滚，无残留 |
| 15 | Instance UPDATE 失败 | 事务回滚，无残留 |
| 16 | Event INSERT 失败 | 事务回滚，无残留 |
| 17 | Receipt Completion 失败 | 事务回滚，新 Receipt 回滚 |

---

## 13. Event 字段矩阵

### Event 类型

```
WORKFLOW_TRANSITION_COMMITTED
```

### 矩阵

| 字段 | 值 |
|------|-----|
| `event_type` | `WORKFLOW_TRANSITION_COMMITTED` |
| `transition_effect` | `ADVANCE` / `RETURN` / `TERMINATE`（来自 Transition Definition） |
| `source_node_visit_id` | 当前 Visit（旧 Visit） |
| `target_node_visit_id` | 新创建的 Visit |
| `context_revision_id` | 命令完成后的 `currentContextRevisionId`（Transition-only 时等于命令前） |
| `submission_id` | 本命令创建的 Submission ID（若 `submission_payload` 存在）或 NULL |
| `old_workflow_state_version` | 命令前版本 `N` |
| `new_workflow_state_version` | 命令后版本 `N+1` |
| `event_sequence` | `N+1` |
| `actor_principal_id` | 执行者 principal ID |
| `command_id` | 当前 Receipt 的 command_id |
| `event_schema_version` | `"v1"` |
| `from_node_id` | `source_node_visit.node_id` |
| `to_node_id` | `target_node_visit.node_id` |

### Event Data

```json
{
  "transitionDefinitionId": "...",
  "transitionKey": "...",
  "transitionEffect": "ADVANCE",
  "sourceNodeId": "...",
  "targetNodeId": "...",
  "sourceNodeVisitId": "...",
  "targetNodeVisitId": "...",
  "contextRevisionId": "...",
  "submissionPayloadDigest": "sha256..." // if submission exists
}
```

`event_data_digest` = JCS(event_data) → SHA-256。

**Event Data 不应包含完整 Submission**。

---

## 14. requestHash 草案

### Canonical 结构

```json
JCS({
  "command_schema_version": "v1",
  "command_type": "EXECUTE_WORKFLOW_TRANSITION",
  "route_parameters": {},
  "request_body": {
    "principal_id": "<uuid>",
    "workflow_instance_id": "<uuid>",
    "expected_workflow_state_version": 2,
    "transition_definition_id": "<uuid>",
    "submission_payload": {}
  }
}) → SHA-256
```

### 规则

| 属性 | 值 |
|------|-----|
| `submission_payload` 可选 | 当 `None` 时序列化为 JSON `null` |
| 字段命名 | `snake_case` |
| `idempotency_key` 排除 | 不进入 Hash |
| Golden test | 需要固定的 canonical JSON + SHA-256 hex |

---

## 15. 成功响应草案

### 最小字段

```json
{
  "workflowInstanceId": "...",
  "workflowStateVersion": 3,
  "currentContextRevisionId": "...",
  "sourceNodeVisitId": "...",
  "currentNodeVisitId": "...",
  "submissionId": "...",
  "eventSequence": 3
}
```

### submissionId 空值处理

当 `submission_payload` 为 `None` 时，`submissionId` 应为 `null`（JSON null）。客户端应处理可空字段。

`response_digest` = JCS(response) → SHA-256。

---

## 16. Migration 缺口

### 调查结果

```
git diff --name-status 4a06c66c25782e184a689e01c00c87b8b4f0db95..HEAD -- migrations/
(empty)
```

### 结论：存在 BLOCKING_MIGRATION_REQUIRED

现有 Schema 不足以实现 PR 3C。需要以下 Migration：

### M1: `workflow_node_visits` 缺少 `entered_by_transition_id` 的 FK

当前 `workflow_node_visits` 有 `entered_by_transition_id UUID` 列但没有 FK 约束。需要添加 FK：

```sql
ALTER TABLE workflow_node_visits
    ADD CONSTRAINT fk_visit_entered_by_transition
    FOREIGN KEY (entered_by_transition_id, workflow_instance_id)
    REFERENCES workflow_transition_definitions (transition_id, definition_version_id)
    DEFERRABLE INITIALLY DEFERRED;
```

**注意**：`entered_by_transition_id` 引用的是 `workflow_transition_definitions`，它工作在 `definition_version_id` 上而非 `workflow_instance_id` 上。由于 Transition Definition 不属于同一个 Instance（它属于 Definition Version），这个复合 FK 需要仔细设计。

**实际分析**：当前 `entered_by_transition_id` 列已经是 UUID 类型但没有 FK 约束。PR 3C 在 INSERT NodeVisit 时填入该值。严格来说，Transition Definition 在发布后不可变，Instance 引用它不需要跨 Instance FK 保护。可以直接保留无 FK 约束的设计，只在应用层保证引用合法性。

**结论**：不需要新增 FK 约束。现有设计是充分的。

### BLOCKING MIGRATION 最终结论

**无需 Migration。**

现有 Schema 完整支持 PR 3C 的所有写入：
- `workflow_submissions` 表存在且约束完整
- `workflow_node_visits` 表存在且约束完整
- `workflow_events` 表存在且约束完整
- `workflow_instances` 可更新 `current_node_visit_id` + `workflow_state_version`
- `workflow_transition_definitions.submission_schema` 可用

---

## 17. 并发线性化分析

### 场景 1：相同 key/hash

- 第一个请求插入 PROCESSING Receipt，获得所有权
- 第二个请求遇到已存在的 Receipt，等待后重放 COMPLETED 响应
- 不创建第二个 Submission/Visit/Event

### 场景 2：不同 key、同 expectedVersion

- 一个请求先获得 Instance 行锁，成功 Transition
- 另一个请求进入后读取已更新的 stateVersion，发生 `WorkflowStateVersionConflict`
- 确定性失败 Receipt 持久化
- 不产生第二 Submission/Visit/Event

### 场景 3：相同 key、不同 Transition 或 Submission

- 一个请求先成为权威
- 另一个请求发现 requestHash 不匹配 → `IdempotencyConflict`
- 写 AttemptAudit，不修改原 Receipt

### 场景 4：Context Revision 与 Transition 同 expectedVersion 并发

- 两个请求竞争 Instance 行锁
- 先获得锁的成功，后获得的 `WorkflowStateVersionConflict`
- 如果先获得的是 Context Revision，`workflowStateVersion` 增加
- 后获得的 Transition 因 expectedVersion 过时而拒绝
- **不会出现"劫持"现象**

---

## 18. 测试矩阵

### 最少测试项：约 40-45 项

### 成功路径（10+）

| # | 测试场景 | 说明 |
|---|---------|------|
| 1 | DRAFT → NORMAL (ADVANCE) 成功 | 主干推进 |
| 2 | NORMAL → TERMINAL (ADVANCE) 成功 | 正常完成到 done |
| 3 | RETURN 成功 | 返回前序节点 |
| 4 | TERMINATE 成功 | 异常终止到 TERMINAL |
| 5 | 带 Submission 的 ADVANCE | Schema 校验通过 |
| 6 | 无 Submission 的 ADVANCE | Transition Schema = null |
| 7 | 目标 Visit 创建正确 | visit_number, node_id, assignee |
| 8 | stateVersion/eventSequence 正确 | +1 且相等 |
| 9 | Event 矩阵验证 | 所有字段正确 |
| 10 | Payload/response digest 回读 | 一致性 |

### 授权（6+）

| # | 测试场景 | 说明 |
|---|---------|------|
| 11 | 当前 assignee 成功 | 正常授权 |
| 12 | 非 assignee（其他有效 principal）拒绝 | 403 |
| 13 | 非 assignee（Workflow Creator）拒绝 | 403 |
| 14 | 非 assignee（Domain Owner）拒绝 | 403 |
| 15 | Disabled assignee 拒绝 | 403 |
| 16 | 非 assignee 但 Domain Owner 拒绝 | 明确测试 |

### Definition 和 Transition 校验（8+）

| # | 测试场景 | 说明 |
|---|---------|------|
| 17 | Transition 不属于当前 source Node | 拒绝 409 |
| 18 | Transition 属于其他 Definition Version | 拒绝 409 |
| 19 | Target Node 不属于 Definition Version | 拒绝 |
| 20 | Version REVOKED | 拒绝 409 |
| 21 | Version DEPRECATED | 允许 |
| 22 | Version DRAFT（内部错误） | 500 |
| 23 | primary ADVANCE 但 target 缺失 | 内部一致性 |
| 24 | RETURN 到更高的 orderIndex | 拒绝（应走图校验） |

### Submission Schema 校验（6+）

| # | 测试场景 | 说明 |
|---|---------|------|
| 25 | Schema 合法 payload | 成功 |
| 26 | required 字段缺失 | 422 |
| 27 | 类型错误 | 422 |
| 28 | additionalProperties: false | 422 |
| 29 | 本地 $ref 可用 | 成功 |
| 30 | payload 超过 1 MiB | 413 |

### 幂等和并发（6+）

| # | 测试场景 | 说明 |
|---|---------|------|
| 31 | 相同 key/hash → 重放相同结果 | 同一 submission/visit/event |
| 32 | 重放不增加 stateVersion | 正确 |
| 33 | 相同 key、不同 payload → Conflict + Audit | 409 |
| 34 | 不同 key、同 expectedVersion → 一个成功一个冲突 |  |
| 35 | 与 Context Revision 同 expectedVersion 并发 | 一个成功 |
| 36 | PROCESSING Receipt → CommandStillProcessing | 425 |

### 原子性故障注入（5+）

| # | 测试场景 | 说明 |
|---|---------|------|
| 37 | Submission INSERT 失败 → 全部回滚 | 条件 Trigger |
| 38 | NodeVisit INSERT 失败 → 全部回滚 | 条件 Trigger |
| 39 | Instance UPDATE 失败 → 全部回滚 | 条件 Trigger |
| 40 | Event INSERT 失败 → 全部回滚 | 条件 Trigger |
| 41 | Receipt Completion 失败 → 全部回滚 | 条件 Trigger |

**注意**：所有原子性故障注入测试必须使用 PR 3A 模式的条件 Trigger（`TriggerGuard` + 主键条件），不得使用无条件 Trigger。

---

## 19. 结构规划

### 当前状态

```text
tests/
  01_migration_tests.rs
  02_domain_owner_tests.rs
  03_context_revision_constraints.rs
  ... (14 more flat files/dirs)
  17_workflow_runtime/        # 目录（子项 2+7=9）
    17_workflow_runtime.rs    # mod + 种子函数
    instance_create/          # 7 个子项
    context_revision/         # 7 个子项
  common/
```

`tests/` 直接子项数：20（**已达到极限**）。

### 建议：在 `17_workflow_runtime/` 下新增 `transition/`

```text
tests/17_workflow_runtime/
  instance_create/
  context_revision/
  transition/                 # 新增
    success.rs
    authorization.rs
    definition_gates.rs
    submission_validation.rs
    idempotency.rs
    concurrency.rs
    atomicity.rs
    request_hash_contract.rs
```

- `17_workflow_runtime/` 子项：`instance_create/` + `context_revision/` + `transition/` = 3 目录 + `mod.rs` = 4
- 目录深度：3（`tests/17_workflow_runtime/transition/success.rs`）
- 不增加 `tests/` 直接子项数（仍为 20）

### 源码现状

| 最大文件 | 行数 | 限制 |
|---------|------|------|
| `revise_transaction.rs` | 477 | 500 ✅ |
| `create_transaction.rs` | 455 | 500 ✅ |

**PR 3C 应创建新的 `transition_transaction.rs`**（建议 < 500 行），不塞入现有文件。

---

## 20. 当前 Medium 债务处理建议

| # | 问题 | 建议 |
|---|------|------|
| M1 | 非 Creator 错误类型不准确（返回 PrincipalNotFound 而非授权错误） | 在 PR 3C 的 `principal_not_assignee` 中直接使用正确错误类型（403），不复用 M1 模式。PR 3C 应自始使用正确的授权错误码 |
| M2 | PR 3B 缺少 Instance UPDATE / Receipt Completion 独立故障测试 | PR 3C 必须包含这些测试（与 PR 3B 缺陷无关，PR 3C 自己的测试全覆盖即可） |
| M3 | Schema 编译错误分类（`validator_for` 编译出错被归类为 422） | 此问题跨 PR，需统一修复。建议在独立的"Schema 错误分类修复"PR 中处理，不扩大 PR 3C |
| M4 | `tests/` 顶层达 20 个 | 结构规划中的 transition 子目录方案不增加顶层子项，不加剧此问题 |
| M5 | Golden canonical JSON 常量未直接断言 | PR 3C 的 Golden test 应直接断言 canonical JSON 和 SHA-256 |

---

## 21. Blocker

**无 Blocker。**

现有 Schema 完整支持 PR 3C。不需要新增 Migration。

---

## 22. High-Risk Design Gaps

| # | 风险 | 缓解 |
|---|------|------|
| HR1 | `entered_by_transition_id` 无 FK 保护 | Transition Definition 发布后不可变，Instance 引用它是只读查找。应用层保证即可 |
| HR2 | NodeVisit `visit_number` 计算存在并发安全风险 | MAX + 1 在 Instance 行锁保护下安全。同一 Instance 不会有两个并发 Transition |
| HR3 | RETURN 的 `rootCauseNodeVisitId` 和 `relatedSubmissionIds` 引用校验 | 需要在事务内校验引用存在且属于同一 Instance |
| HR4 | Transition Schema 可能为 NULL（版本级别有 Schema 但 Transition 级别为 NULL） | 按 Transition 级别优先。NULL 表示"不需要校验"或"任意 JSON 接受" |

---

## 23. 最终回报

### 核心结论

| # | 问题 | 答案 |
|---|------|------|
| 1 | 调查路径 | `/Users/yanfenma/workspace/project/svc-workflow` |
| 2 | Base SHA | `4a06c66c25782e184a689e01c00c87b8b4f0db95` |
| 3 | 当前 HEAD | `4a06c66c25782e184a689e01c00c87b8b4f0db95` |
| 4 | 是否修改代码 | **否**（只读调查） |
| 5 | 推荐的唯一命令名 | `ExecuteWorkflowTransition` |
| 6 | 命令是否强制包含 Submission | **否**（`submission_payload: Option<Value>`） |
| 7 | PR 3C / PR 3D 边界 | 3C = Transition-only，3D = Context Revision + Transition 组合命令 |
| 8 | 正常执行者 | 当前 `NodeVisit.assignee_principal_id` |
| 9 | Transition 选择方式 | 客户端传 `transition_definition_id`（UUID） |
| 10 | ADVANCE 语义 | 使用 `primaryAdvanceTransitionId`，非终态→终态或非终态→非终态 |
| 11 | RETURN 语义 | 非 primary Transition，target orderIndex 更小，创建新 Visit |
| 12 | TERMINATE 语义 | 非 primary，target 为 TERMINAL，需要 Submission（reasonCode + reason） |
| 13 | 目标 NodeVisit 创建 | 服务端预生成 ID，事务内 INSERT，不可变 |
| 14 | 目标 assignee 解析 | 事务内按目标 Node 的 assignee 规则重新解析并快照 |
| 15 | Submission Schema 来源 | `workflow_transition_definitions.submission_schema`（Transition 级别） |
| 16 | 一个 Visit 一个 Submission 保证 | `UNIQUE (source_node_visit_id)` 数据库唯一约束 |
| 17 | Definition Version 状态语义 | PUBLISHED/DEPRECATED 允许，REVOKED 拒绝，DRAFT 内部错误 |
| 18 | 锁顺序 | Receipt → Instance → DefinitionVersion |
| 19 | 与 Revoke 并发 | DefinitionVersion `FOR UPDATE` 行锁保证序列化 |
| 20 | 与 Context Revision 并发 | Instance `FOR UPDATE` 行锁保证状态版本 |
| 21 | 状态版本和 Event | `old+1 = new = eventSequence`，一个 Event |
| 22 | Event 字段矩阵 | `WORKFLOW_TRANSITION_COMMITTED`，含 source/target visit、context、submission、effect、version |
| 23 | requestHash 草案 | `{"command_schema_version":"v1","command_type":"EXECUTE_WORKFLOW_TRANSITION","route_parameters":{},"request_body":{...}}` |
| 24 | 成功响应草案 | `{"workflowInstanceId","workflowStateVersion","currentContextRevisionId","sourceNodeVisitId","currentNodeVisitId","submissionId","eventSequence"}` |
| 25 | 是否需要 Migration | **否** |
| 26 | 若需要，最小 Migration | N/A |
| 27 | 原子失败点 | Submission INSERT, NodeVisit INSERT, Instance UPDATE, Event INSERT, Receipt Completion （全部导致事务回滚） |
| 28 | 测试矩阵数量 | 约 40-45 项 |
| 29 | 结构方案 | `tests/17_workflow_runtime/transition/` 新增 8 个子文件 |
| 30 | Medium 债务处理 | M1 在 PR 3C 自始使用正确错误码；M3 需独立修复 PR；M4 不加剧；M5 包含 Golden test |
| 31 | Blocker | **无** |
| 32 | High-risk gaps | HR1-HR4（均有缓解方案） |
| 33 | 报告路径 | `./PR3C_SUBMISSION_TRANSITION_INVESTIGATION_REPORT.md` |
| 34 | `git status --short` | (clean) |
| 35 | 明确状态 | **`SVC_WORKFLOW_PR3C_INVESTIGATION_COMPLETE`** |

---

### 单一推荐实施切片

PR 3C 应实现：

```
命令: ExecuteWorkflowTransition
Event: WORKFLOW_TRANSITION_COMMITTED
文件:
  src/domain/workflow_instance/commands.rs
    → 新增 ExecuteWorkflowTransitionCommand struct
  src/domain/workflow_instance/errors.rs
    → 新增 ExecuteWorkflowTransitionError enum
  src/domain/workflow_instance/events.rs
    → 新增 TRANSITION_COMMITTED_EVENT_TYPE
    → 新增 TransitionCommittedEventData struct
    → 新增 COMMAND_TYPE_EXECUTE_TRANSITION
  src/application/workflow_instance/mod.rs
    → 新增 execute_transition module
  src/application/workflow_instance/idempotency.rs
    → 新增 compute_transition_request_hash
  src/store/postgres/workflow_instance_repository/mod.rs
    → 新增 transition_transaction module
  src/store/postgres/workflow_instance_repository/transition_transaction.rs (NEW)
    → 完整的原子事务逻辑
  src/store/postgres/workflow_instance_repository/transition_validation.rs (NEW)
    → Transition-specific validators

测试:
  tests/17_workflow_runtime/transition/ (NEW directory)
    success.rs
    authorization.rs
    definition_gates.rs
    submission_validation.rs
    idempotency.rs
    concurrency.rs
    atomicity.rs
    request_hash_contract.rs
  tests/17_workflow_runtime.rs
    → 添加 transition 模块引用和种子函数
```

不包含：
- HTTP/API 路由
- Context Revision（PR 3D）
- 管理员紧急修复（独立 PR）
- Reassign 或 Handoff（v0.3.1 不提供）
