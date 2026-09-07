//! 求解器命令：环境探测、case 生成与子进程运行（GPL 隔离：仅子进程 + 文件交换）。
//! 见 ai-docs/decisions/openfoam-gpl-compliance.md。

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::material::Material;
use kairos_core::models::mesh::VolumeMesh;
use kairos_core::models::process::ProcessSettings;
use kairos_core::models::solver::AnalysisStage;
use kairos_core::services::openfoam;
use kairos_core::services::project::new_id;
use serde::Serialize;
use tauri::State;
use tauri::ipc::Channel;

use crate::commands::geometry::GeometryStore;

/// 运行中的求解进程表（run_id → bash 子进程），供取消命令终止。
#[derive(Default)]
pub struct SolverRuns(pub Arc<Mutex<HashMap<String, Child>>>);

/// OpenFOAM 环境探测结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentCheck {
    /// OpenFOAM 工具链（以 blockMesh 为代表）是否可用。
    pub openfoam: bool,
    /// openInjMoldSim 求解器是否可用。
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
    let (openfoam, solver) = (check("blockMesh"), check("openInjMoldSim"));
    let hint = if openfoam && solver {
        "OpenFOAM 与 openInjMoldSim 已就绪。".into()
    } else if !openfoam {
        "未检测到 OpenFOAM 7（.org 版）。请安装后重试：openfoam.org".into()
    } else {
        "已检测到 OpenFOAM，但缺少 openInjMoldSim 求解器。请编译并加入 PATH。".into()
    };
    EnvironmentCheck {
        openfoam,
        solver,
        hint,
    }
}

/// 由已导入几何生成 OpenFOAM case（polyMesh + 场 + 字典）。
#[tauri::command]
pub fn generate_openfoam_case(
    store: State<'_, GeometryStore>,
    geometry_id: String,
    case_dir: String,
    material: Material,
    process: ProcessSettings,
    stage: AnalysisStage,
    cores: u32,
) -> Result<String> {
    let mut sessions = store.0.lock().unwrap();
    let session = sessions.get_mut(&geometry_id).ok_or_else(|| {
        kairos_core::error::KairosError::not_found(format!("几何不存在：{geometry_id}"))
    })?;
    let volume_mesh: &VolumeMesh = session
        .volume
        .as_ref()
        .ok_or_else(|| KairosError::validation("该几何尚未生成体积网格，请先执行网格划分。"))?;
    let cores = cores.clamp(1, 64) as usize;
    openfoam::generate_case(
        std::path::Path::new(&case_dir),
        volume_mesh,
        &material,
        &process,
        &stage,
        cores,
    )?;
    Ok(case_dir)
}

fn spawn_run_script(case_dir: &str) -> Result<Child> {
    let script = format!("cd '{case_dir}' && decomposePar -force && openInjMoldSim -parallel");
    let mut command = Command::new("bash");
    command
        .arg("-lc")
        .arg(&script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // unix 下让 bash 成为独立进程组长：取消时可整组终止，避免孤儿求解进程。
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
        .spawn()
        .map_err(|e| KairosError::io(format!("启动求解进程失败：{e}")))
}

/// 启动求解：立即返回 run_id，stdout 进度经 Channel 流式回传（含 __TIME__ 前缀的时间步行）。
#[tauri::command]
pub fn start_openfoam_run(
    runs: State<'_, SolverRuns>,
    case_dir: String,
    progress: Channel<String>,
) -> Result<String> {
    let run_id = new_id("run");
    let mut child = spawn_run_script(&case_dir)?;
    let stdout = child.stdout.take();
    runs.0.lock().unwrap().insert(run_id.clone(), child);

    let runs_map = Arc::clone(&runs.0);
    let run_id_for_thread = run_id.clone();
    thread::spawn(move || {
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(std::result::Result::ok) {
                if let Some(time_s) = openfoam::parse_time_line(&line) {
                    let _ = progress.send(format!("__TIME__{time_s}"));
                }
                let _ = progress.send(line);
            }
        }
        runs_map.lock().unwrap().remove(&run_id_for_thread);
    });

    Ok(run_id)
}

/// 取消运行中的求解：unix 下整组终止（bash 以独立进程组启动），并回收子进程。
#[tauri::command]
pub fn cancel_openfoam_run(runs: State<'_, SolverRuns>, run_id: String) -> Result<()> {
    let child = runs.0.lock().unwrap().remove(&run_id);
    if let Some(mut child) = child {
        let pid = child.id();
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-9", &format!("-{pid}")])
                .status();
        }
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}
