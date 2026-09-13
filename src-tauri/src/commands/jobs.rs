//! 求解作业调度器：队列、并发预算与作业生命周期。
//! core（services::jobs）负责状态机与并发策略的纯逻辑；本模块负责进程副作用与进度回传。

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
#[cfg(target_os = "macos")]
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use kairos_core::error::{KairosError, Result};
use kairos_core::models::jobs::Job;
use kairos_core::services::jobs as job_logic;
use kairos_core::services::jobs::SchedulerLimits;
use kairos_core::services::moldingfoam;
use kairos_core::services::paths;
use kairos_core::services::project::new_id;
// 结果回传只在 macOS（multipass）通道用得上；VM 路径 macOS 与 Windows 都用
#[cfg(target_os = "macos")]
use kairos_core::services::results;
// VM 通道（macOS/Windows 的 env source）与原生环境（Linux 的 source 行）都用它，
// 因此导入**不带 cfg**——只在 cfg 块里引用会让 Linux 构建找不到符号。
use kairos_core::services::vm as vm_logic;
use tauri::State;
use tauri::ipc::Channel;

struct Inner {
    jobs: Vec<Job>,
    children: HashMap<String, Child>,
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
                children: HashMap::new(),
                channels: HashMap::new(),
                limits: SchedulerLimits::new(2, 8),
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

/// Windows 盘符路径 → WSL 的 /mnt 形态：映射逻辑在 core（纯路径策略，
/// 在 macOS 上也能测——Unix 解析不出 `Component::Prefix`，实现里两种形态都认）。
fn to_wsl_path(path: &str) -> String {
    paths::wsl_path(path).unwrap_or_else(|| path.to_string())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 把 case 目录复制进 VM 原生文件系统（tar 管道，macOS multipass 通道专用；
/// multipass mount 的 sshfs 权限映射不可用）。返回 VM 内路径：求解进程必须在
/// 虚拟机里 cd 到它，宿主绝对路径在 VM 内并不存在。大 case 可能耗时数分钟，
/// 只允许在作业线程调用——同步命令线程（主线程）绝不进入本函数。
#[cfg(target_os = "macos")]
fn copy_case_into_vm(case_dir: &str, vm_shell: Option<&str>) -> Result<Option<String>> {
    let Some("multipass") = vm_shell else {
        return Ok(None);
    };
    let parent = Path::new(case_dir)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let name = Path::new(case_dir)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "case".into());
    let vm_case = vm_logic::vm_case_dir(case_dir);
    let mut tar_cmd = Command::new("tar");
    tar_cmd
        .arg("-C")
        .arg(&parent)
        .arg("-czf")
        .arg("-")
        .arg(&name);
    let mut tar_child = tar_cmd
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| KairosError::io(format!("tar 启动失败：{e}")))?;
    let tar_stdout = tar_child
        .stdout
        .take()
        .ok_or_else(|| KairosError::io("tar stdout 管道不可用。"))?;
    // 环境树由 vm_deploy_bundle 预先解压在 ~/moldingfoam-env。
    // 先删掉 VM 内同名目录：否则重跑时残留的上一次时间目录会被回传逻辑当作
    // 本次结果，与本次结果混在同一个 case 目录里。
    let mut mp = Command::new("multipass");
    mp.args([
        "exec",
        "kairos",
        "--",
        "bash",
        "-lc",
        &format!("rm -rf '{vm_case}' && mkdir -p '{vm_case}' && tar -xzf - -C '{vm_case}'"),
    ])
    .stdin(tar_stdout);
    let status = mp
        .status()
        .map_err(|e| KairosError::io(format!("multipass 启动失败：{e}")))?;
    let _ = tar_child.wait();
    if !status.success() {
        return Err(KairosError::io("case 目录复制进虚拟机失败。"));
    }
    Ok(Some(vm_case))
}

#[cfg(not(target_os = "macos"))]
fn copy_case_into_vm(_case_dir: &str, _vm_shell: Option<&str>) -> Result<Option<String>> {
    Ok(None)
}

/// 把 VM 内求解产生的时间目录回传宿主 case 目录：结果写在 VM 原生文件系统，
/// 宿主侧 results 服务只认宿主目录。时间目录名由 core 的
/// `results::time_dir_names` 筛查（与结果扫描同一套判定）。
#[cfg(target_os = "macos")]
fn copy_results_from_vm(case_dir: &str, vm_case: Option<&str>) -> Result<()> {
    let Some(vm_case) = vm_case else {
        return Ok(());
    };
    let safe_vm_case = vm_case.replace('\'', "'\\''");
    let listing = Command::new("multipass")
        .args([
            "exec",
            "kairos",
            "--",
            "bash",
            "-lc",
            &format!("cd '{safe_vm_case}' && ls -d [0-9]* 2>/dev/null"),
        ])
        .output()
        .map_err(|e| KairosError::io(format!("multipass 启动失败：{e}")))?;
    let names = results::time_dir_names(&String::from_utf8_lossy(&listing.stdout));
    if !listing.status.success() || names.is_empty() {
        return Err(KairosError::io(
            "虚拟机内没有可回传的结果时间目录（求解未产生输出）。",
        ));
    }
    let mut pack = Command::new("multipass")
        .args([
            "exec",
            "kairos",
            "--",
            "bash",
            "-lc",
            &format!("cd '{safe_vm_case}' && tar -czf - {}", names.join(" ")),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| KairosError::io(format!("multipass 启动失败：{e}")))?;
    let pack_stdout = pack
        .stdout
        .take()
        .ok_or_else(|| KairosError::io("multipass stdout 管道不可用。"))?;
    let unpack = Command::new("tar")
        .args(["-xzf", "-", "-C", case_dir])
        .stdin(pack_stdout)
        .status()
        .map_err(|e| KairosError::io(format!("tar 启动失败：{e}")))?;
    let _ = pack.wait();
    if !unpack.success() {
        return Err(KairosError::io("求解结果回传宿主失败。"));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn copy_results_from_vm(_case_dir: &str, _vm_case: Option<&str>) -> Result<()> {
    Ok(())
}

/// 求解脚本里的 case 目录（含 shell 单引号转义）。
///
/// 平台经 `os` 传入（与 `std::env::consts::OS` 同口径）而不是编译期分支——三端
/// 取值都能在同一平台单测覆盖，且函数在原生平台（走宿主路径分支）同样被调用，
/// 不会因为只有 VM 平台用得上而被判成死代码：
/// - macOS：VM 内暂存路径（宿主绝对路径在虚拟机里并不存在）；
/// - Windows：WSL 的 `/mnt` 形态路径（WSL 能直接读宿主文件系统）；
/// - 原生：宿主路径。
fn script_case_dir(os: &str, case_dir: &str, vm_case: Option<&str>) -> String {
    let dir = match os {
        "macos" => vm_case.unwrap_or(case_dir),
        "windows" => &to_wsl_path(case_dir),
        _ => case_dir,
    };
    dir.replace('\'', "'\\''")
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

/// 原生求解脚本：`[source env; ] [export PATH; ] cd '<case>' && <solve>`。
/// 顺序固定——环境 source 在最前（它决定 OpenFOAM 的 PATH / LD_LIBRARY_PATH），
/// 受管 bin 目录随后追加，避免被环境树的 PATH 覆盖。
fn native_solve_script(
    case_dir: &str,
    env_source: Option<&str>,
    managed_path: Option<&str>,
    solve: &str,
) -> String {
    let source = env_source
        .map(|command| format!("{command}; "))
        .unwrap_or_default();
    let path_export = managed_path
        .map(|prefix| format!("export PATH='{prefix}:$PATH'; "))
        .unwrap_or_default();
    format!("{source}{path_export}cd '{case_dir}' && {solve}")
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
    if let Some(shell) = vm_shell {
        let inner = format!(
            "{} && cd '{}' && {solve}",
            vm_logic::env_source_command(),
            script_case_dir(std::env::consts::OS, case_dir, vm_case)
        );
        let mut command = Command::new(shell);
        #[cfg(target_os = "macos")]
        command.args(["exec", "kairos", "--", "bash", "-lc", &inner]);
        #[cfg(target_os = "windows")]
        command.args([
            "-d",
            kairos_core::services::vm::WSL_DISTRO,
            "bash",
            "-lc",
            &inner,
        ]);
        return command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| KairosError::io(format!("VM 求解启动失败：{e}")));
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = vm_shell;
    // 原生平台走宿主路径分支（同一处做 shell 单引号转义，防路径注入）。
    let safe_dir = script_case_dir(std::env::consts::OS, case_dir, vm_case);
    // 原生（Linux）环境：bundle 解压在本机时先 source 其 bashrc，
    // 布局与 VM 内一致（core 侧单点构造，路径含空格 / 引号时自动转义）。
    let env_source =
        native_env.map(|root| vm_logic::native_env_source_command(std::path::Path::new(root)));
    // 求解入口：foamRun 是 OpenFOAM 11+ 的模块化运行器，具体求解模块由
    // case 的 controlDict（solver 键，见 moldingfoam.rs::SOLVER_MODULE）提供；
    // 并行由 mpirun 发起（见 moldingfoam::solve_command）。
    let script = native_solve_script(&safe_dir, env_source.as_deref(), managed_path, &solve);
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
    /// 数分钟）再拉起求解进程——同步命令 submit_job 在主线程调用本函数，
    /// 因此任何重活都不允许留在 promote 路径上。
    fn promote_and_spawn(&self, now: u64) {
        let started = {
            let mut inner = self.lock();
            let limits = inner.limits;
            job_logic::promote_ready(&mut inner.jobs, &limits, now)
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
                let staged =
                    copy_case_into_vm(&case_dir, context.vm_shell.as_deref()).and_then(|vm_case| {
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
        let _ = job_logic::mark_failed(&mut guard.jobs, &job_id, message, now);
    }
    let scheduler = JobScheduler {
        inner: Arc::clone(&inner),
        managed_path: context.managed_path.clone(),
        vm_shell: context.vm_shell.clone(),
    };
    scheduler.set_native_env(context.native_env.clone());
    scheduler.promote_and_spawn(now);
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
    let mut stdout = child.stdout.take();
    let mut solver_aborted = false;
    if let Some(pipe) = stdout.take() {
        let reader = BufReader::new(pipe);
        for line in reader.lines().map_while(std::result::Result::ok) {
            if moldingfoam::is_abort_line(&line) {
                solver_aborted = true;
            }
            let time_s = moldingfoam::parse_time_line(&line);
            let forward = {
                let mut guard = inner
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if let Some(time_s) = time_s {
                    let _ = job_logic::update_progress(&mut guard.jobs, &job_id, time_s);
                }
                guard.channels.get(&job_id).cloned()
            };
            if let Some(channel) = forward {
                let _ = channel.send(line.clone());
            }
        }
    }
    let exit_ok = child.wait().map(|status| status.success()).unwrap_or(false);
    // 结果写在 VM 原生文件系统里，回传宿主后 results 服务才读得到；求解失败时
    // 也走一遍回传——已写出的部分时间目录对排查有用——但只有成功路径把回传
    // 失败当作作业失败，避免用回传问题覆盖求解本身的失败原因。
    let copy_back = copy_results_from_vm(&case_dir, vm_case.as_deref());
    let copy_back_error = copy_back.err().map(|error| error.message().to_string());
    // 求解失败按输出里的异常标记区分原因：求解器主动报错（FOAM FATAL 等）与
    // 进程被终止 / 崩溃（无任何求解器错误标记）；成功路径下回传失败仍算失败，
    // 避免静默丢结果。
    let failure = if exit_ok {
        copy_back_error
    } else if solver_aborted {
        Some("求解器报错退出（输出含 FOAM FATAL，详见作业日志）".to_string())
    } else {
        Some("进程异常退出（输出无求解器错误标记）".to_string())
    };
    let now = now_ms();
    {
        let mut guard = inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match &failure {
            None => {
                let _ = job_logic::mark_done(&mut guard.jobs, &job_id, now);
            }
            Some(message) => {
                let _ = job_logic::mark_failed(&mut guard.jobs, &job_id, message, now);
            }
        }
        guard.children.remove(&job_id);
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
}

/// 提交求解作业：入队并按预算立即尝试启动。progress 通道回传日志行与 __TIME__ 进度标记。
#[tauri::command]
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
    let id = new_id("job");
    let now = now_ms();
    let job = {
        let mut inner = scheduler.lock();
        job_logic::submit(&mut inner.jobs, id.clone(), study_id, case_dir, cores, now)?;
        inner.channels.insert(id.clone(), progress);
        let limits = inner.limits;
        // submit 里已尝试提升，启动在 promote_and_spawn 的作业线程中进行。
        job_logic::promote_ready(&mut inner.jobs, &limits, now);
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

/// 取消作业：运行中的先终止进程组，再迁移状态；排队中的直接取消。
#[tauri::command]
pub fn cancel_job(scheduler: State<'_, JobScheduler>, job_id: String) -> Result<()> {
    let child = scheduler.lock().children.remove(&job_id);
    if let Some(mut child) = child {
        let pid = child.id();
        // unix：进程组整杀；windows：taskkill 树杀（含 decomposePar/solver 子进程）。
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-9", &format!("-{pid}")])
                .status();
        }
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .status();
        }
        let _ = child.kill();
        let _ = child.wait();
    }
    let mut inner = scheduler.lock();
    job_logic::cancel(&mut inner.jobs, &job_id, now_ms())
}

#[tauri::command]
pub fn list_jobs(scheduler: State<'_, JobScheduler>) -> Result<Vec<Job>> {
    Ok(scheduler.lock().jobs.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 原生求解脚本：环境 source 在最前，随后 PATH 导出、cd、求解命令。
    #[test]
    fn native_solve_script_orders_source_path_and_solve() {
        let script = native_solve_script(
            "/data/cases/study-1",
            Some("source '/opt/moldingfoam-env/openfoam14/etc/bashrc'"),
            Some("/data/downloads/moldingfoam/bin"),
            "decomposePar -force && mpirun -np 4 foamRun -parallel; reconstructPar",
        );
        assert_eq!(
            script,
            "source '/opt/moldingfoam-env/openfoam14/etc/bashrc'; export PATH='/data/downloads/moldingfoam/bin:$PATH'; cd '/data/cases/study-1' && decomposePar -force && mpirun -np 4 foamRun -parallel; reconstructPar"
        );
    }

    /// 无环境 / 无受管目录（VM 通道未介入时的原生回退）只留 cd + 求解；
    /// 单引号路径原样透传（转义已由 script_case_dir 完成）。
    #[test]
    fn native_solve_script_without_optional_segments() {
        let script = native_solve_script("/cases/plain", None, None, "foamRun");
        assert_eq!(script, "cd '/cases/plain' && foamRun");
        let env_only = native_solve_script(
            "/cases/a",
            Some("source '/env/openfoam14/etc/bashrc'"),
            None,
            "foamRun",
        );
        assert_eq!(
            env_only,
            "source '/env/openfoam14/etc/bashrc'; cd '/cases/a' && foamRun"
        );
    }

    #[test]
    fn script_case_dir_picks_path_per_platform() {
        // macOS：VM 内暂存路径（宿主路径在虚拟机里不存在）；没有暂存路径时退回宿主路径
        assert_eq!(
            script_case_dir("macos", "/host/study-1", Some("/home/ubuntu/study-1")),
            "/home/ubuntu/study-1"
        );
        assert_eq!(
            script_case_dir("macos", "/host/study-1", None),
            "/host/study-1"
        );
        // Windows：WSL 的 /mnt 形态（WSL 能直接读宿主文件系统）
        assert_eq!(
            script_case_dir("windows", r"C:\cases\study-1", None),
            "/mnt/c/cases/study-1"
        );
        // 原生 Linux：宿主路径
        assert_eq!(
            script_case_dir("linux", "/data/cases/study-1", None),
            "/data/cases/study-1"
        );
        // 单引号转义：拼进 bash -lc 脚本前必须处理，防路径注入
        assert_eq!(
            script_case_dir("linux", "/data/it's", None),
            r"/data/it'\''s"
        );
    }

    #[test]
    fn wsl_path_maps_drive_letters_only() {
        assert_eq!(to_wsl_path(r"C:\a\b"), "/mnt/c/a/b");
        assert_eq!(to_wsl_path(r"D:\cases"), "/mnt/d/cases");
        assert_eq!(to_wsl_path("/already/unix"), "/already/unix");
    }
}
