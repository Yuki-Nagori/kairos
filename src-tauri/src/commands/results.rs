//! 结果命令：时间目录扫描、场加载与派生场（异步，避免阻塞主线程）。
//! 已加载场缓存于会话（ResultSession），派生在 Rust 侧完成，无需前端回传大数组。

use std::sync::{Arc, Mutex};

use kairos_core::error::{KairosError, Result};
use kairos_core::models::render::RenderMeshData;
use kairos_core::models::results::{DeriveRequest, ResultCatalog, ScalarField, VectorField};
use kairos_core::services::results;
use kairos_core::services::results::{FieldCache, field_binary};
use tauri::ipc::Response;

/// 会话内的场槽位：派生直接基于缓存计算，前端不回传数值数组。
pub struct ResultSlots {
    /// 主场：普通加载 / 展示 / 单场派生的数据源。
    pub primary: Option<ScalarField>,
    /// 对比场：两场差值派生的减数。
    pub compare: Option<ScalarField>,
    /// 有界场缓存（FIFO 淘汰）：命中时跳过磁盘读取。
    pub cache: FieldCache,
    /// 最近加载的矢量场三分量（变形显示用；与标量槽位分开，避免形状混淆）。
    pub vectors: Option<VectorField>,
}

/// 会话缓存：主场与对比场双槽 + 有界场缓存（差值派生需要两份场数据）。
pub struct ResultSession(pub Arc<Mutex<ResultSlots>>);

impl Default for ResultSession {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(ResultSlots {
            primary: None,
            compare: None,
            cache: FieldCache::new(8).expect("默认容量在合法区间"),
            vectors: None,
        })))
    }
}

impl ResultSession {
    fn lock(&self) -> std::sync::MutexGuard<'_, ResultSlots> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// 场加载槽位："primary"（默认）或 "compare"。
fn parse_slot(slot: Option<&str>) -> Result<bool> {
    match slot {
        None | Some("primary") => Ok(false),
        Some("compare") => Ok(true),
        Some(other) => Err(KairosError::validation(format!(
            "未知的场加载槽位：{other}（支持 primary / compare）。"
        ))),
    }
}

#[tauri::command]
pub async fn list_result_times(case_dir: String) -> Result<ResultCatalog> {
    tauri::async_runtime::spawn_blocking(move || {
        results::scan_times(std::path::Path::new(&case_dir))
    })
    .await
    .map_err(|e| KairosError::internal(format!("结果扫描任务失败：{e}")))?
}

/// 加载指定时间步的场到会话槽位（默认主场，slot="compare" 写对比场）。
/// 场缓存键：路径 + 时间步 + 场名唯一确定一份数据。
fn cache_key(case_dir: &str, time_dir: &str, field: &str) -> String {
    format!("{case_dir}|{time_dir}|{field}")
}

#[tauri::command]
pub async fn load_result_field(
    session: tauri::State<'_, ResultSession>,
    case_dir: String,
    time_dir: String,
    field: String,
    slot: Option<String>,
) -> Result<ScalarField> {
    let compare = parse_slot(slot.as_deref())?;
    let key = cache_key(&case_dir, &time_dir, &field);
    let cached = session.lock().cache.get(&key).cloned();
    let loaded = match cached {
        Some(field) => field,
        None => tauri::async_runtime::spawn_blocking(move || {
            results::read_field(std::path::Path::new(&case_dir), &time_dir, &field)
        })
        .await
        .map_err(|e| KairosError::internal(format!("结果加载任务失败：{e}")))??,
    };
    session.lock().cache.put(key, loaded.clone());
    {
        let mut slots = session.lock();
        if compare {
            slots.compare = Some(loaded.clone());
        } else {
            slots.primary = Some(loaded.clone());
        }
    }
    Ok(loaded)
}

/// 二进制通道加载标量场：返回 [magic][meta JSON][f32 值区] 的原始字节
/// （值区按 f32 截断，相对误差 ≤ 2^-24）；同时写入会话槽位与 LRU 缓存。
#[tauri::command]
pub async fn load_result_field_binary(
    session: tauri::State<'_, ResultSession>,
    case_dir: String,
    time_dir: String,
    field: String,
    slot: Option<String>,
) -> Result<Response> {
    let compare = parse_slot(slot.as_deref())?;
    let key = cache_key(&case_dir, &time_dir, &field);
    let cached = session.lock().cache.get(&key).cloned();
    let loaded = match cached {
        Some(field) => field,
        None => tauri::async_runtime::spawn_blocking(move || {
            results::read_field(std::path::Path::new(&case_dir), &time_dir, &field)
        })
        .await
        .map_err(|e| KairosError::internal(format!("结果加载任务失败：{e}")))??,
    };
    session.lock().cache.put(key, loaded.clone());
    {
        let mut slots = session.lock();
        if compare {
            slots.compare = Some(loaded.clone());
        } else {
            slots.primary = Some(loaded.clone());
        }
    }
    Ok(Response::new(field_binary::encode(&loaded)))
}

/// 矢量场三分量通道：[magic][meta JSON][f32 值区 ×3]（每单元 x/y/z 顺序平铺）。
/// 供变形显示与矢量派生消费；标量模量仍走 load_result_field_binary。
#[tauri::command]
pub async fn load_vector_field_binary(
    session: tauri::State<'_, ResultSession>,
    case_dir: String,
    time_dir: String,
    field: String,
) -> Result<Response> {
    let vectors = tauri::async_runtime::spawn_blocking(move || {
        results::read_vector_field(std::path::Path::new(&case_dir), &time_dir, &field)
    })
    .await
    .map_err(|e| KairosError::internal(format!("矢量场加载任务失败：{e}")))??;
    // 入会话槽位：变形显示直接复用，不必再把分量回传前端绕一圈。
    session.lock().vectors = Some(vectors.clone());
    Ok(Response::new(field_binary::encode_vector(&vectors)))
}

/// 对称张量场读取（残余应力 sigma / 取向张量等）：返回分量 + 模量 + 主方向。
/// 走 JSON（当前用途是面板展示与主方向读数，网格规模见发布说明）。
#[tauri::command]
pub async fn load_tensor_field(
    case_dir: String,
    time_dir: String,
    field: String,
) -> Result<kairos_core::models::results::TensorField> {
    tauri::async_runtime::spawn_blocking(move || {
        results::read_tensor_field(std::path::Path::new(&case_dir), &time_dir, &field)
    })
    .await
    .map_err(|e| KairosError::internal(format!("张量场加载任务失败：{e}")))?
}

/// 变形显示：用会话里最近加载的矢量场（位移）偏移渲染网格顶点。
/// 位移单位按 m → mm 换算（case 场是 SI）；scale 为显示倍数（0 = 原位）。
#[tauri::command]
pub fn deform_render_mesh(
    session: tauri::State<'_, ResultSession>,
    store: tauri::State<'_, crate::commands::geometry::GeometryStore>,
    geometry_id: String,
    scale: f64,
) -> Result<RenderMeshData> {
    if !scale.is_finite() || scale < 0.0 {
        return Err(KairosError::validation("变形倍数必须为非负有限数。"));
    }
    let displacements = {
        let slots = session.lock();
        let vectors = slots.vectors.as_ref().ok_or_else(|| {
            KairosError::validation(
                "尚未加载矢量场：请在结果面板加载位移场（如 D）后再开变形显示。",
            )
        })?;
        vectors.components.clone()
    };
    let sessions = store.lock();
    let session_mesh = sessions
        .get(&geometry_id)
        .ok_or_else(|| KairosError::not_found(format!("几何不存在：{geometry_id}")))?;
    let render = match &session_mesh.volume {
        Some(volume) => kairos_core::services::render_mesh::from_volume_mesh(volume),
        None => kairos_core::services::render_mesh::from_surface_mesh(&session_mesh.mesh),
    };
    Ok(kairos_core::services::deformation::deformed_render_mesh(
        &render,
        &displacements,
        scale,
    ))
}

/// 派生场：基于会话主场的归一化 / 阈值掩码 / 线性映射，返回新场。
/// 派生在 wgpu compute 完成（GPU 主路径，后处理以硬件加速 GPU 为运行前提，
/// 无可用 GPU 时明确报错不降级）；CPU 参考实现住 core 仅作正确性基准。
/// 派生不改写会话缓存（源场保持不变，派生场只回显前端）。
#[tauri::command]
pub async fn derive_field(
    session: tauri::State<'_, ResultSession>,
    request: DeriveRequest,
) -> Result<ScalarField> {
    let field = {
        let slots = session.lock();
        slots
            .primary
            .clone()
            .ok_or_else(|| KairosError::validation("请先加载结果场，再执行派生。"))?
    };
    tauri::async_runtime::spawn_blocking(move || {
        super::gpu_ops::derive_scalar_field_gpu(&field, &request)
    })
    .await
    .map_err(|e| KairosError::internal(format!("派生任务失败：{e}")))?
}

/// 两场差值：会话主场 − 对比场，wgpu compute 逐值相减，返回新场（GPU 主路径）。
#[tauri::command]
pub async fn derive_difference(session: tauri::State<'_, ResultSession>) -> Result<ScalarField> {
    let (primary, compare) = {
        let slots = session.lock();
        let primary = slots
            .primary
            .clone()
            .ok_or_else(|| KairosError::validation("请先加载主场，再执行两场差值。"))?;
        let compare = slots.compare.clone().ok_or_else(|| {
            KairosError::validation("请先加载对比场（加载时选择「对比场」槽位），再执行两场差值。")
        })?;
        (primary, compare)
    };
    tauri::async_runtime::spawn_blocking(move || {
        super::gpu_ops::derive_difference_gpu(&primary, &compare)
    })
    .await
    .map_err(|e| KairosError::internal(format!("差值任务失败：{e}")))?
}
