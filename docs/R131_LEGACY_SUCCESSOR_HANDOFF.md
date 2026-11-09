# R131 v5/v6 → v7 合法承接方案（隔离准备）

> 方案文档，不执行实例迁移、取消、归档、发布或数据库写入。

## 六对唯一映射

| v7 候选实例 | 旧实例 | 平台 |
|---|---|---|
| `0e17efaa-a355-414e-9bfe-7b458d5e261a` | `9975aa98-0c5a-4a24-a312-1df8828327a4` | 掘金 |
| `0f960c7f-2542-4b9a-9f7d-fde84a7a0da3` | `407141d7-2013-4ec9-b830-bb6297308920` | CSDN |
| `a70577fb-d2e6-4f2f-8a5a-6cea6c5c62d3` | `06905ef3-2073-43ad-8b70-830257dee1ee` | 知乎 |
| `e266baff-41b4-45d7-9f26-20faf0e98363` | `ead8fc1f-a06d-4856-be8b-dc87f3c78cb2` | 掘金 |
| `a65e1cb2-89c5-415a-8126-8666dbb3ccb0` | `5f5fa1e4-4d86-4851-af84-ac1ccf7879c8` | CSDN |
| `57f81c18-3eaa-4b06-9709-1356cee75188` | `b08748ff-2e83-4e17-a7fb-b22703c37a48` | 知乎 |

## 承接规则

1. 旧实例不取消、不归档，直到新实例的正式 receipt/readback 与父验收均可见；旧实例继续保留唯一历史责任。
2. 对每一对只允许一个 successor 执行责任；以 `(oldInstanceId, platform, contentVersion)` 作为逻辑绑定。
3. 发布操作必须使用稳定 `dedupeKey` 与唯一 `operationKey`；重启/回执丢失时先平台真实读回，再决定是否重试，禁止盲发。
4. 掘金/CSDN/知乎授权只按已有 ownerAuthorization 记录使用，不扩大平台或文章范围。
5. v7 的 `verify` 是独立核验节点：须读回真实 URL/postId、内容版本、平台结果；非空字段或 accepted 消息不能作为完成。
6. 父实例 `89f19a8b-da6a-4eed-819b-730b49aa670c`（sv32）与 `2a5c8a00-8716-4bae-8ac4-a37fea07ab6f`（sv41）的验收读取 successor receipt、旧实例坐标、operation/dedupe 绑定和最终状态；父引用仍指向旧实例时必须先补可读回映射，不得伪造完成。
7. 失败按既定 `publish_failed`/通知/终止政策收口；授权缺失进入 `waiting_authorization`，不得伪报失败或成功。
8. Definition Owner（文章发布管家）审阅并批准 v7 候选后，才可在隔离域创建 DRAFT/Canary；本准备任务不改 PUBLISHED 版本、不改 jobs.json、不部署。

## 当前准备结论

v6 的 `publishing` 缺少成功出口，因此 v7 必须先完成候选图和 5 项回归，再由 Definition Owner 进行隔离审阅。六对实例仅形成唯一映射与责任规则，尚未执行迁移或发布。
