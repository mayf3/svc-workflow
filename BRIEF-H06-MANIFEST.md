# 合同式交办：H06 迁移身份映射共享 manifest 提取（#1360，2026-11-06）

## 背景（审计源码确认，本仓 main=59daec04）
同一份 86 组迁移身份事实（344 字符串）字节相同地复制在两处：
- `src/bin/trusted_fleet_principal_cutover_v1.rs:13-22`（PAIRS 常量）
- `src/store/postgres/admin_recovery_repository/event_replay.rs:17-536,1157-1198`

## 任务（重构，零语义变化）
1. 提取单一受版本控制、内容哈希绑定的共享模块（建议 `src/store/postgres/admin_recovery_repository/migration_manifest.rs` 或仓内惯例位置）：常量 + 绑定该常量字节的内容哈希断言（编译期或测试期）
2. 两处只读复用该模块；**不删除历史映射、不改动态查询、不建通用迁移平台**（issue 明令）
3. 自校验：提取前先把两处现行 86 组/344 字符串做字节级对比（应全同），提取后模块常量与两处原字节全同（测试断言）

## 门（全绿才算交付候选）
- `cargo build` rc0；`cargo test` 相关套件（event_replay / admin_recovery / cutover 相关）全 rc0
- 回放结果不变证明：event_replay 既有测试全绿 + 新增一个"manifest 与历史内联字节一致"的测试
- 如测试需 Postgres：仅用仓内既有本地测试库惯例，零真实生产 DB 写入

## 交付定义
RECEIPT-H06-MANIFEST.md：base SHA、diff stat、前后字节一致性证明、全部门输出原文、UNKNOWN 如实；push origin ac-h06-migration-manifest + gh 建 PR 引 #1360（gh 超时则提交即停，推送监管补做）

## 红线
零生产行为变化；零生产 DB/服务写入；不动 core 仓与窗口工具；失败即停写 STOPPED-H06-MANIFEST.md；apply_patch 锚点完整行
