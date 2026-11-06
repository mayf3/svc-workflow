# 合同式交办：#129 Tier-2 conformance 等价隔离环境（Owner 裁定允许，2026-11-06）

## 背景
Docker 引擎本机不可用（EOF）。Owner 裁定："允许使用等价隔离 PostgreSQL 环境执行 Tier-2 conformance，不使用生产数据库"。本机已有 homebrew postgresql@16 工具链（psql/pg_ctl），服务未启动。

## 任务
1. **起一次性本地 Postgres**：initdb 到 /private/tmp/h06-tier2-pg/（独立数据目录、非默认端口如 55432、unix socket 限该目录）——零生产 DB 接触、零系统服务注册
2. 跑 `scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh`（同一冻结 fixture：workflow_trusted_fleet_successor_plan_v2.json SHA 0a05ed2d…；如脚本硬编码 Docker，允许最小适配为指向本地 socket（diff 留痕），不得改变测试语义/断言）
3. rc0 + 全部断言输出存档；失败如实 STOP

## 交付
RECEIPT-H06-TIER2.md（pg 启动参数、脚本 diff（如有）、全部输出原文、rc）+ 提交 + PR 引 #129/#1360 更新（gh 超时提交即停）

## 红线
零生产 DB/服务写入；测试完拆卸临时实例（pg_ctl stop + 数据目录留存作证据）；失败即停写 STOPPED-H06-TIER2.md
