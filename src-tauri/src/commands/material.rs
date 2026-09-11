//! 材料命令：内置材料、自定义材料库（应用数据目录持久化）的增删查与导入导出。
//! 校验与合并规则在 core（services::material）。

use std::fs;
use std::path::PathBuf;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::material::Material;
use kairos_core::services::material as material_service;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn list_builtin_materials() -> Result<Vec<Material>> {
    material_service::builtin_materials()
}

#[tauri::command]
pub fn list_custom_materials(app: AppHandle) -> Vec<Material> {
    read_custom(&app)
}

/// 从 JSON / CSV 文件导入材料：校验、按 id 合并入库，返回合并后的完整材料库。
#[tauri::command]
pub fn import_custom_materials(app: AppHandle, path: String) -> Result<Vec<Material>> {
    let content =
        fs::read_to_string(&path).map_err(|e| KairosError::io(format!("读取材料文件失败：{e}")))?;
    let incoming = if path.to_ascii_lowercase().ends_with(".csv") {
        material_service::parse_custom_csv(&content)?
    } else {
        material_service::parse_custom(&content)?
    };
    let merged = material_service::merge_custom(read_custom(&app), incoming);
    material_service::write_custom_file(&custom_file(&app)?, &merged)?;
    Ok(merged)
}

/// 新增或更新一个自定义材料（按 id 合并），返回合并后的完整材料库。
#[tauri::command]
pub fn upsert_custom_material(app: AppHandle, material: Material) -> Result<Vec<Material>> {
    let merged = material_service::merge_custom(read_custom(&app), vec![material]);
    material_service::write_custom_file(&custom_file(&app)?, &merged)?;
    Ok(merged)
}

#[tauri::command]
pub fn delete_custom_material(app: AppHandle, id: String) -> Result<Vec<Material>> {
    let remaining: Vec<Material> = read_custom(&app)
        .into_iter()
        .filter(|material| material.id != id)
        .collect();
    material_service::write_custom_file(&custom_file(&app)?, &remaining)?;
    Ok(remaining)
}

/// 把给定材料集合导出为 JSON 文件（另存为对话框选定的路径）。
#[tauri::command]
pub fn export_materials_to_file(path: String, materials: Vec<Material>) -> Result<()> {
    material_service::write_custom_file(std::path::Path::new(&path), &materials)
}

/// 读取自定义材料库；路径不可得或文件损坏时按空库处理（非关键数据）。
fn read_custom(app: &AppHandle) -> Vec<Material> {
    custom_file(app)
        .ok()
        .map(|file| material_service::read_custom_file(&file))
        .unwrap_or_default()
}

fn custom_file(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| KairosError::io(format!("无法定位应用数据目录：{e}")))?;
    Ok(dir.join("custom-materials.json"))
}
