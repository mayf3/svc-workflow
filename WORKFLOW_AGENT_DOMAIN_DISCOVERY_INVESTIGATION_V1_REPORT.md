# WORKFLOW_AGENT_DOMAIN_DISCOVERY_INVESTIGATION_V1_REPORT

> 日期：2026-08-10 | 状态：仅调查，未改生产代码
> 范围：svc-workflow（后端）+ openclaw-adc-canary-extension broker（Agent-facing grouped tool）

## 背景与目标

DOMAIN_OWNER 需要调用 `workflow_read(action=domain_instances, domainId=...)`，但 Agent 没有可靠方式自主发现自己的 `domainId` / 所属 Domain / 自身 role。本报告确认：

1. svc-workflow 是否已有 caller-scoped 的 my-domains 类正式 API；
2. 若无，记录最小后端缺口与推荐语义；
3. 判断 `domain_instances` 是否应支持隐式 domainId 推导。

约束（已遵守）：不人工写 UUID 进 AGENTS.md/Skill、不硬编码 Knowledge Domain、不让 Agent 读 domains.yaml/DB/exec、不做 HR/知识管家特判、Broker 不查 role、Broker 不维护 membership。

## 一、现有正式 API 调查结果

### 已存在的相关端点（全部 domain-scoped 或 admin-scoped，无 caller-scoped）

| 端点 | scope | 语义 | 是否满足需求 |
|------|-------|------|-------------|
| `GET /internal/v1/workflow-instances/domain`（domain_instances） | `workflow.read` | 按 `domainId` 列实例；服务端校验调用者是否为该域 `DOMAIN_OWNER`（`query_visibility::check_domain_owner`，`src/store/postgres/workflow_instance_repository/query_visibility.rs:34`） | 需要先知道 domainId，不能自举 |
| `GET /internal/v1/domains/{domainId}/members` | `workflow.read` + direct-token | 列某域的 DOMAIN_MEMBER；仍需 domainId 且调用者须为 OWNER | 不满足（参数化方向反了） |
| `GET/PUT/DELETE /internal/v1/domains/{domainId}/members/{principalId}` | `workflow.execute` | 成员管理（写） | 不满足 |
| `PUT /internal/v1/principals/me` | — | 自我投影（AGENT 类型 upsert） | 与发现无关 |
| `GET/PUT/DELETE /internal/v1/admin/domains/*`、`role-bindings/*` | admin | 平台供给；`GET /internal/v1/admin/domains/{domainId}` 是唯一返回 `domain_key/display_name` 的端点，但需 admin 权限且按 domainId 查询 | 不满足（admin-scoped，Agent 不可用） |

正式契约：`contracts/workflow-http/v1/openapi.yaml` 中无任何 caller-scoped "我的域/我的绑定"路径。

### 数据层核查

- `domains`：`domain_id / domain_key / display_name / enabled / metadata`（`migrations/0001_identity_domain.sql:65`）。
- `domain_role_bindings`：`binding_id / domain_id / principal_id / role_key / enabled / created_at / disabled_at`（同文件）。
- 所有 `domain_role_bindings` 查询点（共 20+ 处）均为：给定 domainId 查 caller（`check_domain_owner` / `check_has_role`）、admin 写、或实例级可见性分类。
- 所有 `domains` 表读取均按 `domain_id` / `domain_key` 定位；**没有任何按 `principal_id` 反向列出所属域的查询**（`domain_role_repository.rs` 只有 `list_member_bindings`：按域列成员，方向相反）。

### 身份可靠性

- caller 主体验证：Auth V1（RS256/JWKS，auth-service 签发）；`AuthenticatedPrincipal.principal_id` 来自已验证 token 的 `sub`（OBO 为 `act.sub`），`src/auth/claims.rs`、`src/auth/principal.rs`。是可信 caller，可直接作为 caller-scoped 查询键。

## 二、结论

```
EXISTING_MY_DOMAINS_API = 不存在（No）
WORKFLOW_READ_MY_DOMAINS_NEEDED = true（后端缺口，Broker 侧 action 也需新增）
```

### 最小 svc-workflow 缺口（只做推荐，不实现）

推荐语义：**返回当前 caller 自己的 `domain_role_bindings` + 对应 `domains` 基本信息**。

- 推荐端点：`GET /internal/v1/principals/me/domains`（与既有 `PUT /internal/v1/principals/me` 自我投影路径风格一致；caller-scoped，无 domainId 参数）
- 授权：`require_scope(&principal, "workflow.read")`（与其它只读端点一致）
- 建议响应字段（每项）：
  - `domainId`
  - `domainKey`
  - `displayName`
  - `callerRole`（`DOMAIN_OWNER` / `DOMAIN_MEMBER`；role_key 原样）
  - `bindingCreatedAt`
  - `domainEnabled` / `bindingEnabled`
- 过滤：仅 `enabled = TRUE` 的绑定与域（与 `check_domain_owner` 语义一致：`role_key='DOMAIN_OWNER' AND enabled=TRUE`）
- 不做：HR 特判、知识管家特判、返回所有 Domain、Broker 查 role、Broker 维护 membership

### Broker 侧落点（后端落地后）

`workflow_read`（`openclaw-adc-canary-extension/broker/src/adapters/workflow-read.ts`）是现有 grouped tool，已有 4 个 action（my_tasks / submission_history / domain_instances / instance_detail），每个 action 映射一个 capability → 一个 svc-workflow 端点，身份唯一来源 `ctx.agentId`，授权下沉 svc-workflow。`my_domains` 应作为新 action 加入同一 tool：

- 新 capability `workflow_my_domains`（config + `WORKFLOW_READ_CAPABILITIES` 列表 + `index.ts` 工厂注册 + 按 agent 授权 mapping）
- 新变体 `MyDomainsVariant`（`{ action: 'my_domains' }`，无参数）
- 薄封装：`authorizedFetch → GET /internal/v1/principals/me/domains`，原样直出，不做投影
- **不新增顶层 tool**，不新增 Broker 角色逻辑

## 三、UX 判断：`domain_instances` 是否隐式推导 domainId

**判断：不要隐式推导。** 维持 `domainId` 必填，显式链：

```
workflow_read(action=my_domains)
→ workflow_read(action=domain_instances, domainId=<来自 my_domains>)
```

理由：

1. **将来一个 Agent 可 OWNER 多个 Domain**：隐式推导规则（"恰好一个 OWNER 域时自动填充"）在绑定集合变化时静默失效，Agent 可能拿到错误域的实例而不知情。
2. **语义歧义**：0 个域 / 多个域 / 仅 DOMAIN_MEMBER 的 caller 行为需要额外错误分支，每个分支都是新契约面。
3. **可解释性**：显式 `domainId` 使每一次 domain_instances 调用自证作用域；隐式推导让调用不可复现、日志难审计。
4. `my_domains` 本身是廉价且独立有用的动作（同时解决 membership + role 发现），显式链只多一次调用，成本可忽略。

## 推荐调用链

```
workflow_read(action=my_domains)
→ 找到 callerRole=DOMAIN_OWNER 的 Domain（取 domainId/domainKey/displayName）
→ workflow_read(action=domain_instances, domainId=..., [limit/cursor/lifecycle/status/...])
```

## 结论标志

```
EXISTING_MY_DOMAINS_API=no
WORKFLOW_READ_MY_DOMAINS_NEEDED=true
DOMAIN_DISCOVERY_OWNED_BY=svc-workflow
BROKER_ROLE_LOGIC_ADDED=false
HARDCODED_DOMAIN_IDS=false
EXEC_REQUIRED=false
RECOMMENDED_CALL_FLOW=workflow_read(my_domains) -> workflow_read(domain_instances, domainId=<owner domain>)
SAFE_TO_IMPLEMENT=true
REPORT_PATH=WORKFLOW_AGENT_DOMAIN_DISCOVERY_INVESTIGATION_V1_REPORT.md
COMMIT=none（调查完成，未修改生产代码）
```

## 实施记录（2026-08-10，已批准后实施）

### svc-workflow（后端，已完成）

- `GET /internal/v1/principals/me/domains` 新端点（`src/http/mod.rs` 路由 + `src/http/handlers/self_projection.rs` handler）
- `domain_membership::list_my_domains` 应用服务（`src/application/domain_membership/mod.rs`）
- `domain_role_repository::list_my_domains` 查询：`domain_role_bindings ⋈ domains`，`principal_id = $1 AND b.enabled AND d.enabled`，按 `domain_key, role_key` 排序
- 授权：`workflow.read` scope；direct/OBO 均接受（只读、自作用域）
- 响应：`{"items": [{domain_id, domain_key, display_name, caller_role, binding_created_at}]}`（snake_case，与既有 wire 格式一致）
- 契约：`contracts/workflow-http/v1/openapi.yaml` 新增 path + `MyDomainsList`/`MyDomainItem` schema；changelog 新增条目
- 测试：`tests/18_self_projection_and_domain_members.rs` 新增 8 用例（owner/member 多角色、caller-scoped、空列表、disabled binding/domain 排除、缺 scope 403、OBO 可用）
- 验证：`cargo check` ✅、`cargo fmt`（仅本变更文件）✅、`cargo clippy --lib`（无本变更告警）✅、`cargo test --lib` 140 ✅
- 集成测试**已运行**：见下「测试运行记录」。初始受阻原因为本地 5432 被 Homebrew postgres 占用且无超级用户凭据；已通过「docker 测试库 + 环境变量覆盖」解决（详见测试基建改动）

### Broker（openclaw-adc-canary-extension，已完成）

- `workflow_read` 新增 `action=my_domains`（`broker/src/adapters/workflow-read.ts`）：无参数变体、`workflow_my_domains` capability 原子、薄封装原样直出
- 未新增顶层 tool（遵守调研结论）；capability 原子由 grouped tool 实现
- 测试：`tests/unit/grouped-tools.test.ts` 新增 3 用例；`npm test` 192 全过 ✅、`tsc` build ✅
- 部署时控制面需：`capabilities += workflow_my_domains`（GET `/internal/v1/principals/me/domains`，scope `workflow.read`，无 allowedAgentIds）+ 目标 agent `alsoAllow += workflow_my_domains`（与 workflow_domain_instances 上线方式一致）

### 测试运行记录（2026-08-10，docker 测试库）

环境：`svc-workflow-test-pg` 容器（postgres:16-alpine，127.0.0.1:55432，postgres/postgres，初始库 `svc_workflow`，迁移自动应用）。

测试基建改动（默认值不变，向后兼容）：
- `tests/common/mod.rs` 新增 `test_database_url()` / `test_database_base()` / `admin_database_url()`：读 `TEST_DATABASE_URL` 环境变量，缺省用原硬编码 URL
- 12 处测试文件硬编码 `localhost:5432` 替换为上述 helper（00_upgrade_verification、17 子树 e2e/database + 9 个 atomicity/helpers 文件）
- 注意：`#[sqlx::test]` 套件（21/22）读 `DATABASE_URL`（并会加载仓库根 `.env`，其指向 `svc_workflow_dogfood`），运行此类套件需同时设置 `DATABASE_URL` 指向 docker 库的 `postgres` 维护库

结果（全部对 docker 测试库）：
| 套件 | 结果 |
|---|---|
| `cargo test --lib` | 140 ✅ |
| 18_self_projection_and_domain_members（含 8 个新 my_domains 用例） | 25 ✅ |
| 17_workflow_runtime（domain_list/worklists/submissions/e2e/原子性等） | 446 ✅ |
| 00_upgrade_verification / 02_domain_owner_tests | 2+2 ✅ |
| 01、03–12、14、15、20、provisioning_validation（14 个套件） | 全部 ✅ |
| 21_instance_cancel_archive / 22_repair_context | 20+10 ✅ |

未运行：13/16/19（既有编译错误，`semantic_model_version` 字段由近期 commit 引入、测试未同步，与本次变更无关）。

复跑命令：
```
export TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/svc_workflow
export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/postgres   # 仅 sqlx::test 套件需要
cargo test --test 18_self_projection_and_domain_members
```
停止/删除测试容器：`docker rm -f svc-workflow-test-pg`

### 遗留说明

- 测试 13/16/19 存在既有编译错误（`semantic_model_version` 字段由近期 commit 引入，测试未同步更新），与本次变更无关，全量 `cargo test` 因此受阻
- 契约中既有 `MemberItem` schema 为 camelCase 而实际 wire 为 snake_case（既有漂移，未在本次修正）

## 证据索引

- 路由全集：`src/http/mod.rs:31-203`
- domain_instances handler + DOMAIN_OWNER 校验：`src/http/handlers/instances.rs:100`、`src/application/workflow_instance/query_service.rs:74`、`src/store/postgres/workflow_instance_repository/query_visibility.rs:34`
- 成员管理（domain-scoped）：`src/http/handlers/domain_members.rs`、`src/store/postgres/domain_role_repository.rs`
- 表结构：`migrations/0001_identity_domain.sql:65-90`
- 正式契约：`contracts/workflow-http/v1/openapi.yaml`（无 caller-scoped 域端点）
- Broker grouped tool：`openclaw-adc-canary-extension/broker/src/adapters/workflow-read.ts`
- 产品边界：`PRODUCT-BOUNDARY.md`（Domain 隔离归属 svc-workflow）
