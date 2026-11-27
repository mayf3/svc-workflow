# Wake：结果保持、确定性拒绝与提交未知

本目录把上一轮调查中的 Wake 模型纳入 `svc-workflow`，并将确定性拒绝、失败回执持久化、拒绝审计和提交未知补成第二个形式化切片。它提供 **44 条显式 Lean 定理**：`WakeReceipt` 的 21 条与 `WakeDurability` 的 23 条。数量包括辅助引理、历史反例和非空正例，不代表 44 个独立的生产保障。

需求来源是当前基线已接受的 `SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3` 的 **CTR-VAI-008、CTR-VAI-014**，以及既有存储合同。历史反例固定于 `mayf3/svc-workflow@c50ab2ac98decceaa26138a282c34dd92ede0ddc`。本轮实现、测试和独立复审见 [`../../docs/reports/WAKE_RECEIPT_FORMAL_REPAIR_20261127.md`](../../docs/reports/WAKE_RECEIPT_FORMAL_REPAIR_20261127.md)。本目录没有创建新产品合同，也没有更新生产状态。

## 1. 证明对象及证据层

| 对象 | 说明 | 证明边界 |
|---|---|---|
| `WakeReceipt.decodeHistorical` | 修复前按 HTTP status 将所有 200 解成 Applied 的分派 | 历史反例，不是修复后实现 |
| `WakeReceipt.decodeTyped`、`run` | 用 `wakeApplied` 区分 Applied/NoOp，保留原 scope/hash 的结果 | 一个成功完成的原子串行事务；实际 JSON/SQL 精化另验 |
| `WakeDurability.encode`、`decode` | 两类成功结果和五类确定性拒绝；状态码与错误码共同校验 | 结构化 payload 抽象；`invalid_cause` 需要原请求的 cause |
| `stageRejection` | 暂存一个拒绝回执和一个拒绝审计，业务状态保持 | 暂存写集不代表已经提交 |
| `settle`、`observeRead` | 提交确认、已知回滚、未知提交的状态与调用方观察 | 明确可能结果；不证明 PostgreSQL/SQLx 的实现 |
| `retryTrace` | 任意有限次同请求重放；后续 fresh 判定和传输结果可以变化 | 已记录结果优先；读回失败可返回 unavailable，不承诺每次请求都成功 |

运行 `verify.py` 会用固定 Lean 4.19.0 在新临时目录编译所有模块，检查每个显式定理的传递公理依赖，并从编译环境枚举本目录模块的**全部安全逻辑声明**，包括自动生成和私有内部声明。标准公理白名单为 `propext`、`Classical.choice`、`Quot.sound`；实际依赖以本次回执为准。编译器标记为 unsafe/partial 的辅助代码单独列出，不能排除任何 theorem。证明源禁止 `sorry`、自定义公理、`native_decide` 和自定义元编程。

验证回执在 `evidence/verification.json`。它分别记录模型源码哈希和 Rust/规范上下文哈希，并在运行前后冻结核对。**上下文哈希只固定被审阅的版本，不是实现精化证明。** Rust/PostgreSQL 回归证据由主变更记录单独列出，不混入 Lean 通过状态。

## 2. 原有成功结果模型

`WakeReceipt.State = (Business, Activation, Receipts)`。业务状态保存 instance、currentVisit、owner、closed、version、业务 Event 数与 eligibility fact 数。激活状态保存 Visit、closed 与 nextEligibleAt。回执键为 `(principal,key)`，值保存 canonical request hash 和原结果。

`Resolved(state,request)` 要求查出的 instance 与 activation 绑定到请求目标。该谓词是源码查询对应关系的外部前提，不是新运行时 guard。旧目标 Visit 可以与当前 Visit 不同，因此正常 stale/closed/version-mismatch 路径并未被假设排除。

回执重放先于 fresh 当前状态判定。新请求按源码中的顺序判断 `INSTANCE_CLOSED`、`ACTIVATION_CLOSED`、`VISIT_NOT_CURRENT`、`VERSION_MISMATCH`、`ALREADY_DUE`；只有可提前唤醒且版本未到 i32 正上界时才同时增加 version、Event、eligibility fact。NoOp 仍写回执，不能表述为“整个数据库没有变化”。本模块的状态不包含 attempt audit。

核心定理包括：

- `typed_roundtrip`：正常编码的 Applied/NoOp 由业务标签恢复原结果。
- `historical_noop_replay_fails`：旧分派对任意正常 NoOp 都无法还原原结果；`due_noop_counterexample_is_reachable_in_classifier` 给出正常 ALREADY_DUE 条件。
- `same_request_replays_without_writes`：已有同 scope/hash 回执时，成功完成的模型事务保持所建模状态，不重新计算 fresh 结果。
- `apply_then_arbitrary_replays_never_duplicate_business_effects`、`noop_then_arbitrary_replays_never_change_business`：通过任意自然数次数的归纳证明重复调用不会重复业务效果。
- `resolved_future_witness_is_applicable`：满足绑定、未来有效时间和版本上界的 apply 域非空。它不证明生产环境存在 `SCHEDULER_DEFER` 的可达 writer。

这些无传输错误的成功事务性质与下面的失败观察切片分开。不能从第一个模块推断网络失效时也一定拿到原响应。

## 3. 确定性拒绝的类型与请求上下文

持久化失败 body 保留既有 `{error: code}` 形状；无需重写历史回执。

| HTTP status | 持久化 error | 还原类型 |
|---|---|---|
| 422 | `invalid_cause` | `InvalidCause(original cause)` |
| 404 | `principal_not_found` | `PrincipalNotFound` |
| 403 | `principal_disabled` | `PrincipalDisabled` |
| 404 | `instance_not_found` | `InstanceNotFound` |
| 404 | `dispatch_intent_not_found` | `DispatchIntentNotFound` |

这里有一个必须说清的前提：两个不同 invalid cause 的持久化 payload 相同。因此**只看失败 body 不可能还原原 detail**。`invalid_cause_payload_does_not_store_detail` 证明这一信息损失；`ContextMatches` 将“匹配请求摘要所绑定的 cause 与原拒绝 detail 相同”显式写成条件。

在这一条件下，`completed_outcome_roundtrip` 对成功和拒绝统一证明：

\[
\operatorname{decode}(\operatorname{encode}(o), c)=\operatorname{Some}(o).
\]

`invalid_cause_without_request_context_is_corrupt` 保证缺少请求 cause 时不会凭空生成 detail；`rejection_is_not_successful_noop` 保证已知拒绝不会被当成成功 NoOp。状态码不匹配、未知错误码和 malformed payload 保持失败边界。

实际 hash envelope 在 `src/application/workflow_instance/idempotency.rs` 中包含 command type、principal、instance、Visit、expected version 与 cause。模型中的 hash 是符号；JCS 规范化、SHA-256 无碰撞及真实 JSON 请求解析不是上述定理的结论。

五种拒绝均是 codec 的合法类型，不因此宣称每种 fresh 路径在当前数据库都可达。例如 PrincipalNotFound 分支之前插入 receipt 时已有 principal 外键，真实不存在的 principal 可能更早返回存储错误；此项属于实现可达性边界，不能把 codec 正例写成 HTTP/PG 正例。

## 4. 失败写集与提交观察

`WakeDurability.Database` 保存业务状态、激活状态、回执表和 **WAKE_REJECTED 类别**的审计计数。`stageRejection` 只暂存目标 scope/hash 的失败回执和一个拒绝审计，业务状态与激活保持。

| 实际提交结果 | 模型持久状态 | 调用方观察 |
|---|---|---|
| `acknowledgedCommit` | 完整 staged 状态 | 原业务结果/原拒绝 |
| `knownRollback` | before 状态 | unavailable |
| `unknownCommitted` | staged 状态 | unavailable |
| `unknownRolledBack` | before 状态 | unavailable |

`knownRollback` 表示有独立依据确认事务未提交，例如提交前失败后回滚；它不表示普通 SQLx `commit()` 错误能告诉调用方数据库的最终事实。后两行是调用方无法区分的两种可能现实。

`acknowledged_rejection_is_durable` 证明收到原拒绝时采用的是完整 staged 状态；`historical_drop_loses_staged_failure_receipt` 对历史“暂存后直接返回而回滚”的路径保留反例。`fresh_rejection_never_changes_business_even_when_commit_is_unknown` 只适用于这类纯拒绝写集，不能推广到发生了业务 apply 的未知事务。

`unknown_commit_does_not_reveal_durability` 证明两个未知现实具有相同观察。`unknown_apply_can_hide_either_business_result` 提供完整建模写集的正例：同一个 unavailable 观察可对应 version 仍为 1，也可对应 version 已变为 2、激活时间已更新且成功回执已持久化。这个结果阻止把提交错误直接解释成“零业务效果”。

`runRejection` 先检查已有回执，只有新 key 才使用传入的 fresh rejection。传入拒绝必须来自实际已完成的分类；本模型没有另造权限或存在性判定。`PROCESSING` 不被当成完成结果，其他 command type 或不同 hash 冲突。

已完成回执的读事务也可能提交确认失败。`observeRead` 对这种情况返回 unavailable，数据库中已经持久化的原回执仍在。`arbitrary_replays_preserve_state_and_only_observe_original_or_unavailable` 对任意有限列表归纳：**业务状态、激活、回执与拒绝审计计数都保持，已成功返回的业务结果只能是原结果，其他观察可以是 unavailable。** 这比“所有重试一定返回成功”更符合实际错误边界。

`rejection_then_replays_never_duplicate_rejection_audit` 证明第一次已提交拒绝只增加一个拒绝审计，任意后续同请求重放不会重复它。`unknown_committed_rejection_is_recoverable_by_same_request` 说明实际已提交但响应未知的拒绝，可以在后续一次成功读回后恢复为同一原拒绝；它没有授予换 key、重新执行业务副作用或无限盲重试的权限。

## 5. 代码对应与剩余义务

以下符号以同次验证回执中的实现哈希为准，避免行号随修复移动：

| 模型 | Rust/SQL 对应位置 | 实现验证重点 |
|---|---|---|
| Scope/hash/CommandType/ReceiptState | `wake_transaction.rs::acquire_receipt` | 同一 principal/key；command type/hash 一致；PROCESSING 与 COMPLETED 分开 |
| `decodeTyped`、`WakeDurability.decode` | `wake_transaction.rs::decode_replay` | `wakeApplied` 必须是 bool；对应必填字段；错误 status/code 与 original cause |
| `ContextMatches` | `application/workflow_instance/idempotency.rs` 的 Wake hash envelope | 同请求摘要确实覆盖 cause；不依赖调用方新 key |
| `classify` 与 `Resolved` | `wake_dispatch_intent_atomically` 及 `activation_facts::find_dispatch_activation_for_wake` | 当前 Visit/版本、目标绑定、关闭及到期谓词 |
| `stageRejection`、`settle` | `wake_transaction.rs::deterministic_failure` | 在同一个拥有的事务完成回执和拒绝审计；commit 成功后才返回原错误；错误传播 |
| `appliedState` | Wake apply 分支 | version/Event/fact 一次共同提交；应用审计为单独实现义务 |
| `observeRead` | replay 分支先 `tx.commit().await` 再 `decode_replay` | 重放事务提交失败仍可以返回存储不可用 |
| 错误到 HTTP | `src/http/error.rs::from_wake` | 保留原拒绝状态和详情；InternalConsistency/StorageError 为既有 503 |

必要边界：

- 原子性与串行化是模型前提。真实 SQL、唯一约束、锁顺序、MVCC、约束触发器、崩溃和网络中断未被 Lean 精化证明，需真实 Rust/PostgreSQL 集成证据。
- 真正 HTTP 身份、scope、角色、可见性与 opaque error 在事务前还有检查；模型没有把历史回执当成绕过这些检查的能力。
- `WakeReceipt` 的计数只包含业务 Event/fact；`WakeDurability` 的审计计数只包含 WAKE_REJECTED。Conflict、NoOp、Applied 的审计不能从这一个计数推断。
- `Response` 是对已解析字段的结构化抽象。缺失/错型 JSON 映射到 malformed 的实际准确性，以及额外字段处理，由 Rust 单元与集成测试验证。
- IDs、时间、hash 主要使用自然数符号。没有证明 UUID/RFC3339/时钟一致性；版本域限定为非负、可映射到实际 i32 的输入。
- 没有公平调度、无限执行、消费者覆盖或“最终业务任务完成”定理；没有生产部署或真实任务验收声明。

## 6. 复现

在仓库根执行：

```bash
python3 formal/wake/verify.py --lean /absolute/path/to/lean-4.19.0-linux/bin/lean
```

默认输出在 `formal/wake/evidence/`，可用 `--out /temporary/path` 在独立位置复核。脚本只依赖 Python 标准库和固定 Lean 工具链。它不使用已有 `.olean`，不下载依赖，不运行 Rust/数据库测试。具体实现回归命令和环境由主变更记录保存。
