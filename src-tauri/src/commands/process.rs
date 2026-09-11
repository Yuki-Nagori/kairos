//! 工艺命令：成型工艺设置校验。规则在 core（services::process）。

use kairos_core::error::Result;
use kairos_core::models::process::ProcessSettings;
use kairos_core::services::process;

/// 校验工艺设置，返回问题清单（空 = 通过）。
#[tauri::command]
pub fn check_process(settings: ProcessSettings) -> Result<Vec<String>> {
    Ok(process::validate(&settings))
}
