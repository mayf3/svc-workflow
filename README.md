# svc-workflow

`svc-workflow` 是 Rust + PostgreSQL 实现的串行受治理工作流内核。PostgreSQL
保存权威事实，Context Revision、Node Visit、Submission 与 Event 均不可变，
Instance 仅保存可重建的当前投影。

当前领域版本：`v0.3.1`，Schema 版本：`0014`。

当前已提供完整 HTTP API（`/internal/v1/**`），包括工作流实例创建/查询/迁移、
事件 Timeline、工作清单、域成员管理、Definition 治理、Admin 控制面等端点。

## 文档入口

主线只保留当前有效合同，不保存已结束 PR 的长篇审计快照；历史审计仍可从 Git
对应提交恢复。

| 类型 | 文档 |
|---|---|
| 冻结领域架构 | [SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md](docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md) |
| HTTP Contract（OpenAPI） | `contracts/workflow-http/v1/` |
| TypeScript SDK | `sdk/typescript/` |
| 实施层总契约 | [IMPLEMENTATION_CONTRACT_V0_1.md](docs/contracts/IMPLEMENTATION_CONTRACT_V0_1.md) |
| 存储合同 | [POSTGRES_STORAGE_CONTRACT_V0_1.md](docs/contracts/POSTGRES_STORAGE_CONTRACT_V0_1.md) |
| Definition Service 合同 | [DEFINITION_SERVICE_CONTRACT_V0_1.md](docs/contracts/DEFINITION_SERVICE_CONTRACT_V0_1.md) |
| Runtime 创建合同 | [WORKFLOW_INSTANCE_CREATE_CONTRACT_V0_1.md](docs/contracts/WORKFLOW_INSTANCE_CREATE_CONTRACT_V0_1.md) |
| Runtime 流转合同 | [WORKFLOW_TRANSITION_CONTRACT_V0_1.md](docs/contracts/WORKFLOW_TRANSITION_CONTRACT_V0_1.md) |
| Runtime 查询合同 | [WORKFLOW_QUERY_CONTRACT_V0_1.md](docs/contracts/WORKFLOW_QUERY_CONTRACT_V0_1.md) |
| Admin Recovery 合同 | [ADMIN_RECOVERY_CONTRACT_V0_1.md](docs/contracts/ADMIN_RECOVERY_CONTRACT_V0_1.md) |
| Legacy Initial Import 合同 | [LEGACY_IMPORT_CONTRACT_V0_1.md](docs/contracts/LEGACY_IMPORT_CONTRACT_V0_1.md) |
| Legacy ADC 迁移 | [LEGACY_ADC_MIGRATION_V0_1.md](docs/migration/LEGACY_ADC_MIGRATION_V0_1.md) |

## 维护规则

1. 每个切片基于最新 `main`，独立审计关闭全部 Blocker/High 后只做
   `git merge --ff-only`。
2. PostgreSQL 是唯一权威状态源；Instance 当前字段只是可重建投影。
3. 一次成功状态命令只增加一个状态版本，并只写一条对应 Event。
4. 当前树只保留冻结架构和仍有效合同；已结束 PR 的调查与审计证据保留在 Git 历史。

## 本地验证

测试默认连接：

```text
postgres://postgres:postgres@localhost:5432/svc_workflow
```

启动 PostgreSQL 后运行：

```bash
docker compose up -d postgres
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
