---
spec_id: SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1
title: GLOBAL_WORKFLOW_COORDINATOR Control Plane V1 (domain admin, member governance, cross-domain cancel/archive, binding reconcile)
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: none
production_apply_authority: none
scope:
  - mayf3/svc-workflow
  - GLOBAL_WORKFLOW_COORDINATOR governance control plane (server-side role gates only)
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V7
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_1
companion_specs:
  - mayf3/svc-workflow SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1 (accepted; this Spec
    AMENDS its §3 COORDINATOR column and supersedes its §8 grantee rationale per
    NEW_EVIDENCE — see §2 and §8)
  - mayf3/svc-workflow SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2 (accepted;
    identity authority context — Auth owns principal mapping; this Spec adds NO
    identity authority and NO instance-assignee repair surface)
external_authorities:
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_EXACT_PRINCIPAL_AGENT_RESOLUTION_V2
    relation: downstream_consumer
    note: exact Principal UUID -> canonical enabled Agent resolution is owned
      downstream; this Spec consumes exact UUIDs only and pins NO dsh head
      (same downstream-pin discipline as SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1 §9)
supersedes: []
superseded_by: null
owners:
  - mayf3
date: 2026-09-09
product_direction: WORKFLOW_COORDINATOR_CONTROL_PLANE_V1 (Owner goal directive, 2026-09-09)
role_applied_by_this_spec: false
database_migration_required: false
---

# SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1

> **STATUS = proposed (docs-only Draft PR).** 本轮只提交 docs-only Draft PR，
> 不实现、不接受、不 merge、不做任何 role grant、不做任何 production mutation。
> `implementation_authority: none`、`production_apply_authority: none`。
> 独立语义 review + Owner acceptance 后方按本仓库 governance 走 acceptance
> 事务（frontmatter 翻转 + implementation authority 激活）。

## 1. Goal

Owner Goal Directive `WORKFLOW_COORDINATOR_CONTROL_PLANE_V1`（2026-09-09）冻结了
产品方向：`GLOBAL_WORKFLOW_COORDINATOR` 是正式的 Workflow 治理控制面角色——
当前实际 holder 为 HR main principal——允许它跨 Domain 执行治理操作：

1. Domain discovery / metadata management（list / get / update displayName）；
2. Domain provisioning（已有，保持不变）；
3. get/set（replace）Domain Owner（set 已有，get 缺失）；
4. 跨 Domain 管理 `DOMAIN_MEMBER` bindings；
5. 跨 Domain 对 Workflow instance 执行正式 cancel / archive；
6. 受控 canonical binding reconciliation（plan / apply）；
7. 所有 mutation 保持 server-side authorization、Idempotency-Key、
   durable receipt/audit、read-after-write。

本 Spec 是上述方向在 svc-workflow 侧的**最小正式 delta**。它不改变任何
业务生命周期语义（cancel/archive 的 lifecycle legality 逐字不变），只：

- **W**：把三组既有端点的 application-layer 授权从 `DOMAIN_OWNER`-only
  放宽为 `DOMAIN_OWNER OR GLOBAL_WORKFLOW_COORDINATOR`（W1 cancel、
  W2 archive、W3 member list/add/remove）；
- **N**：新增 coordinator 面的窄读/治理端点（N1 domain list、N2 domain get、
  N3 domain update、N4 get owner、N5/N6 binding reconcile plan/apply）。

一切授权判断由 svc-workflow server-side role bindings 完成；JWT 只携带
coarse scope；下游 Broker（dsh-agent-core，另行 Spec）不复制 role logic。

## 2. Amendment relations and NEW_EVIDENCE

**Amends `SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1`（accepted）**，两处：

1. §3 permission matrix 的 `GLOBAL_WORKFLOW_COORDINATOR` 列扩展：
   新增本 Spec §9 CTR-CM-001 冻结的放宽行与新增行；READER 列与
   `SERVER_GATE (global list) = GLOBAL_WORKFLOW_READER OR
   GLOBAL_WORKFLOW_COORDINATOR` 逐字不变。
2. §8 rejected alternative「Coordinator to HR main — rejected: role
   conflates read with domain-management write gates; final model grants
   coordinator to NOBODY in this family (HR_GLOBAL_COORDINATOR = NO …)」的
   裁决理由被本 Spec 撤销：

```text
NEW_EVIDENCE = Owner Goal Directive WORKFLOW_COORDINATOR_CONTROL_PLANE_V1
               (2026-09-09) §2 FROZEN:
               "GLOBAL_WORKFLOW_COORDINATOR = Workflow global governance
               control-plane role …… 这是一项有意的权限模型扩展";
               并强制 server-side role checks + Idempotency-Key + durable
               receipt/audit + read-after-write + negative security tests
               (directive §13/§15)，即当年拒绝理由所担心的 blast radius
               现由显式合同约束接管。
```

按 `.agents` standing order 第 5 条，未附 NEW_EVIDENCE 不得重开 rejected
alternative；本节即该 NEW_EVIDENCE 的显式登记。READER_V1 的 READER 角色、
两个 grantee、§4 改动闭包**全部保持 accepted 有效**，本 Spec 不 supersede
READER_V1。

## 3. Scope and non-goals

**In scope（accept 后的实现面；全部为受控代码改动，零 migration）：**

- `src/application/workflow_instance/cancel.rs`、`archive.rs`（事务层 gate
  谓词放宽 + 错误语义保持）；
- `src/application/domain_membership/mod.rs`（三函数 gate 谓词放宽）；
- `src/http/handlers/coordinator_domains.rs`（新增 list/get/update/get_owner）；
- `src/http/handlers/domain_members.rs`（gate 放宽的 handler 侧配套）；
- `src/application/domain_binding_reconciliation/`（新模块：plan/apply）；
- `src/store/postgres/`（对应只读/事务查询）；
- `src/http/error.rs`（新错误码映射）；
- `src/http/mod.rs`（新路由注册）；
- 单元 + conformance 测试。

**Non-goals（显式冻结，与 Owner directive §10/§15 一致）：**

- `WORKFLOW_DOMAIN_DELETE = OUT_OF_SCOPE`；`WORKFLOW_INSTANCE_DELETE =
  OUT_OF_SCOPE`；无 retention purge、无 bulk SQL、无 admin cleanup endpoint。
- 不复用、不放宽、不代理任何 `/internal/v1/admin/*` 面（admin allow-list
  门禁逐字不变）。
- 不改变 cancel/archive 的业务语义与 lifecycle legality
  （only terminal/cancelled 可 archive；active 才能 cancel）——本 Spec 只动
  「谁被允许调用」，不动「调用之后发生什么」。
- 不改变 transition/assignee 授权（仍 assignee-gated）。
- Domain `enabled` flip **不纳入 V1**（N3 仅 displayName；disable 语义涉及
  active-instance 安全 invariant，留 §13 open question）。
- 不新增 identity authority；不做实例 assignee 修复（那是
  CANONICAL_IDENTITY_RECONCILIATION_V2 的 frozen scope）。
- Role grant（GLOBAL_WORKFLOW_COORDINATOR → HR）**不在本 Spec 内执行**；
  §11 只冻结 grant plan，apply 是 separately owner-authorized production step。
- 不做 `if agent == HR` / principal 硬编码特判；角色判定只查
  `global_role_bindings` / `domain_role_bindings`。

## 4. Authority and dependencies

- Product/arch authority：`SVC_WORKFLOW_PRODUCT_BOUNDARY_V7` +
  `SVC_WORKFLOW_ARCHITECTURE_V0_4_1`（current main authorities）。
- Amendment target：`SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1`（accepted，
  reviewed head f900586f…）——§2。
- Identity context：`SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2`
  （accepted）——Auth 是 principal mapping 唯一 owner；本 Spec 的
  reconcile 输入是**已经过下游 canonical discovery 的 exact UUID**，服务端
  只做 existence/enabled 校验，不做名字解析。
- Downstream（不 pin head）：dsh-agent-core broker 能力面另行 Spec；
  本 Spec 冻结的 wire contract 是其上游输入。
- Dependency direction：无上游新增依赖；auth-service 无需改动
  （GLOBAL_WORKFLOW_COORDINATOR role key 已在 admin
  global-role-bindings 白名单内，READER_V1 §4 item 5）。

## 5. Current State（capability census, fresh read-back @ github/main 4bbbbe9）

按 Owner directive §3 的 A/B/C/D 分类（fresh read-back 于 2026-09-09）：

| # | Capability | Class | Evidence（fresh read-back） |
|---|---|---|---|
| S1 | Global instance read | A | `GET /internal/v1/workflow-instances/global`；gate=`GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR`（READER_V1 §4 已部署语义；query_visibility.rs） |
| S2 | Coordinator create domain | A | `POST /internal/v1/domains`（coordinator_domains.rs `create_domain`：`workflow.execute` + `require_direct_token` + `require_global_coordinator` + Idempotency-Key receipt） |
| S3 | Coordinator set owner | A | `PUT /internal/v1/domains/{domainId}/owner`（`set_domain_owner`，同门禁，`replace_owner` 原子替换） |
| S4 | Coordinator domain list/get | C | 无端点；`domains` 表列齐全（domain_key UNIQUE, display_name, enabled, created_at, updated_at） |
| S5 | Coordinator domain update | C | 无端点 |
| S6 | GET domain owner | C | 无端点；仅 `PUT` 存在 |
| S7 | Member list/add/remove | B | `GET/POST/DELETE /internal/v1/domains/{domainId}/members(/{principalId})` 存在；application 层 `check_domain_owner`-only → `NotDomainOwner` |
| S8 | cancel/archive | B | 端点+幂等+receipt 在（`compute_cancel_request_hash` / `compute_archive_request_hash`，`workflow_command_receipts`）；application 层 DOMAIN_OWNER-only（「Only DOMAIN_OWNER may cancel instances in their domain」）；错误表齐（`not_domain_owner`/`already_cancelled`/`instance_archived`/`invalid_reason`/`idempotency_conflict`/`command_still_processing`） |
| S9 | Binding reconcile plan/apply | C | 缺；DB 兜底 = `idx_drb_single_owner` 部分唯一索引（at most one enabled DOMAIN_OWNER per domain）+ `idx_drb_domain_principal_role`（per (domain,principal,role) 唯一） |
| S10 | Receipts / audit | A | `workflow_command_receipts` + `workflow_command_attempt_audits` + `workflow_security_audits`（migrations/0005）；`command_type` 为自由 TEXT（新命令种类零 migration） |
| S11 | `/internal/v1/admin/*` | D | admin principals/domains/role-bindings/global-role-bindings 面——本 Goal **不得直接复用**（Broker 面一律走 §9 新窄端点） |

## 6. Observations

- OBS-CP-001 — cancel/archive 的 HTTP 层只查 `workflow.execute` scope；
  DOMAIN_OWNER 判定在 application 事务层（`cancel.rs`/`archive.rs` 头注 +
  事务内 `check_domain_owner` 等价校验），因此放宽点唯一且在事务层。
- OBS-CP-002 — member add 已幂等（重复 add 返回成功语义）、remove 只移除
  `DOMAIN_MEMBER`（显式不影响 `DOMAIN_OWNER`），并有
  `principal_not_registered`/`principal_disabled`/`principal_is_owner`
  目标校验——B 类放宽只需动「caller 是谁」，不动目标校验。
- OBS-CP-003 — `workflow_command_receipts` 的幂等键唯一索引是
  `(principal_id, idempotency_key)`；replay 返回原 outcome 的机制由
  receipt 状态机承载（同 cancel/archive/create_domain 现状）。
- OBS-CP-004 — `idx_drb_single_owner` 在 DB 层强制单 enabled owner；
  任何 owner 迁移必须单事务内 disable 旧 + establish 新（`replace_owner`
  已是该形态）。
- OBS-CP-005 — READER_V1 §6 记录 HR main principal =
  `dc702687-6515-4a2a-91ae-e572a9bbd766`（agt_hr-agent），持
  machine_access_grants v2 = {workflow.read, workflow.execute}——即 HR 无需
  任何新 scope 即可在获得 role binding 后调用本 Spec 全部端点（所有端点
  scope 要求 ∈ {workflow.read, workflow.execute}）。

## 7. Claims and assumptions

- CLM-CP-001 — 三组 B 类放宽是纯谓词扩展：`is_owner(domain) OR
  is_coordinator()`，两者皆为既有 enabled-binding 查询，无新表无新迁移。
- CLM-CP-002 — C 类新端点全部可以复用既有 receipt/audit 机制（写面）与
  既有查询（读面），不需要 schema 变更。
- CLM-CP-003 — 放宽后 DOMAIN_OWNER 的既有能力与全部 negative 边界保持
  字节级不变（owner 仍不能 set owner、不能管他域、不能 reconcile）。
- ASSUMPTION-CP-001 — HR main identity 的 `workflow.execute` token 铸造
  能力保持现状（该能力已由 READER_V1 §2 记录并被 Owner 方向接受为前提）。

## 8. Decisions

- DEC-CP-001（W 门禁谓词）— cancel/archive/member 的授权谓词放宽为
  `caller is DOMAIN_OWNER(instance.domain) OR caller holds enabled
  GLOBAL_WORKFLOW_COORDINATOR`；判定输入只有真实 instance.domainId（从
  instance 行读出）与 caller credential principal，**绝不接受 model/请求体
  提供的 domainId 作为授权依据**。
- DEC-CP-002（错误码保持）— 既有错误码逐一保持：caller 两者皆无时
  cancel/archive 仍 403 `not_domain_owner`、member 面仍既有
  NotDomainOwner 映射码。不加新「角色不足」码，避免下游双码过渡
  （与 READER_V1 §5 的双码教训相反，这里无部署过渡窗口需求——谓词放宽
  与部署同事务生效）。Broker 侧 declarer 继续声明既有码。
- DEC-CP-003（N3 update 范围）— V1 仅 `displayName`（1..256、trim、无控制
  字符——复用 create_domain 的同套校验）；`domainId`/`domainKey`/`enabled`
  不可改。写面，走 Idempotency-Key + receipt（command_type =
  `domain.update`）。
- DEC-CP-004（N4 get owner 授权）— `workflow.read` scope +（coordinator OR
  该 domain 的 enabled DOMAIN_OWNER）；返回
  `{domainId, ownerPrincipalId, ownerDisplayName|null, ownerEnabled}`；
  domain 不存在 → `not_found`；无 enabled owner → 404
  `domain_owner_missing`。不返回成员列表、不返回业务正文。
- DEC-CP-005（reconcile 形态）— 只做窄 plan/apply，不做通用 role-binding
  writer：输入必须 exact UUID（domainId/fromPrincipalId/toPrincipalId/role
  ∈ {DOMAIN_OWNER, DOMAIN_MEMBER}/reason）；plan 纯只读并列 blockers；
  apply 校验 exact preimage（source binding 仍 enabled 且 binding_id 一致、
  target principal 仍存在且 enabled、target 无该 role enabled binding），
  任一不匹配 → 409 `binding_conflict` 零 mutation。
- DEC-CP-006（owner reconcile 复用 owner-swap）— role=DOMAIN_OWNER 的
  apply 复用 `replace_owner` 原子语义（单事务 disable 旧 + establish 新，
  OBS-CP-004）；role=DOMAIN_MEMBER 的 apply 在单事务内完成旧 binding
  disable + 新 binding establish，杜绝半迁移。
- DEC-CP-007（identity 错误码）— 服务端输入是 exact UUID：目标/来源
  principal 不存在或 disabled → `identity_not_found`（404/422 按 hit 面）。
  `identity_ambiguous` 是下游 discovery 面（name→UUID 搜索）的错误码，本
  服务端按 exact UUID 查询**不可能**产生歧义，不声明该码。
- DEC-CP-008（reconcile 授权）— plan 与 apply 都 coordinator-only
  （DOMAIN_OWNER 不需要、也不获得 reconcile 面；与 directive §15 一致）。
- DEC-CP-009（读面 scope）— N1/N2/N4 用 `workflow.read`；N3/N5/N6 与全部
  W 面用 `workflow.execute`；一律叠加 `require_direct_token`（与既有
  coordinator 写面一致，拒绝 OBO）。

## 9. Contracts

### CTR-CP-001 — Authorization matrix（amends READER_V1 §3 COORDINATOR column）

| Surface | GLOBAL_WORKFLOW_READER | DOMAIN_OWNER(own domain) | GLOBAL_WORKFLOW_COORDINATOR |
|---|---|---|---|
| global list | ALLOW（不变） | — | ALLOW（不变） |
| cancel/archive instance | DENY | ALLOW（不变） | **ALLOW（本 Spec 放宽）** |
| member list/add/remove | DENY | ALLOW own domain（不变） | **ALLOW cross-domain（本 Spec 放宽）** |
| domain list/get | DENY | DENY（不变） | **ALLOW（新增）** |
| domain update (displayName) | DENY | DENY（不变） | **ALLOW（新增）** |
| get owner | DENY | ALLOW own domain（新增，窄） | **ALLOW（新增）** |
| create domain / set owner | DENY | DENY（不变） | ALLOW（不变） |
| binding reconcile plan/apply | DENY | DENY | **ALLOW（新增）** |
| transitions / assistance / admin / scheduler | 不变 | 不变 | 不变（admin 仍 DENY） |

DOMAIN_MEMBER：以上治理面全 DENY（不变）。负向边界（directive §15）由
ACC-CP-004..006 逐条验收。

### CTR-CP-002 — Wire contracts（freeze）

```text
GET    /internal/v1/domains?limit&beforeCreatedAt&beforeId
GET    /internal/v1/domains/{domainId}
PATCH  /internal/v1/domains/{domainId}              body {displayName}
GET    /internal/v1/domains/{domainId}/owner
POST   /internal/v1/domains/{domainId}/binding-reconcile/plan
       body {role, fromPrincipalId, toPrincipalId, reason}
POST   /internal/v1/domains/{domainId}/binding-reconcile/apply
       同 plan body；Idempotency-Key 必带
```

- domain list/get 返回最小治理 metadata：
  `{domainId, domainKey, displayName, enabled, createdAt, updatedAt}`；
  list 为 keyset cursor（`beforeCreatedAt`+`beforeId` 成对，沿用 repo 惯例；
  无 offset/page/total）。
- get owner 返回 `{domainId, ownerPrincipalId, ownerDisplayName|null,
  ownerEnabled}`（ownerDisplayName 取 principals projection，缺省 null）。
- reconcile plan 返回只读判定：
  `{sourceBindingExists, sourceEnabled, targetPrincipalExists,
  targetPrincipalEnabled, targetHasEnabledBinding, singleOwnerInvariantOk,
  plan, blockers[]}`；plan 恒 200（blockers 以列表表达，非错误）。
- reconcile apply 返回 `{outcome: applied|already_applied|noop, …}`；
  幂等 replay 返回原 outcome（OBS-CP-003）。

### CTR-CP-003 — Error table（新增码，全部进 Broker declarer 表）

```text
domain_owner_missing      404  无 enabled DOMAIN_OWNER（get owner / plan）
binding_conflict          409  preimage 不匹配 / target 冲突 / 单 owner
                               invariant 冲突（apply）
identity_not_found        404|422  reconcile 输入 principal 不存在或 disabled
invalid_input             422  字段校验失败（displayName/reason/role/UUID）
（既有码全部保持：not_found / forbidden / not_domain_owner /
 already_member 语义（幂等 add 成功路径） / already_cancelled /
 instance_archived / idempotency_conflict / command_still_processing /
 invalid_cursor / global_coordinator_required / invalid_reason …）
```

不暴露 raw SQL / storage detail（error envelope 四要素不变）。

对 Owner directive §12 错误码清单的显式映射（"至少稳定保留"逐项对账）：
`not_found`/`forbidden` = 既有通用码，保持；`already_member` = member add
的幂等成功语义（非错误码，OBS-CP-002），response 稳定可判定；`domain_owner_missing`
= 新增（CTR-CP-002）；`identity_not_found` = 新增（DEC-CP-007）；
`identity_ambiguous` = 属下游 discovery 面（name→UUID），服务端 exact-UUID
输入不可能歧义，不声明；`idempotency_conflict` = 既有，保持；
`invalid_state` = 既有 lifecycle conflict 家族码承载
（`already_cancelled`/`instance_archived`/`instance_not_terminal`/
`already_archived`——语义更精确，全部保持）；`binding_conflict` = 新增
（DEC-CP-005）。

### CTR-CP-004 — Audit / idempotency（directive §13）

全部 coordinator 写面（W1–W3 复用既有、N3/N6 新增）必须：
authenticated actor + server-side role check + Idempotency-Key +
`workflow_command_receipts` durable receipt（新 command_type：
`domain.update` / `domain.binding_reconcile`）+ security/attempt audit 落
既有三表 + response 可 read-after-write。同 key replay 返回原 outcome、
零重复 mutation；新 logical request 撞已完成状态返回稳定业务 outcome
（member add 幂等语义，OBS-CP-002）。

### CTR-CP-005 — Lifecycle invariants（unchanged, verbatim）

- cancel 仅 active/non-terminal；already-cancelled → `already_cancelled`；
  archived → `instance_archived`。
- archive 仅 terminal/cancelled；coordinator 权限**不**绕过 lifecycle
  legality。
- transition/assignee 授权不变；coordinator 不是 implicit assignee。
- cleanup 语义（cancel→verify→archive；archived=no-op）是**下游编排
  纪律**，svc 不建 cleanup 状态机（directive non-goals）。

## 10. Acceptance

- ACC-CP-001 — coordinator holder 可：list/get domain、update displayName、
  get owner（含 read-after-write）、set owner（回归不破坏）、跨域
  cancel→read-back→archive→read-back；每步 server response 即验收依据。
- ACC-CP-002 — member 面：coordinator 跨域 add（幂等 replay 稳定）、
  list、remove；member 自读（`principals/me/domains`）出现/消失一致。
- ACC-CP-003 — reconcile：fixture 域上 plan（blockers 正确枚举）→ apply →
  旧 binding disabled + 新 binding enabled 原子生效 → replay 返回原
  outcome → 篡改 preimage 后 apply 409 `binding_conflict` 零 mutation。
- ACC-CP-004 — DOMAIN_MEMBER 对本 Spec 全部面 fail-closed（403）。
- ACC-CP-005 — DOMAIN_OWNER：own-domain member/cancel/archive 保持 PASS；
  他域全部 403；set owner / domain update / reconcile 403。
- ACC-CP-006 — GLOBAL_WORKFLOW_COORDINATOR 负向：无 SQL/admin fallback
  面（admin 端点仍 admin-gated 403）、不能 transition 非己 assignee 实例、
  不能 delete 任何东西、不能绕过 lifecycle（active→archive 仍 4xx）。
- ACC-CP-007 — 既有错误码字节不回归（DEC-CP-002）；repo 测试套件全绿。
- ACC-CP-008 — role grant 未随本 Spec 执行（frontmatter
  `role_applied_by_this_spec: false`）；grant 按 §11 单独授权。

## 11. Migration, compatibility, and rollback

- 部署顺序：本 Spec 实现部署 **先于** dsh broker 能力面部署（下游 Spec
  依赖本 wire contract）；role grant（coordinator → HR
  dc702687-6515-4a2a-91ae-e572a9bbd766，admin
  `PUT /internal/v1/admin/global-role-bindings/{principalId}` body
  `{roleKey:"GLOBAL_WORKFLOW_COORDINATOR", enabled:true}`，Idempotency-Key
  + pre/post enumeration per READER_V1 §6 checklist 形态）在其后、
  separately owner-authorized 执行。
- 数据库：**零 migration**（C 类端点全部用既有表/索引；OBS-CP-003/004、
  S10）。`database_migration_required: false`。
- 兼容：全部既有码/端点行为对 DOMAIN_OWNER、READER、admin、dispatch 面
  零变化（ACC-CP-007 兜底）。
- 回滚：代码 revert 即回滚（新端点消失、谓词收窄回 DOMAIN_OWNER-only）；
  receipt/audit 行是 append-only 历史，不需清理；role grant 回滚 =
  admin DELETE global-role-binding（既有幂等撤销路径）。

## 12. Alternatives and disposition

- 新增独立 `workflow_cancel` / `workflow_archive` 平行写工具 — **rejected**：
  违反 dsh 侧「唯一 Workflow 写工具」contract（dsh
  AGENT_CORE_WORKFLOW_ASSIGNEE_TRANSITION_CAPABILITY_V1 §21/§23）；svc 侧
  本就无需新端点。
- 复用 `/internal/v1/admin/*`（D 类）作为 coordinator 面 — **rejected**：
  admin allow-list 门禁不可下放；directive §15 明确 coordinator 不得有
  admin fallback。
- N3 纳入 `enabled` flip — **rejected（V1）**：disable 语义需要
  active-instance 安全 invariant 设计（不隐式 cancel 全域任务），收益低；
  留 §13。
- domain list 用 offset/page/total — **rejected**：违反 repo keyset-cursor
  惯例（keyset continuation Spec 同一纪律）。
- reconcile 用「先 remove 后 add」两步 — **rejected**：可产生半迁移
  （DEC-CP-006）。
- 给 coordinator 通用 role-binding writer — **rejected**：超出 directive §9
  「不要任意写 principals / role_bindings」边界；只做窄 reconcile。
- 服务端自建 name→UUID discovery — **rejected**：identity authority 在
  Auth/dsh exact resolver（§4）；服务端只收 exact UUID（DEC-CP-007）。

## 13. Open questions

- Domain `enabled` flip 的安全 invariant（active instances 处置）——V1.x
  候补，需独立小型 audit。
- 除 HR 外的 coordinator holder（dispatcher 等）——本 Spec 不冻结任何
  grantee；grant 一律 separately authorized per principal。
- reconcile 的跨域批量形态（bulk plan）——无真实需求前不做。

## 14. What this PR changes

```text
DOCS ONLY — adds exactly this file.
SVC_WORKFLOW_CODE_CHANGE (this PR) = NONE
ROLE_CHANGE = NONE (plan only, §11)
PRODUCTION_CHANGE = NONE
DATABASE_MIGRATION_REQUIRED = NO
READY_FOR_INDEPENDENT_REVIEW = YES
STATUS = proposed
```
