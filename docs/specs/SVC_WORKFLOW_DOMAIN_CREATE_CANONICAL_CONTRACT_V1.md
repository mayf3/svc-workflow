---
spec_id: SVC_WORKFLOW_DOMAIN_CREATE_CANONICAL_CONTRACT_V1
title: Domain Create Canonical Contract V1 (server-generated domainId, creator-becomes-owner)
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: none
production_apply_authority: none
scope:
  - mayf3/svc-workflow
  - POST /internal/v1/domains (agent-facing coordinator domain create) wire + application + persistence behavior
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V7
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_1
amends:
  - mayf3/svc-workflow SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1 §5 S2 (Class A pre-existing
    create capability: the census recorded it; it never froze the create wire contract. This
    Spec freezes the replacement wire contract and supersedes the coordinator-only create gate.)
companion_specs:
  - mayf3/dsh-agent-core AGENT_CORE_DOMAIN_CREATE_CANONICAL_CONTRACT_BROKER_V1 (proposed;
    companion broker delta amending the workflow_domain_admin create operation per the
    amendment path reserved by CTR-DCP-001 of AGENT_CORE_WORKFLOW_COORDINATOR_CONTROL_PLANE_BROKER_V1)
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
date: 2026-09-16
product_direction: DOMAIN_CREATE_CANONICAL_CONTRACT_ALIGNMENT_V1 (Owner goal directive, 2026-09-16)
database_migration_required: false
---

# SVC_WORKFLOW_DOMAIN_CREATE_CANONICAL_CONTRACT_V1

> **PROPOSED — INERT until independent review + Owner exact-head acceptance.**
> Source work (code + tests on this branch) is gated on this Spec being
> accepted; until then the branch carries no merge/production authority.

## 1. Problem（contract drift 定性）

Agent/Broker 面的 domain create 当前要求调用方提供 `domainId`（新资源的
UUID 主键）。链路 fresh read-back（2026-09-16，github/main 8c78c8e）：

```text
broker workflow_domain_admin.create  required: [domainId, domainKey, enabled]
  → POST /internal/v1/domains
    → coordinator_domains::create_domain
      → ProvisionDomainRequest { domain_id: Uuid (required), domain_key, display_name, enabled }   // deny_unknown_fields
        → application::provision_domain  (caller-supplied id upsert)
          → provisioning_repository::upsert_domain (domain_id = caller UUID as PK)
```

根因：agent-facing coordinator create（LANE_I PR #42 落地）复用了
`docs/contracts/IDENTITY_PROVISIONING_API_V0.md`（FROZEN_FOR_PROVISIONING_READY，
治理 `workflow.admin` + allow-list 管理面）的 `ProvisionDomainRequest`
DTO 与 upsert 语义。caller-supplied 资源主键是 provisioning-系统契约
的形状，不是 agent 调用面的形状：普通 Broker 调用
`create(domainKey, displayName, enabled)` 因此必然失败。这不是
generated-client drift，也不是 backend bug，而是两个授权体制不同的
面共享了同一个 admin provisioning DTO。

同时 create 不产生任何 owner binding——owner 只能由 GWC 事后 `set_owner`
指派。canonical owner assignment 缺位于 create 事务内。

## 2. Canonical contract（本 Spec 冻结）

North Star（Owner directive）：

```text
NORMAL_DOMAIN_CREATE_REQUIRES_ONLY_BUSINESS_INPUTS
SERVER_OWNS_DOMAIN_ID_GENERATION
CANONICAL_OWNER_ASSIGNMENT
```

### CTR-DCC-001 — wire contract（取代旧行为，无双轨）

```text
POST /internal/v1/domains
Authorization: workflow.execute scope + direct token（OBO 拒绝，与现况一致）
               + caller 是 principals 表中 enabled 且 type=AGENT 的 principal
               （validate_provisioning_actor，与全部 provisioning 写面一致）
               GLOBAL_WORKFLOW_COORDINATOR 不再是 create 前置（见 CTR-DCC-004）

Request body（serde camelCase + deny_unknown_fields）：
  domainKey    string   required（1-128，无空白/控制字符；unique）
  displayName  string   optional（1-256，trim，无控制字符；缺省=domainKey）
  enabled      boolean  required
  （domainId 不是契约字段；携带即 400 unknown_field——旧路径退出，
    无兼容双轨）

Response 200：
  { domainId, domainKey, displayName, enabled, ownerPrincipalId }
  domainId = 服务端生成的资源主键
  ownerPrincipalId = authenticated caller（creator-becomes-owner）

Idempotency-Key：required（trusted seam，与现况一致），receipt 机制不变。
```

### CTR-DCC-002 — server-generated domainId

- 生成位置：application 层（receipt-owned 分支内），`DomainId::new()`
  （`src/domain/ids.rs` `make_id!` → `Uuid::new_v4`）——项目 canonical
  ID 生成机制，与 definition service 同源；不新造第二套 UUID 机制。
- 请求 hash 只含业务入参 `{commandType, domainKey, displayName, enabled}`，
  不含 domainId。同 key replay 返回原 receipt response（原 domainId 稳定
  不变）；同 key 不同入参 → `idempotency_conflict`；新 key + 已占用
  domainKey → `409 domain_identity_conflict`（零 mutation）。

### CTR-DCC-003 — canonical owner assignment（creator-becomes-owner）

- Owner canonical 来源 = **authenticated principal**（Option A；
  Owner directive §二 的 canonical 设计判定）。ownerPrincipalId 永远
  不是请求字段——任何 caller（含 GWC）都无法借 create 指定他人为 owner
  （结构性满足 directive 测试 #7）。
- 建立时机：与 domain INSERT 同事务；复用
  `idx_drb_single_owner`（单 enabled DOMAIN_OWNER）既有不变量。
- `replace_owner`/`set_owner`（GWC-only）语义不变——owner 重指派权
  不扩散；GWC create 出的 domain owner=GWC 自身，可经 set_owner 移交。

### CTR-DCC-004 — authorization delta（唯一放宽点，显式冻结）

- create 门从 `GLOBAL_WORKFLOW_COORDINATOR` 改为
  `direct token + workflow.execute + enabled AGENT actor`。
- 放宽的 blast radius：caller 仅获得**它新建 domain** 的 DOMAIN_OWNER；
  对既有 domain、他域、set_owner/list/get/update/reconcile 面零增益。
- 受信 fleet 前提下接受「任意 enabled agent 可创建 domain」为产品方向
  （Owner directive §二/§三：家庭管家 canonical Principal 须能经正常
  Broker create 成为自己的 Domain owner——其不应也不需要持有 GWC）。
- 风险对冲（既有机制，无新增）：`workflow_command_receipts` 全量记账
  （actor、idempotency key、command type、request hash、response——
  canonical domain create 的唯一持久记账载体）+ tracing 记账
  （`log_provisioning`）；canonical domain create **不写**
  `workflow_command_attempt_audits`（该表属 coordinator control-plane/
  member/wake 面）。另有 domainKey unique、direct-token 强制、
  AGENT-only actor 校验（HUMAN principal 拒绝，`principal_type_not_allowed`）。

### CTR-DCC-005 — receipt / audit

- 新 command_type：`domain.create`（`workflow_command_receipts.command_type`
  为自由 TEXT，零 migration——S10 既有事实）。admin 面继续用
  `PROVISION_DOMAIN`，两命令类型在 receipts（`command_type` 列）中可区分。
- application 层新增 `create_domain`（不改动 `provision_domain`——admin
  面 upsert 语义原样保留）。

### CTR-DCC-006 — 面的边界（不减法对象显式豁免）

- agent/broker 公共 create 契约：caller-supplied domainId 路径**整体移除**
  （新 DTO，无兼容层，无 mapping shim）。
- `POST /internal/v1/admin/domains`（IDENTITY_PROVISIONING_API_V0，
  workflow.admin + allow-list）：**不动**。它是另一冻结契约、零 broker
  可达性、仓内零生产 caller（fresh census：仅 svc 自身契约测试引用）。
  其 caller-supplied upsert 语义属 provisioning 系统形状，不构成本
  Spec 的「public create contract」。未来是否退役为独立 product 决策。

## 3. 既有 artifact 处置

- SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1 §5 S2：Class A「已存在」
  census 行被本 Spec 的 CTR-DCC-001/004 取代（该 Spec 从未冻结 create
  wire contract，无条款冲突；方向一致，无需 SUPERSEDE 整个 Spec）。
- CTR-DCP-001 冻结的 negative 测试
  `non_coordinator_agent_denied_for_create_domain` 被本 Spec 显式取代：
  非 coordinator 的 enabled agent create 成功且成为 owner（ACC-DCC-002）。
  `read_scope_denied` / `obo_token_denied` / admin 面全部既有 negative
  保持字节级不变。
- IDENTITY_PROVISIONING_API_V0：零改动、零冲突（§2 CTR-DCC-006）。
- dsh CTR-DCP-001 create body freeze：经其自身预留的 amendment 路径由
  dsh companion spec 处置（本仓不做 dsh 的 spec 修改）。

## 4. Acceptance

| ID | 证明 | 对应 Owner directive 测试 |
|---|---|---|
| ACC-DCC-001 | create 仅带业务入参成功；domainId 非空且服务端生成（请求体无 domainId 可传） | #1 #2 #3 |
| ACC-DCC-002 | 非 coordinator enabled agent create → 200，ownerPrincipalId=caller，DOMAIN_OWNER binding enabled | #4 #5（svc 侧） #7 |
| ACC-DCC-003 | 同 key replay → 同 domainId、零第二 domain；新 key 重复 domainKey → 409 domain_identity_conflict；同 key 改入参 → idempotency_conflict | #6 |
| ACC-DCC-004 | 请求携带 domainId → 400 unknown_field（旧 caller-supplied 路径退出） | #9 |
| ACC-DCC-005 | OBO 拒绝；workflow.read-only token 拒绝；disabled/HUMAN actor 拒绝 | #7 负面 |
| ACC-DCC-006 | read/list/get（既有域读面）与 admin provisioning 面测试零回归 | #8 |
| ACC-DCC-007 | 家庭管家 canonical principal（fresh evidence 完整 UUID，roster 本地件）作为 JWT subject 走真实 svc HTTP 链 create 成功且 owner=该 principal | #5 |

## 5. What this PR changes

```text
src/http/dto.rs                                  + CreateDomainRequest（deny_unknown_fields）
src/domain/provisioning/mod.rs                   + COMMAND_TYPE_CREATE_DOMAIN、CreateDomainCommand
src/application/provisioning/mod.rs              + create_domain（server-generated id + in-tx owner）
src/store/postgres/provisioning_repository/      + establish_domain_owner（单 tx INSERT..ON CONFLICT enable）
src/http/handlers/coordinator_domains.rs         create_domain 换新 DTO/新 app fn（CTR-DCC-001/002）；门替换（CTR-DCC-004）
tests/24_coordinator_domain_management.rs        create 测试族重写 + 新 negative/正面（ACC-DCC-001..006）
                                                 + env-gated canonical_principal_create_chain（ACC-DCC-007，
                                                 经 CANONICAL_CREATE_TEST_PRINCIPAL_ID 注入，无独立新文件）
docs/specs/SVC_WORKFLOW_DOMAIN_CREATE_CANONICAL_CONTRACT_V1.md（本文件）

IDENTITY_PROVISIONING_API_V0 admin 面：零改动。
set_owner/list/get/update/reconcile/members/cancel/archive：零改动。
DB schema：零 migration。
```

PRODUCTION_MUTATION_BY_THIS_BRANCH = NO（部署与角色/客户端 scope 变更
均为 Owner-gated 生产事务）。
