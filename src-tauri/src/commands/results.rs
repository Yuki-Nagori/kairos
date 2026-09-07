//! 结果命令：时间目录扫描与场加载（异步，避免阻塞主线程）。

use kairos_core::error::{KairosError, Result};
use kairos_core::models::results::{ResultCatalog, ScalarField};
use kairos_core::services::results;

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
    case_dir: String,
    time_dir: String,
    field: String,
) -> Result<ScalarField> {
    tauri::async_runtime::spawn_blocking(move || {
        results::read_field(std::path::Path::new(&case_dir), &time_dir, &field)
    })
    .await
    .map_err(|e| KairosError::internal(format!("结果加载任务失败：{e}")))?
}
