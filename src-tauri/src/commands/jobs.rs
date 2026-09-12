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
use kairos_core::services::project::new_id;
use kairos_core::services::results;
use kairos_core::services::vm as vm_logic;
use tauri::State;
use tauri::ipc::Channel;

struct Inner {
    jobs: Vec<Job>,
    children: HashMap<String, Child>,
    channels: HashMap<String, Channel<String>>,
    limits: SchedulerLimits,
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

/// Windows 盘符路径 → WSL 的 /mnt 形态（C:\a\b → /mnt/c/a/b）。
#[cfg(target_os = "windows")]
fn to_wsl_path(path: &str) -> String {
    let lower = path.replace('\\', "/");
    let bytes = lower.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        format!(
            "/mnt/{}{}",
            (bytes[0] as char).to_ascii_lowercase(),
            &lower[2..]
        )
    } else {
        path.to_string()
    }
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

fn spawn_run_script(
    case_dir: &str,
    vm_case: Option<&str>,
    cores: u32,
    managed_path: Option<&str>,
    vm_shell: Option<&str>,
) -> Result<Child> {
    let solve = moldingfoam::solve_command(cores);
    if let Some(shell) = vm_shell {
        // macOS：宿主路径在 VM 内不存在，必须用 tar 复制后的 VM 路径。
        #[cfg(target_os = "macos")]
        let safe_dir = vm_case.unwrap_or(case_dir).replace('\'', "'\\''");
        // Windows：WSL 能直接读宿主文件系统，用 /mnt 形态路径。
        #[cfg(target_os = "windows")]
        let safe_dir = to_wsl_path(case_dir).replace('\'', "'\\''");
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let inner = {
            let source = vm_logic::env_source_command();
            format!("{source} && cd '{safe_dir}' && {solve}")
        };
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
    let path_export = managed_path
        .map(|prefix| format!("export PATH='{prefix}:$PATH'; "))
        .unwrap_or_default();
    // 单引号内的 shell 转义：' → '\''（防路径注入）。
    let safe_dir = case_dir.replace('\'', "'\\''");
    // 求解入口：foamRun 是 OpenFOAM 11+ 的模块化运行器，具体求解模块由
    // case 的 controlDict（solver 键，见 moldingfoam.rs::SOLVER_MODULE）提供；
    // 并行由 mpirun 发起（见 moldingfoam::solve_command）。
    let script = format!("{path_export}cd '{safe_dir}' && {solve}");
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
            let (case_dir, cores, managed_path, vm_shell) = {
                let inner = self.lock();
                let job = inner.jobs.iter().find(|job| job.id == job_id);
                (
                    job.map(|job| job.case_dir.clone()).unwrap_or_default(),
                    job.map(|job| job.cores).unwrap_or(1),
                    self.managed_path.clone(),
                    self.vm_shell.clone(),
                )
            };
            let inner = self.arc();
            thread::spawn(move || {
                let staged =
                    copy_case_into_vm(&case_dir, vm_shell.as_deref()).and_then(|vm_case| {
                        spawn_run_script(
                            &case_dir,
                            vm_case.as_deref(),
                            cores,
                            managed_path.as_deref(),
                            vm_shell.as_deref(),
                        )
                        .map(|child| (child, vm_case))
                    });
                match staged {
                    Ok((child, vm_case)) => run_job_body(
                        inner,
                        job_id,
                        child,
                        case_dir,
                        vm_case,
                        managed_path,
                        vm_shell,
                    ),
                    Err(e) => fail_and_promote(inner, job_id, e.message(), managed_path, vm_shell),
                }
            });
        }
    }
}

/// 作业失败收尾：标记失败并立即尝试提升下一个排队作业（与正常结束路径的
/// 语义一致，避免队列因单个作业启动失败而停滞）。
fn fail_and_promote(
    inner: Arc<Mutex<Inner>>,
    job_id: String,
    message: &str,
    managed_path: Option<String>,
    vm_shell: Option<String>,
) {
    let now = now_ms();
    {
        let mut guard = inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = job_logic::mark_failed(&mut guard.jobs, &job_id, message, now);
    }
    let scheduler = JobScheduler {
        inner,
        managed_path,
        vm_shell,
    };
    scheduler.promote_and_spawn(now);
}

/// 单作业运行主体：流式回传日志与进度，收尾后写回状态并提升下一个排队作业。
fn run_job_body(
    inner: Arc<Mutex<Inner>>,
    job_id: String,
    mut child: Child,
    case_dir: String,
    vm_case: Option<String>,
    managed_path: Option<String>,
    vm_shell: Option<String>,
) {
    let mut stdout = child.stdout.take();
    let mut abort_marker = false;
    let mut completion_marker = false;
    if let Some(pipe) = stdout.take() {
        let reader = BufReader::new(pipe);
        for line in reader.lines().map_while(std::result::Result::ok) {
            match moldingfoam::solver_signal(&line) {
                Some(moldingfoam::SolverSignal::Aborted) => abort_marker = true,
                Some(moldingfoam::SolverSignal::Completed) => completion_marker = true,
                None => {}
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
    // 求解器退出码不可信时（上游 bundle 退出期堆破坏）以输出里的收尾标记为准：
    // 没有异常标记且见过 "End" → 该非零码来自退出期崩溃，不是求解失败。
    let teardown_crash = !exit_ok && !abort_marker && completion_marker && copy_back.is_ok();
    let now = now_ms();
    {
        let mut guard = inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if teardown_crash {
            if let Some(channel) = guard.channels.get(&job_id) {
                let _ = channel.send(
                    "求解已走完时间循环（输出以 End 收尾）；非零退出码来自已知的上游\
                     bundle 退出期崩溃，结果仍有效。"
                        .to_string(),
                );
            }
            let _ = job_logic::mark_done(&mut guard.jobs, &job_id, now);
        } else {
            match (exit_ok, copy_back) {
                (true, Ok(())) => {
                    let _ = job_logic::mark_done(&mut guard.jobs, &job_id, now);
                }
                (true, Err(e)) => {
                    let _ = job_logic::mark_failed(&mut guard.jobs, &job_id, e.message(), now);
                }
                (false, _) => {
                    let _ = job_logic::mark_failed(&mut guard.jobs, &job_id, "进程异常退出", now);
                }
            }
        }
        guard.children.remove(&job_id);
        guard.channels.remove(&job_id);
    }
    // 一个作业结束 → 立即尝试提升队列中的下一个（自动续跑）
    let scheduler = JobScheduler {
        inner,
        managed_path,
        vm_shell,
    };
    scheduler.promote_and_spawn(now);
}

/// 提交求解作业：入队并按预算立即尝试启动。progress 通道回传日志行与 __TIME__ 进度标记。
#[tauri::command]
pub fn submit_job(
    scheduler: State<'_, JobScheduler>,
    case_dir: String,
    cores: u32,
    study_id: Option<String>,
    progress: Channel<String>,
) -> Result<Job> {
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
