# Legacy ADC / llm-todo Read-Only Investigation Report

```text
Status: INVESTIGATION_COMPLETE
Version: v0.1
Architecture: SVC_WORKFLOW_ARCHITECTURE_V0_3_1 (ARCHITECTURE_FROZEN)
Target Service: svc-workflow (Rust + PostgreSQL)
```

> 本报告基于对现有 ADC (Agent Delivery Center) 与 llm-todo 仓库的只读调查完成。
> 本报告不修改现有系统，不实现 `svc-workflow`，不创建数据库迁移。

---

## 0. Investigated Repositories

| Repository | Absolute Path | Branch | HEAD SHA |
|---|---|---|---|
| agent-dev-center (ADC) | `/Users/yanfenma/workspace/project/agent-dev-center` | `temp-oci-revision-ff` | `bcae54fdaaa8fe7aad87b1dfe708a9284ade8d87` |
| llm-todo | `/Users/yanfenma/workspace/project/llm-todo` | `main` | `7cc746240ba15161a5350bbe4c6d8fb88f41f5c6` |
| auth-service | `/Users/yanfenma/workspace/project/auth-service` | (shared DB with ADC) | `---` |
| svc-workflow | `/Users/yanfenma/workspace/project/svc-workflow` | `main` | `080ca8ddb7c2286cdc4595219db040ae085f7f57` (frozen tag: `svc-workflow-architecture-v0.3.1-frozen`) |

---

## 1. 现有 ADC 数据模型

### 1.1 实体总览

ADC 使用 **PostgreSQL** + **Prisma ORM**，Schema 位于：
`/Users/yanfenma/workspace/project/agent-dev-center/backend/prisma/schema.prisma`（998 行）

迁移目录：
`/Users/yanfenma/workspace/project/agent-dev-center/backend/prisma/migrations/`（36 个迁移）

#### BusinessDomain（`business_domains` 表）

**定义文件**: `schema.prisma:84-98`

| 字段 | 类型 | 备注 |
|---|---|---|
| `key` | `String @id` | 稳定业务键（如 `"engineering"`），非 UUID |
| `name` | `String` | |
| `description` | `String` | |
| `isActive` | `Boolean` | 映射 `is_active` |
| `isSystem` | `Boolean` | 映射 `is_system` |
| `createdAt` / `updatedAt` | `DateTime` | |

**外键**: 被 `Requirement.domainKey` 引用（FK），被 `DomainRoleBinding.domainKey` 引用（CASCADE DELETE）

**当前权威状态字段**: 无 owner 字段；Domain Owner 不在此实体

**是否仍被生产使用**: 是，`domain-scope.ts` 与 `domains.ts` 中读取

**对应 svc-workflow 实体**: `Domain`

**差异**: svc-workflow 的 Domain 使用 `domainId` (UUID) 做主键；ADC 使用 `key` (String) 做主键。svc-workflow 要求 `enabled` 字段（ADC 对应 `isActive`）。

#### DomainRoleBinding（`domain_role_bindings` 表）

**定义文件**: `schema.prisma:170-196`

| 字段 | 类型 | 备注 |
|---|---|---|
| `id` | `String @id @default(uuid())` | |
| `role` | `String` | 平台角色字符串，如 `"adc:developer"` |
| `domainKey` | `String` | FK → `BusinessDomain.key` |
| `isDomainAdmin` | `Boolean @default(false)` | **近似 DOMAIN_OWNER 的标记** |
| `isGlobal` | `Boolean @default(false)` | 跨域访问 |
| `createdAt` / `updatedAt` | `DateTime` | |

**唯一约束**: `@@unique([role, domainKey])`

**关键发现**: ADC 的 DomainRoleBinding 是 **role-based**（一个 binding 绑定一个角色+域），而 svc-workflow 的 DomainRoleBinding 是 **principal-based**（一个 binding 绑定一个 principal + 角色）。ADC 中没有 `principalId` 字段。

**`isDomainAdmin` 实现分析** (`domain-scope.ts:56-57`):
```typescript
if (b.isDomainAdmin) {
  adminDomains.add(b.domainKey);
}
```
当 `isDomainAdmin=true` 时，该 binding 授予该域的管理员权限。但**非唯一约束**——同一域可以有多个 `isDomainAdmin=true` 的 binding（不同 role）。

**当前权威状态字段**: `isDomainAdmin` 标记近似但不等同于 DOMAIN_OWNER。ADC 中**没有强制执行"一个域一个所有者"**。

**是否仍被生产使用**: 是，`domain-scope.ts`（权限解析）、`domains.ts`（域 API）、`core-patch.ts`（管理操作）中使用

**对应 svc-workflow 实体**: `DomainRoleBinding`

**差异**:
- ADC 使用 `role` (String) 作为主键组合的一部分，svc-workflow 使用 `principalId`
- ADC 的 `isDomainAdmin` 标记是布尔值，svc-workflow 使用 `roleKey = DOMAIN_OWNER`
- ADC 不保证"一个域一个 Owner"

#### WorkflowTemplate（`workflow_templates` 表）

**定义文件**: `schema.prisma:583-596`

| 字段 | 类型 | 备注 |
|---|---|---|
| `id` | `String @id @default(uuid())` | |
| `name` | `String @unique` | 编程名称，如 `"backend-dev"` |
| `displayName` | `String` | |
| `description` | `String` | |
| `steps` | `Json` | JSON 数组或 `{ steps, roleUserMap }` 对象 |
| `isActive` | `Boolean @default(true)` | |

**外键**: `Requirement.workflowId` → `WorkflowTemplate.id`

**当前权威状态字段**: `steps` 中定义了所有步骤信息。但**模板本身不是不可变的**——`ensureWorkflowTemplates()` 在每次启动时执行 `upsert`，可以覆盖已有模板。

**是否仍被生产使用**: 是，是工作流系统的核心定义

**对应 svc-workflow 实体**: `WorkflowDefinition` + `WorkflowDefinitionVersion`

**无法直接映射的差异**:
- ADC 模板是**可变的**（启动 upsert），svc-workflow 要求 Definition Version 发布后不可变
- ADC 没有独立的 Version 概念
- ADC 的 `steps` 是松散 JSON，svc-workflow 要求结构化 NodeDefinition + TransitionDefinition
- ADC 没有 `contextSchema` 或 `submissionSchema`

#### Requirement（`requirements` 表）

**定义文件**: `schema.prisma:100-168`

核心字段：

| 字段 | 类型 | 备注 |
|---|---|---|
| `id` | `String @id @default(uuid())` | |
| `title` / `description` | `String` | |
| `priority` | `RequirementPriority` | P0-P3 |
| `status` | `String @default("pending")` | **已废弃**，遗留兼容 |
| `currentStep` | `String?` | **当前工作流步骤名**，替代 `status` |
| `workflowId` | `String? @db.Uuid` | FK → WorkflowTemplate.id |
| `workflowSnapshot` | `Json?` | **冻结的工作流步骤副本** |
| `stateVersion` | `Int @default(0)` | **乐观锁版本号** |
| `assigneeId` | `String? @db.Uuid` | FK → User.id |
| `assignee` | `String?` | 遗留文字字段 |
| `requesterId` | `String? @db.Uuid` | FK → User.id |
| `requester` | `String?` | |
| `domainKey` | `String?` | FK → BusinessDomain.key |
| `projectId` | `String? @db.Uuid` | FK → Project.id |
| `gitHash` / `deployVersion` / `branch` / `repoPath` | `String?` | 开发元数据 |
| `tags` | `String[]` | |

**关键外键**: `workflowId`, `assigneeId`, `requesterId`, `domainKey`, `projectId`

**当前权威状态字段**: `currentStep` 是当前工作流位置的真实状态（取代已废弃的 `status`）。
`workflowSnapshot` 是步骤的不可变副本（在 assign 时深拷贝）。
`stateVersion` 用于乐观锁 CAS。

**是否仍被生产使用**: 是，是核心实体

**对应 svc-workflow 实体**: **无直接对应**。Requirement 是上层业务实体，svc-workflow 不拥有 Requirement。但 `currentStep`、`assigneeId`、`workflowSnapshot` 中的信息对应 `WorkflowInstance` + `NodeVisit`。

#### WorkflowTransition（`workflow_transitions` 表）

**定义文件**: `schema.prisma:598-616`

| 字段 | 类型 | 备注 |
|---|---|---|
| `id` | `String @id @default(uuid())` | |
| `requirementId` | `String` | FK → Requirement |
| `fromStep` | `String` | |
| `toStep` | `String` | |
| `action` | `String` | `"advance"`, `"reject"`, `"assign-workflow"` 等 |
| `actorId` | `String?` | |
| `actorName` | `String` | |
| `actorRole` | `String` | |
| `comment` | `String?` | |
| `metadata` | `Json?` | |
| `createdAt` | `DateTime` | |

**对应 svc-workflow 实体**: 部分对应 `WorkflowEvent` + `Submission`

**差异**: ADC 只有 transition 记录，没有 `Submission`、`ContextRevision` 概念。

#### RequirementReport（`requirement_reports` 表）

**定义文件**: `schema.prisma` (RequirementReport model)

| 字段 | 类型 | 备注 |
|---|---|---|
| `reportType` | `ReportType` | `DEV_SELF_CHECK`, `TEST_REPORT`, `CTO_REVIEW` 等 |
| `content` | `Json` | 报告内容 |
| `status` | `ReportStatus` | `pending`, `approved`, `rejected`, `changes_requested` |
| `workflowStep` | `String` | 报告所属工作流步骤 |
| `submittedById` | `String` | FK → User.id |

**唯一约束**: `@@unique([requirementId, reportType, workflowStep])`——每个步骤每种报告最多一个

**对应 svc-workflow 实体**: 部分对应 `Submission`

**差异**: RequirementReport 有审核状态（pending→approved/rejected），svc-workflow 的 Submission 创建后不可修改，没有审核状态。ADC 的 report 系统实际上是两阶段（提交→审核），而 svc-workflow 只有一次提交。

#### RequirementEventLedger（`requirement_event_ledger` 表）

**定义文件**: `schema.prisma:931-961`

| 字段 | 类型 | 备注 |
|---|---|---|
| `eventType` | `String` | `WORKFLOW_ADVANCED`, `WORKFLOW_REJECTED` 等 |
| `actorId` | `String?` | |
| `fromStep` / `toStep` | `String?` | |
| `fromAssigneeId` / `toAssigneeId` | `String?` | |
| `reasonCode` / `reasonText` | `String?` | |
| `evidence` | `Json?` | |
| `metadata` | `Json?` | |
| `stateVersionBefore` / `stateVersionAfter` | `Int?` | |
| `correlationId` | `String?` | |
| `relatedTransitionId` / `relatedReportId` / `relatedAuditLogId` | `String?` | |

**对应 svc-workflow 实体**: `WorkflowEvent`（最接近的对应）

**差异**: ADC 的 EventLedger 是追加型（不保证严格递增的事件序列），svc-workflow 要求 `eventSequence` 严格递增且 `eventSequence = newWorkflowStateVersion`。

#### 其他实体

| 实体 | svc-workflow 对应 | 备注 |
|---|---|---|
| `RequirementAuditLog` | 部分对应 `SecurityAudit` | 简化版审计，不含事件序列 |
| `RequirementComment` | 无对应 | svc-workflow 不支持评论（上层业务） |
| `RequirementRevision` | 部分对应 `WorkflowContextRevision` | 保存 title/description 快照，非 JSON Schema 结构化 |
| `Task` | 无对应 | 上层业务子任务 |
| `ExecutionLease` | 无对应 | Agent 执行租约，超出 v0.3.1 边界 |
| `FeedbackEvent` | 部分对应 `WorkflowEvent` | 仅记录 reject/反馈 |

---

### 1.2 Domain Owner 当前到底存在哪里

**当前结论**: Domain Owner **不存在于专用字段**，而是通过 `DomainRoleBinding.isDomainAdmin` 间接表示。但该标记**非唯一**——同一域可以有多个 `isDomainAdmin=true` 的 binding。

证据：
- `BusinessDomain` 模型 (`schema.prisma:84-98`) 没有 `ownerPrincipalId` 或类似字段
- `DomainRoleBinding` (`schema.prisma:170-196`) 有 `isDomainAdmin` 布尔标记
- `domain-scope.ts:56-57` 中逻辑：一个用户的任一 role binding 带有 `isDomainAdmin=true` 即可获得管理员权限
- `domains.ts:60-67` 中域 API 使用 `adminSet` 判断用户角色

**迁移含义**: 需要为每个 Domain 选择一个 `isDomainAdmin=true` 的用户作为 DOMAIN_OWNER。无法确定时使用 `DOMAIN_OWNER_FALLBACK`。

### 1.3 DomainRoleBinding 的真实角色和约束

当前 binding 使用平台角色字符串（如 `adc:developer`），约束为 `@@unique([role, domainKey])`。svc-workflow 需要迁移到 `(domainId, principalId, DOMAIN_OWNER)` 模型。

**现有约束无法保证"一个域一个 Owner"**。

### 1.4 Requirement.domainKey 的使用方式

`domainKey` (`schema.prisma:100-168` 中可选字段) 将 Requirement 关联到 BusinessDomain。在 `requirement-create-service.ts:37` 是**必填参数**，在 `domain-scope.ts` 中用于权限过滤。

### 1.5 Workflow Template / Snapshot 是否真正不可变

**模板本身不是不可变的**：
- `ensureWorkflowTemplates()` (`workflow-templates.ts:398-427`) 在**每次服务启动时**执行 `upsert`
- 启动代码 `server.ts:15` 调用 `await ensureWorkflowTemplates()`
- 这意味着代码升级可以更改模板定义

**Snapshot 是近似不可变的**：
- `assignWorkflowAtomic()` (`workflow-assign-service.ts:90`) 执行 `JSON.parse(JSON.stringify(template.steps))` 深拷贝
- `getWorkflowRawJson()` (`workflow-helpers.ts:157-173`) 优先使用 `workflowSnapshot`
- **但** `workflowSnapshot` 在 `ensureWorkflowTemplates` 中并没有被保护——只有新分配的需求才获得新 snapshot

---

## 2. 所有工作流状态写路径

### 写路径表

| 写路径 | 文件与行号 | 修改字段 | 是否事务化 | 是否写审计 | 是否标准入口 | 迁移风险 |
|---|---|---|---|---|---|---|
| **workflow-advance** | `backend/src/routes/requirements/workflow-advance.ts:308-358` | `currentStep`, `assigneeId`, `stateVersion` | **是** (CAS + tx) | **是** (EventLedger + WorkflowTransition) | **是** | 低 |
| **workflow-reject** | `backend/src/routes/requirements/workflow-reject.ts:233-327` | `currentStep`, `assigneeId`, `stateVersion` | **是** (CAS + tx) | **是** (EventLedger + WorkflowTransition) | **是** | 低 |
| **workflow-assign** | `backend/src/routes/requirements/workflow-assign.ts:53-58` | `currentStep`, `workflowId`, `workflowSnapshot`, `assigneeId`, `stateVersion` | **是** (单查询 + update) | **是** (WorkflowTransition) | **是** | 中 |
| **workflow-assign-service (atomic)** | `backend/src/routes/requirements/workflow-assign-service.ts:128-143` | `currentStep`, `workflowId`, `workflowSnapshot`, `assigneeId`, `stateVersion` | **是** (Prisma interactive tx + CAS) | **否** (无 EventLedger) | **是** | 中 |
| **requirement-create-service** | `backend/src/services/requirement-create-service.ts:229-233` | `currentStep`, `workflowId`, `workflowSnapshot`, `assigneeId` | **是** (tx) | **否** (创建时写 EventLedger 在别处) | **是** | 中 |
| **reports-approval (auto-advance)** | `backend/src/routes/reports-approval.ts:198` | `currentStep` | **是** (CAS + tx) | **是** (EventLedger) | **否** (旁路) | **高** |
| **core-crud PATCH** | `backend/src/routes/requirements/core-patch.ts` | 可能直接更新 `currentStep`, `assigneeId`, `stateVersion` | **是** (单个 Prisma update) | **是** (EventLedger) | **否** (通用更新) | **高** |
| **admin cleanup scripts** | `backend/src/scripts/*` | 各种字段 | 可变 | 可变 | **否** (脚本) | **高** |
| **agent-todo-efficiency-cutover** | `backend/src/scripts/agent-todo-efficiency-cutover/service.ts:205-432` | `workflowSnapshot`, `currentStep` | 部分 | 部分 | **否** (迁移脚本) | **高** |
| **ensureWorkflowTemplates (startup)** | `backend/src/lib/workflow-templates.ts:398-427` | `workflow_templates` 表中的 `steps` | **否** (每个模板独立 upsert) | **否** | **否** (启动初始化) | **高** |
| **db seed** | `backend/prisma/migrations/*` 和 `seed.ts` | 表初始数据 | **是** (迁移) | **否** | **否** | 中 |

### 必须特别标出的写路径

#### ⚠️ 绕过标准 Transition Service 的写路径

1. **reports-approval.ts:198** — 报告审核通过后自动推进 `currentStep`，绕过了 workflow-advance 的完整校验链
   ```typescript
   // reports-approval.ts ~198
   await tx.requirement.update({
     where: { id: requirementId },
     data: { currentStep: actualTarget, stateVersion: { increment: 1 } }
   });
   ```

2. **core-patch.ts** — 通用 PATCH 路由可以更新 `currentStep`
   ```typescript
   // core-patch.ts ~43-47
   const isDomainAdmin = actor.crossDomainAccess || ...;
   // 如果是 Domain Admin，可以直接 PATCH currentStep
   ```

3. **agent-todo-efficiency-cutover/service.ts** — 迁移脚本直接更新 `workflowSnapshot` 和 `currentStep`
   ```typescript
   // service.ts ~205-215
   data: { workflowSnapshot: ... as any, currentStep: ... }
   ```

#### ⚠️ 直接修改 `currentStep` 或状态的路径

- 所有写路径都直接修改 `currentStep`，没有统一的抽象层
- CAS 机制 (`casUpdateRequirement` in `workflow-cas-helper.ts`) 提供了乐观锁，但**不是所有写路径都使用 CAS**

#### ⚠️ 会在启动或 Seed 时覆盖模板的路径

`ensureWorkflowTemplates()` (`workflow-templates.ts:398-427`) 在**每次服务启动时执行 `upsert`**。这意味着：
- 如果代码中的模板定义被更新，生产数据库中的已有模板会被修改
- 已有 `workflowSnapshot` 的需求不受影响（snapshot 在 assign 时冻结）
- 但尚未 assign 模板的需求将使用新版本

#### ⚠️ 无审计或非事务写入路径

- **ensureWorkflowTemplates** 没有审计日志
- **admin/seed scripts** 审计覆盖不一致
- **workflow-assign-service** 使用 CAS 事务但**不写 EventLedger**

---

## 3. 模板和图结构映射

### 3.1 当前模板如何表达图结构

ADC 模板 (`workflow-templates.ts`) 使用**线性数组**定义步骤：
```typescript
steps: [
  { name: 'draft', displayName: '草稿', role: 'requester', requiredReports: [], autoAdvance: false },
  { name: 'pm_review', displayName: 'PM评审', role: 'pm', requiredReports: ['DEV_SELF_CHECK'], autoAdvance: false },
  // ...更多步骤
  { name: 'done', displayName: '已完成', role: 'cto', requiredReports: [], autoAdvance: false },
]
```

**节点顺序**: 数组索引顺序决定

**当前步骤**: `Requirement.currentStep` 字符串值匹配 `step.name`

**负责人角色**: 每个步骤的 `role` 字段，在实例化时通过 `roleUserMap` 或 `assignee-resolver.ts` 解析为具体 User ID

**下一节点**: 
- `getNextStep()` (`workflow-helpers.ts:265-278`) 使用显式 `step.next` 或默认数组顺序
- `autoAdvance: true` 的步骤会自动跳过

**驳回目标**:
- `workflow-reject.ts` 使用显式 `step.rejectTo` 配置，或默认回退逻辑
- 大型保护步骤（如 security_review）强制回退到 `dev_self_check`

**终态**: 线性数组的最后一个步骤（通常名为 `"done"`）

**提交格式**: 通过 `requiredReports` 数组指定需要的 Report 类型，而非 JSON Schema

**自动重新分配**: `workflow-advance.ts:278-305` 在推进时自动解析下一步骤的负责人

### 3.2 能否转换为冻结模型

| svc-workflow 概念 | ADC 对应 | 机械转换 | 需要人工规则 | 无法映射 |
|---|---|---|---|---|
| `WorkflowDefinition` | `WorkflowTemplate` 元数据 | ✓ `name → definitionKey` | ✓ `displayName`, `description` | - |
| `WorkflowDefinitionVersion` | `WorkflowTemplate.steps` 内容 | - | ✓ 需要将数组转为版本化实体 | - |
| `NodeDefinition` | 每个 `step` | ✓ `name → nodeKey` | ✓ `displayName`, `role → assigneeRef` | `instructions` 不存在 |
| `primaryAdvanceTransitionId` | `getNextStep()` 逻辑 | - | ✓ 需要从数组顺序生成显式 Transition ID | - |
| `RETURN` Transition | `rejectTo` 或默认回退逻辑 | - | ✓ 需要枚举所有可能的 RETURN 路径 | - |
| `TERMINATE` Transition | 无显式概念 | - | ✓ 需要从"终止"操作生成 | - |
| `assigneeRef` | `role` 字段 + `roleUserMap` | ✓ `role=requester` → `WORKFLOW_CREATOR` | ✓ `role=cto/admin` → `DOMAIN_OWNER` 或 `FIXED_PRINCIPAL` | 部分角色需要映射 |
| `contextSchema` | 无 | - | - | **不存在** |
| `submissionSchema` | `requiredReports` | - | ✓ 需要根据 ReportType 定义 JSON Schema | - |

### 3.3 当前模板中是否存在并行、条件分支或动态节点

| 特性 | 是否存在 | 证据 |
|---|---|---|
| 并行节点 | **否** | 所有模板为线性数组，`currentStep` 是单值 |
| 条件分支 | **部分** | `autoAdvance` 和 skip 逻辑（如 security_review 对非 SECURITY 类型） |
| 动态节点 | **否** | 所有步骤在模板定义时已确定 |
| 循环/回退 | **是** | `workflow-reject.ts` 支持 RETURN 到任意前序步骤 |

### 3.4 哪些现有能力超出 v0.3.1 边界

- **WIP 限制** (`wipLimit`, `getStepWipCount()`) — v0.3.1 不支持
- **自动跳过** (`autoAdvance`) — v0.3.1 不支持
- **测试环境锁** (`TestEnvLock` 单例锁) — 超出工作流内核边界
- **ExecutionLease** — 超出 v0.3.1 边界
- **具体 Report 审核流程** (pending→approved/rejected) — v0.3.1 只有一次提交
- **RequirementComment** — 上层业务功能

---

## 4. Requirement 与 Workflow Context 边界

### 4.1 字段归属分析

| Requirement 字段 | 上层业务数据 | 应复制到 Context Revision | 备注 |
|---|---|---|---|
| `title` | ✓ | ✓ | 工作流需要知道"做什么" |
| `description` | ✓ | ✓ | 工作内容描述 |
| `acceptance criteria` | (不存在独立字段，在 description 中) | ✓ | 需要提取为独立字段 |
| `priority` (P0-P3) | ✓ | ✗ | 业务优先级，不影响工作流流转 |
| `domainKey` | ✓ | ✓ (作为 metadata) | 标识归属域 |
| `assigneeId` | **工作流数据** | ✓ | 当前负责人（将被 NodeVisit 取代） |
| `projectId` | ✓ | ✗ | 业务项目归属 |
| `repository` / `repoPath` / `branch` | ✓ | ✓ (作为 metadata) | 开发流程需要 |
| `gitHash` / `deployVersion` | ✓ | ✓ (作为 Submission payload) | 实际交付证据 |
| `status` (已废弃) | - | - | 已被 `currentStep` 取代 |
| `workflowId` / `workflowSnapshot` / `currentStep` | **工作流数据** | - | 将被 svc-workflow 完全取代 |
| `tags` | ✓ | ✗ | 业务分类 |
| `type` (FEATURE/BUGFIX) | ✓ | ✓ (作为 metadata) | 影响流程行为 |
| `dueDate` | ✓ | ✗ | 业务截止时间 |

### 4.2 开发流程 contextSchema 候选

基于 `Requirement` 当前字段，推荐首个开发流程的 `contextSchema`：

```json
{
  "type": "object",
  "required": ["title", "description", "domainKey"],
  "properties": {
    "title": { "type": "string", "minLength": 1, "maxLength": 200 },
    "description": { "type": "string", "minLength": 1, "maxLength": 10000 },
    "acceptanceCriteria": {
      "type": "array",
      "items": { "type": "string", "minLength": 1 }
    },
    "domainKey": { "type": "string", "minLength": 1 },
    "type": { "type": "string", "enum": ["FEATURE", "BUGFIX", "INFRA", "SECURITY"] },
    "repoPath": { "type": "string" },
    "branch": { "type": "string" },
    "gitHash": { "type": "string" }
  }
}
```

**说明**: 不包含 `priority`、`projectId`、`tags`、`dueDate` 等业务字段，这些继续由上层平台持有。

---

## 5. Submission 与旧 Report / Audit 的映射

### 5.1 迁移分类

#### SAFE_TO_IMPORT_AS_SUBMISSION

| 旧数据 | 条件 | 说明 |
|---|---|---|
| `RequirementReport` (status = approved) | 报告可以明确关联到某个步骤的 | 提交即审核通过的最终版本 |
| `DEV_SELF_CHECK` 报告 | 始终可映射 | 自检报告 ≈ Submission payload |
| `DEPLOY_CONFIRM` 报告 | 始终可映射 | 部署确认 ≈ 最终交付 |
| `MERGE_REPORT` 报告 | 始终可映射 | 合并报告 ≈ 代码交付证据 |

#### IMPORT_AS_LEGACY_REFERENCE

| 旧数据 | 说明 |
|---|---|
| `RequirementReport` (status = rejected / changes_requested) | 被驳回的版本，不是 committed Submission |
| `RequirementAuditLog` | 简化的审计记录，保留为 Legacy |
| `WorkflowTransition` | 过渡记录，是 Event 的前身 |
| `RequirementComment` | 评论，无对应 Submission 概念 |

#### KEEP_ONLY_IN_LEGACY

| 旧数据 | 原因 |
|---|---|
| `Requirement` 表本身 | 上层业务数据，svc-workflow 不拥有 |
| `ExecutionLease` | 超出 v0.3.1 边界 |
| `FeedbackEvent` | 过渡设计，已被 EventLedger 取代 |
| `TestEnvLock` | 出工作流内核边界 |
| `Task` (ADC) | 上层业务子任务 |
| `RequirementRevision` | 被 ContextRevision 替代 |

#### UNMAPPABLE

| 旧数据 | 原因 |
|---|---|
| 无法确定属于哪个 NodeVisit 的旧 Report | 无 `workflowStep` 或 `workflowStep` 无法对应当前定义 |
| 启动 upsert 覆盖的旧模板定义 | 版本丢失 |
| Admin 手动修复的记录 | 可能无对应 NodeVisit |

### 5.2 冻结规则遵守

无法确定属于哪个 NodeVisit 的旧报告：
- **不得伪造为 committed Submission**
- 作为 Legacy 数据保留在原表
- 导入 Event 记录 `importedNodeId` 和 `creatorResolution`

---

## 6. Principal 与 Agent 身份映射

### 6.1 当前身份来源

| 身份来源 | 字段 | 类型 | 说明 |
|---|---|---|---|
| auth-service JWT `sub` | `User.id` | UUID | **主身份标识** |
| auth-service `User.agentId` | `User.agentId` | String (unique) | Agent 次要标识 |
| ADC JWT `sub` | `User.id` | UUID | 与 auth-service 同表 |
| LP Token `agentId` | 外部传入 | String | Agent Token 的 sub |

### 6.2 Human / Agent / Service 统一 ID

**当前没有统一的 Principal ID**。auth-service JWT 使用 `User.id` (UUID) 作为 `sub`，但存在以下异源 ID：

- `User.id` (UUID) — auth-service 数据库主键
- `User.agentId` — 外部 Agent 标识（如 `"agent-frontend-developer-1"`）
- `Requirement.assigneeId` — 引用 `User.id` (UUID)
- `Requirement.requesterId` — 引用 `User.id` (UUID)
- `WorkflowTransition.actorId` — 引用 `User.id` (UUID)
- `RequirementEventLedger.actorId` — 引用 `User.id` (UUID)

### 6.3 映射方案

`principalId` = `User.id` (UUID) from auth-service/ADC shared users table.

| Legacy Identity | 映射方式 | 稳定性 |
|---|---|---|
| `Requirement.assigneeId` (UUID) | 直接映射为 `principalId` | **稳定** |
| `Requirement.requesterId` (UUID) | 直接映射为 `principalId` | **稳定** |
| `User.agentId` (String) | 通过 `User.agentId → User.id` 解析 | **稳定**（已建立唯一映射） |
| External LP Token agentId | 需要先确保在 `User` 表中有记录 | 条件稳定 |

### 6.4 Legacy Creator 能否稳定映射

**可以稳定映射**，因为：
- `Requirement.requesterId` 是 `User.id` 的 FK，所有正常创建的需求一定有有效的 `requesterId`（UUID）
- `requirement-create-service.ts:211` 强制 `requesterId: actor.id`
- 只针对**历史无 requesterId 或已删除用户**的记录需要 `DOMAIN_OWNER_FALLBACK`

### 6.5 需要 DOMAIN_OWNER_FALLBACK 的记录

- `assignee = "system"` 或 `assigneeId = null` 的历史需求
- 已删除用户的旧需求
- 通过 admin PATCH 直接设置的记录（无 actor 链）
- 迁移脚本 `agent-todo-efficiency-cutover` 创建的记录（部分无标准 actor）

### 6.6 Migration Service Principal

应创建一个专用的 auth-service User：
- `name = "svc-workflow-migration"`
- `email = "migration@svc-workflow.local"`
- `role = "admin"`（或新创建 `service` 角色）
- 固定 `User.id`（UUID）作为导入命令的 `principalId`

### 6.7 固定 Agent 是否可能处于 disabled 或不存在状态

**是**。ADC 中的固定 Agent 通过 `MarketplaceAgent` 管理，有 `status` 字段（`active`, `inactive`, `maintenance`）。auth-service 中的 `User` 没有 `enabled`/`disabled` 标记但有 `PasswordPolicy` 相关的密码管理。

---

## 7. llm-todo 边界

### 7.1 llm-todo 当前业务字段

llm-todo 使用 **SQLite**（`better-sqlite3`），无 Prisma。数据库在 `src/db.ts:56-96` 中定义。

主表 `todos` 关键字段：

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | INTEGER PK | |
| `title` / `description` | TEXT | |
| `status` | TEXT | 8 种状态 |
| `priority` | TEXT | high/medium/low |
| `type` | TEXT | personal/agent/review/discuss |
| `assignee_agent_id` | TEXT | 权威代理身份 |
| `assignment_state` | TEXT | unassigned/assigned/self_owned/not_applicable |
| `area` | TEXT | dev/ops/life/health 等 |
| `horizon` | TEXT | day/week/month/quarter/year |
| `target_date` | TEXT | 计划日期 |
| `depends_on` | TEXT (JSON) | 依赖任务 |
| `parent_id` | INTEGER? | 父任务 |

### 7.2 哪些字段继续留在 llm-todo

所有**业务字段**继续留在 llm-todo：
- `title`, `description` — 任务内容
- `priority` — 业务优先级
- `type`, `area`, `horizon` — 分类
- `target_date`, `due_date` — 日期
- `tags` — 标签
- `depends_on`, `parent_id` — 任务关系图
- `assignee_agent_id`, `assignment_state` — 负责人

### 7.3 哪些状态流转由 svc-workflow 承担

llm-todo 的状态机**不适合由 svc-workflow 替代**。原因：
1. svc-workflow 是**串行受治理工作流**，llm-todo 是**灵活任务管理**
2. llm-todo 的状态是**平面 8 状态机**（pending→active→in_progress→review→done 等），vs svc-workflow 的有向图
3. llm-todo 的 `dependency-guard.ts` 实现自动阻塞/解锁，超出 v0.3.1 边界

**唯一可能由 svc-workflow 承担的**：`review` 状态下的审核流程（如果未来标准化），但当前 llm-todo 的审核子系统 (`task_review_cycles`) 已有自己的独立状态机。

### 7.4 llm-todo 如何保存 "workflowInstanceId"

**当前不存在 `workflowInstanceId` 字段**。未来需要：
- 在 llm-todo `todos` 表新增 `workflow_instance_id TEXT` 列
- 或使用 `metadata` JSON 字段存储关联

### 7.5 现有 Todo 状态写路径

| 写路径 | 文件 | 事务化 | 审计 |
|---|---|---|---|
| `PUT /:id` 状态更新 | `status.ts:36` | **是**（SQLite `BEGIN IMMEDIATE`） | **是**（audit_logs） |
| 依赖自动阻塞/解锁 | `dependency-guard.ts:122-162` | **是** | **是** |
| 调度/暂停自动转换 | `status.ts:98-121` | **是** | **是** |
| POST /:id/promote | `crud.ts:110` | **是** | **否** |
| DELETE /:id | `crud.ts:198` | **是** | **是** |
| 子任务完成传播 | `crud.ts` | 部分 | **否** |

### 7.6 适合做试点的最小 Todo 模板

**不适合**。llm-todo 的灵活状态机与 svc-workflow 的严格有向图不匹配。

如果必须在 llm-todo 领域试点，最小方案：

```
步骤: draft → review → done
负责人: 创建者 → 审核者 → 无
Transition: ADVANCE (draft→review), ADVANCE (review→done), RETURN (review→draft)
```

但这将要求 llm-todo 接入 svc-workflow API，改变现有状态机——增加复杂度而非简化。

### 7.7 llm-todo 与 ADC 是否存在直接依赖

**不存在直接数据库或代码依赖**。llm-todo 只通过 JWT 认证与 ADC 交互：
- `sso-auth.ts:168-186` 验证 ADC 签发的 JWT
- `agent-sso.ts:35-51` 使用 ADC JWT 进行 SSO
- 没有共享数据库（llm-todo 用 SQLite，ADC 用 PostgreSQL）
- 没有代码库依赖

---

## 8. Shadow Relay 与 Cutover 接入点

### 8.1 推荐插入点

**标准推荐**: `casUpdateRequirement()` (`workflow-cas-helper.ts:25-42`)

```typescript
// 在成功 CAS 更新后、事务提交前插入 Relay 写入
export async function casUpdateRequirement(...) {
  const result = await tx.requirement.updateMany({ where: { id, stateVersion: expectedStateVersion }, data });
  if (result.count === 0) throw new HttpError(409, '版本冲突');
  
  // ← 在这里插入：await tx.legacyWorkflowRelay.create({ ... })
  
  return tx.requirement.findUnique({ where: { id }, select: REQUIREMENT_TRANSITION_SELECT });
}
```

**备选**: `workflow-advance.ts:308-358` 中事务的最后一步（创建 EventLedger 后）

### 8.2 当前事务边界

- `workflow-advance.ts` 使用 Prisma interactive transaction (`$transaction`)
- `casUpdateRequirement` + `txCreateTransition` + `recordRequirementEvent` 在同一事务
- `workflow-reject.ts` 使用相同模式
- `workflow-assign-service.ts` 使用独立 interactive transaction

### 8.3 是否存在可复用的 Outbox / Ledger

**否**。当前 system:
- `RequirementEventLedger` 在事务内联写入（`requirement-event-ledger-service.ts:66-102` 接受 `txOrPrisma`）
- 没有 `Transactional Outbox` 表或 Worker
- 没有 `relay` 相关表或代码

### 8.4 Relay 最小字段

```sql
CREATE TABLE legacy_workflow_relay (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  domain_key TEXT NOT NULL,
  requirement_id UUID NOT NULL,
  event_type TEXT NOT NULL,            -- 'advanced' | 'rejected' | 'assigned' | 'abandoned'
  current_step TEXT NOT NULL,          -- 旧系统 currentStep
  assignee_id UUID,                    -- 旧系统 assigneeId
  state_version INTEGER NOT NULL,      -- 旧系统 stateVersion
  relay_payload JSONB,                 -- 完整命令信封
  relay_status TEXT NOT NULL DEFAULT 'pending',  -- 'pending' | 'relayed' | 'failed'
  idempotency_key TEXT NOT NULL,        -- 去重键
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  relayed_at TIMESTAMPTZ,
  retry_count INTEGER DEFAULT 0,
  error_message TEXT,
  
  UNIQUE(idempotency_key)
);
```

### 8.5 去重键

```
legacy:<domainKey>:<requirementId>:<stateVersion>
```

### 8.6 Worker 部署位置候选

1. **ADC 内部的侧车进程** — 读取同一数据库的 Relay 表
2. **独立 Relay Worker 服务** — 拆分为单独部署单元
3. **svc-workflow 启动时的消费者** — 如果 svc-workflow 可访问旧数据库

### 8.7 Cutover 对账数据来源

| 对账字段 | Legacy 来源 | 路径 |
|---|---|---|
| `nodeId` / `currentStep` | `Requirement.currentStep` | `schema.prisma:100-168` |
| `assigneePrincipalId` | `Requirement.assigneeId` | 同一表 |
| Terminal 状态 | `currentStep === 'done'` 或 `status` | 通过步骤名判断 |
| 最后 transitionEffect | `WorkflowTransition` 最新记录 | `workflow_transitions` 表 |
| Context digest | 无对应 — 需要从 `RequirementRevision` 或 `title+description` 计算 | 新创建 |

---

## 9. 当前数据库与 PostgreSQL 差异

### 9.1 各项目数据库现状

| 项目 | 数据库 | ORM | 迁移方式 |
|---|---|---|---|
| agent-dev-center | **PostgreSQL 16** | Prisma | `prisma migrate` |
| auth-service | **PostgreSQL 16** (共享 ADC 数据库) | Prisma (仅 User 模型) | 共享 ADC 迁移 |
| llm-todo | **SQLite** (`better-sqlite3`) | 原生 SQL | `CREATE TABLE IF NOT EXISTS` |
| svc-workflow | **PostgreSQL** (目标) | 无 ORM (Rust 原生) | SQL 迁移 |

### 9.2 新 svc-workflow 使用独立 PostgreSQL 的接入成本

**低**。原因是：
- 当前环境已有 PostgreSQL 16 运行（ADC 使用）
- 开发环境 docker-compose 已包含 PostgreSQL (`docker-compose.dev.yml`)
- svc-workflow 可以在同一集群创建独立 database/schema

### 9.3 本地开发如何运行

ADC 开发环境：
```yaml
# docker-compose.dev.yml
postgres:
  image: postgres:16-alpine
  ports: ["5432:5432"]
  environment:
    POSTGRES_DB: agent_dev_center
    POSTGRES_USER: postgres
    POSTGRES_PASSWORD: postgres
```

svc-workflow 可以：
1. 使用同一 PostgreSQL 实例，新建 `svc_workflow` 数据库
2. 或在同一 docker-compose 中增加第二个 PostgreSQL 实例
3. 推荐：使用 `CREATE DATABASE svc_workflow` 在同一集群

### 9.4 是否已有可复用 PostgreSQL

**是**。ADC 开发环境已有 PostgreSQL 16。

### 9.5 SQLite → PostgreSQL 数据转换问题

llm-todo 使用 SQLite，但 svc-workflow 不需要迁移 llm-todo 的数据——llm-todo 继续使用自己的 SQLite 数据库，通过 API 与 svc-workflow 交互。

### 9.6 是否共用集群但使用独立 database/schema

**推荐**: 同一 PostgreSQL 集群，独立 database `svc_workflow`，schema 名为 `workflow`。理由：
- 降低运维复杂度（一个集群）
- 硬隔离保证（独立 database 防止跨 schema 写）
- 与现有 ADC 数据库 `agent_dev_center` 完全隔离

---

## A. 总体判定

```text
READY_WITH_BLOCKING_MIGRATION_GAPS
```

**理由**: 架构基线和实施契约已经冻结且有充分细节。主要数据模型（Domain、DomainRoleBinding、WorkflowTemplate、Requirement）有清晰的对应关系。但存在**三个需要修复的阻塞性缺口**（见 B 节），需要在正式实施前由对应仓库解决。

---

## B. Blocker / High

### B1. Startup 模板覆盖（HIGH）

| 项目 | 内容 |
|---|---|
| **证据** | `workflow-templates.ts:398-427` 中 `ensureWorkflowTemplates()` 在 `server.ts:15` 每次启动时执行 `upsert`；模板 `steps` 可以随时被代码覆盖 |
| **影响** | 如果 svc-workflow 导入旧模板后 ADC 重启，模板定义可能与导入的 DefinitionVersion 不一致 |
| **为什么阻断** | svc-workflow 的 Definition Version 发布后必须不可变。但 ADC 的模板 upsert 可以在不通知 svc-workflow 的情况下修改模板 |
| **最小解决方案** | 在 ADC 中停止启动时 upsert 模板，改为只做首次初始化或仅添加不修改。或者确保模板的 `id`/`name` 与 svc-workflow `definitionKey` 的映射稳定 |
| **应由哪个仓库解决** | `agent-dev-center` |

### B2. 非标准 currentStep 写路径（HIGH）

| 项目 | 内容 |
|---|---|
| **证据** | `reports-approval.ts:198` 在报告批准时直接 `update` 跳过标准 advance 流程；`core-patch.ts` 允许 Domain Admin 直接 PATCH `currentStep`；`agent-todo-efficiency-cutover/service.ts` 迁移脚本直接写 |
| **影响** | 这些路径不会写入 Relay，导致 Shadow 期状态不一致 |
| **为什么阻断** | 如果 Shadow 依赖标准 CAS advance 路径作为唯一 Relay 写入点，绕过这些路径的更新将丢失 |
| **最小解决方案** | 在 ADC 中将这些旁路路径改为走标准 advance/reject 路径，或在 Relay 插入前统一拦截所有 `currentStep` 更新 |
| **应由哪个仓库解决** | `agent-dev-center` |

### B3. Domain Owner 非唯一（HIGH）

| 项目 | 内容 |
|---|---|
| **证据** | `DomainRoleBinding` (`schema.prisma:170-196`) 有 `isDomainAdmin` 标记但**非唯一约束**；`domain-scope.ts:56-57` 允许用户查询所有 `isDomainAdmin=true` 的 binding；`domains.ts` API 将 `isDomainAdmin` 视为"admin 级别"而非"唯一所有者" |
| **影响** | 迁移时无法确定哪个 Principal 是 DOMAIN_OWNER，且 svc-workflow 要求"一个 Domain 唯一一个 Owner" |
| **为什么阻断** | svc-workflow 架构冻结要求 `DomainRoleBinding(roleKey = DOMAIN_OWNER)` 唯一（`同一 Domain 最多一条 enabled DOMAIN_OWNER Binding`）。但 ADC 允许多个 `isDomainAdmin=true` 的 binding |
| **最小解决方案** | 在 ADC 中对每个 Domain 选择唯一的 `isDomainAdmin` 转换为 `DOMAIN_OWNER`；或暂时允许多个 Owner 但选择其中一个作为迁移目标 |
| **应由哪个仓库解决** | `agent-dev-center` |

---

## C. 推荐第一条垂直闭环

### 推荐：开发 Requirement 流程

**不推荐：llm-todo 流程**

选择理由：

| 维度 | Requirement 流程 | llm-todo 流程 |
|---|---|---|
| 现有模板结构 | 14 步线性流程，`contextSchema` 易于定义 | 平面状态机，不匹配有向图 |
| 负责人解析 | 已有 `roleUserMap` + 角色系统 | `assignee_agent_id` 灵活但非结构化 |
| 事务边界 | 已有 CAS + 事务模式 | SQLite，无 Oracle 级一致性需求 |
| Pilot 复杂度 | 可用单一模板（如 `hotfix` 或 `backend-dev`） | 需要修改现有状态机 |
| 迁移风险 | Requirement 是 ADC 的核心，迁移价值高 | llm-todo 状态灵活，迁移后反而不灵活 |
| 对账可行性 | `currentStep` 直接映射为 `nodeId` | 无对应 `currentStep` |

### 最小试点细节

| 项目 | 内容 |
|---|---|
| **模板** | `hotfix`（3 步：`draft → dev_self_check → done`，或保留 `backend-dev` 14 步完整版）|
| **contextSchema** | 见 4.2 节的候选 JSON Schema |
| **节点** | `draft`(DRAFT, WORKFLOW_CREATOR) → `dev_self_check`(NORMAL, DEVELOPER) → `done`(TERMINAL) |
| **负责人** | draft = WORKFLOW_CREATOR, dev_self_check = FIXED_PRINCIPAL (从 roleUserMap 解析), done = 无 |
| **Transition** | ADVANCE (draft→dev_self_check), ADVANCE (dev_self_check→done), RETURN (dev_self_check→draft) |
| **验收证据** | 在 svc-workflow 中创建 Instance，执行 ADVANCE，查询 assigned-to-me，再次 ADVANCE，确认进入 done |

---

## D. 实施拆分

### PR 1：仓储骨架 + 不可变事实表

- **内容**: PostgreSQL schema（`workflow.*`）、Rust 基础模块结构、领域类型
- **验收**: `cargo test` 通过，数据库 migration 可运行
- **不做的**: 路由、服务、导入

### PR 2：Definition + Version 管理

- **内容**: WorkflowDefinition CRUD、Version 发布/废弃/吊销、图结构验证
- **验收**: 可以创建模板、发布、发布后不可修改、校验通过
- **不做的**: Instance 创建、Transition

### PR 3：Instance + Context + NodeVisit + Submission

- **内容**: 创建 Instance、Context Revision（Draft 节点）、Node Visit 创建、Submission 创建
- **验收**: 从创建到提交的基本生命周期可运行
- **不做的**: Transition 引擎、返回

### PR 4：Transition 引擎（ADVANCE / RETURN / TERMINATE）

- **内容**: Transition 内核（ADVANCE 主干、RETURN 返回、TERMINATE 终止）、状态版本控制
- **验收**: 完整生命周期（创建→Draft 提交→ADVANCE→RETURN→再提交→ADVANCE→Done），幂等
- **不做的**: Event 查询、Agent 查询

### PR 5：API + 查询（assigned-to-me、timeline、Domain Owner 视图）

- **内容**: 全部查询 API、Field 权限过滤
- **验收**: Agent 可以查询自己的任务，Domain Owner 可以查看域全部流程
- **不做的**: 管理员修复、迁移导入

### PR 6：管理员修复（投影重建 + 紧急覆盖）

- **内容**: REBUILD_PROJECTION、ADMIN_EMERGENCY_OVERRIDE (MOVE_TO_NODE + TERMINATE_INSTANCE)
- **验收**: 损坏的 Instance 可以修复，紧急终止正常运行
- **不做的**: Legacy 导入、Shadow

### PR 7：Legacy 模板导入

- **内容**: 将旧 `WorkflowTemplate` 转为 `WorkflowDefinition` + `WorkflowDefinitionVersion`
- **验收**: 8 个默认模板全部可导入为不可变 Definition Version，`definitionDigest` 正确
- **不做的**: 旧数据实例导入、Shadow Relay、Cutover

---

## 验证命令结果

```bash
# git diff --check
# → 无空白错误

# cargo fmt --check
# → 格式检查通过

# cargo build
# → 编译通过（当前为 stub main.rs）

# cargo test
# → 测试通过（当前无测试）
```

---

## Git 提交

```
docs: investigate legacy ADC workflow migration
```

---

## 最终状态

```text
SVC_WORKFLOW_LEGACY_INVESTIGATION_COMPLETE
```
