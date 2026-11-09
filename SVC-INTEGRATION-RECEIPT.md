# SVC 最终整合候选源码验收回执

候选 / main 快照：`f51d8724fe0dc80053a82c37f19862b059ba6a37`。仓库 `mayf3/svc-workflow`，隔离目录 `/private/tmp/svc-integration/repo`，提交分支 `test/integration-receipt-20261109`。验收日期：2026-11-09（Asia/Shanghai）。下述 main 行坐标均绑定此 SHA；三个实现文件与该快照逐字节一致。本文所在提交是测试及证据提交，不改变产品实现。

执行授权：[Owner BRIEF 留档](docs/evidence/svc-integration-20261109/OWNER-RECEIPT-BRIEF.md) 及本次续办指令；只做本地源码验收、单 commit、push、base-main PR，不合并，零生产写入。路由 `REUSE / BRIEF / DURABLE`，`SPEC_GOVERNANCE_MODE=COMPLIANCE`；复用 base 中 accepted 的 Product V13、Architecture V0.4.7 与 `SVC_WORKFLOW_EXECUTION_CONTROL_V3`（`implementation_authority: contracts`、`production_apply_authority: none`）。[治理校验](docs/evidence/svc-integration-20261109/governance.json) rc=0。完成边界为回执、证据、单次提交及 PR 或 push 失败后的 bundle 交接；生产恢复不在本任务内。

## 1. #128 / #71 / #132 三语义保留核

| 语义 | main 文件与行坐标 | 结论与证据关系 |
|---|---|---|
| #128 发布归因初始化 | `scripts/release.sh:92-112` | `id -un` 取得真实 `osUser`；actor 缺省/空值记录 `unattributed` 并警告；保留 `authRef`，生成或验证 `attemptId`，非法 UUID 拒绝。EVD-128：源码及 shell 归因正负用例支持保留。 |
| #128 归因写入及 verify 边界 | `scripts/release.sh:431-441,550-564,742-752` | build provenance、deploy ledger 均含四项归因；all 共用初始化；verify 不重建或冒充历史归因。新增交互测试成功、失败路径均通过。 |
| #71 wake 独立批次 | `src/store/postgres/outbox.rs:157-188`；`src/forum_sync.rs:870-900` | 查询先按 `outbox_kind=$1` 隔离再应用 limit；wake 先独立 drain，kick / Forum 分别 drain，其他 kind 积压不挤占 wake 批次。EVD-71-OUTBOX：library 的积压与原生多后端测试、三通道测试支持保留。 |
| #71 fail-closed 历史收据保护 | `scripts/release.sh:628-676,678-699,700-724` | 版本、binary、migration、健康与认证检查任一失败均在改写前退出；最新 receipt 必须精确匹配 provenance；只更新该 receipt 的 verification，历史前缀按字节保留，完成后才以临时文件 mv 替换。EVD-71-RECEIPT：历史保护、并发与新增归因交互用例支持保留。 |
| #132 Forum 可选与配置隔离 | `src/forum_sync.rs:103-111,249-345,884-899` | Forum 为 Option；kick / wake 显式开关各自生效；partial wake config 保持活跃并 fail-closed；Forum 未配置时不查询其 binding、不消费其行。EVD-132：配置单测与 kick-only / disabled-channel 集成测试支持保留。 |
| #132 后台通道隔离 | `src/forum_sync.rs:1213-1268` | 每个启用通道使用独立 worker，慢请求或 worker 失败不阻塞其他通道；保留原 run_once、重试、锁和排序语义。三方向持续超时、单连接池及并发用例均通过。 |

[独立只读复核](docs/evidence/svc-integration-20261109/INDEPENDENT-REVIEW.md) 由 `/root/semantic_review` 完成：三语义保留，未发现本次交互新增缺口。该复核读取实际日志，不代替执行测试。

## 2. 历史测试复用复跑与计数比较

| 入口 | 历史基线 | 本候选实际结果 | 比较 |
|---|---|---|---|
| `bash scripts/test_release.sh` | 整合后 113 PASS；早期 #71 核心 shell 24/24 | **114 PASS / 0 FAIL，rc=0**（核心 26 + 上游 88） | 比 113 增 1；核心比 24 增 2，main 原有 25 个核心用例均保留。 |
| `cargo test --locked --offline --jobs 4 --lib -- --test-threads=1 --nocapture` | PG library **215/215**；#132 后续记录为 269 PASS / 1 ignored | **276 PASS / 0 FAIL / 1 既有 ignored，rc=0** | 比 215 增 61，比 #132 的 269 增 7；无新增失败。 |
| `cargo test --locked --offline --jobs 4 --test 48_outbox_three_channels -- --test-threads=1 --nocapture` | #132 通道集 **22/22** | **22 PASS / 0 FAIL / 0 ignored，rc=0** | 持平，无减少、无新增失败。 |

shell 与编译是中断前已完成的实际观察，本次复用，不虚构复跑：shell 时间 `2026-11-09T15:27:05Z–15:31:55Z`，原始日志逐行计数为 114 PASS，第 3 行为新增交互测试；末尾 `RESULT: 88 passed` 仅汇总上游子套件。见 [shell 元数据](docs/evidence/svc-integration-20261109/release-baseline.json)、[完整日志](docs/evidence/svc-integration-20261109/release-baseline.log)、[编译 rc=0](docs/evidence/svc-integration-20261109/compile.json)。Rust 测试则是本次续办实际执行：library `15:41:01Z–15:41:05Z`，通道 `15:41:05Z–15:41:31Z`；[library 命令/rc/计数](docs/evidence/svc-integration-20261109/library.json)、[原始日志](docs/evidence/svc-integration-20261109/library.log)、[通道命令/rc/计数](docs/evidence/svc-integration-20261109/channels.json)、[原始日志](docs/evidence/svc-integration-20261109/channels.log)。

历史来源保留：[24/24 commit 记录](docs/evidence/svc-integration-20261109/historical-shell-baseline.txt)、[215/215 commit 记录](docs/evidence/svc-integration-20261109/historical-library-baseline.txt)、[#132 历史回执](RECEIPT-1359-OUTBOX.md)。[机器可读比较](docs/evidence/svc-integration-20261109/test-comparison.json) 明确计数、增量、原有核心用例未删除及唯一新增用例。

PG 使用任务专用 PostgreSQL 16.15：`127.0.0.1:55947`、角色 `svc_integration_test`、library 库 `svc_integration_20261109_lib`、data-dir `/private/tmp/svc-integration/postgres/data`。执行前 [五项身份实测](docs/evidence/svc-integration-20261109/postgres-resume-identity.json) 全部匹配才准入；三通道各测试的新建库也只位于同一专用服务器。未回落到默认 5432。测试后 [469 个源码/迁移/构建输入哈希全部不变](docs/evidence/svc-integration-20261109/source-integrity.json)。[复现脚本](docs/evidence/svc-integration-20261109/resume-validation.py) 保留命令、环境清理和身份保护；重跑前需恢复该任务实例。

既有 ignored 项为 `forum_sync::owner_assistance_wake_tests::owner_wake_outbox_stays_pending_then_recovers_on_later_pass`（`src/forum_sync.rs:2587-2591`），本次未执行，不算 PASS。未运行整个 cargo 测试全集；本次通过只对应上述指定门禁。任务 PG 在验后再次核验身份并停止，实际 rc=0，端口关闭、PID 文件消失，数据保留：[清理回执](docs/evidence/svc-integration-20261109/postgres-cleanup.json)。未停止任何既有或生产服务。

## 3. 实际受影响范围与补充测试判定

本次产品实现变更为 **0**；唯一测试 delta 是 `scripts/test_release.sh:194-223` 的 `test_verify_preserves_attribution_and_history` 及其注册项（相对 main 增 33 行）。其直接覆盖 #128 归因包裹 #71 verify 隔离流程：构造含引号、换行和中文的历史归因；使用不同 verify 调用者归因；成功 verify 后历史 receipt 前缀逐字节相同、两条 receipt 的 `osUser/actor/authRef/attemptId` 不变；随后 readyz=503 的失败 verify 保留完整 ledger。该用例已实际 PASS。

outbox 批次 × 通道隔离 **无需新增**：`src/store/postgres/outbox.rs:462` 覆盖 kind-scoped 批次与 dead-letter 排除；`src/forum_sync.rs:1429,1465-1560,1591` 覆盖 Forum 查询失败前 wake 投递、21 条旧 Forum + 21 条旧 kick 不挤占 wake、关闭 Forum 不增加其 attempt、未知 kind 保留、慢 Forum 不阻塞 wake。`tests/48_outbox_three_channels.rs:286,669-679,785-938,1476` 已覆盖超时、三方向持续阻塞、kick-only、通道关闭和三通道投递，另含并发/崩溃/UNKNOWN/重启/单连接池/排序回归，均在本次 22 项测试内执行。

未更改 migration、配置、归因/授权模型、重试或历史数据语义；未以修复旧套件、扩大覆盖或生产恢复为由增加工作范围。所有新增文件均为本次回执、复现与实际证据。

## 4. 结论与交付边界

三语义保留核通过；指定门禁实际零失败，计数只增不减，输入完整性通过。`IMPLEMENTATION_STATE=COMPLETE`，`VERIFICATION_STATE=SUFFICIENT`，`CONFORMANCE_RESULT=VERIFIED`，仅限本文绑定的源码与隔离测试范围。

SOURCE_INTEGRATION_ACCEPTED=YES

生产唤醒恢复=SEPARATE

本结论不证明安装、生产加载、唤醒恢复、历史积压消费或业务验收。生产恢复归 deploy 链另验；本轮零生产写入，未发布、部署、合并或改动生产服务。按授权仅创建一个测试/回执提交并尝试推送 `origin test/integration-receipt-20261109`，成功后创建 base `main`、标题 `test(integration): final integrated-candidate receipt` 的 PR，保持未合并；push 因代理 7890 失败时生成 bundle，不重试。包含本文的提交 SHA 与实际 PR / bundle 交付结果由提交后的独立记录及最终交接绑定，本文不预先宣称远端成功。
