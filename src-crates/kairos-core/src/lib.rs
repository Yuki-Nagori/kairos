//! Kairos 领域层：纯 Rust，不依赖 Tauri 与任何 UI。
//!
//! 分层约定（详见仓库根 `ai-docs/ARCHITECTURE.md`）：
//! - [`models`]   —— IPC DTO，serde 序列化形状是前后端契约，由契约测试锁定；
//! - [`services`] —— 领域服务与引擎逻辑，命令层只做装配与调度；
//! - [`error`]    —— 统一错误类型与 IPC 错误契约。
//!
//! 未来的几何 / 网格 / 求解器 / 结果模块都放在本 crate，保证：
//! 可脱离 Tauri 独立测试、编译快、未来可直接复用给 CLI 或脚本批处理。

pub mod error;
pub mod models;
pub mod services;
