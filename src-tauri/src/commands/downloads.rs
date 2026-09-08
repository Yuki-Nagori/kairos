//! 应用内下载：MIT 组件点击直接下载到受管目录（GPL 组件仍走引导安装）。
//! 存放位置固定为 `<应用数据目录>/downloads/`，面板展示路径并支持打开。

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;

use kairos_core::error::{KairosError, Result};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

/// 允许下载的官方源前缀白名单（防任意 URL 下载）。
const ALLOWED_PREFIXES: &[&str] = &[
    "https://gmsh.info/",
    "https://files.openfoam.com/",
    "https://openfoam.org/",
    "https://github.com/OpenFOAM/",
    "https://codeload.github.com/OpenFOAM/",
    "https://github.com/krebeljk/",
    "https://codeload.github.com/krebeljk/",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDownload {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
}

pub fn downloads_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| KairosError::io(format!("无法定位应用数据目录：{e}")))?;
    Ok(dir.join("downloads"))
}

/// 白名单校验：只允许目录里登记过的官方源。
fn ensure_allowed(url: &str) -> Result<()> {
    if ALLOWED_PREFIXES
        .iter()
        .any(|prefix| url.starts_with(prefix))
    {
        Ok(())
    } else {
        Err(KairosError::validation(format!(
            "下载源不在白名单内：{url}"
        )))
    }
}

/// 下载文件到受管目录：流式写盘并按百分比回传进度（Channel<u64>）。
#[tauri::command]
pub async fn download_file(
    app: AppHandle,
    url: String,
    progress: Channel<u64>,
) -> Result<SavedDownload> {
    ensure_allowed(&url)?;
    // 取 URL 末段做文件名：剥离 query/hash，拒绝空段与相对路径段，防目录跳跃。
    let last_segment = url.rsplit('/').next().unwrap_or_default();
    let stem = last_segment.split(['?', '#']).next().unwrap_or_default();
    let file_name = if stem.is_empty() || stem == "." || stem == ".." {
        "download.bin".to_string()
    } else {
        stem.to_string()
    };

    tauri::async_runtime::spawn_blocking(move || {
        let dir = downloads_dir(&app)?;
        fs::create_dir_all(&dir)?;
        let dest = dir.join(&file_name);

        let response = ureq::get(&url)
            .call()
            .map_err(|e| KairosError::io(format!("下载请求失败：{e}")))?;
        let total: u64 = response
            .header("Content-Length")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);

        let mut reader = response.into_reader();
        let mut file = fs::File::create(&dest)?;
        let mut buffer = [0u8; 65_536];
        let mut downloaded: u64 = 0;
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])?;
            downloaded += read as u64;
            if total > 0
                && let Some(percent) = (downloaded * 100).checked_div(total)
            {
                let _ = progress.send(percent);
            }
        }
        file.flush()?;
        drop(file);
        Ok(SavedDownload {
            path: dest.to_string_lossy().to_string(),
            file_name,
            size_bytes: downloaded,
        })
    })
    .await
    .map_err(|e| KairosError::internal(format!("下载任务失败：{e}")))?
}

/// 返回下载目录路径（前端展示「文件存放在哪里」）。
#[tauri::command]
pub fn get_downloads_dir(app: AppHandle) -> Result<String> {
    let dir = downloads_dir(&app)?;
    Ok(dir.to_string_lossy().to_string())
}

/// 在系统文件管理器中打开下载目录。
#[tauri::command]
pub fn open_downloads_dir(app: AppHandle) -> Result<String> {
    let dir = downloads_dir(&app)?;
    fs::create_dir_all(&dir)?;
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&dir)
            .status()
            .map_err(|e| KairosError::io(format!("打开目录失败：{e}")))?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&dir)
            .status()
            .map_err(|e| KairosError::io(format!("打开目录失败：{e}")))?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&dir)
            .status()
            .map_err(|e| KairosError::io(format!("打开目录失败：{e}")))?;
    }
    Ok(dir.to_string_lossy().to_string())
}
