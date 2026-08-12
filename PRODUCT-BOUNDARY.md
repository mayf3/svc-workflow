# svc-workflow 产品边界定义

> 日期：2026-08-02 | 状态：生效中
> 来源：项目梳理工作流（实例 2cf9dde3，CTO+PM+架构审查员联合梳理）

## 定位
平台级工作流引擎（Rust + Axum + PostgreSQL）：为研发交付等场景提供工作流实例生命周期、流转执行、事件溯源与 Domain 隔离能力，自身不承载任何业务逻辑、不承担 UI 展示。

## 做什么（In Scope）
- 工作流实例生命周期管理：创建、流转（ADVANCE/RETURN/TERMINATE）、取消、归档
- 事件溯源：所有状态变更以不可变 Event 记录，timeline 数据源
- Definition 治理：版本化（DRAFT→PUBLISHED→DEPRECATED→REVOKED）、发布、归档、图验证（nodes/transitions 不可变）
- Domain 隔离：多域支持，跨 Domain 不可见（独立 definitions/members/owner）
- 工作清单：按 assignee/creator 查询待办（worklists/assigned-to-me、creator-owned-drafts）
- Admin/Provisioning：Principal、Domain、RoleBinding 管理
- 幂等与并发治理：client-supplied idempotency_key + request_hash 自动 replay，workflow_state_version 乐观并发

## 不做什么（Out of Scope）
| 不做 | 归属 |
|------|------|
| 业务逻辑（需求管理、任务看板、审批业务规则） | adc-v2 / 各业务层 |
| UI 展示与前端交互 | adc-v2 / architecture-portal |
| 身份认证与令牌签发（只验证不签发） | auth-service |
| 外部系统对接（消息推送、邮件、第三方集成） | 集成层 / 各业务服务 |

## 与其他平台的边界
| 平台 | 关系 |
|------|------|
| auth-service | svc-workflow 消费 auth-service 签发的 RS256 机器 token（aud=svc-workflow、sub=MachinePrincipal UUID），经 JWKS 端点离线验证；跨服务委托通过 auth-service OBO token exchange |
| adc-v2 | 通过 auth-service client_credentials 获取 workflow token 操作工作流；ADC V2 不做状态持久化，svc-workflow 是工作流状态的唯一权威 |
| architecture-portal | 门户展示项目边界/架构数据，工作流状态与实例数据归 svc-workflow，二者不混 |

## 需求归属关键词
| 关键词 | 归属 |
|--------|------|
| 工作流/实例/流转/节点推进/审批流 | svc-workflow |
| 状态机/transition/ADVANCE/RETURN/TERMINATE | svc-workflow |
| 事件溯源/timeline/不可变事件 | svc-workflow |
| Definition/工作流模板/版本发布/图验证 | svc-workflow |
| Domain/工作流域/跨域隔离 | svc-workflow |
| 待办工作清单/worklist/我的草稿 | svc-workflow |
| 实例取消/归档/cancel/archive | svc-workflow |

## 已知风险（V1 设计权衡，非缺陷）
| 风险 | 说明 |
|------|------|
| R1 | 无 DELETE API：实例只能 cancel/archive，不可物理删除（设计如此） |
| R2 | 单实例 PostgreSQL，无读写分离（V1 可接受，后续可加只读副本） |
| R3 | JWKS 缓存失效窗口内可能接受已吊销 token（标准 trade-off，窗口可控） |

## 技术栈
Rust / Axum / PostgreSQL / sqlx / tokio / tower-http

## 架构分层
| 层 | 职责 |
|----|------|
| domain | 领域模型：强类型 ID、枚举、工作流定义、实例命令/事件/错误 |
| application | 用例服务：编排领域逻辑与存储操作，不依赖 HTTP 框架 |
| http | Axum 适配器：路由、处理器、DTO、认证中间件、错误映射 |
| store | PostgreSQL 存储层：仓库模式、原子事务、迁移管理 |
| auth | 认证：Auth V1 RS256/JWKS 离线验证，canary 写门控 |
