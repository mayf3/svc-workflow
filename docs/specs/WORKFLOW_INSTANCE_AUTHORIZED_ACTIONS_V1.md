---
spec_id:
accepted_at: 2026-11-02
accepted_by: Owner (mayf3) via supervisor binding; receipt = PR acceptance comment at exact head (see agent-control#1344 timeline) WORKFLOW_INSTANCE_AUTHORIZED_ACTIONS_V1
status: accepted
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
scope:
  - svc-workflow instance-detail-only caller-scoped task and governance action projection
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V13
  - SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3
  - SVC_WORKFLOW_EXECUTION_CONTROL_V2
  - SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V2
  - SVC_WORKFLOW_OWNER_OBLIGATION_WORKFLOWS_V3
  - SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1
architecture_authority: SVC_WORKFLOW_ARCHITECTURE_V0_4_7
external_authorities: []
supersedes: []
superseded_by: null
legacy_successor_of:
  - path: docs/contracts/WORKFLOW_QUERY_CONTRACT_V0_1.md
    revision: 5bf2f14ed471c7be0aac097067f0c92aa626505e
    replacement_scope: whole_document
owners:
  - mayf3
base_head: 5bf2f14ed471c7be0aac097067f0c92aa626505e
date: 2026-11-01
acceptance: PENDING_OWNER_EXACT_HEAD_ACCEPTANCE
task: https://github.com/mayf3/agent-control/issues/1344
---

# WORKFLOW_INSTANCE_AUTHORIZED_ACTIONS_V1

## 1. Goal

让已获 full instance detail 可见性的调用者，直接读到与各操作独立执行检查一致的
当前身份/状态操作集合，以及有来源的当前节点类型。清单不能把旧布尔投影、读取
权限或任务文案当成新的写权限，也不承诺一个尚未提交的 payload 必定执行成功。

```text
GOAL_OR_TARGET = #1344 D6/D11 direction b; explicit, accurate authorized_actions
CURRENT_GAP = closed detail schema + incomplete executable_for_actor predicate
SUCCESS_OUTCOME = independently reviewed docs-only authority candidate and open PR
```

## 2. Scope and non-goals

唯一新增响应面是 `GET /internal/v1/workflow-instances/{workflowInstanceId}` 的
`visibility=full` 分支。V1 的集合范围固定为
`INSTANCE_TASK_AND_GOVERNANCE_V1`：图内 transition（ADVANCE/RETURN/TERMINATE）、
请求 Assistance、普通 Owner Assistance escalate/resolve、cancel、archive。
这里“全部”指该命名范围内的全部操作实例，不能解释成全系统所有 API。

不实现、不运行产品服务、不部署、不变更角色或凭据、不操作生产实例、不 merge。
不增加新命令、Event、Scope、Grant、业务任务分类、持久化操作表或第二套权限模型。
不修订 `executable_for_actor` / `blocked_reason` 的历史含义；不把本 Spec 变成
Broker/agent 提示词或自动派发的跨仓权威。方向 a 的 bot 消息摄入不在本次范围。

## 3. Authority and dependencies

所有下列本仓引用均固定于 frontmatter 的 `base_head`，不以“最新文件”取代权威：

| 权威及实际状态 | 本 Spec 的关系 |
|---|---|
| `PRODUCT-BOUNDARY.md`（superseded），现行 `docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V13.md`（accepted） | 沿显式继承链使用 V13；不从 V1 的旧生效措辞推导权限 |
| `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_4_7.md`（accepted），§5.1/5.2/5.14、CTR-ARCH-003 | 显式模型、串行状态机、兼容面需 accepted implementation Spec；Minimal 数值 2 不能冒充模型 3 |
| `docs/contracts/WORKFLOW_QUERY_CONTRACT_V0_1.md`（CURRENT）§§1–9 | **整份 legacy authority 的拟议 whole-authority successor**；附录 A 完整保留原文，§9 的明确差异优先，其余义务全保留 |
| `docs/contracts/WORKFLOW_TRANSITION_CONTRACT_V0_1.md`（IMPLEMENTATION_CONTRACT），§§2/4–8；VAI V3、ACB V2 | 复用流转身份、图规则、Submission/RETURN/ACB、模型与原子提交检查 |
| `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_2.md`（EFFECTIVE），Coordinator Control Plane V1（accepted）CTR-CP-001 | 保留 cancel/archive 生命周期；使用已明确扩展的 Owner **或** Coordinator 治理授权，不退回 Owner-only |
| `contracts/workflow-http/v1/contract.md` 的 `Workflow Assistance V1`（Current-state freeze） | assignee 请求、Owner escalate/resolve 的起源合同；无虚构的 Assistance Spec ID |
| Execution Control V2（accepted）CTR-SWEC-004/005/006/010 | open case 阻断、RETURN 计数及上限、Owner resolve 证明；system escalation 不是普通动作 |
| Governance Child Return V2、Owner Obligation Workflows V3（accepted） | linked child terminal proof、managed effect/Owner fence、真实 producer binding、Review outcome |
| Current Task Input V1（accepted）§2/3、CTR-CTI-005/006；Work Eligibility Projection V2（accepted） | CTI 原有 wire/body、其他查询面不变；时间资格和 READY dispatch 不是 transition 执行闸 |

`supersedes: []` 仅因 legacy Query 文档没有 governing `spec_id`，其名称也不是合法
新 Spec ID；不得伪造 ID、把段号塞入 supersedes 或声称无 supersession。
`legacy_successor_of` 以路径/精确 revision 标识**整份替代意图**。附录全文和
AMENDMENT map 是 whole-successor 的差异说明，不是 partial supersession。
原 legacy 文档在本 PR 只加非生效候选链接，仍为 CURRENT；正式接受生命周期必须
原子记录原文的 whole-successor backlink/status 与本 Spec 的接受状态。

```text
SPEC_GOVERNANCE_MODE = AUTHOR
AUTHORITY_ACTION = SUPERSEDE
AUTHORITY_OWNERSHIP = mayf3/svc-workflow
PRIMARY_AUTHORITY = WORKFLOW_QUERY_CONTRACT_V0_1.md (whole legacy document)
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = CONTROLLED
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO
DOCS_FIRST_REQUIRED = YES
IMPLEMENTATION_ALLOWED = NO
OPERATION_ALLOWED = NO
```

方向 b 的 Owner 裁定授权此次起草和 GitHub 交付，不接受尚未形成的文本。不得把
`implementation_authority: contracts` 解释成 proposed 分支已可实现；须先精确 head
接受及 main 合并，再由新实施任务走 REUSE。无 production apply authority。

## 4. Current State

`STATE-AA-001`：2026-11-01，本地 source-only，base `5bf2f14ed471c7be0aac097067f0c92aa626505e`。
detail 缺新字段；`executable_for_actor` 不含所有状态执行闸。依据 OBS-AA-001/002。

`STATE-AA-002`：本候选不包含实现、RED→GREEN、运行时或 agent 自主推进验收。
这些结果均为 `NOT_RUN`；独立文档审查与 Owner 接受是不同事件。

## 5. Observations

- `OBS-AA-001`：在上述 base 静态读取 `contracts/workflow-http/v1/openapi.yaml`
  的 `FullWorkflowInstanceDetail`：`additionalProperties: false`，required 为
  `instance,current_context_revision_id,current_node_visit_id,current_context,current_visit,outgoing_transitions`。
  `WorkflowInstanceDetailResponse` 外层只有 visibility/detail；内外闭集都不能偷偷加字段。
- `OBS-AA-002`：静态读取 `query_detail.rs::load_outgoing`：旧布尔只判终态、
  DRAFT/REVOKED、primary ADVANCE、effective assignee、目标 assignee availability。
  `transition_transaction.rs::execute_workflow_transition_atomically` 另外检查 cancelled、
  open Assistance、RETURN 计数及 managed guards；代码不是新权限的来源。
- `OBS-AA-003`：`query_detail.rs::build_full` 还被 worklist/current-task-input 使用；
  `assistance_transaction/write.rs::request_assistance` 的 raw assignee 等式不同于
  transition 的 effective-assignee 检查。`intents::require_effect` 还限制 opted-in case。
- `OBS-AA-004`：读取 [r1 停点回执](https://github.com/mayf3/agent-control/issues/1344#issuecomment-6552425487)，
  与以上 source observation 交叉核对。未运行 DB、命令写路径或生产调用。

上述源路径均相对 `src/store/postgres/workflow_instance_repository/`，除明确的
OpenAPI 路径外。观察日期、环境、方法、revision 如本节所列；可由 reviewer 在同一
Git revision 复读；不把静态观察标成执行证明。

## 6. Claims and assumptions

- `CLM-AA-001`（SUPPORTED）：旧布尔过滤会同时遗漏独立操作类及漏掉流转拒绝条件。
- `CLM-AA-002`（SUPPORTED）：端点专属投影可保留 shared Full DTO/CTI 的既有语义；
  不能通过直接扩展共享 serializer 达成这一点。
- 无假设授予角色、推定业务任务类别或保证未来 payload/CAS 成功。

## 7. Evidence relations

- `EVD-AA-001`：OBS-AA-001/002/004 SUPPORTS STATE-AA-001、CLM-AA-001；坐标为
  base+2026-11-01 local static source。足以证明起草缺口，不证明运行时行为。
- `EVD-AA-002`：OBS-AA-003 SUPPORTS CLM-AA-002；同坐标，证明共享与身份检查差异，
  不证明未来重构正确。provenance 为具名文件/函数及 r1 固定评论。

## 8. Decisions

- `DEC-AA-001`（proposed）：以各操作自身的 accepted 身份/状态检查派生清单；
  同一 pure/read-only 判定供读与写复用，不维护另一套 SQL/规则表。
- `DEC-AA-002`（proposed）：操作可选择资格与请求验证分离。列入表示当前 token、actor、
  instance/visit 状态允许尝试该类操作，不表示 payload 已验证、业务证据充分或命令已提交。
- `DEC-AA-003`（proposed）：仅扩展 instance detail Full，明确集合范围、空集合与节点类型。
  旧布尔继续作为旧诊断字段，不能据它反推完整清单或改写它的既有 meaning。

## 9. Contracts

### CTR-AA-001 — 范围、调用身份与全类覆盖

清单 MUST 只包含 §2 命名范围内的操作。以下覆盖表是闭集；不得把同类合法操作只取
第一条，也不得添加 Definition 未声明的 transition。重复身份 MUST 去重。

| 操作类 / `operation` | V1 disposition | accepted 判定来源及复用/补检查 |
|---|---|---|
| ADVANCE / RETURN / TERMINATE：`transition` | INCLUDED，每条合法 edge 一项 | Transition 合同、VAI V3、EC V2、ACB V2、Owner Obligation V3；见 CTR-AA-004，补齐旧投影缺失的执行检查 |
| 打开 Assistance：`request_assistance` | INCLUDED | Runtime Assistance 合同；复用 `assistance_transaction/write.rs::request_assistance` 的独立检查，不能复用 transition 的 effective-assignee 等式 |
| Owner escalate：`escalate_assistance` | INCLUDED，每个合法当前 case 一项 | 同合同及 GCR V2/Owner Obligation V3；`owner_action_on_tx` 的 Owner、case status、require_effect |
| Owner resolve：`resolve_assistance` | INCLUDED，每个合法当前 case 一项 | 同上，另含 `governance_child::validate_resolve` 状态证明，不能只判 open case |
| 图外取消：`cancel` | INCLUDED | v0.3.2 + CTR-CP-001，`cancel_workflow_instance_atomically`；独立于所有 outgoing edge |
| 归档：`archive` | INCLUDED | v0.3.2 + CTR-CP-001 + VAI V3，`archive_workflow_instance_atomically`；非重复归档且已结束 |
| Legacy revise Context / combined revise+advance / revise-and-transition | EXCLUDED_BY_V1_SCOPE | legacy 应用命令独立，当前 HTTP router 无普通 revise 路由；model3 仍按 CTR-VAI-012 拒绝，不能杜撰为 detail 动作 |
| governance-child bind / create-and-bind | EXCLUDED_BY_V1_SCOPE | GCR V2 专用 case/child 操作、目标参数和 source/child 联合检查；既有专用入口仍有效 |
| managed operation-effect execute（含 effect 执行的 resolve/escalate/create/cancel） | EXCLUDED_BY_V1_SCOPE | Owner Obligation V3 专用 slot/effect 入口；普通 resolve/escalate 若被 require_effect 拒绝 MUST 不列出，不能伪装成普通动作 |
| wake / system execution-escalation | EXCLUDED_BY_V1_SCOPE | VAI V3 / CTR-SWEC-004 的 scheduler 专用身份和直接 token，不由 full detail 授权 |
| instance create / migration / admin emergency override / rebuild / identity/Domain/Definition/Grant mutation | EXCLUDED_BY_V1_SCOPE | create 尚无当前实例动作；其余分别由专用 accepted 权威/mandate 约束，不列入普通 detail |
| 只读 readiness / owner-obligation / effect-read / case-read / history / worklist | EXCLUDED_BY_V1_SCOPE | 不是本清单的 mutation 操作；清单不决定其读取可见性 |

存在被排除的合法专用操作时，空清单仍只表示 V1 范围为空。不得以清单缺项撤销专用
入口的既有权限。新增操作类或扩大清单范围须新权威，不得隐式扩展此枚举。

### CTR-AA-002 — 一个授权快照、无副作用与错误诚实性

detail 的原有 Principal/full/restricted/masked 404 检查 MUST 先执行。新增字段 MUST
在同一个 `REPEATABLE READ` 事务中使用同一 actor、Instance、当前 Visit、Context、
workflow state version、角色、case、Event、managed facts。token Scope 与服务写门配置
取本次请求验证结果/配置快照；不得换身份、借 Owner token 或给读取 token 提权。
MUST 检查本次 credential 有 `workflow.execute` 及各动作现有 transport/token 限制；
所有 INCLUDED 路由的既有 canary write gate 关闭时不得宣称可用。

MUST 从各命令的执行前身份/状态检查提取可复用的无写入判定；只读调用不得使用
`FOR UPDATE`、执行 dummy 命令、领取 receipt、创建/resolve Assistance、推进 RETURN
计数、写 STOP/outbox、唤醒或修复数据。原 Query 拒绝审计是唯一既有查询写副作用。
真实执行仍在原锁/事务内重判；读后角色变更、状态变化或配置变化允许导致执行拒绝。

所有所需事实必须读取成功。存储失败为现有 503 `service_unavailable`，不一致为现有
500 `internal_consistency_error`，不得返回假 `[]`、UNKNOWN-as-success 或部分清单。
因不可见失败不得通过新增 reason/ID 泄露实例或 case 存在性。

### CTR-AA-003 — 精确派生关系与 payload 边界

对同一授权快照 S、credential C、actor A 和每个 INCLUDED 操作身份 O：

```text
O in authorized_actions <=> operation_specific_identity_and_state_checks(S,C,A,O) = ALLOW
```

判定 MUST 包括执行器在不依赖尚未提交 payload 时即可确定的全部拒绝条件，不能
偷换成 `outgoing_transitions.filter(executable_for_actor)`。独立操作检查出错按 CTR-AA-002
返回错误，不能当成 DENY 或遗漏。重构只能抽取/共享原判定，不得顺带改变写命令规则。

尚未提交的 Submission/理由/Context/resolution 内容、大小/Schema/JCS、RETURN 引用、
ACB/Review outcome、真实请求的 Idempotency-Key/hash/receipt replay、CAS 等仍 MUST
在执行时检查。清单不创建这些输入、不猜测证据、不预先分配 UUID。所有项均携带
`request_validation: REQUIRED_AT_EXECUTION`。调用者必须按现有接口提供合法输入，
原请求错误、失败 receipt、replay、transaction rollback 语义保持原样。

### CTR-AA-004 — Transition 三种 effect 的独立执行判定

每个候选必须复用 executor 对**该显式模型**的 source/target/Definition/primary/effect
合法性、Principal enabled 和 `actor_is_effective_current_assignee`、target owner 解析及
enabled 检查。MUST 补 cancelled/open Assistance 检查；不得因为旧字段 true 而略过。
模型3还须复用当前 activation 的唯一性、当前绑定及可合法关闭条件；缺失或冲突事实
按既有一致性错误处理，不能靠把动作藏成空清单通过。
Archived/terminal 状态使用既有执行生命周期规则；不可把非 terminal Visit 的 cancelled
实例当作 active。TERMINATE 仅在图内真实 edge 合法时存在，不能作为 cancel 的别名。

RETURN MUST 按 CTR-SWEC-005 从该 Instance+transition 的权威 RETURN Events 派生计数，
普通实例采用服务配置的 max（default 3，min 1），managed edge 固定 3；达到上限排除。
count=max-1 时仍可列入，真正成功的 max-th RETURN 才在原事务打开 target Assistance，
detail 读取不能提前触发。RETURN payload 的 same-instance references 仍执行时验证。

Managed transition MUST 复用 `owner_obligation::{reads,guards,transition_context}`：
当前 Owner/Domain/intent fence 和适用的责任状态。每项检查 MUST 保留原 accepted
动作/effect/节点分支，不能提升成所有 managed transition 的统一前置：

- 仅 ADVANCE 且 source 声明 requiredForAdvance operation 时，要求该 source effect
  SUCCEEDED；相关 terminal acceptance 保留其原有 required-effect 证明。source effect
  未完成本身不阻断其他条件合法的 RETURN，不能把 requiredForAdvance 变成 requiredForReturn。
- 仅进入声明 prerequisite gate 的 target 时，要求真实 producer/create receipt/Instance
  binding；无 gate 的 target 不增加 producer 要求。该分支没有真实 producer 时不得把
  “未来可填 UUID”当成合法；已有真实 producer、仅需 gated-entry `contextPayload` 时
  保留 payload 待验，不因 detail 未提交 payload 误删合法操作。
- 仅目标 TERMINAL 且 managed kind=REVIEW 时，要求 Review 自身成功 effect、对应 case
  outcome 等原 terminal 证明；`reviewOutcome` 内容仍执行时核验，非终态 Review 不套用。

进入 gate 可以绑定真实 WAITING supplier；不把后续 READY dispatch 闸加入 transition。

`ACTIONABLE_NOW`/`WAITING_FOR_TIME` 和 prerequisite READY 不是本 Spec 新增的 transition
授权条件。WEP V2 明确 transition 不 enforce eligibility；不得修补该债务或声称此清单
就是 scheduler due-set。`executable_for_actor=true` 既不是充分条件也不是唯一枚举来源。

### CTR-AA-005 — Assistance 各动作分别判定

`request_assistance` MUST 独立要求当前非 terminal、非 cancelled/archived Visit、enabled
requester、**该命令实际使用的 raw current assignee 等式**、enabled Domain 及唯一有效
current Owner、无 OWNER_PENDING/HUMAN_REQUIRED case，并复用适用 managed fence。不可因 transition
blocked 或无 outgoing edge 自动隐藏求助；也不能将 successor-effective transition 权限
自动复制为 raw-assignee Assistance 权限。

`escalate_assistance` MUST 要求当前 enabled Domain Owner、同 Instance 当前 Visit 的
OWNER_PENDING case；`resolve_assistance` 对应 OWNER_PENDING 或 HUMAN_REQUIRED。
两者均复用 `owner_action_on_tx`/`intents::require_effect`；opted-in 的普通 HTTP 调用
不能满足 managed REVIEW effect command provenance 时，两项均排除，指向既有专用面
不在本清单虚构直写权限。linked resolve 的 child terminal Event、accepted Submission、
digest/selected transition 等已有事实必须通过 GCR 独立检查；尚待提交的
`governanceChildReview`/resolution payload 不代造、不免验。无合法终态证据则不列 resolve。
case ID 仅在该 actor 按既有 case 权限可见且操作合法时返回；不能列旧 Visit 的 case。

### CTR-AA-006 — Cancel / Archive 独立于 transition

`cancel` MUST 使用原 cancel 的 enabled Principal、现行治理授权（current enabled
Domain Owner 或 CTR-CP-001 的 enabled GLOBAL_WORKFLOW_COORDINATOR，及仅在原权威
明确允许时的 managed cleanup actor）、非 cancelled、非 archived、非 terminal、
activation closure/managed fence 等状态检查。没有 RETURN/TERMINATE edge、open
Assistance 或 revoked Definition 本身不得被误当成 cancel 的 transition 拒绝条件。
只有执行 cancel 才 void 原 case/close activation；读取零变更。

`archive` MUST 使用其自身治理授权，要求 terminal 或 cancelled、尚未 archived，
并满足 VAI 的无 active activation 检查；不继承 cancel 的 active-instance 条件。
已取消实例的合法 archive 可以成为唯一项；终态不等于永远空清单。当前 Owner 更换立即
使用新 binding；不授权旧 Owner。当前 full detail 本来因断裂指针失败的实例仍失败，
不以本 Spec 实施 proposed `SVC_WORKFLOW_DANGLING_INSTANCE_CANCEL_V1` 的修复路线。

### CTR-AA-007 — 精确 wire/schema

外层继续是闭集 `{visibility,detail}`。full `detail` 保留六个原字段，**新增且必填**
以下四个 snake_case 字段；这是 endpoint 专属 `InstanceDetailWithAuthorizedActionsV1`
闭对象（`additionalProperties:false`），不是给共享 `FullWorkflowInstanceDetail` 放开任意属性：

```text
authorized_actions_scope: "INSTANCE_TASK_AND_GOVERNANCE_V1"
authorized_actions_status: "AVAILABLE" | "NONE_FOR_ACTOR" |
                           "MISSING_EXECUTE_SCOPE" | "WRITE_GATE_CLOSED" |
                           "UNSUPPORTED_MODEL"
authorized_actions: AuthorizedAction[]
current_task_type: {
  source: "CURRENT_NODE_TYPE",
  node_type: "DRAFT" | "NORMAL" | "TASK" | "TERMINAL",
  business_task_type: null
}
```

`current_task_type` 自身也是闭集，三个字段必填。`business_task_type:null` 精确表示当前
没有 accepted 的独立业务任务分类可投影；不能从 node_key/display_name/instructions、
角色名或任意 metadata 推断“审批/开发/评审”等类别。node_type 必须逐值等于同快照
`current_visit.node.node_type`；未知值/损坏指针按原一致性错误处理，不能补 TASK。

`AuthorizedAction` 是以下互斥 `oneOf`，每种对象 `additionalProperties:false`，所列键
全部 required，不适用键 MUST 缺省（不是 null），UUID 来自已有事实：

```text
{operation:"transition", transition_id:UUID,
 transition_effect:"ADVANCE"|"RETURN"|"TERMINATE",
 request_validation:"REQUIRED_AT_EXECUTION"}
{operation:"request_assistance", request_validation:"REQUIRED_AT_EXECUTION"}
{operation:"escalate_assistance", assistance_case_id:UUID,
 request_validation:"REQUIRED_AT_EXECUTION"}
{operation:"resolve_assistance", assistance_case_id:UUID,
 request_validation:"REQUIRED_AT_EXECUTION"}
{operation:"cancel", request_validation:"REQUIRED_AT_EXECUTION"}
{operation:"archive", request_validation:"REQUIRED_AT_EXECUTION"}
```

transition identity 是现有 DefinitionVersion 的 `transition_id`，effect 必须逐值等于
该 edge；不得用 transition_key 代替 UUID 或凭 node_type 合成 effect。需要的 schema
仍读既有 outgoing 项和原 command contract，不复制或删改其字段。排序为上述 operation
列举顺序，类内按 transition_id/assistance_case_id 的 UUID 字节升序；唯一键为
`(operation,transition_id)` / `(operation,assistance_case_id)` / `(operation)`。
成功返回不得截断；读取全部合法 edge/case，不能使用历史分页/limit 造成漏项。

### CTR-AA-008 — 空集合、scope、restricted 与模型

full 成功必须同时返回清单与标注：有项 iff status=AVAILABLE；所有其他 status 必须
`authorized_actions:[]`，无项不能省字段、用 null 或填“默认 ADVANCE/RETURN/求助”。
优先级为 MISSING_EXECUTE_SCOPE → WRITE_GATE_CLOSED → UNSUPPORTED_MODEL →
NONE_FOR_ACTOR/AVAILABLE；原授权/一致性/存储错误始终优先于成功响应。
NONE_FOR_ACTOR 仅断言在该 token/actor/快照和 V1 范围内确无合法操作，不断言业务完成。

UNSUPPORTED_MODEL 用于已可合法读取、但缺少 accepted 操作语义的显式模型（包括不可
冒充 VISIT_ACTIVATION_V1 的描述性 Minimal 数值 2）；不是断言它不存在任何源码路径。
不得擅自实现其 non-primary ADVANCE 或套用 Legacy primary 规则。这一状态保留缺口，
对应实现 readiness 为 NO，直到另有 accepted 模型权威；不能报合法操作集合验收 PASS。

`historical_participant` 分支 MUST 保持原字段集，四个新字段全部缺省；不返回空清单
冒充完成全量授权评估，不泄露 assignee/case/指针。没有 full 权限的 Owner/Coordinator/
Reader/Auditor/Scheduler 不因角色名称或此清单获得 full detail 权限。

### CTR-AA-009 — AMENDMENT 映射与共享面隔离

下表“AMENDMENT”是 whole-successor 的原文→新语义地图，当前未激活。旧 IDs、旧布尔、
CTI 的既有 Contracts 不在原地赋予新含义。既有 Query 后续已被 accepted 子权威细化的
身份/eligibility 等规则继续保留，不借重载 legacy 原文倒退。

| ID | 固定原文 / 定位 | 明确的新语义或保留关系 |
|---|---|---|
| MAP-AA-01 | Query §4 `Full 包含：` 及六组内容，`current Node 的全部 outgoing Transitions` | 全保留，再按 CTR-AA-007/008 仅为 instance detail Full 加四字段；whole legacy replacement，非偷偷开放闭集 |
| MAP-AA-02 | Query §4 `executableForActor` 与六个稳定 blockedReason | 原投影的 meaning/字段/blockedReason 枚举不变；新增独立清单依据各执行检查，不承诺与旧 true 集合相等 |
| MAP-AA-03 | OpenAPI `FullWorkflowInstanceDetail` 的 `additionalProperties: false` 和六项 required；`WorkflowInstanceDetailResponse.detail: {type: object}` | 后续实现新增 endpoint 专属十字段闭 schema，并让 full response variant 引用它；原 shared 六字段 schema不变；historical response 独立保持原 schema |
| MAP-AA-04 | Query §4 `HistoricalParticipant 只含实例公开摘要和 current Node 公开摘要` | 原句逐义保留，新字段不进入 restricted |
| MAP-AA-05 | Query §6 `每项包含 Full detail`；CTI §2 `Existing endpoints and their wire/order/visibility contracts are unchanged.`；CTR-CTI-005 `New views MUST NOT alter old worklist/history/detail results` | legacy shared Full 原型、worklists、CTI input/body CURRENT_DETAIL bytes/digest 与截断契约不变；仅 detail endpoint 使用新 wrapper；CTI 自身不改旧面 |
| MAP-AA-06 | Internal API Contract `Detail and timeline preserve ... snake_case item fields`；Runtime HTTP Contract §4.6 `{visibility,detail}` | 既有 envelope、字段命名、错误与可见性保留；不据此升级旧 HS256 等历史配置为当前权威 |
| MAP-AA-07 | Architecture §5.14 `Future wire/schema changes require an accepted implementation-authorizing Spec.` | 本候选只有在精确 head 接受、legacy 原子生命周期、merge main 后才满足该门；当前不满足 |

后续实现 MUST 保持 shared `build_full` 的既有输出，通过端点专属组合在同一快照添加
投影，不能二次授权读取不同快照。worklists、current-task-input（含 CURRENT_DETAIL
body/digest）、timeline、list、history、case 专用读均不得增加字段或改变原结果。
DTO/serializer 共用不是扩展授权。旧 outgoing 原样返回，即使旧 true 与新清单不同。

### CTR-AA-010 — 实现闸、兼容性与命名集

本 docs-only PR 不写代码/OpenAPI/SDK/行为测试。后续实现必须已有 accepted authority
在其 PR base，独立 affected-Contract review 及 §10 的真实 RED→GREEN Evidence。
命令、Event、错误码、Scope、角色/Capability、持久化实体及既有路由的命名集 MUST
与 implementation base 相同（集合 diff=∅）；新增 wire 字段/其类型枚举仅限本 Spec
明确列出项，作为单独白名单列出，不能用此例外新增业务动作或错误家族。
生产/跨仓 consumer 另有自己的授权和验收，本 Spec 的 source PASS 不代替它们。

## 10. Acceptance

以下均是**未来实施验收要求，当前 NOT_RUN**。每项需要精确 Spec/implementation SHA、
isolated PostgreSQL+真实 HTTP 配置、调用身份/token scope（不记 Secret）、原始命令/rc、
请求响应、所需事实 readback。独立审核必须同时做正向和反向集合检查。

| Acceptance | Contracts | Method / inputs | Required evidence / expected result / failure condition |
|---|---|---|---|
| ACC-AA-001 | CTR-AA-001/CTR-AA-003/CTR-AA-004 | RED：旧 true+cancelled、open case、RETURN count=max 各一独立 fixture；GREEN：各操作执行前共享 evaluator 与 detail 集合双向比对 | 写命令以合规 payload 验证拒绝并零业务事实；原缺新字段/旧过滤方案必须有针对性 RED，不能只报编译失败；GREEN 每类集合相等，任一 false positive/negative 即 FAIL |
| ACC-AA-002 | CTR-AA-003/CTR-AA-004 | Legacy/模型3的合法 primary ADVANCE、RETURN、TERMINATE；RETURN count=0/max-1/max；target disabled、revoked/DRAFT、Owner/assignee 更换、managed effect/producer/Review fixtures | 明确覆盖 executable_for_actor=true 且合法项被保留，以及 true 但实际状态拒绝被排除；source effect未成功但其他条件合法且target无gate的managed RETURN仍列入并可执行，ADVANCE相应被阻断；非gate target/非terminal Review不误套producer/terminal proof；max-1 成功写才开 target case，detail 读取零写；无 edge 不虚构，漏项/多项/读提前开 case 即 FAIL |
| ACC-AA-003 | CTR-AA-001/CTR-AA-005 | raw assignee vs canonical successor；open/not-open/旧 Visit case；Owner/non-owner；linked child 未终态/终态证明；opted-in ordinary vs真实 effect入口 | request/escalate/resolve 分别对照原独立判定；不能从 transition bool 推权；managed普通直写不在清单，专用操作不冒充 INCLUDED；任一错误权限或伪证据即 FAIL |
| ACC-AA-004 | CTR-AA-001/CTR-AA-006 | 无 outgoing 的 active Owner；cancelled/terminal 未归档；已归档；current/replaced Owner、Coordinator；active activation、cleanup actor | cancel/archive 独立集合，普通 assignee无治理权；cancelled允许合法 archive但无 transition；无重复归档；原执行器角色/closure一致，套用transition过滤或扩权即 FAIL |
| ACC-AA-005 | CTR-AA-002/CTR-AA-007/CTR-AA-008 | full read+execute / read-only token / write gate关闭；无合法动作；restricted/隐藏/disabled Principal；四种node_type、业务类别缺失、Minimal2 | 精确 ten-field full / 原 restricted 字段集与 oneOf schema；[]及status一致，business_task_type=null不臆测；Minimal2显式UNSUPPORTED_MODEL且其实现验收非PASS；任意漏标、假动作、case泄露即FAIL |
| ACC-AA-006 | CTR-AA-002/CTR-AA-003/CTR-AA-004 | 同一快照并发 Owner/case/RETURN 变化；DB/事实故障；合法动作+无效 payload/ACB/RETURN refs/Context/CAS/key；WAITING_FOR_TIME与真实WAITING supplier | 绑定版本的判定正确，执行仍重判、保留原错误；不新加时间/READY transition闸；错误不变空清单；无dummy写、STOP/receipt/outbox；不合规payload成功或失败吞成[]即FAIL |
| ACC-AA-007 | CTR-AA-007/CTR-AA-009/CTR-AA-010 | baseline/candidate 同 fixture 对照所有共享读面；schema/SDK validation；多edge完整排序；受影响模块命名集提取 | 仅四个批准detail字段差异；旧 outgoing/CTI body及digest/worklists/error原样；逐模块命名集与全域集 diff=∅，附提取方法/集合原文/哈希，任何新增业务名或非白名单wire扩散即FAIL |
| ACC-AA-008 | CTR-AA-009/CTR-AA-010 | docs-only authoring、独立非作者全操作类覆盖审查、exact head recheck、Owner lifecycle/base gate | 两个r1阻断逐条有合同映射；schema/frontmatter/whole-legacy原文检查PASS；独立review绑定final head且全类disposition无遗漏；无Owner精确head接受则保持PENDING、禁实现/merge；伪造接受或行为PASS即FAIL |

ACC-AA-001..007 的 RED/GREEN 必须复用同一有区分力 fixture，记录 RED 为什么失败和
GREEN 为什么修复。只验证字段存在、只对照同一个新函数两次或只跑 mock snapshot
不够；需独立调用实际原执行入口，验证允许/拒绝、response、receipt/Event/version
不变量。故障注入和真实写仅在隔离测试环境；本轮不运行。

EVIDENCE_NEEDED = 最终 docs diff、治理/语法检查、独立非作者逐类审查；运行验收另轮。
DONE_WHEN = 文档+映射、独立审查PASS、commit/push/open PR、#1344链接与人话摘要。
EXPANSION_TRIGGER = 改写旧布尔/权限/执行语义、触及排除类、非docs变更或main相关权威变动。
NEXT_REAL_ACTION = independent REVIEW of exact candidate, then authorized GitHub delivery only。

## 11. Alternatives and disposition

- `ALT-AA-001` REJECTED：直接过滤 executable_for_actor；缺 cancelled/open case/RETURN
  cap 和非 transition 操作；r1 的第二项阻断保留为有区分力反例。
- `ALT-AA-002` REJECTED：让 executable_for_actor 升级为完整执行承诺，或修改其
  blockedReason 枚举；会更改旧 accepted meaning，超出本次 whole legacy 的保留边界。
- `ALT-AA-003` REJECTED：原地放开 shared Full schema / 顺带改 CTI body；扩大既有冻结面。
- `ALT-AA-004` RETAIN：专用 managed effect、governance child、scheduler/admin 入口承担
  各自不同责任，不合并成普通动作，不增加永久兼容权限表。旧布尔保留稳定诊断 facade。

## 12. Migration, compatibility, and rollback

### r1 两项阻断闭合地图

| r1 停点 | 本候选的解决 | 当前 readiness |
|---|---|---|
| 封闭 detail schema 无 accepted 覆盖 | CTR-AA-007/009、MAP-AA-01..07、whole legacy正文和原文候选backlink，明确十字段endpoint schema及受影响面 | 文本待review；只有Owner接受+main合并后才可实施，当前NO |
| 旧 true 不能代表完整准确操作集合 | CTR-AA-001..006/008，全部操作类disposition、每类独立执行检查、payload界限、空/UNSUPPORTED标注、ACC-AA-001..007 | 起草定义了可验证闭合要求；当前无实现/行为PASS |

### 冲突与 conformance drift（不得静默抹平）

1. **既有闭集与候选字段扩展冲突**：Query §4/OpenAPI 六字段对比新十字段；仅通过本
   whole-successor 的 Owner exact-head acceptance + 原子legacy lifecycle解除，未接受
   前原合同优先。不是以Issue、代码或review偏好覆盖旧 authority。
2. **旧 projection 与执行器不一致**：cancelled/open case/RETURN cap/managed guard
   可在旧 true 时拒绝；属于旧diagnostic范围与新完整状态集合不同，不暗改旧字段。
3. **Minimal2 模型差异**：源码允许非primary ADVANCE，Architecture CTR-ARCH-003不许
   把它当模型3；UNSUPPORTED_MODEL显式暴露，不在此新增或修复模型权威。
4. **cancel/archive 角色历史差异**：v0.3.2 Owner-only 已被 accepted CTR-CP-001有意扩展，
   必须沿已接受映射复用；不得据旧原文缩权，也不得把proposed dangling Spec当权威。
5. **Assistance身份差异、managed普通直写拒绝、WEP时间不强制**均按具名accepted
   语义/独立检查分别处理。若后续发现代码与accepted权限矛盾，必须报 conformance drift、
   对应实现 gate=NO；不能复制bug、让本Spec自授修复或用假清单声称已闭合。

MIGRATION = 无存储migration/backfill；更新endpoint专属DTO/schema及授权投影的后续源实现。
COMPATIBILITY = 协议消费者必须接受精确新detail schema；shared六字段类型和CTI维持旧版。
ROLLBACK = 源实现可revert其endpoint扩展；已被consumer采用后的wire撤回应另审compatibility，
不能靠删除历史合同/证据。当前候选撤回只关闭docs PR，不改变生效权威或生产。

## 13. Open questions and acceptance

```text
ACCEPTANCE = PENDING_OWNER_EXACT_HEAD_ACCEPTANCE
OWNER_DIRECTION = direction b authorized; not final-text acceptance
OPEN_OWNER_DECISIONS = exact final head acceptance and atomic legacy lifecycle
NORMATIVE_TBD = NONE
PARTIAL_SUPERSESSION = NONE (whole legacy successor with full carried body)
UNRESOLVED_AUTHORITY_CONFLICT = proposed replacement is inactive; baseline remains authoritative
READY_TO_MARK_ACCEPTED = NO
IMPLEMENTATION_VERIFICATION = NOT_RUN
PRODUCTION_APPLY_AUTHORITY = NONE
```

Owner须明确绑定最终reviewed commit、Spec内容哈希、独立review结论及legacy整份替代。
接受只翻生命周期并更新whole legacy backlink，不重写reviewed语义。最终head或相关base
语义变动必须独立复核。此次完成定义止于open-only PR与#1344回填，不请求或执行merge。

## Appendix A — Whole legacy Query authority, verbatim baseline

下文是 `docs/contracts/WORKFLOW_QUERY_CONTRACT_V0_1.md` 在 base
`5bf2f14ed471c7be0aac097067f0c92aa626505e` 的完整原文（Git blob
`a24b08ea5f2e7e9c89fb201e55379faa6f9e454d`）。原文的 CURRENT/旧Architecture标记
是保留的历史坐标，不接受本候选；其当前有效后续细化仍按§3/CTR-AA-009保留。
仅§9明确列明的instance-detail增量在本Spec正式生效后优先；其他查询义务完整延续。

<!-- BEGIN VERBATIM LEGACY QUERY CONTRACT -->
# Workflow Query Service Contract v0.1

```text
Status: CURRENT
Architecture: svc-workflow v0.3.1
Slice: PR 4
Authority: PostgreSQL workflow projection plus immutable facts
```

本合同冻结 PR 4 的读取边界。查询服务不缓存业务状态、不创建第二套实体，也不从 Event
反推 Context Revision、Node Visit 或 Submission。

## 1. 查询与错误

公开查询固定为：

```text
GetWorkflowInstanceDetail
ListWorkflowTimeline
ListContextRevisions
ListNodeVisits
ListSubmissionHistory
ListAssignedToMe
ListCreatorOwnedDrafts
```

错误集合固定为：

```text
PrincipalNotFound
PrincipalDisabled
WorkflowInstanceNotFoundOrNotVisible
RestrictedHistoryNotVisible
InvalidPagination(detail)
InternalConsistency(detail)
StorageError(detail)
```

不存在实例与不可见实例统一返回 `WorkflowInstanceNotFoundOrNotVisible`。响应不得帮助调用者
探测实例是否存在。

## 2. 一致快照与副作用

每次查询必须：

1. 开启一个 PostgreSQL 事务；
2. 在首次读取前设置 `REPEATABLE READ`；
3. 在同一快照内完成 Principal、授权、投影、事实和分页读取；
4. 不使用 `FOR UPDATE`；
5. 成功时不写 Definition、Runtime、Event、Receipt 或 Audit。

事务不能全局设为 `READ ONLY`，因为拒绝路径必须在同一事务写 SecurityAudit。查询只允许
该拒绝审计副作用。审计写入失败时回滚并返回 `StorageError`，绝不放行读取。

## 3. Principal 与可见性

Principal 不存在时返回 `PrincipalNotFound`，不写审计；Principal 存在但 disabled 时，所有
查询均返回 `PrincipalDisabled`，并写：

```text
action = DISABLED_PRINCIPAL_READ_ATTEMPT
```

Domain disabled 不阻断历史或当前读取。实例可见性按以下优先级判定：

| 等级 | 条件 | 范围 |
|---|---|---|
| `DOMAIN_OWNER_FULL` | actor 是当前 enabled `DOMAIN_OWNER` | full |
| `CURRENT_ASSIGNEE_FULL` | actor 是 current Visit assignee，且 current Node 非 TERMINAL | full |
| `CREATOR_DRAFT_FULL` | actor 是 creator，且 current Node 是 DRAFT | full |
| `HISTORICAL_PARTICIPANT_RESTRICTED` | actor 是 creator、曾任 Visit assignee 或曾创建 Submission | restricted |

其他 actor 收到 masked error，并写：

```text
action       = UNAUTHORIZED_WORKFLOW_READ
resourceType = WORKFLOW_INSTANCE
resourceId   = workflowInstanceId
details      = { queryType, reason: NO_VISIBILITY, domainId }
```

不存在的实例不写审计。Domain Owner 更换后必须立即使用新 binding，旧 Owner 不保留权限。

## 4. Detail 形态

`GetWorkflowInstanceDetail` 返回 tagged `Full` 或 `HistoricalParticipant`。

Full 包含：

```text
instance id / domain / definition version and status / creator
currentContextRevisionId / currentNodeVisitId / workflowStateVersion
externalReference / externalUrl / metadata / createdAt
domainEnabled / isTerminal
完整 current Context（chain、payload、digest、creator、time）
完整 current Visit（Node、assignee、instructions、entered transition、time）
current Node 的全部 outgoing Transitions
```

每条 outgoing Transition 包含 target Node 摘要、effect、submission schema、
`executableForActor` 与稳定的 `blockedReason`：

```text
ACTOR_NOT_CURRENT_ASSIGNEE
CURRENT_NODE_TERMINAL
DEFINITION_VERSION_REVOKED
DEFINITION_VERSION_DRAFT
ADVANCE_NOT_PRIMARY
TARGET_ASSIGNEE_UNAVAILABLE
```

`PUBLISHED` 与 `DEPRECATED` 可执行；`REVOKED` 与防御性 `DRAFT` 可读但阻断。target 的
`WORKFLOW_CREATOR`、`DOMAIN_OWNER` 或 `FIXED_PRINCIPAL` 无法解析为 enabled Principal 时，
返回 `TARGET_ASSIGNEE_UNAVAILABLE`。

`ADVANCE` 只有等于 source Node 的 `primaryAdvanceTransitionId` 时才可执行；额外的非-primary
`ADVANCE` 必须返回 `ADVANCE_NOT_PRIMARY`，与写命令的 Transition 选择规则保持一致。

HistoricalParticipant 只含实例公开摘要和 current Node 公开摘要。它不含 creator、current
pointers、external reference/URL、metadata、Context payload、current assignee、instructions
或 outgoing schema。

## 5. 历史读取

### Timeline

Full 可读取全部 Event 领域字段，但 Event DTO 不复制 Context 或 Submission payload。
restricted 只可读取：

```text
绑定 actor 自己 Submission 的 Event
RETURN Submission 的 relatedSubmissionIds 精确引用 actor 自己 Submission 的反馈 Event
进入 TERMINAL Node 或 TERMINATE 的 outcome Event
```

UUID 必须按 JSON array element 精确匹配，禁止字符串或前缀匹配；跨实例引用不得产生可见性。

### Context revisions

只允许 full。restricted 请求写 `UNAUTHORIZED_WORKFLOW_READ`，details reason 为
`RESTRICTED_SCOPE`，提交审计后返回 `RestrictedHistoryNotVisible`。

### Node visits

Full 返回全部 Visit 和 instructions。restricted 只返回 `assigneePrincipalId = actor` 的 Visit，
并移除 instructions；不派生 open/closed/exited 等非权威状态。

### Submission history

Full 返回全部 Submission、绑定的 Context Revision ID 和 source Node 摘要。restricted 只返回：

```text
actor 自己创建的 Submission
RETURN Submission，且 relatedSubmissionIds 精确引用同实例中 actor 自己的 Submission
```

他人的普通 Submission 和跨实例关联始终不可见。

## 6. Worklists

`ListAssignedToMe` 只匹配 instance 的 current Visit pointer，要求其 assignee 为 actor 且
current Node 非 TERMINAL。旧 Visit 和 TERMINAL Visit 不命中。每项包含 Full detail、当前
instructions/outgoing schema/state，以及最近 50 条 upstream Submissions 和最近 50 条 RETURN
Events；还有更多记录时分别设置 `submissionsTruncated` 或 `returnEventsTruncated`，调用方据此
选择对应的独立 history 查询继续翻页。REVOKED/DRAFT 实例仍列出但 blocked。

`ListCreatorOwnedDrafts` 的 DRAFT 指 current Node `node_type = DRAFT`，不是 Definition Version
状态。它只匹配 `createdByPrincipalId = actor`，包含最新 current Context 和 Full detail：

```text
contextEditable = definition status in {PUBLISHED, DEPRECATED}
combinedExecutable = contextEditable
                     AND actor is current assignee
                     AND at least one outgoing transition executable for actor
```

creator 离开 DRAFT 后不再命中；creator 与 current assignee 不同时仍列出，但
`combinedExecutable = false`。

## 7. Keyset pagination

禁止 offset pagination。所有 page 使用 `items + nextCursor`，均以 `limit + 1` 判断是否还有
下一页。

| 查询 | cursor / 顺序 | default | max |
|---|---|---:|---:|
| Timeline | `afterEventSequence`, ASC | 50 | 100 |
| Context revisions | `afterRevisionNumber`, ASC | 50 | 100 |
| Node visits | `(createdAt,id)`, ASC | 50 | 100 |
| Submission history | `(createdAt,id)`, ASC | 50 | 100 |
| Assigned to me | instance `(createdAt,id)`, DESC | 20 | 20 |
| Creator drafts | instance `(createdAt,id)`, DESC | 20 | 50 |

`limit = 0`、超过 max、负数 sequence/revision cursor 返回 `InvalidPagination`。UUID 是相同
timestamp 下的稳定 tie-breaker。

## 8. 防御性一致性

在返回任何 DTO 前必须验证：

```text
instance domain 与 definition domain 一致
current Context pointer 指向本实例的完整 Context fact
current Visit pointer 指向本实例的完整 Visit fact
Visit Node 属于实例绑定的 Definition Version
event sequence 从 1 连续到 workflowStateVersion
event count/max sequence 与 workflowStateVersion 一致
Event 引用的 Context/Visit/Submission 属于同一实例
Submission 的 Context、source Visit、Node 和 Transition 属于同一实例/版本
全部历史 Visit 的 Node 属于实例 Definition Version
Context revisionNumber 从 1 连续，previousRevisionId 紧邻前一 Revision，current pointer 指向链头
```

破坏任一条件返回 `InternalConsistency`，不得返回部分 DTO。

## 9. 存储边界

PR 4 不新增 Migration、缓存、read-model table 或外部索引。PostgreSQL
`workflow_instances` 投影与 Context Revision、Node Visit、Submission、Workflow Event 事实仍是
唯一权威来源。

<!-- END VERBATIM LEGACY QUERY CONTRACT -->
