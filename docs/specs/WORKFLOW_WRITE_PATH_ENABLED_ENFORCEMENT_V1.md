---
spec_id:
accepted_at: 2026-11-02
accepted_by: Owner (mayf3) via supervisor binding; receipt = PR acceptance comment (agent-control#1344 timeline, exact-head c6f1567f lineage) WORKFLOW_WRITE_PATH_ENABLED_ENFORCEMENT_V1
status: accepted
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
scope:
  - svc-workflow seven audited missing local enabled-status checks on existing write paths
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V13
  - SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1
  - SVC_WORKFLOW_EXECUTION_CONTROL_V2
  - SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3
architecture_authority: SVC_WORKFLOW_ARCHITECTURE_V0_4_7
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
base_head: 96385d68ff571a25fdde55842ee70b40830b0783
date: 2026-11-01
acceptance: PENDING_OWNER_EXACT_HEAD_ACCEPTANCE
task: https://github.com/mayf3/agent-control/issues/1344
---

# WORKFLOW_WRITE_PATH_ENABLED_ENFORCEMENT_V1

## 1. Goal

为 #1344 全写路径审计发现的 **4 组 7 项候选缺失**提供有界实施合同：首次执行时，
本地 disabled Principal 不得通过现有 Owner/scheduler 身份写入成功业务事实，
Minimal/model2 不得把 disabled creator 分配为新的 assignee。新补拒绝使用既有语义化
错误、确定性失败 receipt 和同事务审计，保持 enabled 主体的合法行为。

```text
GOAL_OR_TARGET = seven audited local enabled-status gaps; docs-only authority authoring
CURRENT_GAP = Minimal local target refusal/error/receipt lacks accepted implementation authority
SUCCESS_OUTCOME = complete candidate + independent coverage review + commit/push/open PR + #1344 backfill
```

## 2. Scope and non-goals

### 审计全表引用与七项覆盖地图

原始全量命名审计表、静态候选清单、合法例外和执行限制共同作为范围输入：
`/tmp/sw-1344-enabled-audit-evidence/WRITE_PATH_AUDIT.md`，SHA256
`e5fecd940fbf82cec5bececfaffdb5b37d1c06de36aa89a6cffaf76ca81d3b8d`；
持久副本见 [#1344 审计回填](https://github.com/mayf3/agent-control/issues/1344#issuecomment-6554457880)。
该表的“有”仅指静态检查链存在，不表示竞争窗口已关闭或回归通过。
以下完整列出其中 4 组 7 项；#115/#116 是 §3 的两个已修先例，不计入七项。
路径/行号均固定于 `base_head`；`W=src/store/postgres/workflow_instance_repository/`，
`A=src/application/`，`D=src/store/postgres/domain_role_repository.rs`。

| 候选 ID / 组 | 操作/变体 | 审计已证静态事实与来源 | 拟议检查对象 / 合同 / 验收 |
|---|---|---|---|
| GAP-WPE-01 / 生命周期 actor | cancel Owner | `W/cancel_transaction.rs:288–292` 只查 binding.enabled，未覆盖 Principal.enabled | 实际 Owner actor；CTR-WPE-001/002/006/007；ACC-WPE-001/005/006 |
| GAP-WPE-02 / 生命周期 actor | archive Owner | `W/archive_transaction.rs:267–271` 只查 binding.enabled | 实际 Owner actor；CTR-WPE-001/002/006/007；ACC-WPE-001/005/006 |
| GAP-WPE-03 / 成员治理 actor | add_member Owner | `A/domain_membership/mod.rs:306` → `D:192–207`，Owner 分支只 binding；target enabled 已有 `:341–353` | 实际 Owner actor；CTR-WPE-001/003/006/007；ACC-WPE-002/005/006 |
| GAP-WPE-04 / 成员治理 actor | remove_member Owner | `A/domain_membership/mod.rs:474` → 同 shared helper | 实际 Owner actor；CTR-WPE-001/003/006/007；ACC-WPE-002/005/006；不检查 remove target.enabled |
| GAP-WPE-05 / 系统入口 actor | SYSTEM_EXECUTION_ESCALATION | HTTP `src/http/handlers/escalations.rs:51` → wake role gate → `src/store/postgres/provisioning_repository/mod.rs:107–111` 只 binding；`W/execution_escalation.rs:454→521→56` 无 actor enabled | 实际 scheduler Principal；CTR-WPE-001/004/006/007；ACC-WPE-003/005/006 |
| GAP-WPE-06 / Minimal target | model2 ADVANCE→creator | `W/transition_validation.rs:222` 返回 creator UUID；`W/transition_transaction.rs:706` 后查仅 model1，model3 另查；Fixed/Input 已查 | 实际解析的 creator target；CTR-WPE-001/005/006/007；ACC-WPE-004/005/006 |
| GAP-WPE-07 / Minimal target | model2 RETURN→creator | 同上；审计未找到 accepted Minimal 修复权威 | 实际解析的 creator target；CTR-WPE-001/005/006/007；ACC-WPE-004/005/006 |

仅补上述漏检及其拒绝证据/replay，不重做已通过的 #115/#116，不处理 #1348 fixture 债。
不实现 authorized_actions，不扩大其 Minimal 投影范围；不改模型编码、validator、图拓扑、
primary/RETURN/TERMINATE 规则、其他既有检查项及其相对优先级。不是“所有写 SQL 前”通用中间件。
不新增授权面、命令、Event、错误家族、Scope、角色/Capability、路由、业务实体或 migration。
不部署、不运行产品/DB 测试、不写生产、不 merge；本 PR 仅文档。

## 3. Authority and dependencies

本仓父权威均固定在 `base_head`，各自在已接受范围内约束本候选：

| 权威 / 状态 | 本候选关系与边界 |
|---|---|
| `PRODUCT-BOUNDARY.md`（superseded）；`docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V13.md`（accepted） | 沿显式继承链使用 V13；仅工作流串行内核与本地身份投影，不取得 Auth 身份权威 |
| `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_4_7.md`（accepted）CTR-ARCH-003/039 | model2 不冒充 model3；受保护写及 authenticated denial 的 durable audit 必须 atomic、不可用则 fail closed；保留 365 天，不增审计读取/导出 |
| `docs/specs/SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1.md`（accepted）DEC-CP-001/002/007、CTR-CP-001/004/005/006 | Owner/Coordinator 治理范围、无角色错误、receipt/audit、生命周期及 member 三态均保持 |
| `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_2.md`（EFFECTIVE，受 CP 已接受扩展约束） | cancel/archive 生命周期、历史事实保留；不能退回 Owner-only |
| `docs/architecture/AUTH_PRINCIPAL_SELF_PROJECTION_AND_DOMAIN_MEMBERSHIP_V1.md`（FROZEN） | token.sub 是 actor，member 只管 DOMAIN_MEMBER，同事务 durable security audit，不扩投影/角色权 |
| `docs/contracts/WORKFLOW_TRANSITION_CONTRACT_V0_1.md`（IMPLEMENTATION_CONTRACT）§§2/3/8/15/16 | 现有身份/锁序、target resolver、422 确定性失败 receipt、基础设施回滚和幂等合同；不把其 Legacy primary 规则套给 model2 |
| `docs/specs/SVC_WORKFLOW_EXECUTION_CONTROL_V2.md`（accepted）CTR-SWEC-004 | scheduler ingress mirrors wake：execute scope、direct token、server-side scheduler binding 和 durable denial audit；current Visit/Owner/case规则不变 |
| `docs/specs/SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3.md`（accepted）CTR-VAI-012 | Legacy/Minimal 不读写 activation；其中 unchanged 是该模型引入切片的保持面，不是永久禁止另获权威的本地修复 |
| `docs/specs/WORKFLOW_INSTANCE_AUTHORIZED_ACTIONS_V1.md`（已外部 exact-head 接受并合入；仓内生命周期元数据漂移）CTR-AA-003/004/006/008/010 | 现行投影不自授 writer 修复；cancel enabled 义务、既有 model1 先例复用；Minimal UNSUPPORTED_MODEL 不被本 Spec 解除 |
| `docs/specs/SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2.md`（accepted）CTR-CIR-003/DA-001 | 仅作边界约束；Auth principalStatus/AgentDefinition.enabled 的跨服务 admission_target_disabled 含 receipt 零 delta，与本地 enabled 拒绝不同 |

AA 的接受坐标为 semantic head `69ce065c00082c01c3dc03defc72e46a7d0d18a1`，
[Owner receipt](https://github.com/mayf3/svc-workflow/pull/114#issuecomment-6552672226)，
merge `108a47a5eb3c3fb0aee7ee548e297fd65b134013`；当前文件 SHA256
`bd49f9ddceaa5d78ff5246d0782197e7530929ae54befa61be824ff316251e43`。
审计已记录 proposed/PENDING/CURRENT 与外部接受不同步；本任务不重开接受、不改这些文件。

### 与 #115/#116 的语义一致性对照

| 已修先例 / 精确实施 head | 已有合同与修复语义 | 本候选一致性 / 不可推导事项 |
|---|---|---|
| [#115 Legacy RETURN](https://github.com/mayf3/svc-workflow/pull/115) / `22419bfc2712752164e5122addcef2b4f82822f9` | resolver 后在提交事务使用既有 enabled helper；disabled creator → 422/assignee_resolution_failed；COMPLETED 失败 receipt、零 runtime facts、同 key失败 replay | model2 RETURN 拟用相同本地 target 错误/receipt语义；不把 model1 授权当成既有 model2 授权 |
| [#116 Legacy ADVANCE](https://github.com/mayf3/svc-workflow/pull/116) / `5f63842982c2bedc64e3db95b697cefa31f77c20` | 将 model1 后置 enabled检查从 RETURN 扩展至 ADVANCE；None target 无分配；enabled target 合法成功不变 | model2 ADVANCE 沿用 target拒绝合同；Owner/scheduler 是 actor，使用403而非照抄422；不扩大模型/primary规则 |
| 两项共同证据边界 | 已合并，审计 base 含两项；原 passed 证据保留 | 本轮不重跑、不声明新PASS；现有失败持久化 helper 只有receipt+commit，不能把同事务 denial audit 也写成已通过事实 |

```text
SPEC_GOVERNANCE_MODE = AUTHOR
AUTHORITY_ACTION = NEW
AUTHORITY_OWNERSHIP = mayf3/svc-workflow; local write-path enabled enforcement only
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_ARCHITECTURE_V0_4_7
PRIMARY_AUTHORITY = WORKFLOW_WRITE_PATH_ENABLED_ENFORCEMENT_V1 (candidate)
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = CONTROLLED
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO_FOR_THIS_NEW_SPEC
DOCS_FIRST_REQUIRED = YES
IMPLEMENTATION_ALLOWED = NO
MERGE_READY = NO
OPERATION_ALLOWED = NO
```

NEW 原因：新增独立的 Minimal 本地 target 义务及七项新补拒绝的精确 receipt/audit 合同，
没有接受过的独立权威覆盖这个完整切片。其余五项继续受既有 accepted enabled/治理义务
约束；不为重做 REUSE 工作造新产品权限。不覆盖/撤回任何既有 accepted 合同，
`supersedes: []`，无 partial supersession。若发现父权威明确禁止任一拟议义务，须
RE_PREFLIGHT，不能把冲突藏进 NEW child。

Owner 破局文档的逐个修复裁定与本任务预授权是本次起草/GitHub交付的 mandate 输入；
本任务未提供该外部文档的 exact revision，记为 UNKNOWN，不伪造逐字引文或接受回执。
本 Spec 的最终文本仍须 Owner exact-head acceptance，预授权不等于接受未形成的文本。

## 4. Current State

`STATE-WPE-001`：2026-11-01，source-only 静态审计环境，`base_head` 下为4组7项候选；
取消仅沿用旧轮 `/tmp/sw-1344-evidence/STOP_RECEIPT.md` 的 disabled Owner + enabled
binding、HTTP200/version1→2；其余6项没有新增执行反例。依据 OBS-WPE-001/EVD-WPE-001。

`STATE-WPE-002`：在该 base，未找到 model2 本地 enabled 拒绝及错误/receipt 的 accepted
实施权威；不是证明两个 accepted 产品契约冲突。#115/#116 已合入，不再作为缺失项。
依据 OBS-WPE-001/002、CLM-WPE-001。

`STATE-WPE-003`：本次仅提出合同和未来验收要求，产品 RED→GREEN、实现命名集比较、
数据库/HTTP/生产验收均 NOT_RUN。文档独立复核不等于功能已修复。

## 5. Observations

- `OBS-WPE-001`：2026-11-01 精读 §2 固定哈希审计报告及 #1344 持久回填；方法是该报告的
  全写面静态枚举和已有停止证据复用。结果/限制原样按 STATE-WPE-001/002 保存；
  不把缺静态检查升级为额外HTTP成功、RED或竞争证明。
- `OBS-WPE-002`：同日只读核对 base 源码、冻结合同和 #115/#116 PR。
  actor 错误映射既有403/principal_disabled；transition target 既有422/assignee_resolution_failed。
  member role gate 在 receipt 前，add target disabled 已有失败receipt；cancel/archive
  的错误 replay decoder 未识别 principal_disabled，system escalation replay 忽略status并
  强解成功outcome。`transition_validation.rs:584–603` 失败helper未写denial audit。
  Provenance：§3权威，`src/http/error.rs`，`A/domain_membership/{mod,receipt}.rs`，
  `W/{cancel_transaction,archive_transaction,execution_escalation,transition_validation}.rs`；
  这些是静态观察，不是本轮运行验证。

## 6. Claims and assumptions

`CLM-WPE-001` — SUPPORTED（EVD-WPE-001/002）：需为 Minimal 本地 target 和七项新增拒绝
提供明确合同，不能借 CIR 跨服务 admission、代码已存在或 model1 先例自授 model2 实施。
不确定性：静态候选的真实 RED 尚待实施轮隔离验证；若实际入口已拒绝，须记录事实和
RE_PREFLIGHT，不能删旧守卫、改fixture制造RED。

## 7. Evidence relations

| Evidence | Source → target / relation | Coordinates / sufficiency / limitation / provenance |
|---|---|---|
| EVD-WPE-001 | OBS-WPE-001 → STATE-WPE-001/002、CLM-WPE-001；SUPPORTS | base_head、2026-11-01、本地静态环境；足够界定候选/授权停点；不是七项执行证据；§2审计固定哈希和回填 |
| EVD-WPE-002 | OBS-WPE-002 → CLM-WPE-001；SUPPORTS | 同base/日期/source-only；足够选择拟议错误和receipt边界；不是产品验收；§3权威/源码/#115/#116 |

## 8. Decisions

- `DEC-WPE-001`（拟议，Owner mayf3）：在现有合法写路径中补齐七项本地 enabled 检查；
  Minimal 仅获得该有界负向义务，不形成完整模型操作权威。理由：闭合审计停点且不混模型。
- `DEC-WPE-002`（拟议，Owner mayf3）：七项新补检查在 first execution 中产生既有语义化
  确定性失败 receipt，且同事务 durable denial audit；历史 replay 与先于receipt的既有门
  保持原边界。理由：拒绝可复读、重放不变500；ALT-WPE-001/002被拒绝。

## 9. Contracts

### CTR-WPE-001 — 首次执行与事务内 fresh read

对 GAP-WPE-01..07，系统 MUST 在实际写事务内、取得本命令 receipt 的首次执行权并
排除历史 replay 后、任何成功业务 mutation 前 fresh read **实际本地 Principal UUID**
的 `principals.enabled`。不能用 token 声明、binding.enabled、缓存、请求体替代身份、
事务外 pool读取或先前查询结论代替。已有角色/领域/scope/direct-token/模型/状态/CAS/
payload/admission检查 MUST 全部保留，既有检查彼此的相对顺序不变；只在各类合同指定
位置插入 enabled gate，已经在该位置之前发生的拒绝不得被覆盖。多重无效输入按此
明确顺序判定：新gate可先于其后原有检查拒绝，不承诺它之后的旧错误仍优先于新拒绝。

事务 MUST 保证新业务事实与该 enabled 判定一致；同事务读不是无限有效的承诺。
若 disable 已在 fresh read 前提交，新请求 MUST 拒绝；并发 disable 与写入必须有明确
可验证的数据库串行化/锁定关系，不能只凭一次静态SELECT宣称竞争已关闭。
不要求更改既有 Receipt→Instance→DefinitionVersion 锁序或做跨服务原子性。

### CTR-WPE-002 — Cancel / Archive Owner

GAP-WPE-01/02 在原 receipt/replay、实际 instance domain 和现行治理角色判定后，
成功业务写之前 MUST 验证实际 Owner actor enabled。有效 Owner binding 不能使 disabled
Principal 成功；新补拒绝 MUST 是各自既有 `PrincipalDisabled`、HTTP403/
`principal_disabled`，并遵守 CTR-WPE-006/007。

两角色皆无仍按 CP DEC-CP-002 返回原 `not_domain_owner`；Coordinator 与 managed cleanup
沿既有权限和 enabled 检查，不获新权限。cancel仅active/nonterminal，archive仅terminal
或cancelled且无非法active activation等既有检查不变。旧creator/旧assignee disabled
不得成为新增拒绝条件；不能修改历史Visit、要求历史主体重新启用或阻止合法清理。

### CTR-WPE-003 — Member add / remove Owner

GAP-WPE-03/04 MUST 保留现有 shared governance-role gate（含一般role-denial），add 的
domain enabled等前置检查、receipt/replay顺序；仅在 receipt owned 的 first-execution
分支，在现有 target检查/绑定写之前新增事务内实际 Owner enabled检查。
disabled Owner +有效binding MUST 403/principal_disabled，并完成确定性失败receipt
及同事务denial audit。这是拟议的新补 actor 拒绝合同，不声称当前已具备。

MUST 保留 add target存在/enabled/非Owner检查、三态 added/already_member/replay与
no-op审计纪律；remove disabled target在其他条件合法时 MUST 仍可撤销。不新增 remove
target enabled门。现有Coordinator路径/领域权限不变；不把所有 shared helper调用者
改成新拒绝、不移动其receipt前门以阻断原本合法的历史replay。

### CTR-WPE-004 — System execution-escalation scheduler

GAP-WPE-05 MUST 保留 HTTP 的 workflow.execute、direct-token、GLOBAL_SCHEDULER_READ
server-side binding和原denial路径；在原receipt owned分支、instance/case/Owner业务操作前，
store命令事务 MUST fresh read实际scheduler Principal enabled。不得因system/service
称谓豁免或只在HTTP检查；非HTTP合法调用也必须经过此门。
disabled actor MUST 使用既有 AssistanceError::PrincipalDisabled →403/principal_disabled，
CTR-WPE-006/007的失败receipt与审计。已有current Visit、cancelled/archived/terminal、
current Owner enabled、open case幂等、reason/attempt/provenance及forum/outbox逻辑不变。
新拒绝不能开case、发Event、增长版本、写forum/outbox或触发自动人类升级。

### CTR-WPE-005 — Minimal/model2 creator target

GAP-WPE-06/07 MUST 在原 ADVANCE/RETURN 检查和 target resolver 后、业务事实写前，
同提交事务验证解析得到的 creator UUID存在且enabled。disabled或不存在target MUST
复用 ExecuteWorkflowTransitionError::AssigneeResolutionFailed →422/
`assignee_resolution_failed`，沿现有确定性失败receipt边界并满足CTR-WPE-006/007；
不能套actor的403，也不能改为CIR的admission_target_disabled或直接500。

enabled creator的合法ADVANCE/RETURN MUST与base相同；Fixed/Input既有检查不删改。
model2的合法nonprimary ADVANCE不套model1 primary限制；RETURN payload/refs/limit保持，
不增加TERMINATE许可、不引入activation、不把model2改为model3。None终态target仍无新
assignment。CTR-AA-008的Minimal UNSUPPORTED_MODEL边界保持；此负向修复不证明完整
Minimal authorized_actions合法集合，也不授权新Minimal生产流量或迁移。

### CTR-WPE-006 — 确定性失败 receipt、replay及零业务增量

七项**新补gate自身**的拒绝 MUST 在原命令的现有receipt中完成：`receipt_status=COMPLETED`，
actor为response_status403、target为422，存该命令既有失败body/错误envelope及digest。
不新造统一receipt协议、不把失败解成成功outcome、不留下PROCESSING；所有业务投影、
Visit/Submission/Context/Event、角色binding、activation/case和outbox增量 MUST 为零。
只允许本次真实失败receipt和CTR-WPE-007的拒绝证据提交。

实现 MUST 同时支持失败的序列化、status分派、typed error解码及同key/hash replay：
cancel/archive decoder必须识别principal_disabled；system escalation必须按status解码失败，
不能对403 body强解SystemEscalationOutcome后变500。member复用原失败receipt形态，
transition复用原422形态。重启后同key失败仍同status/code，重新启用主体也不能将旧key
变为成功；fresh key按当前事实重新评估。成功历史receipt保持各命令原replay行为。

本合同不移动任何既有HTTP/role/domain/input等receipt前门：若原门先拒绝，保留其原
错误/审计/零receipt合同，不强造本次业务receipt。member既有receipt前role gate尤其
保留。replay是历史结果读取，不重新执行新enabled gate、不重复业务事实或首次拒绝
审计；原attempt/replay审计照常。不同body冲突、PROCESSING与unknown outcome的原
合同保持；丢失commit响应须核对原receipt，不能擅自认定失败后换key重放。

### CTR-WPE-007 — 同事务审计与失败原子性

每项新补拒绝 MUST 写现有durable非敏感security/attempt审计，在**同一命令数据库事务**
与失败receipt提交，遵守CTR-ARCH-039的保留/访问边界；独立pool写审计或tracing日志
不能替代。审计 MUST 能关联真实actor、已有操作/command ID、真实Domain/Instance或
member target、拒绝code及检查对象；target禁用不得误记为actor禁用。
复用现有审计表、动作与details表达拒绝，不增命令/Event/错误/Scope/角色/路由名称。

既有成功及denial审计检查不得削弱；本候选只要求七项新补拒绝的完整事务证据，不自称
已修全系统审计。新补gate拒绝时不得提交成功receipt或业务mutation；失败receipt不得
在缺少同事务denial audit时提交。
审计/receipt写失败、数据库故障 MUST 回滚包括本次receipt/audit在内的全部新写并fail
closed，沿既有StorageError基础设施映射；不得把该故障伪装成已完成403/422。
单纯disabled且数据库正常时 MUST 稳定返回语义拒绝，不能500。

### CTR-WPE-008 — 兼容边界与实现闸

enabled主体在所有既有检查合法时 MUST照常成功；不新增授权面、不删改既有检查项。
MUST 保留 CP reconcile disabled source/原角色撤销/provision禁用恢复合法输入、Draft
literal保存、历史Event重建、outbox存量投递/退避、typed STOP、失败receipt和denial证据。
这些写点不因历史actor/target后来disabled而被通用中间件封禁。

后续实现 MUST 以已接受并合入其base的本Spec为依据，逐项取得ACC-WPE-001..007所需
真实证据及独立复核。测试名称、命令、Event、错误、Scope、角色/Capability、实体和
路由的命名集逐模块及全域 MUST diff=∅；本修复不设置新增业务名白名单。
新增探针可置隔离证据overlay，不能删/改名/过滤既有测试制造通过。
已知#1348债仅按既有裁定原名呈现，不在此修复或当成新增语义失败。

## 10. Acceptance

下列是**未来实施闸，当前全部 NOT_RUN**。各运行项必须绑定exact Spec/implementation
SHA、base/candidate、隔离PostgreSQL+实际HTTP/命令入口、身份/scope（无Secret）、fixture
来源、命令/rc、原响应和同快照关系readback。RED/GREEN必须使用同一有区分力请求；
不得把编译/harness错误或静态缺检查当成业务RED。

| Acceptance / Contracts | Method / inputs / environment | Required evidence / expected result / failure condition |
|---|---|---|
| ACC-WPE-001 / CTR-WPE-001/002/006/007/008 | GAP01 cancel、GAP02 archive各独立RED→GREEN；disabled Owner+enabled binding+其余输入合法；隔离真实HTTP；enabled Owner/Coordinator/合法cleanup控制 | 每项RED证明原缺失，GREEN403/principal_disabled、COMPLETED403、同事务denial audit、零业务增量；enabled合法成功、无角色原错误、历史creator/assignee disabled仍可治理；任一漏项/误拒/状态规则改变即FAIL |
| ACC-WPE-002 / CTR-WPE-001/003/006/007/008 | GAP03 add、GAP04 remove各独立RED→GREEN；Owner actor disabled、target存在；enabled Owner/Coordinator、added/already_member、remove disabled target及pre-receipt拒绝控制 | GREEN403及失败receipt+audit原子；binding零变；remove disabled target合法成功；add原target拒绝与角色/领域前置边界不变；对目标新增门或多写member_added审计即FAIL |
| ACC-WPE-003 / CTR-WPE-001/004/006/007/008 | GAP05 RED→GREEN：disabled scheduler+enabled scheduler binding，current Visit/Owner enabled、合法reason；实际HTTP和直接store调用；enabled scheduler/no-role/OBO控制 | GREEN403、失败receipt、同事务audit，case/Event/version/forum/outbox零增量；enabled保持正常/已有case幂等；HTTP-only检查、system豁免或失败replay500即FAIL |
| ACC-WPE-004 / CTR-WPE-001/005/006/007/008 | GAP06 ADVANCE、GAP07 RETURN各RED→GREEN；合法Minimal graph通过其既有validate/publish，enabled当前assignee，creator disabled；合法RETURN引用；enabled creator、Fixed/Input、合法nonprimary控制 | GREEN422/assignee_resolution_failed、COMPLETED422及同事务audit，全部业务事实不变；model1/model3语义保持；启用creator新key成功；偷偷套primary/activation/完整Minimal投影或冒称旧model1证据覆盖即FAIL |
| ACC-WPE-005 / CTR-WPE-001/006/007 | 对七项分别复读失败receipt、同key同body、重启、重新启用后旧key/新key；旧成功receipt、不同body冲突、PROCESSING、原pre-receipt拒绝控制；隔离真实命令 | 原失败status/code/command identity保持，旧key零新业务/零第二条首次拒绝audit，新keyfresh评估；历史成功replay和原错误优先级保持；failure decoder变500、修改历史receipt或越过前门即FAIL |
| ACC-WPE-006 / CTR-WPE-001/006/007 | 七项分别强制audit失败和receipt completion失败；独立只读回查；控制disable先提交再fresh read及并发交错；隔离DB | 成功记录receipt+denial audit的同tx证据；任一写失败两者和业务均零新delta、StorageError fail closed；disable先提交必拒绝；并发附事务顺序/锁证据且无已禁用后漏放；异步audit/只查数量/盲称无竞争即FAIL |
| ACC-WPE-007 / CTR-WPE-008及全部合同 | 文档独立非作者7/7覆盖/父权威审查；实施轮baseline/candidate同套件、逐模块及全域命名集提取；保留#115/#116先例、合法例外和#1348原债 | docs-only diff、schema/生命周期/合同反向映射检查；最终head独立review；七项各有RED/GREEN/receipt/audit证据；命名集原文/方法/哈希diff=∅且无新增语义失败；缺accepted-in-base、漏项、只报源码同一或滤掉债即FAIL |

### Contract coverage（双向）

| Contract | Acceptance | Evidence class |
|---|---|---|
| CTR-WPE-001 | ACC-WPE-001..006/007 | isolated executed HTTP/command + transaction evidence; independent review |
| CTR-WPE-002 | ACC-WPE-001/007 | isolated lifecycle controls + review |
| CTR-WPE-003 | ACC-WPE-002/007 | isolated membership controls + review |
| CTR-WPE-004 | ACC-WPE-003/007 | isolated HTTP/store controls + review |
| CTR-WPE-005 | ACC-WPE-004/007 | isolated model2 target controls + review |
| CTR-WPE-006 | ACC-WPE-001..006/007 | receipt/replay/zero-delta/fault evidence + review |
| CTR-WPE-007 | ACC-WPE-001..006/007 | same-transaction audit/fault evidence + review |
| CTR-WPE-008 | ACC-WPE-001..004/007 | positive/exception controls + exact naming sets + review |

本轮文档完成定义：单份Spec含七项地图和两项先例表，独立非作者覆盖复核PASS，
commit/push/open-only PR到main，#1344回填。产品验收留待accepted authority进入实施base。
EXPANSION_TRIGGER：行为/测试/配置写入、父权威冲突、七项外修复、相关base语义变动。
发生后停止扩张并RE_PREFLIGHT；不重跑已通过的不受影响修复。

## 11. Alternatives and disposition

- `ALT-WPE-001` REJECTED：统一所有actor/target为422，或把disabled当500；违背现有403/422语义区分。
- `ALT-WPE-002` REJECTED：只加if后直接Err、先提交失败receipt后异步audit、只测首次响应；
  无法满足原子拒绝和失败replay合同。必须验证真实receipt/事务，不能仅查日志。
- `ALT-WPE-003` REJECTED：借CIR/model1授权改model2，或删除model2/改为model3；权威对象与模型不同。
- `ALT-WPE-004` RETAIN：Owner治理、Coordinator、scheduler、Minimal/Legacy/model3各自职责和稳定
  HTTP facade；没有退役/迁移/永久兼容桥需求。拒绝历史写证据不等于赋予当前成功权限。

## 12. Migration, compatibility, and rollback

### r1 停点逐条闭合（文本拟议，接受前仍不放行实施）

| 审计/r1停点 | 本Spec明确补充 | Acceptance / readiness |
|---|---|---|
| Minimal/model2本地enabled拒绝无accepted实施权威 | DEC-WPE-001、CTR-WPE-001/005，GAP06/07；NEW有界child，不套model1 primary、不冒充model3 | ACC-WPE-004/007；Owner exact-head接受且合入实施base之前NO |
| Minimal本地错误无accepted边界，不能借CIR | CTR-WPE-005/006明确422/assignee_resolution_failed；actor403；CIR admission_target_disabled及含receipt零delta原合同不变 | ACC-WPE-004/005；不能仅靠代码/跨服务合同推导 |
| Minimal receipt/拒绝证据边界无accepted权威 | CTR-WPE-006/007明确COMPLETED失败receipt、零业务事实、同事务denial audit、失败replay与故障回滚 | ACC-WPE-004..006；当前NOT_RUN |
| 完整4组7项切片因Minimal缺口整体停闸 | §2七项全覆盖、CTR-WPE-002..004、§3先例表、逐项RED→GREEN与命名集/独立复核 | ACC-WPE-001..007；不自行缩成仅五项，不报全量修复PASS |

MIGRATION = 不改schema/历史数据，不backfill历史失败receipt/audit，不重分配既有任务。
COMPATIBILITY = 仅七项原漏检输入新增拒绝及拒绝证据；enabled合法成功与所有既有检查保留。
新增actor失败receipt及其decoder/audit是明确拟议增量，不假称当前已存在；其他既有拒绝边界不变。
ROLLBACK = 文档候选可关闭PR；接受后实现回退按原授权流程，保留已提交receipt/audit历史，
不能删除证据或把已失败旧key转成功。代码回退不撤销accepted enabled义务，须记录conformance drift。
PRODUCTION = 本Spec不提供部署、身份/配置切换或生产apply授权。

## 13. Open questions and acceptance

```text
ACCEPTANCE = PENDING_OWNER_EXACT_HEAD_ACCEPTANCE
OWNER_DIRECTION = individual repair ruling and task preauthorization; not final-text acceptance
OPEN_OWNER_DECISIONS = exact final head acceptance only
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE_IDENTIFIED; missing Minimal authority is a gap, not proven conflict
PARTIAL_SUPERSESSION = NONE
READY_TO_MARK_ACCEPTED = NO
IMPLEMENTATION_ALLOWED = NO
IMPLEMENTATION_VERIFICATION = NOT_RUN
MERGE_READY = NO (open-only task)
OPERATION_ALLOWED = NO
PRODUCTION_APPLY_AUTHORITY = NONE
```

Owner接受须绑定最终reviewed commit、Spec内容SHA256和独立非作者结论；作者不得代签。
相关文本或父权威变动需独立final-head复核。本轮交付止于open PR与#1344回填；未来实施
只有在该权威accepted且已合入其base后，才另行按REUSE和实际mandate执行。
