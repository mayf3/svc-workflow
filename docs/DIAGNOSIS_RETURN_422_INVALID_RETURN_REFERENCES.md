# RETURN 转换 422 invalid_return_references 根因与修复（2026-08-15）

## 1. 现象

实例 `121e76b4-c585-470d-869d-291177a50db0`（project-insight-review-v1 定义，article-review 梳理，当前
cto_review，workflowStateVersion=4）：调用 transition `0059182a-af52-47eb-8b4a-c3019f1df69a`
（return-from-cto-review → pm_review，`executable_for_actor=true`，summary 已满足 submission_schema），
连续 2 次返回 `422 {"code":"invalid_return_references"}`。

对照：同定义同参数形态的 advance-publish（`655ab511-3743-4cce-a412-687951942d3d`）在实例
`99d369f6` 上正常成功（v4→v5），排除鉴权与 schema 编译问题。

影响面：所有 project-insight-review-v1 实例的 cto_review 打回通道失效；同定义
return-from-verify（`84b1084b`）疑似同样受影响。

## 2. 根因（两个缺陷叠加）

### 缺陷 A：RETURN 特有必填字段未在 submission_schema 中声明（契约黑洞）

引擎对 RETURN 转换在 schema 校验**之后**、事务内执行 `validate_return_references`，强制要求：

- `rootCauseNodeVisitId`：必填，合法 UUID，且必须属于本实例的 `workflow_node_visits`
- `reasonCode`：必填
- `reason`：必填
- `relatedSubmissionIds`（可选）：若提供，必须是 UUID 字符串数组，且每个都属本实例
  `workflow_submissions`

但定义的 submission_schema（如 project-insight-review-v1 的 return-from-cto-review）通常只声明
`summary` 等业务字段。调用方严格按 schema 构造 payload（只交 summary）→ schema 校验通过 →
`validate_return_references` 因缺 rootCauseNodeVisitId/reasonCode/reason 判定失败。

即：**契约分为两层（schema 层 + 引擎 RETURN 层），schema 未暴露 RETURN 层要求**，调用方按可见契约
提交必然 422，且无法预知。

### 缺陷 B：HTTP 层吞掉 detail，错误不可定位

`src/http/error.rs` 的 `from_transition` 中：

```rust
E::InvalidReturnReferences(_) => {
    unprocessable("invalid_return_references", "return references are invalid")
}
```

丢弃了 `InvalidReturnReferences(String)` 中携带的具体原因（缺哪个字段、哪个 ID 非法/跨实例），
调用方只能看到固定文案 "return references are invalid"——连续 2 次 422 且零进展的直接原因。

## 3. 修复方案（三层，与 ARCH_DESIGN 一致）

### L0a 错误透出（src/http/error.rs）

`InvalidReturnReferences(detail)` 映射时保留 code `invalid_return_references`（向后兼容），
同时通过 `with_details(json!({"detail": detail}))` 透出具体原因。调用方可一次性看到
缺哪些字段 / 哪个引用非法，不再盲猜。

### L0b 契约显性化（src/store/postgres/workflow_instance_repository/transition_validation.rs）

- 新增纯函数 `collect_return_contract_errors(payload) -> Vec<String>`：在 DB 查询前一次性
  聚合所有缺失/格式非法的 RETURN 契约字段（rootCauseNodeVisitId 缺失或非 UUID、
  reasonCode/reason 缺失、relatedSubmissionIds 形状非法、条目非 UUID）。
- `validate_return_references` 先调用它；有错则单条错误消息列出**全部**问题，并声明完整契约：
  `RETURN submissions require: rootCauseNodeVisitId (valid UUID), reasonCode, reason,
  relatedSubmissionIds (optional array of UUIDs)`。
- 引擎层不改各定义静态 submission_schema（避免破坏既有定义兼容），而是让校验失败时
  返回完整契约提示——调用方提交前可知。

### L1 回归测试（tests + 单元 PBT）

- 集成测试补齐四类用例（`tests/17_workflow_runtime/transition/submission_validation.rs`）：
  - 正常路径：rootCause 为上游 visit + 空 relatedSubmissionIds + reasonCode/reason → 成功
  - 缺字段：仅 summary（复现 incident 场景，schema 只声明 summary）→ 聚合错误列出全部 3 项
  - 缺单个字段：schema 声明 reasonCode/reason 但缺 rootCauseNodeVisitId → 单字段错误
  - 跨实例：rootCause / relatedSubmissionId 属其他实例 → 拒绝（既有 2 例保留）
  - 格式非法：rootCause 非 UUID → 明确提示
- 单元 PBT（`transition_validation.rs` 内 `proptest`）：
  - 任意字符串 rootCauseNodeVisitId：合法 UUID 则零错误，非法则必报（绝不静默忽略）
  - 任意字节序列反序列化为 JSON：校验器总终止且错误数有界
- HTTP 单元测试：`invalid_return_references_exposes_detail` 验证 code 稳定 + detail 透出。

## 4. RETURN reference 判 invalid 的全部条件（AC3 文档化）

| 条件 | 校验位置 | 错误消息要点 |
|------|----------|--------------|
| `rootCauseNodeVisitId` 缺失 | collect_return_contract_errors | "rootCauseNodeVisitId is required and must be a valid UUID" |
| `rootCauseNodeVisitId` 非 UUID | collect_return_contract_errors | "rootCauseNodeVisitId is not a valid UUID: '<值>'" |
| `rootCauseNodeVisitId` 不属于本实例（不存在或跨实例） | validate_return_references（DB 查询） | "rootCauseNodeVisitId does not exist or belongs to a different instance" |
| `reasonCode` 缺失 | collect_return_contract_errors | "reasonCode is required for RETURN submissions" |
| `reason` 缺失 | collect_return_contract_errors | "reason is required for RETURN submissions" |
| `relatedSubmissionIds` 存在但不是数组 | collect_return_contract_errors | "relatedSubmissionIds must be an array of UUID strings when present" |
| `relatedSubmissionIds` 条目非字符串 | collect_return_contract_errors | "relatedSubmissionIds entries must be strings" |
| `relatedSubmissionIds` 条目非 UUID | collect_return_contract_errors | "relatedSubmissionIds entry is not a valid UUID: '<值>'" |
| `relatedSubmissionIds` 条目不属于本实例 | validate_return_references（DB 查询） | "relatedSubmissionId <id> does not exist or belongs to a different instance" |

注：本例（121e76b4）中 pm_review 来源 visit 存在于 upstream_submissions，但调用方 payload 缺
rootCauseNodeVisitId/reasonCode/reason——所以即使 visit 存在也会在字段缺失检查处失败。
字段缺失检查先于 DB 归属检查执行（契约字段先行，避免无谓查询）。

## 5. 验证证据

- `cargo test --lib`：148 passed（含 2 组 PBT + HTTP detail 透出测试）
- `cargo test --test 17_workflow_runtime`：450 passed（含 5 个新增 RETURN 用例）
- `cargo check --all-targets`：exit 0

## 6. 兼容性与回滚

- 错误响应 code 不变（`invalid_return_references`），仅新增 `details.detail` 字段——向后兼容，
  不影响既有客户端错误分类逻辑。
- 引擎 RETURN 契约语义未变（必填字段与归属校验不变），只改错误信息聚合与透出。
- 回滚 = revert 本修复 commit；无 schema/数据迁移。
