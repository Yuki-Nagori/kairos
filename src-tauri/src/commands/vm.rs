//! 虚拟机命令：Multipass（macOS）/ WSL2（Windows）/ 原生 bash（Linux）的
//! 探测、一键安装、启动、应用内 Shell 与停止；应用退出时联动关闭虚拟机（T35）。
//! 纯逻辑（参数构造 / 输出解析 / 提示文案）在 `kairos_core::services::vm`，
//! 本模块只做进程副作用与流式回传。

use std::io::{Read, Write};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use kairos_core::error::{KairosError, Result};
use kairos_core::models::vm::{VmProviderKind, VmState, VmStatus};
use kairos_core::services::vm as vm_logic;
use std::path::{Path, PathBuf};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

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

/// 平台与 provider 一一对应。
fn provider() -> Result<VmProviderKind> {
    vm_logic::provider_for(std::env::consts::OS)
        .ok_or_else(|| KairosError::validation("不支持的平台。"))
}

/// 构造受管命令（macOS）：GUI 进程只继承精简 PATH，必须补上
/// Homebrew（/opt/homebrew/bin）与官方 pkg（/usr/local/bin）的常见安装位置。
#[cfg(target_os = "macos")]
fn platform_command(bin: &str) -> Command {
    let mut command = Command::new(bin);
    let path = std::env::var("PATH").unwrap_or_default();
    if !path.starts_with("/opt/homebrew/bin:") {
        command.env("PATH", format!("/opt/homebrew/bin:/usr/local/bin:{path}"));
    }
    command
}

/// 构造受管命令（Windows / Linux）：无需 PATH 修补。
#[cfg(not(target_os = "macos"))]
fn platform_command(bin: &str) -> Command {
    Command::new(bin)
}

/// 带超时的一次性探测：multipass / wsl 首次调用可能要按需拉起守护进程，
/// 慢起来没有上限——超时按失败处理，绝不挂死调用线程。
fn run_bytes(args: &[String]) -> Result<Option<Output>> {
    let mut child = platform_command(&args[0])
        .args(&args[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| KairosError::io(format!("命令启动失败（{args:?}）：{e}")))?;
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match child
            .try_wait()
            .map_err(|e| KairosError::io(format!("命令等待失败：{e}")))?
        {
            Some(status) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let _ = pipe.read_to_end(&mut stdout);
                }
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_end(&mut stderr);
                }
                return Ok(Some(Output {
                    status,
                    stdout,
                    stderr,
                }));
            }
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(None);
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

/// 解码一次探测输出的全部文本（WSL 的 UTF-16LE 在 core 统一归一）。
fn output_text(output: &Output) -> String {
    format!(
        "{}\n{}",
        vm_logic::decode_wsl_output(&output.stdout),
        vm_logic::decode_wsl_output(&output.stderr)
    )
}

/// 探测宿主机资源并推导虚拟机默认规格。
/// CPU 用标准库 available_parallelism；内存按平台取总量，检测失败回退
/// 保守默认 16 GiB。全部调用点都在 spawn_blocking 内，不在主线程执行。
fn detect_host_resources() -> vm_logic::VmResources {
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4);
    let memory_gib = detect_memory_gib().unwrap_or(16);
    vm_logic::plan_resources(cpus, memory_gib)
}

/// 宿主物理内存总量（GiB，向下取整）。
#[cfg(target_os = "macos")]
fn detect_memory_gib() -> Option<u32> {
    // 绝对路径：GUI 进程的精简 PATH 不含 /usr/sbin。
    let output = Command::new("/usr/sbin/sysctl")
        .arg("-n")
        .arg("hw.memsize")
        .output()
        .ok()?;
    let bytes: u64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .ok()?;
    Some((bytes / (1024 * 1024 * 1024)) as u32)
}

#[cfg(target_os = "linux")]
fn detect_memory_gib() -> Option<u32> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|line| line.starts_with("MemTotal:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some((kb / (1024 * 1024)) as u32)
}

#[cfg(target_os = "windows")]
fn detect_memory_gib() -> Option<u32> {
    // wmic 已从新 Windows 移除，走 PowerShell CIM（秒级，调用点在 spawn_blocking）。
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
        ])
        .output()
        .ok()?;
    let bytes: u64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .ok()?;
    Some((bytes / (1024 * 1024 * 1024)) as u32)
}

/// 从探测输出解析实例状态；命令失败 / 超时视为实例不存在。
fn probe_instance_state(provider: VmProviderKind) -> Result<VmState> {
    let Some(output) = run_bytes(&vm_logic::instance_info_args(provider))? else {
        return Ok(VmState::Missing); // 超时视为不存在，交给创建流程兜底
    };
    let text = output_text(&output);
    match provider {
        // `multipass info <实例>` 只在实例不存在（或守护进程未起）时非 0 退出，
        // 此时 stderr 的报错文本不代表状态，一律按 Missing 走创建流程。
        VmProviderKind::Multipass => {
            if output.status.success() {
                Ok(vm_logic::parse_multipass_state(&text))
            } else {
                Ok(VmState::Missing)
            }
        }
        VmProviderKind::Wsl => {
            // `wsl -l -v` 在没有发行版时退出码非 0，但有输出（Missing 由解析兜底）。
            if output.status.success() || text.contains("Ubuntu") {
                Ok(vm_logic::parse_wsl_list(&text))
            } else {
                Ok(VmState::Missing)
            }
        }
        // 原生环境恒就绪。
        VmProviderKind::Native => Ok(VmState::Running),
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

/// 宿主 IANA 时区名（如 Asia/Shanghai）：unix 读 /etc/localtime 链接目标，
/// Windows 用 PowerShell Get-TimeZone，兜底 TZ 环境变量。纯本机判断，零网络请求。
fn host_timezone() -> String {
    #[cfg(not(target_os = "windows"))]
    {
        // /etc/localtime → .../zoneinfo/Asia/Shanghai：取 zoneinfo 之后的完整路径段。
        if let Ok(target) = std::fs::read_link("/etc/localtime") {
            let text = target.to_string_lossy();
            if let Some(pos) = text.find("zoneinfo/") {
                return text[pos + "zoneinfo/".len()..].to_string();
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("powershell")
            .args(["-NoProfile", "-Command", "(Get-TimeZone).Id"])
            .output()
        {
            let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !id.is_empty() {
                return id;
            }
        }
    }
    std::env::var("TZ").unwrap_or_default()
}

/// 国内时区：从清华镜像预下载 24.04 云镜像到受管目录（已存在则复用），
/// 之后 `multipass launch file://` 本地导入——multipassd 直连上游下载慢。
/// 失败返回 None 并回退官方直连（不阻塞创建流程）。
fn ensure_vm_image(app: &AppHandle, progress: &Channel<String>) -> Result<Option<PathBuf>> {
    if provider()? != VmProviderKind::Multipass {
        return Ok(None);
    }
    if !vm_logic::is_china_timezone(&host_timezone()) {
        return Ok(None);
    }
    let Some(file_name) = vm_logic::image_file_name(std::env::consts::ARCH) else {
        return Ok(None);
    };
    let dest = super::downloads::downloads_dir(app)?
        .join("images")
        .join(file_name);
    if dest.exists() {
        let _ = progress.send("── 复用已下载的云镜像 ──".into());
        return Ok(Some(dest));
    }
    let part = dest.with_extension("img.part");
    for url in &vm_logic::image_mirror_urls(std::env::consts::ARCH) {
        let _ = progress.send(format!("── 从镜像源下载云镜像：{url} ──"));
        match download_to_file(url, &part, progress) {
            Ok(()) => {
                std::fs::rename(&part, &dest)?;
                return Ok(Some(dest));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&part);
                let _ = progress.send(format!("镜像源不可用：{e}"));
            }
        }
    }
    let _ = progress.send("── 镜像源均不可用，改用官方源直连 ──".into());
    Ok(None)
}

/// 从单个 URL 流式下载到目标文件，按去重后的百分比回传进度。
fn download_to_file(url: &str, dest: &Path, progress: &Channel<String>) -> Result<()> {
    // images/ 子目录尚不存在时会 os error 2（真机踩过），先建父目录。
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // 大文件（约 600MB）：连接 30s、总量 1h 超时，UA 与依赖下载保持一致。
    let agent: ureq::Agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(3600))
        .user_agent("kairos-dependency-manager/1.0")
        .build();
    let response = agent
        .get(url)
        .call()
        .map_err(|e| KairosError::io(format!("下载请求失败：{e}")))?;
    let total: u64 = response
        .header("Content-Length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let mut file = std::fs::File::create(dest)?;
    let mut reader = response.into_reader();
    let mut buffer = [0u8; 65_536];
    let mut downloaded: u64 = 0;
    let mut last_percent: Option<u64> = None;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                file.write_all(&buffer[..n])?;
                downloaded += n as u64;
            }
            Err(e) => return Err(KairosError::io(format!("镜像下载中断：{e}"))),
        }
        // checked_div 兼容 total=0（内容长度未知）时不发进度，模式同 downloads.rs。
        match (downloaded * 100).checked_div(total) {
            Some(percent) if last_percent != Some(percent) => {
                let _ = progress.send(format!("镜像下载 {percent}%"));
                last_percent = Some(percent);
            }
            _ => {}
        }
    }
    file.flush()?;
    Ok(())
}

/// 部署求解环境：把受管的 moldingFoam bundle 传输进虚拟机并解压到
/// ~/moldingfoam-env（multipass 平台；作业执行依赖该环境树）。
#[tauri::command]
pub async fn vm_deploy_bundle(app: AppHandle, progress: Channel<String>) -> Result<String> {
    let provider = provider()?;
    if provider != VmProviderKind::Multipass {
        return Err(KairosError::validation(
            "当前平台作业直接在本机执行，无需部署 bundle。",
        ));
    }
    let entry = super::downloads::manifest_entry(&app, "moldingfoam")?
        .ok_or_else(|| KairosError::not_found("尚未下载求解环境 bundle，请先在依赖面板下载。"))?;
    let archive = super::downloads::downloads_dir(&app)?.join(&entry.file_name);
    if !archive.exists() {
        return Err(KairosError::not_found(
            "bundle 归档文件缺失，请重新下载求解环境。",
        ));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _ = progress.send("── 传输 bundle 进虚拟机（约 120MB）──".into());
        let transfer = platform_command("multipass")
            .args([
                "transfer",
                &archive.to_string_lossy(),
                "kairos:/home/ubuntu/moldingfoam-bundle.tar.xz",
            ])
            .status()
            .map_err(|e| KairosError::io(format!("传输启动失败：{e}")))?;
        if !transfer.success() {
            return Err(KairosError::io("bundle 传输失败，请确认虚拟机已启动。"));
        }
        let _ = progress.send("── 解压环境树 ──".into());
        let extract = platform_command("multipass")
            .args([
                "exec",
                "kairos",
                "--",
                "bash",
                "-lc",
                "mkdir -p ~/moldingfoam-env && tar -xJf ~/moldingfoam-bundle.tar.xz -C ~/moldingfoam-env && test -f ~/moldingfoam-env/openfoam14/etc/bashrc",
            ])
            .status()
            .map_err(|e| KairosError::io(format!("解压启动失败：{e}")))?;
        if !extract.success() {
            return Err(KairosError::io("解压失败，请确认 bundle 完整后重试。"));
        }
        // 求解器运行时依赖：foamRun 链接 libmpi.so.40（真机踩坑），VM 内必须
        // 有 OpenMPI。ubuntu 用户免密 sudo，非交互安装无阻碍。
        let _ = progress.send("── 安装 OpenMPI 运行时（约 1 分钟）──".into());
        // 国内时区：先切清华 apt 镜像源（与云镜像同源策略，加速 update）。
        if vm_logic::is_china_timezone(&host_timezone()) {
            let _ = progress.send("── 国内时区：VM 内 apt 源切换清华镜像 ──".into());
            let _ = platform_command("multipass")
                .args([
                    "exec",
                    "kairos",
                    "--",
                    "bash",
                    "-lc",
                    "sudo sed -i 's|http://archive.ubuntu.com/ubuntu|https://mirrors.tuna.tsinghua.edu.cn/ubuntu|g; s|http://security.ubuntu.com/ubuntu|https://mirrors.tuna.tsinghua.edu.cn/ubuntu|g' /etc/apt/sources.list /etc/apt/sources.list.d/*.sources 2>/dev/null || true",
                ])
                .status();
        }
        let apt = platform_command("multipass")
            .args([
                "exec",
                "kairos",
                "--",
                "bash",
                "-lc",
                "sudo apt-get update -qq && sudo apt-get install -y -qq libopenmpi-dev openmpi-bin && sudo ldconfig && ldconfig -p | grep -q libmpi.so.40",
            ])
            .status()
            .map_err(|e| KairosError::io(format!("apt 安装启动失败：{e}")))?;
        if !apt.success() {
            return Err(KairosError::io(
                "OpenMPI 安装失败：foamRun 缺 libmpi.so.40 将无法启动，请检查 VM 网络后重试。",
            ));
        }
        Ok("求解环境已部署（含 OpenMPI）。提交作业即可在虚拟机内执行。".into())
    })
    .await
    .map_err(|e| KairosError::internal(format!("部署任务失败：{e}")))?
}

/// 探测虚拟机运行时状态（异步：进程探测可能到秒级，绝不阻塞主线程）。
#[tauri::command]
pub async fn vm_status() -> Result<VmStatus> {
    let provider = provider()?;
    tauri::async_runtime::spawn_blocking(move || {
        let tool_installed = run_bytes(&vm_logic::version_args(provider))?
            .map(|output| output.status.success())
            .unwrap_or(false);
        let instance_state = if tool_installed {
            probe_instance_state(provider)?
        } else {
            VmState::Missing
        };
        Ok(vm_logic::build_status(
            provider,
            tool_installed,
            instance_state,
        ))
    })
    .await
    .map_err(|e| KairosError::internal(format!("探测任务失败：{e}")))?
}

/// 一键安装运行时：macOS 走 Homebrew cask（日志实时回传）；
/// Windows 走 UAC 提权（安装窗口在系统层，无法回传日志）。
#[tauri::command]
pub async fn vm_install(progress: Channel<String>) -> Result<String> {
    let provider = provider()?;
    if provider == VmProviderKind::Native {
        return Err(KairosError::validation("Linux 原生环境无需安装虚拟机。"));
    }
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
pub async fn vm_start(app: AppHandle, progress: Channel<String>) -> Result<String> {
    let provider = provider()?;
    if provider == VmProviderKind::Native {
        return Ok("Linux 原生环境无需虚拟机，可直接进入 Shell。".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _ = progress.send("── 检查受管实例状态 ──".into());
        let resources = detect_host_resources();
        let _ = progress.send(format!(
            "── 虚拟机规格（按宿主推导）：{} 核 / {}G 内存 / {}G 磁盘 ──",
            resources.cpus, resources.memory_gib, resources.disk_gib
        ));
        let state = probe_instance_state(provider)?;
        if state == VmState::Running {
            return Ok("虚拟机已在运行。".into());
        }
        // 需要创建：国内时区先从镜像源预下载云镜像（multipass launch file:// 导入）。
        let image = ensure_vm_image(&app, &progress)?;
        let image_str = image.as_deref().map(|p| p.to_string_lossy().into_owned());
        let launch = vm_logic::launch_args(provider, &resources, image_str.as_deref());
        match state {
            VmState::Missing => {
                let _ = progress.send("── 实例不存在，开始创建（首次需下载镜像）──".into());
                if run_and_stream(&launch, &progress)? {
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
                        if run_and_stream(&launch, &progress)? {
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

/// 开启应用内 Shell：杀掉旧会话，实例未运行则先拉起，新子进程双向管道，
/// 输出经 Channel 流式回传（异步：拉起实例到秒级，不阻塞主线程）。
#[tauri::command]
pub async fn vm_shell_start(state: State<'_, VmShellState>, log: Channel<String>) -> Result<()> {
    let provider = provider()?;
    let inner = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        shell_start_blocking(&provider, &mut guard, &log)
    })
    .await
    .map_err(|e| KairosError::internal(format!("Shell 任务失败：{e}")))?
}

fn shell_start_blocking(
    provider: &VmProviderKind,
    guard: &mut std::sync::MutexGuard<'_, Option<Child>>,
    log: &Channel<String>,
) -> Result<()> {
    if let Some(mut old) = guard.take() {
        let _ = old.kill();
        let _ = old.wait();
    }
    // 实例未运行时先拉起（multipass exec 对停止实例直接报错）。
    match probe_instance_state(*provider) {
        Err(e) => {
            let _ = log.send(format!("── 实例状态检查失败：{e} ──"));
        }
        Ok(VmState::Missing) => {
            let _ = log.send("── 虚拟机尚未创建，请先点击「启动虚拟机」──".into());
            return Ok(());
        }
        Ok(VmState::Stopped) | Ok(VmState::Unknown) => {
            if let Some(start) = vm_logic::start_args(*provider) {
                let _ = log.send("── 实例未运行，自动拉起（约 10–30 秒）──".into());
                if let Err(e) = run_and_stream(&start, log) {
                    let _ = log.send(format!("── 实例拉起失败：{e} ──"));
                    return Err(e);
                }
            }
        }
        _ => {}
    }
    let args = wrap_pty(vm_logic::shell_args(*provider));
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
        let log = log.clone();
        std::thread::spawn(move || forward_output(stderr, &log));
    }
    **guard = Some(child);
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

/// 给交互 Shell 包一层 PTY：multipass exec / bash 在非 TTY 管道下是批处理
/// 语义（stdin 读到 EOF 才执行，无法交互——真机踩过），script 提供伪终端后
/// multipass 检测到 TTY 即切完整交互模式（回显/提示符/输出全通）。
#[cfg(target_os = "macos")]
fn wrap_pty(args: Vec<String>) -> Vec<String> {
    let mut wrapped = vec!["script".into(), "-q".into(), "/dev/null".into()];
    wrapped.extend(args);
    wrapped
}

#[cfg(target_os = "linux")]
fn wrap_pty(args: Vec<String>) -> Vec<String> {
    vec![
        "script".into(),
        "-qec".into(),
        args.join(" "),
        "/dev/null".into(),
    ]
}

/// Windows：wsl.exe 自带 ConPTY 桥，管道模式保持交互转发。
#[cfg(target_os = "windows")]
fn wrap_pty(args: Vec<String>) -> Vec<String> {
    args
}

/// 停止受管虚拟机实例（先结束 Shell 会话；graceful stop 到秒级，走异步）。
#[tauri::command]
pub async fn vm_stop(shell: State<'_, VmShellState>) -> Result<String> {
    kill_session(&shell);
    let provider = provider()?;
    if provider == VmProviderKind::Native {
        return Ok("Linux 原生环境无需停止虚拟机。".into());
    }
    let args = vm_logic::stop_args(provider);
    tauri::async_runtime::spawn_blocking(move || {
        if run_bytes(&args)?
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            Ok("虚拟机已停止。".into())
        } else {
            Err(KairosError::io("停止命令失败，请检查虚拟机状态。"))
        }
    })
    .await
    .map_err(|e| KairosError::internal(format!("停止任务失败：{e}")))?
}

/// 应用退出联动（RunEvent::Exit）：杀 Shell 子进程，并以 detached 方式派发
/// 停止命令——故意不 wait，进程在应用退出后由系统回收。
pub fn cleanup_on_exit(shell: &VmShellState) {
    kill_session(shell);
    let Ok(provider) = provider() else {
        return;
    };
    // 原生环境无虚拟机可停。
    if provider == VmProviderKind::Native {
        return;
    }
    let args = vm_logic::stop_args(provider);
    let _ = platform_command(&args[0])
        .args(&args[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
