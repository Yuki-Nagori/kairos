//! 虚拟机命令：Multipass（macOS）/ WSL2（Windows）的探测、一键安装、启动、
//! 应用内 Shell 与停止；应用退出时联动关闭虚拟机（T35）。
//! 纯逻辑（参数构造 / 输出解析 / 提示文案）在 `kairos_core::services::vm`，
//! 本模块只做进程副作用与流式回传。

use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use kairos_core::error::{KairosError, Result};
use kairos_core::models::vm::{VmProviderKind, VmState, VmStatus};
use kairos_core::services::vm as vm_logic;
use tauri::State;
use tauri::ipc::Channel;

#[cfg(target_os = "windows")]
use std::io::Read;

/// 应用内 Shell 子进程句柄（同一时刻至多一个会话；新会话顶替旧会话）。
pub struct VmShellState(Arc<Mutex<Option<Child>>>);

impl Default for VmShellState {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

impl VmShellState {
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Child>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// 平台与 provider 一一对应；Linux 原生环境不需要虚拟机。
fn provider() -> Result<VmProviderKind> {
    vm_logic::provider_for(std::env::consts::OS)
        .ok_or_else(|| KairosError::validation("Linux 原生环境不需要虚拟机。"))
}

/// 构造受管命令。GUI 进程在 macOS 下只继承精简 PATH，必须补上
/// Homebrew（/opt/homebrew/bin）与官方 pkg（/usr/local/bin）的常见安装位置。
fn platform_command(bin: &str) -> Command {
    let mut command = Command::new(bin);
    #[cfg(target_os = "macos")]
    {
        let path = std::env::var("PATH").unwrap_or_default();
        if !path.starts_with("/opt/homebrew/bin:") {
            command.env("PATH", format!("/opt/homebrew/bin:/usr/local/bin:{path}"));
        }
    }
    command
}

fn run_bytes(args: &[String]) -> std::io::Result<std::process::Output> {
    platform_command(&args[0]).args(&args[1..]).output()
}

/// 解码一次探测输出的全部文本（WSL 的 UTF-16LE 在 core 统一归一）。
fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}\n{}",
        vm_logic::decode_wsl_output(&output.stdout),
        vm_logic::decode_wsl_output(&output.stderr)
    )
}

/// 从探测输出解析实例状态；命令失败视为实例不存在。
fn probe_instance_state(provider: VmProviderKind) -> VmState {
    let output = match run_bytes(&vm_logic::instance_info_args(provider)) {
        Ok(output) => output,
        Err(_) => return VmState::Missing,
    };
    let text = output_text(&output);
    match provider {
        // `multipass info <实例>` 只在实例不存在（或守护进程未起）时非 0 退出，
        // 此时 stderr 的报错文本不代表状态，一律按 Missing 走创建流程。
        VmProviderKind::Multipass => {
            if output.status.success() {
                vm_logic::parse_multipass_state(&text)
            } else {
                VmState::Missing
            }
        }
        VmProviderKind::Wsl => {
            // `wsl -l -v` 在没有发行版时退出码非 0，但有输出（Missing 由解析兜底）。
            if output.status.success() || text.contains("Ubuntu") {
                vm_logic::parse_wsl_list(&text)
            } else {
                VmState::Missing
            }
        }
    }
}

/// 把管道输出按平台策略回传：Unix 逐行实时流；Windows 读完后整体解码再按行
/// 发送（wsl 系列输出为 UTF-16LE，按字节流式会撕裂码元）。
fn forward_output<R: std::io::Read>(pipe: R, progress: &Channel<String>) {
    #[cfg(target_os = "windows")]
    {
        let mut pipe = pipe;
        let mut buffer = Vec::new();
        let _ = pipe.read_to_end(&mut buffer);
        for line in vm_logic::decode_wsl_output(&buffer).lines() {
            let cleaned = vm_logic::clean_terminal_line(line);
            if !cleaned.is_empty() {
                let _ = progress.send(cleaned);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::io::{BufRead, BufReader};
        for line in BufReader::new(pipe)
            .lines()
            .map_while(std::result::Result::ok)
        {
            let cleaned = vm_logic::clean_terminal_line(&line);
            if !cleaned.is_empty() {
                let _ = progress.send(cleaned);
            }
        }
    }
}

/// 运行一条受管命令：stdout 实时回传，stderr 在旁路线程并发回传（防管道填满死锁）。
fn run_and_stream(args: &[String], progress: &Channel<String>) -> Result<bool> {
    let mut child = platform_command(&args[0])
        .args(&args[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| KairosError::io(format!("启动命令失败（{args:?}）：{e}")))?;
    let stderr_thread = child.stderr.take().map(|stderr| {
        let progress = progress.clone();
        std::thread::spawn(move || forward_output(stderr, &progress))
    });
    if let Some(stdout) = child.stdout.take() {
        forward_output(stdout, progress);
    }
    if let Some(handle) = stderr_thread {
        let _ = handle.join();
    }
    Ok(child.wait().map(|status| status.success()).unwrap_or(false))
}

/// 探测虚拟机运行时状态（同步命令；进程调用总量小，主线程可承受）。
#[tauri::command]
pub fn vm_status() -> Result<VmStatus> {
    let provider = provider()?;
    let tool_installed = run_bytes(&vm_logic::version_args(provider))
        .map(|output| output.status.success())
        .unwrap_or(false);
    let instance_state = if tool_installed {
        probe_instance_state(provider)
    } else {
        VmState::Missing
    };
    Ok(vm_logic::build_status(
        provider,
        tool_installed,
        instance_state,
    ))
}

/// 一键安装运行时：macOS 走 Homebrew cask（日志实时回传）；
/// Windows 走 UAC 提权（安装窗口在系统层，无法回传日志）。
#[tauri::command]
pub async fn vm_install(progress: Channel<String>) -> Result<String> {
    let provider = provider()?;
    let args = vm_logic::install_args(provider);
    tauri::async_runtime::spawn_blocking(move || {
        let _ = progress.send("── 开始安装（可能需要管理员授权 / 数分钟）──".into());
        if run_and_stream(&args, &progress)? {
            Ok("安装完成。请点击「重新探测」确认。".into())
        } else {
            Err(KairosError::io("安装命令失败，详见上方日志。"))
        }
    })
    .await
    .map_err(|e| KairosError::internal(format!("安装任务失败：{e}")))?
}

/// 确保受管实例就绪：缺则创建（multipass launch / wsl --install），停则启动。
#[tauri::command]
pub async fn vm_start(progress: Channel<String>) -> Result<String> {
    let provider = provider()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _ = progress.send("── 检查受管实例状态 ──".into());
        match probe_instance_state(provider) {
            VmState::Running => Ok("虚拟机已在运行。".into()),
            VmState::Missing => {
                let _ = progress.send("── 实例不存在，开始创建（首次需下载镜像）──".into());
                if run_and_stream(&vm_logic::launch_args(provider), &progress)? {
                    Ok("虚拟机已创建并就绪。".into())
                } else {
                    Err(KairosError::io("实例创建失败，详见上方日志。"))
                }
            }
            _ => match vm_logic::start_args(provider) {
                Some(start) => {
                    let _ = progress.send("── 启动已存在的实例 ──".into());
                    if run_and_stream(&start, &progress)? {
                        Ok("虚拟机已启动。".into())
                    } else {
                        // 探测与实际状态存在竞态（或状态解析异常）：start 失败时
                        // 不直接报错，自动回落到创建流程自愈。
                        let _ = progress.send("── 实例启动失败，改用创建流程 ──".into());
                        if run_and_stream(&vm_logic::launch_args(provider), &progress)? {
                            Ok("虚拟机已创建并就绪。".into())
                        } else {
                            Err(KairosError::io("实例创建失败，详见上方日志。"))
                        }
                    }
                }
                // WSL 实例随首次执行自启，无独立 start 步骤。
                None => Ok("发行版已安装，进入 Shell 时自动启动。".into()),
            },
        }
    })
    .await
    .map_err(|e| KairosError::internal(format!("启动任务失败：{e}")))?
}

/// 开启应用内 Shell：杀掉旧会话，新子进程双向管道，输出经 Channel 流式回传。
#[tauri::command]
pub fn vm_shell_start(state: State<'_, VmShellState>, log: Channel<String>) -> Result<()> {
    let provider = provider()?;
    let mut guard = state.lock();
    if let Some(mut old) = guard.take() {
        let _ = old.kill();
        let _ = old.wait();
    }
    let args = vm_logic::shell_args(provider);
    let mut child = platform_command(&args[0])
        .args(&args[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| KairosError::io(format!("Shell 启动失败：{e}")))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let _ = log.send("── Shell 会话已建立（输入 exit 或点击「停止 Shell」结束）──".into());
    if let Some(stdout) = stdout {
        let progress = log.clone();
        std::thread::spawn(move || forward_output(stdout, &progress));
    }
    if let Some(stderr) = stderr {
        std::thread::spawn(move || forward_output(stderr, &log));
    }
    *guard = Some(child);
    Ok(())
}

/// 向 Shell 会话发送一行命令（换行由本函数补齐）。
#[tauri::command]
pub fn vm_shell_send(state: State<'_, VmShellState>, line: String) -> Result<()> {
    let mut guard = state.lock();
    let child = guard
        .as_mut()
        .ok_or_else(|| KairosError::validation("Shell 会话未开启。"))?;
    let stdin = child
        .stdin
        .as_mut()
        .ok_or_else(|| KairosError::internal("Shell 标准输入不可用。"))?;
    stdin
        .write_all(line.as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .and_then(|_| stdin.flush())
        .map_err(|e| KairosError::io(format!("命令发送失败（会话可能已结束）：{e}")))
}

/// 结束 Shell 会话（杀死子进程并回收）。
#[tauri::command]
pub fn vm_shell_stop(state: State<'_, VmShellState>) -> Result<()> {
    let mut guard = state.lock();
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

fn kill_session(state: &VmShellState) {
    let mut guard = state.lock();
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// 停止受管虚拟机实例（先结束 Shell 会话）。
#[tauri::command]
pub fn vm_stop(shell: State<'_, VmShellState>) -> Result<String> {
    kill_session(&shell);
    let provider = provider()?;
    let args = vm_logic::stop_args(provider);
    if run_bytes(&args)
        .map(|output| output.status.success())
        .unwrap_or(false)
    {
        Ok("虚拟机已停止。".into())
    } else {
        Err(KairosError::io("停止命令失败，请检查虚拟机状态。"))
    }
}

/// 应用退出联动（RunEvent::Exit）：杀 Shell 子进程，并以 detached 方式派发
/// 停止命令——故意不 wait，进程在应用退出后由系统回收。
pub fn cleanup_on_exit(shell: &VmShellState) {
    kill_session(shell);
    if let Ok(provider) = provider() {
        let args = vm_logic::stop_args(provider);
        let _ = platform_command(&args[0])
            .args(&args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}
