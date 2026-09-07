//! 依赖命令：运行时依赖清单（许可分级 + 就绪状态）与官方页打开。

use std::process::Command;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::dependencies::RuntimeDependency;
use kairos_core::services::dependencies as dependencies_service;
use serde::Serialize;

/// 单个运行时依赖的状态视图（目录项 + 探测结果）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    #[serde(flatten)]
    pub dependency: RuntimeDependency,
    /// 就绪判定命令是否成功。
    pub ready: bool,
}

#[tauri::command]
pub fn list_runtime_dependencies() -> Vec<DependencyStatus> {
    let catalog = dependencies_service::catalog();
    catalog
        .iter()
        .map(|dep| DependencyStatus {
            dependency: dep.clone(),
            ready: command_exists(&dep.check_command),
        })
        .collect()
}

/// 打开组件的官方下载 / 编译页（引导安装的落地动作，GPL 组件不分发二进制）。
#[tauri::command]
pub fn open_dependency_page(page_url: String) -> Result<()> {
    if !page_url.starts_with("https://") {
        return Err(KairosError::validation("仅允许打开 https 页面。"));
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&page_url)
            .status()
            .map_err(|e| kairos_core::error::KairosError::io(format!("打开浏览器失败：{e}")))?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &page_url])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .status()
            .map_err(|e| kairos_core::error::KairosError::io(format!("打开浏览器失败：{e}")))?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&page_url)
            .status()
            .map_err(|e| kairos_core::error::KairosError::io(format!("打开浏览器失败：{e}")))?;
    }
    Ok(())
}

fn command_exists(command: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where")
            .arg(command)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {command} >/dev/null 2>&1"))
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}
