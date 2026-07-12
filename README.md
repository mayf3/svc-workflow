# svc-workflow

`svc-workflow` 是独立 Rust + PostgreSQL 串行受治理工作流内核。

它负责保证：正确的负责人，在正确的节点，基于明确版本的工作内容，提交符合模板协议的 JSON；
工作流按不可变模板合法流转；每一次内容修改、阶段提交、正常推进、跨级返回和异常终止都有不可修改的历史。

`svc-workflow` 不理解业务内容，也不运行 LLM。

---

## 当前状态

```text
ARCHITECTURE_FROZEN
IMPLEMENTATION_NOT_STARTED
```

* 领域架构已冻结于 v0.3.1；
* 数据库 Schema、Command Service、HTTP API 与任何业务逻辑均尚未实现；
* 本仓库当前仅包含可编译的最小占位程序与文档基线。

---

## 文档

| 文档 | 说明 |
| --- | --- |
| [架构基线 v0.3.1（冻结）](docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md) | 正式领域架构，不可重新设计。 |
| [实施契约勘误 v0.1](docs/contracts/IMPLEMENTATION_CONTRACT_V0_1.md) | 已确认的实施层契约，不改变领域架构。 |
| [Legacy ADC 迁移勘误 v0.1](docs/migration/LEGACY_ADC_MIGRATION_V0_1.md) | 迁移调查占位，状态 `PENDING_READ_ONLY_INVESTIGATION`。 |

---

## 构建

```bash
cargo build
cargo test
```

当前 `src/main.rs` 仅是最小占位程序：不启动 HTTP 服务，不连接数据库，也不实现任何工作流领域逻辑。
