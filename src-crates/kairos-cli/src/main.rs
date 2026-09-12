//! Kairos 无头 CLI：复用 kairos-core 的纯函数服务，零 Tauri 依赖。
//! 错误一律以 `{code, message}` 结构化输出（与 IPC 契约同形），--json 供脚本消费。

use std::path::Path;
use std::process::Command;

use clap::{Parser, Subcommand};
use kairos_core::error::KairosError;
use kairos_core::models::process::ProcessSettings;
use kairos_core::models::solver::AnalysisStage;
use kairos_core::services::{self, geometry, meshing, moldingfoam, project, results};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "kairos-cli",
    version,
    about = "Kairos 无头 CLI：批处理与自动化冒烟"
)]
struct Cli {
    /// 以 JSON 输出结构化结果（供脚本消费）
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 工程文件管理
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// 体积网格生成
    Mesh {
        #[command(subcommand)]
        action: MeshAction,
    },
    /// 求解（本机 foamRun；虚拟机执行属桌面端能力）
    Solve {
        #[command(subcommand)]
        action: SolveAction,
    },
    /// 结果扫描与读取
    Results {
        #[command(subcommand)]
        action: ResultsAction,
    },
    /// 编排：样例/指定 STL → 网格 → case →（可选）求解 → 结果扫描
    Pipeline {
        #[command(subcommand)]
        action: PipelineAction,
    },
}

#[derive(Subcommand)]
enum PipelineAction {
    /// 单条命令跑完：网格 → case →（可选）求解
    Run {
        /// 使用内置样例方盒（10mm）
        #[arg(long)]
        sample_box: bool,
        /// 指定 STL 路径（与 --sample-box 二选一）
        #[arg(long)]
        stl: Option<String>,
        /// 输出 case 目录
        #[arg(long, default_value = "kairos-case")]
        out_dir: String,
        /// 并行核数
        #[arg(long, default_value_t = 4)]
        cores: u32,
        /// 体素目标尺寸（mm；真实零件按壁厚/算力调整）
        #[arg(long, default_value_t = 1.0)]
        target_size: f64,
        /// 浇口入口 `x,y,z[,半径]`（mm，可重复；不填则回退 z 分带启发式）
        #[arg(long = "gate")]
        gates: Vec<String>,
        /// 实际调用求解器（缺环境时以结构化错误退出）
        #[arg(long)]
        solve: bool,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// 新建工程并写入 <dir>/project.kairos
    New {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = ".")]
        dir: String,
    },
    /// 校验工程文件（解析即验证）
    Open {
        #[arg(long)]
        path: String,
    },
}

#[derive(Subcommand)]
enum MeshAction {
    /// 生成体积网格（voxel = 内置体素引擎；gmsh = T30 落地后可用）
    Generate {
        #[arg(long)]
        stl: String,
        #[arg(long, default_value = "voxel")]
        engine: String,
        #[arg(long, default_value_t = 1.0)]
        target_size: f64,
    },
}

#[derive(Subcommand)]
enum SolveAction {
    /// 在 case 目录执行 decomposePar + foamRun（需本机 OpenFOAM 11+）
    Submit {
        #[arg(long)]
        case_dir: String,
        #[arg(long, default_value_t = 4)]
        cores: u32,
    },
}

#[derive(Subcommand)]
enum ResultsAction {
    /// 列出 case 的时间步目录
    List {
        #[arg(long)]
        case_dir: String,
    },
    /// 读取某时间步的场并输出值
    Dump {
        #[arg(long)]
        case_dir: String,
        #[arg(long)]
        time: String,
        #[arg(long)]
        field: String,
    },
}

fn emit_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(e) => emit_error(&KairosError::internal(format!("JSON 序列化失败：{e}"))),
    }
}

fn emit_error(error: &KairosError) {
    // 结构化错误契约：{code, message}（与 IPC 错误同形），原样穿透。
    eprintln!("{}", serde_json::to_string(error).unwrap_or_default());
}

/// 默认工艺参数（v1 与内置测试夹具一致；正式工艺来自工程文档）。
/// 未给半径时的浇口默认半径（mm）。
const DEFAULT_GATE_RADIUS_MM: f64 = 2.0;

/// 解析 `--gate x,y,z[,半径]`（坐标与网格同单位，mm）。
fn parse_gate(
    spec: &str,
) -> kairos_core::error::Result<kairos_core::services::moldingfoam::GatePortal> {
    let parts: Vec<&str> = spec.split(',').map(str::trim).collect();
    if !(3..=4).contains(&parts.len()) {
        return Err(KairosError::validation(format!(
            "浇口格式应为 x,y,z[,半径]：{spec}"
        )));
    }
    let mut values = Vec::with_capacity(parts.len());
    for part in &parts {
        values.push(
            part.parse::<f64>()
                .map_err(|_| KairosError::validation(format!("浇口坐标/半径不是数字：{part}")))?,
        );
    }
    Ok(kairos_core::services::moldingfoam::GatePortal {
        center: [values[0], values[1], values[2]],
        radius_mm: values.get(3).copied().unwrap_or(DEFAULT_GATE_RADIUS_MM),
    })
}

fn default_process() -> ProcessSettings {
    ProcessSettings {
        melt_temp_c: 230.0,
        mold_temp_c: 40.0,
        ejection_temp_c: 90.0,
        injection_time_s: 1.0,
        vp_switch_volume_percent: 96.0,
        packing_pressure_mpa_curve: vec![(0.0, 60.0), (8.0, 40.0)],
        packing_time_s: 8.0,
        cooling_time_s: 15.0,
        coolant_temp_c: 25.0,
    }
}

fn main() {
    let cli = Cli::parse();
    let json = cli.json;
    let outcome = run(cli.command, json);
    if let Err(error) = outcome {
        if !json {
            emit_error(&error);
        }
        std::process::exit(1);
    }
}

fn run(command: Commands, json: bool) -> kairos_core::error::Result<()> {
    match command {
        Commands::Project { action } => run_project(action, json),
        Commands::Mesh { action } => run_mesh(action, json),
        Commands::Solve { action } => run_solve(action, json),
        Commands::Results { action } => run_results(action, json),
        Commands::Pipeline { action } => match action {
            PipelineAction::Run {
                sample_box,
                stl,
                out_dir,
                cores,
                target_size,
                gates,
                solve,
            } => run_pipeline(
                sample_box,
                stl,
                out_dir,
                cores,
                target_size,
                gates,
                solve,
                json,
            ),
        },
    }
}

fn run_project(action: ProjectAction, json: bool) -> kairos_core::error::Result<()> {
    match action {
        ProjectAction::New { name, dir } => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let doc = project::create(&name, now)?;
            let path = Path::new(&dir).join("project.kairos");
            project::write_atomic(&path, &project::serialize(&doc)?)?;
            if json {
                emit_json(&serde_json::json!({ "path": path, "name": name }));
            } else {
                println!("工程已写入 {}", path.display());
            }
            Ok(())
        }
        ProjectAction::Open { path } => {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| KairosError::io(format!("读取 {} 失败：{e}", path)))?;
            let doc = project::parse(&content)?;
            if json {
                emit_json(&doc);
            } else {
                println!("工程 {} 校验通过（{} 个研究）", path, doc.studies.len());
            }
            Ok(())
        }
    }
}

fn run_mesh(action: MeshAction, json: bool) -> kairos_core::error::Result<()> {
    match action {
        MeshAction::Generate {
            stl,
            engine,
            target_size,
        } => {
            let mesh = geometry::parse_stl_file(Path::new(&stl))?;
            let volume = if engine == "gmsh" {
                // 子进程编排复用 core 服务（与桌面命令层同一路径）。
                let out_msh = Path::new(&stl).with_extension("msh");
                kairos_core::services::gmsh::tetrahedralize(
                    Path::new("gmsh"),
                    Path::new(&stl),
                    &out_msh,
                    Some(target_size),
                )?
            } else {
                let params = meshing::VolumeMeshParams {
                    target_size,
                    refinement: None,
                };
                params.validate()?;
                meshing::generate(&mesh, &params)?
            };
            if json {
                emit_json(&serde_json::json!({
                    "nodes": volume.nodes.len(),
                    "tets": volume.tets.len(),
                }));
            } else {
                println!(
                    "网格完成（{engine}）：{} 节点 / {} 四面体",
                    volume.nodes.len(),
                    volume.tets.len()
                );
            }
            Ok(())
        }
    }
}

fn run_solve(action: SolveAction, json: bool) -> kairos_core::error::Result<()> {
    match action {
        SolveAction::Submit { case_dir, cores } => {
            let dir = Path::new(&case_dir);
            if !dir.join("system/controlDict").exists() {
                return Err(KairosError::not_found(
                    "case 目录缺少 system/controlDict，请先生成 case。",
                ));
            }
            let safe_dir = case_dir.replace('\'', "'\\''");
            // 求解输出落 log.foamRun：管道后接 tail 会让退出码被 tail 覆盖，
            // 求解失败反被报成成功，故先判码再截取尾部日志。
            let solve = kairos_core::services::moldingfoam::solve_command(cores);
            let status = Command::new("bash")
                .arg("-lc")
                .arg(format!(
                    "cd '{safe_dir}' && {solve} > log.foamRun 2>&1; status=$?; tail -20 log.foamRun; exit $status"
                ))
                .status()
                .map_err(|e| {
                    KairosError::io(format!("求解器启动失败（本机需 OpenFOAM 11+）：{e}"))
                })?;
            if !status.success() {
                return Err(KairosError::io("求解失败，详见上方求解器输出。"));
            }
            if json {
                emit_json(
                    &serde_json::json!({ "caseDir": case_dir, "cores": cores, "status": "done" }),
                );
            } else {
                println!("求解完成：{case_dir}");
            }
            Ok(())
        }
    }
}

fn run_results(action: ResultsAction, json: bool) -> kairos_core::error::Result<()> {
    match action {
        ResultsAction::List { case_dir } => {
            let catalog = results::scan_times(Path::new(&case_dir))?;
            if json {
                emit_json(&catalog);
            } else {
                for time in &catalog.times {
                    println!("{} {}", time.dir_name, time.time_s);
                }
            }
            Ok(())
        }
        ResultsAction::Dump {
            case_dir,
            time,
            field,
        } => {
            let field = results::read_field(Path::new(&case_dir), &time, &field)?;
            if json {
                emit_json(&field);
            } else {
                println!("{:?}", field.values);
            }
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_pipeline(
    sample_box: bool,
    stl: Option<String>,
    out_dir: String,
    cores: u32,
    target_size: f64,
    gate_specs: Vec<String>,
    solve: bool,
    json: bool,
) -> kairos_core::error::Result<()> {
    let out = Path::new(&out_dir);
    // 1. 几何
    let mesh_tri = match (sample_box, stl) {
        (true, _) => Ok(kairos_core::models::geometry::TriangleMesh::sample_box(
            10.0,
        )),
        (false, Some(path)) => geometry::parse_stl_file(Path::new(&path)),
        (false, None) => Err(KairosError::validation(
            "必须指定 --sample-box 或 --stl <路径>。".to_string(),
        )),
    }?;
    // 2. 网格
    let volume = meshing::generate(
        &mesh_tri,
        &meshing::VolumeMeshParams {
            refinement: None,
            target_size,
        },
    )?;
    // 3. case（首个内置材料 + 默认工艺 + 填充阶段）
    let material = services::material::builtin_materials()[0].clone();
    let gates = gate_specs
        .iter()
        .map(|spec| parse_gate(spec))
        .collect::<kairos_core::error::Result<Vec<_>>>()?;
    moldingfoam::generate_case(
        out,
        &volume,
        &material,
        &default_process(),
        &AnalysisStage::Fill,
        cores as usize,
        &gates,
    )?;
    if json {
        emit_json(&serde_json::json!({
            "caseDir": out_dir,
            "nodes": volume.nodes.len(),
            "tets": volume.tets.len(),
            "solved": solve,
        }));
    } else {
        println!(
            "case 已生成：{}（{} 节点 / {} 四面体）",
            out_dir,
            volume.nodes.len(),
            volume.tets.len()
        );
    }
    // 4. 求解（可选；缺 OpenFOAM 环境时输出结构化错误，供脚本捕获）
    if solve {
        return run_solve(
            SolveAction::Submit {
                case_dir: out_dir,
                cores,
            },
            json,
        );
    }
    Ok(())
}
