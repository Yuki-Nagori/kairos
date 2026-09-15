//! 求解作业调度器：队列、并发预算与作业生命周期。
//! core（services::jobs）负责状态机与并发策略的纯逻辑；本模块负责进程副作用与进度回传。

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
#[cfg(target_os = "macos")]
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::jobs::Job;
use kairos_core::models::vm::{VmProviderKind, VmState};
use kairos_core::services::jobs as job_logic;
use kairos_core::services::jobs::SchedulerLimits;
use kairos_core::services::moldingfoam;
use kairos_core::services::project::new_id;
use kairos_core::utils::time::now_ms;
// VM 通道（macOS/Windows 的 env source）与原生环境（Linux 的 source 行）都用它，
// 因此导入**不带 cfg**——只在 cfg 块里引用会让 Linux 构建找不到符号。
use kairos_core::services::vm as vm_logic;
use kairos_core::services::vm_run;
// 命令层与作业层共用：宿主命令构造与 VM 通道 runner（core 的参数表 + GUI 进程的 PATH 修补）。
use super::vm::{host_command, host_runner};
use kairos_core::services::vm_run::HostRunner;
use tauri::State;
use tauri::ipc::Channel;

/// 虚拟机就绪轮询：次数 × 间隔（约 2 分钟，够 `multipass start` 后 sshd 起来）。
const VM_READY_ATTEMPTS: usize = 60;
const VM_READY_INTERVAL_S: u64 = 2;

struct Inner {
    jobs: Vec<Job>,
    cancellations: HashMap<String, Arc<AtomicBool>>,
    channels: HashMap<String, Channel<String>>,
    limits: SchedulerLimits,
    /// 原生（Linux）求解环境根目录：Some 时作业先 source 其 bashrc。
    native_env: Option<String>,
}

/// 调度器共享状态（线程安全：作业线程与命令线程共享同一份 Inner）。
#[derive(Clone)]
pub struct JobScheduler {
    inner: Arc<Mutex<Inner>>,
    /// 受管 bin 目录的 PATH 前缀（下载解压后由命令层注入）。
    managed_path: Option<String>,
    /// VM 执行通道：Some("multipass"/"wsl") 时作业在虚拟机内运行。
    vm_shell: Option<String>,
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self {
            managed_path: None,
            inner: Arc::new(Mutex::new(Inner {
                jobs: Vec::new(),
                cancellations: HashMap::new(),
                channels: HashMap::new(),
                limits: SchedulerLimits::new(2, job_logic::MAX_JOB_CORES),
                native_env: None,
            })),
            vm_shell: None,
        }
    }
}

impl JobScheduler {
    /// 由 VM 执行通道构造（启动时探测一次）。
    pub fn with_vm_shell(mut self, vm_shell: Option<String>) -> Self {
        self.vm_shell = vm_shell;
        self
    }

    /// 注入原生（Linux）求解环境根目录（构造期）。
    pub fn with_native_env(self, native_env: Option<String>) -> Self {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.native_env = native_env;
        drop(inner);
        self
    }

    /// 运行期刷新原生环境（启动后下载 / 部署 bundle 时调用）。
    pub fn set_native_env(&self, native_env: Option<String>) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.native_env = native_env;
    }
}

/// 刷新调度器的原生求解环境（启动时与部署 / 下载后调用）：
/// 找到本机解压好的 bundle（含 `openfoam14/etc/bashrc`）并注入其根目录，
/// 使原生平台提交作业时先 source 该环境。没找到则清空（回退系统 OpenFOAM）。
/// 三个平台都调用同一实现：VM 通道存在时 vm_shell 分支优先，不受影响。
pub fn refresh_native_env(app: &tauri::AppHandle) {
    use tauri::Manager;
    let root =
        super::downloads::native_env_root(app).map(|path| path.to_string_lossy().to_string());
    app.state::<JobScheduler>().set_native_env(root);
}

/// 探测本机的 VM 执行通道：macOS 用 multipass，Windows 用 wsl；Linux 原生执行。
pub fn detect_vm_shell() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("multipass")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            .then(|| "multipass".to_string())
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("wsl")
            .arg("--status")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            .then(|| "wsl".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/// 往作业日志通道写一行（VM 生命周期提示走同一通道，用户能在作业日志里看到）。
fn send_job_line(inner: &Arc<Mutex<Inner>>, job_id: &str, line: &str) {
    let channel = {
        let guard = inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.channels.get(job_id).cloned()
    };
    if let Some(channel) = channel {
        let _ = channel.send(line.to_string());
    }
}

/// 作业所在平台对应的虚拟机 provider（原生平台为 None，无需虚拟机）。
fn job_vm_provider(vm_shell: Option<&str>) -> Option<VmProviderKind> {
    let provider = vm_logic::provider_for(std::env::consts::OS)?;
    vm_shell
        .map(|_| provider)
        .filter(|p| *p != VmProviderKind::Native)
}

/// 作业需要虚拟机时确保它可执行：停着就拉起，拉起后轮询到 guest 能跑命令为止。
///
/// 只有作业主动拉起的实例才登记「自动拉起」标记——空闲收尾时据此决定是否关闭，
/// 用户自己启动的实例不动。WSL 的实例随首次执行自启（无独立启动命令），
/// 轮询本身就会把它拉起来。
fn ensure_vm_ready(inner: &Arc<Mutex<Inner>>, job_id: &str, vm_shell: Option<&str>) -> Result<()> {
    check_cancelled(inner, job_id)?;
    let Some(provider) = job_vm_provider(vm_shell) else {
        return Ok(());
    };
    let probe = vm_logic::ready_probe_args(provider)
        .ok_or_else(|| KairosError::internal("缺少虚拟机就绪探测命令。"))?;
    // 探测命令本身（multipass exec）对停止实例会**隐式拉起**，所以先取实例状态：
    // 「作业拉起的实例」这一租约登记必须发生在任何会启动实例的命令之前，
    // 否则收尾时看不到标记、实例会一直开着。
    let stopped = matches!(
        super::vm::probe_instance_state(provider),
        Ok(VmState::Stopped)
    );
    if !stopped && probe_ready(&probe) {
        return Ok(());
    }
    if stopped {
        send_job_line(inner, job_id, "── 启动虚拟机（作业需要，跑完自动关闭）──");
    }
    super::vm_lease::mark_auto_started();
    // 停止的实例显式拉起；已在启动中的（Starting / 状态未知）靠后续探测自己就绪。
    let start = if stopped {
        vm_logic::start_args(provider)
    } else {
        None
    };
    if let Some(start) = start {
        let status = super::vm::platform_command(&start[0])
            .args(&start[1..])
            .status()
            .map_err(|e| KairosError::io(format!("虚拟机启动命令失败：{e}")))?;
        if !status.success() {
            return Err(KairosError::io("虚拟机启动失败，请到虚拟机面板查看状态。"));
        }
    }
    // 启动命令返回时 guest 可能还没起完 sshd：轮询到可执行，最多约 2 分钟。
    for _ in 0..VM_READY_ATTEMPTS {
        check_cancelled(inner, job_id)?;
        if probe_ready(&probe) {
            return Ok(());
        }
        thread::sleep(std::time::Duration::from_secs(VM_READY_INTERVAL_S));
    }
    Err(KairosError::io(
        "虚拟机启动后仍不可执行命令（超时），请检查虚拟机状态后重试。",
    ))
}

/// 跑一次就绪探测（命令能成功返回即就绪）。
fn probe_ready(args: &[String]) -> bool {
    let mut runner = host_runner();
    runner.timeout = std::time::Duration::from_secs(10);
    runner
        .run(&vm_run::HostCommand::new(args.to_vec()), "虚拟机就绪探测")
        .is_ok()
}

/// 收尾：队列空闲且实例是作业自动拉起、又没有 Shell 占用时关闭虚拟机（省内存）。
fn stop_vm_when_idle(inner: &Arc<Mutex<Inner>>) {
    let active = {
        let guard = inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard
            .jobs
            .iter()
            .filter(|job| {
                job.status == kairos_core::models::jobs::JobStatus::Queued
                    || job.status == kairos_core::models::jobs::JobStatus::Running
            })
            .count()
    };
    if !vm_logic::should_stop_when_idle(
        super::vm_lease::auto_started(),
        active,
        super::vm_lease::shell_open(),
    ) {
        return;
    }
    let Some(provider) = vm_logic::provider_for(std::env::consts::OS) else {
        return;
    };
    let args = vm_logic::stop_args(provider);
    let stopped = super::vm::platform_command(&args[0])
        .args(&args[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if stopped {
        super::vm_lease::clear_auto_started();
    }
}

/// 宿主侧归档中转文件：`<临时目录>/kairos-<作业随机串>.tgz`。
#[cfg(target_os = "macos")]
fn host_transfer_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "kairos-{}.tgz",
        kairos_core::services::project::new_id("transfer").replace(':', "-")
    ))
}

/// 删宿主侧中转文件：失败不改变作业结果（临时目录由系统清理）。
#[cfg(target_os = "macos")]
fn remove_quietly(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// 把 case 目录复制进 VM 原生文件系统（macOS multipass 通道专用；multipass mount
/// 的 sshfs 权限映射不可用）。返回 VM 内路径：求解进程必须在虚拟机里 cd 到它，
/// 宿主绝对路径在 VM 内并不存在。大 case 可能耗时数分钟，只允许在作业线程调用
/// ——同步命令线程（主线程）绝不进入本函数。
///
/// 复制走「宿主打包 → `multipass transfer` 落盘 → 远端解压」三步，不用
/// stdin/stdout 管道：管道通道在远端命令结束后偶发不回退（CLI 进程自旋），
/// 每一步命令都必须是「跑完即返回」。
#[cfg(target_os = "macos")]
fn copy_case_into_vm(case_dir: &str, vm_shell: Option<&str>) -> Result<Option<String>> {
    let Some("multipass") = vm_shell else {
        return Ok(None);
    };
    let archive = host_transfer_path();
    // 步骤编排在 core（services::vm_run，与集成测试同一段代码）：打包 → transfer → 解压。
    // 本函数只负责「用哪个 runner」与中转文件的清理。
    let staged = vm_run::stage_case(&host_runner(), &archive, case_dir);
    remove_quietly(&archive);
    staged.map(Some)
}

#[cfg(not(target_os = "macos"))]
fn copy_case_into_vm(_case_dir: &str, _vm_shell: Option<&str>) -> Result<Option<String>> {
    Ok(None)
}

/// 把 VM 内求解产生的时间目录回传宿主 case 目录：结果写在 VM 原生文件系统，
/// 宿主侧 results 服务只认宿主目录。四步编排在 core（services::vm_run），
/// 与集成测试同一段代码；这里只提供 runner 与中转文件。
#[cfg(target_os = "macos")]
fn copy_results_from_vm(case_dir: &str, vm_case: Option<&str>) -> Result<()> {
    let Some(vm_case) = vm_case else {
        return Ok(());
    };
    let archive = host_transfer_path();
    let copied = vm_run::copy_results(&host_runner(), &archive, case_dir, vm_case);
    remove_quietly(&archive);
    copied.map(|_| ())
}

#[cfg(not(target_os = "macos"))]
fn copy_results_from_vm(_case_dir: &str, _vm_case: Option<&str>) -> Result<()> {
    Ok(())
}

/// 运行期上下文：受管 bin 目录前缀 + 执行通道（VM shell / 原生环境）。
/// 作业线程与收尾路径共用一份快照，避免长参数表在多个函数间传递。
#[derive(Clone)]
struct RunContext {
    /// 受管 bin 目录的 PATH 前缀（下载解压后由命令层注入）。
    managed_path: Option<String>,
    /// VM 执行通道：Some("multipass"/"wsl") 时作业在虚拟机内运行。
    vm_shell: Option<String>,
    /// 原生（Linux）求解环境根目录：Some 时脚本先 source 其 bashrc。
    native_env: Option<String>,
}

/// 原生求解启动参数（结构体承载，避免长参数表）。
struct NativeRun<'a> {
    case_dir: &'a str,
    vm_case: Option<&'a str>,
    cores: u32,
    context: &'a RunContext,
}

fn spawn_run_script(run: NativeRun<'_>) -> Result<Child> {
    let NativeRun {
        case_dir,
        vm_case,
        cores,
        context,
    } = run;
    let managed_path = context.managed_path.as_deref();
    let vm_shell = context.vm_shell.as_deref();
    let native_env = context.native_env.as_deref();
    let solve = moldingfoam::solve_command(cores);
    // VM 执行通道只在 macOS（multipass）与 Windows（WSL）存在；原生平台直接本机执行。
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    if let Some(vm_provider) = job_vm_provider(vm_shell) {
        let guest_dir = guest_case_dir(case_dir, vm_case);
        let inner = vm_logic::solve_script(&guest_dir, cores);
        let script = format!(
            "setsid bash -lc {}",
            kairos_core::utils::shell::bash_quote(&inner)
        );
        return host_command(&vm_logic::bash_script_args(vm_provider, &script))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| KairosError::io(format!("VM 求解启动失败：{e}")));
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = vm_shell;
    // 原生平台走宿主路径分支（同一处做 shell 单引号转义，防路径注入）。
    let safe_dir = vm_logic::script_case_dir(std::env::consts::OS, case_dir, vm_case);
    // 原生（Linux）环境：bundle 解压在本机时先 source 其 bashrc，
    // 布局与 VM 内一致（core 侧单点构造，路径含空格 / 引号时自动转义）。
    let env_source =
        native_env.map(|root| vm_logic::native_env_source_command(std::path::Path::new(root)));
    // 求解入口：foamRun 是 OpenFOAM 11+ 的模块化运行器，具体求解模块由
    // case 的 controlDict（solver 键，见 moldingfoam.rs::SOLVER_MODULE）提供；
    // 并行由 mpirun 发起（见 moldingfoam::solve_command）。
    let script =
        vm_logic::native_solve_script(&safe_dir, env_source.as_deref(), managed_path, &solve);
    let mut command = host_command(&vm_logic::bash_script_args(VmProviderKind::Native, &script));
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
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

impl JobScheduler {
    /// 锁的宽容获取：持锁线程 panic 导致中毒时取回内部数据继续（作业列表可重建，
    /// 中毒恢复优于让后续命令整体失效）。
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn arc(&self) -> Arc<Mutex<Inner>> {
        Arc::clone(&self.inner)
    }

    /// 提升排队作业并为其生成运行线程。作业线程内先做 VM tar 复制（可能耗时
    /// 数分钟）再拉起求解进程。提升过程只登记状态与线程，准备和求解不占用调度锁。
    fn promote_and_spawn(&self, now: u64) {
        let started = {
            let mut inner = self.lock();
            let limits = inner.limits;
            let started = job_logic::promote_ready(&mut inner.jobs, &limits, now);
            for id in &started {
                inner
                    .cancellations
                    .insert(id.clone(), Arc::new(AtomicBool::new(false)));
            }
            started
        };
        for job_id in started {
            let (case_dir, cores, context) = {
                let inner = self.lock();
                let job = inner.jobs.iter().find(|job| job.id == job_id);
                (
                    job.map(|job| job.case_dir.clone()).unwrap_or_default(),
                    job.map(|job| job.cores).unwrap_or(1),
                    RunContext {
                        managed_path: self.managed_path.clone(),
                        vm_shell: self.vm_shell.clone(),
                        native_env: inner.native_env.clone(),
                    },
                )
            };
            let inner = self.arc();
            thread::spawn(move || {
                // VM 通道（macOS/multipass）：求解脱离会话执行 + 日志轮询回读，
                // 不做「客户端守着求解进程」的流式管道（会话一回收求解就没了）。
                #[cfg(target_os = "macos")]
                if context.vm_shell.as_deref() == Some("multipass") {
                    let staged = ensure_vm_ready(&inner, &job_id, context.vm_shell.as_deref())
                        .and_then(|()| copy_case_into_vm(&case_dir, context.vm_shell.as_deref()));
                    match staged {
                        Ok(Some(vm_case)) => {
                            run_job_detached(inner, job_id, &vm_case, cores, case_dir, context)
                        }
                        Ok(None) => fail_and_promote(
                            inner,
                            job_id,
                            "虚拟机通道缺少 case 暂存路径。",
                            context,
                        ),
                        Err(e) => fail_and_promote(inner, job_id, e.message(), context),
                    }
                    return;
                }
                let staged = ensure_vm_ready(&inner, &job_id, context.vm_shell.as_deref())
                    .and_then(|()| copy_case_into_vm(&case_dir, context.vm_shell.as_deref()))
                    .and_then(|vm_case| {
                        check_cancelled(&inner, &job_id)?;
                        spawn_run_script(NativeRun {
                            case_dir: &case_dir,
                            vm_case: vm_case.as_deref(),
                            cores,
                            context: &context,
                        })
                        .map(|child| (child, vm_case))
                    });
                match staged {
                    Ok((child, vm_case)) => {
                        run_job_body(inner, job_id, child, case_dir, vm_case, context)
                    }
                    Err(e) => fail_and_promote(inner, job_id, e.message(), context),
                }
            });
        }
    }
}

/// 作业失败收尾：标记失败并立即尝试提升下一个排队作业（与正常结束路径的
/// 语义一致，避免队列因单个作业启动失败而停滞）。
fn fail_and_promote(inner: Arc<Mutex<Inner>>, job_id: String, message: &str, context: RunContext) {
    let now = now_ms();
    {
        let mut guard = inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if cancelled(&guard, &job_id) {
            let _ = job_logic::cancel(&mut guard.jobs, &job_id, now);
        } else {
            let _ = job_logic::mark_failed(&mut guard.jobs, &job_id, message, now);
        }
        guard.cancellations.remove(&job_id);
        guard.channels.remove(&job_id);
    }
    let scheduler = JobScheduler {
        inner: Arc::clone(&inner),
        managed_path: context.managed_path.clone(),
        vm_shell: context.vm_shell.clone(),
    };
    scheduler.set_native_env(context.native_env.clone());
    scheduler.promote_and_spawn(now);
    stop_vm_when_idle(&inner);
}

fn cancelled(inner: &Inner, job_id: &str) -> bool {
    inner
        .cancellations
        .get(job_id)
        .is_some_and(|token| token.load(Ordering::SeqCst))
}

fn check_cancelled(inner: &Arc<Mutex<Inner>>, job_id: &str) -> Result<()> {
    if cancelled(&inner.lock().unwrap_or_else(|e| e.into_inner()), job_id) {
        Err(KairosError::validation("用户取消"))
    } else {
        Ok(())
    }
}

fn forward_job_line(inner: &Arc<Mutex<Inner>>, job_id: &str, line: &str) {
    if let Some(time) = moldingfoam::parse_time_line(line) {
        let mut guard = inner.lock().unwrap_or_else(|e| e.into_inner());
        let _ = job_logic::update_progress(&mut guard.jobs, job_id, time);
    }
    send_job_line(inner, job_id, line);
}

fn drain_job_stream(pipe: impl std::io::Read + Send + 'static, tx: mpsc::SyncSender<String>) {
    thread::spawn(move || {
        for line in BufReader::new(pipe)
            .split(b'\n')
            .map_while(std::result::Result::ok)
        {
            let line = String::from_utf8_lossy(&line)
                .trim_end_matches('\r')
                .to_string();
            if tx.send(line).is_err() {
                break;
            }
        }
    });
}

fn consume_child(
    child: &mut Child,
    token: &AtomicBool,
    on_line: &mut dyn FnMut(&str),
    stop: &mut dyn FnMut(&mut Child) -> Result<()>,
) -> Result<(bool, bool)> {
    let (tx, rx) = mpsc::sync_channel(256);
    if let Some(stdout) = child.stdout.take() {
        drain_job_stream(stdout, tx.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        drain_job_stream(stderr, tx.clone());
    }
    drop(tx);
    let mut aborted = false;
    let mut stopping = false;
    let mut exit = None;
    let mut drained = false;
    while exit.is_none() || !drained {
        if token.load(Ordering::SeqCst) && !stopping {
            stop(child)?;
            stopping = true;
        }
        match rx.recv_timeout(std::time::Duration::from_millis(50)) {
            Ok(line) => {
                aborted |= moldingfoam::is_abort_line(&line);
                on_line(&line);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                drained = true;
                thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        exit = child
            .try_wait()
            .map_err(|e| KairosError::io(format!("等待求解退出失败：{e}")))?;
    }
    Ok((aborted, exit.is_some_and(|status| status.success())))
}

fn stop_remote_solver(provider: VmProviderKind, case_dir: &str) -> Result<()> {
    let mut runner = host_runner();
    runner.timeout = std::time::Duration::from_secs(10);
    // 启动命令返回和 SID 落盘之间有短窗口，重试直到可终止或给出明确失败。
    let command = vm_run::HostCommand::new(vm_logic::bash_script_args(
        provider,
        &vm_logic::solver_stop_command(case_dir),
    ));
    for _ in 0..3 {
        if runner.run(&command, "取消远端求解").is_ok() {
            return Ok(());
        }
        thread::sleep(std::time::Duration::from_millis(100));
    }
    Err(KairosError::io(
        "无法确认远端求解已停止，请检查虚拟机中的运行进程。",
    ))
}

fn guest_case_dir(case_dir: &str, vm_case: Option<&str>) -> String {
    vm_case.map(str::to_owned).unwrap_or_else(|| {
        kairos_core::services::paths::wsl_path(case_dir).unwrap_or_else(|| case_dir.to_string())
    })
}

fn terminate_solver(
    child: &mut Child,
    case_dir: &str,
    vm_case: Option<&str>,
    context: &RunContext,
) -> Result<()> {
    if let Some(provider) = job_vm_provider(context.vm_shell.as_deref()) {
        let path = match provider {
            VmProviderKind::Wsl => guest_case_dir(case_dir, None),
            _ => vm_case.unwrap_or(case_dir).to_string(),
        };
        stop_remote_solver(provider, &path)?;
    }
    terminate_local(child)
}

fn terminate_local(child: &mut Child) -> Result<()> {
    if child
        .try_wait()
        .map_err(|e| KairosError::io(e.to_string()))?
        .is_some()
    {
        return Ok(());
    }
    #[cfg(unix)]
    let status = Command::new("kill")
        .args(["-KILL", "--", &format!("-{}", child.id())])
        .status();
    #[cfg(windows)]
    let status = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .status();
    let status = status.map_err(|e| KairosError::io(format!("终止求解失败：{e}")))?;
    if !status.success() {
        child
            .kill()
            .map_err(|e| KairosError::io(format!("终止求解失败：{e}")))?;
    }
    child
        .wait()
        .map_err(|e| KairosError::io(format!("回收求解失败：{e}")))?;
    Ok(())
}

/// 单作业运行主体：流式回传日志与进度，收尾后写回状态并提升下一个排队作业。
fn run_job_body(
    inner: Arc<Mutex<Inner>>,
    job_id: String,
    mut child: Child,
    case_dir: String,
    vm_case: Option<String>,
    context: RunContext,
) {
    let token = {
        let guard = inner.lock().unwrap_or_else(|e| e.into_inner());
        Arc::clone(&guard.cancellations[&job_id])
    };
    let outcome = consume_child(
        &mut child,
        &token,
        &mut |line| {
            forward_job_line(&inner, &job_id, line);
        },
        &mut |child| terminate_solver(child, &case_dir, vm_case.as_deref(), &context),
    );
    let (solver_aborted, exit_ok) = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            token.store(false, Ordering::SeqCst);
            fail_and_promote(inner, job_id, error.message(), context);
            return;
        }
    };
    finish_job(
        inner,
        job_id,
        solver_aborted,
        exit_ok,
        case_dir,
        vm_case,
        context,
    );
}

/// VM 通道（macOS / multipass）的求解：脱离会话执行 + 日志流式回读。
///
/// 直接 `multipass exec … foamRun` 的求解进程是该 ssh 会话的子进程：客户端一退出、
/// systemd-logind 回收会话，求解就在跑到一半时无声消失。这里改为 `setsid` 独立会话
/// 启动（见 core 的 `detached_launch_command`），日志与退出码落在 case 内，客户端
/// 只负责轮询增量日志与退出码——求解不再依赖会话存活，进度与失败判定都有据可依。
#[cfg(target_os = "macos")]
fn run_job_detached(
    inner: Arc<Mutex<Inner>>,
    job_id: String,
    vm_case: &str,
    cores: u32,
    case_dir: String,
    context: RunContext,
) {
    if let Err(error) = check_cancelled(&inner, &job_id) {
        fail_and_promote(inner, job_id, error.message(), context);
        return;
    }
    let script = vm_logic::solve_script(vm_case, cores);
    let launch = vm_logic::detached_launch_command(vm_case, &script);
    // 启动命令自身立即返回（求解在 setsid 会话里跑）：走 core 的 runner 拿统一判定与超时。
    let launched = host_runner().run(
        &vm_run::HostCommand::new(vm_logic::bash_script_args(
            VmProviderKind::Multipass,
            &launch,
        )),
        "VM 求解启动",
    );
    if let Err(e) = launched {
        fail_and_promote(inner, job_id, e.message(), context);
        return;
    }
    let mut solver_aborted = false;
    let mut offset = 0u64;
    let mut exit_code: Option<i32> = None;
    let mut failures = 0;
    while exit_code.is_none() {
        if check_cancelled(&inner, &job_id).is_err() {
            if let Err(error) = stop_remote_solver(VmProviderKind::Multipass, vm_case) {
                let guard = inner.lock().unwrap_or_else(|e| e.into_inner());
                guard.cancellations[&job_id].store(false, Ordering::SeqCst);
                drop(guard);
                fail_and_promote(inner, job_id, error.message(), context);
                return;
            }
            break;
        }
        match stream_vm_log(&inner, &job_id, vm_case, offset) {
            Ok((aborted, new_offset, code)) => {
                failures = 0;
                solver_aborted |= aborted;
                offset = new_offset;
                exit_code = code;
            }
            // 短暂通道错误允许重试；连续失败则明确报告状态未知，不再无限轮询。
            Err(e) => {
                failures += 1;
                send_job_line(
                    &inner,
                    &job_id,
                    &format!("── 日志回读失败（{failures}/3）：{}", e.message()),
                );
                if failures >= 3 {
                    fail_and_promote(
                        inner,
                        job_id,
                        "无法读取远端求解状态，请检查虚拟机中的运行进程。",
                        context,
                    );
                    return;
                }
            }
        }
        if exit_code.is_none() {
            thread::sleep(std::time::Duration::from_millis(VM_LOG_POLL_MS));
        }
    }
    finish_job(
        inner,
        job_id,
        solver_aborted,
        exit_code == Some(0),
        case_dir,
        Some(vm_case.to_string()),
        context,
    );
}

/// 回读一段增量日志：转发到作业日志通道、更新进度，并报告「有求解器错误标记 /
/// 最新偏移 / 退出码（None = 仍在求解）」。一轮只扫一次日志行——
/// 读命令、哨兵解析与错误标记判定都在 core（`vm_run::poll_log_once`），本函数只做
/// 作业状态与通道的副作用。
#[cfg(target_os = "macos")]
fn stream_vm_log(
    inner: &Arc<Mutex<Inner>>,
    job_id: &str,
    vm_case: &str,
    offset: u64,
) -> Result<(bool, u64, Option<i32>)> {
    let mut runner = host_runner();
    runner.timeout = std::time::Duration::from_secs(10);
    let poll = vm_run::poll_log_once(&runner, vm_case, offset)?;
    for line in &poll.lines {
        let forward = {
            let mut guard = inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(time_s) = moldingfoam::parse_time_line(line) {
                let _ = job_logic::update_progress(&mut guard.jobs, job_id, time_s);
            }
            guard.channels.get(job_id).cloned()
        };
        if let Some(channel) = forward {
            let _ = channel.send(line.clone());
        }
    }
    Ok((poll.aborted, poll.offset, poll.exit_code))
}

/// 日志回读轮询间隔（毫秒）：作业日志要「看着在动」，也不能把 VM 打满。
#[cfg(target_os = "macos")]
const VM_LOG_POLL_MS: u64 = 1000;

/// 求解结束后的统一收尾：回传结果 → 判定成败 → 写回状态 → 提升下一个 → 空闲关实例。
///
/// 求解产出写在 VM 原生文件系统里，回传宿主后 results 服务才读得到；求解失败时
/// 也走一遍回传（已写出的部分时间目录对排查有用），但只有成功路径把回传失败当作
/// 作业失败，避免用回传问题覆盖求解本身的失败原因。判定收敛在 core
/// （求解器错误标记优先于退出码，见 job_logic::job_failure）。
fn finish_job(
    inner: Arc<Mutex<Inner>>,
    job_id: String,
    solver_aborted: bool,
    exit_ok: bool,
    case_dir: String,
    vm_case: Option<String>,
    context: RunContext,
) {
    let copy_back = copy_results_from_vm(&case_dir, vm_case.as_deref());
    let copy_back_error = copy_back.err().map(|error| error.message().to_string());
    let failure = job_logic::job_failure(solver_aborted, exit_ok, copy_back_error);
    let now = now_ms();
    {
        let mut guard = inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if cancelled(&guard, &job_id) {
            let _ = job_logic::cancel(&mut guard.jobs, &job_id, now);
        } else {
            match &failure {
                None => {
                    let _ = job_logic::mark_done(&mut guard.jobs, &job_id, now);
                }
                Some(message) => {
                    let _ = job_logic::mark_failed(&mut guard.jobs, &job_id, message, now);
                }
            }
        }
        guard.cancellations.remove(&job_id);
        guard.channels.remove(&job_id);
    }
    // 一个作业结束 → 立即尝试提升队列中的下一个（自动续跑）
    let scheduler = JobScheduler {
        inner: Arc::clone(&inner),
        managed_path: context.managed_path.clone(),
        vm_shell: context.vm_shell.clone(),
    };
    scheduler.set_native_env(context.native_env.clone());
    scheduler.promote_and_spawn(now);
    // 收尾在提升之后：队列里还有活儿时这次调用会跳过，由最后一个作业关闭实例。
    stop_vm_when_idle(&inner);
}

/// 提交求解作业：入队并按预算立即尝试启动。progress 通道回传日志行与 __TIME__ 进度标记。
#[tauri::command(async)]
pub fn submit_job(
    app: tauri::AppHandle,
    scheduler: State<'_, JobScheduler>,
    case_dir: String,
    cores: u32,
    study_id: Option<String>,
    progress: Channel<String>,
) -> Result<Job> {
    // 提交前刷新原生求解环境：刚下载 / 解压 bundle 的会话也能直接跑（Linux 通道）。
    refresh_native_env(&app);
    let case_dir = std::fs::canonicalize(&case_dir)
        .map_err(|e| KairosError::io(format!("求解目录不可用：{e}")))?
        .to_string_lossy()
        .to_string();
    let id = new_id("job");
    let now = now_ms();
    let job = {
        let mut inner = scheduler.lock();
        if inner.jobs.iter().any(|job| {
            job.case_dir == case_dir
                && matches!(
                    job.status,
                    kairos_core::models::jobs::JobStatus::Queued
                        | kairos_core::models::jobs::JobStatus::Running
                )
        }) {
            return Err(KairosError::validation("该目录已有排队或运行中的作业。"));
        }
        job_logic::submit(&mut inner.jobs, id.clone(), study_id, case_dir, cores, now)?;
        inner.channels.insert(id.clone(), progress);
        // 提升只发生在 promote_and_spawn 里（提升与起线程必须同一处）：
        // 在这里先提升会把作业置成 Running 却不带线程，随后 promote_and_spawn
        // 看不到待提升作业，作业就永远停在「运行中」。
        inner
            .jobs
            .iter()
            .find(|job| job.id == id)
            .cloned()
            .ok_or_else(|| KairosError::internal("刚提交的作业查询失败。"))?
    };
    scheduler.promote_and_spawn(now);
    Ok(job)
}

/// 取消作业：先终止求解进程，再迁移状态；排队中的直接取消。
#[tauri::command]
pub fn cancel_job(scheduler: State<'_, JobScheduler>, job_id: String) -> Result<()> {
    let mut inner = scheduler.lock();
    let status = inner
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .ok_or_else(|| KairosError::not_found("作业不存在。"))?
        .status;
    if status == kairos_core::models::jobs::JobStatus::Running {
        inner.cancellations[&job_id].store(true, Ordering::SeqCst);
        return Ok(());
    }
    job_logic::cancel(&mut inner.jobs, &job_id, now_ms())?;
    inner.channels.remove(&job_id);
    Ok(())
}

#[tauri::command]
pub fn list_jobs(scheduler: State<'_, JobScheduler>) -> Result<Vec<Job>> {
    Ok(scheduler.lock().jobs.clone())
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    fn test_child(script: &str) -> std::process::Child {
        use std::os::unix::process::CommandExt;
        std::process::Command::new("bash")
            .args(["-c", script])
            .process_group(0)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_reaps_a_real_process_even_after_it_closes_output() {
        let mut child = test_child("exec 1>&- 2>&-; sleep 20");
        let token = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let request = token.clone();
        let sender = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            request.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        let started = std::time::Instant::now();
        let (_, success) =
            super::consume_child(&mut child, &token, &mut |_| {}, &mut super::terminate_local)
                .unwrap();
        sender.join().unwrap();
        assert!(!success);
        assert!(child.try_wait().unwrap().is_some());
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }

    #[cfg(unix)]
    #[test]
    fn drains_large_stderr_and_detects_fatal_errors() {
        let mut child = test_child(
            "for ((i=0;i<20000;i++)); do echo diagnostic-diagnostic-diagnostic >&2; done; echo 'FOAM FATAL ERROR' >&2; echo 'Time = 1' ",
        );
        let token = std::sync::atomic::AtomicBool::new(false);
        let mut count = 0;
        let (aborted, success) = super::consume_child(
            &mut child,
            &token,
            &mut |_| {
                count += 1;
            },
            &mut super::terminate_local,
        )
        .unwrap();
        assert!(aborted);
        assert!(success);
        assert_eq!(count, 20002);
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_diagnostics_do_not_truncate_the_stream() {
        let mut child =
            test_child("printf '\\377diagnostic\\r\\n'; echo 'FOAM FATAL ERROR'; echo 'Time = 2'");
        let mut lines = Vec::new();
        let (aborted, success) = super::consume_child(
            &mut child,
            &std::sync::atomic::AtomicBool::new(false),
            &mut |line| lines.push(line.to_string()),
            &mut super::terminate_local,
        )
        .unwrap();
        assert!(aborted && success);
        assert_eq!(lines, ["�diagnostic", "FOAM FATAL ERROR", "Time = 2"]);
    }

    use super::*;

    /// 作业必须由 promote_and_spawn 提升并起线程：先提升（Running）再让
    /// promote_and_spawn 找不到排队作业，作业会永远停在「运行中」。
    #[test]
    fn promote_and_spawn_always_runs_the_promoted_job() {
        let scheduler = JobScheduler::default();
        let case_dir = std::env::temp_dir().join(format!("kairos-job-{}", std::process::id()));
        std::fs::create_dir_all(&case_dir).unwrap();
        let now = now_ms();
        {
            let mut inner = scheduler.lock();
            job_logic::submit(
                &mut inner.jobs,
                "spawn-check".into(),
                None,
                case_dir.to_string_lossy().to_string(),
                1,
                now,
            )
            .unwrap();
        }
        scheduler.promote_and_spawn(now);
        // 线程起进程后很快收尾：case 目录里没有 OpenFOAM 环境，脚本必然失败退出。
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let status = loop {
            let status = scheduler.lock().jobs[0].status;
            if status != kairos_core::models::jobs::JobStatus::Running {
                break status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "作业停在运行中：提升与起线程不在同一处"
            );
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        assert_eq!(status, kairos_core::models::jobs::JobStatus::Failed);
        std::fs::remove_dir_all(&case_dir).ok();
    }
}
