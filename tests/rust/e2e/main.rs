//! 全流程集成测试（正常使用路径）：几何 → 网格 → case → 求解 → 结果。
//!
//! 为什么单独一层：这条链路此前只在界面上人工走过，命令构造、分步编排（打包 → 传输 →
//! VM 内解压）、结果回传与「命令成败怎么判」全都在无人自动验证的地方——真机上出过
//! 「tar 只打了警告、退出码 0，却被判成解压失败、作业直接失败」这类问题，单元测试
//! 全绿也照漏。本测试把整条链路按用户路径串起来跑，并断言可观察产物（case 结构、
//! 求解时间目录、读场结果），而不是断言内部调用。
//!
//! 两层：
//! - **L1（无条件执行，CI 也跑）**：样例几何 → 体素网格 → case（内置材料 + 默认工艺 +
//!   填充阶段）→ 结果目录扫描。跑法 `cargo test -p kairos-tests --test e2e`。
//! - **L2（真机 VM，`KAIROS_E2E_VM=1` 才跑）**：case 进 VM → 脱离会话求解 → 结果回传 →
//!   宿主侧扫描 + 读场。需要 multipass + 名为 `kairos` 的实例 + 虚拟机内已部署求解环境，
//!   缺任何一项即失败（开关是显式打开的，不能静默放过）。
//!
//! 编排本身走 `kairos_core::services::vm_run`（与桌面端作业线程同一段代码），
//! 本测试只提供「怎么跑宿主命令」的 runner 与断言。

use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use kairos_core::models::geometry::TriangleMesh;
use kairos_core::models::process::ProcessSettings;
use kairos_core::models::solver::AnalysisStage;
use kairos_core::models::vm::VmProviderKind;
use kairos_core::services::vm_run::{HostCommand, HostRunner};
use kairos_core::services::{material, meshing, moldingfoam, results, vm as vm_logic, vm_run};

/// L1 网格目标尺寸（mm）：内置样例方盒 10mm，粗网格足够走通链路且秒级完成。
const L1_TARGET_SIZE_MM: f64 = 2.0;
/// 求解核数：真机测试不占满机器。
const SOLVE_CORES: u32 = 2;
/// L2 求解预算（秒）：超预算即判失败（求解器卡住要暴露出来，不是无限等）。
const SOLVE_BUDGET_S: u64 = 600;
/// 失败时打印的日志尾部行数：够定位，不刷屏。
const LOG_TAIL_LINES: usize = 40;

/// 宿主命令执行者：core 的生产实现（补 PATH 前缀 / 抽干管道 / 超时 / 成败判定同一份代码）。
/// 测试从终端跑，PATH 是全的，因此不补前缀。
fn runner() -> vm_run::ProcessRunner {
    vm_run::ProcessRunner::new(None, vm_run::ProcessRunner::DEFAULT_TIMEOUT_S)
}

/// 默认工艺（与 CLI / 面板同一口径的量级：注射时间取小值让端到端跑得快）。
fn test_process(injection_time_s: f64) -> ProcessSettings {
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

/// 本次测试的工作区（临时目录，唯一名；失败时保留现场便于排查）。
fn scratch_workspace() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("kairos-e2e-{stamp}"))
}

/// L1：几何 → 网格 → case → 结果扫描。返回 case 目录与网格规模（节点 / 四面体）。
fn l1_build_case(workspace: &Path) -> (PathBuf, usize, usize) {
    // 1. 几何：内置样例方盒（与「新建方案 → 用样例」同一条 core 路径）
    let mesh_tri = TriangleMesh::sample_box(10.0);
    // 2. 网格：体素引擎（不依赖外部可执行，CI 也能跑）
    let volume = meshing::generate(
        &mesh_tri,
        &meshing::VolumeMeshParams {
            refinement: None,
            target_size: L1_TARGET_SIZE_MM,
        },
    )
    .expect("体素网格生成失败");
    assert!(
        volume.nodes.len() > 3,
        "网格节点数异常：{}",
        volume.nodes.len()
    );
    assert!(!volume.tets.is_empty(), "网格没有四面体");

    // 3. case：首个内置材料 + 默认工艺 + 填充阶段（面板提交时的同一套输入）
    let case_dir = workspace.join("cases").join("e2e");
    let process = test_process(0.4);
    moldingfoam::generate_case(
        &case_dir,
        &moldingfoam::CaseInputs {
            mesh: &volume,
            material: &material::builtin_materials()[0],
            process: &process,
            stage: &AnalysisStage::Fill,
            cores: SOLVE_CORES as usize,
            gates: &[],
            channels: &[],
        },
    )
    .expect("case 生成失败");
    (case_dir, volume.nodes.len(), volume.tets.len())
}

/// case 该有的东西：求解器要读的三个目录 + polyMesh 四件套 + 初始场。
/// 缺任何一项，求解都会在 VM 里以一句 OpenFOAM 报错结束（而不是在生成阶段暴露）。
fn assert_case_layout(case_dir: &Path) {
    for relative in [
        "system/controlDict",
        "system/fvSchemes",
        "system/fvSolution",
        "system/decomposeParDict",
        "constant/polyMesh/points",
        "constant/polyMesh/faces",
        "constant/polyMesh/owner",
        "constant/polyMesh/neighbour",
        "constant/polyMesh/boundary",
        "constant/physicalProperties.melt",
        "0/U",
        "0/p",
        "0/T",
        "0/alpha.melt",
    ] {
        assert!(
            case_dir.join(relative).is_file(),
            "case 缺少 {relative}（求解器会直接报错）"
        );
    }
    // 时间终点口径：填充阶段 = 注射时间 × 2（阶段语义的唯一出处是 models::solver）
    let control_dict = std::fs::read_to_string(case_dir.join("system/controlDict")).unwrap();
    let expected = format!(
        "endTime         {:.6}",
        AnalysisStage::Fill.end_time_s(0.4, 8.0, 15.0)
    );
    assert!(
        control_dict.contains(&expected),
        "controlDict 里的时间终点与阶段口径不一致（期望含 `{expected}`）"
    );
}

/// L1 尾段：结果目录扫描（界面「结果」面板的入口）能认出初始时间目录 0。
fn l1_scan_initial_time(case_dir: &Path) {
    let catalog = results::scan_times(case_dir).expect("结果目录扫描失败");
    assert!(
        catalog.times.iter().any(|time| time.dir_name == "0"),
        "结果扫描应认出初始时间目录 0，实际：{:?}",
        catalog
            .times
            .iter()
            .map(|time| time.dir_name.clone())
            .collect::<Vec<_>>()
    );
}

/// L2 前置：multipass 可用、实例在、求解环境已部署。开关显式打开时缺前置即失败。
fn require_vm_ready(runner: &vm_run::ProcessRunner) {
    runner
        .run(
            &HostCommand::new(vec![
                "multipass".into(),
                "info".into(),
                vm_logic::INSTANCE_NAME.into(),
            ]),
            "读取虚拟机状态",
        )
        .expect("KAIROS_E2E_VM=1 需要 multipass 与 kairos 实例（先启动虚拟机）");
    runner
        .run(
            &HostCommand::new(vm_logic::bash_script_args(
                VmProviderKind::Multipass,
                &vm_logic::env_probe_command(),
            )),
            "求解环境探测",
        )
        .expect("虚拟机内没有求解环境：先在依赖面板下载并部署 moldingFoam bundle");
}

/// 轮询到求解结束：返回（退出码, 末尾日志）。预算内没等到退出码时退出码为 None，
/// 由调用方判定（成功路径要 Some(0)，失败路径要非 0）。预算用尽即返回，不无限等。
fn poll_until_exit(
    runner: &vm_run::ProcessRunner,
    vm_case: &str,
    budget_s: u64,
) -> (Option<i32>, Vec<String>) {
    let deadline = Instant::now() + Duration::from_secs(budget_s);
    let mut offset = 0u64;
    let mut exit_code: Option<i32> = None;
    let mut log_tail: Vec<String> = Vec::new();
    while exit_code.is_none() && Instant::now() < deadline {
        thread::sleep(Duration::from_secs(1));
        match vm_run::poll_log_once(runner, vm_case, offset) {
            Ok(poll) => {
                offset = poll.offset;
                exit_code = poll.exit_code;
                log_tail.extend(poll.lines);
                if log_tail.len() > LOG_TAIL_LINES {
                    log_tail.drain(0..log_tail.len() - LOG_TAIL_LINES);
                }
            }
            // 回读失败多为通道抖动（与桌面端同一处置）：重试，等下一轮。
            Err(error) => eprintln!("日志回读失败（重试中）：{}", error.message()),
        }
    }
    (exit_code, log_tail)
}

/// L2 主体：case 进 VM → 脱离会话求解 → 等退出码 → 结果回传 → 宿主侧扫描与读场。
fn l2_solve_and_collect(case_dir: &Path, workspace: &Path) {
    let runner = runner();
    require_vm_ready(&runner);
    let case_dir_text = case_dir.to_string_lossy().to_string();
    let archive = workspace.join("transfer.tgz");

    // 1. 进 VM：打包 → 传输 → VM 内解压（core 的编排，与桌面端作业线程同一段代码）
    let vm_case = vm_run::stage_case(&runner, &archive, &case_dir_text).expect("case 进虚拟机失败");
    let _ = std::fs::remove_file(&archive);

    // 2. 脱离会话求解：启动命令让 setsid 脱离会话，日志与退出码落盘在 VM 内 case 里
    let script = vm_logic::solve_script(&vm_case, SOLVE_CORES);
    let launch = vm_logic::detached_launch_command(&vm_case, &script);
    runner
        .run(
            &HostCommand::new(vm_logic::bash_script_args(
                VmProviderKind::Multipass,
                &launch,
            )),
            "VM 求解启动",
        )
        .expect("求解启动失败（staging 或环境有问题）");

    // 3. 轮询日志与退出码：预算内必须读到退出码，否则判失败并打印日志尾部
    let (exit_code, log_tail) = poll_until_exit(&runner, &vm_case, SOLVE_BUDGET_S);
    assert_eq!(
        exit_code,
        Some(0),
        "求解没有在 {SOLVE_BUDGET_S} 秒内以退出码 0 结束（末尾日志见下）\n{}",
        log_tail.join("\n")
    );
    assert!(
        log_tail.iter().any(|line| line.starts_with("Time = ")),
        "日志里没有时间步推进：{log_tail:?}"
    );

    // 4. 结果回传：VM 内列出时间目录 → 打包 → 传输回宿主 → 解压到 case 目录
    let copied =
        vm_run::copy_results(&runner, &archive, &case_dir_text, &vm_case).expect("结果回传失败");
    let _ = std::fs::remove_file(&archive);
    assert!(
        copied
            .iter()
            .any(|name| name.parse::<f64>().map(|time| time > 0.0).unwrap_or(false)),
        "回传的时间目录里没有求解结果（只有初始场）：{copied:?}"
    );

    // 5. 宿主侧结果链路：扫描 + 读速度场（界面结果面板同一条 core 路径）
    let catalog = results::scan_times(case_dir).expect("回传后结果扫描失败");
    let solved_time = catalog
        .times
        .iter()
        .find(|time| time.time_s > 0.0)
        .expect("宿主 case 目录里没有求解时间目录");
    assert!(
        solved_time.fields.iter().any(|field| field == "U"),
        "求解时间目录缺速度场：{:?}",
        solved_time.fields
    );
    let field =
        results::read_vector_field(case_dir, &solved_time.dir_name, "U").expect("读取速度场失败");
    assert!(!field.components.is_empty(), "速度场没有分量");
    assert!(field.complete, "速度场分量数与网格不一致（不完整结果）");

    // 6. 失败可见性：把 case 改坏（删掉网格边界文件，等价于用户手改坏 case）→ 必须失败且可归因。
    //    判据用**求解器错误标记**而不是退出码：求解脚本尾部的 `; reconstructPar` 会掩盖
    //    decomposePar / foamRun 的退出码，产品侧同样以日志标记为准（moldingfoam::failure_reason）。
    let break_case = format!(
        "rm -f '{}/constant/polyMesh/boundary'",
        vm_logic::bash_single_quote(&vm_case)
    );
    runner
        .run(
            &HostCommand::new(vm_logic::bash_script_args(
                VmProviderKind::Multipass,
                &break_case,
            )),
            "改坏 case（失败用例）",
        )
        .expect("改坏 case 的命令本身应当成功");
    let broken_launch =
        vm_logic::detached_launch_command(&vm_case, &vm_logic::solve_script(&vm_case, SOLVE_CORES));
    runner
        .run(
            &HostCommand::new(vm_logic::bash_script_args(
                VmProviderKind::Multipass,
                &broken_launch,
            )),
            "VM 求解启动（失败用例）",
        )
        .expect("失败用例的启动命令本身应能跑起来（失败要发生在求解里）");
    let (_broken_code, broken_log) = poll_until_exit(&runner, &vm_case, SOLVE_BUDGET_S);
    let broken_text = broken_log.join("\n");
    assert!(
        broken_log
            .iter()
            .any(|line| moldingfoam::is_abort_line(line)),
        "改坏的 case 必须报出求解器错误标记（作业据此判失败）\n{broken_text}"
    );
    let reason = moldingfoam::failure_reason(&broken_text);
    assert!(
        reason.contains("FOAM FATAL"),
        "失败原因必须可归因（强特征优先），实际：{reason}\n{broken_text}"
    );

    // 7. 清理 VM 内的 case 暂存（宿主 case 目录保留，便于人工查看）
    let cleanup = format!("rm -rf '{}'", vm_logic::bash_single_quote(&vm_case));
    runner
        .run(
            &HostCommand::new(vm_logic::bash_script_args(
                VmProviderKind::Multipass,
                &cleanup,
            )),
            "清理虚拟机内的 case",
        )
        .expect("清理 VM 内 case 失败");
}

/// 全流程：L1 无条件跑；L2 在 `KAIROS_E2E_VM=1` 时跑（真机 VM 通道）。
#[test]
fn normal_usage_flow_geometry_mesh_case_solve_results() {
    let workspace = scratch_workspace();
    std::fs::create_dir_all(&workspace).expect("创建工作区失败");
    let (case_dir, nodes, tets) = l1_build_case(&workspace);
    assert_case_layout(&case_dir);
    l1_scan_initial_time(&case_dir);
    println!(
        "L1 通过：{} 节点 / {} 四面体，case 在 {}",
        nodes,
        tets,
        case_dir.display()
    );

    if std::env::var("KAIROS_E2E_VM").as_deref() != Ok("1") {
        println!(
            "SKIP L2（真机 VM 层）：设 KAIROS_E2E_VM=1 才跑，需要 multipass + 已部署环境的 kairos 实例"
        );
        let _ = std::fs::remove_dir_all(&workspace);
        return;
    }
    l2_solve_and_collect(&case_dir, &workspace);
    println!(
        "L2 通过：求解与结果回传完成，case 在 {}",
        case_dir.display()
    );
    // 成功即清理；失败时保留现场（上面的断言会带出路径）。
    if std::env::var("KAIROS_E2E_KEEP").is_err() {
        let _ = std::fs::remove_dir_all(&workspace);
    }
}
