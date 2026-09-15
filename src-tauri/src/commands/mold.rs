//! 模具网络命令：流道 / 浇口 / 冷却水路校验。规则在 core（services::runners）。

use kairos_core::error::Result;
use kairos_core::models::runners::{CoolingChannel, RunnerElement};
use kairos_core::services::runners;

/// 校验流道 / 浇口 / 冷却水路网络，返回问题清单（空 = 通过）。
///
/// 即使当前不会失败也包一层 `Result`：命令层统一契约，将来加校验（如参数越界）
/// 才不必改签名、不会破坏前端。
#[tauri::command]
pub fn check_mold_network(
    runner_elements: Vec<RunnerElement>,
    cooling_channels: Vec<CoolingChannel>,
) -> Result<Vec<String>> {
    Ok(runners::check_mold_network(
        &runner_elements,
        &cooling_channels,
    ))
}
