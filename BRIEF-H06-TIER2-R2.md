# 合同式交办：#129 Tier-2 R2（2026-11-06，接 STOPPED-H06-TIER2 监管裁定）

## R1 停点裁定：机械环境问题，修复已由监管验证
- R1 失败原因：干净环境缺 LC_ALL/LANG → postmaster 多线程启动失败
- **监管已实测修复**：`export LC_ALL=C LANG=C` 后同一数据目录 `/private/tmp/h06-tier2-pg` pg_ctl start 成功、socket 响应正常（已停回，证据留 /private/tmp/h06-tier2-pg/start.log）
- R2 环境：所有 pg_ctl/psql/脚本调用前 `export LC_ALL=C LANG=C`

## 任务（同 R1）
1. 用现有数据目录启动一次性 PG（端口 55432、socket 限 /private/tmp/h06-tier2-pg；如需重建用户按 initdb 实际 superuser 走）
2. 跑 `scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh`（冻结 fixture 0a05ed2d…；R1 的脚本适配 patch 复用，语义/断言零变化）
3. rc0 + 全部断言输出存档；失败如实 STOP

## 交付
RECEIPT-H06-TIER2-R2.md + 提交 push + PR #129 更新（MERGE_READY 翻正或如实 NO）+ #1360 状态（gh 超时提交即停）

## 红线
零生产 DB/服务写入；测试完拆卸（pg_ctl stop，数据目录留证）；失败即停写 STOPPED-H06-TIER2-R2.md
