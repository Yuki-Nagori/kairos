//! 工艺命令：成型工艺设置校验。规则在 core（services::process）。

use kairos_core::error::Result;
use kairos_core::models::process::ProcessSettings;
use kairos_core::services::process;

/// 校验工艺设置，返回问题清单（空 = 通过）。
///
/// `volume_mm3`（件体积，来自网格报告）与 `inlet_area_m2`（浇口流通面积，
/// 由模具网络的 Gate 单元等效面积或 case 里 inlet patch 的实测面积给出）可选：
/// 给了体积就追加填充工况量级检查（流量包络、浇口名义速度、浇口剪切速率）。
#[tauri::command]
pub fn check_process(
    settings: ProcessSettings,
    volume_mm3: Option<f64>,
    inlet_area_m2: Option<f64>,
) -> Result<Vec<String>> {
    let mut issues = process::validate(&settings);
    if let Some(volume) = volume_mm3 {
        issues.extend(process::fill_load_hints(volume, &settings, inlet_area_m2));
    }
    Ok(issues)
}
