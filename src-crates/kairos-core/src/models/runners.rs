//! 流道 / 浇口 / 冷却水路模型：一维梁式单元，按端点连通成网络。

use serde::{Deserialize, Serialize};

/// 一维单元类型：浇口（连接型腔）或流道（输送熔体）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerKind {
    Gate,
    Runner,
}

/// 流道 / 浇口单元：起点、终点与圆形截面直径（mm）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerElement {
    pub id: String,
    pub kind: RunnerKind,
    pub diameter_mm: f64,
    pub start: [f64; 3],
    pub end: [f64; 3],
}

/// 冷却水路单元：圆形水道 + 入口介质参数（模壁 1D 通道 BC 的口径）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoolingChannel {
    pub id: String,
    pub diameter_mm: f64,
    pub start: [f64; 3],
    pub end: [f64; 3],
    /// 入口介质温度（°C）。
    pub inlet_temp_c: f64,
    /// 介质质量流量（kg/s）；旧文件缺省为 0（校验时提示补全）。
    #[serde(default)]
    pub mass_flow_rate_kg_s: f64,
    /// 介质比热（J/kg/K）；水 4180，旧文件缺省为 0（校验时提示补全）。
    #[serde(default)]
    pub specific_heat_j_kg_k: f64,
}

impl Default for CoolingChannel {
    fn default() -> Self {
        Self {
            id: String::new(),
            diameter_mm: 8.0,
            start: [0.0; 3],
            end: [0.0; 3],
            inlet_temp_c: 25.0,
            mass_flow_rate_kg_s: 0.05,
            specific_heat_j_kg_k: WATER_SPECIFIC_HEAT,
        }
    }
}

/// 水的比热（J/kg/K）：面板与 case 的默认介质。
pub const WATER_SPECIFIC_HEAT: f64 = 4180.0;

/// 默认对流换热系数（W/m²/K）：面板缺省值，材料 / 机型库接入后细化。
pub const DEFAULT_COOLANT_HTC: f64 = 5000.0;
