//! 成型工艺设置：注射 / 保压 / 冷却参数（T08）。

use serde::{Deserialize, Serialize};

/// 压力-时间曲线（时间 s，压力 MPa；时间须严格递增）。
pub type PressureCurve = Vec<(f64, f64)>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSettings {
    /// 熔体温度（°C）。
    pub melt_temp_c: f64,
    /// 模具温度（°C）。
    pub mold_temp_c: f64,
    /// 顶出温度（°C）。
    pub ejection_temp_c: f64,
    /// 注射时间（s）。
    pub injection_time_s: f64,
    /// V/P 切换点（注射体积百分比 0–100）。
    pub vp_switch_volume_percent: f64,
    /// 保压压力-时间曲线（MPa）。
    pub packing_pressure_mpa_curve: PressureCurve,
    /// 保压时间（s）。
    pub packing_time_s: f64,
    /// 冷却时间（s）。
    pub cooling_time_s: f64,
    /// 冷却介质温度（°C）。
    pub coolant_temp_c: f64,
}
