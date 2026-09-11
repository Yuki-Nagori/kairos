//! 修复结果 DTO：各项修复 / 检测计数随摘要跨 IPC 返回，前端修复面板展示。

use super::geometry::GeometrySummary;

/// 修复报告：各项修复 / 检测的计数。
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairReport {
    /// 焊接掉的重复顶点数。
    pub merged_vertices: usize,
    /// 移除的退化三角形数。
    pub removed_degenerate: usize,
    /// 填充的孔洞数。
    pub filled_holes: usize,
    /// 孔洞填充新增的三角形数。
    pub filled_triangles: usize,
    /// 法向一致化翻转的面数。
    pub flipped_faces: usize,
    /// 自交三角形对数（仅检测计数，不做几何重构）。
    pub self_intersections: usize,
}

/// 修复命令返回：修复后的几何摘要 + 各项修复计数。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairOutcome {
    /// 修复后的几何摘要（健康检查结果已刷新）。
    pub summary: GeometrySummary,
    /// 各项修复 / 检测计数。
    pub report: RepairReport,
}
