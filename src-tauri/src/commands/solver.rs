//! 求解器命令：环境探测与 OpenFOAM case 生成（GPL 隔离：仅子进程 + 文件交换）。
//! 作业的启动 / 取消 / 列表由 T10 的调度器（commands::jobs）负责。
//! 见 ai-docs/decisions/openfoam-gpl-compliance.md。

use std::process::Command;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::material::Material;
use kairos_core::models::mesh::VolumeMesh;
use kairos_core::models::process::ProcessSettings;
use kairos_core::models::solver::AnalysisStage;
use kairos_core::services::openfoam;
use serde::Serialize;
use tauri::State;

use crate::commands::geometry::GeometryStore;

/// OpenFOAM 环境探测结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentCheck {
    /// OpenFOAM 工具链（以 blockMesh 为代表）是否可用。
    pub openfoam: bool,
    /// foamRun 模块化运行器是否可用（OpenFOAM 11+ 才有；求解模块由
    /// case 的 controlDict 指定，无需第三方求解器二进制）。
    pub solver: bool,
    /// 面向用户的就绪状态提示。
    pub hint: String,
}

#[tauri::command]
pub fn probe_openfoam() -> EnvironmentCheck {
    let check = |command: &str| -> bool {
        Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {command} >/dev/null 2>&1"))
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    };
    let (openfoam, solver) = (check("blockMesh"), check("foamRun"));
    let hint = if openfoam && solver {
        "OpenFOAM 已就绪（foamRun 模块化求解器可用）。".into()
    } else if !openfoam {
        "未检测到 OpenFOAM（.org 版，需 11+，推荐 14）。可在依赖面板应用内下载，或参考 openfoam.org。"
            .into()
    } else {
        "OpenFOAM 版本过旧：缺少 foamRun 模块化运行器，请升级到 11+（推荐 14）。".into()
    };
    EnvironmentCheck {
        openfoam,
        solver,
        hint,
    }
}

/// 由已导入几何生成 OpenFOAM case（polyMesh + 场 + 字典）。
#[tauri::command]
pub async fn generate_openfoam_case(
    store: State<'_, GeometryStore>,
    geometry_id: String,
    case_dir: String,
    material: Material,
    process: ProcessSettings,
    stage: AnalysisStage,
    cores: u32,
) -> Result<String> {
    let cores = cores.clamp(1, 64) as usize;
    // 锁只用于取网格快照；polyMesh 与场文件的写入在锁外、阻塞线程池中进行。
    let volume_mesh: VolumeMesh = {
        let sessions = store.lock();
        let session = sessions.get(&geometry_id).ok_or_else(|| {
            kairos_core::error::KairosError::not_found(format!("几何不存在：{geometry_id}"))
        })?;
        session
            .volume
            .as_ref()
            .ok_or_else(|| KairosError::validation("该几何尚未生成体积网格，请先执行网格划分。"))?
            .clone()
    };
    tauri::async_runtime::spawn_blocking(move || {
        openfoam::generate_case(
            std::path::Path::new(&case_dir),
            &volume_mesh,
            &material,
            &process,
            &stage,
            cores,
        )?;
        Ok(case_dir)
    })
    .await
    .map_err(|e| KairosError::internal(format!("case 生成任务失败：{e}")))?
}
