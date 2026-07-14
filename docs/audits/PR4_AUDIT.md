# PR 4 独立审计报告

```text
Slice: PR 4 — Read Model / Query Service
Audit role: independent audit agent
Initial audited branch: codex/workflow-query-service-v0
Initial audited HEAD: b527ba541558b875490e5433ef62d41973660ca0
Base: 3a0ec2282d0095da85a127b8fe7f057cf73966a8
Audit date: 2026-07-15 (Asia/Shanghai)
Initial verdict: FAIL_HIGH
Final re-audited HEAD: 9b883ccb11700c927a6906cbca5eb2bf268d37db
Final verdict: PASS_WITH_NOTES
```

## 1. 初审结论

固定 SHA `b527ba5` 不能合并。实现主体覆盖了 7 个查询、四级可见性、masked error、
SecurityAudit、keyset pagination、`REPEATABLE READ`、worklist 和 PostgreSQL 单一权威源；
格式、构建、Clippy、PR 4 定向测试和串行全量测试均通过。

但本轮确认 4 个 High：

1. 新增测试不具备默认并行隔离，固定验收门在第 1 轮即失败；
2. historical 可见性可由未经版本一致性校验的历史 Visit/Submission 事实授予，且事实守卫只校验当前行或当前页；
3. 查询会把非 primary 的 `ADVANCE` 标为可执行，但写命令必定拒绝；
4. 仍有一个 Transition fault-injection 测试不满足固定的 RAII/独立连接清理规范。

此外，Context Revision 单链没有任何查询侧防御校验；该问题与第 2 项同属全事实一致性
缺口，修复时必须一起关闭，不能只修可见性 SQL。

## 2. 初审 High findings

### H1 — 默认并行测试使用全局 Audit 计数，验收门不稳定

位置：`tests/17_workflow_runtime/query/guards.rs:122-142`

测试 `successful_queries_are_read_only_and_rejections_only_add_security_audit` 在拒绝前后执行：

```sql
SELECT COUNT(*) FROM workflow_security_audits
```

它没有按本测试唯一的 `seed.outsider`、instance 或 query type 隔离。默认并行运行时，其他
测试在两个计数之间写入 Audit，实际失败为：

```text
query_guards::successful_queries_are_read_only_and_rejections_only_add_security_audit
left: 76
right: 75
runtime result: 178 passed; 1 failed
```

因此 `cargo test` 第 1 轮失败，无法形成要求的连续 5 次通过。

可执行修复：

- before/after 计数至少限定 `principal_id = seed.outsider`；
- 最好同时限定本实例 `resource_id` 与 `details->>'queryType' = 'ListNodeVisits'`；
- 修复后从第 1 轮重新执行 5 次默认并行全量测试。

### H2 — 未校验的历史事实可授予可见性，全事实一致性守卫也可被分页绕过

位置：

- `src/store/postgres/workflow_instance_repository/query_visibility.rs:210-223`
- `src/store/postgres/workflow_instance_repository/query_visibility.rs:226-265`
- `src/store/postgres/workflow_instance_repository/query_visibility.rs:268-297`
- `src/store/postgres/workflow_instance_repository/query_detail.rs:398-439`
- `src/store/postgres/workflow_instance_repository/query_worklists.rs:70-104`

`classify_visibility` 直接用以下两类事实授予 historical visibility：

```sql
EXISTS (SELECT 1 FROM workflow_node_visits
        WHERE workflow_instance_id = $1 AND assignee_principal_id = $2)
OR EXISTS (SELECT 1 FROM workflow_submissions
           WHERE workflow_instance_id = $1 AND author_principal_id = $2)
```

这里没有验证历史 Visit 的 Node 属于实例绑定 Definition Version，也没有验证 Submission 的
source Visit/Node、Context、Transition 属于同一实例/版本。数据库约束能保证部分同实例外键，
但不能保证 Visit Node 或 Submission Transition 属于实例版本。因此一个跨版本的腐败历史
Visit/Submission 可以先给 actor 授予 restricted detail 可见性。

`validate_base` 只校验 current Context、current Visit 和全量 Event 摘要，不校验全部历史 Visit、
Submission 或 Context chain。`validate_submission_rows` 只在 Submission history 当前
`limit + 1` 行以及 assigned worklist 最近 51 行上执行；页外腐败事实不会阻断前一页 DTO。
`GetWorkflowInstanceDetail`、Timeline、Context history、Visit history 也不会执行全实例
Submission 守卫。这违反 Query 合同第 8 节“在返回任何 DTO 前”校验事实关系且“不得返回部分
DTO”的要求，并把一致性缺口升级成可见性边界问题。

同一修复必须补齐冻结架构 §7.3 的 Context Revision 单链：

- Revision #1 的 `previousRevisionId` 必须为 null；
- 后续 Revision 必须指向紧邻前一 Revision；
- revision number 必须从 1 连续；
- current Context pointer 必须指向链头。

当前 `list_context_revisions` 只是按 revision number 分页返回原始行，`validate_base` 也只证明
current pointer 指向某个完整同实例 fact，不能识别断链、跳链或指向旧 head。

可执行修复：

- 在同一 `REPEATABLE READ` 事务内增加全实例 Context/Visit/Submission consistency summary；
- historical Visit 参与条件只接受 Node 属于实例 Definition Version 的 Visit；
- historical Submission 参与条件只接受 source Visit/Node、Context、Transition 全部一致的事实；
- 在任何 DTO 分页前完成全事实守卫，不能仅校验返回页；
- Submission 还应验证 Transition source node 与 source Visit node 相同；
- 增加跨版本 Visit 获权、跨版本 Submission 获权、腐败事实位于下一页、Context 断链/跳链/
  stale head 的回归测试；不可见 actor 仍须保持 masked error，不得通过 consistency error 探测实例。

### H3 — 非 primary ADVANCE 被错误标记为 executable

位置：

- `src/store/postgres/workflow_instance_repository/query_detail.rs:25-114`
- `src/store/postgres/workflow_instance_repository/transition_transaction.rs:228-242`
- `src/domain/definition/graph/transition_validation.rs:78-116`
- 冻结架构 §10.1/§10.2；Transition 合同 §4

`load_outgoing` 对所有 current Node 出边只判断 terminal、Definition status、actor 和 target
assignee availability；只要这些条件通过，就设置：

```text
executableForActor = true
blockedReason = null
```

它没有读取/比较 source Node 的 `primary_advance_transition_id`。写侧明确要求 `ADVANCE.id ==
source.primary_advance_transition_id`，否则返回 `TransitionNotApplicable`。Definition validator
只验证被 primary 指针引用的 Transition 是 ADVANCE，并未禁止同一 Node 还有额外的非 primary
ADVANCE。因此当前可发布数据能触发“查询称可执行、命令必拒绝”的稳定矛盾，直接破坏 Agent
worklist 的可执行性承诺。

可执行修复：

- 查询加载 source Node 的 primary transition；
- 对非 primary ADVANCE 绝不能返回 `executableForActor = true`；
- 在 Query 合同中冻结相应 blocked reason（或在 Definition 层禁止额外 ADVANCE，同时保留读取
  既存防御数据时的稳定阻断）；
- 增加 published graph 含额外 ADVANCE 的 query/execute 对照回归。

### H4 — 一个既有 Transition fault injection 没有 RAII 和独立连接清理

位置：`tests/17_workflow_runtime/transition/atomicity.rs:243-309`

`test_transition_instance_update_failure_rolls_back` 虽然使用唯一 UUID 名称和 instance 条件，
但手工创建 trigger/function，并在测试主体尾部使用共享 pool 显式 DROP。若命令或断言 panic，
清理不会执行；清理也不是独立连接。它不满足本轮固定的“唯一 UUID + 条件 + RAII + 独立连接
清理”规则。

本 PR 修改了同一文件中 Submission fault trigger 的错误列名：

```text
created_by_principal_id -> author_principal_id
```

该修复正确，串行测试实际触发并通过；但同文件剩余的手工 DDL 不能继续遗留。

可执行修复：扩展/复用该文件的 `TriggerGuard`，支持对普通 table 的 `BEFORE UPDATE`，由 Drop
在独立线程/runtime/fresh `PgConnection` 上删除 trigger 和 function，并保留 instance 条件。

## 3. 初审 Medium / Low notes

### M1 — 两个 history truncation flag 只直接覆盖了一个 true 分支

实现分别计算 `submissions_truncated` 与 `return_events_truncated`，方向正确；现有测试只证明
52 条 Submission 时前者为 true、26 条 RETURN Event 时后者为 false。应增加超过 50 条 RETURN
Event 的测试，证明两个集合互不串扰且后者能独立为 true。

### M2 — pagination 失败矩阵和 tie-breaker 覆盖不完整

SQL 均使用 keyset 且静态检查正确；但 25 个聚合测试没有逐一覆盖 6 个分页查询的 zero/max+1、
两个负数 cursor，以及 assigned/creator 相同 timestamp 的 DESC UUID tie-breaker。现有测试覆盖了
Timeline、worklist max、Visit/Submission tie 和两类 worklist 翻页，遗漏项宜补成表驱动测试。

### M3 — Transition 并发快照测试没有强制建立中间阻塞点

Revision 并发测试通过 table lock 明确证明了同一 `REPEATABLE READ` 快照；Transition 测试用 10
次 barrier race，只证明观察结果是 before/after，没有强制 query 在 base read 后阻塞再提交
Transition。实现统一走 `begin_snapshot`，静态上正确，但建议像 Revision 测试一样加确定性交错。

### L1 — SecurityAudit 失败只直接覆盖 unauthorized 路径

disabled principal 与 restricted Context history 共用同一 `audit_security`，代码路径正确；现有
fault injection 只直接证明 unauthorized audit 失败回滚且不放行。可增加 disabled/restricted
参数化用例，防止未来分叉。

## 4. 初审通过的合同核对

- 7 个公开查询及冻结错误集合均存在；未创建缓存、read-model table 或第二权威实体。
- 每个查询先开启事务，并在首次读取前执行 `SET TRANSACTION ISOLATION LEVEL REPEATABLE READ`；
  查询代码没有 `FOR UPDATE`。
- Principal missing/disabled、masked nonexistent/invisible、owner replacement、domain disabled
  历史读取、current assignee、creator-on-runtime-DRAFT 与 historical restricted 主路径实现正确。
- Historical detail DTO 不含 creator、current pointers、external reference/URL、metadata、Context
  payload、current assignee、instructions 或 outgoing schema。
- restricted Timeline/Submission 对 `relatedSubmissionIds` 使用 JSON array element 精确匹配，
  并限定 own/feedback 同实例；跨实例 UUID 回归通过。
- assigned-to-me 只使用 current Visit pointer 且排除 TERMINAL；creator drafts 使用 runtime Node
  type DRAFT，而非 Definition status。
- PUBLISHED/DEPRECATED、REVOKED、防御性 DRAFT 和三类 target assignee availability 的主体逻辑
  已实现；H3 是额外 ADVANCE 的独立缺口。
- Timeline/Context/Visit/Submission/worklist 均为 keyset pagination，排序和 `limit + 1` 主体实现
  正确；两个 truncation flag 已分离。
- 成功查询静态代码只有 SELECT；拒绝路径唯一写 `workflow_security_audits`。Audit 写失败返回
  `StorageError`，不返回 DTO。
- Event sequence/count/reference 的全量 summary、current pointer 同实例以及 current Node version
  主体守卫存在；H2 列出未覆盖的历史全事实边界。
- PR 4 没有 Migration 变更，也没有外部索引或 read-model 表。
- Query fault guard 使用唯一 UUID、principal 条件、RAII Drop 和独立连接；combined/create/context
  guards 同样符合。H4 是唯一核出的例外。

## 5. 初审实际门禁证据

环境与结构：

```text
PostgreSQL: 16.14 (Homebrew)
Migration diff (base...HEAD): 0
Residual test triggers/functions before run: 0 / 0
Residual test triggers/functions after failed parallel run: 0 / 0
Largest handwritten Rust file: 493 lines
src/store/postgres/workflow_instance_repository direct children: 20
tests direct children: 20
max src/tests file depth: 4
git diff --check: PASS
```

构建与测试：

```text
cargo fmt --check                                      PASS
cargo build                                            PASS
cargo clippy --all-targets --all-features -- -D warnings PASS
cargo test --test 17_workflow_runtime query_ -- --test-threads=1
                                                       PASS 25/25
cargo test -- --test-threads=1                         PASS 351/351
  lib                                                  54
  integration                                          297
  tests/17_workflow_runtime                            179 (其中 PR 4 = 25)
cargo test -- --list                                   PASS, 351 tests
cargo test (default parallel), required 5 consecutive
  attempt 1                                            FAIL, 350/351 overall
  failing runtime test                                 178/179
  attempts 2-5                                         NOT RUN (gate already failed)
```

初次审计开始及写报告前，固定实现 SHA 均为 `b527ba5`；报告之外未修改实现、测试、合同、
Migration 或 Git 历史。

## 6. 初审设定的 Re-audit gate

只有以下条件全部满足才可改为 `PASS` / `PASS_WITH_NOTES`：

1. H1-H4 全部修复并有针对性回归；
2. historical Visit/Submission visibility 与 Context 全链守卫按 H2 做独立负向测试；
3. 非 primary ADVANCE 的 Query/Execute 对照不再矛盾；
4. 所有 fault-injection DDL 具备条件隔离、RAII 和独立连接清理；
5. PR 4 定向、全量串行、test list、结构/DDL/Migration 检查全部通过；
6. 默认并行全量测试从零重新连续通过 5 次；
7. re-audit 固定新的 HEAD，并把结果追加到本报告。

## 7. 独立复审

```text
Re-audit branch: codex/workflow-query-service-v0
Re-audit base: 3a0ec2282d0095da85a127b8fe7f057cf73966a8
Re-audit HEAD: 9b883ccb11700c927a6906cbca5eb2bf268d37db
Re-audit date: 2026-07-15 (Asia/Shanghai)
Verdict: PASS_WITH_NOTES
Blocker: 0
High: 0
```

复审固定在 `9b883cc`，没有信任实施自报。逐项复查初审全部 High、修复增量
`b527ba5..9b883cc`、合同变化、负向测试、SQL NULL/空集语义、masked error 和全部门禁后，
未发现残余 Blocker/High。PR 4 可以进入审计报告提交和 fast-forward merge 流程。

### 7.1 High closure

#### H1 — CLOSED

SecurityAudit before/after 计数现在同时限定：

```text
principal_id = seed.outsider
resource_id = 本测试 workflowInstanceId
details.queryType = ListNodeVisits
```

复审默认并行全量测试从 0 开始连续通过 5 次，初审的全局计数竞态不再复现。

#### H2 — CLOSED

实例级查询在授权分类后、返回 DTO 前执行 `validate_base + validate_all_facts`；两个 worklist
也在构建每个选中候选的 DTO 前执行相同的全事实守卫。守卫现在覆盖：

- Context revision number 从 1 连续，Revision #1 previous 为 null，后续 previous 紧邻前一条，
  current pointer 指向最大 revision；
- 全部历史 Visit 的 Node 属于实例绑定 Definition Version；
- 全部 Submission 的 source Visit/Node、Context、Transition 属于同实例/版本，且 Transition
  source Node 等于 source Visit Node；
- 守卫在独立 history 分页和 assigned 最近 51 条截断之前执行，页外腐败不能返回前一页 DTO。

SQL 语义复核：Context 空集为 false；合法单条 Revision #1 与合法多条链为 true；gap、断链、
跳链、stale head 为 false。Visit 空集的聚合默认 true，但 `validate_base` 已要求完整 current
Visit，因此整体不会误放行；Submission 空集按合同是合法的 true。两个 LEFT JOIN 守卫均使用
`BOOL_AND(COALESCE(predicate, FALSE))`，缺失关联行 fail-closed，不会被 PostgreSQL `BOOL_AND`
忽略 NULL。

可见性复核：historical Visit/Submission 的 EXISTS 均先 join 并验证版本/关系；current assignee
分类还要求 current Visit 属于本实例且 Node 属于实例版本。因此跨实例/跨版本损坏 Visit 的
assignee 仍收到 masked `WorkflowInstanceNotFoundOrNotVisible`，不会通过 `InternalConsistency`
探测实例；合法 Domain Owner 仍收到一致性错误以便发现损坏。新增负向回归覆盖了 corrupt
historical Visit、corrupt current Visit、corrupt Submission、两个 worklist、Context 断链和
stale head。

#### H3 — CLOSED

Query 合同和 DTO 新增稳定的 `ADVANCE_NOT_PRIMARY`。查询读取 current Node 的
`primary_advance_transition_id`，额外非-primary ADVANCE 返回：

```text
executableForActor = false
blockedReason = ADVANCE_NOT_PRIMARY
```

`ListCreatorOwnedDrafts.combinedExecutable` 也只认 primary ADVANCE。回归测试在同一 fixture 中
证明 Query 阻断额外 ADVANCE，而 Execute command 返回 `TransitionNotApplicable`；查询不再声称
该 Transition 可执行。

#### H4 — CLOSED

Transition instance UPDATE fault injection 已复用 `TriggerGuard::install_table_operation`；名称带
唯一 UUID、触发条件限定本测试 instance、Drop 使用独立线程/runtime/fresh `PgConnection`
删除 trigger/function。原 Submission fault trigger 使用 `author_principal_id`，实际 fault 测试
通过。全量串行和 5 轮默认并行结束后，残留测试 trigger/function 均为 `0 / 0`。

### 7.2 Final gate evidence

```text
PostgreSQL                                             16.14 (Homebrew)
cargo fmt --check                                     PASS
cargo build                                           PASS
cargo clippy --all-targets --all-features -- -D warnings
                                                       PASS
cargo test --test 17_workflow_runtime query_ -- --test-threads=1
                                                       PASS 27/27
cargo test -- --test-threads=1                        PASS 353/353
  lib                                                 54
  integration                                         299
  tests/17_workflow_runtime                           181 (PR 4 = 27)
cargo test -- --list                                  PASS, 353 tests
cargo test (default parallel) run 1                   PASS 353/353
cargo test (default parallel) run 2                   PASS 353/353
cargo test (default parallel) run 3                   PASS 353/353
cargo test (default parallel) run 4                   PASS 353/353
cargo test (default parallel) run 5                   PASS 353/353
Migration diff (base...HEAD)                          0
git diff --check (base...HEAD)                        PASS
Residual test triggers/functions before gates        0 / 0
Residual test triggers/functions after gates         0 / 0
Largest handwritten Rust file                        493 lines
workflow_instance_repository direct children         20
tests direct children                                 20
max src/tests file depth                              4
```

测试总数变化明确为：初审固定 SHA `b527ba5` 为 `351`（PR 4 `25`）；最终复审固定 SHA
`9b883cc` 为 `353`（PR 4 `27`）。新增两个聚合测试分别覆盖全事实/可见性防御和 Context chain，
并在同一测试中包含 worklist 与 corrupt-current masked 分支。

复审结束时：

```text
HEAD = 9b883ccb11700c927a6906cbca5eb2bf268d37db
tracked implementation/contract/test changes = clean
only untracked audit artifact = docs/audits/PR4_AUDIT.md
```

### 7.3 Non-blocking notes retained

以下初审 Medium/Low 不构成合并阻断，继续保留为后续测试强化项：

- M1：增加 `returnEventsTruncated = true` 的独立 >50 RETURN Event 覆盖；
- M2：把所有分页 zero/max+1/negative cursor 和两个 DESC 相同 timestamp tie-breaker 补成表驱动矩阵；
- M3：把 Transition/query 并发测试改成像 Revision 测试一样的确定性中间阻塞交错；
- L1：参数化覆盖 disabled principal 与 restricted Context history 的 Audit 写失败。

这些项不改变当前代码路径的合同结论：两个 truncation flag 已独立计算，keyset SQL 使用稳定
UUID tie-breaker，所有查询统一使用 `REPEATABLE READ`，全部拒绝 Audit 共用 fail-closed 写入函数。
