# #1351 P3 Svc 配套合同 authoring record

本记录是任务/路由/定位报告，不是 accepted Product Authority、正式独审、接受事务或运行 receipt。
本次只提交 proposed/none 配套提案，Core P3 的 accepted-companion 门仍未关闭。

## 目标、授权与固定坐标

用户明确要求完整读取根 Brief、治理先行、docs-first、不改 Rust、零生产写入、commit/push/PR(base main)、不合并。
[Brief 原字节副本](BRIEF-SVC-1351-P3-COMPANION.md) 保留具体 scope/effects/Done When；root 文件保留。
源文件及外部输入精确 hashes/坐标见 [provenance.json](provenance.json)。

```text
REPOSITORY = mayf3/svc-workflow
WORKTREE = /Users/yanfenma/workspace/project/worktrees/svc-h1351-p3
BRANCH = docs/ghost-holdings-svc-companion
AUTHORING_BASE = b5096782f666ad22aa3508fdbf9562278a977c0f
INTEGRATION_BASE = main
BASE_HEAD = b5096782f666ad22aa3508fdbf9562278a977c0f
SPEC_GOVERNANCE_MODE = AUTHOR
AUTHORITY_ACTION = NEW (companion proposal, not an owning-authority replacement)
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = CONTROLLED
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO (this new companion)
GOVERNANCE_ADOPTION_IN_BASE = accepted (local lock and main bytes verified)
IMPLEMENTATION_ALLOWED = NO
PRODUCTION_APPLY_AUTHORITY = none
DONE_WHEN = contract + report + checks + commit/push + PR(main); do not merge
EXPANSION_TRIGGER = product code/behavior tests/production/new privilege or accepted-authority bypass
```

首次 Git 状态仅 root `BRIEF-SVC-1351-P3-COMPANION.md` 和 `worker.log` 未跟踪；均不改、不暂存。
隔离分支起点等于 remote main。最终 candidate/pushed head/PR、最终 main impact 与非作者审查坐标
放在持久 PR description，避免在含自身 SHA 的 commit 中制造不可能的自引用。

已完整读 Brief、AGENTS.md、.agents/README.md、.agents/local/README.md、skill router 和唯一选定 AUTHOR mode。
再读根 PRODUCT-BOUNDARY（已 superseded）、有效 Product V13/Architecture V0_4_7 的直接相关 delta/
身份/角色/权限/生命周期/兼容条款、provisioning FROZEN contract、Coordinator accepted contract、CIR V2 admission、
active-agent/global-reader 与 self-projection 范围。未改 vendored grammar、accepted authorities 或生命周期索引。

## Core P3 原文与条款映射

固定外部输入为 Core #674 head `527d641518bdde61267cde56000bfb8299780667`，并非 PR body 中较旧的 reviewed head。
经 GitHub connector 获取该 head 的 Spec/handler；来源封存见 provenance。Core proposal 仍 proposed/none、draft/open/unmerged。

| Core P3 / Brief 要求 | Svc 决策与合同 | 验收定义 / 当前结论 |
|---|---|---|
| 全量 holdings：当前 assignee + 全 enabled domain/global role；身份/错误/时间边界 | CTR-SGHR-001/004：精确生命周期、全 subject 并集、异常不能 inner-join 掉、同事务全量观察、五秒上限、完整读权 gate | ACC-001/002/004；当前普通 global-list 每页快照不是完整 holdings 证明 |
| 全部角色/owner 写点与同效恢复 | CTR-SGHR-002：四 provisioning 点外，成员、reconcile、Principal/Domain restore、离线/import/recovery/未来同效入口 | ACC-003/004；实名源码表如下，未来每个新增入口须入覆盖矩阵 |
| 活性语义及判定权威 | CTR-SGHR-003/007：Auth 精确关系，Core /v1/directory/agents/{agentId} existence/enabled；每命令新鲜、五秒、八在途、无缓存/retry、stale 失效 | ACC-003/004；复用 CIR 语义，role 扩围未获现有接受 |
| 线性化与观察时点分离 | CTR-SGHR-004/005：Svc RR/行锁/receipt约束本地，Core目录是时点观察；不跨存储共同锁；V1 retirementSafety固定UNPROVEN | ACC-005/006；真实删除边界未证明，删除保持拒绝，不冒称退役可用性已通过 |
| holdings空读→新assignment/role/registry变更→离册的两序列 | CTR-SGHR-006：W先与R先，含复读后窗口、不同域/role、恢复和registry同效mutation；正式callable+barrier | ACC-005/006；未来定义，未执行 |
| MachineReadable Contracts、幽灵owner拒绝、wire/降级、Core C1接口 | §9 YAML索引、CTR-001/004/007，闭合拟议payload与错误表，沿用admission码；旧本地错误保留；UNKNOWN不假零 | ACC-001..008；8/8双向定义覆盖，0项产品验收执行 |
| 双仓接受分离及前置 | CTR-SGHR-008、§3/13：owning whole successors、完整读权、独审、真实接受权限、明确none→contracts、各main/base进入 | ACC-008；均非本次已完成的正式接受 |

## Source observations 与实施影响面

以下是 `b5096782f666ad22aa3508fdbf9562278a977c0f` 的源码事实，供未来 implementation PREFLIGHT；
源码不是产品权威。行号仅定位该固定 revision，未来实施须 fresh readback。

| 面 | 当前实名链 / 行号 | 将来实施审查点 |
|---|---|---|
| 全局实例读 | `src/http/mod.rs:144` → `handlers/instances.rs::global_list:194` → `application/workflow_instance/query_service.rs::list_global_instances:180` → `store/postgres/workflow_instance_repository/query_global_instances.rs::list_global_instances:136` | 显式assigneePrincipalId+lifecycle=active+status=active；default20/max100、逐页RR不能跨页证明完整；新holdings读不能改变旧DTO/权限 |
| 责任事实 | 同query文件182–193；`workflow_instances.current_node_visit_id`→`workflow_node_visits.assignee_principal_id` | disabled Domain/Principal、WAITING、未来时刻不能隐藏；缺关联/不一致不能被INNER JOIN当零；successor enrichment不能覆盖原UUID |
| Provisioning role | `src/http/handlers/provisioning/role_bindings.rs` → `application/provisioning/mod.rs` → `store/postgres/provisioning_repository/mod.rs::upsert_role_binding:342` | 新增/enable/idempotent新key都校验target，单owner原子性 |
| Global role | `src/http/handlers/provisioning/global_role_bindings.rs` → `...::upsert_global_role_binding:449` | 全supported role keys及同效写点，不只owner |
| Owner replace | admin/coordinator → `application/provisioning/mod.rs::replace_owner:689` → `...::replace_domain_owner:520` | 校验接收者，失败旧owner不动；source disabled不阻断 |
| 新Domain建owner | `application/provisioning/mod.rs::create_domain:242` → `...::establish_domain_owner:288` | create与owner同事务，不能绕过target活性 |
| Domain member | `application/domain_membership/mod.rs::add_member:290,421` → `store/postgres/domain_role_repository.rs::insert_member_binding:390` | DOMAIN_MEMBER新建/恢复同样覆盖 |
| Reconcile | `application/coordinator_control_plane.rs::reconcile_apply:631,894` → `domain_role_repository.rs::insert_role_binding:626` | exact preimage/CAS、source不要求enabled、目标active、原子迁移和冻结错误 |
| Principal restore | `provisioning_repository/mod.rs::upsert_principal:134` | false→true可使旧enabled roles重新可用，须检查受影响Agent；self-projection `domain_role_repository.rs::upsert_principal_projection:44` 目前拒绝disabled恢复，不应误说可恢复 |
| 离线同效写点 | `src/bin/trusted_fleet_principal_cutover_v1.rs:944,1078–1080` | 直接enabled role upsert与新assignment须列边界；旧有限授权不被本提案扩大 |
| Assignment admission与同效责任入口 | `src/store/postgres/admission_gate.rs`、`src/auth/admission.rs`；create/transition/combined/override/import有目录admission；rebuild从历史重建projection；有限offline migration沿其冻结canary/身份校验 | 后两类不能称已接入同一新鲜Core目录client；均须纳入未来ordering/覆盖审查。REUSE CIR不增加普通reassign API |
| Assignment事实/投影定位 | `workflow_instance_repository/create_transaction/facts.rs:310`、`transition_helpers.rs:220,253`、`combined_helpers.rs:120`；`admin_recovery_repository/override_transaction.rs:456,502`、`rebuild_transaction.rs:232`；`legacy_import_repository/transaction.rs:119` | rebuild为既有事实投影恢复，不据此宣称新建Visit或已有目录保护；未来分别核验责任恢复/迁移的合法边界，保留既有Visit/Event/CAS规则 |
| DTO/错误/事务 | `src/http/dto.rs`、`src/http/error.rs:637`、`src/domain/provisioning/mod.rs:229–244`、`src/auth/admission.rs:206–240`、provisioning receipt模块 | 新读payload、新role admission映射、原错误和历史replay、commit unknown须正式接受后实现 |

Core `workflow-admission.js` 同固定 head 的事实：前缀 `/v1/directory/agents`，真正精确查询是
`GET /v1/directory/agents/{agentId}`，audience=`agent-directory`、scope=`agent.directory.read`；
handler从`definition.listAgents()`同步取观察，missing 200 `exists:false`，disabled 200 `enabled:false`。
它未暴露 registry revision/lease，不能用 enabled=true 作为后续离册竞态已解决的证据。

拟议 holdings `GET /internal/v1/admin/holdings` **不存在于当前注册表**；是有接受前置的payload/route提案。
其候选actor限制复用provisioning的workflow.admin+allowlist+direct-Agent，而不是把Core现有directory身份隐式提升。
若Product/Architecture不能授权全量读，此接口不得启用；Core按旧版本/无权限不可验证处理。

## Authority disposition 与正式接受前置

| 前置 | 已知事实 / 尚需持久证据 | 本次状态 |
|---|---|---|
| Governance adoption | base `.agents/governance.lock.json` accepted，与main字节相同；verify require-accepted | 验证结果见validation |
| Owning provisioning successor | FROZEN `IDENTITY_PROVISIONING_API_V0` §5/6 exists+enabled；远端拒绝改变通过集合 | REQUIRED，未起草/接受其whole successor或回链；旧文件不动 |
| Owning coordinator successor | accepted CP V1 DEC-005/007、CTR-003冻结exact UUID、source/target不对称、错误与无新增上游依赖 | REQUIRED，不能用本NEW取代whole successor；未做接受 |
| Membership/其他owning约束 | FROZEN self-projection/member规则及各offline迁移冻结scope | 受影响语义需纳入完整owning审查；不扩迁移范围 |
| 完整读权限 | Product V13§6/9、Arch0.4.7§5.2禁止child扩跨域权；READER仅global实例，不等于roles/principals | REQUIRED；完整reader/route/caller权限需上级合法继任或可核实既有组合 |
| 最终离册顺序 | Core P1真实删除callable与全部registry写点、所有W、序列化/复核/失效/UNKNOWN恢复、两顺序证明 | UNKNOWN；V1始终UNPROVEN，删除拒绝；不能把拒绝测试说成正常退役已可用 |
| 非作者语义独审 | fixed candidate/base/hash、完整受影响合同、reviewer与author独立、final-head recheck | 本次提案review另记PR；不等于正式接受 |
| 接受事务 | mayf3或明确授权Svc maintainer；权限来源、reviewed head、文档hash、明确none→contracts、final accepted head | NOT_EXECUTED；作者不自接受，Core§3C不自动代签Svc |
| Paired/main/base进入 | 双仓真实ID/Contracts/hash/review与acceptance/main SHAs；双方实施base ancestry证明 | NOT_EXECUTED；Core与Svc各自接受互不替代 |
| 实施/生产 | 后续独立任务；accepted/contracts已在base；源码测试、部署、runtime/business分开 | 本次全部禁止/未执行，生产写入0 |

权威判断：本NEW proposal未激活任何对旧accepted meaning的收窄，未假造supersedes/backlink。
若要将其变为正式可执行配套，必须按表完成owning whole successors；若有人试图跳过则RE_PREFLIGHT，
不能因为Core期望该功能就静默rewrite Svc authority。
`SVC_WORKFLOW_DOMAIN_CREATE_CANONICAL_CONTRACT_V1` 当前frontmatter仍proposed/none，不能拿正文定位作accepted授权。

## 验证、失败语义与停止边界

当前全部产品测试/持久集成/两顺序/部署/生产业务验收为NOT_EXECUTED。
[validation.txt](validation.txt)只记录文档schema、生命周期、contract↔acceptance、来源hash、治理完整性和diff范围检查。
非作者提案review的 exact head/hash 与最终push/PR读回放在PR，不在此记录伪造正式接受。

探索空命中不是失败：首次按惯例猜测`SPEC_TEMPLATE_V1.md`不存在后，从实际目录发现并读`SPEC_TEMPLATE.md`；
`src/auth/admission/`、`src/application/provisioning.rs`不是当前文件布局，随后`rg --files`发现
`src/auth/admission.rs`与`src/application/provisioning/`并继续。未执行“所有搜索必须非空”的错误谓词。
这些只读查找曾有rc=1/2，不是产品检查FAIL；实际必需输入均已取得，未触发STOPPED文件。
治理工具若报告真实谓词不符则停止依赖并持久记录，不能隐藏rc或借空结果伪造通过。

首轮文档检查发生真实schema错误（rc=1）：既有Architecture ID `SVC_WORKFLOW_ARCHITECTURE_V0_4_7`
是legacy ID，不符合frontmatter `governed_by`只接受`_V数字`的pattern。
将其放入显式`architecture_authority`扩展metadata并在§3继续完整约束，`governed_by`保留合法Product V13，
没有改名或降格既有Architecture。首轮后续暂存命令因validation.txt未生成而rc=128、未完成暂存；
修正后重跑文档检查及暂存。该可修复候选语法错误不是必需输入缺失，未伪装为探索空结果。

到文档交付PR即停止。无main合并、accepted改标、Rust/行为测试修改、生产API/DB调用、身份/角色grant、
外部服务启动或生产进程动作。技术文档/提案review通过均不等于Core P3已经关闭或三条业务验收通过。
