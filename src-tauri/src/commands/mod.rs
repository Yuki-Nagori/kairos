//! Tauri 命令层：按领域拆模块，`lib.rs` 的 `generate_handler![]` 是注册唯一入口。

pub mod material;
pub mod project;
pub mod system;
