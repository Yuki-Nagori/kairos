//! 体积网格模型：节点 + 四面体 + 表面三角面，网格化报告 DTO，
//! 以及双域网格（表面 + 杆系耦合）模型与报告。

use serde::Serialize;

use super::runners::RunnerKind;

/// 3D 体积网格：节点数组 + 四面体（节点索引）+ 边界面（供后处理/渲染）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VolumeMesh {
    pub nodes: Vec<[f64; 3]>,
    /// 四面体，按节点索引；绕向保证正体积。
    pub tets: Vec<[usize; 4]>,
    /// 恰好被一个四面体使用的面（边界/表面），节点索引三元组。
    pub surface_faces: Vec<[usize; 3]>,
}

/// 双域网格：表面三角形（每单元带厚度）+ 一维梁单元（流道 / 浇口）。
/// 梁端点尽量捕捉到表面节点形成耦合（见 [`BeamCoupling`]），
/// 这是「表面 + 杆系」双域分析路线的前处理产物。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DualDomainMesh {
    pub nodes: Vec<[f64; 3]>,
    /// 表面三角形（节点索引）；焊接退化与零面积三角形已剔除。
    pub triangles: Vec<[usize; 3]>,
    /// 每个三角形的局部厚度；0 表示未配对到对面（开放 / 非薄壁区域）。
    pub thickness: Vec<f64>,
    /// 杆系梁单元（节点索引对 + 圆截面直径）。
    pub beams: Vec<DualDomainBeam>,
    /// 梁端点与表面节点的耦合记录。
    pub couplings: Vec<BeamCoupling>,
}

/// 梁单元：两端节点索引、圆截面直径与单元类型。
#[derive(Debug, Clone, PartialEq)]
pub struct DualDomainBeam {
    pub nodes: [usize; 2],
    pub diameter: f64,
    pub kind: RunnerKind,
}

/// 梁端点与表面节点的耦合：捕捉距离用于诊断贴合质量。
#[derive(Debug, Clone, PartialEq)]
pub struct BeamCoupling {
    /// 梁单元序号（beams 下标）。
    pub beam: usize,
    /// 端点侧：0 = 起点，1 = 终点。
    pub endpoint: usize,
    /// 被捕捉到的表面节点索引。
    pub node: usize,
    pub distance: f64,
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

/// 双域网格报告：返回给前端的统计信息。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DualDomainReport {
    pub node_count: usize,
    pub triangle_count: usize,
    pub beam_count: usize,
    /// 成功捕捉到表面节点的梁端点数。
    pub coupling_count: usize,
    /// 未捕捉（自成为自由节点）的梁端点数。
    pub uncoupled_endpoints: usize,
    /// 未配对到对面（厚度为 0）的表面三角形数。
    pub unpaired_triangles: usize,
    pub thickness_min: f64,
    pub thickness_max: f64,
    /// 已配对三角形的平均厚度。
    pub thickness_avg: f64,
}
