//! 结果命令：时间目录扫描、场加载与派生场（异步，避免阻塞主线程）。
//! 已加载场缓存于会话（ResultSession），派生在 Rust 侧完成，无需前端回传大数组。

use std::sync::{Arc, Mutex};

use kairos_core::error::{KairosError, Result};
use kairos_core::models::results::{DeriveRequest, ResultCatalog, ScalarField};
use kairos_core::services::results;

/// 会话内的场槽位：派生直接基于缓存计算，前端不回传数值数组。
#[derive(Default, Clone)]
pub struct ResultSlots {
    /// 主场：普通加载 / 展示 / 单场派生的数据源。
    pub primary: Option<ScalarField>,
    /// 对比场：两场差值派生的减数。
    pub compare: Option<ScalarField>,
}

/// 会话缓存：主场与对比场双槽（差值派生需要两份场数据）。
#[derive(Default, Clone)]
pub struct ResultSession(pub Arc<Mutex<ResultSlots>>);

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
#[tauri::command]
pub async fn load_result_field(
    session: tauri::State<'_, ResultSession>,
    case_dir: String,
    time_dir: String,
    field: String,
    slot: Option<String>,
) -> Result<ScalarField> {
    let compare = parse_slot(slot.as_deref())?;
    let loaded = tauri::async_runtime::spawn_blocking(move || {
        results::read_field(std::path::Path::new(&case_dir), &time_dir, &field)
    })
    .await
    .map_err(|e| KairosError::internal(format!("结果加载任务失败：{e}")))??;
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

/// 派生场：基于会话主场的归一化 / 阈值掩码 / 线性映射，返回新场。
/// 派生不改写会话缓存（源场保持不变，派生场只回显前端）。
#[tauri::command]
pub fn derive_field(
    session: tauri::State<'_, ResultSession>,
    request: DeriveRequest,
) -> Result<ScalarField> {
    let slots = session.lock();
    let Some(field) = slots.primary.clone() else {
        return Err(KairosError::validation("请先加载结果场，再执行派生。"));
    };
    results::derive_scalar_field(&field, &request)
}

/// 两场差值：会话主场 − 对比场，逐值相减，返回新场。
#[tauri::command]
pub fn derive_difference(session: tauri::State<'_, ResultSession>) -> Result<ScalarField> {
    let slots = session.lock();
    let Some(primary) = slots.primary.clone() else {
        return Err(KairosError::validation("请先加载主场，再执行两场差值。"));
    };
    let Some(compare) = slots.compare.clone() else {
        return Err(KairosError::validation(
            "请先加载对比场（加载时选择「对比场」槽位），再执行两场差值。",
        ));
    };
    results::derive_difference(&primary, &compare)
}
