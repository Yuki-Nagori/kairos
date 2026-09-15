//! 求解器命令：环境探测与求解 case 生成（GPL 隔离：仅子进程 + 文件交换）。
//! 作业的启动 / 取消 / 列表由调度器（commands::jobs）负责。
//! 见 ai-docs/decisions/openfoam-gpl-compliance.md。

use std::process::Command;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::material::Material;
use kairos_core::models::mesh::VolumeMesh;
use kairos_core::models::process::ProcessSettings;
use kairos_core::models::runners::{CoolingChannel, RunnerElement};
use kairos_core::models::solver::AnalysisStage;
use kairos_core::services::dependencies as dependencies_service;
use kairos_core::services::moldingfoam;
use serde::Serialize;
use tauri::State;

use crate::commands::geometry::GeometryStore;

/// 求解环境探测结果（环境 = moldingFoam bundle：OpenFOAM-14 环境树 + 注塑模块）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentCheck {
    /// 环境工具链（以 blockMesh 为代表）是否可用。
    pub moldingfoam: bool,
    /// foamRun 模块化运行器是否可用（OpenFOAM 11+ 才有；求解模块由
    /// case 的 controlDict 指定，无需第三方求解器二进制）。
    pub solver: bool,
    /// 面向用户的就绪状态提示。
    pub hint: String,
}

/// case 生成结果：case 目录 + 浇口入口口径回显（有效面积 / 等效直径 / 偏差）
/// 与不可表达告警。面板据此展示「请求 vs 实际」，提示不必再猜网格是否表达了浇口。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseOutcome {
    pub case_dir: String,
    /// inlet patch 的实际面积（m²）与等效圆直径（mm）。
    pub inlet_area_m2: f64,
    pub inlet_equivalent_diameter_mm: f64,
    /// 逐浇口回显（请求半径 vs 实际面积、面数、面积比、是否可表达）。
    pub gates: Vec<moldingfoam::GateInlet>,
    /// 不可表达等告警（空 = 通过）。
    pub warnings: Vec<String>,
}

#[tauri::command]
pub fn probe_moldingfoam() -> Result<EnvironmentCheck> {
    let check = |command: &str| -> bool {
        Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {command} >/dev/null 2>&1"))
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    };
    let (moldingfoam, solver) = (check("blockMesh"), check("foamRun"));
    // 就绪判定与用户文案在 core（唯一出处），这里只负责探。
    let hint = dependencies_service::environment_hint(moldingfoam, solver);
    Ok(EnvironmentCheck {
        moldingfoam,
        solver,
        hint,
    })
}

/// 由已导入几何生成求解 case（polyMesh + 场 + 字典）。
/// `runner_elements` 来自方案的模具网络：其中的浇口单元决定 inlet patch
/// （不传则回退 z 分带启发式）。
// IPC 命令保持平铺入参（前端载荷稳定）：参数个数超 clippy 默认阈值属预期。
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn generate_moldingfoam_case(
    store: State<'_, GeometryStore>,
    geometry_id: String,
    case_dir: String,
    material: Material,
    process: ProcessSettings,
    stage: AnalysisStage,
    cores: u32,
    runner_elements: Vec<RunnerElement>,
    cooling_channels: Vec<CoolingChannel>,
) -> Result<CaseOutcome> {
    // 核数夹取在 core（面板没有上界，这里是唯一把关处的调用点）。
    kairos_core::services::jobs::validate_cores(cores)?;
    let cores = cores as usize;
    let gates = moldingfoam::gate_portals(&runner_elements);
    // 锁只用于取网格快照；polyMesh 与场文件的写入在锁外、阻塞线程池中进行。
    let volume_mesh: std::sync::Arc<VolumeMesh> = {
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
        let report = moldingfoam::generate_case(
            std::path::Path::new(&case_dir),
            &moldingfoam::CaseInputs {
                mesh: &volume_mesh,
                material: &material,
                process: &process,
                stage: &stage,
                cores,
                gates: &gates,
                channels: &cooling_channels,
            },
        )?;
        Ok(CaseOutcome {
            case_dir,
            inlet_area_m2: report.inlet_area_m2,
            inlet_equivalent_diameter_mm: report.inlet_equivalent_diameter_mm(),
            gates: report.gates,
            warnings: report.warnings,
        })
    })
    .await
    .map_err(|e| KairosError::internal(format!("case 生成任务失败：{e}")))?
}
