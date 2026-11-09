# #1344 r6 — A/B、SDK schema 现势对齐与 authorized_actions

结果：本切片源码及隔离验证完成；独立 affected-Contract/ACC-AA-001..007 审查 PASS，无 Blocker。既有回归保留四个已定案的 #1348 失败，未把它们报成全套 GREEN。

## 坐标与接受权威

| 项目 | 实际坐标 |
|---|---|
| repository / branch | mayf3/svc-workflow / ac-d6d11-impl-r4 |
| implementation base / fresh remote main | `b32c0d1a82abeafbf993c5638bbc9f55e436d9a1` |
| preserved A/B commit | `d9ebad8db87356eac128e1858ee8f8a2ba1eda73` |
| bundle delivery A | `43daefb11e12b00f3b27097a7c89aa7de5c74de0` |
| implementation B | `1b62c077c9f4e7883ab1ef18c50ce191702e374c` |
| implementation tree | `193165f67f01c6b7c71eb7e208c1deab1a2e188f` |
| accepted AA semantic head | `69ce065c00082c01c3dc03defc72e46a7d0d18a1` |
| accepted AA merge in base | `108a47a5eb3c3fb0aee7ee548e297fd65b134013` |
| AA Spec SHA256 | `bd49f9ddceaa5d78ff5246d0782197e7530929ae54befa61be824ff316251e43` |

路线为 REUSE / IMPLEMENTATION / EXEC_PLAN / CONTROLLED。治理完整性检查通过，accepted adoption 与 AA merge 均已在 base。沿用 [PR114 Owner exact-head 接受](https://github.com/mayf3/svc-workflow/pull/114#issuecomment-6552672226)；仓内 proposed/PENDING 生命周期元数据漂移已被 accepted `WORKFLOW_WRITE_PATH_ENABLED_ENFORCEMENT_V1` 明确处置，本 PR 未改写 authority。A 是本 PR 候选 bundle 交付提交，尚未合入 main。最终 PR head 的证据提交保持 B 的全部产品及测试 blob 不变。

## A — SDK contract lock

保留 d9ebad8d 已通过修复：SDK 原 1.5.0 元数据先对齐既有 1.8.0 manifest，并通过原测试后续 digest/Owner-head 断言。本次 accepted AA bundle 为 1.9.0，最终 SDK version/digest/Owner SHA 与该 bundle 一致。`CONTRACT_MAINLINE_HEAD_SHA` 指向真实 A；独立逐项核验全部 11 个 bundle_file_set 文件在 A/B 字节相同。

## B — create executionClass

保留 accepted WEC 的可选闭枚举 `BUSINESS | NON_BUSINESS_TEST`。省略不注入默认值；未知枚举、错误类型和未知请求成员仍拒绝。原 A/B 测试文件相对 d9ebad8d 无后续改动，最终完整 SDK suite 覆盖这些原断言。

## SDK current_visit schema 现势对齐

只增加实际响应已有的 `assignee_canonical_agent_id`、`eligibility`，后者复用既有闭合 eligibility schema；current_visit 与嵌套 eligibility 的 `.strict()` 保留。真实 frozen detail 在修复前因这两键产生 unrecognized_keys RED，修复后四项通过，包括真正未知 visit 字段和未知 eligibility 字段/分类的拒绝。

真实 fixture 原值保留于 `sdk/typescript/tests/fixtures/current-detail.actual.json`，仅作 JSON pretty-print；旁侧 provenance 记录原 capture、JSON pointer 与哈希。fixture SHA256：`90c51866455cea0960fcf4cac3d6ed34f36586f3e9acd040d1e7ab47ede408f0`。

数据校验另验证 200 个实际 HTTP readback 文件哈希，并对 74 个捕获对象 safeParse/deepStrictEqual：67 个 endpoint detail、3 个原 full envelope、2 个原 restricted、2 个 shared six-field。实际 WAITING_FOR_TIME、canonical agent 字符串、terminal、restricted 均通过。bare detail 与原 envelope 分别记录，未补造缺失原包。

## authorized_actions

完成 endpoint 专属十字段 Full；四个新增必填字段为 authorized_actions_scope/status/actions 和 current_task_type。闭合动作类为 transition（ADVANCE/RETURN/TERMINATE）、request/escalate/resolve Assistance、cancel、archive。current_task_type 来源为实际 node_type，business_task_type 为 null；Minimal2 保持 UNSUPPORTED_MODEL。

同一授权 REPEATABLE READ 快照复用各实际写入口的独立身份/状态判定；真实执行继续检验 payload、ACB、RETURN 引用、CAS、key/replay。shared Full 六字段、旧 outgoing 诊断、restricted、worklists、CTI body/digest 保留。冻结 RED 与 GREEN、同取消实例/同共享持久 fixture、真实允许/拒绝及 receipt/Event/version/activation/outbox 不变量均保留原始证据。

23 个探针通过：原 19 项及四个补充 DRAFT、case/RETURN 并发、实际未来时间资格、retired cleanup actor 用例。独立审查核验 65 次 projection read 和 58 份有请求/响应的实际 writer 记录；ACC-AA-001..007 具体映射在证据包 `r6/review/FINAL_HEAD_REVIEW.json`。

## 验证结果与既有失败列名

- B 的 `cargo check --locked`：rc0。
- 最终 `npm run test:sdk`：42 passed / 1 既有 real-process integration skipped；`npm run build:sdk`：rc0。该 skipped 用例未获得独立服务与凭据环境，未冒称实际 SDK TCP 生命周期验收。
- bundle digests：rc0；schema=`6fa75957e020692c8a9b80cc7c6c7bbe0f8d3dd0733be84074b9dbf53c7d4e5f`；bundle=`99096b6b5a2f91020c060862e2f3a065344848d6368c1305da78e2544c5f4b0e`。
- 14 个既有 targets、945 个原名全部逐名实际调用：940 passed / 4 既有失败 / 1 既有 ignored。原 193 个测试/构建文件对 exact base 逐字节一致；逐 target/global 测试名称 diff=∅，新 23 个 probe 单列。

| target | 原名数 | passed | 既有 failed | 既有 ignored |
|---|---:|---:|---:|---:|
| `lib` | 253 | 248 | 4 | 1 |
| `17_workflow_runtime` | 452 | 452 | 0 | 0 |
| `18_self_projection_and_domain_members` | 25 | 25 | 0 | 0 |
| `21_instance_cancel_archive` | 27 | 27 | 0 | 0 |
| `28_visit_activation_v1` | 9 | 9 | 0 | 0 |
| `33_return_contract_projection` | 1 | 1 | 0 | 0 |
| `35_coordinator_control_plane` | 8 | 8 | 0 | 0 |
| `36_execution_control_v1` | 8 | 8 | 0 | 0 |
| `25_workflow_assistance` | 32 | 32 | 0 | 0 |
| `26_workflow_assistance_http` | 1 | 1 | 0 | 0 |
| `44_governance_child` | 21 | 21 | 0 | 0 |
| `46_owner_obligations` | 43 | 43 | 0 | 0 |
| `41_acceptance_context_binding_v1` | 40 | 40 | 0 | 0 |
| `41_current_task_input` | 25 | 25 | 0 | 0 |

四个已归属 #1348 的原失败均保留真实 rc101，失败列名集合 diff=∅：

- `domain::definition::graph_tests::instance_input_key_covered_by_context_schema_passes` — rc101
- `domain::definition::graph_tests::invalid_json_schema_not_checked_by_graph_validation` — rc101
- `domain::definition::graph_tests::primary_to_done_is_advance` — rc101
- `domain::definition::graph_tests::valid_three_node_trunk_passes` — rc101

既有 ignored 为 `forum_sync::owner_assistance_wake_tests::owner_wake_outbox_stays_pending_then_recovers_on_later_pass`。历史 #1348 材料按其原 3dab 坐标保存，未将它称为 fresh b32 执行；本轮 945 项有自己的真实命令/rc/log 索引。

## 命名集与证据坐标

固定提取器对 exact b32 与 committed B 提取。逐逻辑模块及全域 canonical commands/errors/events/scopes/roles/persistent entities/routes 集合 diff=∅；全域引用集合也相同。物理文件引用搬移、raw literals、helper types 和 wire 差异全部暴露并独立语义审查。AA 新 wire 项按 accepted Spec 白名单；SDK 两键是已有响应表示对齐。

全域原集合规模：commands88、errors465、events33、persistent entities39、roles/capabilities12、routes61、scopes8。原集合、提取方法、逐文件哈希及最终差分在证据包 naming/baseline 与 r6/naming/final-candidate/final-comparison。

| Evidence ID | 目标与实际观察 | 包内记录 |
|---|---|---|
| EVD-1344-R6-001 | A/B 原 RED/GREEN 与当前 SDK strict 解析 | r5/sdk、r6/sdk、r6/final-sdk-* |
| EVD-1344-R6-002 | ACC001..007：冻结 RED、23 GREEN、真实写与只读事实 | r4、r6/tests/FINAL_PROBE_RAW_INDEX.json、r6/tests/EXECUTED_PROBE_COVERAGE.json |
| EVD-1344-R6-003 | 命名集、shared response/CTI bytes/digest 不变 | naming/baseline、r6/naming/final-*、r6/review/FINAL_HEAD_REVIEW.json |
| EVD-1344-R6-004 | 945 原名实际调用、四项原失败集合、fixture 终局 | r6/tests/FINAL_EXECUTED_EXISTING_TEST_INDEX.json、r6/tests/FINAL_VERIFICATION_SUMMARY.json |
| EVD-1344-R6-005 | 非产品作者独立 exact B conformance 与 bundle pointer | r6/review/FINAL_HEAD_REVIEW.json、r6/schema/FINAL_BUNDLE_POINTER_REVIEW.json |

执行工作位 HEAD 为 `725c7996dbfac8a7acc079fd72a7dd1946cf42f7` 加冻结候选；221 个源码条目（220 Rust + 1 registry.json）及 17 个新增测试文件与 B 按实际 Git blob SHA256 绑定相等。执行 binary SHA/环境/源快照均保留；未把旧工作位的嵌入 Git 元数据冒称为 B。B 另行编译 rc0。独立 Reviewer 的四个补充 probe 由另一非作者做设计及执行证据复核，作者归因与独立审查分别标明。

## 测试 harness 历史与 fixture 终局

原始四次 harness rc101/STOP/dump 均保存：新 probe 的 mandatory conflict audit 误断言、Owner snapshot 屏障观测 selector、creator restricted 读取前置预期、既有46的真实55432环境 guard。分别独立归因后，仅更正新测试或外部运行环境，保留原成功不变源证据，按固定 hash 续跑；既有测试未改。各实际业务检查最终通过。

owned127.0.0.1:55483 的 41 个 DB dump/pg_restore-list 均 rc0，已 stop rc0、status3/isready2。为满足未改的46/CTI guard，依据独立审查的 runbook 借用历史127.0.0.1:55432测试服务器，只新建本轮67个隔离 DB 并逐库 dump/验证；既有 DB 名称/OID/owner 保留。borrowed PID48483 保持运行，未变更其角色、配置或生命周期；既有业务 bytes 未读取，未声称 byte 相等。

后续无业务/fixture 操作。生产写入=0，部署=0，merge=0；本结果为源码与隔离验证，不是安装、runtime admission 或生产业务验收。

## 可审阅材料

[evidence-index.json](evidence-index.json) 给出 [evidence.tar.gz](evidence.tar.gz) 的实测 SHA256、大小和条目数。包内 EVIDENCE_INDEX.json 为每项原始材料给出内容哈希；所有压缩条目已解包逐字节校验。绝对原始路径是执行坐标，包内 r6/r5/r4 前缀保存对应材料。数据库 dump 负载仍保留在本机原证据路径；包内提供实际 dump/hash/验证回执、必要 readback 和可在授权隔离环境复现的原测试/命令。未执行的旧单 DSN 复现草稿未交付；现势实际执行脚本及合法 fixture runbook 随包保存。

Evaluation: 2026-11-01T09:52:34.382574+00:00。最终 GitHub PR/head/#1344 回填由交付回执另行绑定。
