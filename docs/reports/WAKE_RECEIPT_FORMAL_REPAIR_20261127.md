# Wake receipt repair and proof — bounded change record

本轮已完成候选修复、Lean 条件模型证明、隔离 PostgreSQL 回归和独立复审。Wake 的 44 条显式定理通过；新代码的 2 项 codec 测试与全部 17 项 Visit/Activation 集成测试通过；同一组集成回归放回旧 Rust 实现时为 10 passed / 7 failed，真实检出原缺陷。下方 PREFLIGHT 保留启动时记录，最终证据和边界见后文。

## Authorization and route

The repository owner requested a scan, mathematical modeling and formal proof of `svc-workflow`, then instructed **“继续完善”** on 2026-11-27. This continuation authorizes an isolated, reviewable implementation of the already identified Wake receipt defects, its models, and focused regression validation. It does not authorize production operations or create new product contracts.

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
TARGET_REPOSITORY = mayf3/svc-workflow
REVIEW_TARGET_HEAD = c50ab2ac98decceaa26138a282c34dd92ede0ddc
BASE_HEAD = c50ab2ac98decceaa26138a282c34dd92ede0ddc
CURRENT_BASE_HEAD = c50ab2ac98decceaa26138a282c34dd92ede0ddc
ISOLATED_REF = fix/wake-receipt-formal-20261127
ROUTE_STAGE = IMPLEMENTATION
AUTHORITY_ACCEPTED_IN_BASE = YES
GOAL_OR_TARGET = Preserve the original typed Wake outcome on replay, including durable deterministic rejection, with zero duplicate business effects.
CURRENT_GAP = Both Applied and NoOp use HTTP 200 but replay is decoded as Applied; deterministic_failure completes a receipt without committing and its replay decoder is also incorrect.
AUTHORITY_ACTION = REUSE
PRIMARY_AUTHORITY = SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3 / CTR-VAI-008, CTR-VAI-014
RELATED_AUTHORITIES = POSTGRES_STORAGE_CONTRACT_V0_3; existing Wake HTTP error contract
IMPLEMENTATION_AUTHORITY = contracts
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = NOT_APPLICABLE
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = DURABLE
EXECUTION_MANDATE = NOT_APPLICABLE
MUTATION_AUTHORIZATION = VALID (owner continuation of the existing review and concrete follow-up)
ISOLATED_WRITE_SURFACE = YES (task-only clone and named ref)
CONTROLLED_RUNBOOK_REQUIRED = NO
SPEC_GAP_DEPENDENCY = NON_LOAD_BEARING
EVIDENCE_REVIEWABILITY = PASS (source, models, commands and local disposable database evidence)
LIVE_AUTHORITY_GAP = DETECTED (registered Product V13 / Architecture V0_4_7 files absent from base)
OWNER_DECISION_REQUIRED = NO for this bounded existing-contract repair
EMERGENCY_STATE = NONE
EMERGENCY_ACTION = NONE
INCIDENT_REFERENCE = NOT_APPLICABLE
BASE_IMPACT = BOUNDED
IMPLEMENTATION_ALLOWED = YES
MERGE_READY = NO pending implementation and independent final review
OPERATION_ALLOWED = NOT_APPLICABLE
EVIDENCE_NEEDED = Lean axiom-audited proofs; real Rust/PostgreSQL regressions; independent source review
DONE_WHEN = Applied/NoOp/rejection replay and persistence match the existing contracts; selected models and exact implementation mapping are reproducible and independently reviewed.
EXPANSION_TRIGGER = Any required change to authorization, public response semantics, accepted durable-data contracts or unknown-commit behavior.
NEXT_REAL_ACTION = Implement typed replay and atomic rejection completion, then exercise actual local database paths.
NEXT_ACTION = CONTINUE
```

The absent parent files prevent a whole-repository conformance statement. They do not decide the narrow outcome-replay behavior already explicitly required by the accepted, present CTR-VAI-008/014. No parent authority is reconstructed from code or silently replaced with an older version.

## Plan and convergence

1. Keep the existing receipt identity `(principal_id, idempotency_key)`, request hash and wire bodies. Decode the stored business discriminator for Applied / NoOp and preserve deterministic errors as errors.
2. Complete and commit deterministic rejection before returning it. Storage/commit errors remain infrastructure/unknown observations; never report them as known zero effects.
3. Keep receipt replay before fresh version/visit classification. Reuse this transaction as the only Wake write path; retire the two incorrect HTTP-status-only replay branches rather than adding another receipt store or parallel workflow engine.
4. Add real HTTP/application/PostgreSQL checks for no-op replay, applied replay, failures, conflicts and bounded concurrency. Use a new task-local database only.
5. Bring the bounded Wake models into this repository with an explicit code mapping, fixed Lean version, audited proof output and limitations; independently review the final candidate.

The reusable Outbox retry model is repaired separately in `mayf3/primitives` at base `9500d8e24730fdbce334b8fdcde71830838c21f4`; it does not change SVC's already capped runtime retry behavior. Existing PR #141 remains a distinct, unmerged change unless later explicitly integrated and recorded.

## 修复内容与实际结果

| 原缺口 | 候选修复 | 实际观察 |
|---|---|---|
| Applied / NoOp 均为 HTTP 200，却统一解成 Applied | 按必需 `wakeApplied` 布尔值和对应字段解码 | 旧版自然 ALREADY_DUE、VERSION_MISMATCH 同键重放实际返回 503；候选保持原 200 body |
| 确定性失败完成 receipt 后直接返回，事务未提交 | 拥有 Transaction 的 helper 完成原失败回执、code-only 拒绝审计，commit 成功后返回原 error | 旧版 PG 回读 RowNotFound；候选持久化完成回执，重复请求保持状态码、错误码和原 detail |
| 非 200 回执错误解为 NoOp | 精确校验支持的 status/error code 组合 | 拒绝保持原领域错误；损坏或不匹配 payload 保持 InternalConsistency 边界 |
| Applied Wake 缺少规范要求的 attempt audit | 同一事务新增一次 WAKE_APPLIED | Applied 与同键并发均只有一次业务效果和一次原始结果审计 |

HTTP scope/direct-token/scheduler-role 入口、请求 hash envelope、公开 DTO/error taxonomy 和 migration 均保持原有契约。`acquire_receipt` 显式区分 PROCESSING / COMPLETED，并验证 command type 和请求 hash；已有回执仍在 fresh actor/Visit/version 判定之前返回。

持久失败 body 保持 `{error: code}`。`invalid_cause` 的 detail 来自已经匹配的原请求 `cause`；现有 JCS hash envelope 包含该字段。缺少 cause 时不编造 detail。模型将此请求上下文绑定列为显式前提，不声称已经证明 JCS 或 SHA-256 无碰撞。

## 数学模型与证明

完整定义、命题、前提、源码映射和复现入口在 [formal/wake](../../formal/wake/README.md)。

- `WakeReceipt`：21 条定理，描述绑定、typed roundtrip、原 NoOp 反例、一次 Applied 的版本/Event/fact 变化、NoOp 业务不变和任意有限同请求重放。
- `WakeDurability`：23 条新定理，描述确定性拒绝、code-only payload 与 cause 上下文、一个拒绝回执与一次拒绝审计、暂存/提交/回滚、读回失败及 UNKNOWN。

对已有同 scope/hash 的结果，任意有限重放轨迹保持已建模持久状态；一次成功读回只能返回原结果，读回失败允许 unavailable。首次写入引理建立该轨迹证明所用的已存回执前提。`settle` 明确建模 before/staged 两种持久结果；同一个 unavailable 观察可以对应未提交，也可以对应已提交。真实 SQL 原子性、锁调度、JSON parser、授权及完整 Rust 精化仍是单独义务。

Lean 4.19.0 在新临时目录编译：44 条显式定理、828 个安全逻辑声明和 328 个编译 theorem 均经传递公理审计，实际依赖仅 `propext` / `Quot.sound`。没有 `sorry`、自定义公理或 `native_decide`。模型作者运行后，root 又独立干净编译；另一个未参与模型编写的 Agent 做了独立语义复审。

共享 Outbox 修复在 [primitives PR #13](https://github.com/mayf3/primitives/pull/13)，精确提交 `6a0386588c767aaa998ff7552ee7364451c73da0`。其普通 retry 增加 below-cap / at-cap 分支：从 fresh kick 的任意有限自动轨迹出发，attempts ≤ 12，pending/claimed 则 attempts < 12。人工 requeue 不包含在该自动轨迹中；两个耐久通道继续保持原重试责任。这修复共享抽象缺口，SVC 既有普通 retry SQL 已执行正确的 12 次上限。

## 执行证据

证据目录：[wake-formal-20261127](../evidence/wake-formal-20261127/)。所有测试使用新建的本地 PostgreSQL 16.13 集群，Rust 为 1.100.0，Cargo.lock 不变。最终 runner 对 470 个输入做前后哈希冻结，涵盖 Rust、SQL、JSON/文本 fixture 和静态 include 宏引用的文件；先只读核验自己创建的集群，再建测试库；结束后关闭该集群。

| 验证 | 结果 | 证据 |
|---|---|---|
| 修复后 codec | 2 passed / 0 failed | [日志](../evidence/wake-formal-20261127/candidate/wake-codec.log) |
| 修复后 Visit/Activation，包括新增 8 项 Wake 回归 | 17 passed / 0 failed / 0 ignored | [回执](../evidence/wake-formal-20261127/candidate/regression-receipt.json)、[日志](../evidence/wake-formal-20261127/candidate/visit-activation.log) |
| 原始 Rust + 完全相同的新回归负对照 | 10 passed / 7 failed；进程退出 101，如实保留 FAIL | [负对照回执](../evidence/wake-formal-20261127/baseline-negative/regression-receipt.json)、[日志](../evidence/wake-formal-20261127/baseline-negative/visit-activation.log) |
| 恢复原 Spec 后的 diagnostics 测试 | 3 passed / 0 failed | [日志](../evidence/wake-formal-20261127/graph-diagnostics-unit.log) |
| Outbox PR #141 精确 head | 23 passed / 0 failed / 0 ignored | [回执](../evidence/wake-formal-20261127/outbox-pr141/regression-receipt.json)、[日志](../evidence/wake-formal-20261127/outbox-pr141/outbox.log) |
| Wake 独立 Lean 重编译 | 44 条条件模型定理通过 | [独立执行摘要](../evidence/wake-formal-20261127/independent-lean-build.json)、[完整模型审计](../../formal/wake/evidence/verification.json) |
| 原语仓库全仓 Lean gate | 134 条显式定理 / 1196 个声明 / 460 个编译 theorem | [精确提交回执](https://github.com/mayf3/primitives/blob/6a0386588c767aaa998ff7552ee7364451c73da0/lean-verification.json) |

Outbox 消费测试绑定尚未合并的 PR #141 head `6c19d318b7aa6f03ed7bbe8cfcc2b768f8c67da6`，不将它写成 main 已包含 UNKNOWN-at-cap 测试。它使用已归档的 `c2be4e…` runner；最终 Wake 使用补齐 concat include 清单的 `c82cc4…` runner。两次运行的具体源清单、runner hash 和退出状态分别保留，不追溯赋予旧运行后来增加的检查。

关键数据库回归覆盖：自然 ALREADY_DUE；VERSION_MISMATCH；真实 Transition 后 ACTIVATION_CLOSED；真实 Owner Cancel 后 INSTANCE_CLOSED；原结果在后续状态改变后重放；原拒绝持久化和 detail；同键/不同键并发；错误提交时全事务回滚。使用限定到测试 key 的 deferred constraint trigger 分别拒绝纯拒绝事务与 Applied 事务，实际回读验证 receipt/audit、版本/Event/eligibility 全部回滚。这个故障是已知数据库拒绝，**不是丢失提交确认的网络 UNKNOWN 模拟**。

Applied 所需未来 eligibility 使用显式测试 fixture，不声明生产已有 SCHEDULER_DEFER 命令。`VISIT_NOT_CURRENT` 和 `PrincipalNotFound` 保留 codec 覆盖：正常 Transition 先关闭源 activation，而正常不存在 Principal 会更早受到 receipt 外键或 HTTP gate 限制，不伪造其数据库可达性。

## 编译输入恢复与中间问题处理

现有 lib 测试引用的 diagnostics Spec 在基线缺失。本轮从已接受原提交 `39279e34ff3124f8ca6f4269f9dab737a28aed75` 逐字恢复，SHA-256 为 `81a76db7948b9388b8a489cea07aabde04c9853981d033eeacd5f8a41cb02c7e`，未制造 stub 或放松断言。接受历史、原始 Git 与重写后 main 的区别见 [恢复记录](GRAPH_DIAGNOSTICS_SPEC_RECOVERY_20261127.md)。

执行环境的不同 exec 会话不共享 loopback 服务，早期跨会话启动没有得到有效数据库结果，未计入 PASS。复现脚本因此将 PostgreSQL 与 Cargo 放入同一进程树。共用 target 的工作树还曾复用旧库产物；最终入口在编译前仅清理 svc-workflow 包自身产物，保留依赖缓存。最终正对照和基线负对照均完成了干净包构建，输入前后未变。中间失败没有被覆盖成成功，也未通过修改测试期待让修复通过。

## 独立复审及完成边界

- [实现/测试复审](../evidence/wake-formal-20261127/source-review.json)：未参与 Rust 编写的 Agent，精确源码哈希绑定，0 blockers。
- [模型语义复审](../evidence/wake-formal-20261127/model-review.json)：未参与模型编写的 Agent，44 条定理的前提、非空正例、UNKNOWN 与陈述边界，0 blockers。
- [复现入口/原规范恢复复审](../evidence/wake-formal-20261127/harness-recovery-review.json)：最终 runner hash、实际日志、输入清单与来源字节核对，0 blockers。
- [Outbox 最终提交复审](../evidence/wake-formal-20261127/outbox-final-head-review.json)：精确 `6a0386…` tree 与已验证模型/回执一致。

Product V13 / Architecture V0.4.7 的历史源码已定位，且与 main 的 pre-stash manifest 哈希一致；其接受 artifact 仍不可读取，完整 paired lifecycle 未建立，因此没有恢复为当前 canonical authority。定位证据单独保留在 [parent-authority-recovery.json](../evidence/wake-formal-20261127/parent-authority-recovery.json)。这不替代本次已存在且明确的 CTR-VAI-008/014，也不支持全仓 conformance 宣称。

本轮完成定义是一个有源码、模型、真实数据库反例/修复验证和独立复审的候选变更。最终提交身份及 PR 状态在交付 PR 中绑定；生产发布、真实网络 UNKNOWN 注入、完整 SQL 精化、消费者公平性及原业务最终完成均未由本轮证明替代。
