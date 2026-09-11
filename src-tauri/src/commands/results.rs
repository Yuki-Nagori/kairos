//! 结果命令：时间目录扫描、场加载与派生场（异步，避免阻塞主线程）。
//! 已加载场缓存于会话（ResultSession），派生在 Rust 侧完成，无需前端回传大数组。

use std::sync::{Arc, Mutex};

use kairos_core::error::{KairosError, Result};
use kairos_core::models::results::{ResultCatalog, ScalarField};
use kairos_core::services::results;

/// 会话内最近加载的场：派生场命令直接基于它计算，前端不再回传数值数组。
#[derive(Default, Clone)]
pub struct ResultSession(pub Arc<Mutex<Option<ScalarField>>>);

impl ResultSession {
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<ScalarField>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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

#[tauri::command]
pub async fn load_result_field(
    session: tauri::State<'_, ResultSession>,
    case_dir: String,
    time_dir: String,
    field: String,
) -> Result<ScalarField> {
    let loaded = tauri::async_runtime::spawn_blocking(move || {
        results::read_field(std::path::Path::new(&case_dir), &time_dir, &field)
    })
    .await
    .map_err(|e| KairosError::internal(format!("结果加载任务失败：{e}")))??;
    session.lock().replace(loaded.clone());
    Ok(loaded)
}

/// 派生场：基于会话缓存的最近加载场执行归一化 / 阈值掩码，返回新场。
#[tauri::command]
pub fn derive_field(session: tauri::State<'_, ResultSession>, kind: String) -> Result<ScalarField> {
    let cached = session.lock().clone();
    let Some(field) = cached else {
        return Err(KairosError::validation("请先加载结果场，再执行派生。"));
    };
    results::derive_scalar_field(&field, &kind)
}
