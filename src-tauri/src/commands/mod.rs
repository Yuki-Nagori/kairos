//! Tauri 命令层：按领域拆模块，`lib.rs` 的 `generate_handler![]` 是注册唯一入口。

pub mod dependencies;
pub mod downloads;
pub mod geometry;
pub mod gpu;
pub mod gpu_ops;
pub mod jobs;
pub mod material;
pub mod mold;
pub mod process;
pub mod project;
pub mod results;
pub mod solver;
pub mod system;
