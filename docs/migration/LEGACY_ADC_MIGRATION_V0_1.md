# Legacy ADC 迁移勘误 v0.1

```text
Status: PENDING_READ_ONLY_INVESTIGATION
Version: v0.1
```

> 本文件当前仅作为迁移调查占位。
> 本任务不访问、不修改任何现有 ADC、llm-todo 或 Agent Core 仓库。
> 后续调查将在独立任务中以只读方式展开。

---

## 待调查范围

后续只读调查需覆盖以下内容（顺序不代表优先级）：

* 现有 ADC Schema；
* `DomainRoleBinding`；
* Workflow Template / Snapshot；
* `currentStep` 的所有写路径；
* Principal 与 Agent ID；
* `llm-todo` 与开发平台业务表边界；
* Legacy Creator 映射；
* Shadow Relay 可落点；
* Cutover 与回滚字段。

---

## 约束

* 本文件不承诺任何迁移方案；
* 任何迁移方案必须基于架构基线 `SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md` 与实施契约 `IMPLEMENTATION_CONTRACT_V0_1.md`；
* 在调查完成并形成正式迁移契约前，本文件保持 `PENDING_READ_ONLY_INVESTIGATION`。
