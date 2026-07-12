# Legacy ADC 迁移勘误 v0.1

```text
Status: INVESTIGATION_COMPLETE
Version: v0.1
Architecture: SVC_WORKFLOW_ARCHITECTURE_V0_3_1 (ARCHITECTURE_FROZEN)
```

> 本文件基于只读调查完成。详细调查文档见：
> `docs/migration/LEGACY_ADC_READ_ONLY_INVESTIGATION_REPORT.md`
>
> 约束：本文件不修改已冻结领域架构，不承诺任何迁移方案。
> 所有迁移方案必须基于架构基线 `SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md`
> 与实施契约 `IMPLEMENTATION_CONTRACT_V0_1.md`。
> 在正式迁移前，本文件保持 `INVESTIGATION_COMPLETE`。

---

## 1. 调查结论

总体判定：`READY_WITH_BLOCKING_MIGRATION_GAPS`

阻塞缺口（必须在实施前由 agent-dev-center 仓库解决）：

### B1. Startup 模板覆盖（HIGH）

`backend/src/lib/workflow-templates.ts:398-427` 中 `ensureWorkflowTemplates()` 在每次服务启动时执行 `upsert`。模板 `steps` 可以被代码覆盖。需要 ADC 停止启动时 upsert 模板，改为仅首次初始化或仅添加不修改。

### B2. 非标准 currentStep 写路径（HIGH）

`reports-approval.ts:198`（报告批准自动推进）、`core-patch.ts`（通用 PATCH 直接更新 currentStep）等路径不会写入 Relay，导致 Shadow 期状态不一致。需要 ADC 将这些路径改为走标准 advance/reject 路径。

### B3. Domain Owner 非唯一（HIGH）

`DomainRoleBinding` 的 `isDomainAdmin` 标记非唯一——同一域可以有多个 `isDomainAdmin=true` 的 binding。svc-workflow 要求"一个 Domain 唯一一个 Owner"。需要 ADC 在迁移前确定每个 Domain 的唯一 DOMAIN_OWNER。

---

## 2. 身份映射

`principalId` = auth-service `User.id` (UUID)。

| Legacy Identity | 映射方式 | 稳定性 |
|---|---|---|
| `Requirement.assigneeId` (UUID) | 直接映射 | 稳定 |
| `Requirement.requesterId` (UUID) | 直接映射 | 稳定 |
| `User.agentId` (String) | 通过 `User.agentId → User.id` 解析 | 稳定 |

无法映射的 Creator 使用 `DOMAIN_OWNER_FALLBACK`。

Migration Service Principal 应创建为固定 auth-service User。

---

## 3. Workflow Context 边界

开发流程 contextSchema 候选（仅调查结论，不修改冻结架构）：

```json
{
  "type": "object",
  "required": ["title", "description", "domainKey"],
  "properties": {
    "title": { "type": "string", "minLength": 1 },
    "description": { "type": "string", "minLength": 1 },
    "acceptanceCriteria": { "type": "array", "items": { "type": "string" } },
    "domainKey": { "type": "string" },
    "type": { "type": "string", "enum": ["FEATURE", "BUGFIX", "INFRA", "SECURITY"] },
    "repoPath": { "type": "string" },
    "branch": { "type": "string" },
    "gitHash": { "type": "string" }
  }
}
```

不包含 `priority`、`projectId`、`tags`、`dueDate` 等业务字段。

---

## 4. 模板映射

| ADC 概念 | svc-workflow 概念 | 备注 |
|---|---|---|
| `WorkflowTemplate.name` | `WorkflowDefinition.definitionKey` | 机械转换 |
| `steps[].name` | `NodeDefinition.nodeKey` | 机械转换 |
| `steps[].role=requester` | `assigneeRef=WORKFLOW_CREATOR` | 机械转换 |
| `steps[].role=cto/ops/qa` | `assigneeRef=FIXED_PRINCIPAL` 或 `DOMAIN_OWNER` | 需要人工规则 |
| 数组顺序 | `primaryAdvanceTransitionId` 链 | 需要从顺序生成 |
| `rejectTo` | `RETURN` Transition | 需要枚举所有路径 |
| 无对应 | `TERMINATE` Transition | 新增 |
| 无对应 | `contextSchema` | 新增 |
| `requiredReports` | `submissionSchema` | 需要根据 ReportType 定义 |

---

## 5. Submission 迁移分类

| 分类 | 旧数据 | 条件 |
|---|---|---|
| SAFE_TO_IMPORT_AS_SUBMISSION | `RequirementReport` (approved) | 可关联到某步骤 |
| IMPORT_AS_LEGACY_REFERENCE | `RequirementReport` (rejected)、`RequirementAuditLog`、`WorkflowTransition` | 保留为历史参考 |
| KEEP_ONLY_IN_LEGACY | `Requirement`、`ExecutionLease`、`FeedbackEvent`、`TestEnvLock` | 上层业务数据 |
| UNMAPPABLE | 无法确定 NodeVisit 归属的旧报告 | 不得伪造为 committed Submission |

---

## 6. 推荐第一条垂直闭环

**推荐：开发 Requirement 流程**（不推荐 llm-todo）

模板：`hotfix`（3 步）或 `backend-dev`（14 步）
Context：见第 3 节 contextSchema
节点：`draft`(DRAFT) → `dev_self_check`(NORMAL) → `done`(TERMINAL)
负责人：WORKFLOW_CREATOR → FIXED_PRINCIPAL → 无
Transition：ADVANCE, RETURN

---

## 7. 实施 PR 顺序

| PR | 内容 | 验收 |
|---|---|---|
| PR 1 | 仓储骨架 + 不可变事实表 | migration + test |
| PR 2 | Definition + Version 管理 | 创建/发布/校验 |
| PR 3 | Instance + Context + NodeVisit + Submission | 基本生命周期 |
| PR 4 | Transition 引擎 (ADVANCE/RETURN/TERMINATE) | 完整流转 + 幂等 |
| PR 5 | API + 查询 | assigned-to-me, timeline |
| PR 6 | 管理员修复 | 投影重建 + 紧急覆盖 |
| PR 7 | Legacy 模板导入 | 8 模板转为 DefinitionVersion |

---

## 8. Shadow Relay 设计要点

推荐插入点：`casUpdateRequirement()` 成功后、事务提交前。

Relay 最小字段：`id`, `domain_key`, `requirement_id`, `event_type`, `current_step`, `assignee_id`, `state_version`, `relay_payload`, `idempotency_key`, `relay_status`, `created_at`

去重键：`legacy:<domainKey>:<requirementId>:<stateVersion>`

当前无 Outbox 模式可复用。

---

## 9. 数据库部署

- ADC 使用 PostgreSQL 16，已有运行实例
- auth-service 共享 ADC 数据库
- llm-todo 使用独立 SQLite
- svc-workflow 使用同一 PostgreSQL 集群，独立 `svc_workflow` database，`workflow` schema

---

## 10. 已完成的调查

详细调查文档（包括所有证据的文件路径、行号、代码引用）：
`docs/migration/LEGACY_ADC_READ_ONLY_INVESTIGATION_REPORT.md`
