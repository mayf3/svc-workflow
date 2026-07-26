# svc-workflow Identity Provisioning API v0 独立复审报告

## 1. 审计结论

- 审计对象：`feat/identity-provisioning-api-v0`
- 最终提交：`b4e8d1f935eaf720e5fc756b2b77bf071097a9fb`
- 最终 tree：`c75d51f3bcbd8ad2b27ce74fdf8a8cc8eeb2e2b9`
- 对比基线：`main` / `d28f578`
- 审计方式：锁定最终 SHA 后进行完整静态检查、全量测试、隔离 PostgreSQL/TCP 动态复测及并发压力复测
- 结论：**可以合并**
- 严重度统计：Blocker 0 / High 0 / Medium 1 / Low 1

最初审计发现的 8 个 High 均已关闭。当前唯一 Medium 是自举与管理员并发写同一新 principal 时的锁序反转；失败事务正确回滚并以 503 暴露，使用相同 idempotency key 重试即可成功，没有观察到状态损坏，因此不阻断本轮合并，但应作为紧随其后的可靠性修复。

## 2. 审计边界与契约基准

本轮以仓库内已跟踪的 `svc-workflow` provisioning contract 为接口基准。跨仓库身份契约的正式 V1 scope 为 `workflow.admin`；未跟踪草稿中的 route/body/OBO 形状不作为本 PR 的冻结契约。

最终实现已统一使用 `workflow.admin`，并保留以下安全边界：

- 仅 allowlist 中的 direct AGENT 可访问 provisioning API。
- human token 不可访问。
- OBO / delegated token 不可访问。
- actor 必须在数据库中存在、类型为 AGENT 且处于 enabled 状态；仅“创建自身 principal”的自举入口允许 actor 尚不存在。
- 每次写入均在事务内再次验证 actor，避免鉴权后到写入前的失效窗口。

## 3. 原始 8 个 High 的关闭情况

### H1 scope 漂移：已关闭

实现与已跟踪契约均使用 `workflow.admin`，不再使用临时的 `workflow.provision`。

### H2 actor/OBO/human 鉴权绕过：已关闭

- HS verifier 明确拒绝 OBO 标记。
- JWKS 路径下 human、`workflow_obo`、`act + access` 均被拒绝。
- disabled actor 在服务重启前后均无法继续写入。
- 写事务内重新校验 actor，使已完成的禁用对后续写入立即生效。

### H3 确定性失败遗留 PROCESSING receipt：已关闭

业务确定性错误会将 receipt 完成并保存外部响应；相同 key/body 重放返回同一业务错误。基础设施与一致性错误回滚整个事务。

动态验证：principal 类型冲突首次返回 409，重放仍返回相同 409；数据库 receipt 状态为 `COMPLETED`、状态码为 409。

### H4 request hash 覆盖不完整：已关闭

hash 已覆盖 `source`、`sourceRevision`、`displayName` 等影响语义或审计来源的字段。

动态验证：相同 idempotency key 仅修改 source revision 或 display name 均返回 409 `idempotency_conflict`。

### H5 空库自举悖论：已关闭

允许 allowlisted AGENT 在空库中创建与 token subject 完全相同的自身 principal；actor principal 与 receipt 在同一事务中建立。

动态验证：空库从 0 principal 开始，错误 target 返回 403，自身 target 返回 200；principal 类型为 AGENT、enabled，provenance 与 receipt 均正确落库。

### H6 domain owner 替换 TOCTOU：已关闭

owner 的查询、相关行锁、旧 owner 禁用和新 owner upsert 已移入同一事务，并使用统一 provisioning advisory lock。

动态验证：两个 owner 替换请求并发执行均返回 200，最终仅一个 enabled owner，旧 owner 已禁用。

### H7 审计 actor 错误：已关闭

审计与 receipt 使用真实认证 actor；provisioning provenance 持久化到 principal metadata。

### H8 质量门禁失败：已关闭

最终 SHA 通过格式化、编译、clippy、测试发现、全量串行和默认并行测试。新 provisioning 实现文件均不超过 500 行。

## 4. 最终 SHA 验证结果

以下命令均在 `b4e8d1f935eaf720e5fc756b2b77bf071097a9fb` 上执行：

| 验证项 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo build --locked --all-targets --all-features` | 通过 |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | 通过 |
| `cargo test --locked --all-targets --all-features -- --list` | 通过，发现 508 tests |
| `cargo test --locked --all-targets --all-features -- --test-threads=1` | 508/508 通过 |
| `cargo test --locked --all-targets --all-features` | 508/508 通过 |
| `git diff --check` | 通过 |

迁移文件 `0001..0010` 未被本分支改写。新/拆分后的 provisioning 实现文件最大 500 行；目录直接子项不超过 20，最大深度为 4。

仓库中仍有基线之前已存在的超长文件，例如 `tests/.../jwks_auth.rs` 和架构文档；它们不是本分支引入的阻断项。

## 5. 隔离数据库与真实 HTTP 动态验证

所有场景使用临时 PostgreSQL 数据库和真实 TCP server；JWKS 场景使用真实 RS256 token 与独立 mock JWKS endpoint。

已通过：

1. 空库自身自举及错误 target 拒绝。
2. HS direct human、OBO、`act + access` 拒绝。
3. JWKS direct human、`workflow_obo`、`act + access` 拒绝。
4. source/sourceRevision 进入 idempotency hash。
5. displayName 进入 idempotency hash。
6. 确定性 409 完成 receipt，并可稳定重放。
7. 未知字段 `actorPrincipalId` 返回 400，且不产生目标 principal。
8. 非法 role key 返回 422。
9. 存储异常映射为 503，而不是伪装成业务 404。
10. 相同 domainKey 并发创建最终只保留一条记录，响应为 200/409。
11. 并发 owner 替换后恰好一个 enabled owner。
12. definition 查询正确返回 `workflowDefinitionId`、`domainId`、`domainEnabled` 与 `canCreate`；禁用 domain 后 `canCreate=false`。
13. actor 并发写入与自禁用保持事务一致；禁用完成后的请求返回 403，服务重启后仍返回 403。

临时服务已停止，临时数据库已删除。

## 6. 剩余问题

### M1 自举与管理员写同一新 principal 时存在锁序反转

严重度：Medium。

`ensure_provisioning_actor` 会在获取全局 provisioning advisory lock 之前插入自举 actor；与此同时，已有管理员请求会先获取全局 advisory lock，再 upsert 同一个新 principal。这形成：

1. 自举事务持有 principal insert/unique-key 相关锁，等待 advisory lock。
2. 管理员事务持有 advisory lock，等待同一 principal 的数据库锁。

PostgreSQL 检测到死锁后会中止其中一个事务，API 返回 503。

10 组 fresh actor 并发压力复测中，多组出现一侧 503、另一侧 200；所有失败请求使用原 idempotency key/body 立即重试后均返回 200。未出现重复 principal、悬挂 receipt 或错误最终状态。

建议：统一事务锁顺序，先获取 provisioning advisory lock，再执行自举 actor insert、actor revalidation、receipt 创建及业务 upsert。同步评估全局 advisory lock 对 control-plane 吞吐的串行化影响；如需拆分锁粒度，必须保持固定的锁层级。

### L1 长度限制按 UTF-8 字节数而非 Unicode 字符数计算

严重度：Low。

部分长度校验使用 Rust `String::len()`，实际计算 UTF-8 字节数；契约文字使用“字符”表述。因此合法的多字节名称可能比预期更早被拒绝。建议明确契约是 bytes/code points/graphemes，或改用与契约一致的计数方式。

## 7. 合并与后续建议

该 SHA 已满足本轮“Blocker/High 清零后可合并”的门槛，可以进入合并与后续 ADC 集成。建议将 M1 作为合并后第一项可靠性修复，并补一条确定性的双事务并发回归测试，固定锁顺序和重试语义。

本报告文件仅作为未提交审计产物保留，不改变被审计提交及其 tree。
