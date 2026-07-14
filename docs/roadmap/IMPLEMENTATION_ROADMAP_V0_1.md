# svc-workflow 实施路线图 v0.1

```text
Status: ACTIVE_EXECUTION_PLAN
Architecture Baseline: v0.3.1
Current Main Baseline: 11758f9f4fc99e521541a08abdd96cd4ac5b330c
```

本文只记录实施顺序和交付状态。领域语义以冻结架构为准，具体运行时行为以
`docs/contracts/` 下仍在维护的合同为准。

## 独立内核主线

| 切片 | 内容 | 状态 |
|---|---|---|
| PR 1 | PostgreSQL Storage Foundation | MERGED |
| PR 2 | Definition Version Service | MERGED |
| PR 3A | `CreateWorkflowInstance` | MERGED |
| PR 3B | `ReviseWorkflowContext` | MERGED |
| PR 3C | `ExecuteWorkflowTransition` | MERGED |
| PR 3D | `ReviseContextAndTransition` | MERGED (`11758f9`) |
| PR 4 | Read Model / Query Service | IN_PROGRESS |
| PR 5 | Admin Emergency Commands 与 projection repair | PLANNED |
| PR 6 | Legacy ADC Migration / Shadow Relay | BLOCKED_BY_LEGACY_GAPS |

PR 3D 的验收门：

```text
Creator + current assignee 双重权限
DRAFT primary ADVANCE only
新 Context Revision + 新 Submission + 新 Node Visit 原子提交
Submission 绑定本命令新 Revision
workflowStateVersion 只增加 1
只创建 WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED 一条 Event
Receipt 幂等、确定性失败重放、基础设施失败全回滚
与 revise-only / transition-only 并发线性化
无需 Migration
```

PR 4 至少提供：

```text
WorkflowInstance detail
current Context
current Visit
timeline/events
submission history
assigned-to-me
creator-owned drafts
```

PR 5 只提供受更高权限保护的：

```text
MOVE_TO_NODE
TERMINATE_INSTANCE
projection rebuild / repair
```

## Legacy ADC 阻断

开始 PR 6 前必须基于当时的 Legacy 权威版本重新确认：

```text
B1 ensureWorkflowTemplates() 启动时覆盖模板
B2 非标准 currentStep 写路径绕过 Relay
B3 Legacy Domain Owner 不唯一
```

这些问题不阻断独立 Rust 内核，但阻断 Shadow/Cutover。需要修改 ADC 或其他仓库时，
必须单独取得授权。

## 维护规则

1. 每个切片从最新 `main` 创建分支，审计通过后 `git merge --ff-only`。
2. Blocker/High 必须关闭；Medium/Low 记录后可继续。
3. PostgreSQL 始终是唯一权威状态源，查询层不得成为第二状态源。
4. 一次成功状态命令只增加一个状态版本并只创建一条 Event。
5. 已结束 PR 的详细审计保留在 Git 历史，不在主线根目录重复堆积。
