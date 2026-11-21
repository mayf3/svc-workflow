# BRIEF-SVC-1351-P3-COMPANION — 幽灵 holdings 防护 Svc 配套合同提案（AUTHOR，docs-first）

## 授权链
- Core 侧提案 `AGENT_CORE_GHOST_HOLDINGS_PROTECTION_V1`（dsh-agent-core PR #674，proposed）——其 P3 门明确要求 Svc 合法责任链出具配套 accepted Contracts
- issue mayf3/agent-control#1351 四道闸（C3 owner 活性校验 = Svc 侧）+ 收敛评论三处置区分
- 常设授权 §3C（责任活性/故障隔离类）受托链
- 前次 PREFLIGHT 定位成果复用（svc main `b5096782`）：HTTP provisioning role-bindings → application provisioning → `src/store/postgres/provisioning_repository/mod.rs::upsert_role_binding`(:342)、`replace_domain_owner`(:520)、`establish_domain_owner`(:288)、global role upsert(:449)——当前只锁本地 `principals.enabled`
- 零生产写入。docs-first。

## 任务：起草 `SVC_WORKFLOW_GHOST_HOLDINGS_ROLE_LIVENESS_V1.md`（命名按仓库惯例可调）
Core 侧 spec 的 P3 要求（逐条落实为 Svc 合同条款）：
1. **覆盖面**：holdings 全量读（活跃实例 assignee 查询）+ 全部 role/owner 写点（upsert_role_binding/replace_domain_owner/establish_domain_owner/global upsert + 任何可新建/恢复 enabled binding 或产生新 owner 的同效入口）+ 身份解析/错误/时间界限
2. **活性校验语义**：写入/启用时校验 principal 在册且 enabled——判定权威 = Core `/v1/directory/agents`（`packages/product-api/src/workflow-admission.js`）；定义缓存/失效边界与 stale 读处理（不宣称分布式事务）
3. **线性化承诺与观察时点分开**（Core spec P3 原文要求）：说明从 holdings 空读到最终离册之间，新增实例 assignment、role 写入、registry 变更如何被已有合法序列化/复核边界约束；提供两种提交顺序的反例测试要求；未证明删除不穿过新持有责任窗口前，删除保持拒绝
4. **验收合同**（MachineReadable Contracts）：两顺序反例、幽灵 owner 写入被拒、错误码/降级行为、与 Core C1 门禁的接口（holdings 查询 API 或目录读）
5. 路由：Svc 侧 NEW（或按仓库惯例）；frontmatter proposed/none 如实；**Core 接受不替代本配套，本配套不替代 Core 接受**
6. 报告：条款↔Core P3 要求映射表 + 实施影响面（哪些 Rust 模块将来实施）+ 前置清单（非作者独审/接受事务/base 进入）

## 完成定义
合同文档 + 报告 → commit → push → PR(base main) **不合并**。失败语义=探索性空结果≠失败（从源码推导实名继续），仅必需输入缺失/谓词非预期才停，写 STOPPED-SVC-1351-P3.md

## 边界
零生产写入；不改 Rust 产品代码；先读仓库 AGENTS.md/.agents 治理入口与既有 Spec 惯例
