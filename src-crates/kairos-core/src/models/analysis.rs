//! 分析序列模型：不需要求解器的轻量启发式分析产物（当前为浇口位置分析）。

use serde::Serialize;

/// 单个浇口候选：网格单元 + 建议落点（网格节点坐标，mm）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateCandidate {
    /// 候选单元（四面体下标，与适合度场同域）。
    pub cell: usize,
    /// 建议落点节点（该单元最靠近形心的顶点）。
    pub node: usize,
    /// 建议落点坐标（mm）——直接可写进模具网络的浇口终点。
    pub center: [f64; 3],
    /// 归一化适合度（0~1，越大越好）。
    pub score: f64,
    /// 该候选到最远单元的最长流动距离（mm，越小填充越均衡）。
    pub max_flow_length_mm: f64,
    /// 该候选所在单元的局部厚度代理（最短高，mm；薄处易冻死）。
    pub thickness_mm: f64,
}

/// 浇口位置分析报告：适合度场（逐单元）+ Top-N 建议。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateLocationReport {
    /// 逐单元适合度（0~1）；云图口径与求解结果一致（faceCells 映射）。
    pub field: Vec<f64>,
    /// 实际评分的候选数（受候选项上限约束，等距抽样）。
    pub candidate_count: usize,
    /// 参与评分的单元数。
    pub cell_count: usize,
    /// Top-N 建议（按适合度降序）。
    pub top: Vec<GateCandidate>,
    /// 包围盒对角线（mm）：流动长度按它归一。
    pub diagonal_mm: f64,
    /// 评分口径说明（面板直接展示，避免把它当求解结果读）。
    pub basis: String,
}

/// 填充预览报告：从浇口出发的充填覆盖估计（图连通 + 到达序，非求解结果）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FillPreviewReport {
    /// 逐单元归一化到达序（0 = 浇口所在单元，1 = 最远可达单元）。
    /// 未覆盖单元固定为 1.0，需结合 `uncoveredCells` 判读。
    pub field: Vec<f64>,
    /// 被充填覆盖的单元数与占比（0~1）。
    pub covered_count: usize,
    pub coverage_ratio: f64,
    /// 无法从任何浇口到达的单元（孤立区域 / 与大块断开的薄壁）。
    pub uncovered_cells: Vec<usize>,
    /// 各浇口命中的单元（按浇口顺序）。
    pub gate_cells: Vec<usize>,
    /// 最远可达单元的最短路径长度（mm）。
    pub arrival_max_mm: f64,
    /// 落点提示（浇口离制品过远、存在孤立区域等）。
    pub warnings: Vec<String>,
    /// 口径说明（面板直接展示）。
    pub basis: String,
}
