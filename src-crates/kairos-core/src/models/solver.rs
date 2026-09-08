//! 求解器模型：分析阶段与 OpenFOAM case 的求解器侧概念。

use serde::{Deserialize, Serialize};

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
