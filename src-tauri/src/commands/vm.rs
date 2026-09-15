//! 虚拟机命令：Multipass（macOS）/ WSL2（Windows）/ 原生 bash（Linux）的
//! 探测、一键安装、启动、应用内 Shell 与停止；应用退出时联动关闭虚拟机。
//! 纯逻辑（参数构造 / 输出解析 / 提示文案）在 `kairos_core::services::vm`，
//! 本模块只做进程副作用与流式回传。

use std::io::{Read, Write};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use kairos_core::error::{KairosError, Result};
use kairos_core::models::vm::{VmProviderKind, VmState, VmStatus};
use kairos_core::services::host;
use kairos_core::services::vm as vm_logic;
use kairos_core::services::vm_run;
use kairos_core::utils::fs::write_atomic;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

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

/// GUI 进程缺失的 PATH 前缀：应用从 Finder 启动时只继承精简 PATH，
/// multipass / brew 这类工具都在 Homebrew（`/opt/homebrew/bin`）或官方 pkg
/// （`/usr/local/bin`）的目录里。非 macOS 无此问题。
#[cfg(target_os = "macos")]
fn mac_path_prefix() -> Option<&'static str> {
    Some("/opt/homebrew/bin:/usr/local/bin")
}

#[cfg(not(target_os = "macos"))]
fn mac_path_prefix() -> Option<&'static str> {
    None
}

/// 构造受管命令：补上 GUI 进程缺失的 PATH（`prefixed_path` 不重复叠加已有前缀）。
pub(crate) fn platform_command(bin: &str) -> Command {
    let mut command = Command::new(bin);
    if let Some(prefix) = mac_path_prefix() {
        let path = std::env::var("PATH").unwrap_or_default();
        command.env("PATH", vm_run::prefixed_path(&path, prefix));
    }
    command
}

/// VM 通道的宿主命令 runner：宿主差异（PATH 前缀）在这里注入，
/// 起进程 / 抽干管道 / 超时 / 成败判定全部复用 core（services::vm_run）。
pub(crate) fn host_runner() -> vm_run::ProcessRunner {
    vm_run::ProcessRunner::new(mac_path_prefix(), vm_run::ProcessRunner::DEFAULT_TIMEOUT_S)
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

/// 宿主物理内存总量（GiB，向下取整）。取数与解析分开：解析在 core（三端都能测）。
#[cfg(target_os = "macos")]
fn detect_memory_gib() -> Option<u32> {
    // 绝对路径：GUI 进程的精简 PATH 不含 /usr/sbin。
    let output = Command::new("/usr/sbin/sysctl")
        .arg("-n")
        .arg("hw.memsize")
        .output()
        .ok()?;
    host::memory_gib_from_bytes(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "linux")]
fn detect_memory_gib() -> Option<u32> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    host::memory_gib_from_meminfo(&text)
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
    host::memory_gib_from_bytes(&String::from_utf8_lossy(&output.stdout))
}

/// 从探测输出解析实例状态；命令失败 / 超时视为实例不存在。
pub(crate) fn probe_instance_state(provider: VmProviderKind) -> Result<VmState> {
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
/// 发送（wsl 系列输出为 UTF-16LE，按字节流式会撕裂码元）。清洗与空行过滤共用一处。
fn forward_output<R: std::io::Read>(pipe: R, progress: &Channel<String>) {
    let emit = |line: &str| {
        let cleaned = vm_logic::clean_terminal_line(line);
        if !cleaned.is_empty() {
            let _ = progress.send(cleaned);
        }
    };
    #[cfg(target_os = "windows")]
    {
        let mut pipe = pipe;
        let mut buffer = Vec::new();
        let _ = pipe.read_to_end(&mut buffer);
        for line in vm_logic::decode_wsl_output(&buffer).lines() {
            emit(line);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::io::{BufRead, BufReader};
        for line in BufReader::new(pipe)
            .lines()
            .map_while(std::result::Result::ok)
        {
            emit(&line);
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

/// 由参数表构造宿主命令（`args[0]` = 可执行名，与 core 侧 `*args` 构造同口径），
/// 并补上 PATH——应用作为 GUI 进程只继承精简 PATH，multipass / brew 这类工具都住在
/// Homebrew 或官方 pkg 的目录里。命令层与作业层共用这一处。
pub(crate) fn host_command(args: &[String]) -> Command {
    let mut command = platform_command(&args[0]);
    command.args(&args[1..]);
    command
}

/// 跑一条受管命令并等它结束（stdout/stderr 交给父进程，不进 UI）：非零退出即
/// `{what}失败：{hint}`。`hint` 是给用户的处置建议，不是命令本身的输出。
fn run_quiet(args: &[String], what: &str, hint: &str) -> Result<()> {
    match host_command(args).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(KairosError::io(format!("{what}失败：{hint}"))),
        Err(e) => Err(KairosError::io(format!("{what}启动失败：{e}"))),
    }
}

/// 跑一条需要实时日志的命令（安装 / 启动这类分钟级步骤）：成功返回 `ok` 文案，
/// 非零退出按 `fail` 文案报错。
fn stream_step(
    args: &[String],
    progress: &Channel<String>,
    ok: &str,
    fail: &str,
) -> Result<String> {
    if run_and_stream(args, progress)? {
        Ok(ok.to_string())
    } else {
        Err(KairosError::io(fail.to_string()))
    }
}

/// 宿主 IANA 时区名（如 Asia/Shanghai）：unix 读 /etc/localtime 链接目标，
/// Windows 用 PowerShell Get-TimeZone，兜底 TZ 环境变量。纯本机判断，零网络请求。
fn host_timezone() -> String {
    #[cfg(not(target_os = "windows"))]
    {
        // /etc/localtime → .../zoneinfo/Asia/Shanghai：取 zoneinfo 之后的完整路径段。
        if let Ok(target) = std::fs::read_link("/etc/localtime") {
            let text = target.to_string_lossy();
            if let Some(zone) = host::zoneinfo_name(&text) {
                return zone;
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
    // images/ 子目录尚不存在时会 os error 2，先建父目录。
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

/// 本机部署记录文件名：VM 内版本标记的本机副本（见 `write_deployed_record`）。
const DEPLOYED_RECORD_FILE: &str = "vm-deployed-tag";

/// 本机部署记录的存放目录（应用数据目录，与下载清单同层）。
fn deployed_record_dir(app: &AppHandle) -> Result<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| KairosError::io(format!("无法定位应用数据目录：{e}")))
}

/// 读一个「内容就是版本标签」的文件（VM 标记的本机记录 / 原生环境的标记）：
/// 缺失、读失败、全空白都按「没有部署过」处理，归一在 core 一处。
fn read_tag_file(path: &Path) -> Option<String> {
    vm_logic::parse_env_tag(&std::fs::read_to_string(path).ok()?)
}

/// 读本机部署记录：文件缺失 / 空白 → None（未部署过）。
fn read_deployed_record(dir: &Path) -> Option<String> {
    read_tag_file(&dir.join(DEPLOYED_RECORD_FILE))
}

/// 写本机部署记录（原子替换：半截文件会被当成另一个版本标签）。
/// `tag = None` 表示 VM 内已确认没有环境树，记录一并清掉。
fn write_deployed_record(dir: &Path, tag: Option<&str>) -> Result<()> {
    let path = dir.join(DEPLOYED_RECORD_FILE);
    let Some(tag) = tag else {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    };
    std::fs::create_dir_all(dir).map_err(|e| KairosError::io(format!("创建数据目录失败：{e}")))?;
    write_atomic(&path, tag, "部署记录")?;
    Ok(())
}

/// 读取 VM 内已部署的求解环境版本标记（「更新未部署」提醒的比对源）。
///
/// VM 可达（Running）时以 VM 内标记为准并同步刷新本机记录；VM 停机 / 启动中 /
/// 状态未知时**不启动实例**，回落本机记录——multipass 的 exec 对停止实例会隐式
/// 拉起，为读一个标记把十几 GB 的虚拟机拉起来不值得，而「部署过」是持久事实，
/// 不该因为虚拟机停机（作业跑完与应用退出都会关）就退回「未部署」。
/// 非 multipass 平台无部署概念，恒 null。
#[tauri::command]
pub async fn vm_deployed_release_tag(app: AppHandle) -> Result<Option<String>> {
    // 原生（Linux）：环境就在本机，标记文件直接读盘。
    if provider()? == VmProviderKind::Native {
        let Some(root) = super::downloads::native_env_root(&app) else {
            return Ok(None);
        };
        return Ok(read_tag_file(&vm_logic::native_env_tag(&root)));
    }
    if provider()? != VmProviderKind::Multipass {
        return Ok(None);
    }
    let dir = deployed_record_dir(&app)?;
    // 实例状态探测不启动实例：停止 / 启动中 / 状态未知一律走本机记录。
    if !matches!(
        probe_instance_state(VmProviderKind::Multipass),
        Ok(VmState::Running)
    ) {
        return Ok(read_deployed_record(&dir));
    }
    let read_args = vm_logic::bash_script_args(
        VmProviderKind::Multipass,
        &vm_logic::vm_env_tag_read_command(),
    );
    tauri::async_runtime::spawn_blocking(move || {
        let output = host_command(&read_args)
            .output()
            .map_err(|e| KairosError::io(format!("读取版本标记失败：{e}")))?;
        // 探测到 Running 之后实例又停了（竞态）：按不可达处理，回落本机记录。
        if !output.status.success() {
            return Ok(read_deployed_record(&dir));
        }
        let tag = vm_logic::parse_env_tag_reply(&String::from_utf8_lossy(&output.stdout));
        // VM 可达时它是唯一事实来源：读到什么记什么（标记没了 → 记录一并清掉）。
        // 记录只是停机期间的存档，写失败不改变本次读数。
        let _ = write_deployed_record(&dir, tag.as_deref());
        Ok(tag)
    })
    .await
    .map_err(|e| KairosError::internal(format!("读取部署版本失败：{e}")))?
}

/// 原生（Linux）求解环境状态：环境根 / 是否就绪 / OpenMPI 运行时 / 处置提示。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeEnvStatus {
    /// 环境根目录（未解压时为 null）。
    pub env_root: Option<String>,
    /// bashrc 是否就位（就绪 = 可直接提交作业）。
    pub env_ready: bool,
    /// OpenMPI 运行时（`mpirun`）是否可用。
    pub mpi_ready: bool,
    /// moldingFoam 动态库布局是否无重复且 solver 入口链接正确。
    pub library_ready: bool,
    /// 面向用户的处置提示（空 = 无问题）。
    pub hints: Vec<String>,
}

/// 探测原生求解环境（Linux 面板用；其它平台也返回结果，由前端按 provider 决定是否展示）。
#[tauri::command]
pub fn native_env_status(app: AppHandle) -> Result<NativeEnvStatus> {
    let root = super::downloads::native_env_root(&app);
    let env_ready = root
        .as_ref()
        .map(|path| vm_logic::native_env_bashrc(path).exists())
        .unwrap_or(false);
    let mpi_ready = Command::new("bash")
        .arg("-lc")
        .arg("command -v mpirun >/dev/null 2>&1")
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    let library_ready = root
        .as_ref()
        .map(|path| {
            Command::new("bash")
                .args(["-lc", &vm_logic::native_solver_lib_guard_command(path)])
                .output()
                .map(|output| {
                    output.status.success()
                        && vm_logic::native_solver_lib_guard_ok(&String::from_utf8_lossy(
                            &output.stdout,
                        ))
                })
                .unwrap_or(false)
        })
        .unwrap_or(false);
    let mut hints = vm_logic::native_dependency_hints(mpi_ready, env_ready);
    if env_ready && !library_ready {
        hints.push("moldingFoam 动态库布局异常：请清理重复库，并确保 libmoldingFoamSolver.so 为同目录相对软链接。".to_string());
    }
    Ok(NativeEnvStatus {
        env_root: root.map(|path| path.to_string_lossy().to_string()),
        env_ready,
        mpi_ready,
        library_ready,
        hints,
    })
}

/// 原生（Linux）「部署」：bundle 解压在本机即已就位，这里只做结构校验
/// 与版本标记落盘（供「更新未部署」提醒比对），不复制大文件。
#[tauri::command]
pub fn native_deploy_bundle(app: AppHandle) -> Result<String> {
    let root = super::downloads::native_env_root(&app).ok_or_else(|| {
        KairosError::not_found(
            "尚未下载求解环境 bundle：请在依赖面板下载（下载后本机自动解压即就位）。",
        )
    })?;
    let bashrc = vm_logic::native_env_bashrc(&root);
    if !bashrc.exists() {
        return Err(KairosError::validation(format!(
            "求解环境结构异常：缺少 {}，请重新下载 bundle。",
            bashrc.to_string_lossy()
        )));
    }
    let entry = super::downloads::manifest_entry(&app, "moldingfoam")?;
    if let Some(tag) = entry.and_then(|entry| entry.release_tag) {
        let marker = vm_logic::native_env_tag(&root);
        std::fs::write(&marker, &tag)
            .map_err(|e| KairosError::io(format!("写入版本标记失败：{e}")))?;
    }
    Ok(root.to_string_lossy().to_string())
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
        run_quiet(
            &vm_logic::transfer_args(
                &archive.to_string_lossy(),
                &vm_logic::vm_bundle_transfer_target(),
            ),
            "bundle 传输",
            "请确认虚拟机已启动",
        )?;
        let _ = progress.send("── 解压环境树 ──".into());
        run_quiet(
            &vm_logic::bash_script_args(VmProviderKind::Multipass, &vm_logic::env_deploy_command()),
            "解压环境树",
            "请确认 bundle 完整后重试",
        )?;
        // 版本标记：把 releaseTag 写进 VM，供「更新未部署」提醒比对。
        // 非 release 流组件无标签，跳过标记（比对端视为未部署）。
        if let Some(tag) = &entry.release_tag {
            run_quiet(
                &vm_logic::bash_script_args(
                    VmProviderKind::Multipass,
                    &vm_logic::vm_env_tag_write_command(tag),
                ),
                "写入版本标记",
                "请重试部署",
            )?;
            // 本机记录同步落盘：虚拟机停机期间靠它记住「已部署」（见读取策略）。
            // 记录写不进去不影响本次部署结果，但要在日志里留痕。
            if let Err(error) = write_deployed_record(&deployed_record_dir(&app)?, Some(tag)) {
                let _ = progress.send(format!(
                    "── 本机部署记录未写入（不影响本次部署）：{}",
                    error.message()
                ));
            }
        }
        // 求解器运行时依赖：foamRun 链接 libmpi.so.40，VM 内必须
        // 有 OpenMPI。ubuntu 用户免密 sudo，非交互安装无阻碍。
        let _ = progress.send("── 安装 OpenMPI 运行时（约 1 分钟）──".into());
        // 国内时区：先切清华 apt 镜像源（与云镜像同源策略，加速 update）。
        if vm_logic::is_china_timezone(&host_timezone()) {
            let _ = progress.send("── 国内时区：VM 内 apt 源切换清华镜像 ──".into());
            // best-effort：源切换只影响下载速度，失败继续走官方源。
            let _ = run_quiet(
                &vm_logic::bash_script_args(
                    VmProviderKind::Multipass,
                    &vm_logic::apt_mirror_command(),
                ),
                "apt 源切换",
                "继续使用官方源",
            );
        }
        run_quiet(
            &vm_logic::bash_script_args(
                VmProviderKind::Multipass,
                &vm_logic::openmpi_install_command(),
            ),
            "OpenMPI 安装",
            "请检查虚拟机网络后重试（缺 libmpi.so.40 时 foamRun 无法启动）",
        )?;
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
    let Some(args) = vm_logic::install_args(provider()?) else {
        return Err(KairosError::validation("Linux 原生环境无需安装虚拟机。"));
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _ = progress.send("── 开始安装（可能需要管理员授权 / 数分钟）──".into());
        stream_step(
            &args,
            &progress,
            "安装完成。请点击「重新探测」确认。",
            "安装命令失败，详见上方日志。",
        )
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
                stream_step(
                    &launch,
                    &progress,
                    "虚拟机已创建并就绪。",
                    "实例创建失败，详见上方日志。",
                )
            }
            _ => match vm_logic::start_args(provider) {
                Some(start) => {
                    let _ = progress.send("── 启动已存在的实例 ──".into());
                    let started = stream_step(
                        &start,
                        &progress,
                        "虚拟机已启动。",
                        "实例启动失败，详见上方日志。",
                    );
                    if let Err(error) = started {
                        // 探测与实际状态存在竞态（或状态解析异常）：start 失败时
                        // 不直接报错，自动回落到创建流程自愈。
                        let _ =
                            progress.send(format!("── 实例启动失败（{error}），改用创建流程 ──"));
                        stream_step(
                            &launch,
                            &progress,
                            "虚拟机已创建并就绪。",
                            "实例创建失败，详见上方日志。",
                        )
                    } else {
                        started
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
        let started = shell_start_blocking(&provider, &mut guard, &log);
        if started.is_ok() {
            super::vm_lease::set_shell_open(true);
        }
        started
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
    kill_session(&state);
    Ok(())
}

fn kill_session(state: &VmShellState) {
    let mut guard = state.lock();
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    super::vm_lease::set_shell_open(false);
}

/// 给交互 Shell 包一层 PTY：multipass exec / bash 在非 TTY 管道下是批处理
/// 语义（stdin 读到 EOF 才执行，无法交互），script 提供伪终端后
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
    super::vm_lease::clear_auto_started();
    let provider = provider()?;
    if provider == VmProviderKind::Native {
        return Ok("Linux 原生环境无需停止虚拟机。".into());
    }
    let args = vm_logic::stop_args(provider);
    tauri::async_runtime::spawn_blocking(move || {
        run_quiet(&args, "停止虚拟机", "请检查虚拟机状态")?;
        Ok("虚拟机已停止。".into())
    })
    .await
    .map_err(|e| KairosError::internal(format!("停止任务失败：{e}")))?
}

/// 应用退出联动（RunEvent::Exit）：杀 Shell 子进程，并以 detached 方式派发
/// 停止命令——故意不 wait，进程在应用退出后由系统回收。
pub fn cleanup_on_exit(shell: &VmShellState) {
    kill_session(shell);
    super::vm_lease::clear_auto_started();
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

#[cfg(test)]
mod tests {
    use super::{read_deployed_record, write_deployed_record};
    use std::path::PathBuf;

    /// 用例的临时目录根：进程号隔离不同 `cargo test` 运行，各用例只用它自己的子目录。
    fn scratch_dir() -> PathBuf {
        std::env::temp_dir().join(format!("kairos-deployed-tag-{}", std::process::id()))
    }

    /// 本机部署记录：未写 → 未部署；写入后读回同一标签；清空（None）后回到未部署。
    #[test]
    fn deployed_record_round_trips_and_clears() {
        // 每个用例独占一个子目录：并行跑时清理父目录会把别的用例的现场删掉
        let dir = scratch_dir().join("round-trip");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(read_deployed_record(&dir), None, "没有记录 = 未部署");
        write_deployed_record(&dir, Some("v1.0.0")).expect("写记录");
        assert_eq!(read_deployed_record(&dir), Some("v1.0.0".to_string()));
        // 覆盖写：换版本时记录跟着走
        write_deployed_record(&dir, Some("v1.1.0")).expect("覆盖写记录");
        assert_eq!(read_deployed_record(&dir), Some("v1.1.0".to_string()));
        // None = VM 内已确认没有环境 → 记录清掉，不能留旧标签充数
        write_deployed_record(&dir, None).expect("清记录");
        assert_eq!(read_deployed_record(&dir), None);
        // 清空后再清一次：幂等（文件不存在不是错误）
        write_deployed_record(&dir, None).expect("重复清记录");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 记录文件内容两端空白不算版本标签（写坏的文件不能变成「某个版本」）。
    #[test]
    fn deployed_record_treats_blank_file_as_missing() {
        let dir = scratch_dir().join("blank");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建目录");
        std::fs::write(dir.join("vm-deployed-tag"), "  \n").expect("写空白记录");
        assert_eq!(read_deployed_record(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 原子替换的临时文件与正式文件同级、写完即被 rename 掉（不留半截文件）。
    #[test]
    fn deployed_record_writes_beside_the_target() {
        let dir = scratch_dir().join("shape");
        let _ = std::fs::remove_dir_all(&dir);
        write_deployed_record(&dir, Some("v9.9.9")).expect("写记录");
        assert!(dir.join("vm-deployed-tag").exists());
        assert!(
            !dir.join("vm-deployed-tag.tmp").exists(),
            "临时文件必须已被 rename 掉"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
