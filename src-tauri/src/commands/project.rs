//! 项目命令：创建、打开/保存工程文件、最近项目。
//! 工程文档由前端状态持有；本层只负责校验落盘与最近项目登记。
//! 最近项目持久化在应用数据目录，序列化与登记规则在 core（services::project）。

use std::fs;
use std::path::PathBuf;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::project::{GeometryRef, Project, RecentProject};
use kairos_core::services::paths;
use kairos_core::services::project as project_service;
use kairos_core::services::workspace;
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
pub fn list_recent_projects(app: AppHandle) -> Result<Vec<RecentProject>> {
    Ok(read_recents(&app))
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

/// 用户文档目录：由平台 API 解析（各语言下目录名不同，如 Documents / 文档 / Dokumente），
/// 不硬编码 `~/Documents`。
pub(crate) fn documents_dir(app: &AppHandle) -> Result<PathBuf> {
    app.path()
        .document_dir()
        .map_err(|e| KairosError::io(format!("无法定位用户文档目录：{e}")))
}

/// 默认工作区根：`<文档目录>/kairos`（新建项目对话框的初值；只计算不创建）。
#[tauri::command]
pub fn default_workspace_path(app: AppHandle) -> Result<String> {
    Ok(workspace::default_root(&documents_dir(&app)?)
        .to_string_lossy()
        .to_string())
}

/// 新工程路径：`<工作区根>/<工程名>/<工程名>.kairos`（工作区留空时用默认根）。
/// 只计算路径并创建工程目录，不写文件（保存由 save_project_file 完成）。
#[tauri::command]
pub fn project_path(app: AppHandle, workspace: String, project_name: String) -> Result<String> {
    let trimmed = workspace.trim();
    let root = if trimmed.is_empty() {
        workspace::default_root(&documents_dir(&app)?)
    } else {
        PathBuf::from(trimmed)
    };
    let dir = workspace::project_dir(&root, &project_name);
    fs::create_dir_all(&dir).map_err(|e| KairosError::io(format!("创建工程目录失败：{e}")))?;
    Ok(workspace::project_file_path(&root, &project_name)
        .to_string_lossy()
        .to_string())
}

/// 工程的工作区根：工程按 `<工作区根>/<工程名>/<同名>.kairos` 布局存放时为该根，
/// 否则 None（散装工程，数据留在应用数据目录）。
#[tauri::command]
pub fn workspace_root_of(project_path: String) -> Result<Option<String>> {
    Ok(
        workspace::workspace_root(std::path::Path::new(&project_path))
            .map(|root| root.to_string_lossy().to_string()),
    )
}

/// 把导入的几何归档进工作区 `geometry/`：原样拷贝（保留来源文件名，重名加 id 前缀）。
#[tauri::command]
pub fn archive_workspace_geometry(
    project_path: String,
    geometry_id: String,
    source_path: String,
) -> Result<GeometryRef> {
    let root = workspace::workspace_root(std::path::Path::new(&project_path)).ok_or_else(|| {
        KairosError::validation("当前工程不在工作区目录中，无法归档几何（请先另存为工程目录）。")
    })?;
    let target_dir = workspace::geometry_dir(&root);
    fs::create_dir_all(&target_dir)
        .map_err(|e| KairosError::io(format!("创建几何目录失败：{e}")))?;
    let source = std::path::Path::new(&source_path);
    let source_name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "geometry.stl".to_string());
    let taken: Vec<String> = fs::read_dir(&target_dir)
        .map_err(|e| KairosError::io(format!("读取几何目录失败：{e}")))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    let file_name = workspace::archive_file_name(&source_name, &geometry_id, &taken);
    let target = target_dir.join(&file_name);
    fs::copy(source, &target).map_err(|e| KairosError::io(format!("归档几何失败：{e}")))?;
    Ok(GeometryRef {
        id: geometry_id,
        relative_path: workspace::geometry_relative(&file_name),
        file_name,
    })
}

/// 新方案的默认 case 目录：工作区内 `<工作区>/cases/<方案 id>`，
/// 散装工程回退应用数据目录（旧行为）。
#[tauri::command]
pub fn default_case_dir(
    app: AppHandle,
    project_path: Option<String>,
    study_id: String,
) -> Result<String> {
    let root = project_path
        .as_deref()
        .and_then(|path| workspace::workspace_root(std::path::Path::new(path)));
    match root {
        Some(root) => Ok(workspace::cases_dir(&root, &study_id)
            .to_string_lossy()
            .to_string()),
        None => {
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
    }
}

/// 把报告写进工作区 `reports/`（返回写入路径）；散装工程明确报错（由前端走下载）。
/// 文件名做安全清洗、扩展名固定 .html，避免覆盖工作区内的其它文件。
#[tauri::command]
pub fn save_report_to_workspace(
    project_path: String,
    file_name: String,
    content: String,
) -> Result<String> {
    let root = workspace::workspace_root(std::path::Path::new(&project_path))
        .ok_or_else(|| KairosError::validation("当前工程不在工作区目录中，报告将作为文件下载。"))?;
    let dir = workspace::reports_dir(&root);
    fs::create_dir_all(&dir).map_err(|e| KairosError::io(format!("创建报告目录失败：{e}")))?;
    // 文件名清洗与扩展名都走库：主干取 `Path::file_stem`，扩展名用 `with_extension`
    let stem = paths::file_stem(&file_name, "report");
    let safe = paths::path_segment(&stem, "report");
    let path = dir.join(safe).with_extension("html");
    fs::write(&path, content).map_err(|e| KairosError::io(format!("写入报告失败：{e}")))?;
    Ok(path.to_string_lossy().to_string())
}

/// 报告导出页面的载荷（与前端 `ReportSlide` 镜像）。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSlidePayload {
    pub title: String,
    pub bullets: Vec<String>,
    /// 快照 dataURL（可选；缺省为空，旧调用方不受影响）。
    #[serde(default)]
    pub images: Vec<String>,
}

/// 把报告写成 PPTX 落进工作区 `reports/`（返回写入路径）；散装工程明确报错。
///
/// 生成在 core（`services::report_pptx`），命令层只做落盘与文件名清洗——
/// 与 HTML 报告同一口径，重复导出互不覆盖（扩展名不同）。
#[tauri::command]
pub fn save_report_pptx_to_workspace(
    project_path: String,
    file_name: String,
    title: String,
    slides: Vec<ReportSlidePayload>,
) -> Result<String> {
    let root = workspace::workspace_root(std::path::Path::new(&project_path))
        .ok_or_else(|| KairosError::validation("当前工程不在工作区目录中，报告将作为文件下载。"))?;
    let deck_slides: Vec<kairos_core::services::report_pptx::ReportSlide> = slides
        .into_iter()
        .map(|slide| {
            let images = slide
                .images
                .iter()
                .map(|data_url| kairos_core::services::report_pptx::image_from_data_url(data_url))
                .collect::<Result<Vec<_>>>()?;
            Ok(kairos_core::services::report_pptx::ReportSlide {
                title: slide.title,
                bullets: slide.bullets,
                images,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let bytes = kairos_core::services::report_pptx::build_report_deck(&title, &deck_slides)?;
    let dir = workspace::reports_dir(&root);
    fs::create_dir_all(&dir).map_err(|e| KairosError::io(format!("创建报告目录失败：{e}")))?;
    let stem = paths::file_stem(&file_name, "report");
    let safe = paths::path_segment(&stem, "report");
    let path = dir.join(safe).with_extension("pptx");
    fs::write(&path, bytes).map_err(|e| KairosError::io(format!("写入报告失败：{e}")))?;
    Ok(path.to_string_lossy().to_string())
}
