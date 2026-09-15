//! 求解器模型：分析阶段与 OpenFOAM case 的求解器侧概念。

use serde::{Deserialize, Serialize};

/// 求解环境探测结果，作为跨 IPC 的稳定 DTO。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentCheck {
    pub moldingfoam: bool,
    pub solver: bool,
    pub hint: String,
}

/// case 生成结果，包含目录和浇口入口表达度量。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseOutcome {
    pub case_dir: String,
    pub inlet_area_m2: f64,
    pub inlet_equivalent_diameter_mm: f64,
    pub gates: Vec<GateInlet>,
    pub warnings: Vec<String>,
}

/// 单个浇口的请求口径与网格实际入口面回显。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateInlet {
    pub index: usize,
    pub requested_radius_mm: f64,
    pub requested_area_mm2: f64,
    pub actual_area_mm2: f64,
    pub face_count: usize,
    pub equivalent_diameter_mm: f64,
    pub area_ratio: f64,
    pub expressible: bool,
    pub min_face_area_mm2: f64,
}

/// 分析阶段：v1 覆盖填充 / 填充+保压 / 填充+保压+冷却（OpenFOAM compressibleVoF 链路）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStage {
    Fill,
    FillPack,
    FillPackCool,
}

impl AnalysisStage {
    /// 求解物理时长（秒）：由工艺参数推导的近似值，控制字典 endTime 使用。
    pub fn end_time_s(
        &self,
        injection_time_s: f64,
        packing_time_s: f64,
        cooling_time_s: f64,
    ) -> f64 {
        match self {
            Self::Fill => injection_time_s * 2.0,
            Self::FillPack => injection_time_s + packing_time_s + 1.0,
            Self::FillPackCool => injection_time_s + packing_time_s + cooling_time_s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_time_s_follows_stage() {
        assert_eq!(AnalysisStage::Fill.end_time_s(1.0, 8.0, 15.0), 2.0);
        assert_eq!(AnalysisStage::FillPack.end_time_s(1.0, 8.0, 15.0), 10.0);
        assert_eq!(AnalysisStage::FillPackCool.end_time_s(1.0, 8.0, 15.0), 24.0);
    }
}
