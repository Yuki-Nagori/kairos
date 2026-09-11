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
use kairos_core::models::jobs::{Job, JobStatus};
use kairos_core::services::jobs as job_logic;
use kairos_core::services::jobs::SchedulerLimits;
use kairos_core::services::openfoam;
use kairos_core::services::project::new_id;
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

fn spawn_run_script(
    case_dir: &str,
    managed_path: Option<&str>,
    vm_shell: Option<&str>,
) -> Result<Child> {
    // VM 执行：case 目录经 tar 管道复制进 VM 原生文件系统
    // （multipass mount 的 sshfs 权限映射不可用），
    // 环境树由 vm_deploy_bundle 预先解压在 ~/moldingfoam-env。
    if let Some(shell) = vm_shell {
        // multipass mount 的 sshfs 权限映射导致子目录不可读，
        // 改为 tar 管道把 case 目录复制进 VM 原生文件系统（权限与性能可靠）。
        #[cfg(target_os = "macos")]
        if shell == "multipass" {
            let parent = Path::new(case_dir).parent().unwrap_or(Path::new("."));
            let name = Path::new(case_dir)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "case".into());
            let vm_case = format!("/home/ubuntu/{name}");
            let mut tar_cmd = Command::new("tar");
            tar_cmd
                .arg("-C")
                .arg(parent)
                .arg("-czf")
                .arg("-")
                .arg(&name);
            let mut mp = Command::new("multipass");
            mp.args([
                "exec",
                "kairos",
                "--",
                "bash",
                "-lc",
                &format!("mkdir -p '{vm_case}' && tar -xzf - -C '{vm_case}'"),
            ]);
            let tar_child = tar_cmd.stdout(Stdio::piped()).spawn();
            if let Ok(mut tchild) = tar_child {
                mp.stdin(tchild.stdout.take().expect("tar stdout"));
                let st = mp.status()?;
                let _ = tchild.wait();
                if !st.success() {
                    return Err(KairosError::io("case 目录复制进虚拟机失败。"));
                }
            }
        }
        #[cfg(target_os = "windows")]
        let case_dir = to_wsl_path(case_dir);
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let safe_dir = case_dir.replace('\'', "'\\''");
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let inner = format!(
            "source ~/moldingfoam-env/openfoam14/etc/bashrc && cd '{safe_dir}' && decomposePar -force && foamRun -parallel"
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
    let path_export = managed_path
        .map(|prefix| format!("export PATH='{prefix}:$PATH'; "))
        .unwrap_or_default();
    // 单引号内的 shell 转义：' → '\''（防路径注入）。
    let safe_dir = case_dir.replace('\'', "'\\''");
    // 求解入口：foamRun 是 OpenFOAM 11+ 的模块化运行器，具体求解模块由
    // case 的 controlDict（solver 键，见 openfoam.rs::SOLVER_MODULE）提供。
    let script =
        format!("{path_export}cd '{safe_dir}' && decomposePar -force && foamRun -parallel");
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

    /// 提升排队作业并为其生成运行线程。
    fn promote_and_spawn(&self, now: u64) {
        let started = {
            let mut inner = self.lock();
            let limits = inner.limits;
            job_logic::promote_ready(&mut inner.jobs, &limits, now)
        };
        for job_id in started {
            let (case_dir, managed_path, vm_shell) = {
                let inner = self.lock();
                let case_dir = inner
                    .jobs
                    .iter()
                    .find(|job| job.id == job_id)
                    .map(|job| job.case_dir.clone())
                    .unwrap_or_default();
                (case_dir, self.managed_path.clone(), self.vm_shell.clone())
            };
            match spawn_run_script(&case_dir, managed_path.as_deref(), vm_shell.as_deref()) {
                Ok(child) => self.run_job_thread(job_id, child),
                Err(e) => {
                    let mut inner = self.lock();
                    let _ = job_logic::mark_failed(&mut inner.jobs, &job_id, e.message(), now_ms());
                }
            }
        }
    }

    /// 单作业运行线程：流式回传日志与进度，收尾后写回状态并提升下一个排队作业。
    fn run_job_thread(&self, job_id: String, mut child: Child) {
        let inner = self.arc();
        let managed_path = self.managed_path.clone();
        let vm_shell = self.vm_shell.clone();
        let mut stdout = child.stdout.take();
        thread::spawn(move || {
            if let Some(pipe) = stdout.take() {
                let reader = BufReader::new(pipe);
                for line in reader.lines().map_while(std::result::Result::ok) {
                    let time_s = openfoam::parse_time_line(&line);
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
            let now = now_ms();
            {
                let mut guard = inner
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if exit_ok {
                    let _ = job_logic::mark_done(&mut guard.jobs, &job_id, now);
                } else {
                    let _ = job_logic::mark_failed(&mut guard.jobs, &job_id, "进程异常退出", now);
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
        });
    }
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
        let started = job_logic::promote_ready(&mut inner.jobs, &limits, now);
        let job = inner.jobs.iter().find(|job| job.id == id).cloned();
        (started, job)
    };
    let job = job.1.expect("刚提交的作业必然存在");
    scheduler.promote_and_spawn(now);
    let _ = started_marker(job.status);
    Ok(job)
}

// submit 里已尝试提升，这里的标记仅用于可读性。
fn started_marker(status: JobStatus) -> JobStatus {
    status
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
pub fn list_jobs(scheduler: State<'_, JobScheduler>) -> Vec<Job> {
    scheduler.lock().jobs.clone()
}
