//! 几何命令：STL 导入与会话缓存。全量网格只保留在 Rust 侧，前端接触摘要。

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::geometry::{GeometrySummary, TriangleMesh};
use kairos_core::models::mesh::{MeshingReport, VolumeMesh};
use kairos_core::services::geometry as geometry_service;
use kairos_core::services::meshing::{self, VolumeMeshParams};
use kairos_core::services::project::new_id;
use kairos_core::services::repair;
use kairos_core::services::step;
use tauri::State;

/// 单个导入几何的会话缓存（表面网格 + 生成的体积网格）。
pub struct MeshSession {
    pub mesh: TriangleMesh,
    pub file_name: String,
    pub volume: Option<VolumeMesh>,
}

/// 几何会话缓存：渲染与网格生成（T06/T14）从这里取全量数据。
/// 内部为 Arc 句柄：async 命令克隆句柄后在阻塞线程池访问，不阻塞主线程。
#[derive(Clone, Default)]
pub struct GeometryStore(pub Arc<Mutex<HashMap<String, MeshSession>>>);

impl GeometryStore {
    /// 锁的宽容获取：持锁线程 panic 导致中毒时取回内部数据继续。
    /// 锁只保护 HashMap 本身，恢复后不存在被破坏的不变量。
    pub fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, MeshSession>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[tauri::command]
pub async fn import_stl(store: State<'_, GeometryStore>, path: String) -> Result<GeometrySummary> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mesh = geometry_service::parse_stl_file(Path::new(&path))?;
        let file_name = Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        let geometry_id = new_id("geom");
        let summary = geometry_service::summarize(geometry_id.clone(), file_name.clone(), &mesh);
        store.lock().insert(
            geometry_id,
            MeshSession {
                mesh,
                file_name,
                volume: None,
            },
        );
        Ok(summary)
    })
    .await
    .map_err(|e| KairosError::internal(format!("导入任务失败：{e}")))?
}

/// 修复已导入几何：顶点焊接 / 退化面移除 / 孔洞填充 / 法向一致化 / 自交检测。
/// 修复后体积网格失效（作废待重新生成），返回更新后的摘要与修复报告。
#[tauri::command]
pub async fn repair_geometry(
    store: State<'_, GeometryStore>,
    geometry_id: String,
) -> Result<GeometrySummary> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mesh = {
            let sessions = store.lock();
            let session = sessions
                .get(&geometry_id)
                .ok_or_else(|| KairosError::not_found(format!("几何不存在：{geometry_id}")))?;
            session.mesh.clone()
        };
        let (repaired, report) = repair::repair_mesh(&mesh)?;
        let file_name = {
            let sessions = store.lock();
            sessions
                .get(&geometry_id)
                .map(|session| session.file_name.clone())
                .ok_or_else(|| KairosError::not_found(format!("几何不存在：{geometry_id}")))?
        };
        let summary = geometry_service::summarize(geometry_id.clone(), file_name, &repaired);
        if let Some(session) = store.lock().get_mut(&geometry_id) {
            session.mesh = repaired;
            session.volume = None;
        }
        Ok((summary, report))
    })
    .await
    .map_err(|e| KairosError::internal(format!("修复任务失败：{e}")))?
    .map(|(summary, report)| {
        let _ = report;
        summary
    })
}

/// 导入 STEP 镶嵌网格（AP242 TRIANGULATED_FACE_SET / POLY_LOOP 子集）。
#[tauri::command]
pub async fn import_step(store: State<'_, GeometryStore>, path: String) -> Result<GeometrySummary> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mesh = step::parse_step_file(Path::new(&path))?;
        let file_name = Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        let geometry_id = new_id("geom");
        let summary = geometry_service::summarize(geometry_id.clone(), file_name.clone(), &mesh);
        store.lock().insert(
            geometry_id,
            MeshSession {
                mesh,
                file_name,
                volume: None,
            },
        );
        Ok(summary)
    })
    .await
    .map_err(|e| KairosError::internal(format!("导入任务失败：{e}")))?
}

/// 对已导入几何生成 3D 体积网格，返回统计报告（网格保留在会话缓存中）。
#[tauri::command]
pub async fn generate_volume_mesh(
    store: State<'_, GeometryStore>,
    geometry_id: String,
    target_size: f64,
) -> Result<MeshingReport> {
    let params = VolumeMeshParams { target_size };
    params.validate()?;
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // 重计算在锁外进行：锁只覆盖表面网格快照取出与体积网格放回两个瞬间。
        let mesh = {
            let sessions = store.lock();
            let session = sessions
                .get(&geometry_id)
                .ok_or_else(|| KairosError::not_found(format!("几何不存在：{geometry_id}")))?;
            session.mesh.clone()
        };
        let volume = meshing::generate(&mesh, &params)?;
        let report = meshing::report(&volume);
        if let Some(session) = store.lock().get_mut(&geometry_id) {
            session.volume = Some(volume);
        }
        Ok(report)
    })
    .await
    .map_err(|e| KairosError::internal(format!("网格任务失败：{e}")))?
}

/// 生成 Gmsh 引擎网格：定位应用内下载的 gmsh 可执行文件，写临时 STL 后
/// 子进程调用（GPL 隔离红线），解析 msh2 回 VolumeMesh。
#[tauri::command]
pub async fn generate_gmsh_mesh(
    app: tauri::AppHandle,
    store: State<'_, GeometryStore>,
    geometry_id: String,
    target_size: f64,
) -> Result<MeshingReport> {
    let _ = target_size;
    // 定位 gmsh 可执行文件：受管 bin 目录优先，其次 PATH。
    let mut gmsh_path: Option<std::path::PathBuf> = None;
    for dir in super::downloads::managed_bin_dirs(&app) {
        let candidate = dir.join("gmsh");
        if candidate.exists() {
            gmsh_path = Some(candidate);
            break;
        }
    }
    let gmsh_path = gmsh_path.ok_or_else(|| {
        KairosError::not_found("未找到 gmsh 可执行文件，请先在依赖面板下载 Gmsh。")
    })?;

    let store_clone = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // 取出表面网格并写临时 STL（gmsh 只接受文件输入）
        let mesh = {
            let sessions = store_clone.lock();
            let session = sessions
                .get(&geometry_id)
                .ok_or_else(|| KairosError::not_found(format!("几何不存在：{geometry_id}")))?;
            session.mesh.clone()
        };
        let temp = std::env::temp_dir().join(format!("kairos-gmsh-{geometry_id}.stl"));
        let out_msh = std::env::temp_dir().join(format!("kairos-gmsh-{geometry_id}.msh"));
        let _ = std::fs::remove_file(&out_msh);
        kairos_core::services::geometry::write_stl_binary(&mesh, &temp)?;

        let args = kairos_core::services::gmsh::tetrahedralize_args(&temp, &out_msh);
        let output = std::process::Command::new(&gmsh_path)
            .args(&args)
            .output()
            .map_err(|e| KairosError::io(format!("gmsh 启动失败：{e}")))?;
        if !output.status.success() {
            let tail = String::from_utf8_lossy(&output.stderr)
                .lines()
                .last()
                .unwrap_or("无 stderr 输出")
                .to_string();
            return Err(KairosError::io(format!("gmsh 网格化失败：{tail}")));
        }

        let content = std::fs::read_to_string(&out_msh)
            .map_err(|e| KairosError::io(format!("读取 msh 失败：{e}")))?;
        let volume = kairos_core::services::gmsh::parse_msh_v2(&content)?;
        let mut report = kairos_core::services::meshing::report(&volume);
        report.engine = "gmsh".into();

        if let Some(session) = store_clone.lock().get_mut(&geometry_id) {
            session.volume = Some(volume);
        }
        let _ = std::fs::remove_file(&temp);
        let _ = std::fs::remove_file(&out_msh);
        Ok(report)
    })
    .await
    .map_err(|e| KairosError::internal(format!("gmsh 网格任务失败：{e}")))?
}

#[tauri::command]
pub fn remove_geometry(store: State<'_, GeometryStore>, geometry_id: String) -> Result<()> {
    store.lock().remove(&geometry_id);
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
    let sessions = store.lock();
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
    store.lock().insert(
        geometry_id,
        MeshSession {
            mesh,
            file_name: "样例立方体.stl".into(),
            volume: None,
        },
    );
    Ok(summary)
}
