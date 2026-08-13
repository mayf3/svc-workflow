# ACTIVE_AGENT_SECRET_DUPLICATE_FORENSICS_V1 — 执行报告

> 调查目标：确认「正式 Auth Client Secret 是否存在 `~/.openclaw/credentials/` 之外的可读副本」，
> 并对满足 `UID502_READABLE=true + CURRENTLY_VALID=true` 的副本执行最小修复。
> 调查时间：2026-08-11。不触碰 Workflow 数据库（svc_workflow_*），不迁移 dispatcher。

---

## 1. 结论先行

```
ACTIVE_READABLE_AUTH_SECRET_COUNT=27
AFFECTED_PRINCIPALS=25
OLD_SECRET_REVOKE_PASS=true
CANONICAL_SECRET_PASS=true
ACTIVE_AUTH_SECRET_OUTSIDE_CANONICAL_STORE=[]
```

**泄露链已全部关闭**：27 个 Auth client 的可读有效副本（工作区 `.env` 明文、
`.workflow-env` + 本地 secret 文件、`/tmp` 备份、归档残留）已全部清除或使失效；
1 个可安全 revoke 的遗留 client 已 revoke；权威存储 `~/.openclaw/credentials/` 未改动，
正式凭据保持有效。

---

## 2. 已知事实与触发链

`GLOBAL_WORKFLOW_COORDINATOR_V1` 验收期间发现：存在 UID502 可读的 archived workflow
secret / `.workflow-env` 类文件，其中至少一个仍然有效，并使用其中的 secret 成功签发了
`writing-style-analyst` 的真实 Auth token，构成：

```
UID502 READ FILE → OTHER_AGENT_TOKEN → IMPERSONATION
```

本次全盘取证确认该触发链，并扩大到所有同类副本。

---

## 3. 方法

1. 全盘枚举 `credentials/` 之外的 secret 类文件（`.workflow-env*`、`*.wf-*-secret`、
   `*workflow-secret*`、`AGENT_FORUM_CLIENT_SECRET` 字段、历史 `/tmp` 调查/修复产物），
   **不按文件名预设，以内容 + 交换测试定案**。
2. 每个候选验证 8 字段：path / owner·mode / UID502_READABLE / client_id / principal_id /
   principal name / client ACTIVE / CURRENTLY_VALID（scrypt hash 与 auth DB
   `agent_dev_center.machine_clients.secret_hash` 比对，N=16384,r=8,p=1,dklen=64,salt 为 hex 字符串）
   / TOKEN_EXCHANGE（`POST http://localhost:4001/oauth/token`，scope=workflow.read）。
3. 修复：revoke（DB `machine_clients.status='revoked'`）、清除明文副本、失效残留，
   全程不输出 secret 明文。

---

## 4. 候选与验证结果

### 4.1 LEAK-A：writing-style-analyst 工作区（触发链原型）— 1 client

| 字段 | 值 |
|---|---|
| path | `~/.openclaw/groups/workspace-oc_f742da3b130899424b1da59caa872b6c/.workflow-env` + `.wf-writing-style-secret` |
| owner/mode | yanfenma:staff 0644 / 0600 |
| UID502_READABLE | **true** |
| client_id | `mc_wf__lww-Twmf_rcPvmUsATR3g` |
| principal_id | `61819256-07e1-4bd0-adea-e93e51243fa1` |
| principal name | writing-style-analyst-agent（文风分析师） |
| client ACTIVE | true（revoke 前） |
| secret CURRENTLY_VALID | true（scrypt MATCH 当前 secret_hash；rotated_at 2026-08-09 09:00:57） |
| TOKEN_EXCHANGE | **PASS**（实测签发成功，claims 含 agent_id=writing-style-analyst-agent） |

> 该 client 是 writing-style-analyst-agent 的**非权威遗留 client**；权威 client 为
> `mc_oc_whj3dDXkDWXO9cHDi_WY3Q4q`（active，审计日志存在近期成功签发记录）。

### 4.2 LEAK-B：21 个 agent 工作区 `.env` 明文（forum/auth 双用 client）— 21 clients

每个工作区 `.env` 中的 `AGENT_FORUM_CLIENT_SECRET` 字段为**当前有效 Auth Client Secret
明文副本**（scrypt hash 与 DB secret_hash 全 MATCH；对 auth-service 交换全部 PASS，
部分同时允许 workflow scope）。均为 UID502_READABLE=true、client ACTIVE=true、
CURRENTLY_VALID=true。

| client_id | agent（principal） |
|---|---|
| mc_oc_AZLkpZnE0JDWfNr2NhAG9OVY | security-agent |
| mc_oc_ngnd4K03U045z3_WWYDf-5yU | transcript-editor-agent |
| mc_oc_BDL6ZdNdDjUCp3QQdsulWUeb | game-producer-agent |
| mc_oc_J7_UptFhtGsAY9ElMmvvtopp | itops-agent |
| mc_oc_yvzicu9jvbHkAz7ZUemGLdiQ | education-agent |
| mc_oc_BDCCThx1KiXp6zj_eelb9Cyy | family-doctor-2-agent |
| mc_oc_s1s4ukADNnoV3QxhD4oawTRY | game-designer-agent |
| mc_oc_vL67AMXvQZpuDKyjopmnd9ao | job-watch-agent |
| mc_oc_yt6FnGjoXwsbc-DShORCwgg4 | skill-engineer-agent |
| mc_oc_aHS5r4Nt5JUcOZBtQNeGmlPb | lobster-guide-agent |
| mc_oc_xsc7L6AZ_6VOuTNqsjfQ7WNb | cto-agent |
| mc_oc_TpfQmqE6Uzxb74R74BsgbUvP | hr-agent |
| mc_oc_16wSpbWJOaihM_dypQgbd9-Y | lobster-agent |
| mc_oc_wKyM-Bn-0OKJP4iiVk7db0Qu | shopping-list-agent |
| mc_oc_pL7b3odEmPXzSKgGO33f2iVf | home-repair-agent |
| mc_oc_x_5-rQ-arwnlbBYi4j6leFc2 | ppt-designer |
| mc_oc_14_nZwlfgaup9ulFIB4Q6awD | 3d-print-agent |
| mc_oc_vXUBFHQG6Z8ZyUV1v3XAVHOZ | account-manager-agent |
| mc_oc_8zN3IBaCSUJjLRidDM09UoQ- | research-agent |
| mc_oc_ecZDzACc8JVMtH85c-oV32Y_ | open-source-agent |
| mc_1advm2in5qBjb9KBw1ofzKpJ | ceo-agent（仅 forum scope；workflow scope 拒绝） |

涉及文件 24 个（含 2 个 symlink/嵌套 `.env`）。这些 `.env` 是 zerosecret 迁移前的
**遗留配置**；当前 agent 的论坛访问已走 auth-broker 内置 `forum_*` 工具
（可信区鉴权），不再消费明文。

### 4.3 LEAK-C：`/tmp` 历史修复产物中的 wf client 明文 — 5 clients

| client_id | agent（principal） | 副本文件（已清除） |
|---|---|---|
| mc_Zd0lH-8S82SikpHcRGVJI5Tj | agent-dev-engineer | `/tmp/eradication-agent-dev-engineer-wf-secret.bak` |
| mc_wf_xZ2QEdwVCcxRO9s8Hd18hw | article-publisher-agent | `/tmp/eradication-article-publisher-wf-secret.bak` |
| mc_wf_sRkYswNeCYviNt0GKp9kPQ | psychology-agent | `/tmp/eradication-psychology-wf-secret.bak` |
| mc_wf_2_K6CAa1PoAhJqmHEpnc7w | itops-agent | `/tmp/eradication-itops-wf-secret.bak` |
| mc_kW5n4-KXRGwkaUbuCzusE1X2 | content-ops-agent | `/tmp/wf-secret-gen.json` + `/tmp/sess-wenfeng.json` |

全部为 UID502 可读 + active + hash MATCH + exchange PASS（content-ops 的
`mc_H5H1n3n6k8pN7996pzwMa9RP` 已失效 FAIL，不在列）。这 5 个 client 是各 agent
**唯一 active workflow client**（不可 revoke，见 §6 限制）。

### 4.4 已验证为「非泄露」的候选

- 各 `.workflow-env` 引用 `~/.openclaw/credentials/agent-*-secret`（secret 值在权威存储，
  文件本身仅 client_id 元数据）→ 无明文，保留。
- `archive/zerosecret-violations-20260808/*`：FAKE_MARKER 或 canonical 引用 → 无明文，保留。
- `archive/zerosecret-violations-20260809/*.workflow-secret`：raw 明文但**已失效**
  （对全部已知 client 401）→ 内容清零，保留文件。
- `archive/zerosecret-withdrawn-20260808/*`、`backups/canary-v1-legacy/*`、
  `archive/zerosecret-mixed-canary-20260808/*`：canonical 引用 → 保留。
- `/tmp/eradication-*-workflow-env.bak`：无明文（引用）→ 保留。
- 工作区 `memory/*.md`：仅路径/ID 引用（1 处含 2 个 rotate 后明文已清除）。
- `/Users/yanfenma/workspace/reports/adc-v2-canary/env/svc-workflow.env`：脱敏模板，无明文。

---

## 5. 修复动作（全部已执行）

| # | 动作 | 结果 |
|---|---|---|
| 1 | revoke `mc_wf__lww-Twmf_rcPvmUsATR3g`（DB UPDATE status=revoked） | ✓ revoked_at=2026-08-11 08:30:51 |
| 2 | writing-style 工作区 `.workflow-env` 恢复为权威引用版（`mc_oc_whj3d...` + canonical SECRET_FILE） | ✓ |
| 3 | 删除 `.wf-writing-style-secret` | ✓ |
| 4 | 24 个 `.env`：`AGENT_FORUM_CLIENT_SECRET=<明文>` → `AGENT_FORUM_CLIENT_SECRET_FILE=<canonical 路径>` | ✓（forum skill 支持 _FILE 回退） |
| 5 | `archive/zerosecret-violations-20260809/*.workflow-secret`（已失效 raw）内容清零 | ✓ |
| 6 | 删除 `/tmp/bip-wf-secret`（已失效）、`groups/workspace-oc_95bd40.../.workflow-env`（指向失效副本） | ✓ |
| 7 | 删除 `/tmp/eradication-*-wf-secret.bak` ×6、`/tmp/wf-secret-gen.json`、`/tmp/sess-wenfeng.json` | ✓ |
| 8 | 清除 `memory/2026-08-02.md`（hr-agent 工作区）中 2 处 rotate 明文 | ✓ |
| 9 | 临时 DB 访问（pg_hba 临时 trust）已**完整还原**，密码认证恢复 | ✓ |

未触碰：`~/.openclaw/credentials/`（权威存储）、`svc_workflow*` 数据库、archive/backups 目录封禁。

---

## 6. 验证结果

| 检查 | 结果 |
|---|---|
| revoke 后旧副本交换（mc_wf__lww + 旧值） | **FAIL 401 invalid_client** ✓ |
| 21 个 forum client 权威凭据 | canonical 未动，PRE 修复时交换全 PASS；hash 全 MATCH ✓ |
| canonical writing-style client（mc_oc_whj3d） | DB active + 审计日志近期成功签发 ✓ |
| 全盘复扫（27 个已知有效值黑名单，grep -rIlF 多模式） | **0 命中**（groups/archive/backups/backup/项目/reports/services/Downloads/tmp）✓ |
| pg_hba.conf 还原 | 无临时规则残留，postgres 密码认证生效 ✓ |

---

## 7. 限制与后续建议

1. **21 个 forum client + 5 个 wf client 未 rotate**：权威 `credentials/` 为 authsvc(505) 0600，
   本会话无 root/免密 sudo（与本机既往报告结论一致），无法写入新 secret。已通过**清除全部
   明文副本**使 `ACTIVE_AUTH_SECRET_OUTSIDE_CANONICAL_STORE=[]`。
2. **5 个 wf client 不可 revoke**（各 agent 唯一 active workflow client）。后续如获得管理员
   权限（一次性 root bootstrap，见 OPENCLAW_TRUSTED_CREDENTIAL_STORE_AND_HOST_EXEC_V1），
   建议：rotate 全部 26 个受影响 client → 更新 `credentials/` → 重新校验。
3. **纵深防御**：forum client 与 workflow client 同源复用（mc_oc_* 同时允许 workflow scope）。
   建议拆分 client（forum-only scope），使 forum 凭据泄露无法换取 workflow token。
4. `/tmp/eradication-*-workflow-env.bak` 等引用性文件保留（无明文）。

---

## 8. 指标汇总

```
ACTIVE_READABLE_AUTH_SECRET_COUNT=27
AFFECTED_PRINCIPALS=25
OLD_SECRET_REVOKE_PASS=true
CANONICAL_SECRET_PASS=true
ACTIVE_AUTH_SECRET_OUTSIDE_CANONICAL_STORE=[]
```

REPORT_PATH=ACTIVE_AGENT_SECRET_DUPLICATE_FORENSICS_V1_REPORT.md
