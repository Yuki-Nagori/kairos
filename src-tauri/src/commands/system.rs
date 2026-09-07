//! 系统信息命令：适配层只做「调用核心服务 → 返回 DTO」，不写业务逻辑。
//! 领域逻辑一律放 `kairos-core`，保持可独立测试（见 ai-docs/ARCHITECTURE.md）。

use kairos_core::models::system::SystemInfo;

/// name 是 Cargo 包名（小写）；面向用户的产品显示名是 tauri.conf.json 的 productName。
#[tauri::command]
pub fn system_info() -> SystemInfo {
    kairos_core::services::system::system_info(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}
