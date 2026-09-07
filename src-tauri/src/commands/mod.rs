//! Tauri 命令层：按领域拆模块，`lib.rs` 的 `generate_handler![]` 是注册唯一入口。

pub mod geometry;
pub mod jobs;
pub mod material;
pub mod mold;
pub mod process;
pub mod project;
pub mod solver;
pub mod system;
