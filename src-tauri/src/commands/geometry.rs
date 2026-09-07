//! 几何命令：STL 导入与会话缓存。全量网格只保留在 Rust 侧，前端接触摘要。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;

use kairos_core::error::Result;
use kairos_core::models::geometry::{GeometrySummary, TriangleMesh};
use kairos_core::models::mesh::{MeshingReport, VolumeMesh};
use kairos_core::services::geometry as geometry_service;
use kairos_core::services::meshing::{self, VolumeMeshParams};
use kairos_core::services::project::new_id;
use tauri::State;

/// 单个导入几何的会话缓存（表面网格 + 生成的体积网格）。
pub struct MeshSession {
    pub mesh: TriangleMesh,
    pub file_name: String,
    pub volume: Option<VolumeMesh>,
}

/// 几何会话缓存：渲染与网格生成（T06/T14）从这里取全量数据。
#[derive(Default)]
pub struct GeometryStore(pub Mutex<HashMap<String, MeshSession>>);

#[tauri::command]
pub fn import_stl(store: State<'_, GeometryStore>, path: String) -> Result<GeometrySummary> {
    let mesh = geometry_service::parse_stl_file(Path::new(&path))?;
    let file_name = Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    let geometry_id = new_id("geom");
    let summary = geometry_service::summarize(geometry_id.clone(), file_name.clone(), &mesh);
    store.0.lock().unwrap().insert(
        geometry_id,
        MeshSession {
            mesh,
            file_name,
            volume: None,
        },
    );
    Ok(summary)
}

/// 对已导入几何生成 3D 体积网格，返回统计报告（网格保留在会话缓存中）。
#[tauri::command]
pub fn generate_volume_mesh(
    store: State<'_, GeometryStore>,
    geometry_id: String,
    target_size: f64,
) -> Result<MeshingReport> {
    let params = VolumeMeshParams { target_size };
    params.validate()?;
    let mut sessions = store.0.lock().unwrap();
    let session = sessions.get_mut(&geometry_id).ok_or_else(|| {
        kairos_core::error::KairosError::not_found(format!("几何不存在：{geometry_id}"))
    })?;
    let volume = meshing::generate(&session.mesh, &params)?;
    let report = meshing::report(&volume);
    session.volume = Some(volume);
    Ok(report)
}

#[tauri::command]
pub fn remove_geometry(store: State<'_, GeometryStore>, geometry_id: String) -> Result<()> {
    store.0.lock().unwrap().remove(&geometry_id);
    Ok(())
}

/// 渲染网格数据（供前端 WebGL2 视口上传）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderMeshData {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    /// 每个三角形所属单元索引（云图按单元值着色）。
    pub face_cells: Vec<u32>,
}

fn collect_render_mesh(volume: &VolumeMesh) -> RenderMeshData {
    let mut positions = Vec::with_capacity(volume.nodes.len() * 3);
    for node in &volume.nodes {
        positions.push(node[0] as f32);
        positions.push(node[1] as f32);
        positions.push(node[2] as f32);
    }
    // 边界面归属：重算面计数后，只保留边界三角面并记录 owner 单元。
    let mut face_count: HashMap<[usize; 3], usize> = HashMap::new();
    for tet in &volume.tets {
        for face in [
            [tet[0], tet[1], tet[2]],
            [tet[0], tet[1], tet[3]],
            [tet[0], tet[2], tet[3]],
            [tet[1], tet[2], tet[3]],
        ] {
            let mut key = face;
            key.sort_unstable();
            *face_count.entry(key).or_insert(0) += 1;
        }
    }
    let mut indices = Vec::new();
    let mut face_cells = Vec::new();
    for (cell_index, tet) in volume.tets.iter().enumerate() {
        for face in [
            [tet[0], tet[1], tet[2]],
            [tet[0], tet[1], tet[3]],
            [tet[0], tet[2], tet[3]],
            [tet[1], tet[2], tet[3]],
        ] {
            let mut key = face;
            key.sort_unstable();
            if face_count.get(&key) == Some(&1) {
                indices.extend_from_slice(&(face.map(|i| i as u32)));
                face_cells.push(cell_index as u32);
            }
        }
    }
    RenderMeshData {
        positions,
        indices,
        face_cells,
    }
}

fn collect_stl_render_mesh(mesh: &TriangleMesh) -> RenderMeshData {
    let mut positions = Vec::with_capacity(mesh.triangles.len() * 9);
    let mut indices = Vec::with_capacity(mesh.triangles.len() * 3);
    let mut face_cells = Vec::new();
    for (face, triangle) in mesh.triangles.iter().enumerate() {
        let base = (face * 3) as u32;
        for vertex in [triangle.a, triangle.b, triangle.c] {
            positions.push(vertex[0] as f32);
            positions.push(vertex[1] as f32);
            positions.push(vertex[2] as f32);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);
        face_cells.push(face as u32);
    }
    RenderMeshData {
        positions,
        indices,
        face_cells,
    }
}

/// 导出渲染网格：优先体积网格边界面，否则回退 STL 表面。
#[tauri::command]
pub fn get_render_mesh(
    store: State<'_, GeometryStore>,
    geometry_id: String,
) -> Result<RenderMeshData> {
    let sessions = store.0.lock().unwrap();
    let session = sessions.get(&geometry_id).ok_or_else(|| {
        kairos_core::error::KairosError::not_found(format!("几何不存在：{geometry_id}"))
    })?;
    if let Some(volume) = &session.volume {
        Ok(collect_render_mesh(volume))
    } else {
        Ok(collect_stl_render_mesh(&session.mesh))
    }
}

/// 导入内置样例立方体（首次使用引导 / 端到端冒烟），无需外部 STL 文件。
#[tauri::command]
pub fn import_sample_box(store: State<'_, GeometryStore>, size: f64) -> Result<GeometrySummary> {
    let mesh = TriangleMesh::sample_box(size);
    let geometry_id = new_id("geom");
    let summary = geometry_service::summarize(geometry_id.clone(), "样例立方体.stl".into(), &mesh);
    store.0.lock().unwrap().insert(
        geometry_id,
        MeshSession {
            mesh,
            file_name: "样例立方体.stl".into(),
            volume: None,
        },
    );
    Ok(summary)
}
