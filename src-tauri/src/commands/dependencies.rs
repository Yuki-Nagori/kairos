//! 依赖命令：运行时依赖清单（许可分级 + 就绪状态）与官方页打开。

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// creation_flags（CREATE_NO_WINDOW）来自 Windows 专属 trait；cfg 裁剪外的平台
// 看不到这段代码，import 必须同样带 cfg，否则非 Windows 编译报未使用。
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::dependencies::RuntimeDependency;
use kairos_core::services::dependencies as dependencies_service;
use serde::Serialize;
use tauri::AppHandle;
use tauri::ipc::Channel;

use super::downloads;

/// 单个运行时依赖的状态视图（目录项 + 探测结果）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    #[serde(flatten)]
    pub dependency: RuntimeDependency,
    /// PATH 中的就绪判定命令是否成功。
    pub ready: bool,
    /// 应用内受管目录（downloads/<id>/）中检测到可执行副本
    /// （下载 + 自动解压后即可用，无需手动加 PATH）。
    pub managed_ready: bool,
}

#[tauri::command]
pub fn list_runtime_dependencies(app: AppHandle) -> Vec<DependencyStatus> {
    let downloads_dir = downloads::downloads_dir(&app).ok();
    dependencies_service::catalog()
        .iter()
        .map(|dep| {
            // PATH 命中或应用内受管副本命中，都算就绪。
            let managed_ready = downloads_dir
                .as_ref()
                .and_then(|dir| find_managed_executable(&dir.join(&dep.id), &dep.check_command))
                .is_some();
            DependencyStatus {
                dependency: dep.clone(),
                ready: managed_ready || command_exists(&dep.check_command),
                managed_ready,
            }
        })
        .collect()
}

/// 在受管组件目录中查找可执行文件（gmsh SDK 解压后位于 bin/ 子目录）。
/// Windows 匹配 <binary>.exe；Unix 额外要求可执行位。
fn find_managed_executable(dir: &Path, binary: &str) -> Option<PathBuf> {
    find_executable(dir, binary, 0)
}

fn find_executable(dir: &Path, binary: &str, depth: u8) -> Option<PathBuf> {
    if depth > 4 {
        return None;
    }
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_executable(&path, binary, depth + 1) {
                return Some(found);
            }
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let stem_matches = name == binary || name == format!("{binary}.exe");
        #[cfg(unix)]
        let executable = {
            use std::os::unix::fs::PermissionsExt;
            entry
                .metadata()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        };
        #[cfg(not(unix))]
        let executable = true;
        if stem_matches && executable {
            return Some(path);
        }
    }
    None
}

/// 定位解压出的内容目录（压缩包内通常有单一顶层目录）。
fn locate_extracted(component_dir: &Path) -> Result<PathBuf> {
    let entries = fs::read_dir(component_dir)
        .map_err(|e| KairosError::io(format!("读取组件目录失败：{e}")))?;
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            return Ok(entry.path());
        }
    }
    Err(KairosError::not_found(format!(
        "组件目录不存在或为空：{}",
        component_dir.display()
    )))
}

/// 组件编译脚本：目前仅 openfoam 全量构建（注塑求解器将由 OpenFOAM-14
/// fork 以求解模块形式提供，随其本体一起编译，见 ai-docs/tasks/T34）。
fn build_compile_script(component_id: &str, src: &Path) -> Result<String> {
    let sh_quote = |path: &Path| format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"));
    let src_q = sh_quote(src);
    match component_id {
        "openfoam" => Ok(format!(
            "cd {src_q} && source ./etc/bashrc && ./Allwmake 2>&1"
        )),
        other => Err(KairosError::validation(format!(
            "组件 {other} 没有编译流程"
        ))),
    }
}

/// 一键编译：下载解压完成后构建源码组件（OpenFOAM 全量构建 30–60 分钟级），
/// 日志行经 Channel 流式回传。
#[tauri::command]
pub async fn compile_dependency(
    app: AppHandle,
    component_id: String,
    progress: Channel<String>,
) -> Result<String> {
    let downloads_dir = downloads::downloads_dir(&app)?;
    let src = locate_extracted(&downloads_dir.join(&component_id))?;
    let script = build_compile_script(&component_id, &src)?;

    tauri::async_runtime::spawn_blocking(move || {
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg(format!("{script} 2>&1"))
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|e| KairosError::io(format!("编译启动失败：{e}")))?;
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(std::result::Result::ok) {
                let _ = progress.send(line);
            }
        }
        let status = child
            .wait()
            .map_err(|e| KairosError::io(format!("编译进程等待失败：{e}")))?;
        if status.success() {
            Ok("编译完成".into())
        } else {
            Err(KairosError::io(format!(
                "编译失败（退出码 {:?}），详见上方日志",
                status.code()
            )))
        }
    })
    .await
    .map_err(|e| KairosError::internal(format!("编译任务失败：{e}")))?
}

/// 打开组件的官方下载 / 编译页（引导安装的落地动作，GPL 组件不分发二进制）。
#[tauri::command]
pub fn open_dependency_page(page_url: String) -> Result<()> {
    // 双重校验：https 前缀 + 必须是依赖目录里登记过的页面（防任意 URL 打开）。
    if !page_url.starts_with("https://") {
        return Err(KairosError::validation("仅允许打开 https 页面。"));
    }
    let registered = dependencies_service::catalog()
        .iter()
        .any(|dep| dep.page_url == page_url);
    if !registered {
        return Err(KairosError::validation("页面不在依赖目录内。"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_managed_executable_detects_sdk_layout() {
        let dir = std::env::temp_dir().join(format!("kairos-mgd-{}", std::process::id()));
        let bin_dir = dir.join("gmsh").join("gmsh-4.15.2-sdk").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join("gmsh");
        std::fs::write(&bin, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        assert!(find_managed_executable(&dir.join("gmsh"), "gmsh").is_some());
        assert!(find_managed_executable(&dir.join("other"), "gmsh").is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
