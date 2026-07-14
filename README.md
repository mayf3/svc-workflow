# svc-workflow

`svc-workflow` 是 Rust + PostgreSQL 实现的串行受治理工作流内核。PostgreSQL
保存权威事实，Context Revision、Node Visit、Submission 与 Event 均不可变，
Instance 仅保存可重建的当前投影。

## 当前状态

冻结领域版本：`v0.3.1`。

已合并：

- PostgreSQL Storage Foundation；
- Definition Version Service；
- `CreateWorkflowInstance`；
- `ReviseWorkflowContext`；
- `ExecuteWorkflowTransition`。

当前实施：`ReviseContextAndTransition`（PR 3D）。后续范围见实施路线图。

当前尚未提供 HTTP/gRPC 服务；可用入口是 Rust application service。

## 文档入口

主线只保留当前有效合同，不保存已结束 PR 的长篇审计快照；历史审计仍可从 Git
对应提交恢复。

| 类型 | 文档 |
|---|---|
| 冻结领域架构 | [SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md](docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md) |
| 实施层总契约 | [IMPLEMENTATION_CONTRACT_V0_1.md](docs/contracts/IMPLEMENTATION_CONTRACT_V0_1.md) |
| 当前实施顺序 | [IMPLEMENTATION_ROADMAP_V0_1.md](docs/roadmap/IMPLEMENTATION_ROADMAP_V0_1.md) |
| 存储合同 | [POSTGRES_STORAGE_CONTRACT_V0_1.md](docs/contracts/POSTGRES_STORAGE_CONTRACT_V0_1.md) |
| Definition Service 合同 | [DEFINITION_SERVICE_CONTRACT_V0_1.md](docs/contracts/DEFINITION_SERVICE_CONTRACT_V0_1.md) |
| Runtime 创建合同 | [WORKFLOW_INSTANCE_CREATE_CONTRACT_V0_1.md](docs/contracts/WORKFLOW_INSTANCE_CREATE_CONTRACT_V0_1.md) |
| Runtime 流转合同（含 PR 3D） | [WORKFLOW_TRANSITION_CONTRACT_V0_1.md](docs/contracts/WORKFLOW_TRANSITION_CONTRACT_V0_1.md) |
| Legacy ADC 迁移 | [LEGACY_ADC_MIGRATION_V0_1.md](docs/migration/LEGACY_ADC_MIGRATION_V0_1.md) |

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
