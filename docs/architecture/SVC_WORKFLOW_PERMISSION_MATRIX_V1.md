---
doc_id: SVC_WORKFLOW_PERMISSION_MATRIX_V1
doc_type: reference-matrix (非规范；non-normative)
date: 2026-10-08
base_rev: 7c533cfc2676ea970d502969662dff3bbdd3e66c (github/main)
status: reference
authorities:
  - 本仓库 `src/` 实际实现（文件:行号以 base_rev 为准，行号随演进漂移，谓词函数名为准）
  - docs/specs/ 下各 accepted Spec（WORK_ELIGIBILITY_PROJECTION_V1、GLOBAL_WORKFLOW_READER_V1、VISIT_ACTIVATION_IMPL_V1 等）
  - docs/architecture/AUTH_PRINCIPAL_SELF_PROJECTION_AND_DOMAIN_MEMBERSHIP_V1.md
  - 跨仓（仅引用）：mayf3/dsh-agent-core docs/specs/AGENT_CORE_AGENT_CREDENTIAL_PROVISIONING_V1.md（accepted）、
    AGENT_CORE_WORKFLOW_ASSIGNEE_TRANSITION_CAPABILITY_V1.md（accepted）、AGENT_CORE_WORKFLOW_DOMAIN_INSTANCES_BROKER_V1.md（proposed）
---

# SVC_WORKFLOW_PERMISSION_MATRIX_V1 — Workflow 权限矩阵（当前实现 vs 已批准/建议待实现）

> **本文不是规则源。** 它把「当前代码实际强制」的权限行为归纳成一张可读表，并把
> 「已批准但未部署」与「讨论中未批准」的变更**分开列出**。规则本身的权威仍是
> 源码与 accepted Spec；本文与其冲突时以源码/Spec 为准。
>
> 本文为公开文档：不含真实 principal/群组标识、业务数据、主机路径或 secret。

## 0. 身份与角色模型（先读）

三层控制，全部在服务端强制（`src/http/handlers/mod.rs:25-34` `require_scope`；
缺失 scope → 403 `forbidden`）：

| 层 | 载体 | 说明 |
|---|---|---|
| OAuth scope | auth-service 签发的 JWT：`workflow.read` / `workflow.execute` / `workflow.admin` | 粗粒度；`workflow.admin` 仅守 `/internal/v1/admin/*` 供给面 |
| 服务端业务角色 | 本仓库 Postgres 表 `domain_role_bindings`（`DOMAIN_OWNER` / `DOMAIN_MEMBER`，含 enabled 位）与 `global_role_bindings`（`GLOBAL_WORKFLOW_COORDINATOR` / `GLOBAL_WORKFLOW_READER` / `GLOBAL_SCHEDULER_READ`） | 角色绑定由本仓库自身管理/写入，**不放在 JWT 里**（`src/http/handlers/coordinator_domains.rs:12-14`；`src/store/postgres/domain_role_repository.rs`） |
| 供给 allow-list | `/internal/v1/admin/*` 额外要求：principal_type=agent + 直连 token + 配置化 principal allow-list | `src/http/handlers/provisioning/mod.rs:26-117`（缺 scope→403 `insufficient_scope`；不在名单→403 `provisioning_not_allowed`） |

要点：

- **没有** platform_admin / superuser 角色（全仓无此概念）。所谓"平台管理能力"
  = `workflow.admin` scope + allow-list 的供给 API 面，不等于任何业务域的 Owner。
- 写路径普遍另要求**直连 access token**（禁止 OBO/委托主体）：定义写
  （`src/http/handlers/definitions.rs:88-99`）、成员写
  （`src/http/handlers/domain_members.rs:39-50`）→ 违反 403 `direct_token_required`。
- Broker/Gateway（跨仓 dsh-agent-core）**不做角色判断**：按实际运行进程的 agentId
  取凭据、按 manifest 声明 `requiredScopes` 铸 token，scope+角色一律由下游（本服务）
  裁决；凭据缺失 fail-closed（dsh-agent-core `packages/broker/src/credential.js`、
  `src/transport.js`）。

记号：✅ 允许；❌ 拒绝；⚠️ 仅限/条件；「不透明 404」= 为防存在性泄露统一返回
`definition_not_found` 或 `workflow_instance_not_found_or_not_visible`。

## 1. 定义（Definition）面

| # | 操作 | DomainOwner | DomainMember | 域外 Agent | 全局角色 | 供给 admin 面 | 额外条件 | 依据（@base_rev） |
|---|---|---|---|---|---|---|---|---|
| A1 | 列出域内 Definitions | ✅ | ❌（不透明 404） | ❌（不透明 404） | ❌（本面无全局旁路） | ❌（另有只读摘要面，见 A6） | scope `workflow.read`；principal enabled | `handlers/definitions.rs:111`；`application/definition/lifecycle/reads.rs:156-162`；`handlers/definitions.rs:437-446` |
| A2 | 读 Definition 详情 / 版本列表 / **版本输入契约（context schema，含 PUBLISHED 精确版本）** | ✅（含 DRAFT/已归档，无状态过滤） | ❌（不透明 404） | ❌（不透明 404） | ❌ | ⚠️ 仅摘要+`canCreateInstances`，**不含 schema** | scope `workflow.read` | `reads.rs:33,61,98,134`（`ensure_domain_owner`）；`handlers/definitions.rs:171-203,433-446`；`handlers/provisioning/definitions.rs:11-36` |
| A3 | 创建 Definition / 新建草稿版本 / 替换草稿图 | ✅ | ❌ | ❌ | ❌ | ❌ | `workflow.execute` + 直连 token + Idempotency-Key；已归档→409 `definition_not_editable`；非 DRAFT 替换→409 `definition_version_immutable` | `definitions.rs:209-246,252-303,309-339`；`application/definition/draft_graph.rs:30-32`；`service.rs:49,108` |
| A4 | 发布版本（DRAFT→PUBLISHED） | ✅（事务内校验 enabled 绑定） | ❌ | ❌ | ❌ | ❌ | `workflow.execute` + 直连 token；发布消费 DRAFT 状态 | `handlers/definitions.rs:345-388`；`store/postgres/definition_repository/lifecycle_transactions.rs:217-231` |
| A5 | 修改已发布版本图/schema | **不存在**（不可变） | 同左 | 同左 | 同左 | 同左 | 发布后只能：新建草稿版本 / 归档；deprecate/revoke 仅有库内命令无 HTTP 路由 | `draft_graph.rs:30-32`；`lifecycle/status_changes.rs`（无路由） |
| A6 | 归档 Definition | ✅ | ❌ | ❌ | ❌ | ❌ | `workflow.execute` + 直连 token；幂等 | `application/definition/service.rs:195-213`（owner check :207） |

**读写不一致（本轮已批准窄修复的目标，见 §4-P1）**：满足 A7 创建准入的同域成员
可以 `create_instance`，但读不回该 PUBLISHED 版本的输入契约（A2 被 `ensure_domain_owner`
拦截 → 404），只能靠 Owner 人工转交 schema。

## 2. 实例（Instance）面

| # | 操作 | DomainOwner | DomainMember | 当前 assignee | 创建者 | 域外 Agent | 全局角色 | 额外条件 | 依据（@base_rev） |
|---|---|---|---|---|---|---|---|---|---|
| B1 | 创建实例 `POST /internal/v1/workflow-instances` | ✅ | ✅（域内任一 enabled 绑定即可，无 role_key 过滤） | 同 Member | 同 Member | ❌ 403 `cross_domain_violation`（域不符）/ 403 `domain_membership_required`（无绑定） | ❌ | `workflow.execute`；版本必须 `PUBLISHED`（409 `version_not_published`）；`executionClass=NON_BUSINESS_TEST` 额外要求 Owner（403 `not_domain_owner`）；**创建者不获得持久角色**，仅记 `created_by_principal_id` | `handlers/instances.rs:29-35`；`store/postgres/workflow_instance_repository/create_transaction.rs:214-274,259-274`；`validation_helpers.rs:76-97` |
| B2 | 我的任务 `GET /internal/v1/worklists/assigned-to-me` | ⚠️ 仅自己被派/继承的 | 同左 | ✅（本表主体） | ⚠️ 当前节点=DRAFT 时另有创建者草稿清单 | ❌ | ❌ | `workflow.read`；谓词=当前 visit assignee==actor（或 successor line）、node≠TERMINAL、实例未取消、actor 在该域有 enabled 绑定或属派系链 | `handlers/worklists.rs:31-36`；`query_worklists.rs:45-96,112-132` |
| B3 | 域内实例枚举 `GET /internal/v1/workflow-instances/domain` | ✅ | ❌（不透明 404，**非 403**） | ❌ | ❌ | ❌ | ❌ | `workflow.read`；`check_domain_owner` 服务端强制 | `handlers/instances.rs:122-134`；`application/workflow_instance/query_service.rs:80-92`；`query_visibility.rs:34-50`；`http/error.rs:586-589` |
| B4 | 全局实例枚举 `GET /internal/v1/workflow-instances/global` | ❌（除非另有全局角色） | ❌ | ❌ | ❌ | ❌ | ✅ `GLOBAL_WORKFLOW_READER` 或 `GLOBAL_WORKFLOW_COORDINATOR`（403 `global_read_role_required`） | `workflow.read`；**仅摘要投影**（不含 context/提交 payload） | `handlers/instances.rs:179-198`；`query_visibility.rs:60-75`；`query_service.rs:142-154`；`http/error.rs:594-597` |
| B5 | 实例详情（含业务内容 context） | ✅ Full | ❌（除非历史参与者，见左） | ✅ Full（当前节点≠TERMINAL） | ⚠️ 仅当前节点=DRAFT 时 Full；其后降为 Restricted | ❌（不透明 404） | ❌（全局角色**不授予**详情） | `workflow.read`；四级可见性 `DomainOwnerFull`→`CurrentAssigneeFull`→`CreatorDraftFull`→`HistoricalParticipantRestricted`；Restricted 只得摘要（无 context）；无任何匹配→不透明 404 | `query_visibility.rs:271-318`（`classify_visibility`）；`handlers/instances.rs:99-104` |
| B6 | 提交历史 `GET …/submissions` / 时间线 `GET …/timeline` | ✅ 全量 | ⚠️ Restricted=仅自己 authored 的提交 + RETURN 引用自己提交的条目；时间线另含 TERMINATE/终态 visit 事件 | 同 Member（自己相关） | 同 Member | ❌ | ❌ | `workflow.read`；同一四级门；Restricted 查 context 修订→403 `restricted_history_not_visible`；`ADMIN_EMERGENCY_OVERRIDE_*` 事件对非 Full 级剥离 | `handlers/submissions.rs:34-40`；`handlers/timeline.rs:15-21`；`application/workflow_instance/query_detail.rs:233-274,303-310,404-451,280-283` |
| B7 | 提交 transition `POST …/transitions` | ❌ **不能代 assignee 提交** | ❌ | ✅ **仅当前 visit assignee**（403 `principal_not_assignee`） | ❌（不能以创建者身份提交） | ❌ | ❌ | `workflow.execute`；已取消/TERMINAL→409 `source_node_terminal`；assist open→409 `assistance_open`；状态版本冲突→409 `workflow_state_version_conflict` | `handlers/transitions.rs:19-26`；`store/postgres/workflow_instance_repository/transition_transaction.rs:210-215`；`http/error.rs:190-193` |
| B8 | revise-and-transition | ❌ | ❌ | ⚠️（且须同时是创建者、当前节点 DRAFT）——**HEAD 无 HTTP 路由**（仅库/测试面） | 同左 | ❌ | ❌ | 非 HTTP | `application/workflow_instance/revise_and_transition.rs:42`；`combined_transaction.rs:205-224` |
| B9 | 取消 `POST …/cancel` | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ `GLOBAL_WORKFLOW_COORDINATOR` | `workflow.execute`；403 `not_domain_owner`；已取消→409 `already_cancelled`；TERMINAL→409；已归档→409 `instance_archived` | `handlers/cancel.rs:36-43`；`store/postgres/workflow_instance_repository/cancel_transaction.rs:280-316`；`http/error.rs:505` |
| B10 | 归档 `POST …/archive` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | 仅终态实例（非终态 403/409 映射 `not_domain_owner` 族） | `http/error.rs:543-547` |
| B11 | 恢复（取消/归档后） / 重派 assignee | **不存在**（无 HTTP 端点；assignee 仅在 visit 创建时落定） | 同左 | 同左 | 同左 | 同左 | 同左 | 紧急恢复 `admin_recovery`/`admin_repair` 仅为 binary/库面（非 HTTP），要求 DOMAIN_OWNER 或 `WORKFLOW_ADMIN` enabled 绑定 | `src/main.rs:5`；`store/postgres/admin_recovery_repository/authorization.rs:57`；`application/workflow_instance/admin_repair.rs:5-11` |
| B12 | 调度只读面（wake / escalations / dispatch-intents） | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ `GLOBAL_SCHEDULER_READ`（fail-closed） | `workflow.read` | `handlers/wake.rs:23-54`；`handlers/escalations.rs:8`；`http/error.rs:473,600` |

## 3. 成员、域与管理面

| # | 操作 | 谁可做 | 额外条件 | 依据（@base_rev） |
|---|---|---|---|---|
| C1 | 列域成员 `GET /internal/v1/domains/{id}/members` | DOMAIN_OWNER **或** GLOBAL_WORKFLOW_COORDINATOR | `workflow.read` + 直连 token | `handlers/domain_members.rs:39-66`；`application/domain_membership/mod.rs:246-250` |
| C2 | 加成员 `PUT …/members/{principalId}` | DOMAIN_OWNER | `workflow.execute` + 直连 token；目标不得已是 DOMAIN_OWNER（409 `principal_is_owner`）；重复添加=幂等 `already_member` | `domain_members.rs:131`；`application/domain_membership/mod.rs:306-408` |
| C3 | 删成员 `DELETE …/members/{principalId}` | DOMAIN_OWNER | 只移除 `DOMAIN_MEMBER`，永不移除 Owner | `domain_members.rs:167`；`application/domain_membership/mod.rs:474-510` |
| C4 | 创建域 `POST /internal/v1/domains` | 任一 enabled Agent 调用者（事务内"创建者即 Owner"） | `workflow.execute` + 直连 token | `handlers/coordinator_domains.rs:60-121` |
| C5 | 改换域 Owner `PUT …/owner` | GLOBAL_WORKFLOW_COORDINATOR（403 `global_coordinator_required`） | `workflow.execute` + 直连 token | `coordinator_domains.rs:41-58,135-137` |
| C6 | 供给/管理 API `/internal/v1/admin/*`（principals、domains、域角色绑定 `DOMAIN_OWNER\|WORKFLOW_ADMIN`、全局角色绑定、定义版本摘要） | allow-list 内的 agent principal | `workflow.admin` scope + 直连 token + principal_type=agent；未列名单→403 `provisioning_not_allowed` | `handlers/provisioning/mod.rs:26-117`；`role_bindings.rs:37-83`；`global_role_bindings.rs:34-41` |
| C7 | `WORKFLOW_ADMIN`（域级）角色的实际强制面 | — | 目前**仅** admin_recovery / admin_repair（binary 紧急恢复路径）；HTTP 业务面无此角色判定 | `admin_recovery_repository/authorization.rs:57`；`admin_repair.rs:5-11` |

## 4. 会话 / 附件 / 凭据（明确不并入任务可见性）

- **会话与附件不是 svc-workflow 对象**：本服务无 session/attachment/chat 端点
  （全路由表见 `src/http/mod.rs:32-301`）。Agent 会话内容的可见性由运行时层
  （dsh-agent-core）承载；其 broker 能力面不含会话/附件读取能力（forum 线程
  transcript 属 Forum 域，另计）。
- **凭据不可回读**：agent 机器凭据由部署/控制面一次性写入受控凭据文件（0600、
  专属 uid），唯一读者是 broker 网关（按次重读、按实际 agentId 取用、fail-closed）；
  子进程与模型永不持有原始 secret。规则源：dsh-agent-core
  `docs/specs/AGENT_CORE_AGENT_CREDENTIAL_PROVISIONING_V1.md`（accepted，§authority model）。

## 5. 已批准待实现 / 讨论中（与上表"当前实现"严格分开）

| # | 变更 | 状态 | 落地后矩阵差异 |
|---|---|---|---|
| P1 | **已批准窄修复（进行中，未部署）**：已具备同域创建资格者（满足 B1 准入=域内任一 enabled 绑定）可读取目标域 **PUBLISHED 精确版本**的**输入契约**（context schema 的必要内容） | 已批准、源码修复进行中；**不代表已部署** | A2 的 DomainMember 列将由「❌」变为「⚠️ 仅 PUBLISHED 精确版本输入契约」；**不**扩及：草稿、跨域、管理数据、assignee 提交权、全局读；**不**移除 Owner/Member 概念 |
| P2 | 成员可见的「域内任务概况」（摘要级） | **讨论中，无批准记录，未实现** | 若未来批准，B3 才可能出现 Member 摘要只读列；当前矩阵维持 Owner-only |

## 6. 职责必要性与最小调整建议（供后续讨论，非决定）

- **Owner/Member 二元继续保留**：Owner=定义治理（A1-A6）+ 域管理（C1-C3）+ 高危生命周期
  （B9-B10）；Member=在域内**发起/承接工作**（B1）并按四级可见性参与（B5-B6）。二者职责
  边界目前与实现一致，未见需要新增第三业务角色。
- **最小修复优先复用既有准入谓词**（P1 即按此原则），避免第二套"可创建"定义漂移。
- 若 P2 未来要推进：建议走**摘要级只读投影**（复用 B5 四级哲学的第五档"域成员摘要"），
  而非放开 B3 的 Owner-only 枚举；决策归属后续 Spec 流程。
- 不建议：新增权限平台/角色体系重构、把全局读角色当详情读、让 Owner 默认代 assignee 提交、
  在代码或文档硬编码具体 Agent 名或事件编号。

## 7. 已知边界与 UNKNOWN

- 行号以 `base_rev` 为准，会随后续合并漂移；以谓词函数名
  （`ensure_domain_owner` / `validate_domain_membership` / `classify_visibility` /
  `check_domain_owner` / `check_global_workflow_read_role` / `check_domain_write_role`）
  为稳定锚点。
- Broker 能力清单跨仓迭代中（`workflow_domain_instances` 为 proposed；`workflow_transition` /
  `workflow_global_instances` 已按各仓 Spec/reports 合并到 dsh-agent-core github main），
  以 dsh-agent-core 仓内 Spec 状态为准，本文不重复维护。
- P1 的最终返回字段集与 wire 语义以其评审通过的 Spec/PR 为准，本文不预写。
