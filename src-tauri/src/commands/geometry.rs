//! 几何命令：STL 导入与会话缓存。全量网格只保留在 Rust 侧，前端接触摘要。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

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
