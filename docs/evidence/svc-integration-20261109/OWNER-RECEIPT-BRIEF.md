# Svc 最终整合候选验收（Owner 2026-11-09 差量2：源码合并验收，生产唤醒恢复另验）

仓库：/tmp/svc-integration/repo（origin/main HEAD 即候选）。本地 PG localhost:5432 可用（仅本仓测试库）。零生产访问；不执行真实发布/部署。

## 任务

以当前 main 为最终整合候选，出验收回执 SVC-INTEGRATION-RECEIPT.md（repo 根）+ 单 commit + push 分支 test/integration-receipt-20261109 + PR(base main) 不合并：

1. **三语义保留核**（逐项给 main 文件/行坐标）：
   - #128 发布归因：release.sh 的 osUser/actor(authRef/attemptId) 字段与 unattributed 警告、非法 UUID 拒绝
   - #71 唤醒隔离：outbox.rs wake 独立批次不被 Forum/kick 挤占；release.sh fail-closed 历史收据保护
   - #132 outbox 通道隔离：forum_sync.rs 的 Forum 可选性与通道隔离语义
2. **历史测试复用复跑**：scripts/test_release.sh（113 PASS 基线）+ 上轮 outbox 测试入口（shell 24/24 + PG 215/215 或现行等价集）——记录 rc 与计数，与基线对比（只允许增不减；新失败=FAIL 并如实记录）
3. **补实际受影响范围**：仅当 1/2 揭示三者交互处（release.sh 归因包裹隔离流程、outbox 批次与通道隔离交叉）缺覆盖时，补最小测试；否则明确写"无需新增"
4. **结论行**：SOURCE_INTEGRATION_ACCEPTED=YES|NO + 生产唤醒恢复=SEPARATE（deploy 链滚动，不在本验收）

## 工具指令
- shell git/gh；禁 MCP github；push 失败（代理 7890）→ bundle 交接不重试
