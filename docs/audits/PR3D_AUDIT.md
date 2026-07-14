# PR 3D 独立审计

```text
Verdict: PASS_WITH_NOTES
Initial Verdict at 6ed3a71: FAIL_HIGH
Branch: codex/revise-context-and-transition-v0
Base: f5fb0b48d548537809cc107e55752131cdea917a
Initial implementation: 6ed3a717e2b74aef83c328e4a76fe89277953cad
Re-audited fixed HEAD: 99284f74b72c2c836a3767f3ec8724b9ee8268e6
Audit date: 2026-07-14 (Asia/Shanghai)
```

审计严格只读实现；除本报告外未修改源代码、测试、既有文档或 Git 历史。

## 定向复审结论（99284f7）

原 High 已关闭；`6ed3a71..99284f7` 增量未引入 Blocker / High。

1. `DEFINITION_SERVICE_CONTRACT_V0_1.md` 已精简收敛原 supplement 中仍有效的
   lifecycle actor 写/读矩阵、`context_schema` 三态 Patch、外部 Schema reference
   禁止、四个读操作的 DOMAIN_OWNER 授权、禁用 Domain 门禁与 typed PostgreSQL
   error mapping。新增内容与现有代码、测试一致；原 supplement 可安全从当前主线删除。
2. Application 层按 Patch 后有效值校验：`None` 使用已存 Schema，JSON `null` 视为
   无 Schema，其他值作为替换 Schema（`draft_graph.rs` 第 145–165 行）。这避免
   `None` 跳过现存 Schema 校验，也不会把 JSON `null` 当作待编译 Schema。
3. Storage 层在持有 DefinitionVersion `FOR UPDATE` 的事务内实现：`None` 不执行
   UPDATE；JSON `null` 明确 `SET context_schema = NULL`；其他值参数化写入 JSONB
   （`graph_write.rs` 第 28–43、105–129 行）。Graph replacement 与 Patch 同事务。
4. 新 PostgreSQL 集成测试通过 application service 连续执行 keep → clear → replace，
   并直接读库断言保留原对象、`IS NULL = true`、替换对象精确相等；不是 mock 或仅
   单元测试（`b2_schema_validation.rs` 第 7–67 行）。

复审实际运行并通过：

```text
cargo fmt --check
cargo build
cargo clippy --all-targets --all-features -- -D warnings
cargo test --test 16_definition_service_audit_fix \
  b2_schema_validation::test_context_schema_patch_keeps_clears_and_replaces -- --exact
cargo test -- --test-threads=1
cargo test --quiet
git diff --check (working tree tracked diff and 6ed3a71..99284f7)
```

复审全量为 326 tests（54 lib + 272 integration）；Definition audit-fix 集成测试由
31 增至 32，PR 3D 仍为 23。PostgreSQL 16.14；残留 test Trigger/Function 0/0；
Migration diff 0；最大 Rust 文件 493 行；`tests/` 直接子项 20；目录深度 4。

## 初审阻断结论（已关闭）

### H1 — 文档收敛误删仍有效的 Definition Service 安全与行为合同（CLOSED）

本提交删除了 `docs/contracts/DEFINITION_SERVICE_FIX_CONTRACT_V0_1.md`。该文件在
base 中明确标记为 `IMPLEMENTATION_CONTRACT (audit fix supplement)`，并声明其补充
冻结架构与基础合同，不是一次性审计快照（base 文件第 4、11–13 行）。

删除前没有把以下仍由当前代码和测试执行、但保留合同未完整表达的权威事实合并到
`docs/contracts/DEFINITION_SERVICE_CONTRACT_V0_1.md`：

- 外部 `$ref`、`$dynamicRef`、`$recursiveRef` 禁止，仅允许本地 fragment（原补充合同
  第 88–109 行）；保留合同第 118–121 行只说 Schema 可编译，没有冻结此安全边界。
- `get_definition`、`get_definition_version`、`list_definition_versions`、
  `get_complete_version_graph` 四个读操作也必须通过 DOMAIN_OWNER 授权（原第
  179–196 行）；保留合同第 207–227 行只枚举 Definition 管理写操作。
- lifecycle actor 写入/读回矩阵（原第 157–175 行）。
- typed DB error 映射及 `context_schema` 的 keep / clear / replace 三态 patch 语义
  （原第 213–246 行）。

同时，`README.md` 第 25–36 行声明主线只保留当前有效合同，并把保留的 Definition
合同列为唯一入口。因此这不是“历史审计从 Git 恢复”即可覆盖的删除，而是当前合同
出现缺口。前两项分别对应已关闭的 B-2 与 H-5 安全修复，未来维护者按现存合同实施
或重构时可能重新开放外部 Schema 解析或越权读取，定级 High。

初审要求保持文档收敛方向，不恢复长补充文档，而将上述仍有效事实精简合并进主
Definition 合同；该要求已由 99284f7 落实并通过本次定向复审。

## PR 3D 实现核对

实现本身未发现 Blocker / High：

- Creator 与 current assignee 使用 AND 门禁；current Visit 必须为 DRAFT；仅接受其
  primary ADVANCE（`combined_transaction.rs` 第 139–219 行）。
- 两个 payload 在命令类型中均为必填 `Value`，并分别进行 1 MiB 与 JSON Schema
  校验（`commands.rs` 第 100–115 行；`combined_transaction.rs` 第 229–252、
  389–433 行）。
- 新 Revision 指向命令前 Revision；Submission 绑定新 Revision；随后创建新 Visit，
  两个 projection 指针与 state version 在同一事务更新（第 279–332 行）。
- 锁序为 Receipt → Instance `FOR UPDATE` → DefinitionVersion `FOR UPDATE`（第
  34–51、139–148 行）。
- 成功只增加一次 `workflowStateVersion`，且只插入一条组合 Event；Event 字段矩阵和
  `eventSequence = new stateVersion` 正确（第 286、333–363 行；
  `combined_helpers.rs` 第 149–228 行）。
- 同 hash 成功/失败均重放存储响应；不同 hash 写 AttemptAudit；失败 Receipt 的
  response digest 采用 canonical JSON digest（`combined_transaction.rs` 第 45–124、
  355–373 行；`combined_receipt.rs`）。
- 基础设施错误经 `?` 离开未提交事务并整体回滚；故障注入覆盖 Submission、Instance、
  Event、Receipt completion，且 Trigger/Function 名唯一、条件化、RAII 独立连接清理。
- revise-only、transition-only 与组合命令均先锁同一 Instance；两组混合并发测试都验证
  同一 expected version 只能成功一个。
- requestHash 冻结信封覆盖 principal、instance、expected version、transition 与两个
  payload；golden canonical JSON 和 SHA-256 测试通过。成功 response digest 读回一致。
- Migration diff 为 0，现有表和约束足够。

## Medium / Low Notes

- Medium：PR 3D 的 23 项测试没有组合命令专属的 context/submission 超 1 MiB 用例，
  也没有 ContextRevision INSERT、NodeVisit INSERT 两个独立故障注入点。实现路径与
  事务边界正确，现有测试已覆盖同类 size helper、DDL 限制及其前后写入点回滚，因此
  不上调为 High。
- Low：组合命令未单独覆盖 DEPRECATED 成功、DRAFT 防御拒绝、FIXED_PRINCIPAL 目标
  assignee 成功；对应分支复用已审计的 transition validation/assignee resolver。

## 初审实际验证（6ed3a71）

以下 325 项结果记录初审固定实现 `6ed3a71`；最终 HEAD `99284f7` 的 326 项复审
结果及本轮命令见文首“定向复审结论”，两者不是同一 SHA 的测试总数。

```text
PostgreSQL: 16.14 (Homebrew)
cargo fmt --check: PASS
cargo build: PASS
cargo clippy --all-targets --all-features -- -D warnings: PASS
cargo test -- --test-threads=1: PASS
cargo test (default parallel) x5: PASS x5
cargo test -- --list: PASS
git diff --check: PASS (working tree before report and fixed base..HEAD range)

Tests: 325 total = 54 lib + 271 integration
PR 3D tests: 23
  success 2; validation 6; concurrency 4; idempotency 4;
  request-hash 3; atomicity 4

Residual test triggers: 0
Residual test functions: 0
Migration diff: 0 files
Largest handwritten Rust file: transition_transaction.rs, 493 lines
Largest directory direct children: tests/, 20
Maximum directory depth: 4
```

最终工作区预期仅有本报告为未跟踪文件；未 commit。
