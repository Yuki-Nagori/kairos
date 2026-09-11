//! 项目命令：创建、打开/保存工程文件、最近项目。
//! 工程文档由前端状态持有；本层只负责校验落盘与最近项目登记。
//! 最近项目持久化在应用数据目录，序列化与登记规则在 core（services::project）。

use std::fs;
use std::path::PathBuf;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::project::{Project, RecentProject};
use kairos_core::services::project as project_service;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn create_project(name: String) -> Result<Project> {
    project_service::create(&name, project_service::now_ms())
}

#[tauri::command]
pub fn save_project_file(app: AppHandle, path: String, project: Project) -> Result<()> {
    let content = project_service::serialize(&project)?;
    project_service::write_atomic(std::path::Path::new(&path), &content)?;
    let recents = project_service::record_recent(
        read_recents(&app),
        path,
        project.name,
        project_service::now_ms(),
    );
    write_recents(&app, &recents)
}

#[tauri::command]
pub fn load_project_file(app: AppHandle, path: String) -> Result<Project> {
    let content =
        fs::read_to_string(&path).map_err(|e| KairosError::io(format!("读取工程文件失败：{e}")))?;
    let project = project_service::parse(&content)?;
    let recents = project_service::record_recent(
        read_recents(&app),
        path,
        project.name.clone(),
        project_service::now_ms(),
    );
    write_recents(&app, &recents)?;
    Ok(project)
}

#[tauri::command]
pub fn list_recent_projects(app: AppHandle) -> Vec<RecentProject> {
    read_recents(&app)
}

/// 最近项目以应用数据目录下的 JSON 为准（跨会话）；读取失败按空处理。
fn read_recents(app: &AppHandle) -> Vec<RecentProject> {
    recents_file(app)
        .ok()
        .and_then(|file| fs::read_to_string(file).ok())
        .map(|content| project_service::parse_recents(&content))
        .unwrap_or_default()
}

fn write_recents(app: &AppHandle, recents: &[RecentProject]) -> Result<()> {
    let file = recents_file(app)?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| KairosError::io(format!("创建数据目录失败：{e}")))?;
    }
    project_service::write_atomic(&file, &project_service::serialize_recents(recents))
}

fn recents_file(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| KairosError::io(format!("无法定位应用数据目录：{e}")))?;
    Ok(dir.join("recent-projects.json"))
}

/// 新研究的默认 case 目录（应用数据目录下，按研究 ID 隔离）。
#[tauri::command]
pub fn default_case_dir(app: AppHandle, study_id: String) -> Result<String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| KairosError::io(format!("无法定位应用数据目录：{e}")))?;
    Ok(dir
        .join("cases")
        .join(study_id)
        .to_string_lossy()
        .to_string())
}
