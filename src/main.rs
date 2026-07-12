//! svc-workflow
//!
//! Serial governed workflow kernel.
//!
//! 当前状态：`ARCHITECTURE_FROZEN` / `IMPLEMENTATION_NOT_STARTED`。
//!
//! 本占位程序仅为保证仓库可编译、可运行。
//! 它不启动 HTTP 服务，不连接数据库，也不实现任何工作流领域逻辑。
//! 数据库 Schema、Command Service 与业务逻辑将在后续任务中按已冻结的架构契约实现。

fn main() {
    println!("svc-workflow: architecture baseline only (v0.3.1, IMPLEMENTATION_NOT_STARTED)");
}
