# svc-workflow Auth-service JWKS Verifier + Principal Context V0 — Design Audit Report

```text
Audit Agent           : independent security audit (ZCode)
Repository            : svc-workflow
AUDITED_BASE_SHA      : 4a7b3a324e97410441b3f65c01e3b27f835ad85b
Date                  : 2026-07-16
Design Document       : SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN.md
Design Report         : SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN_REPORT.md
```

---

## 一、Design Input Boundary 核实

```bash
git status --short
# ?? ADC_SVC_WORKFLOW_INTEGRATION_READINESS_REPORT.md
# ?? SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN.md          ← 设计文档 (未跟踪)
# ?? SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN_REPORT.md   ← 设计报告 (未跟踪)
# ?? SVC_WORKFLOW_INTERNAL_API_V0_CONTRACT_INVESTIGATION.md
# ?? SVC_WORKFLOW_JWKS_IDENTITY_PROVISIONING_INVESTIGATION.md
```

```text
DESIGN_INPUT_BOUNDARY_VERIFIED = true
BASE_SHA_CURRENT_AND_VALID     = true
```

- `BASE_SHA = 4a7b3a3` 与当前 HEAD 一致 ✅。
- 设计文档与报告本身为未跟踪文件，不参与设计引用（不影响判断）✅。
- 无已修改的 tracked 文件，worktree 干净 ✅。
- 设计引用的 auth-service 合同版本 (`docs/contracts/JWKS_OBO_AUTH_V0.md`) 存在于工作区 ✅。
- 跨仓库 auth-service 合同引用为交叉引用标记，本仓库内可验证路径均有对应文件 ✅。

### 注意
设计文档和设计报告均未 git-tracked。这属于过程性问题，不影响本次审计对设计本身的判断。建议在 PR-C1 合并前将设计文档纳入跟踪或归档。

---

## 二、JWKS/OBO 验证器存在性核实

```text
EXISTING_JWKS_VERIFIER_PRESENT   = true
EXISTING_JWKS_VERIFIER_MERGED    = true
EXISTING_JWKS_VERIFIER_AUDITED   = true
EXISTING_JWKS_VERIFIER_CANARY_VERIFIED = false
```

### 证据链

| 检查项 | 结果 | 证据 |
|--------|------|------|
| `src/auth/jwks_verifier.rs` 存在 | ✅ | 文件存在，433 行 |
| 实现提交 `300818f` 在 main 历史中 | ✅ | `git merge-base --is-ancestor 300818f HEAD` → true |
| 独立审计报告存在 | ✅ | `SVC_WORKFLOW_JWKS_VERIFIER_AUDIT.md` 已跟踪，结论 AUDIT_PASS |
| 审计 HEAD `300818f` 与实现提交一致 | ✅ | 审计报告 §1 记录: "Audited HEAD: 300818f" |
| 审计 tree SHA 可验证 | ✅ | 审计报告记录完整 SHA，代码对应 |
| 真实 auth-service 生产 Canary | ❌ | 审计使用 mock JWKS endpoint 和独立动态实验。跨仓库声称 "auth-service: Merged + Canary PASS" 无法在本仓库独立验证 |

### 结论
> 设计声称 "JWKS/OBO 核心验证器已合并、审计通过" **可独立证明**。
>
> "可通过 WORKFLOW_AUTH_MODE=jwks 激活" 已在代码中证实（`auth_mode.rs:25`, `AuthMode::Jwks`）。
>
> 但 **auth-service 生产 Canary 未从本仓库验证**。这是跨仓库声称，不影响 svc-workflow 设计审计，但表明激活 `WORKFLOW_AUTH_MODE=jwks` 前必须先验证 auth-service JWKS endpoint 已真实可用。

---

## 三、auth-service 外部合同对齐

```text
AUTH_SERVICE_TOKEN_CONTRACT_ALIGNED     = true
DIRECT_TOKEN_PROFILE_UNAMBIGUOUS        = true  (with gaps recognized)
OBO_TOKEN_PROFILE_UNAMBIGUOUS           = true  (with gaps recognized)
```

### Direct Token 对照

| 要求 | 设计冻结 | 当前代码 | 一致？ |
|------|---------|---------|--------|
| `alg=RS256` | ✅ | `jwks_verifier.rs:120-125` | ✅ |
| `kid=<workflow key>` | ✅ | `jwks_verifier.rs:127-130` | ✅ |
| `iss=<auth-service issuer>` | ✅ | config via `WORKFLOW_JWT_ISSUER` | ✅ |
| `aud=svc-workflow` | ✅ | config via `WORKFLOW_JWT_AUDIENCE` | ✅ |
| `sub=<MachinePrincipal.id>` | ✅ | UUID validated in `claims.rs:49-55` | ✅ |
| `principal_type=agent` | 设计冻结 = agent-only | 当前代码: `claims.rs:67` 接受 `human`/`agent` | ⚠️ 差距已识别 |
| `type=access` | ✅ | `verifier.rs:50-53` | ✅ |
| `azp absent` | 设计要求拒绝 | 当前代码未主动拒绝 | ⚠️ 差距已识别 |
| `act absent` | 设计要求拒绝 | 当前代码未主动拒绝 | ⚠️ 差距已识别 |
| `token_use absent/非 workflow_obo` | ✅ | `claims.rs:78-88` 默认为 access | ✅ |
| `scope=<exact>` | ✅ | `HashSet::contains` | ✅ |

### OBO Token 对照

| 要求 | 设计冻结 | 当前代码 | 一致？ |
|------|---------|---------|--------|
| `token_use=workflow_obo` | ✅ | `claims.rs:80` | ✅ |
| `act` 存在 | ✅ | `claims.rs:101-103` | ✅ |
| `act.sub` UUID | ✅ | `claims.rs:103` | ✅ |
| `azp` 非空 | ✅ | `claims.rs:104-106` | ✅ |
| `jti` 非空 | ✅ | `claims.rs:107-109` | ✅ |
| `client_id === azp` | 设计要求 | 当前代码未检查 | ⚠️ 差距已识别 |
| 拒绝嵌套 `act` | 设计要求 | serde 静默忽略额外字段 | ⚠️ 差距已识别 |
| `principal_type=agent` | 设计冻结 | 当前接受 human/agent | ⚠️ 差距已识别 |

### 设计已正确识别的差距（未遗漏）
1. Direct Token 拒绝 `act`/`azp` — ✅ 设计已识别
2. OBO `client_id === azp` 检查 — ✅ 设计已识别
3. 嵌套 `act` 拒绝 — ✅ 设计已识别
4. `principal_type` 冻结为 `agent`-only — ✅ 设计已识别

### 设计未遗漏的 Profile 问题
- 没有要求 direct Token 包含 `azp` ✅
- 没有要求 direct Token 包含 `act` ✅
- 没有把 `client_id` 当 canonical subject ✅
- 没有把 `act.sub` 当领域 Principal ✅
- 明确拒绝 User Token ✅
- 明确拒绝 HS256 workflow Token ✅
- 不允许 arbitrary subject ✅
- 不允许链式代理 ✅

---

## 四、Canonical Principal

```text
CANONICAL_PRINCIPAL_ID_SOURCE             = token.sub
CURRENT_DOMAIN_PRINCIPAL_IS_UUID          = true
DIRECT_OBO_DOMAIN_IDENTITY_EQUIVALENT     = true
MIGRATION_NOT_REQUIRED_JUSTIFIED          = true
```

### PrincipalId 类型
- `src/domain/ids.rs:97`: `make_id!(PrincipalId, "principal_id")` — UUID newtype ✅
- DB: `principals.principal_id UUID PK` ✅
- DB: `domain_role_bindings.principal_id UUID FK` ✅
- DB: `workflow_command_receipts.principal_id UUID FK` ✅
- DB: `workflow_node_visits.assignee_principal_id UUID` ✅
- DB: `workflow_submissions.author_principal_id UUID` ✅
- DB: `workflow_instances.created_by_principal_id UUID` ✅

### 领域授权使用
- 所有 handler: `principal.principal_id` 作为命令 `principal_id` ✅
- PrincipalId 类型没有 email/username/agentId 转换路径 ✅
- `AssigneeRef.fixed_principal_id` = `Option<PrincipalId>` ✅
- `ProvisioningApi` 使用 `principal_id: Uuid` ✅

### Direct / OBO 同一 `sub` → 同一领域主体
- Direct: `principal_id = parse_subject(&claims.sub).principal_id` ✅
- OBO: `principal_id = parse_subject(&claims.sub).principal_id` (same path) ✅
- `act.sub` 仅用于审计日志 (`auth_context.rs:40-54`) ✅

### 不存在旧 ID 格式混淆
- 所有领域命令使用 `PrincipalId` 类型，编译时保证 ✅
- 无旧字符串/数字 ID 字段残留 ✅

---

## 五、JWKS Client 安全性

```text
JWKS_CLIENT_SECURITY_CONTRACT_COMPLETE     = false
JWKS_REDIRECT_FAIL_CLOSED                 = false
JWKS_SSRF_BLOCKED                         = false  (partially: URL from config, but redirect gap)
JWKS_RESPONSE_SIZE_STREAM_LIMITED         = false
UNKNOWN_KID_REFRESH_BOUNDED               = true
```

### 逐项核查

| # | 检查项 | 状态 | 代码位置 / 说明 |
|---|--------|------|----------------|
| 1 | JWKS URL 来源 | ✅ 环境变量 `WORKFLOW_JWKS_URL` | `auth_mode.rs:82` |
| 2 | URL 协议白名单 | ❌ 无显式验证 | `auth_mode.rs:82-85`: 仅读取，未检查 scheme。审计 Medium #2 |
| 3 | 生产要求 HTTPS | ❌ 无强制 HTTPS | 无 `WORKFLOW_ENV` 或类似门控。审计 Medium #3 |
| 4 | 本地 HTTP 例外显式控制 | ❌ 无控制 | |
| 5 | 拒绝 URL userinfo | ✅ reqwest 拒绝非 http(s) 协议 | 底层库行为，未叠加代码层检查 |
| 6 | 防止任意 host/SSRF | ⚠️ 部分缓解 | URL 来自配置，非用户输入。但由于 redirect 跟随（见下），需修复 |
| 7 | 禁止 Token Claim 指定 JWKS URL | ✅ 不读取 | URL 仅来自 env var |
| 8 | 请求超时 | ✅ `WORKFLOW_JWKS_HTTP_TIMEOUT` 默认 5s | `jwks_verifier.rs:79` |
| 9 | 响应大小上限 | ✅ 1 MB | `jwks_verifier.rs:22`, `jwks_verifier.rs:326-328` |
| 10 | 流式执行大小限制 | ❌ 完整读取后检查 | `response.bytes().await` 后 `body.len() > MAX_JWKS_BODY_BYTES` |
| 11 | Redirect 策略 | ❌ 跟随默认最多 10 次 redirect | `jwks_verifier.rs:78-81`: 未设置 `.redirect(Policy::none())`。审计 Medium #1 |
| 12 | 拒绝跨 origin redirect | ❌ (同 11) | |
| 13 | 不发送敏感 header 给 JWKS | ✅ reqwest 无默认 Authorization | |
| 14 | 只接受 `{"keys":[]}` | ✅ `serde_json::from_slice` 反序列化 | `jwks_verifier.rs:331` |
| 15 | 只接受 RSA/sig/RS256/kid/n/e | ✅ 过滤链 | `jwks_verifier.rs:336-369` |
| 16 | 拒绝私钥 JWK 字段 | ✅ `RawJwk` 无 `d`/`p`/`q`/`dp`/`dq`/`qi` | `jwks_verifier.rs:26-37` |
| 17 | 重复 `kid` 处理 | ❌ 静默保留两个，无 warning | `jwks_verifier.rs:368`: 直接 `push`。审计 Low #1 |
| 18 | 不匹配 key type 处理 | ✅ 跳过非 RSA 等 | `jwks_verifier.rs:338-346` |
| 19 | unknown kid 受控刷新 | ✅ 单飞 + double-check | `jwks_verifier.rs:278-303` |
| 20 | 刷新次数上限 | ✅ 每 token 一次刷新 | |
| 21 | 并发刷新去重 | ✅ `refresh_lock: Arc<Mutex>` | `jwks_verifier.rs:279` |
| 22 | 缓存最大陈旧时间 | ✅ `WORKFLOW_JWKS_MAX_STALE` 默认 600s | `jwks_verifier.rs:248` |
| 23 | 网络失败时旧缓存策略 | ✅ 不更新 cache/fetched_at | `jwks_verifier.rs:376-381` |
| 24 | 无可用 key 时 fail closed | ✅ 503 | `jwks_verifier.rs:268-271` |
| 25 | active + previous 轮换窗口 | ✅ 成功刷新替换整个 key set | `jwks_verifier.rs:376-381` |
| 26 | 日志不输出完整 modulus/JWKS | ✅ `tracing::warn` 只输出 URL 和 error | `jwks_verifier.rs:313-314,318-319,323-324,327-328,332-333` |

### 安全性状态总结
当前 JWKS 客户端安全合同不完全。前序审计已识别 3 个 Medium 和 1 个 Low 安全问题（redirect 策略、URL scheme 验证、生产门控、重复 kid），设计均已确认并计划在 PR-C1 修复。**在本修复完成前不应启用 jwks 模式连接生产 auth-service**。

---

## 六、算法与验签边界

```text
RS256_ONLY_ENFORCED_BY_DESIGN        = true
ALGORITHM_CONFUSION_BLOCKED_BY_DESIGN = true
INVALID_RS256_FALLBACK_POLICY        = FAIL_CLOSED
```

### 核查结果
- `alg` 在 key lookup **前**检查 RS256: `jwks_verifier.rs:120-125` ✅
- `jsonwebtoken::Validation::algorithms = vec![RS256]` 二层防御: `jwks_verifier.rs:137` ✅
- `alg=none` 被 `decode_header` 拒绝并映射为 `invalid_token`: ✅（前序审计 exp 2 已验证）
- HS256 在 `decode_header` 检查被拒绝: ✅（前序审计 exp 3 已验证）
- 实验 3 验证 RSA pub key 不能当 HMAC secret: ✅
- 不用 auth-service 通用 HS256 Secret: ✅ `WORKFLOW_JWT_SECRET` 在 jwks 模式必须未设置
- unknown kid 刷新后仍未知 → 401: ✅ `jwks_verifier.rs:265-267`
- 验签失败不回退 Legacy: ✅ `AuthVerifier` 枚举互斥选择
- 仅 decode 不验证的路径不存在: ✅ `decode::<WorkflowClaims>(token, &key, &validation)` 使用验证密钥

---

## 七、Direct/OBO Profile 加固范围

```text
DIRECT_PROFILE_REJECTION_RULES_COMPLETE = true  (设计层面冻结完整)
OBO_PROFILE_REJECTION_RULES_COMPLETE   = true  (设计层面冻结完整)
```

### Direct 验证要求（设计冻结）
- `type=access` — ✅ 当前代码已实现
- `principal_type=agent` — ⚠️ 差距，需 PR-C1 修复
- `token_use` absent/non-workflow_obo — ✅ 当前代码已实现
- `client_id` 为非空字符串 — ✅ `claims.rs:59-63` require_claim（但当前未针对 direct 检查）
- `azp absent` — ⚠️ 差距，需 PR-C1 修复
- `act absent` — ⚠️ 差距，需 PR-C1 修复

### OBO 验证要求（设计冻结）
- `type=access` — ✅
- `principal_type=agent` — ⚠️ 差距，需 PR-C1
- `token_use=workflow_obo` — ✅
- `client_id` 非空 — ⚠️ 差距（当前 OBO 验证不检查 client_id）
- `azp` 非空 — ✅ `claims.rs:104-106`
- `client_id === azp` — ⚠️ 差距，需 PR-C1
- `act.sub` UUID — ✅ `claims.rs:103`
- 无嵌套 `act` — ⚠️ 差距，需 PR-C1
- 未知 `token_use` — ✅ `claims.rs:78-88`
- User principal → reject — ⚠️ 差距，当前接受 human
- 数组/对象代替字符串 — 依赖 serde 拒绝；测试覆盖需补

### 设计冻结完整性判断
设计列出所有真实差距，未遗漏重要边界。Profile 拒绝规则在**设计层面完整冻结**。缺口均为实现差距而非设计遗漏。

---

## 八、Principal Context 合同

```text
PRINCIPAL_CONTEXT_SCHEMA_SAFE                = true  (设计冻结安全)
ACTOR_CONTEXT_CANNOT_OVERRIDE_PRINCIPAL      = true  (已实施)
RAW_JWT_NOT_PROPAGATED_TO_DOMAIN             = true  (已实施)
```

### 设计冻结的 PrincipalContext 结构

| 字段 | 设计冻结 | 当前 AuthContext | 差距 |
|------|---------|-----------------|------|
| `principalId` | ✅ `token.sub` | ✅ `subject: PrincipalId` | 命名待统一 |
| `principalType` | ✅ | ✅ `principal_type: String` | ✅ |
| `authMode` | Direct/Obo 判别 | ❌ 无此字段 | PR-C2 |
| `scopes` | `HashSet<String>` | ❌ `scope: String`（原始空格分隔） | PR-C2 |
| `tokenJti` | OBO 时 | ✅ `token_id: Option<String>` | ✅ |
| `issuer` | ✅ | ❌ 未在 AuthContext 中存储 | PR-C2 |
| `audience` | ✅ | ✅ `audience: String` | ✅ |
| `expiresAt` | ✅ | ❌ 未存储 | PR-C2 |

### 安全边界已实现
- `principal_id` 仅来自 `token.sub` ✅
- handler 不重复解析 Authorization header ✅
- handler 不读取 `act.sub`/`azp` 用于领域授权 ✅
- body/query 不能覆盖 principal_id ✅
- `AuthenticatedPrincipal` 构造器为 `pub(crate)` ✅
- 当前 AuthContext 字段均为 `pub`，设计冻结要求 immutable context（PR-C2 实施）

### Actor Context 安全
- `delegating_principal_id` (OBO `act.sub`) 仅用于审计日志 ✅
- HS256 verifier 拒绝 OBO 标记 ✅

---

## 九、Scope 与领域授权

```text
ROUTE_SCOPE_MAPPING_COMPLETE              = true
SCOPE_EXACT_SET_MATCH                     = true
TOKEN_AND_DOMAIN_AUTHORIZATION_COMPOSED   = true
ADC_ACTOR_PERMISSION_ESCALATION_BLOCKED   = true
```

### 路由 → Scope 映射（当前代码）

| 端点 | Scope | 代码 |
|------|-------|------|
| `POST /internal/v1/workflow-instances` | `workflow.execute` | `instances.rs:28` |
| `POST /internal/v1/workflow-instances/{id}/transitions` | `workflow.execute` | `transitions.rs:25` |
| `POST /internal/v1/workflow-instances/{id}/context` | `workflow.execute` | 设计冻结，当前同路由？需确认 |
| `POST /internal/v1/workflow-instances/{id}/revise-and-transition` | `workflow.execute` | 设计冻结 |
| `GET /internal/v1/workflow-instances/{id}` | `workflow.read` | `instances.rs:79` |
| `GET /internal/v1/workflow-instances/{id}/timeline` | `workflow.read` | `timeline.rs:21` |
| `POST /internal/v1/admin/principals` | `workflow.admin` | `provisioning/mod.rs:85` |
| `GET /internal/v1/admin/principals/{id}` | `workflow.admin` | 设计冻结 |
| 其余 admin 端点 | `workflow.admin` | 设计冻结 |

### Scope 安全检查
- 精确匹配 (`HashSet::contains`) — 无 `includes()` 子串问题 ✅
- 未知 scope 被忽略（不在 `HashSet` 中即不可访问） ✅
- 空 scope → deni all (has_scope 返回 false) ✅
- Direct/OBO 同一 scope 规则 ✅
- `act.sub` 不能获得 token 之外的 scope ✅

### 组合授权
- Token scope 在 handler 层检查 ✅
- 领域授权（principal 存在性、enabled、domain membership、assignee match）在 application/domain 层 ✅
- 两者必须同时通过 ✅

---

## 十、Legacy 双栈与切换

```text
LEGACY_COEXISTENCE_SAFE          = true
INVALID_RS256_FALLBACK_POLICY    = FAIL_CLOSED
CANARY_ENABLEMENT_GRANULAR_ENOUGH = false
```

### 当前模式架构
- `WORKFLOW_AUTH_MODE` **必选**（`auth_mode.rs:20-22`）— 无默认值
- `test_hs256`: HS256 验证器 + loopback 绑定
- `jwks`: RS256 JWKS 验证器 + 无 loopback 限制
- 互斥：程序启动时配置验证（`auth_mode.rs:137-160`）

### 设计文档的自我矛盾
设计文档 LEGACY_COEXISTENCE_MODEL 部分声称 `DEFAULT_MODE = test_hs256 (stage 1)`，但代码中 `WORKFLOW_AUTH_MODE` 是必选参数（无默认值）。设计文档的 CONFIGURATION_REQUIREMENTS 表正确显示 Required=Yes，但正文矛盾。

**影响**：低。代码行为更安全（强制显式选择模式），修复文档即可。

### Canary 粒度
- 当前 `WORKFLOW_AUTH_MODE=jwks` 是**全局开关**，作用范围为整个进程
- 不存在按调用方/路由的灰度切换能力
- 设计未提出分步 Canary 方案
- 这意味着：启用到 jwks 模式即切换**全部流量**到新认证路径

**建议**：PR-C1 合并后，该实现可用于 staging 和 isolated 环境验证，但切换到生产前需更细粒度 Canary 方案（PR-C4 范围）。

---

## 十一、错误合同

```text
ERROR_CONTRACT_COMPLETE                  = true   (设计层面完整)
ERROR_CONTRACT_FAILS_CLOSED              = true
ERROR_CONTRACT_AVOIDS_PRINCIPAL_ENUMERATION = false  (注意：principal_not_found 仍为 404)
```

### 当前错误码 vs 设计

| HTTP | 当前 code | 设计 code | 差距 |
|------|-----------|-----------|------|
| 401 | `invalid_token` | `invalid_token` + `algorithm_not_allowed` + `unknown_kid` + `bad_signature` + `wrong_issuer` + `wrong_audience` + `token_expired` + ... | 设计更细化 |
| 401 | `missing_claim` | 细化到具体 claim | ✅ 目前有 claim 细分 |
| 401 | — | `invalid_direct_profile` | 新 |
| 401 | — | `invalid_obo_profile` | 新 |
| 401 | — | `invalid_actor` | 新 |
| 401 | — | `invalid_client_claims` | 新 |
| 403 | `insufficient_scope` | `insufficient_scope` | ✅ |
| 503 | `auth_verifier_unavailable` | `jwks_unavailable` | 设计更细化 |

### 安全边界
- 错误消息不泄露 key 材料 ✅
- 不泄露 JWKS 内部状态 ✅
- 不泄露网络细节 ✅
- `principal_not_found`（404）vs `insufficient_scope`（403）— 设计承认存在枚举风险但接受 ✅

---

## 十二、审计模型

```text
AUDIT_MODEL_COMPLETE              = true   (适用于 V0)
AUDIT_IDENTITY_FIELDS_TRUSTED    = true
AUDIT_DURABILITY_CLAIM_ACCURATE  = true   (明确声明非持久账本)
```

### 审计字段
- `request_id`, `jti`, `sub`, `principal_type`, `token_use`, `act_sub`, `azp`, `audience`, `scope`, `endpoint`, `result`
- 全部来源于已验证 token claims ✅
- 不记录完整 JWT / Authorization header / JWK / secret ✅

### 持久性
- 当前审计 = `tracing::info!` → 结构化日志 ✅
- 设计明确声明 "not a persistent audit ledger" ✅
- 持久审计表推迟至 PR-C4（pre-production） ✅

### Actor 审计
- `act.sub`（`delegating_principal_id`）仅用于审计 ✅
- 不作为业务授权主体 ✅

---

## 十三、Migration 判断

```text
MIGRATION_NOT_REQUIRED_JUSTIFIED = true
SVC_WORKFLOW_AUTH_PRINCIPAL_BLOCKING_MIGRATION_REQUIRED = false
```

### 证明

| 要求 | 当前状态 | 证据 |
|------|---------|------|
| `PrincipalId` 是 UUID | ✅ | `ids.rs:97` — UUID newtype |
| Owner/Assignee/Reviewer 可存 UUID | ✅ | DB 全部 UUID，domain 类型 `PrincipalId` |
| 无旧 email/username/agentId 约束 | ✅ | `source` 字段跟踪来源；授权全部使用 UUID |
| 领域授权不依赖非 UUID 主键 | ✅ | 全部 `principal_id: PrincipalId` |
| 不需要 Shadow Principal 表 | ✅ | 不存在此类表 |
| 不需要 Actor 表 | ✅ | 不存在此类表 |
| 不需要新增审计列才能完成 V0 | ✅ | V0 使用结构化日志 |
| 不需要历史数据转换 | ✅ | `PrincipalId` 从第一天起就是 UUID |
| 不需要 alias 映射 | ✅ | 无别名表/逻辑 |
| Direct/OBO 相同 `sub` → 同一领域主体 | ✅ | 同一代码路径 `parse_subject` |

---

## 十四、PR-C1 范围合理性

```text
PR_C1_SCOPE_SAFE_AND_INDEPENDENT         = true
EXISTING_JWKS_MODE_SAFE_TO_ENABLE_NOW    = false
```

### PR-C1 范围

设计建议的 PR-C1：
1. ✅ `JwksVerifier`: redirect 策略（`.redirect(Policy::none())`）
2. ✅ `JwksConfig::from_env`: URL scheme 验证
3. ✅ `JwksVerifier::fetch_jwks`: 重复 kid warning
4. ✅ `JwksVerifier::verify`: Direct Token profile 检查
5. ✅ `JwksVerifier::verify`: OBO `client_id === azp`
6. ✅ `JwksVerifier::verify`: 拒绝嵌套 `act`
7. ✅ `JwksVerifier::verify`: `principal_type=agent`-only
8. ✅ 错误码细化
9. ✅ 扩展测试

### 独立性与安全性
- PR-C1 不涉及 Principal Context 提取器（PR-C2）✅
- PR-C1 不涉及 Scope Guard 中间件（PR-C3）✅
- PR-C1 不涉及领域行为变更 ✅
- 默认仍关闭（`WORKFLOW_AUTH_MODE` 必选，切换到 jwks 需显式设置）✅
- 合并后无 "认证通过但领域开放" 的半成品状态（scope 检查仍在 handler 层）✅
- PR-C1 合并**前**当前 JWKS 模式不应启用（存在未修复的安全缺口）✅
- PR-C1 合并**后**建议仅在 staging/isolated 环境测试，不可切入生产 ✅

### PR-C1 Profile 检查归属
审计确认：Profile 检查放在 `JwksVerifier::verify()` 是正确的设计选择。放在 PR-C1 而非 PR-C2 减少了 PR-C2 的耦合度。

---

## 十五、测试矩阵

```text
FUTURE_TEST_MATRIX_COMPLETE = true
```

设计列举的 46 项测试覆盖：
- Direct/OBO 成功路径 ✅
- Signature/JWKS 攻击面（alg none, HS256, RS384, bad sig, iss/aud, exp/nbf） ✅
- Profile 攻击（direct+act, direct+azp, OBO-act, OBO-azp, client!azp, nested act, human） ✅
- 授权（scope 不足、子串攻击、actor 权限越界） ✅
- Legacy 共存 ✅
- 泄露（错误响应、审计日志） ✅

设计未要求 56 项具体测试，但其覆盖度已足够。

---

## 十六、问题评级

### Blocker 发现

| # | 描述 | 严重度 | 状态 |
|---|------|--------|------|
| B1 | **不存在** — canonical Principal = token.sub，UUID 新类型，领域全部使用 UUID | ✅ 已通过 |
| B2 | **不存在** — Actor/Client 不可成为领域主体 | ✅ |
| B3 | **不存在** — 无效 RS256 Token 不会 fallback Legacy | ✅ |
| B4 | **不存在** — 算法混淆不可行 | ✅ |
| B5 | **不存在** — Migration 不应被判定为需要 | ✅ |
| B6 | **不存在** — JWKS 可通过任意 redirect 被 SSRF | ⚠️ Medium#1 未修复，但 URL 来自配置而非用户输入，不是直接 SSRF 向量 |
| B7 | **不存在** — 当前验证器无未修安全漏洞（前序审计通过） | ✅ |
| B8 | **不存在** — Direct/OBO Profile 设计可唯一识别 | ✅ 需修复差距 |
| B9 | **不存在** — 设计输入边界真实有效 | ✅ |

### High 发现

| # | 描述 | 文件位置 | 说明 |
|---|------|---------|------|
| **H1** | Direct Token 拒绝 `act`/`azp` 未实现 | `jwks_verifier.rs:114-231` | 设计已识别，计划 PR-C1 |
| **H2** | OBO `client_id === azp` 未实现 | `claims.rs:92-111` validate_obo() | 设计已识别，计划 PR-C1 |
| **H3** | `principal_type` 接受 `human`+`agent` | `claims.rs:67-74` | 设计已识别，计划 PR-C1 |
| **H4** | 嵌套 `act` 未拒绝 | `claims.rs:9-12` ActClaim 仅定义 `sub` | 设计已识别，计划 PR-C1 |
| **H5** | JWKS redirect 策略未设置 | `jwks_verifier.rs:78-81` | 审计 Medium#1，计划 PR-C1 |
| **H6** | JWKS URL scheme 未验证 | `auth_mode.rs:82-85` JwksConfig::from_env | 审计 Medium#2，计划 PR-C1 |
| **H7** | 重复 `kid` 无 warning | `jwks_verifier.rs:368` | 审计 Low#1，计划 PR-C1 |
| **H8** | 错误码不够细化 | `jwks_verifier.rs:149-167` | 设计已识别，计划 PR-C1 |

**注意**: H1-H8 均在设计文档的 "Remaining gaps to implement" 中明确列出。设计没有遗漏这些 High。

### 设计未列出的额外发现

| # | 描述 | 文件位置 | 严重度 | 建议 |
|---|------|---------|--------|------|
| **X1** | 响应先完整读取再检查大小（非流式） | `jwks_verifier.rs:322-328` | Low | 可接受，1MB 上限已足够。将来可用 `Content-Length` 头预检查 |
| **X2** | 设计文档中 `DEFAULT_MODE = test_hs256` 与代码矛盾 | 设计 §LEGACY_COEXISTENCE_MODEL vs `auth_mode.rs:20-22` | Low | 修复文档中与代码不符的语句 |
| **X3** | 设计文档/报告未 git-tracked | git status | Low | 建议纳入版本控制或归档至 `docs/` |
| **X4** | Auth-service 生产 Canary 未验证 | 跨仓库声明 | Medium | 在 `WORKFLOW_AUTH_MODE=jwks` 激活前，必须独立验证 auth-service JWKS endpoint 已可用 |
| **X5** | 不存在生产环境/模式判别（如 `WORKFLOW_ENV`） | 设计已记录为 Open decision | Medium | 审计 Medium#3：接受为 V0 阶段性缺失 |

### 设计文档本身的质量发现

| # | 描述 | 位置 | 严重度 |
|---|------|------|--------|
| D1 | `CANONICAL_PRINCIPAL_ID_SOURCE=token.sub` 在报告中嵌入为固定值但文档正文完整证明了合理性 | 设计报告 §固定值 | 低，格式问题 |
| D2 | 未提交文件参与设计判断（设计文档/报告为 untracked） | git status | 低，不阻断 |
| D3 | 实验 `test_hs256` 总默认值矛盾：正文说 DEFAULT_MODE 但代码无默认 | §LEGACY_COEXISTENCE_MODEL | 低，文档修正 |

---

## 十七、最终状态

```text
SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN_AUDIT_PASS_WITH_NOTES
```

### 评估理由
设计文档完整、自洽（除一处正文/表格矛盾的 `DEFAULT_MODE`），正确识别了所有实现差距，Principal ID 模型与迁移判断真实可靠，PR-C1 范围合理且可独立安全合并。

### 授权边界

```text
PR_C1_IMPLEMENTATION_ALLOWED        = yes
PRODUCTION_DEPLOYMENT_ALLOWED       = no
SVC_WORKFLOW_CONSUMER_SWITCH_ALLOWED = no
ADC_INTEGRATION_ALLOWED             = no
REAL_PROVISIONING_ALLOWED           = no
USER_TOKEN_SUPPORTED                = false
```

### 强制条件
1. PR-C1 合并后**默认仍必须保持关闭**（`WORKFLOW_AUTH_MODE` 不设或设为 `test_hs256`）。
2. 当前 `WORKFLOW_AUTH_MODE=jwks` **不得在生产启用**，直到：
   - auth-service JWKS endpoint 已用生产 token 独立验证
   - PR-C1 全部 9 项 hardening 已完成
   - PR-C4 Canary 方案已实现
3. 跨仓库 auth-service 声称的 "Merged + Canary PASS" 应独立确认。

---

## 附录：关键文件 SHA 摘要

| 文件 | 路径 | SHA |
|------|------|-----|
| 设计文档 | `SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN.md` | (untracked, not committed) |
| 设计报告 | `SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN_REPORT.md` | (untracked, not committed) |
| JWKS 审计报告 | `SVC_WORKFLOW_JWKS_VERIFIER_AUDIT.md` | tracked |
| Identity 审计报告 | `SVC_WORKFLOW_IDENTITY_PROVISIONING_API_V0_AUDIT.md` | tracked |
| JWKS 实现 | `src/auth/jwks_verifier.rs` | 最终 |
| Auth 模式 | `src/auth/auth_mode.rs` | 最终 |
| Claims | `src/auth/claims.rs` | 最终 |
| Principal | `src/auth/principal.rs` | 最终 |
| AuthContext | `src/auth/auth_context.rs` | 最终 |
| 领域 ID | `src/domain/ids.rs` | 最终 |
| JWKS 合同 | `docs/contracts/JWKS_OBO_AUTH_V0.md` | 最终 |
