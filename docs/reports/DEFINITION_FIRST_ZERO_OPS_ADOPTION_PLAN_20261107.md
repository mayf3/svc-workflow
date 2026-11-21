# Definition-first 无人值守改造：Svc 合同与业务闭环计划

日期：2026-11-07。性质：Owner 请求的本层设计对齐/实施计划，不是新 governing Spec。本 PR 仅此文和根 AGENTS 导航，不修改已接受权威、Rust、migration、线上 Definition、权限或实例。

总设计固定于 agent-control `782dc58fb87abe9ad394975df5a1e258b41eeb2d`：
- [总设计](https://github.com/mayf3/agent-control/blob/782dc58fb87abe9ad394975df5a1e258b41eeb2d/docs/design/definition-first-zero-ops/README.md)
- [实施切片](https://github.com/mayf3/agent-control/blob/782dc58fb87abe9ad394975df5a1e258b41eeb2d/docs/design/definition-first-zero-ops/IMPLEMENTATION.md)
- [固定回归Z01–Z22](https://github.com/mayf3/agent-control/blob/782dc58fb87abe9ad394975df5a1e258b41eeb2d/docs/design/definition-first-zero-ops/REGRESSION.md)
- [本地交接](https://github.com/mayf3/agent-control/blob/782dc58fb87abe9ad394975df5a1e258b41eeb2d/docs/design/definition-first-zero-ops/HANDOFF.md)

这些跨仓链接不转移Product Authority；有新语义时由本仓和必要配对仓库处理正式权威。原#127/#1352/#1374实施任务复用，不另造Goal。

## 1. 现有权威优先与范围

只读核验基线：`d4c510d94f8929f5377989bd69229d611ab49304`。实施前以当前有效 `.agents/local/README.md`、Product Boundary、Architecture、Storage及accepted Specs核定，不能只看文件名数字。该基线索引包含Product Boundary V13、Architecture V0_4_7、Owner Obligation V3、Visit Activation V3、Acceptance Context Binding V2等；实施者核对其完整接受/生效关系和已存在实现。

Svc继续是Definition/Instance/Visit/Submission/Assistance/command receipt/Event的唯一业务权威。Core负责执行；Auth负责身份与授权；agent-control只负责其原本地工程command bus。目标是让Definition承担业务合同、服务提交边界执行硬约束，不是把Svc变成任意代码执行器或增加第二Scheduler。

当前NodeType已有DRAFT/NORMAL/TASK/TERMINAL；本计划不直接新增CODE枚举。先复用现有schema、可接受操作和事务验证；确有独立外部执行需求时，再在明确缺口上做有限配对合同。

## 2. 修改位置与合法工具

| 事项 | 既有位置/入口 | 边界 |
|---|---|---|
| 图与合同验证 | `src/domain/definition/model.rs`、图校验、`acceptance_binding.rs`、transition事务 | 核验必要产物/关系/版本，不能由模型自称通过替代 |
| 正式定义演进 | Core现有`workflow_definition_authoring`配对Svc的draft/replace/publish接口 | 只有有权Owner；PUBLISHED版本不可原地修改 |
| 当前输入与结果 | 原instance detail、submission history、Context/产物引用和command receipts | 权威读取，保留历史；404不被强行解释成不存在 |
| 技术门与治理子链 | `assistance_transaction`、原Owner obligation/child/review关系 | 只有有权角色处理；ACK不等于解决；不批量清HUMAN |
| 准入与版本 | `src/auth/admission.rs`、transition transaction、继任关系 | 查真实目标Principal，不能按同名、前缀合并 |
| Human闭环 | 原escalate、通知投递/回流与resume证据 | 身份/Case/目标版本绑定；不创造默示授权 |

工具命名仅定位现有能力，实际调用须使用installed manifest与相应角色权限。脚本直接DB写入不替代正式入口。

## 3. 首个业务切片：补文章出边合同

同日既有审计曾观察到authoring/review/reader_test/approval文字要求材料、关键transition却允许空submission；这是时间绑定的反例，不假定当前仍未修。先复用最新合法成果和原文章实例，不重做稿件。

在Owner的后继Definition草稿里按原业务目的适配以下输出：

- authoring：精确稿件revision/digest、合法可读材料、来源、自检。
- reviewing：所审稿件版本、结论、问题位置与理由、报告/读回引用。
- reader_test：所测版本、测试方式、范围、结果与证据；模拟与真人明确区分。
- approval：批准所依据的稿件/审阅/读者测试证据；公开发布仍须独立有效授权。

缺必需输出不能走成功出边；合法RETURN、依赖补给和真实Human路径不得因加强成功schema被堵死。不要要求实例创建者先交应由后续Agent生成的稿件，也不要把可选平台私有数据强行升级为全部复盘工作的前置。

ACB能核对当前Context revision和验收项集合，但`PASS`字符串与schema不证明语义正确。输出声明、服务可证事实、模型语义评估分别保存。必要硬门槛应用原事务/提交验证；若需要产物外部读取或耗时计算，先交注册的有权执行器，再验证可信回执，不在PG事务内访问任意文件/网络/模型。

## 4. Context与产物交付不是一回事

现有Product Boundary V13的contextPayload例外仅用于已声明managed prerequisite gate，保留除workflowPrerequisite外的原业务Context。不能据此授权普通博客authoring任意重写Context。

优先按实际已接受路径使用Submission、产物引用或合法正文快照。生产者记录生成事实和版本，交付责任持久承接，消费者以自己的身份读取并确认；共享操作系统用户或路径可见不等于服务授权。

材料接收去重与文章创建意图去重分开；旧日期材料迟到仍可处理。通知丢失不等于素材没生成，重复消息不应多建单。真实产物可以复用于不同合法目标，不以“用过一次”永远禁止后续使用。

## 5. 受控的Definition自我改进

Domain Owner可以沿已有授权提出、验证、发布后继Definition；不是模型每次运行都编辑自己验收标准。典型过程：具体反例→原治理案件→最小draft差量→正例/负例/保留场景→独立审查→有权发布→明确的旧实例承接→原业务验证。

需要检查：

- input/output是否可满足，谁产生上游材料，消费者如何读到；
- 成功边是否强制真实必要产物，失败边是否有合法下一责任；
- 输出合同是否在当前工具/权限下实际可达；
- budget/RETURN/STOP是否遵守既有规则，不能新建实例清零；
- 当前用户决定是否让目标失效或已完成，重建后继是否重复承担同一效果；
- 旧实例仍钉原版本，新版本不能作为旧门已合法通过的凭证。

Definition不能授予新权限、允许任意代码、改变Svc全局政策或接受自己的长久语义变更。确需突破已接受合同的地方，按本仓docs-first/后继规则处理，范围内配置与bug修复不要求另一个宏大Spec。

## 6. 技术治理与Human通知

普通技术失败先交已有合法Owner/Debug/Dev链。治理子任务结束后，当前Owner应依据精确terminal/submission证明判断原门，而不是借子流程done直接完成原业务。Owner身份变化、旧节点、过期回复都要重新核验，不默默选择更方便身份。

真正Human：已有机制将升级事实可靠传到原授权通道，携带Case/目标/版本/选项/已尝试证据；回复经身份、选项与范围核验，由有权角色执行原命令。通知失败和Owner回合失败保留责任；无回复不默认同意；重复答复不多次resolve或公开发布。复用#1352与现有event/outbox/通知消费者，缺接线就补接线，不另外创建人类决策数据库。

如果现有协议明确不提供某种自动恢复，先给出负担承接的合法路径；不能把“WAITING存在”误说成已有自动恢复保证。自动化也不能把UNKNOWN状态按超时改成失败/成功。

## 7. 回归、版本组合与完成标准

本仓优先映射Z01/Z03–Z08/Z10–Z16/Z20–Z22。使用真实隔离PostgreSQL与正式写入口验证：空成功载荷拒绝、合法载荷通过、旧revision拒绝、Case/Instance错配、同actor多实例、合法子流程、旧回执不污染、事务冲突零部分写、重复命令同效果、Human回流、Definition后继与目标暂停。

让同一个审阅角色处理文章与PPT时，只变当前合同/方法/输入，不新增Principal。Svc验证业务关系，角色记忆不能覆盖当前任务。语义评估在独立保留场景验证，不把schema通过等于稿件质量通过。

冻结实际Core/Svc/迁移/Definition/角色/Skill/任务配置组合。源码通过与PUBLISHED定义、生产installed/enabled、真实业务accepted分开。当前accepted ACB等已有反例保留；若新要求与已接受语义冲突，明确最小权威差量而非绕过校验。

本次文档PR没有执行上述产品验证。后续第一项交付应是一个实际Definition草稿或最小代码diff、同一原反例RED→GREEN、独立回执与原文章合法接续，而不是再交一份原则清单。小切片可单独交付；不要将本计划全部功能变成现有安全恢复的总前置。
