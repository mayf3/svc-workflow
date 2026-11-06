# 合同式交办：H06 R2 两梯队门（2026-11-06，接 STOPPED-H06-MANIFEST）

## 环境已由监管修复
1. v2 fixture 已回位：/Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json（SHA256 0a05ed2d…f3606 == 59daec04 blob，逐字节）
2. Docker daemon：仍 EOF（backend 进程在、引擎未就绪，根因 UNKNOWN）——故本轮改两梯队门

## 任务（同 R1：提取共享 manifest，零语义变化）
R1 简报的任务与禁令（不删历史映射/不改动态查询/不建通用平台）全部沿用；提取前 86 组/344 字符串字节对比断言保留。

## 两梯队门
- **Tier-1（本轮必须全绿）**：cargo build rc0；event_replay / admin_recovery 相关 cargo test rc0（不需 Docker 的全部测试）；新增"manifest 常量与两处历史内联字节一致"测试 rc0
- **Tier-2（PENDING-DOCKER，如实记录不冒充）**：scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh 全跑——本轮不执行，RECEIPT 记录精确命令与 PENDING 状态；daemon 恢复后由监管补跑，补跑 rc0 前该交付不合并

## 交付定义
RECEIPT-H06-MANIFEST-R2.md：base SHA、diff stat、Tier-1 全门输出原文、Tier-2 PENDING 声明（命令+依赖+UNKNOWN）；push origin ac-h06-migration-manifest + gh 建 PR 引 #1360 标注 "Tier-2 conformance pending Docker"（gh 超时则提交即停）

## 红线
零生产行为变化；零生产 DB/服务写入；不动 core 仓与窗口工具；失败即停写 STOPPED-H06-MANIFEST-R2.md
