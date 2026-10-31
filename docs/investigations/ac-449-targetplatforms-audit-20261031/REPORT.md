# #449 targetPlatforms 创建校验核验报告

```text
INVESTIGATION_ID = INV-SW-449-TARGETPLATFORMS-20261031
REPOSITORY = mayf3/svc-workflow
SUBJECT = dsh-agent-core#449 non-production definition/creation closure
DISPOSITION = no_change / reuse
TERMINAL_STATE = STOPPED-with-EVIDENCE
REASON = 已由 PR #72 等价覆盖
REVIEW_TARGET_HEAD = ddf81196225f28bb5ffe0b8e73c8bc747aae736b
BASE_HEAD = ddf81196225f28bb5ffe0b8e73c8bc747aae736b
CURRENT_BASE_HEAD = ddf81196225f28bb5ffe0b8e73c8bc747aae736b
WORKTREE = /tmp/sw-449-fix
BRANCH = ac-449-targetplatforms-validation
IMPLEMENTATION_CHANGE = NONE
PRODUCTION_MUTATION = NONE
GITHUB_MUTATION = NONE
COMMIT / PUSH / NEW_PR = NOT_APPLICABLE (first-step no-change branch)
```

## 第一步：三个问题的结论

1. **已覆盖定义声明及创建校验的目标切片。** 对 distribution 声明 `requiredContextInputs: ["targetPlatforms"]`、且 `context_schema` 使用既有数组契约的定义，draft replacement / publish 会拒绝未 required、未 typed、允许空数组或缺少非空闭合 enum 的契约。创建事务使用该定义版本的实际 schema 验证 context；缺失、`[]`、非 enum 值返回 `422 context_validation_failed`，先于 Instance/Visit 事实写入。
2. **存在直接回归。** `corrected_v5_contract_refuses_invalid_context_and_projects_nothing` 在真实 router + PostgreSQL、仅 JWKS 为本地 mock 的 fixture 上验证上述三种输入创建被拒、同键重放仍被拒、合法值 201，且该域仅存在合法实例。被拒 context 没有 Instance，因而不可能产生 distribution Visit 或到达 `ACTIONABLE_NOW`。
3. **#72 已作等价覆盖，且不只是泛化的 schema 测试。** #72 同时加入了声明/schema 一致性规则和该 #449 专用创建拒绝回归。当前 7 个测试全部复跑通过；测试文件相对 #72 merge 无差异。因此不进入第二步，不新增实现、测试、提交或 PR。

**结论边界：** 这是已有“Definition 自有声明 + schema enum registry”契约的覆盖，不是引擎按 `distribution`/`targetPlatforms` 名称无条件硬编码业务规则。没有 `requiredContextInputs` 声明的历史定义不会被此规则自动改写，旧 published schema 也不会被追溯修改。registry 是定义版本 `items.enum` 中冻结的集合，不是另一个真实服务。此核验未读取任何真实 svc 端点，生产已装载定义、creator cutover、旧实例恢复和业务完成状态均为 **NOT_VERIFIED**。

## 坐标、授权与治理路由

- 2026-10-31 现场 `git ls-remote origin refs/heads/main` 回读为 `ddf81196225f28bb5ffe0b8e73c8bc747aae736b`，与 HEAD、本地 origin/main 相同，见 [coordinates.txt](coordinates.txt)。未更新其他 checkout 或 ref。
- [.agents/governance.lock.json](../../../.agents/governance.lock.json) 为 accepted（mayf3，2026-09-19）；该文件最近提交 `f4a591af81c2f40f330d8254fc2aaa9700270eb7` 已在当前 main 的历史中。无 bootstrap candidate 被当成实现授权。
- 当前 Product Authority：`docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V13.md:1092,1107`，Definition 拥有 Context schema，发布冻结 schema；Architecture `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_4_7.md:1147-1149`，内核验证 shape/digest，不判断业务真值。
- 读取 governing Specs：`SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3`（accepted，`implementation_authority: contracts`，`:54-68` 保留既有 graph/schema、运行时边界）、`SVC_WORKFLOW_DEFINITION_GRAPH_DIAGNOSTICS_V1`（既有 graph-validation 诊断范围）。本报告不创建产品权威，不以代码、测试或 PR 描述覆盖产品契约，也不据此授权新实现。
- 用户本次 mandate：仅当前隔离 worktree；先核验、缺口才实现；零生产写入；不 install/restart；测试只允许回环 fixture；无缺口即报告并 STOP。

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
ROUTE_STAGE = IMPLEMENTATION (requested route, no-change investigation outcome)
AUTHORITY_ACTION = REUSE
PRIMARY_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V13 @ BASE_HEAD
RELATED_AUTHORITIES = SVC_WORKFLOW_ARCHITECTURE_V0_4_7; SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3; SVC_WORKFLOW_DEFINITION_GRAPH_DIAGNOSTICS_V1
AUTHORITY_ACCEPTED_IN_BASE = YES
IMPLEMENTATION_AUTHORITY = not_applicable (no implementation performed)
GOAL_OR_TARGET = verify required/non-empty/registry-backed creation gate and direct no-visit regressions
CURRENT_GAP = no remaining source/test gap for the declared corrected contract
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = ROUTINE (source inspection and isolated local fixture)
MUTATION_AUTHORIZATION = VALID (local evidence and test fixture only)
ISOLATED_WRITE_SURFACE = YES
SPEC_GAP_DEPENDENCY = NONE for this no-change investigation
IMPLEMENTATION_ALLOWED = NOT_APPLICABLE
MERGE_READY = NOT_APPLICABLE
OPERATION_ALLOWED = NO production operation
EVIDENCE_REVIEWABILITY = PASS (source, command, rc, raw local logs)
DONE_WHEN = three audit answers + exact-source rerun + durable report
EXPANSION_TRIGGER = semantic failure, relevant base drift or required business-contract change
NEXT_ACTION = STOP
```

## 代码与测试证据

所有路径和行号均指向 `BASE_HEAD`，没有引用未合并候选代码。

| 核验点 | 文件:行号 | 证据含义 |
|---|---|---|
| 既有 draft / publish 校验面 | `src/application/definition/draft_graph.rs:211-231`; `src/application/definition/lifecycle/publish.rs:80-94` | 三种 semantic model 的 graph 分派后均进入共同 schema pass |
| 声明规则被调用 | `src/application/definition/lifecycle/validation.rs:151-156` | `validate_declared_context_inputs` 在共同 pass 内 |
| required / typed | 同文件 `:213-244` | 声明键必须位于 schema.required 并有 properties.type |
| 非空数组 | 同文件 `:246-261` | array 类型要求整数 `minItems >= 1` |
| 每个元素受 registry 约束 | 同文件 `:264-277`; `tests/38_declared_context_inputs_v5_distribution.rs:134-149` | 非空 `items.enum`；该专用定义为 array + minItems 1 + 平台 enum |
| 创建时实际校验 | `src/store/postgres/workflow_instance_repository/create_transaction/facts.rs:159-162`; `src/store/postgres/workflow_instance_repository/validation_helpers.rs:220-239` | jsonschema 验证实际 context，失败是既有 ContextValidationFailed |
| 先拒绝再写实例 | `src/store/postgres/workflow_instance_repository/create_transaction/facts.rs:243-272` | Instance INSERT 晚于 schema gate |
| 错误风格 | `src/store/postgres/workflow_instance_repository/validation_helpers.rs:342,361` | 422 / context_validation_failed |
| 缺失 / 空 / 非 registry 直接回归 | `tests/38_declared_context_inputs_v5_distribution.rs:609,627-664` | 三种 context 创建及重放均为 422 |
| 合法值正控 | 同文件 `:667-680` | `['掘金','知乎']` 创建 201 |
| 不能出现无效 Visit | 同文件 `:683-715` | 域中精确只有合法实例，拒绝输入没有实例载体 |
| worklist 阴性 | 同文件 `:718-728` | 被拒输入不出现在 assignee 工作清单 |
| SMV3 声明校验 | 同文件 `:514-596` | `declared_input_rule_covers_visit_activation_model` 验证 model3 也经过声明规则 |

**避免过度解读：** `tests/38...:734-750` 的 feed 断言实际仅过滤合法实例 ID，不能单独充当全部被拒 context 的 dispatch feed 阴性证明。本结论依赖更早的三次创建 422 + 同域精确一实例 + 写入前 schema gate 的联合证据，满足任务明确允许的“创建被拒或不可达 ACTIONABLE_NOW”。没有声称额外执行一次缺失 context 的真实 distribution transition。

现有 7 个测试名与定位：

- `declared_input_not_required_by_schema_fails_publish` — `tests/38...:412`
- `declared_input_without_schema_type_fails_publish` — `:434`
- `declared_array_input_allowing_empty_fails_publish` — `:456`
- `declared_array_input_with_unbounded_values_fails_publish` — `:478`
- `malformed_declared_inputs_fail_publish` — `:501`
- `declared_input_rule_covers_visit_activation_model` — `:514`
- `corrected_v5_contract_refuses_invalid_context_and_projects_nothing` — `:609`

前五个测试共用 `assert_rejected_at_draft_and_unpublishable` (`:369-408`)，既断言 draft rejection 的 422 和具体诊断代码，也断言后续 publish 仍 422。

## #72 的精确归属与历史回执

- [svc-workflow PR #72](https://github.com/mayf3/svc-workflow/pull/72)：GitHub 现场读回 merged=true；head `6b316a7438602d22ab02bdfc659bfcb24de2911c`；merge `7c533cfc2676ea970d502969662dff3bbdd3e66c`（2026-10-04）。原始结构化读回见 [github-pr72.json](github-pr72.json)。本地 `git merge-base --is-ancestor` 为 rc=0；测试文件从该 merge 到当前 main 的 `git diff --exit-code` 为 rc=0、输出为空。
- [#449 的 r2 收尾证据](https://github.com/mayf3/dsh-agent-core/issues/449#issuecomment-5980771242) 已明确记录这套机制、7/7 测试、创建拒绝和 no-ACTIONABLE_NOW 语义。它是历史执行证据，不是本次产品授权。
- [后续 publish packet 回执](https://github.com/mayf3/dsh-agent-core/issues/449#issuecomment-5981870660) 和 [后续生产历史回执](https://github.com/mayf3/dsh-agent-core/issues/449#issuecomment-5982395513) 属于其他任务阶段；本次只读其 GitHub 记录，未访问对应生产环境，也未将历史状态当成当前生产事实。选定回执原文见 [github-449-receipts.json](github-449-receipts.json)。

## 本次复跑：原文、rc 与 fixture

环境：macOS 本地 Rust cargo 1.98.1，PostgreSQL 16.15；全新 `initdb` cluster，独立 DB `sw449_targetplatforms`，仅 `127.0.0.1:64370`，身份为本次 fixture 的 `sw449_fixture`。显式覆盖 `TEST_DATABASE_URL` 和 `DATABASE_URL`，没有使用 `tests/common/mod.rs:310` 的默认数据库。应用及 mock JWKS 也仅 bind `127.0.0.1:0`（suite `:106`、common `:46`），测试 client `no_proxy`。

Cargo 使用 `--offline --locked` 且 `CARGO_NET_OFFLINE=true`，依赖和 lock 无改动，无 npm install，无外部网络测试。构建输出含两个既有 unused-import warning（common 的 TcpListener、AsyncWriteExt），没有失败。没有环境重试或语义失败。

实际命令：

```text
cargo test --offline --locked --test 38_declared_context_inputs_v5_distribution -- --nocapture --test-threads=1
```

执行开始：2026-10-31T15:57:14.272957+00:00；结束：2026-10-31T15:58:17.531027+00:00。下列为 raw log 中测试结果至退出标记的原文摘录；完整编译/测试 stdout+stderr 见 [focused-rerun.txt](focused-rerun.txt)，机器可读坐标/rc 见 [focused-receipt.json](focused-receipt.json)。

```text
running 7 tests
test corrected_v5_contract_refuses_invalid_context_and_projects_nothing ... ok
test declared_array_input_allowing_empty_fails_publish ... ok
test declared_array_input_with_unbounded_values_fails_publish ... ok
test declared_input_not_required_by_schema_fails_publish ... ok
test declared_input_rule_covers_visit_activation_model ... ok
test declared_input_without_schema_type_fails_publish ... ok
test malformed_declared_inputs_fail_publish ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.28s


RC=0
FINISHED_AT=2026-10-31T15:58:17.531027+00:00
```

该次 focused 套件原数：**7 passed / 0 failed / 0 ignored / 0 measured / 0 filtered out，rc=0**。本次不进入实现分支，因此 RED→GREEN、受影响套件全量、commit/push/new PR 为 NOT_APPLICABLE；没有把 #72 历史 RED 或全量结果冒充本次执行。

本地 fixture 的 [setup 原文](fixture-setup.txt) 三步均 rc=0；[shutdown 原文](fixture-shutdown.txt) 为 rc=0，`postmaster.pid` 已不存在。没有 install/restart 任何既有服务，仅启动并停止本次授权的临时测试 fixture。

## 独立核验与交付状态

独立只读代理 `/root/audit_72` 对当前 exact Head 的调用链、7 个测试和 fixture 边界做了复核；结论为该声明式切片已由 #72 覆盖，并提示上文 feed 断言的真实限制。随后对报告、执行原文、receipt 和坐标复核为 **PASS**，详见 [独立复核原文](INDEPENDENT_REVIEW.md)。独立代理没有写文件、运行测试或连接 svc；本次测试由 root 执行。远端 main 仅绑定 coordinates.txt 所记观察时点，未声称其后永不移动。

代码、测试、依赖、治理文件均未改。`git diff --raw HEAD` 与 `git diff --numstat HEAD` 为空，HEAD 不变。仅新增本地未提交的本调查目录及被 gitignore 的 target 测试产物；没有 commit、push、PR、merge、installed 文件变更或其他仓写入。

**终态：STOPPED-with-EVIDENCE — 已由 PR #72 等价覆盖。** 如未来出现声明正确但上述任一种 context 仍创建成功的精确反例，或接受新的跨定义业务强制契约，应重新 PREFLIGHT；本次不扩展为全局业务规则或生产操作。
