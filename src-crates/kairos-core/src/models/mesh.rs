//! 体积网格模型：节点 + 四面体 + 表面三角面，以及网格化报告 DTO。

use serde::Serialize;

/// 3D 体积网格：节点数组 + 四面体（节点索引）+ 边界面（供后处理/渲染）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VolumeMesh {
    pub nodes: Vec<[f64; 3]>,
    /// 四面体，按节点索引；绕向保证正体积。
    pub tets: Vec<[usize; 4]>,
    /// 恰好被一个四面体使用的面（边界/表面），节点索引三元组。
    pub surface_faces: Vec<[usize; 3]>,
}

/// 单个四面体的质量指标。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshQuality {
    /// 全场最小的「最长边 / 最短边」比（越接近 1 越好）。
    pub min_edge_ratio: f64,
    pub avg_edge_ratio: f64,
    pub max_edge_ratio: f64,
    /// 最小四面体体积（可为负值 sentinel 不出现——生成时已保证正体积）。
    pub min_volume: f64,
}

/// 网格化报告：返回给前端的统计信息。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshingReport {
    /// 生成引擎标识（voxel / gmsh）。
    pub engine: String,
    pub node_count: usize,
    pub element_count: usize,
    pub surface_face_count: usize,
    /// 网格总体积（与制品体积对比可评估占用率）。
    pub total_volume: f64,
    pub quality: MeshQuality,
}
