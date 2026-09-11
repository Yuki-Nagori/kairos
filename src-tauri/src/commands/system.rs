//! 系统信息命令：适配层只做「调用核心服务 → 返回 DTO」，不写业务逻辑。
//! 领域逻辑一律放 `kairos-core`，保持可独立测试（见 ai-docs/ARCHITECTURE.md）。

use kairos_core::error::Result;
use kairos_core::models::system::SystemInfo;

/// name 是 Cargo 包名（小写）；面向用户的产品显示名是 tauri.conf.json 的 productName。
/// 全部命令统一 Result 契约（当前实现无失败路径，包装 Ok 保持口径一致）。
#[tauri::command]
pub fn system_info() -> Result<SystemInfo> {
    Ok(kairos_core::services::system::system_info(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    ))
}
