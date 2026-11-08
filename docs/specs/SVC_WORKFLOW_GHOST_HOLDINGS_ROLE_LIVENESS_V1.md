---
spec_id: SVC_WORKFLOW_GHOST_HOLDINGS_ROLE_LIVENESS_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: none
production_apply_authority: none
authority_action: NEW
owning_repository: mayf3/svc-workflow
authoring_base: b5096782f666ad22aa3508fdbf9562278a977c0f
scope:
  - Proposed complete holdings read contract for Core C1 and C2
  - Proposed Agent target liveness at all role and owner activation boundaries
  - Observation versus retirement linearization and paired acceptance gates
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V13
architecture_authority: SVC_WORKFLOW_ARCHITECTURE_V0_4_7
external_authorities:
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_GHOST_HOLDINGS_PROTECTION_V1
    revision: 527d641518bdde61267cde56000bfb8299780667
    relation: interoperates_with
  - repository: mayf3/auth-service
    authority_id: AUTH_SERVICE_INTERNAL_IDENTITY_DIRECTORY_V1
    revision: f2b7d4c91ad657464816108062145ae95c21d686
    relation: depends_on
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_INTERNAL_AGENT_DIRECTORY_V1
    revision: 7f620ad87ebadc9571eaf527bc370f259dfd9cc6
    relation: depends_on
supersedes: []
superseded_by: null
owners: [mayf3]
---

# SVC_WORKFLOW_GHOST_HOLDINGS_ROLE_LIVENESS_V1

**配套协议提案，proposed / none。** 本文不接受、不实施、不合并任何合同，不提供生产权限。
Core P3 要求的是 Svc 正式 accepted Contracts；本提案尚未满足该门。
Core 接受不能代替 Svc 接受；Svc 接受也不能代替 Core 接受。

## 1. Goal

为 [Core #674 的 P3](https://github.com/mayf3/dsh-agent-core/blob/527d641518bdde61267cde56000bfb8299780667/docs/specs/AGENT_CORE_GHOST_HOLDINGS_PROTECTION_V1.md)
提出可测试的 Svc 配套：完整识别当前实例与 enabled role 债务，阻止新的幽灵角色生效，
并使“刚读取为空”不能成为未经证明的离册许可。业务目标来自
[#1351](https://github.com/mayf3/agent-control/issues/1351) 及
[三处置收敛](https://github.com/mayf3/agent-control/issues/1351#issuecomment-6716360936)。

```text
GOAL_OR_TARGET = P3 companion proposal with executable acceptance definitions
CURRENT_GAP = roles only check local Principal; complete read permission and retirement ordering unproven
DONE_WHEN = proposed contract + report + checks + commit/push + PR(base main), no merge
```

## 2. Scope and non-goals

覆盖全量 holdings、角色生效全写点、身份/错误/时间边界、Core C1/C2 适配、两提交顺序验收。
“全量”包括 disabled Principal/Domain、WAITING、未到执行时刻和非 Agent 执行类别中的真实责任，
不等于某个调用者的 my_tasks 可见集合。不因源身份已经 disabled/离册而要求先恢复其权限。

三处置分别是 disable（保留身份与 holdings）、交接（独立合法过户）、退役（交接完成后离册）。
新增 GC 门只限制退役终点，不给 disable 增加 holdings/目录可用性前置。
不授权 Auth 身份删除、任意实例改派、Visit 重写、自动清债、模型特权、身份/Grant 创建、
新锁/lease/分布式事务、生产扫描、部署、迁移执行或已有 UNKNOWN/fence 清除。

## 3. Authority and dependencies

```text
SPEC_GOVERNANCE_MODE = AUTHOR
AUTHORITY_ACTION = NEW
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = CONTROLLED
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO
DOCS_FIRST_REQUIRED = YES
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V13
IMPLEMENTATION_AUTHORITY = none
```

NEW 只拥有本配套的协议提案身份，不把旧通过集合的收窄伪称 additive AMEND。
所有本地权威均以 authoring base 的文件为坐标；以下是**接受前置**，不是本 PR 已完成的继任：

| 既有权威 / 精确范围 | 对本提案的约束 / 关闭的门 |
|---|---|
| Product V13 §6、§9；Architecture V0_4_7 §5.2、CTR-ARCH-004/032/033/040 | child 不得扩大跨域读权。完整 holdings 读须证明已有权限组合；否则先由 owning Product/Architecture 完整继任并接受。普通 global reader 不自动获得 Principal/全部 role 枚举权 |
| `docs/contracts/IDENTITY_PROVISIONING_API_V0.md`，FROZEN_FOR_PROVISIONING_READY，§1/3/5/6/8/9 | 本地 exists+enabled、角色/owner 原子性、provisioning actor、错误和幂等仍有效；新增远端拒绝须完整 successor 及合法原子回链，不能本文直接覆盖 |
| `SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1`，accepted/contracts，DEC-CP-005/007、CTR-CP-003 | 新 registry 依赖/失败收窄其通过集合，须 whole-authority successor 及原子回链；source disabled 合法、target enabled、coordinator 无 admin fallback 不变 |
| `AUTH_PRINCIPAL_SELF_PROJECTION_AND_DOMAIN_MEMBERSHIP_V1`，FROZEN，成员管理/审计 | 自投影不建 role，disabled 不可由 self-projection 恢复；成员角色扩围须纳入 owning-authority 继任审查，不把旧历史背景当新身份权限 |
| `SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2`，accepted，CTR-CIR-003、CTR-CIR-DA-001/002 | REUSE 精确身份/五秒观测语义；不扩其冻结迁移集合，不把 assignment admission 的既有接受推导为 role 扩围已接受 |
| `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1`、Architecture CTR-ARCH-009/035、Owner Obligation Workflows V3 | 旧 Visit 不可任意改派；有限 successor 不泛化；managed Owner replacement 保留 STOP/OWNER_CHANGED_UNSUPPORTED、原预算和 fence |

外部 Core GHP 引用是 **proposed** 配对输入；目录 authority pins 从本地 accepted CIR V2 原样继承。
固定源码观察不取代目录权威。真正激活前须重新核验各外部 authority 的正式接受和实施 base。
Core 的常设 §3C 委托不能自动成为 Svc 接受权限；Svc 接受者须是 mayf3 或被明确授权的 Svc maintainer。
用户任务及 [Brief 原字节副本](../reports/ghost-holdings-1351-p3/BRIEF-SVC-1351-P3-COMPANION.md)
只授权本次文档交付。未改任何 accepted 文件、回链或索引。

## 4. Current State

`STATE-SGHR-001`：as-of=`b5096782f666ad22aa3508fdbf9562278a977c0f`；environment=隔离
`svc-h1351-p3` / `docs/ghost-holdings-svc-companion`；观察日期=2026-11-08 UTC；
basis=`OBS-SGHR-001..003`、`EVD-SGHR-001/002`。现有全局实例 API 是每页独立快照，
role 写点只锁本地 Principal；assignment 已有目录 admission。完整读权和跨服务最终离册
序列化均未证明。生产状态、部署绑定、产品测试与两顺序执行均 `NOT_EXECUTED/UNKNOWN`。

## 5. Observations

- `OBS-SGHR-001`：上述 Svc base，2026-11-08 UTC，本地 Git/文档/源码只读。
  `query_global_instances.rs::list_global_instances` 逐页 REPEATABLE READ；
  `provisioning_repository/mod.rs` 四处 role/owner 写点及成员/reconcile/离线同效入口见报告。
  现有 admission gate 文件顶注明确 provisioning replace_owner 不在 assignment 覆盖面。
- `OBS-SGHR-002`：Core PR #674 精确 head=`527d641518bdde61267cde56000bfb8299780667`，
  2026-11-08 UTC，经 GitHub connector 读文件；P3 要求覆盖、观察/线性化分离、两顺序及 accepted 配套。
  同 head `packages/product-api/src/workflow-admission.js`：`/v1/directory/agents/{agentId}`
  读取 `definition.listAgents()`；精确 missing 是 200 `exists:false`，disabled 是 `enabled:false`。
  此同步快照没有 Svc 锁、registry revision、holdings 证明或 lease。
- `OBS-SGHR-003`：同 Svc base 读取 §3 权威与 governance lock：adoption accepted，字节在 main；
  provisioning FROZEN，Coordinator accepted。来源 hash、检索方法、限定范围见
  [AUTHORING 报告](../reports/ghost-holdings-1351-p3/AUTHORING.md) 和 provenance。
  不声称每次读取有独立精确时钟；provenance 时间是证据封存时间。

## 6. Claims and assumptions

- `CLM-SGHR-001`，SUPPORTED，EVD-SGHR-001：扩 role admission 改变旧目标通过/失败语义，
  不能仅凭本 NEW 开始实现；接受须先处理 owning successors。不存在源码高于权威的假设。
- `CLM-SGHR-002`，SUPPORTED，EVD-SGHR-002：现有五秒 admission、DB 行锁和一次完整空读
  均不能证明 Core 离册与并发 assignment/role 提交互斥。因此本版本不提供删除许可。
  未知删除 callable 或锁的存在不被当成否定证明；若以后发现合法边界须按精确坐标另审。

## 7. Evidence relations

| Evidence | Source → Target / relation | Bound coordinates / sufficiency / limitations |
|---|---|---|
| EVD-SGHR-001 | OBS-SGHR-001/003 → CLM-SGHR-001、STATE-SGHR-001 / SUPPORTS | Svc base、§3 原文及报告源码表；足以定位及路由，不证明覆盖所有未来入口或现网已加载 |
| EVD-SGHR-002 | OBS-SGHR-001/002 → CLM-SGHR-002 / SUPPORTS | Svc admission/CIR 与 Core pinned handler/P3；证明不能把观测当 lease，不是并发实测通过或真实删除实现不存在的证明 |

## 8. Decisions

以下 Decision owner=Svc 合法接受者，均为 PROPOSED：

- `DEC-SGHR-001`：完整 holdings 观察与最终离册许可是两个不同结果；无法证明最终边界则拒绝删除。
- `DEC-SGHR-002`：新增/启用/恢复全部 Agent role 使用 CIR 的精确目录观测模型；human 不套 Agent registry。
- `DEC-SGHR-003`：既有行锁/单 owner/幂等仍约束 Svc 本地提交；不宣称 Core 与 Svc 共享锁。
- `DEC-SGHR-004`：用正式入口的反例定义约束未来实现，paired 接受及 owning successors 未齐时不开新面。

## 9. Contracts

以下 MUST 是拟议义务。机器索引仅索引本文条款，不表示测试已执行或机器能够替代语义独审。

```yaml
machine_readable_contracts:
  schema_version: 1
  status: proposed
  implementation_authority: none
  contracts:
    - {id: CTR-SGHR-001, acceptance: [ACC-SGHR-001, ACC-SGHR-002]}
    - {id: CTR-SGHR-002, acceptance: [ACC-SGHR-003, ACC-SGHR-004]}
    - {id: CTR-SGHR-003, acceptance: [ACC-SGHR-003, ACC-SGHR-004]}
    - {id: CTR-SGHR-004, acceptance: [ACC-SGHR-001, ACC-SGHR-004]}
    - {id: CTR-SGHR-005, acceptance: [ACC-SGHR-005, ACC-SGHR-006]}
    - {id: CTR-SGHR-006, acceptance: [ACC-SGHR-005, ACC-SGHR-006, ACC-SGHR-007]}
    - {id: CTR-SGHR-007, acceptance: [ACC-SGHR-004, ACC-SGHR-007]}
    - {id: CTR-SGHR-008, acceptance: [ACC-SGHR-008]}
```

### CTR-SGHR-001 — 完整 holdings 与合法读面

Svc MUST 对精确 Principal UUID 读取以下当前事实，或明确返回不可验证，不能返回可见子集为完整：

1. 所有 Domain 的实例：`current_node_visit_id` 指向本实例真实当前 Visit，
   `assignee_principal_id == subject`，当前 node 非 TERMINAL，`cancelled=false` 且 `archived_at IS NULL`。
   无论 Domain/Principal 是否 enabled，是否 WAITING、未到 nextEligibleAt、是否 eligible 或 quarantine，均计入。
2. `domain_role_bindings` 中该 subject 的**所有** `enabled=true` 行，包含 DOMAIN_OWNER、
   DOMAIN_MEMBER、WORKFLOW_ADMIN 及其他实际存储 role key；不套某一写 API 的白名单过滤历史债务。
3. `global_role_bindings` 中该 subject 的**所有** `enabled=true` 行，包含所有 reader/coordinator/auditor 等实际键。

历史 Visit、terminal/cancelled 实例、disabled binding 不计当前 holdings，历史不能删除。
非终态却 archived、有 open activation 却标终态/取消、缺当前 Visit/Node/Principal 关系、
重复/未知生命周期或不一致关联 MUST 报 `UNVERIFIABLE`，不能靠 inner join 漏掉并报零。
Principal 自己缺失仍须从实际 role/assignee 外键候选报告 unresolved 债务，不伪装空集合。

拟议 Svc HTTP 观察接口为 `GET /internal/v1/admin/holdings`，只允许无 query（全候选）或
一个 `principalId=<exact UUID>`（单 subject）；没有 body，未知/重复 query → 422 `invalid_input`。
全候选集 MUST 为 `principals` 与当前 assignee、enabled domain/global role subjects 的并集，
不能只遍历 Core 在册者或只遍历存在的 Principal。它供 C2 检测和 C1 取单 subject 观察，非正常 dispatch feed。

此接口**尚未注册/授权**。拟复用 provisioning 的 `workflow.admin`、allowlist、direct Agent token、
已 provisioned actor 限制与非敏感审计，不新增角色/Grant，不向普通 Core directory caller 下放 admin 权限。
须按 §3 先证明更高 Product 读范围及实际 caller 权限；不足时不挂载/不启用，调用得到既有拒绝/404，
Core 必须归为不可验证。选择目录的 baseline read grant、global reader、Domain Owner 均不自动获得该权限。
protected read 的审计必须在发布前完成；失败不泄露 subject 是否存在或任何 holdings。

成功响应采用现有 HTTP 序列化约定承载下列闭合 payload（本合同定义新 payload，不改旧端点）：

```yaml
schemaVersion: 1
scopePrincipalId: UUID_or_null  # null means the complete union
complete: true                # only complete results may be 200
readStartedAt: RFC3339_UTC
readCompletedAt: RFC3339_UTC
retirementSafety: UNPROVEN    # constant in V1; never a removal permit
subjects:
  - principalId: UUID
    principalType: AGENT_or_HUMAN_or_SERVICE_or_null
    principalEnabled: boolean_or_null
    instances:               # [] is legal
      - {domainId: UUID, workflowInstanceId: UUID, nodeVisitId: UUID, workflowStateVersion: integer}
    domainRoles:
      - {domainId: UUID, bindingId: UUID, roleKey: string, enabled: true}
    globalRoles:
      - {bindingId: UUID, roleKey: string, enabled: true}
```

nullable 本地类型/enabled 只表示缺投影，不证明 Auth 缺失、human、合法离册或无债务。
结果不带业务 Context/标题/Submission、token、credential、目录内网地址或新权限。
单 UUID 查询即使没有任何本地行也保留该 subject 的空数组与 null 身份；不得 404 假扮“无债务”。

### CTR-SGHR-002 — 所有新增/恢复角色边界

Svc MUST 在以下实际执行边界执行相同 Agent target admission，不能只补 HTTP handler：
`upsert_role_binding`、`upsert_global_role_binding`、`replace_domain_owner`、`establish_domain_owner`、
成员 `insert_member_binding`、reconcile `insert_role_binding`、可使旧 enabled binding 恢复生效的
Principal false→true，以及未来任何 create/enable/restore/import/recovery/离线同效写点。
新 key 的 enabled upsert 即使角色已经启用仍需当前检查；同 key 历史 replay 按 CTR-007。
Domain/Principal 恢复若会重新激活角色，须检查其全部受影响 Agent targets；不能用名称或入口差异绕过。
离线工具若缺当前合法 mandate/目录依赖必须拒绝该生效步骤，不能因为旧工具存在便获新生产权限。

target MUST 满足原有本地存在/enabled、类型、Domain/role、鉴权、CAS、单 enabled owner 约束；
AGENT 还须满足 CTR-003 的精确 Auth/Core 条件。已证明为合法 HUMAN 则沿原 human 规则，
不因 Agent registry 无 human 项拒绝；类型未知不能默认为 human。SERVICE 不获得 owner/assignment 新资格，
既有独立合法 global-role 资格也不被误当 Agent，按其 owning authority 处理。
replace/reconcile 必须在一个本地事务中保持旧 owner/role 到新 owner/role 的原子替换；
target admission 失败时旧 owner、所有 role 业务前态不变，不得先撤旧再读新。

### CTR-SGHR-003 — 权威、时间、缓存与失效

对每个 distinct Agent target，Svc 自己以现有 canonical backend SERVICE 身份及已合法取得的 baseline
目录权限，先读 Auth `GET /api/v1/directory/principals/{principalId}/agent` 精确关系/type/status/cardinality，
再读 Core **`GET /v1/directory/agents/{agentId}`**（由 `workflow-admission.js` 挂载的 `/v1/directory/agents` 前缀）。
Auth owns Principal 与 Agent 映射；Core Agent Definition 目录 owns 在册 existence 与 enabled。
通过条件为 Auth 单一精确关系且 active、请求/响应 UUID/Agent ID 一致、Core `exists=true AND enabled=true`。
Core missing 是合法 200 观察 `exists=false`，在角色 admission 中是拒绝；不得当网络失败或成功启用。
完整扫描无幽灵则是合法空结果；探索 rg 无命中则从源码实名继续，均不是执行失败。

复用 CIR CTR-CIR-003：每命令每 audience 新 token；固定受审 backend URL，禁止请求自带 URL、redirect、
跨 audience 转发 Workflow token、display name/别名/前缀推断；非 loopback 必须认证 TLS。
五秒以内完成 admission-through-COMMIT：从第一 admission 请求开始用单调时钟计时，本地 statement/
transaction deadline 不晚于该界限，最多八个在途读，无自动重试。
同命令同身份可共用仍有效观察；正/负结果均不跨命令缓存，不使用 stale-while-revalidate。
本地 Principal/Domain/目标 binding/owner preimage 或相关 Instance/Visit/Context tuple 漂移、
目录映射不一致、超时、未知字段/结构异常、预算到期都使观察失效，拒绝整笔新业务写入。
这五秒是观察可消费边界，不是 registry 不会变化的承诺。

### CTR-SGHR-004 — 观察时点、完整性和 C1 错误适配

holdings MUST 在同一个 Svc REPEATABLE READ 事务中读取全部三维及完整性检查，稳定按 UUID/role key 排序；
从开始读取到完成发布最长五秒，超时/审计失败则不给成功 payload。V1 接口无分页：不得截断、默认 limit、
返回下一页前缀为完整、或把多个独立 page snapshot 拼成同一快照。
既有 `GET /internal/v1/workflow-instances/global?assigneePrincipalId=UUID&lifecycle=active&status=active`
可作诊断/对照；其 default20/max100、每页独立事务和 INNER JOIN 不能证明本文全量/完整性。
数据库一致性事务不是 Core registry 快照；`readCompletedAt` 不是 lease、revision、CAS 或未来空集合保证。

| 观察/失败 | 拟议 Svc HTTP code 或适配 | Core C1 逻辑类 / 动作 |
|---|---|---|
| 完整且任一数组非空 | 200，真实完整列表 | HOLDINGS_PRESENT / 拒绝离册 |
| 完整且空，retirementSafety=UNPROVEN | 200，真实空列表 | HOLDINGS_UNVERIFIABLE（最终边界未证明）/ 拒绝离册，不能把空查询称故障 |
| DB/审计/超时/完整性检查失败 | 503 `holdings_unverifiable`，无部分成功 payload | HOLDINGS_UNVERIFIABLE / 拒绝离册 |
| 未启用/旧版本/未知响应/缺字段/缺读权 | 保留原 404/401/403 等，不借读面改身份 | HOLDINGS_UNVERIFIABLE / 拒绝离册 |
| Auth canonical subject 关系无法证明，包括 disabled subject 的关系不明 | holdings 仍可报告；Core 独立身份检查拒绝 | SUBJECT_IDENTITY_UNVERIFIABLE / 拒绝离册 |
| Core 最终 registry/target 前态变化 | 原合法 registry 冲突，Svc 不伪造 registry version | SUBJECT_CHANGED / 拒绝离册 |

Core C2 仅对已证明 Agent 关系调用目录：missing→NOT_IN_REGISTRY；disabled→IN_REGISTRY_DISABLED；
映射/查询不明→INSPECTION_INCOMPLETE，不报 clean。Svc 不代 Core 判断幽灵，也不产生删除副作用。

### CTR-SGHR-005 — Svc 本地提交与最终离册强门分离

Svc MUST 保持既有命令 receipt/Instance/Domain 锁顺序、Principal enabled 行锁/前态复核、单 owner 索引、
事务与版本/Event/审计规则。assignment 仍由既有合法 create/transition/RETURN/recovery/有界 migration
形成，绝不由 holdings API 改派旧 Visit。Core MUST 保持真实 registry mutation 的原鉴权/前态复核。

这些本地机制**没有共同锁住两个存储**。若 W 是新增 assignment/role/恢复责任提交，R 是最终离册：
仅持 Svc Domain/Principal 行锁不能阻断 R；仅持 Core 文件锁不能阻断 W；再读一次 holdings 仍有
再读至 R 的窗口。现有 admission 也允许“先读有效目录，稍后 registry 改变”。
因此本 V1 响应 `retirementSafety` 固定 UNPROVEN，不能由 caller 改为 PROVEN。
在真实删除 callable、所有 W/同效 registry mutation 入口和合法序列化/复核边界得到逐项证明前，
Core C1 MUST 保持拒绝删除，即使刚读到完整空集合。只证明拒绝分支不等于退役可用性验收通过。

若已有合法边界可实现，后续 owning authority 必须绑定其精确 path/symbol/revision、参与者和作用域、
真实跨边界前态/版本检查、竞争时拒绝点与 UNKNOWN 恢复。任何新协议/权限/锁/lease 或改变旧失败语义
先 docs-first 处理，不由本合同虚构。须在两真实提交顺序均证明不变量后另次修订本响应/激活合同：
不能有“R 已成功而同时尚有应保留的 W 责任且无合法处理”的状态。

### CTR-SGHR-006 — 两提交顺序反例义务

未来验收 MUST 用可控 barrier 和真实 command/registry callable（非 helper-only），对 assignment、
domain role/owner、global role、Principal/Domain restore 各运行以下两序列，记录提交与否及实际提交序：

- **W 先**：R 先取得空观察；W 新建/恢复责任并 COMMIT；R 再到最终 mutation。
  R 必须复核后拒绝或被合法共同边界阻止，保留身份及 W 责任。旧空观察、错误 cursor、已有文件锁
  均不能放行 R。必须另试 registry disable/enable/整表替换在中间发生，前态漂移不得悄然通过。
- **R 先**：W 已取得在册 enabled 观察但尚未 COMMIT；R 尝试最终离册；W 再提交。
  若边界已合法证明 R 可先成功，则 W 必须因最终保护/失效而拒绝并零业务 delta；
  若无该保护，R 必须被拒绝/等待而不成功。在本 V1 未证明状态，R 必须拒绝。

两序列均要测试首次读与复读后插入、不同 Domain、不同 role、恢复、多个请求及 mutation 回包丢失。
仅顺序单元测试、五秒内的读取、同一个连接串行执行、漏 registry 入口，或只证明“通常能拒绝”均不通过。
registry 变更的观察晚于 W 的普通 disable 情形仍按既有 no-send/巡检处理，不承诺 disable/W 跨服务原子性。

### CTR-SGHR-007 — 错误、降级、幂等与历史

原本地错误/鉴权/CAS 先按原合同执行并保留映射；以下是**role 扩围的拟议映射**，不是当前 role API 已支持：

| 远端结果 / Core 逻辑类 | Svc HTTP / code | 业务效果 |
|---|---|---|
| authoritative missing / TARGET_NOT_IN_REGISTRY；其他身份不一致 | 422 `admission_rejected` | 拒绝新角色，零业务 delta |
| Auth disabled 或 Core enabled=false / TARGET_DISABLED | 422 `admission_target_disabled` | 同上 |
| 读取超时、directory 拒绝 | 422 `admission_timeout` / `admission_denied` | TARGET_LIVENESS_UNVERIFIABLE，零业务 delta |
| 配置/不可用/响应畸形 | 503 `admission_configuration` / `admission_unavailable` / `admission_malformed_response` | TARGET_LIVENESS_UNVERIFIABLE，零业务 delta |
| 本地 exact preimage 漂移 | 原入口 409 冲突，例如 reconcile 的 `binding_conflict` | 丢弃观察，零业务 delta |
| COMMIT 已发出但结果无法证明 | 503 `admission_commit_outcome_unknown` 或原命令 outcome_unknown | **业务效果 UNKNOWN**；不能声称零写入，先读回原 receipt/状态，禁止换 key 重放 |

本地 missing/disabled 仍为各入口原码，例如 provisioning `principal_not_found`/`principal_disabled`，
reconcile `identity_not_found`/`principal_disabled`；不把本地错误改报 registry missing。
错误沿现有 ApiError envelope，响应不含 token、上游 body、内网地址；逻辑原因不能靠名字匹配作判定。
admission 确定失败回滚全部新业务事实及成功 receipt，不缓存 admission 拒绝供后续命令复用；
既有独立安全审计保留，不另造业务 Event/receipt。成功 receipt 的同 key/body 历史 replay 返回原事实，
不按今日目录重算；不同 body 保留 idempotency_conflict，PROCESSING 不接管。

权限减少的 binding disable/revoke、Principal/Agent disable 不加本合同目标活性或空 holdings 前置；
仍保留原鉴权和原业务条件。replace 校验接收 target，不要求 source enabled/在册。
原有 holdings、历史、UNKNOWN、fence、managed STOP/OWNER_CHANGED_UNSUPPORTED 不删不继承为新预算。
无合法过户命令时记录 unresolved/park、拒绝离册；本配套不能授权通用实例过户。

### CTR-SGHR-008 — 配对、接受与实施前置

正式接受前 MUST 关闭 §3 owning successors/回链、更高完整读权限、接口和全部写点审查，以及
最终离册边界的合法方案和两顺序可复核证明；不能在这些 load-bearing 缺件仍 UNKNOWN 时标 accepted。
权威审查阶段的证明可由精确源码边界、时序论证及可执行验收定义承载；真实两顺序运行另属
后续受授权实现的启用门，不能先写未经授权的产品实现再倒推合同接受。
Svc/Core 各自非作者语义独审及各自真实权限的接受者不可互相替代，PR review PASS 不等于接受。
接受记录须绑定双方 ID/Contract IDs、文档 SHA256、reviewed semantic HEAD、reviewer 身份独立性、
接受者及权限来源、明确 `none→contracts` 的另次 docs-only 事务、最终 accepted HEAD、main SHA，
以及各实施 base 包含上述 accepted authorities 的证明。保持 none 则合 main 也不授权代码。
实际需要的外部 directory 权威/权限及 registry owner 必须独立核验；Core P1/P2/P4 缺件仍由 Core 关闭。
后续实现另次 RE_PREFLIGHT/REUSE；source PASS、部署 health、合同接受、合 main 不产生生产 apply 权限。

## 10. Acceptance

以下都是未来定义。除本次文档检查外，产品/持久集成/并发/生产结果全部 **NOT_EXECUTED**。
每项 evidence 必含 authority+implementation exact refs、环境/配置/输入、实际入口、barrier 时序、
HTTP/body/rc、调用次数、业务前后态/receipt/audit hashes、权限与独立复核记录；计划或 mock helper 不代执行。

| Acceptance | Contracts | Method | Environment | Required evidence | Expected result / Failure condition |
|---|---|---|---|---|---|
| ACC-SGHR-001 完整读 | CTR-SGHR-001/004 | 实际读入口：>100实例、多 Domain、disabled Principal/Domain、WAITING、未到期、所有实际 role keys、仅 domain/global role、仅历史/disabled role、异常关联 | 未来隔离持久 Svc | exact全集 fixture 与单事务结果比对、权限负控、审计/timeout trace | 全集相等；历史不算；异常不可验证。少页/inner join丢债/错误空200/漏role/越权泄漏失败 |
| ACC-SGHR-002 C1/C2 | CTR-SGHR-001 | Core实际适配：精确 disabled source映射、类型未知、人类、幽灵、零幽灵、目录失败及候选集并集 | 未来隔离Core+Svc+Auth | subject解析链、候选全集、报告、零mutation计数 | missing/disabled/incomplete分开；完整零幽灵合法。只遍历在册者、把未知当human/clean、空探索当执行失败即失败 |
| ACC-SGHR-003 全写点 | CTR-SGHR-002/003 | 报告全入口逐项 ghost/disabled/合法Agent/HUMAN、unknown类型，含成员/reconcile/restore/离线工具 | 未来真实HTTP→application→repository集成 | 入口覆盖矩阵、目录请求、旧新owner/role/receipt前后态 | 新幽灵owner与所有角色被拒；source disabled仍可合法交出；失败保留旧owner。仅handler/helper测过或漏入口失败 |
| ACC-SGHR-004 失败/缓存 | CTR-SGHR-002/003/004/007 | 每命令新观测、过期/5秒边界、错ID/重复映射/redirect/错audience/目录denied/畸形/DB漂移/审计失败 | 未来隔离受控故障注入 | wire codes、token audience脱敏证明、时钟/事务deadline、调用计数和业务diff | 确定失败零delta、无跨命令缓存/自动retry，旧本地码保持。超时误成功、先撤旧owner、缓存成功/拒绝失败 |
| ACC-SGHR-005 W先反例 | CTR-SGHR-005/006 | CTR-006 第一序列：空读后及复读后插入每类W；registry各同效变更插入 | 未来真实Svc+registry并发fixture | 双命令真实barrier/commit顺序、W已提交读回、R结果、共同边界证明 | R不穿过新责任；身份/债务保留。读旧空放行、漏恢复/不同域、只同连接串行失败 |
| ACC-SGHR-006 R先反例 | CTR-SGHR-005/006 | CTR-006 第二序列：W目录观察后暂停，R尝试离册后放W；含丢回包 | 同上 | R真实结果、W提交/回滚、registry+holdings最终读回、receipt | V1未证明则R拒绝；以后合法R先成功须W零delta。R成功且W留下责任、UNKNOWN假零/换key重放失败 |
| ACC-SGHR-007 兼容/处置 | CTR-SGHR-006/007 | 非空/目录故障下disable、revoke、disabled source交出、历史replay、PROCESSING/UNKNOWN、managed Owner replacement、无合法过户命令 | 未来隔离正式命令 | GC/目录调用计数、历史/fence/预算diff、原receipt及STOP | 无新增disable门；历史保留；UNKNOWN读回；无命令则未完+零离册。隐式改派/重放/继承预算失败 |
| ACC-SGHR-008 文档与接受 | CTR-SGHR-008 | frontmatter/schema/13节/双向覆盖、非作者exact-head提案审查；未来逐项核验接受/各base | 本次本地docs；未来独立接受事务 | validation、固定sources/hash、review head、successors/权限/双方acceptance/main/base证据 | 本轮proposed/none/production-none且代码diff=0；未来缺前置不开闸。自接受或把提案PASS写成P3闭合失败 |

双向覆盖：001→001/002；002→003/004；003→003/004；004→001/004；
005→005/006；006→005/006/007；007→004/007；008→008。
**8/8 Contracts 有定义覆盖；0 项产品验收被本次执行。**

## 11. Alternatives and disposition

- REJECT：普通 my_tasks/global list 子集为空即离册；缺 role/跨页/权限完整性。
- REJECT：五秒目录成功、Core 文件锁、Svc Domain 锁或二次读取等于分布式 lease；两序列可证伪。
- REJECT：仅改四个 provisioning handler；成员、reconcile、恢复和离线同效入口能绕过。
- REJECT：在旧 ID 下加 registry 拒绝或错误，称 additive；违反完整继任边界。
- RETAIN：Auth 映射、Core registry、Svc 责任事实各自归属，原命令/历史/UNKNOWN/单owner/隔离。
- REJECT：新 lookup 身份、影子映射库、隐式 admin fallback、自动任意改派或自动清债。

## 12. Migration, compatibility, and rollback

本次无 Rust/行为测试/schema/配置/部署/生产变化，原 root Brief 与 worker.log 保留。
提案回退为撤回本次文档，不改旧 authority。现存幽灵只是待合法处置债务，不自动修复或永久豁免。
未来旧 binary/缺配套/缺读权返回不支持或不可验证，Core 不得 fallback 到部分 worklist 后放行。
未来恢复/回退保留已提交 receipt、历史/过户事实、身份 tombstone、fence 和未完项，不能还原已退役权限。

## 13. Open questions and AUTHOR output

缺件是接受/实施前置，不是本次 docs proposal 的必需输入缺失。未证明的读权限/真实删除共同边界
不能靠本作者写一句“已解决”补齐；本候选可供非作者审查，保持不可接受、不可实施。

```text
SPEC_GOVERNANCE_MODE = AUTHOR
AUTHORITY_ACTION = NEW
SPEC_ID = SVC_WORKFLOW_GHOST_HOLDINGS_ROLE_LIVENESS_V1
STATUS = proposed
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V13
EXTERNAL_AUTHORITIES = exact pins in frontmatter; Core GHP remains proposed
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = CONTROLLED
DOCS_FIRST_REQUIRED = YES
OPEN_OWNER_DECISIONS = owning successors and full-read authorization acceptance, not performed here
NORMATIVE_TBD = lawful retirement serialization proof/activation; V1 always refuses unproven removal
PARTIAL_SUPERSESSION = NONE (no replacement activated; owning whole successors required)
UNRESOLVED_AUTHORITY_CONFLICT = dependent behavior cannot activate before owning successors
CONTRACT_COUNT = 8
CONTRACTS_WITH_ACCEPTANCE = 8
AUTHORING_READY_FOR_REVIEW = YES (proposal only)
READY_TO_MARK_ACCEPTED = NO
IMPLEMENTATION_ALLOWED = NO
MERGE_READY = NO
OPERATION_ALLOWED = NO
EVIDENCE_NEEDED = independent exact-head review; owning successors; lawful full reader and ordering proof; paired acceptance
EXPANSION_TRIGGER = new code/permission/protocol/production request or attempt to bypass owning authority
NEXT_ACTION = REVIEW
```
