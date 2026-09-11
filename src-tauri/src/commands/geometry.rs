//! 几何命令：STL 导入与会话缓存。全量网格只保留在 Rust 侧，前端接触摘要。

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use kairos_core::error::{KairosError, Result};
use kairos_core::models::geometry::{GeometrySummary, TriangleMesh};
use kairos_core::models::mesh::{
    DualDomainMesh, DualDomainReport, MeshRefinement, MeshingReport, MidplaneMesh, MidplaneReport,
    VolumeMesh,
};
use kairos_core::models::render::RenderMeshData;
use kairos_core::models::repair::RepairOutcome;
use kairos_core::models::runners::RunnerElement;
use kairos_core::services::dualdomain::{self, DualDomainParams};
use kairos_core::services::geometry as geometry_service;
use kairos_core::services::iges;
use kairos_core::services::meshing::{self, VolumeMeshParams};
use kairos_core::services::midplane::{self, MidplaneParams};
use kairos_core::services::project::new_id;
use kairos_core::services::render_mesh;
use kairos_core::services::repair;
use kairos_core::services::step;
use tauri::State;

/// 单个导入几何的会话缓存。表面网格是唯一入口数据；体积 / 双域 / 中面
/// 网格是各生成命令的产物，供后续渲染与求解消费（暂未被下游读取）。
pub struct MeshSession {
    pub mesh: TriangleMesh,
    pub file_name: String,
    pub volume: Option<VolumeMesh>,
    pub dual: Option<DualDomainMesh>,
    pub midplane: Option<MidplaneMesh>,
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
        Ok(store_import(&store, &path, mesh))
    })
    .await
    .map_err(|e| KairosError::internal(format!("导入任务失败：{e}")))?
}

/// 解析后的网格登记入会话缓存并生成摘要。
fn store_import(store: &GeometryStore, path: &str, mesh: TriangleMesh) -> GeometrySummary {
    let file_name = Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let geometry_id = new_id("geom");
    let summary = geometry_service::summarize(geometry_id.clone(), file_name.clone(), &mesh);
    store.lock().insert(
        geometry_id,
        MeshSession {
            mesh,
            file_name,
            volume: None,
            dual: None,
            midplane: None,
        },
    );
    summary
}

/// 修复已导入几何：顶点焊接 / 退化面移除 / 孔洞填充 / 法向一致化 / 自交检测。
/// 修复后体积网格失效（作废待重新生成），返回更新后的摘要与修复报告。
#[tauri::command]
pub async fn repair_geometry(
    store: State<'_, GeometryStore>,
    geometry_id: String,
) -> Result<RepairOutcome> {
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
        Ok(RepairOutcome { summary, report })
    })
    .await
    .map_err(|e| KairosError::internal(format!("修复任务失败：{e}")))?
}

/// 导入 STEP 镶嵌网格（AP242 TRIANGULATED_FACE_SET / POLY_LOOP 子集）。
#[tauri::command]
pub async fn import_step(store: State<'_, GeometryStore>, path: String) -> Result<GeometrySummary> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mesh = step::parse_step_file(Path::new(&path))?;
        Ok(store_import(&store, &path, mesh))
    })
    .await
    .map_err(|e| KairosError::internal(format!("导入任务失败：{e}")))?
}

/// 导入 IGES 镶嵌网格（实体 106 / 63 子集）。
#[tauri::command]
pub async fn import_iges(store: State<'_, GeometryStore>, path: String) -> Result<GeometrySummary> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mesh = iges::parse_iges_file(Path::new(&path))?;
        Ok(store_import(&store, &path, mesh))
    })
    .await
    .map_err(|e| KairosError::internal(format!("导入任务失败：{e}")))?
}

/// 生成双域网格：表面三角形厚度配对 + 流道/浇口梁单元耦合
/// （网格保留在会话缓存中，前端获得统计报告）。
#[tauri::command]
pub async fn generate_dual_domain_mesh(
    store: State<'_, GeometryStore>,
    geometry_id: String,
    runners: Vec<RunnerElement>,
) -> Result<DualDomainReport> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mesh = {
            let sessions = store.lock();
            let session = sessions
                .get(&geometry_id)
                .ok_or_else(|| KairosError::not_found(format!("几何不存在：{geometry_id}")))?;
            session.mesh.clone()
        };
        let dual = dualdomain::generate(&mesh, &runners, &DualDomainParams::default())?;
        let report = dualdomain::report(&dual);
        if let Some(session) = store.lock().get_mut(&geometry_id) {
            session.dual = Some(dual);
        }
        Ok(report)
    })
    .await
    .map_err(|e| KairosError::internal(format!("双域网格任务失败：{e}")))?
}

/// 生成中面网格：顶点配对法（1D/2.5D 快速分析路线），杆系梁耦合中面节点
/// （网格保留在会话缓存中，前端获得统计报告）。
#[tauri::command]
pub async fn generate_midplane_mesh(
    store: State<'_, GeometryStore>,
    geometry_id: String,
    runners: Vec<RunnerElement>,
) -> Result<MidplaneReport> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mesh = {
            let sessions = store.lock();
            let session = sessions
                .get(&geometry_id)
                .ok_or_else(|| KairosError::not_found(format!("几何不存在：{geometry_id}")))?;
            session.mesh.clone()
        };
        let (mid, report) = midplane::generate(&mesh, &runners, &MidplaneParams::default())?;
        if let Some(session) = store.lock().get_mut(&geometry_id) {
            session.midplane = Some(mid);
        }
        Ok(report)
    })
    .await
    .map_err(|e| KairosError::internal(format!("中面网格任务失败：{e}")))?
}

/// 对已导入几何生成 3D 体积网格，返回统计报告（网格保留在会话缓存中）。
/// refinement 为可选分级加密（边界层 / 区域盒，仅体素引擎支持）。
#[tauri::command]
pub async fn generate_volume_mesh(
    store: State<'_, GeometryStore>,
    geometry_id: String,
    target_size: f64,
    refinement: Option<MeshRefinement>,
) -> Result<MeshingReport> {
    let params = VolumeMeshParams {
        target_size,
        refinement,
    };
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
/// 子进程调用（GPL 隔离红线），解析 msh2 回 VolumeMesh。target_size 作为
/// 目标单元尺寸上限（-clmax）传入 Gmsh。
#[tauri::command]
pub async fn generate_gmsh_mesh(
    app: tauri::AppHandle,
    store: State<'_, GeometryStore>,
    geometry_id: String,
    target_size: f64,
) -> Result<MeshingReport> {
    if !target_size.is_finite() || target_size <= 0.0 {
        return Err(KairosError::validation("目标网格尺寸必须为正数。"));
    }
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
        kairos_core::services::geometry::write_stl_binary(&mesh, &temp)?;

        let volume = kairos_core::services::gmsh::tetrahedralize(
            &gmsh_path,
            &temp,
            &out_msh,
            Some(target_size),
        )?;
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
        Ok(render_mesh::from_volume_mesh(volume))
    } else {
        Ok(render_mesh::from_surface_mesh(&session.mesh))
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
            dual: None,
            midplane: None,
        },
    );
    Ok(summary)
}
