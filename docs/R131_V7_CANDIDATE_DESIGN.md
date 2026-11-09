# R131 `single_platform_dist_v1` v7 候选设计稿（隔离准备）

> 仅为候选设计与测试夹具，不修改任何 PUBLISHED Definition、不创建实例、不部署。
> 基线：v6=`0175f649-d7e2-47b8-9f5a-31f25a9febd9`。

## 目标语义

`publishing` 不再是无成功出口的死节点。成功发布必须进入 `verify`，由独立核验确认：

- 适用平台授权已存在且绑定文章/平台；
- 平台真实 `url` 与 `postId` 均为非空字符串；
- 内容版本与本次操作版本一致；
- 通过平台真实读回确认，而非仅凭本地非空字段或 HTTP 200；
- 核验完成后才进入合法终态 `done`。

`non-empty string != done`：缺少读回、版本不匹配或只有字符串字段时不得结束成功。

`done` 专指成功完成，仅允许 `verify → done`。失败停留在 `publish_failed`，
只有 `notified` 为真且失败原因与失败 receipt 均可读回，才按指定的
`failed_terminate` 路径进入显式失败终态 `failed_terminal`；该终态不表示发布成功，
不得进入 `done`。缺少任一失败政策条件时继续停留在 `publish_failed`。

## v7 图草案（节点/边级）

### 节点

| key | type | 语义 |
|---|---|---|
| `entry` | TASK | 接收文章版本与平台目标 |
| `publishing` | TASK | 执行平台发布/幂等提交 |
| `waiting_authorization` | TASK | 缺少适用授权时等待；授权补齐后可 RETURN 回 publishing |
| `publish_failed` | TASK | 记录失败原因并按政策通知；政策条件齐备后才可走失败终止路径 |
| `verify` | TASK | 独立平台 URL/postId、内容版本、真实读回核验 |
| `done` | TERMINAL | 仅在 verify 通过后成功完成 |
| `failed_terminal` | TERMINAL | 按失败通知与 receipt 政策终止；不表示成功 |

### 转移

| key | source → target | effect | 条件 |
|---|---|---|---|
| `entry_publish` | `entry → publishing` | ADVANCE | 输入版本有效 |
| `publish_need_auth` | `publishing → waiting_authorization` | ADVANCE | 缺适用授权 |
| `auth_return_publish` | `waiting_authorization → publishing` | RETURN | 可信授权已到位 |
| `publish_success_verify` | `publishing → verify` | ADVANCE | 平台返回可核验发布结果；不得直接 done |
| `publish_failure` | `publishing → publish_failed` | ADVANCE | 明确失败且按既定策略记录 |
| `verify_done` | `verify → done` | ADVANCE | 真实 URL/postId、版本、平台读回全部通过 |
| `verify_retry` | `verify → publishing` | RETURN | 读回缺失/版本不一致且可重试 |
| `failed_terminate` | `publish_failed → failed_terminal` | TERMINATE | notified 为真、失败原因和失败 receipt 均可读回 |

`publishing` 的成功出口仅为 `verify`；`verify` 是独立核验边界，禁止把非空字符串或 accepted 消息当作完成。所有真实创建/发布须由 Definition Owner 在隔离环境审阅后另行执行。

## 本轮 B1/B2 最小闭合范围

基线 `0312c45f5ae0bc8d96b0ea1d9950b25a7a14706b`；按本次用户任务授权，
仅修候选设计和纯内存回归夹具。路由为 `AMEND / BRIEF / DURABLE`、
`AUTHORITY_AUTHORING`；候选不具产品实现或生产操作权限。
目标是独立验证 verify 拒绝条件、可观察的重启读回恢复，以及成功/失败终态一致性；
以突变自证、指定回归全绿（用例数大于 5）、独立复核和单 commit 交付为停止条件。
若涉及真实 Definition、实例、部署或生产访问，超出本轮范围，须重新 PREFLIGHT。
