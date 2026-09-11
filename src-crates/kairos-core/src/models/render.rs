//! 渲染网格 DTO：前端 WebGL2 / WebGPU 视口上传用的顶点/索引形态。

/// 渲染网格数据（供前端 WebGL2 视口上传）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderMeshData {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    /// 每个三角形所属单元索引（云图按单元值着色）。
    pub face_cells: Vec<u32>,
}
