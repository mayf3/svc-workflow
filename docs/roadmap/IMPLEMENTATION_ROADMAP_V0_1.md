# svc-workflow 实施路线图 v0.1

```text
Status: ACTIVE_EXECUTION_PLAN
Version: v0.1
Architecture Baseline: v0.3.1
Current Main Baseline: 619f34320d92e9b6666374b4a56c8cc21614f26b
```

> 本文档是实施顺序和状态记录，不是领域架构合同。
> 领域架构以 `SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md` 为准。
> 实施路线可以调整 PR 拆分，但不能静默改变冻结架构。

---

## Phase A：独立 svc-workflow 内核 MVP

共 7 个 PR，其中 PR 1 已完成。

| PR | 内容 | 状态 | 验收门 |
|---|---|---|---|
| PR 1 | PostgreSQL Storage Foundation | MERGED | 不可变事实、约束和 Migration 通过独立审计 |
| PR 2 | Definition 与不可变版本发布服务 | IN_PROGRESS | 可创建 Draft、校验图、发布、弃用、撤销 |

PR 2 实现分支：`feat/definition-version-service-v0`，基线 `d8e9808`。  
当前 HEAD：`a64e41a`（含后续结构收敛提交）。

| PR 2 实施文件 | 行数 |
|---|---|
| 领域模块（model, graph, digest, error） | 1,396 |
| 应用层（service, lifecycle, draft_graph, commands, queries, repository trait） | 1,038 |
| 存储层（PgDefinitionRepository, repository_rows） | 670 |
| 单元测试（graph_tests, digest_tests, enums, ids） | 1,313 |
| 集成测试（13_definition_service, 14_lifecycle） | 688 |
| Migration 0008 | 124 |
| 合同文档 | 308 |
| PR 3 | 通用 Command 执行与幂等框架 | PLANNED | Receipt 并发、requestHash、权限、审计 |
| PR 4 | Instance 与 Workflow Context | PLANNED | 创建实例、Context Revision、Draft 修改 |
| PR 5 | Transition、Submission 与 Event Engine | PLANNED | ADVANCE、RETURN、TERMINATE 完整闭环 |
| PR 6 | Query Model 与 HTTP API | PLANNED | assigned-to-me、timeline、feedback |
| PR 7 | 管理员恢复与完整 E2E | PLANNED | 投影重建、紧急修复、MVP 端到端 |

Phase A 完成标志：

```text
SVC_WORKFLOW_KERNEL_MVP_READY
```

---

## Phase B：ADC 真实 Domain 试点

| PR | 内容 | 状态 |
|---|---|---|
| PR 8 | ADC 迁移前置：模板覆盖、唯一 Domain Owner、身份映射 | BLOCKED_BY_LEGACY_GAPS |
| PR 9 | ADC 状态入口收敛与持久 Relay | BLOCKED_BY_LEGACY_GAPS |
| PR 10 | Legacy 导入与 Shadow Compare | PLANNED |
| PR 11 | Domain Cutover 与 Rollback Window | PLANNED |

Phase B 完成标志：

```text
SVC_WORKFLOW_ADC_PILOT_CUTOVER_PASS
```

---

## Phase C：llm-todo 接入

| PR | 内容 | 状态 |
|---|---|---|
| PR 12 | llm-todo Workflow Adapter | PLANNED |
| PR 13 | llm-todo 状态机 Cutover | PLANNED |

---

## Legacy Blocker（来自调查报告）

以下三个迁移阻断由 `LEGACY_ADC_READ_ONLY_INVESTIGATION_REPORT.md` 发现：

```text
B1 Startup 模板覆盖（HIGH）—— ensureWorkflowTemplates() 启动时 upsert 模板
B2 非标准 currentStep 写路径（HIGH）—— reports-approval、core-patch 等旁路
B3 Domain Owner 非唯一（HIGH）—— isDomainAdmin 非唯一约束
```

- 它们 **不阻断** Phase A 的 Rust 内核开发；
- 它们 **阻断** Phase B 的 Shadow 和 Cutover；
- 在真正修复前，必须基于当时 `agent-dev-center/main` 或生产权威版本重新确认。

---

## 当前非阻断存储 Notes

### Medium

```text
Definition Graph 子表修改 definition_version_id 的窄逃逸路径
```

处理计划：

```text
PR 2 关闭
```

### Low

```text
PROCESSING CommandReceipt 当前可删除
```

处理计划：

```text
PR 3 关闭
```

### Deferred Note

```text
非循环复合外键暂时仍为 DEFERRABLE INITIALLY DEFERRED
```

处理计划：

```text
PR 3 在完整 Command 事务设计下重新评估和收窄
```

---

## 路线图维护规则

1. 每个 PR 合并后更新状态和实际 merge SHA。
2. 只有真实 Blocker / High 阻断合并。
3. 每个功能 PR 完成后先独立审查，再合并。
4. 不同时启动两条高风险实施线。
5. 不在一个 PR 中同时修改 `svc-workflow` 和多个上层产品。
6. 架构变化必须单独讨论，不能通过路线图静默修改。
7. 预计总 PR 数是执行计划，不是必须机械维持的数字。
8. 合并或拆分 PR 时必须保留独立验收门。
