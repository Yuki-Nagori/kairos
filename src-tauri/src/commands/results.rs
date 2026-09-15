//! 结果命令：时间目录扫描、场加载与派生场（异步，避免阻塞主线程）。
//! 已加载场缓存于会话（ResultSession），派生在 Rust 侧完成，无需前端回传大数组。

use std::sync::{Arc, Mutex};

use kairos_core::error::{KairosError, Result};
use kairos_core::models::results::{DeriveRequest, ResultCatalog, ScalarField, VectorField};
use kairos_core::services::results;
use kairos_core::services::results::{FieldCache, field_binary};
use tauri::ipc::Response;

/// 会话内的场槽位：派生直接基于缓存计算，前端不回传数值数组。
pub struct ResultSlots {
    /// 当前槽位所属算例目录；每个算例目录对应一次运行结果。
    pub case_dir: Option<String>,
    /// 算例作用域代际；切换算例后旧异步请求不得回写。
    generation: u64,
    /// 槽位请求序号；同一算例内只接受最后一次请求的乱序返回。
    request_seq: u64,
    primary_request: u64,
    compare_request: u64,
    /// 主场：普通加载 / 展示 / 单场派生的数据源。
    pub primary: Option<Arc<ScalarField>>,
    /// 对比场：两场差值派生的减数。
    pub compare: Option<Arc<ScalarField>>,
    /// 有界场缓存（LRU 淘汰）：命中时跳过磁盘读取。
    pub cache: FieldCache,
    /// 最近加载的矢量场三分量（变形显示用；与标量槽位分开，避免形状混淆）。
    pub vectors: Option<Arc<VectorField>>,
}

/// 会话缓存：主场与对比场双槽 + 有界场缓存（差值派生需要两份场数据）。
pub struct ResultSession(pub Arc<Mutex<ResultSlots>>);

impl Default for ResultSession {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(ResultSlots {
            case_dir: None,
            generation: 0,
            request_seq: 0,
            primary_request: 0,
            compare_request: 0,
            primary: None,
            compare: None,
            cache: FieldCache::new(8).expect("默认容量在合法区间"),
            vectors: None,
        })))
    }
}

impl ResultSession {
    /// 切换算例时原子清空所有结果槽位，避免旧运行的场参与新运行混用。
    fn begin_case(&self, case_dir: &str) -> u64 {
        let scope = std::path::Path::new(case_dir)
            .to_string_lossy()
            .into_owned();
        let mut slots = self.lock();
        if slots.case_dir.as_deref() == Some(scope.as_str()) {
            return slots.generation;
        }
        slots.case_dir = Some(scope);
        slots.generation = slots.generation.wrapping_add(1);
        slots.request_seq = 0;
        slots.primary_request = 0;
        slots.compare_request = 0;
        slots.primary = None;
        slots.compare = None;
        slots.vectors = None;
        slots.cache.clear();
        slots.generation
    }

    fn begin_load(&self, case_dir: &str, compare: bool) -> (u64, u64) {
        let generation = self.begin_case(case_dir);
        let mut slots = self.lock();
        slots.request_seq = slots.request_seq.wrapping_add(1);
        let request = slots.request_seq;
        if compare {
            slots.compare_request = request;
        } else {
            slots.primary_request = request;
        }
        (generation, request)
    }

    pub fn reset(&self) {
        let fresh = Self::default();
        let mut slots = self.lock();
        let mut replacement = fresh.lock();
        std::mem::swap(&mut *slots, &mut *replacement);
    }

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
pub async fn list_result_times(
    session: tauri::State<'_, ResultSession>,
    case_dir: String,
) -> Result<ResultCatalog> {
    session.begin_case(&case_dir);
    // 目录重扫显式失效同一算例的文件版本，确保新增/替换文件可见。
    session.lock().cache.clear();
    tauri::async_runtime::spawn_blocking(move || {
        results::scan_times(std::path::Path::new(&case_dir))
    })
    .await
    .map_err(|e| KairosError::internal(format!("结果扫描任务失败：{e}")))?
}

/// 加载指定时间步的场到会话槽位（默认主场，slot="compare" 写对比场）。
/// 场缓存键包含文件路径、修改时间与长度；目录重扫显式失效，未完成场不入缓存。
fn cache_key(case_dir: &str, time_dir: &str, field: &str) -> Result<String> {
    let path = std::path::Path::new(case_dir).join(time_dir).join(field);
    let metadata =
        std::fs::metadata(&path).map_err(|e| KairosError::io(format!("读取场版本失败：{e}")))?;
    let modified = metadata
        .modified()
        .map_err(|e| KairosError::io(format!("读取场修改时间失败：{e}")))?;
    Ok(format!("{path:?}|{modified:?}|{}", metadata.len()))
}

/// 两个传输入口共用加载与槽位写入，文件访问与编码均在阻塞线程池执行。
fn load_into_slot(
    session: &ResultSession,
    case_dir: &str,
    time_dir: &str,
    field: &str,
    compare: bool,
) -> Result<ScalarField> {
    let (generation, request) = session.begin_load(case_dir, compare);
    let key = cache_key(case_dir, time_dir, field)?;
    let cached = session.lock().cache.get(&key).cloned();
    let loaded = match cached {
        Some(field) => field.clone(),
        None => results::read_field(std::path::Path::new(case_dir), time_dir, field)?,
    };
    let mut slots = session.lock();
    let current_request = if compare {
        slots.compare_request
    } else {
        slots.primary_request
    };
    if slots.generation != generation || current_request != request {
        return Ok(loaded);
    }
    if loaded.complete {
        slots.cache.put(key, loaded.clone());
    }
    if compare {
        slots.compare = Some(Arc::new(loaded.clone()));
    } else {
        slots.primary = Some(Arc::new(loaded.clone()));
    }
    Ok(loaded)
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
    let session = ResultSession(Arc::clone(&session.0));
    tauri::async_runtime::spawn_blocking(move || {
        load_into_slot(&session, &case_dir, &time_dir, &field, compare)
    })
    .await
    .map_err(|e| KairosError::internal(format!("结果加载任务失败：{e}")))?
}

/// 二进制标量场：元信息 JSON 与 f32 值区；保留原始 f64 场供后续派生。
#[tauri::command]
pub async fn load_result_field_binary(
    session: tauri::State<'_, ResultSession>,
    case_dir: String,
    time_dir: String,
    field: String,
    slot: Option<String>,
) -> Result<Response> {
    let compare = parse_slot(slot.as_deref())?;
    let session = ResultSession(Arc::clone(&session.0));
    tauri::async_runtime::spawn_blocking(move || {
        let loaded = load_into_slot(&session, &case_dir, &time_dir, &field, compare)?;
        Ok(Response::new(field_binary::encode(&loaded)))
    })
    .await
    .map_err(|e| KairosError::internal(format!("结果加载任务失败：{e}")))?
}

/// 在 core 侧完成场 CSV 编码，避免把整场复制成前端二维行数组。
#[tauri::command]
pub async fn export_result_field_csv(
    case_dir: String,
    time_dir: String,
    field: String,
) -> Result<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let loaded = results::read_field(std::path::Path::new(&case_dir), &time_dir, &field)?;
        Ok(results::scalar_field_csv(&loaded))
    })
    .await
    .map_err(|e| KairosError::internal(format!("结果 CSV 导出任务失败：{e}")))?
}

/// 在 core 侧完成结果统计，避免前端再次扫描大数组。
#[tauri::command]
pub async fn summarize_result_field(
    case_dir: String,
    time_dir: String,
    field: String,
) -> Result<kairos_core::models::results::FieldStats> {
    tauri::async_runtime::spawn_blocking(move || {
        let loaded = results::read_field(std::path::Path::new(&case_dir), &time_dir, &field)?;
        Ok(results::field_stats(&loaded.values))
    })
    .await
    .map_err(|e| KairosError::internal(format!("结果统计任务失败：{e}")))?
}

#[tauri::command]
pub async fn summarize_vector_field(
    case_dir: String,
    time_dir: String,
    field: String,
) -> Result<kairos_core::models::results::FieldStats> {
    tauri::async_runtime::spawn_blocking(move || {
        let loaded =
            results::read_vector_field(std::path::Path::new(&case_dir), &time_dir, &field)?;
        Ok(results::vector_field_stats(&loaded.components))
    })
    .await
    .map_err(|e| KairosError::internal(format!("矢量统计任务失败：{e}")))?
}

#[tauri::command]
pub async fn summarize_tensor_field(
    case_dir: String,
    time_dir: String,
    field: String,
) -> Result<kairos_core::models::results::FieldStats> {
    tauri::async_runtime::spawn_blocking(move || {
        let loaded =
            results::read_tensor_field(std::path::Path::new(&case_dir), &time_dir, &field)?;
        Ok(results::field_stats(&loaded.magnitudes))
    })
    .await
    .map_err(|e| KairosError::internal(format!("张量统计任务失败：{e}")))?
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
    let session = ResultSession(Arc::clone(&session.0));
    let load_session = ResultSession(Arc::clone(&session.0));
    let vectors = tauri::async_runtime::spawn_blocking(move || {
        load_session.begin_case(&case_dir);
        results::read_vector_field(std::path::Path::new(&case_dir), &time_dir, &field)
    })
    .await
    .map_err(|e| KairosError::internal(format!("矢量场加载任务失败：{e}")))??;
    // 入会话槽位：变形显示直接复用，不必再把分量回传前端绕一圈。
    session.lock().vectors = Some(Arc::new(vectors.clone()));
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
pub async fn deform_render_mesh(
    session: tauri::State<'_, ResultSession>,
    store: tauri::State<'_, crate::commands::geometry::GeometryStore>,
    geometry_id: String,
    scale: f64,
) -> Result<Response> {
    if !scale.is_finite() || scale < 0.0 {
        return Err(KairosError::validation("变形倍数必须为非负有限数。"));
    }
    let session = Arc::clone(&session.0);
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let vectors = session
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .vectors
            .clone()
            .ok_or_else(|| KairosError::validation("尚未加载矢量场，请先加载位移场。"))?;
        let render = super::geometry::render_snapshot(&store, &geometry_id)?;
        let deformed = kairos_core::services::deformation::deformed_render_mesh(
            &render,
            &vectors.components,
            scale,
        );
        Ok(Response::new(kairos_core::services::render_mesh::encode(
            &deformed,
        )))
    })
    .await
    .map_err(|e| KairosError::internal(format!("变形任务失败：{e}")))?
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

/// 只读探针曲线，避免取样污染主场槽位。
#[tauri::command]
pub async fn sample_probe_series(
    case_dir: String,
    field: String,
    probes: Vec<kairos_core::models::results::Probe>,
) -> Result<Vec<kairos_core::models::results::ProbeTimeSeries>> {
    tauri::async_runtime::spawn_blocking(move || {
        results::sample_probes(std::path::Path::new(&case_dir), &field, &probes)
    })
    .await
    .map_err(|e| KairosError::internal(format!("探针采样任务失败：{e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_loader_refreshes_values_and_keeps_primary_and_compare_separate() {
        let root = std::env::temp_dir().join(kairos_core::services::project::new_id("slot-load"));
        std::fs::create_dir_all(root.join("1")).unwrap();
        let path = root.join("1").join("p");
        let write = |value| {
            std::fs::write(&path, format!(
            "FoamFile\n{{\nclass volScalarField;\nobject p;\n}}\ninternalField uniform {value};\n"
        )).unwrap()
        };
        let session = ResultSession::default();
        let case = root.to_str().unwrap();
        write(1);
        assert_eq!(
            load_into_slot(&session, case, "1", "p", false)
                .unwrap()
                .values,
            vec![1.0]
        );
        assert_eq!(
            load_into_slot(&session, case, "1", "p", false)
                .unwrap()
                .values,
            vec![1.0]
        );
        write(200);
        assert_eq!(
            load_into_slot(&session, case, "1", "p", true)
                .unwrap()
                .values,
            vec![200.0]
        );
        assert_eq!(session.lock().primary.as_ref().unwrap().values, vec![1.0]);
        assert_eq!(session.lock().compare.as_ref().unwrap().values, vec![200.0]);
        session.lock().cache.clear();
        assert_eq!(
            load_into_slot(&session, case, "1", "p", false)
                .unwrap()
                .values,
            vec![200.0]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replacing_a_field_changes_its_cache_revision_and_reset_discards_slots() {
        let root = std::env::temp_dir().join(kairos_core::services::project::new_id("field-cache"));
        let time = root.join("1");
        std::fs::create_dir_all(&time).unwrap();
        let path = time.join("p");
        std::fs::write(&path, "old").unwrap();
        let case = root.to_str().unwrap();
        let first = cache_key(case, "1", "p").unwrap();
        let session = ResultSession::default();
        let old = ScalarField {
            field: "p".into(),
            time_dir: "1".into(),
            time_s: 1.0,
            values: vec![1.0],
            is_magnitude: false,
            complete: true,
        };
        session.lock().cache.put(first.clone(), old.clone());
        session.lock().primary = Some(Arc::new(old));
        std::fs::write(&path, "new").unwrap();
        let file = std::fs::File::options().write(true).open(&path).unwrap();
        file.set_times(
            std::fs::FileTimes::new().set_modified(
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(123),
            ),
        )
        .unwrap();
        let second = cache_key(case, "1", "p").unwrap();
        assert_ne!(first, second);
        assert!(session.lock().cache.get(&second).is_none());
        session.reset();
        assert!(session.lock().cache.get(&first).is_none());
        assert!(session.lock().primary.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn switching_case_scope_clears_scalar_and_vector_slots() {
        let first = std::env::temp_dir().join(kairos_core::services::project::new_id("case-a"));
        let second = std::env::temp_dir().join(kairos_core::services::project::new_id("case-b"));
        let session = ResultSession::default();
        let field = ScalarField {
            field: "p".into(),
            time_dir: "1".into(),
            time_s: 1.0,
            values: vec![1.0],
            is_magnitude: false,
            complete: true,
        };
        session.begin_case(first.to_str().unwrap());
        {
            let mut slots = session.lock();
            slots.primary = Some(Arc::new(field));
            slots.compare = Some(Arc::new(ScalarField {
                field: "p".into(),
                time_dir: "2".into(),
                time_s: 2.0,
                values: vec![2.0],
                is_magnitude: false,
                complete: true,
            }));
            let cached = slots.primary.clone().unwrap();
            slots.cache.put("first".into(), (*cached).clone());
            slots.vectors = Some(Arc::new(VectorField {
                field: "U".into(),
                time_dir: "1".into(),
                time_s: 1.0,
                components: vec![[0.0, 0.0, 0.0]],
                complete: true,
            }));
        }
        session.begin_case(second.to_str().unwrap());
        let mut slots = session.lock();
        assert_eq!(slots.case_dir.as_deref(), second.to_str());
        assert!(slots.primary.is_none());
        assert!(slots.compare.is_none());
        assert!(slots.vectors.is_none());
        assert!(slots.cache.get("first").is_none());
    }

    #[test]
    fn newer_slot_request_supersedes_older_request_token() {
        let session = ResultSession::default();
        let case = std::env::temp_dir().join(kairos_core::services::project::new_id("request"));
        let case = case.to_str().unwrap();
        let (generation, first) = session.begin_load(case, false);
        let (_, second) = session.begin_load(case, false);
        let slots = session.lock();
        assert_eq!(slots.generation, generation);
        assert!(second > first);
        assert_eq!(slots.primary_request, second);
        assert_ne!(slots.primary_request, first);
    }

    #[test]
    fn incomplete_result_is_not_cached_and_refreshes_after_file_update() {
        let root = std::env::temp_dir().join(kairos_core::services::project::new_id("incomplete"));
        let path = root.join("1").join("p");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let write = |values: &str| {
            std::fs::write(
                &path,
                format!(
                    "FoamFile\n{{\nclass volScalarField;\nobject p;\n}}\ninternalField nonuniform List<scalar>\n2\n(\n{values}\n)\n;\n"
                ),
            )
            .unwrap();
        };
        write("1");
        let session = ResultSession::default();
        let case_dir = root.to_str().unwrap();
        let first = load_into_slot(&session, case_dir, "1", "p", false).unwrap();
        assert!(!first.complete);
        write("1\n2");
        let second = load_into_slot(&session, case_dir, "1", "p", false).unwrap();
        assert!(second.complete);
        assert_eq!(second.values, vec![1.0, 2.0]);
        assert_eq!(session.lock().cache.stats(), (0, 2));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn concurrent_field_reads_share_session_without_cross_slot_corruption() {
        let root = std::env::temp_dir().join(kairos_core::services::project::new_id("concurrent"));
        let time = root.join("1");
        std::fs::create_dir_all(&time).unwrap();
        let content = |name: &str, value: f64| {
            format!(
                "FoamFile\n{{\nclass volScalarField;\nobject {name};\n}}\ninternalField uniform {value};\n"
            )
        };
        std::fs::write(time.join("p"), content("p", 1.0)).unwrap();
        std::fs::write(time.join("T"), content("T", 80.0)).unwrap();
        let session = ResultSession::default();
        let case_dir = root.to_str().unwrap().to_owned();
        let left = ResultSession(Arc::clone(&session.0));
        let right = ResultSession(Arc::clone(&session.0));
        let left_case = case_dir.clone();
        let right_case = case_dir.clone();
        let first = std::thread::spawn(move || load_into_slot(&left, &left_case, "1", "p", false));
        let second =
            std::thread::spawn(move || load_into_slot(&right, &right_case, "1", "T", true));
        assert_eq!(first.join().unwrap().unwrap().values, vec![1.0]);
        assert_eq!(second.join().unwrap().unwrap().values, vec![80.0]);
        let slots = session.lock();
        assert_eq!(slots.primary.as_ref().unwrap().field, "p");
        assert_eq!(slots.compare.as_ref().unwrap().field, "T");
        assert_eq!(slots.cache.stats(), (0, 2));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_result_revision_uses_io_error_contract() {
        let error = cache_key("/definitely-missing-case", "1", "p").unwrap_err();
        assert_eq!(error.kind(), kairos_core::error::ErrorKind::Io);
        assert_eq!(error.kind().as_code(), "io");
    }
}
