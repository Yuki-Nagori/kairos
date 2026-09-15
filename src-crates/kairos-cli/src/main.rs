//! Kairos 无头 CLI：复用 kairos-core 的纯函数服务，零 Tauri 依赖。
//! 错误一律以 `{code, message}` 结构化输出（与 IPC 契约同形），--json 供脚本消费。

use std::fs;
use std::path::Path;
use std::process::Command;

use clap::{Parser, Subcommand};
use kairos_core::error::KairosError;
use kairos_core::models::process::ProcessSettings;
use kairos_core::models::solver::AnalysisStage;
use kairos_core::services::{
    self, doe, geometry, material_curve, meshing, moldingfoam, optimize, project, results,
    vm as vm_logic, vm_run,
};
use kairos_core::utils::shell;
use kairos_core::utils::time::now_ms;
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
    /// 试验设计：参数矩阵与批次汇总表（求解执行循环属下一批）
    Doe {
        #[command(subcommand)]
        action: DoeAction,
    },
    /// 工艺寻优候选规划（只生成参数，不启动求解器）
    Optimize {
        #[command(subcommand)]
        action: OptimizeAction,
    },
    /// 材料资产校验与摘要
    Material {
        #[command(subcommand)]
        action: MaterialAction,
    },
}

#[derive(Subcommand)]
enum MaterialAction {
    /// 校验 JSON/CSV 自定义材料文件并输出摘要
    Validate {
        #[arg(long)]
        path: String,
    },
    /// 以模板材料和曲线执行第一阶段确定性参数回填
    Fit {
        #[arg(long)]
        template: String,
        #[arg(long)]
        viscosity: String,
        #[arg(long)]
        pvt: String,
        #[arg(long)]
        output: String,
    },
}

#[derive(Subcommand)]
enum OptimizeAction {
    /// 根据范围生成粗搜候选点
    Plan {
        /// 因子，形如 `熔体温度=200:240:3`（最小值:最大值:层数）
        #[arg(long = "factor", required = true)]
        factors: Vec<String>,
        /// 目标：fill-time 或 injection-pressure
        #[arg(long, default_value = "fill-time")]
        objective: String,
        /// 总评估预算
        #[arg(long, default_value_t = 64)]
        budget: usize,
    },
}

#[derive(Subcommand)]
enum DoeAction {
    /// 生成参数矩阵并落盘汇总表骨架（所有运行为 pending）
    Matrix {
        /// 因子，形如 `熔体温度=200,210,220`（可重复；正交表 L9 要求每因子 3 水平）
        #[arg(long = "factor", required = true)]
        factors: Vec<String>,
        /// 编排方式：orthogonal（L9）或 full（全因子）
        #[arg(long, default_value = "orthogonal")]
        plan: String,
        /// 输出目录：汇总表落 `<dir>/doe/<批次名>/`；不填只打印
        #[arg(long)]
        out_dir: Option<String>,
        /// 批次名（缺省 = 因子名以短横连接）
        #[arg(long)]
        batch: Option<String>,
    },
    /// 串行执行整个矩阵：每次运行独立 case 目录，逐次提取指标并重写汇总表
    Run {
        /// 因子，形如 `熔体温度=200,210,220`（可重复）
        #[arg(long = "factor", required = true)]
        factors: Vec<String>,
        /// 编排方式：orthogonal（L9）或 full（全因子）
        #[arg(long, default_value = "orthogonal")]
        plan: String,
        /// 使用内置样例方盒（10mm）
        #[arg(long)]
        sample_box: bool,
        /// 指定 STL 路径（与 --sample-box 二选一）
        #[arg(long)]
        stl: Option<String>,
        /// 工作区目录：case 落 `<dir>/cases/<方案>/run-XXX/`，汇总表落 `<dir>/doe/<批次>/`
        #[arg(long, default_value = "workspace")]
        out_dir: String,
        /// 并行核数
        #[arg(long, default_value_t = 4)]
        cores: u32,
        /// 体素目标尺寸（mm）
        #[arg(long, default_value_t = 1.0)]
        target_size: f64,
        /// 基准注射时间（s）；被因子「注射时间」覆盖时以因子为准
        #[arg(long = "injection-time", default_value_t = 1.0)]
        injection_time_s: f64,
        /// 保压曲线起点压力（MPa）；用于复现固定工艺基线
        #[arg(long)]
        packing_pressure_mpa: Option<f64>,
        /// 完整保压曲线，形如 `0=0.9229,0.2=27.6282,315.0797=27.6282`（s=MPa）
        #[arg(long)]
        packing_pressure_curve: Option<String>,
        /// 保压时间（s）；用于复现固定工艺基线
        #[arg(long)]
        packing_time_s: Option<f64>,
        /// 冷却时间（s）
        #[arg(long)]
        cooling_time_s: Option<f64>,
        /// 批次名（缺省 = 因子名以短横连接）
        #[arg(long)]
        batch: Option<String>,
        /// 实际调用求解器（缺环境时逐次记为失败，批次继续）
        #[arg(long)]
        solve: bool,
        /// 使用 GUI 部署的 Multipass kairos 虚拟机运行求解器
        #[arg(long)]
        vm: bool,
        /// 自定义材料文件（JSON 或 CSV）；缺省使用内置参考材料
        #[arg(long)]
        material: Option<String>,
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
        /// 工作区目录（缺省 = 当前目录下的 workspace/；case 落在其 cases/ 子目录）
        #[arg(long, default_value = "workspace")]
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
        /// 注射时间（s）：真实件按件体积给合理值（工况校验会提示建议下限）
        #[arg(long = "injection-time", default_value_t = 1.0)]
        injection_time_s: f64,
        /// 实际调用求解器（缺环境时以结构化错误退出）
        #[arg(long)]
        solve: bool,
        /// 自定义材料文件（JSON 或 CSV）；缺省使用内置参考材料
        #[arg(long)]
        material: Option<String>,
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
    /// 生成体积网格（voxel = 内置体素引擎；gmsh = 外部引擎，需在依赖面板下载）
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
        /// 使用 GUI 部署的 Multipass kairos 虚拟机运行求解器
        #[arg(long)]
        vm: bool,
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

/// 默认工艺（注射时间可调：真实件按件体积给合理值）。
fn default_process_with(injection_time_s: f64) -> ProcessSettings {
    ProcessSettings {
        melt_temp_c: 230.0,
        mold_temp_c: 40.0,
        ejection_temp_c: 90.0,
        injection_time_s,
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
        Commands::Doe { action } => run_doe(action, json),
        Commands::Optimize { action } => run_optimize(action, json),
        Commands::Material { action } => run_material(action, json),
        Commands::Pipeline { action } => match action {
            PipelineAction::Run {
                sample_box,
                stl,
                out_dir,
                cores,
                target_size,
                gates,
                injection_time_s,
                solve,
                material,
            } => run_pipeline(
                sample_box,
                stl,
                out_dir,
                cores,
                target_size,
                gates,
                injection_time_s,
                solve,
                material,
                json,
            ),
        },
    }
}

fn run_material(action: MaterialAction, json: bool) -> kairos_core::error::Result<()> {
    match action {
        MaterialAction::Validate { path } => {
            let materials = services::material::read_custom_material_file(Path::new(&path))?;
            if materials.is_empty() {
                return Err(KairosError::validation("材料文件没有可用材料。"));
            }
            if json {
                emit_json(&serde_json::json!({
                    "path": path,
                    "count": materials.len(),
                    "materials": materials.iter().map(|material| serde_json::json!({
                        "id": material.id,
                        "name": material.name,
                        "family": material.family,
                        "manufacturer": material.manufacturer,
                    })).collect::<Vec<_>>(),
                }));
            } else {
                println!("材料文件校验通过：{}（{} 个材料）", path, materials.len());
                for material in materials {
                    println!("- {} [{}] {}", material.name, material.family, material.id);
                }
            }
            Ok(())
        }
        MaterialAction::Fit {
            template,
            viscosity,
            pvt,
            output,
        } => {
            let materials = services::material::read_custom_material_file(Path::new(&template))?;
            let material = match materials.as_slice() {
                [material] => material.clone(),
                [] => return Err(KairosError::validation("模板材料文件没有可用材料。")),
                _ => return Err(KairosError::validation("模板材料文件必须只有一个材料。")),
            };
            let viscosity_content = fs::read_to_string(&viscosity)
                .map_err(|error| KairosError::io(format!("读取黏度曲线失败：{error}")))?;
            let viscosity_points = if Path::new(&viscosity)
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("txt"))
            {
                material_curve::parse_viscosity_text(&viscosity_content)?
            } else {
                material_curve::parse_viscosity_csv(&viscosity_content)?
            };
            let pvt_content = fs::read_to_string(&pvt)
                .map_err(|error| KairosError::io(format!("读取 PVT 曲线失败：{error}")))?;
            let pvt_points = if Path::new(&pvt)
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("txt"))
            {
                material_curve::parse_pvt_text(&pvt_content)?
            } else {
                material_curve::parse_pvt_csv(&pvt_content)?
            };
            let (rheology, viscosity_residual) =
                material_curve::fit_cross_wlf_d1(&material.rheology, &viscosity_points)?;
            let (pvt, pvt_residual) = material_curve::fit_tait_b1(&material.pvt, &pvt_points)?;
            let viscosity_rows =
                material_curve::cross_wlf_residual_rows(&rheology, &viscosity_points)?;
            let pvt_rows = material_curve::tait_residual_rows(&pvt, &pvt_points)?;
            let mut fitted = material;
            fitted.rheology = rheology;
            fitted.pvt = pvt;
            services::material::write_custom_file(Path::new(&output), &[fitted.clone()])?;
            let solver_material_report = format!("{output}.moldingfoam");
            fs::write(
                &solver_material_report,
                services::material::serialize_moldingfoam_material(&fitted)?,
            )
            .map_err(|error| KairosError::io(format!("写入 moldingFoam 材料字典失败：{error}")))?;
            let viscosity_report = format!("{output}.viscosity-residual.json");
            let pvt_report = format!("{output}.pvt-residual.json");
            fs::write(
                &viscosity_report,
                material_curve::residual_rows_json(&viscosity_rows)?,
            )
            .map_err(|error| KairosError::io(format!("写入黏度残差报告失败：{error}")))?;
            fs::write(&pvt_report, material_curve::residual_rows_json(&pvt_rows)?)
                .map_err(|error| KairosError::io(format!("写入 PVT 残差报告失败：{error}")))?;
            if json {
                emit_json(&serde_json::json!({
                    "output": output,
                    "material": fitted,
                    "viscosityResidual": viscosity_residual,
                    "pvtResidual": pvt_residual,
                    "viscosityReport": viscosity_report,
                    "pvtReport": pvt_report,
                    "solverMaterial": solver_material_report,
                }));
            } else {
                println!("材料拟合完成：{}", output);
                println!("Cross-WLF log10(η) RMSE：{}", viscosity_residual.rmse_log10);
                println!("Tait 比容 RMSE：{}", pvt_residual.rmse_m3_per_kg);
            }
            Ok(())
        }
    }
}

fn run_optimize(action: OptimizeAction, json: bool) -> kairos_core::error::Result<()> {
    match action {
        OptimizeAction::Plan {
            factors,
            objective,
            budget,
        } => {
            let ranges = factors
                .iter()
                .map(|spec| parse_optimize_factor(spec))
                .collect::<kairos_core::error::Result<Vec<_>>>()?;
            let objective = match objective.as_str() {
                "fill-time" => optimize::Objective::MinFillTime,
                "injection-pressure" => optimize::Objective::MinInjectionPressure,
                other => {
                    return Err(KairosError::validation(format!(
                        "未知寻优目标「{other}」，可用 fill-time 或 injection-pressure。"
                    )));
                }
            };
            let mut planner = optimize::Optimizer::new(ranges, objective, budget)?;
            let candidates = planner.next_candidates();
            if json {
                emit_json(&serde_json::json!({
                    "objective": objective.metric_name(),
                    "budget": budget,
                    "candidates": candidates,
                }));
            } else {
                println!(
                    "目标 {}，生成 {} 个候选：",
                    objective.metric_name(),
                    candidates.len()
                );
                for (index, candidate) in candidates.iter().enumerate() {
                    println!("{}: {}", index + 1, format_optimize_parameters(candidate));
                }
            }
            Ok(())
        }
    }
}

fn parse_optimize_factor(spec: &str) -> kairos_core::error::Result<optimize::FactorRange> {
    let (name, values) = spec.split_once('=').ok_or_else(|| {
        KairosError::validation(format!("寻优因子格式应为 名称=min:max:levels：{spec}"))
    })?;
    let mut parts = values.split(':');
    let min = parts.next().and_then(|value| value.parse().ok());
    let max = parts.next().and_then(|value| value.parse().ok());
    let levels = parts.next().and_then(|value| value.parse().ok());
    if parts.next().is_some()
        || name.trim().is_empty()
        || min.is_none()
        || max.is_none()
        || levels.is_none()
    {
        return Err(KairosError::validation(format!(
            "寻优因子格式应为 名称=min:max:levels：{spec}"
        )));
    }
    Ok(optimize::FactorRange {
        name: name.trim().to_string(),
        min: min.unwrap(),
        max: max.unwrap(),
        levels: levels.unwrap(),
    })
}

fn format_optimize_parameters(parameters: &std::collections::BTreeMap<String, f64>) -> String {
    parameters
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn run_project(action: ProjectAction, json: bool) -> kairos_core::error::Result<()> {
    match action {
        ProjectAction::New { name, dir } => {
            let doc = project::create(&name, now_ms())?;
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
                println!("工程 {} 校验通过（{} 个方案）", path, doc.studies.len());
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

fn run_doe(action: DoeAction, json: bool) -> kairos_core::error::Result<()> {
    match action {
        DoeAction::Run {
            factors,
            plan,
            sample_box,
            stl,
            out_dir,
            cores,
            target_size,
            injection_time_s,
            packing_pressure_mpa,
            packing_pressure_curve,
            packing_time_s,
            cooling_time_s,
            batch,
            solve,
            vm,
            material,
        } => run_doe_batch(
            &factors,
            &plan,
            sample_box,
            stl,
            &out_dir,
            cores,
            target_size,
            injection_time_s,
            packing_pressure_mpa,
            packing_pressure_curve,
            packing_time_s,
            cooling_time_s,
            batch,
            solve,
            vm,
            material,
            json,
        ),
        DoeAction::Matrix {
            factors,
            plan,
            out_dir,
            batch,
        } => {
            let parsed = parse_doe_factors(&factors)?;
            let runs = doe::build_matrix(parse_doe_plan(&plan)?, &parsed)?;
            let batch_name = doe_batch_name(batch, &parsed);
            if let Some(dir) = out_dir {
                let target = doe::batch_dir(Path::new(&dir), &batch_name);
                write_doe_summary(&target, &runs)?;
                println!(
                    "批次「{batch_name}」已写入 {}（{} 次运行）",
                    target.display(),
                    runs.len()
                );
            }
            if json {
                print!("{}", doe::summary_json(&runs));
            } else {
                print!("{}", doe::summary_csv(&runs));
            }
            Ok(())
        }
    }
}

/// 串行执行整个矩阵：网格与几何只准备一次（每次运行只换工艺参数），
/// 每次运行独立 case 目录并按「跑完即回填」更新汇总表——中途中断也留下已完成的行。
fn load_material(
    path: Option<&str>,
) -> kairos_core::error::Result<kairos_core::models::material::Material> {
    match path {
        None => Ok(services::material::builtin_materials()[0].clone()),
        Some(path) => {
            let materials = services::material::read_custom_material_file(Path::new(path))?;
            match materials.as_slice() {
                [material] => Ok(material.clone()),
                [] => Err(KairosError::validation("自定义材料文件没有可用材料。")),
                _ => Err(KairosError::validation(
                    "DOE 运行一次只能选择一个自定义材料。",
                )),
            }
        }
    }
}

fn parse_pressure_curve(raw: &str) -> kairos_core::error::Result<Vec<(f64, f64)>> {
    let mut points = Vec::new();
    for item in raw.split(',') {
        let (time, pressure) = item
            .split_once('=')
            .ok_or_else(|| KairosError::validation("保压曲线点必须使用 time=pressure 格式。"))?;
        let time = time
            .trim()
            .parse::<f64>()
            .map_err(|_| KairosError::validation("保压曲线时间不是有效数字。"))?;
        let pressure = pressure
            .trim()
            .parse::<f64>()
            .map_err(|_| KairosError::validation("保压曲线压力不是有效数字。"))?;
        points.push((time, pressure));
    }
    if points.is_empty()
        || points.iter().any(|(time, pressure)| {
            !time.is_finite() || !pressure.is_finite() || *time < 0.0 || *pressure < 0.0
        })
        || points.windows(2).any(|pair| pair[1].0 <= pair[0].0)
    {
        return Err(KairosError::validation(
            "保压曲线必须包含非负有限值，时间必须严格递增。",
        ));
    }
    Ok(points)
}

#[allow(clippy::too_many_arguments)]
fn run_doe_batch(
    factor_specs: &[String],
    plan: &str,
    sample_box: bool,
    stl: Option<String>,
    out_dir: &str,
    cores: u32,
    target_size: f64,
    injection_time_s: f64,
    packing_pressure_mpa: Option<f64>,
    packing_pressure_curve: Option<String>,
    packing_time_s: Option<f64>,
    cooling_time_s: Option<f64>,
    batch: Option<String>,
    solve: bool,
    vm: bool,
    material_path: Option<String>,
    json: bool,
) -> kairos_core::error::Result<()> {
    let parsed = parse_doe_factors(factor_specs)?;
    let mut runs = doe::build_matrix(parse_doe_plan(plan)?, &parsed)?;
    let spacing = |run: usize| -> String { format!("[{}]", run) };
    let batch_name = doe_batch_name(batch, &parsed);
    let workspace = Path::new(out_dir);
    let batch_root = doe::batch_dir(workspace, &batch_name);
    write_doe_summary(&batch_root, &runs)?;

    // 几何与网格准备一次：矩阵只改工艺参数，重复划分网格纯属浪费。
    let mesh_tri = match (sample_box, stl) {
        (true, _) => kairos_core::models::geometry::TriangleMesh::sample_box(10.0),
        (false, Some(path)) => services::geometry::parse_stl_file(Path::new(&path))?,
        (false, None) => {
            return Err(KairosError::validation(
                "必须指定 --sample-box 或 --stl <路径>。".to_string(),
            ));
        }
    };
    let volume = services::meshing::generate(
        &mesh_tri,
        &services::meshing::VolumeMeshParams {
            refinement: None,
            target_size,
        },
    )?;
    let material = load_material(material_path.as_deref())?;
    let mut base_process = default_process_with(injection_time_s);
    if packing_pressure_mpa.is_some() && packing_pressure_curve.is_some() {
        return Err(KairosError::validation(
            "--packing-pressure-mpa 与 --packing-pressure-curve 不能同时使用。",
        ));
    }
    if let Some(curve) = packing_pressure_curve {
        base_process.packing_pressure_mpa_curve = parse_pressure_curve(&curve)?;
    } else if let Some(pressure) = packing_pressure_mpa {
        base_process.packing_pressure_mpa_curve = vec![(0.0, pressure)];
    }
    if let Some(time) = packing_time_s {
        base_process.packing_time_s = time;
    }
    if let Some(time) = cooling_time_s {
        if !time.is_finite() || time < 0.0 {
            return Err(KairosError::validation(
                "--cooling-time-s 必须是非负有限数值。",
            ));
        }
        base_process.cooling_time_s = time;
    }

    let total = runs.len();
    for index in 0..total {
        let case_dir = doe::run_case_dir(workspace, DOE_STUDY_ID, index + 1);
        let settings = doe::apply_factors(&base_process, &runs[index])?;
        let started = std::time::Instant::now();
        write_run_timestamp(&case_dir, "started-at-ms", now_ms())?;
        match run_doe_case(&case_dir, &volume, &material, &settings, cores, solve, vm) {
            Err(error) => doe::mark_failed(
                &mut runs[index],
                error.message(),
                Some(started.elapsed().as_secs_f64()),
            ),
            Ok(None) => {}
            Ok(Some(metrics)) => {
                doe::mark_done(&mut runs[index], metrics, started.elapsed().as_secs_f64())
            }
        }
        write_run_timestamp(&case_dir, "finished-at-ms", now_ms())?;
        let status = match &runs[index].status {
            doe::DoeStatus::Pending => "pending".to_string(),
            doe::DoeStatus::Done => "done".to_string(),
            doe::DoeStatus::Failed(reason) => format!("failed（{reason}）"),
        };
        if !json {
            println!(
                "{} 运行 {}/{}：{} → {}",
                spacing(index + 1),
                index + 1,
                total,
                runs[index]
                    .parameters
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                status
            );
        }
        // 每次运行后立刻重写汇总表：中断时已完成的行不丢
        write_doe_summary(&batch_root, &runs)?;
    }

    if json {
        print!("{}", doe::summary_json(&runs));
    } else {
        println!(
            "批次「{batch_name}」完成：{} 次运行，汇总表在 {}",
            runs.len(),
            batch_root.display()
        );
        print!("{}", doe::summary_csv(&runs));
    }
    let failed = runs
        .iter()
        .filter(|run| matches!(run.status, doe::DoeStatus::Failed(_)))
        .count();
    if failed > 0 {
        return Err(KairosError::solver(format!(
            "DOE 批次包含 {failed}/{} 个失败运行；汇总表已保留",
            runs.len()
        )));
    }
    Ok(())
}

/// 每个 DOE 运行目录保留原始日志旁的时间戳文件，便于与外部参考结果逐次对照。
fn write_run_timestamp(
    case_dir: &Path,
    name: &str,
    timestamp_ms: u64,
) -> kairos_core::error::Result<()> {
    std::fs::create_dir_all(case_dir)
        .map_err(|e| KairosError::io(format!("创建运行留档目录失败：{e}")))?;
    std::fs::write(case_dir.join(name), timestamp_ms.to_string())
        .map_err(|e| KairosError::io(format!("写入运行时间戳失败：{e}")))?;
    Ok(())
}

/// 执行一次 DOE case：生成 case，可选启动求解并解析日志指标。
/// 优化器回填复用此函数，避免再复制一套求解与指标解析链。
fn run_doe_case(
    case_dir: &Path,
    volume: &kairos_core::models::mesh::VolumeMesh,
    material: &kairos_core::models::material::Material,
    process: &ProcessSettings,
    cores: u32,
    solve: bool,
    vm: bool,
) -> kairos_core::error::Result<Option<std::collections::BTreeMap<String, f64>>> {
    moldingfoam::generate_case(
        case_dir,
        &moldingfoam::CaseInputs {
            mesh: volume,
            material,
            process,
            stage: &AnalysisStage::Fill,
            cores: cores as usize,
            gates: &[],
            channels: &[],
        },
    )?;
    if !solve {
        return Ok(None);
    }
    let case_text = case_dir.to_string_lossy().to_string();
    run_solver(&case_text, cores, vm)?;
    let log = std::fs::read_to_string(case_dir.join("log.foamRun")).unwrap_or_default();
    Ok(Some(moldingfoam::parse_metrics(&log)))
}

/// DOE 批次的方案 id（case 目录 `<工作区>/cases/<方案 id>/run-XXX/`）。
const DOE_STUDY_ID: &str = "doe";

/// 跑一次求解并把完整输出留到 `log.foamRun`（DOE 逐次读它取指标）。
fn run_solver(case_dir: &str, cores: u32, vm: bool) -> kairos_core::error::Result<()> {
    if vm {
        return run_solver_in_vm(case_dir, cores);
    }
    let dir = Path::new(case_dir);
    if !dir.join("system/controlDict").exists() {
        return Err(KairosError::not_found(
            "case 目录缺少 system/controlDict，请先生成 case。",
        ));
    }
    let safe_dir = shell::bash_single_quote(case_dir);
    let solve = kairos_core::services::moldingfoam::solve_command(cores);
    // 重定向必须**分组**：`A && B; C > log` 里 `>` 只绑定 C，前面命令的报错根本进不了
    // 日志（曾因此把 decomposePar 的错报成 reconstructPar 的错）。
    let status = Command::new("bash")
        .arg("-lc")
        .arg(format!("cd '{safe_dir}' && ( {solve} ) > log.foamRun 2>&1"))
        .status()
        .map_err(|e| KairosError::io(format!("求解器启动失败（本机需 OpenFOAM 11+）：{e}")))?;
    // 退出码可能被 `; reconstructPar` 掩盖（求解失败但重建成功 → 退出码 0），
    // 因此日志里的错误标记优先于退出码——与桌面端的失败判定同一口径。
    let log = std::fs::read_to_string(dir.join("log.foamRun")).unwrap_or_default();
    let masked = log.contains("not found") || log.contains("FOAM FATAL");
    if !status.success() || masked {
        // 失败原因交给 core 判读（优先第一条错误行，见 moldingfoam::failure_reason）
        let reason = moldingfoam::failure_reason(&log);
        return Err(KairosError::solver(if reason.is_empty() {
            "求解失败（日志为空）".to_string()
        } else {
            format!("求解失败：{reason}")
        }));
    }
    Ok(())
}

/// 复用 GUI 已部署的 Multipass 环境运行一次 case，并把日志与结果回传宿主。
fn run_solver_in_vm(case_dir: &str, cores: u32) -> kairos_core::error::Result<()> {
    let runner = vm_run::ProcessRunner::new(None, vm_run::ProcessRunner::DEFAULT_TIMEOUT_S);
    let archive = std::env::temp_dir().join(format!("kairos-cli-{}-case.tgz", now_ms()));
    let vm_case = vm_run::stage_case(&runner, &archive, case_dir)?;
    let solve = moldingfoam::solve_command(cores);
    let script = format!(
        "{}; cd '{}' && ( {} ) > log.foamRun 2>&1",
        vm_logic::env_source_command(),
        shell::bash_single_quote(&vm_case),
        solve
    );
    let command = vm_run::HostCommand::new(vm_logic::bash_script_args(
        kairos_core::models::vm::VmProviderKind::Multipass,
        &script,
    ));
    let solve_result = vm_run::HostRunner::run(&runner, &command, "虚拟机求解");
    let log_command = vm_run::HostCommand::new(vm_logic::bash_script_args(
        kairos_core::models::vm::VmProviderKind::Multipass,
        &format!(
            "cat '{}/log.foamRun' 2>/dev/null",
            shell::bash_single_quote(&vm_case)
        ),
    ));
    if let Ok(log) = vm_run::HostRunner::capture(&runner, &log_command, "回读虚拟机求解日志")
    {
        std::fs::write(Path::new(case_dir).join("log.foamRun"), log)
            .map_err(|e| KairosError::io(format!("写入求解日志失败：{e}")))?;
    }
    let result = solve_result
        .and_then(|()| vm_run::copy_results(&runner, &archive, case_dir, &vm_case).map(|_| ()));
    let _ = std::fs::remove_file(&archive);
    result
}

/// 解析 DOE 因子参数：`名称=值1,值2,…`（空列表或坏数字明确报错）。
/// 编排方式名 → 计划。可选取值与错误文案只此一处（`matrix` 与 `batch` 共用）。
fn parse_doe_plan(plan: &str) -> kairos_core::error::Result<doe::DoePlan> {
    match plan {
        "orthogonal" => Ok(doe::DoePlan::OrthogonalL9),
        "full" => Ok(doe::DoePlan::FullFactorial),
        other => Err(KairosError::validation(format!(
            "未知的编排方式「{other}」，可用：orthogonal / full。"
        ))),
    }
}

/// 批次名：显式给出优先，否则由因子名连接（两条子命令同口径）。
fn doe_batch_name(batch: Option<String>, factors: &[doe::DoeFactor]) -> String {
    batch.unwrap_or_else(|| {
        factors
            .iter()
            .map(|factor| factor.name.clone())
            .collect::<Vec<_>>()
            .join("-")
    })
}

/// 把汇总表（csv + json）落到批次目录，缺目录就建。
///
/// 逐次运行后也调它（目录已存在时 `create_dir_all` 是幂等的），以便中断时已完成
/// 的行不丢。
fn write_doe_summary(dir: &Path, runs: &[doe::DoeRun]) -> kairos_core::error::Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| KairosError::io(format!("创建批次目录失败：{e}")))?;
    std::fs::write(dir.join("summary.csv"), doe::summary_csv(runs))
        .map_err(|e| KairosError::io(format!("写入汇总表失败：{e}")))?;
    std::fs::write(dir.join("summary.json"), doe::summary_json(runs))
        .map_err(|e| KairosError::io(format!("写入汇总表失败：{e}")))?;
    Ok(())
}

fn parse_doe_factors(specs: &[String]) -> kairos_core::error::Result<Vec<doe::DoeFactor>> {
    let mut factors = Vec::with_capacity(specs.len());
    for spec in specs {
        let (name, values) = spec.split_once('=').ok_or_else(|| {
            KairosError::validation(format!(
                "因子「{spec}」缺少 `=`，应形如 熔体温度=200,210,220。"
            ))
        })?;
        let name = name.trim();
        if name.is_empty() {
            return Err(KairosError::validation("因子名不能为空。"));
        }
        let parsed = values
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                value.parse::<f64>().map_err(|e| {
                    KairosError::validation(format!("因子「{name}」的水平「{value}」不是数字：{e}"))
                })
            })
            .collect::<kairos_core::error::Result<Vec<f64>>>()?;
        if parsed.is_empty() {
            return Err(KairosError::validation(format!(
                "因子「{name}」没有水平取值（形如 {name}=1,2,3）。"
            )));
        }
        factors.push(doe::DoeFactor {
            name: name.to_string(),
            values: parsed,
        });
    }
    Ok(factors)
}

fn run_solve(action: SolveAction, json: bool) -> kairos_core::error::Result<()> {
    match action {
        SolveAction::Submit {
            case_dir,
            cores,
            vm,
        } => {
            if json {
                run_solver(&case_dir, cores, vm)?;
                emit_json(&serde_json::json!({ "caseDir": case_dir, "ok": true }));
                return Ok(());
            }
            run_solver(&case_dir, cores, vm)?;
            println!("求解完成：{case_dir}");
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
    injection_time_s: f64,
    solve: bool,
    material_path: Option<String>,
    json: bool,
) -> kairos_core::error::Result<()> {
    // 工作区布局：case 写在 <工作区>/cases/<方案>/ 下，与桌面端同一套规则。
    let workspace = Path::new(&out_dir);
    let out = &services::workspace::create_run_dir(workspace, "cli")?;
    let out = out.as_path();
    let case_dir_text = out.to_string_lossy().to_string();
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
    let material = load_material(material_path.as_deref())?;
    let gates = gate_specs
        .iter()
        .map(|spec| parse_gate(spec))
        .collect::<kairos_core::error::Result<Vec<_>>>()?;
    let process = default_process_with(injection_time_s);
    let case_report = moldingfoam::generate_case(
        out,
        &moldingfoam::CaseInputs {
            mesh: &volume,
            material: &material,
            process: &process,
            stage: &AnalysisStage::Fill,
            cores: cores as usize,
            gates: &gates,
            channels: &[],
        },
    )?;
    // 网格尺寸 vs 最小特征提示（与几何面板同一套 core 探测）
    let thickness = services::thickness::probe_thickness(&mesh_tri);
    let thickness_hints = services::thickness::thin_feature_hints(target_size, &thickness);
    // 填充工况量级提示（与工艺面板同一套 core 校验）
    let volume_mm3 = services::moldingfoam::mesh_volume(&volume);
    let inlet = serde_json::json!({
        "areaM2": case_report.inlet_area_m2,
        "equivalentDiameterMm": case_report.inlet_equivalent_diameter_mm(),
        "gates": case_report.gates,
        "warnings": case_report.warnings,
    });
    let load_hints =
        services::process::fill_load_hints(volume_mm3, &process, Some(case_report.inlet_area_m2));
    if json {
        emit_json(&serde_json::json!({
            "workspace": out_dir,
            "caseDir": case_dir_text,
            "nodes": volume.nodes.len(),
            "tets": volume.tets.len(),
            "solved": solve,
            "inlet": inlet,
            "loadHints": load_hints,
            "thicknessHints": thickness_hints,
            "thickness": {
                "minMm": thickness.min_mm(),
                "p05Mm": thickness.quantile_mm(0.05),
                "medianMm": thickness.median_mm(),
                "samples": thickness.count(),
            },
        }));
    } else {
        println!(
            "case 已生成：{}（{} 节点 / {} 四面体）",
            case_dir_text,
            volume.nodes.len(),
            volume.tets.len()
        );
        println!(
            "浇口入口：实际 {:.1} mm²（等效 Ø{:.1} mm）",
            case_report.inlet_area_m2 * 1e6,
            case_report.inlet_equivalent_diameter_mm()
        );
        for gate in &case_report.gates {
            println!(
                "  浇口 #{}：请求 Ø{:.1} mm（{:.1} mm²）→ 实际 {:.1} mm² / {} 面（{:.2}×）",
                gate.index,
                gate.requested_radius_mm * 2.0,
                gate.requested_area_mm2,
                gate.actual_area_mm2,
                gate.face_count,
                gate.area_ratio
            );
        }
        for hint in &thickness_hints {
            println!("网格提示：{hint}");
        }
        for warning in &case_report.warnings {
            println!("入口提示：{warning}");
        }
        for hint in &load_hints {
            println!("工况提示：{hint}");
        }
    }
    // 4. 求解（可选；缺 OpenFOAM 环境时输出结构化错误，供脚本捕获）
    if solve {
        return run_solve(
            SolveAction::Submit {
                case_dir: case_dir_text.clone(),
                cores,
                vm: false,
            },
            json,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_optimize_factor, parse_pressure_curve, write_run_timestamp};

    #[test]
    fn optimize_factor_parser_accepts_named_range() {
        let range = parse_optimize_factor("熔体温度=200:240:3").unwrap();
        assert_eq!(range.name, "熔体温度");
        assert_eq!(range.levels, 3);
        assert_eq!(range.min, 200.0);
        assert_eq!(range.max, 240.0);
    }

    #[test]
    fn optimize_factor_parser_rejects_bad_shape() {
        assert!(parse_optimize_factor("熔体温度=200:240").is_err());
        assert!(parse_optimize_factor("=200:240:3").is_err());
        assert!(parse_optimize_factor("熔体温度=200:240:3:4").is_err());
    }

    #[test]
    fn run_timestamp_is_written_as_raw_epoch_milliseconds() {
        let root = std::env::temp_dir().join(kairos_core::services::project::new_id("cli-run"));
        write_run_timestamp(&root, "started-at-ms", 1_725_000_000_123).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("started-at-ms")).unwrap(),
            "1725000000123"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pressure_curve_parser_keeps_ordered_points() {
        assert_eq!(
            parse_pressure_curve("0=0.9229,0.2=27.6282,315.0797=27.6282").unwrap(),
            vec![(0.0, 0.9229), (0.2, 27.6282), (315.0797, 27.6282)]
        );
    }

    #[test]
    fn pressure_curve_parser_rejects_unordered_or_negative_points() {
        assert!(parse_pressure_curve("0=1,0=2").is_err());
        assert!(parse_pressure_curve("-1=1,2=3").is_err());
        assert!(parse_pressure_curve("0=1,broken").is_err());
    }
}
