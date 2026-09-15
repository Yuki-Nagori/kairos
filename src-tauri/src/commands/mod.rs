//! Tauri 命令层：按领域拆模块，`lib.rs` 的 `generate_handler![]` 是注册唯一入口。

pub mod dependencies;
pub mod downloads;
pub mod geometry;
pub mod gpu;
pub mod gpu_ops;
pub mod jobs;
pub mod material;
// 原生菜单只做 macOS：无边框窗口上原生菜单不渲染，Windows / Linux 用标题栏里的
// web 菜单（MenuBar.vue），故整个模块按平台裁剪。
#[cfg(target_os = "macos")]
pub mod menu;
pub mod mold;
pub mod process;
pub mod project;
pub mod results;
pub mod solver;
pub mod system;
pub mod vm;
pub mod vm_lease;
