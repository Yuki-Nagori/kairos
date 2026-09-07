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

/// 冷却水路单元：圆形水道 + 入口介质参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoolingChannel {
    pub id: String,
    pub diameter_mm: f64,
    pub start: [f64; 3],
    pub end: [f64; 3],
    /// 入口介质温度（°C）。
    pub inlet_temp_c: f64,
}
