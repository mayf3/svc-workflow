# svc-workflow Cancel / Archive 治理边界 v0.3.2

```text
SVC_WORKFLOW_ARCHITECTURE_VERSION=v0.3.2
STATUS=EFFECTIVE
SCHEMA_VERSION=0015
```

> 本文档只记录 v0.3.1 冻结架构之上新增的 Cancel / Archive 治理边界，
> 不重写 v0.3.1 架构，不删除或修改 v0.3.1 的任何既有模型。

---

# 一、变化定位

相对 v0.3.1 冻结架构，本次实现属于：

```text
CANCEL_IS_ARCHITECTURE_CHANGE=true
ARCHIVE_IS_ARCHITECTURE_CHANGE=true（有界：新增治理元数据层，不改终止语义）
DOMAIN_OWNER_PERMISSION_EXPANDED=true
```

Cancel 新增了一条位于 Definition 图外的实例级终止路径；Archive 新增了
terminal/cancelled 实例的一次性治理元数据；两者都扩展了 Domain Owner 的
治理权限。二者均不改变 v0.3.1 的 Node / Transition / TERMINATE 模型。

---

# 二、Cancel 与 TERMINATE 的区别

## 2.1 TERMINATE（v0.3.1 冻结模型，保持不变）

```text
图内异常终止
Definition 预先声明异常终态（如 abandoned / duplicate / rejected 等
  Terminal Node）与 TERMINATE 边
由当前节点负责人执行（普通业务 TERMINATE 需要 Submission）
以 Transition Committed 事件落账（transition_effect = TERMINATE）
实例最终停留在对应 Terminal Node
```

## 2.2 CANCEL（v0.3.2 新增）

```text
图外治理终止
由 Domain Owner 发起独立 Cancel Command
不要求 Definition 预先存在 TERMINATE 边
currentNodeVisitId 保持原节点 Visit 不变
实例标记 cancelled=true，并记录 cancelled_at / cancelled_by / cancel_reason
后续一切流转被阻断（普通 transition 与 combined revise+advance 均拒绝）
已取消实例从默认 worklist 隐藏
以 WORKFLOW_INSTANCE_CANCELLED 事件落账（transition_effect = NULL，
  不产生 from_node / to_node）
```

## 2.3 边界裁定

```text
TERMINATE 适用场景：
  Definition 已声明异常路径、由负责人按流程执行的业务异常结束

CANCEL 适用场景：
  Domain Owner 对活跃实例的治理终止（重复、废弃、无法继续等），
  不依赖 Definition 是否预先画出异常边

为什么 Cancel 不要求 TERMINATE 边：
  Definition 图建模业务路由；治理终止结果只有 Domain Owner 能决定，
  不应要求每个 Definition 为所有可能失败模式预声明治理终态

Cancel 是否属于图外治理终止：
  是。不修改 Definition、不新增 Node Visit、不产生 transition 效果

Cancel 后原 Node Visit 与负责人事实如何解释：
  Visit 保持为当前 Visit，其负责人是取消发生时的历史事实；
  cancelled 标志是权威的关闭标记（非终端 Visit 的负责人保持非空，
  符合既有 CHECK 约束）

Cancel 是否计入 TERMINATE 统计：
  否。Cancel 事件 transition_effect 为 NULL，事件类型独立，
  不进入任何 TERMINATE / transition 统计口径
```

---

# 三、Archive 边界

```text
Archive 只针对 terminal/cancelled instance
Archive 是一次性的治理和展示元数据
Archive 不改变业务终止结果
Archive 不删除 Instance、Visit、Submission、Event 或历史
Archive 后详情、Timeline、Submission History 仍按既有权限可读
```

实现与上述边界一致：

```text
归档写入 archived_at / archived_by_principal_id / archive_reason，
并落账 WORKFLOW_INSTANCE_ARCHIVED 事件
重复归档（新 idempotency key）返回 409 already_archived，
不覆盖归档元数据、不追加第二条事件、不增长状态版本、不产生成功 Receipt
同 key 同请求返回既有成功 Receipt replay
```

---

# 四、Domain Owner 权限扩展

```text
v0.3.1：
  Domain Owner 负责查看、审计和 Definition/Domain 治理

v0.3.2：
  Domain Owner 额外拥有取消 active instance、
  归档 terminal instance 的治理权限
```

权限边界：

```text
普通负责人：不获得 Cancel/Archive 权限
普通 Domain 成员：不获得 Cancel/Archive 权限
跨 Domain Principal：不获得 Cancel/Archive 权限
校验事实：domain_role_bindings 中 role_key = DOMAIN_OWNER 且 enabled = TRUE，
  在事务行锁内完成
HTTP 层复用既有 workflow.execute Scope，未新增 Scope
```

---

# 五、Cancel / Archive 的审计和状态版本规则

```text
一次 Cancel 只增加一个 workflow_state_version，只写一条
  WORKFLOW_INSTANCE_CANCELLED 事件
一次 Archive 只增加一个 workflow_state_version，只写一条
  WORKFLOW_INSTANCE_ARCHIVED 事件
事件包含 actor_principal_id、old/new workflow_state_version 与
  非敏感 event_data（reason、操作者、来源节点/是否已取消）
幂等契约：
  同 key + 同请求 → 既有成功 Receipt replay
  同 key + 不同请求 → 409 idempotency_conflict
  新 key + 已取消 → 409 already_cancelled
  新 key + 已归档 → 409 already_archived
不创建 Definition transition，不新增 Node Visit
```

---

# 六、明确不引入

```text
不引入上层业务对象（Todo、Requirement、Article、Campaign 等）
不运行 LLM
不修改 Broker / OpenClaw
不引入第二个权限中心：
  权限事实仍唯一来自 auth-service 与 domain_role_bindings
```
