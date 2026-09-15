//! VM 求解通道的分步编排：宿主打包 / 传输 / VM 内解压 / 结果回传 / 日志回读。
//!
//! 这里只产出**命令与步骤顺序**，执行交给调用方注入的 [`HostRunner`]——桌面端的作业线程
//! 与脚本化的集成测试因此跑同一段编排，而不是各写一份「先打包再传输再解压」。
//!
//! 命令成败的判定口径收敛在 [`judge`]：**只看退出码**，stderr 仅在失败时作为原因。
//! 这条口径是踩过坑的：宿主是 macOS 的 bsdtar、解压侧是 GNU tar，tar 会为 pax 扩展头
//! 关键字打警告而退出码仍为 0，把「stderr 非空」当失败会把成功的解压报成失败。

use std::path::Path;

use crate::error::{KairosError, Result};
use crate::models::vm::VmProviderKind;
use crate::services::results;
use crate::services::vm as vm_logic;

/// 一条宿主命令：可执行 + 参数 + 环境变量。编排只产出描述，执行在 [`HostRunner`]。
#[derive(Debug, Clone)]
pub struct HostCommand {
    /// 完整参数表，`args[0]` = 可执行名（与 `services::vm` 的 `*args` 同口径）。
    pub args: Vec<String>,
    /// 附加环境变量（如 `COPYFILE_DISABLE=1`）。
    pub env: Vec<(String, String)>,
}

impl HostCommand {
    /// 由参数表构造（无附加环境变量）。
    pub fn new(args: Vec<String>) -> Self {
        Self {
            args,
            env: Vec::new(),
        }
    }

    /// 追加环境变量（覆盖同名项）。
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.retain(|(name, _)| name != key);
        self.env.push((key.to_string(), value.to_string()));
        self
    }
}

/// 宿主命令的执行者：桌面端补 GUI 进程缺失的 PATH 并带超时保护，集成测试直接跑。
///
/// 实现方拿到退出码后**必须**用 [`judge`] 决定成败——判定口径只有一处，
/// 否则「stderr 有警告但退出码 0」这类语义会在某个调用点上悄悄跑偏。
///
/// 编排函数一律取 `&dyn HostRunner`：runner 按调用方各有一份实现（作业线程 / 集成测试），
/// 编排本身不该跟着泛型参数被复制成多份编译产物。
pub trait HostRunner {
    /// 跑一条命令，只关心成败（stdout / stderr 不消费）。
    fn run(&self, command: &HostCommand, what: &str) -> Result<()>;

    /// 跑一条命令并取回 stdout（成败判定同 [`HostRunner::run`]）。
    fn capture(&self, command: &HostCommand, what: &str) -> Result<String>;
}

/// 生产用的 runner：起进程 → 并行抽干 stdout / stderr → 等退出码（带超时）→ [`judge`]。
///
/// 它住在 core 而不是适配层，是因为**这段是接缝上最容易被忽略的一处**：判定口径、
/// 超时兜底、管道抽干各错一处都会表现为「命令明明成功了却被判失败」或「作业永远停在
/// 运行中」，而它此前是 macOS 专属代码，CI 的 Linux / Windows 两个平台根本跑不到。
/// 具体工具的定位（PATH 前缀等宿主差异）仍由调用方注入。
pub struct ProcessRunner {
    /// 补进 PATH 的前缀（GUI 进程只继承精简 PATH；终端里跑传 None）。
    pub path_prefix: Option<String>,
    /// 单条命令的等待上限：超时即杀进程并判失败，卡住的 CLI 不该拖住作业线程。
    pub timeout: std::time::Duration,
}

impl ProcessRunner {
    /// 默认超时（秒）：multipass 的 stdin 管道通道偶发不回退（远端命令早已结束、
    /// CLI 进程仍在自旋），没有上限时作业会永远停在「运行中」。
    pub const DEFAULT_TIMEOUT_S: u64 = 600;

    /// 按前缀与超时构造（`path_prefix = None` 表示用当前进程的 PATH 原样）。
    pub fn new(path_prefix: Option<&str>, timeout_s: u64) -> Self {
        Self {
            path_prefix: path_prefix.map(str::to_string),
            timeout: std::time::Duration::from_secs(timeout_s),
        }
    }

    fn execute(
        &self,
        command: &HostCommand,
        what: &str,
        capture_stdout: bool,
    ) -> Result<(Vec<u8>, String)> {
        let mut process = std::process::Command::new(&command.args[0]);
        process.args(&command.args[1..]);
        for (key, value) in &command.env {
            process.env(key, value);
        }
        if let Some(prefix) = &self.path_prefix {
            process.env("PATH", prefixed_path(&current_path(), prefix));
        }
        let mut child = process
            .stdin(std::process::Stdio::null())
            .stdout(if capture_stdout {
                std::process::Stdio::piped()
            } else {
                std::process::Stdio::null()
            })
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| KairosError::io(format!("{what}启动失败：{e}")))?;
        // stdout / stderr 都要并行抽干：管道写满会卡住子进程，而父进程正在等它退出。
        let stderr_drain = child.stderr.take().map(drain_bytes);
        let stdout_drain = child.stdout.take().map(drain_bytes);
        let outcome = wait_for_exit(&mut child, self.timeout);
        let stderr_text = stderr_drain
            .map(|handle| handle.join().unwrap_or_default())
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        let stdout = stdout_drain
            .map(|handle| handle.join().unwrap_or_default())
            .unwrap_or_default();
        match outcome {
            Ok(()) => Ok((stdout, stderr_text)),
            Err(fallback) => Err(failure(what, &stderr_text, &fallback)),
        }
    }
}

impl HostRunner for ProcessRunner {
    fn run(&self, command: &HostCommand, what: &str) -> Result<()> {
        self.execute(command, what, false).map(|_| ())
    }

    fn capture(&self, command: &HostCommand, what: &str) -> Result<String> {
        self.execute(command, what, true)
            .map(|(stdout, _)| String::from_utf8_lossy(&stdout).into_owned())
    }
}

/// 当前进程的 PATH（缺失时按空串处理：前缀仍然会被补上）。
fn current_path() -> String {
    std::env::var("PATH").unwrap_or_default()
}

/// 补 PATH 前缀：已经以该前缀开头时原样返回（避免同一目录被反复叠加）。
pub fn prefixed_path(path: &str, prefix: &str) -> String {
    if path.starts_with(prefix) {
        path.to_string()
    } else {
        format!("{prefix}:{path}")
    }
}

/// 起一个把管道读空的线程（返回原始字节，解码交给调用方）。
fn drain_bytes(mut pipe: impl std::io::Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = pipe.read_to_end(&mut buffer);
        buffer
    })
}

/// 轮询等待子进程结束（毫秒级间隔：命令本身是分钟级，轮询开销可忽略）。
const WAIT_POLL_MS: u64 = 50;

/// 等子进程结束：成功返回 Ok，失败给出兜底原因（stderr 为空时才用得上它）。
/// 返回的是 `String` 而不是 `KairosError`：调用方要先看 stderr，再决定用哪个当原因。
fn wait_for_exit(
    child: &mut std::process::Child,
    timeout: std::time::Duration,
) -> std::result::Result<(), String> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        // try_wait 报错（平台层罕见）等同「还不知道结果」：与「仍在跑」同处置，
        // 由超时兜底——不让一条永远走不到的错误臂单独占一行。
        let pending = match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("退出状态 {status}")),
            Ok(None) | Err(_) => true,
        };
        if pending && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(WAIT_POLL_MS));
            continue;
        }
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("超时（{} 秒无响应），已中止", timeout.as_secs()));
    }
}

/// 退出码 → 结果：**只看退出码**。stderr 上的警告（tar 的 pax 扩展头关键字、
/// multipass 的进度提示）不是失败信号——这条口径踩过坑，所以只在 core 写一处。
pub fn judge(exit_ok: bool, what: &str, stderr: &str, fallback: &str) -> Result<()> {
    if exit_ok {
        Ok(())
    } else {
        Err(failure(what, stderr, fallback))
    }
}

/// 失败侧的原因构造：stderr 首个非空行（multipass / tar 的失败说明都写在 stderr），
/// 没有内容时用调用方给的兜底描述（如「超时（600 秒无响应），已中止」）。
pub fn failure(what: &str, stderr: &str, fallback: &str) -> KairosError {
    let reason = stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(fallback);
    KairosError::io(format!("{what}失败：{reason}"))
}

/// case 打包命令：`tar -C <父目录> -czf <归档> <叶子名>`，归档条目自带叶子名前缀
/// （解压到 case 根即可还原原目录名）。
///
/// 两个开关去掉 macOS 私有元数据：`COPYFILE_DISABLE=1` 不打包 `._*` 影子文件、
/// `--no-xattrs` 不把扩展属性写成 pax 扩展头——macOS 给每个新文件挂
/// `com.apple.provenance`，照原样打包会让 guest 的 GNU tar 每个成员打一行警告，
/// case 目录里也多出一层 `._*`。
pub fn case_pack_command(archive: &Path, case_dir: &str) -> HostCommand {
    let parent = Path::new(case_dir)
        .parent()
        // 只有叶子名时 `parent()` 给的是空路径（不是 None）：`tar -C ''` 会失败。
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = Path::new(case_dir)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "case".into());
    HostCommand::new(vec![
        "tar".into(),
        "--no-xattrs".into(),
        "-C".into(),
        parent.to_string_lossy().to_string(),
        "-czf".into(),
        archive.to_string_lossy().to_string(),
        name,
    ])
    .with_env("COPYFILE_DISABLE", "1")
}

/// 宿主解压命令：`tar -xzf <归档> -C <目标目录>`（回传结果落到宿主 case 目录）。
pub fn host_unpack_command(archive: &Path, dest: &str) -> HostCommand {
    HostCommand::new(vec![
        "tar".into(),
        "-xzf".into(),
        archive.to_string_lossy().to_string(),
        "-C".into(),
        dest.to_string(),
    ])
}

/// case 进 VM 的三步：宿主打包 → `transfer` 进 VM → VM 内解压（解压后删归档）。
/// 返回 VM 内的 case 路径（求解进程必须在虚拟机里 cd 到它）。
///
/// 编排函数都取 `&dyn HostRunner`：泛型会按 runner 类型各单态化一份，同一源行登记两次，
/// 「提前返回」的那份计数为 0，覆盖率会把整行报成未覆盖（见 ARCHITECTURE §覆盖率
/// 「假未覆盖」的「多份单态实例」）。
pub fn stage_case(runner: &dyn HostRunner, archive: &Path, case_dir: &str) -> Result<String> {
    let vm_case = vm_logic::vm_case_dir(case_dir);
    runner.run(&case_pack_command(archive, case_dir), "case 打包")?;
    runner.run(
        &HostCommand::new(vm_logic::transfer_args(
            &archive.to_string_lossy(),
            &vm_logic::vm_transfer_endpoint(&vm_logic::vm_archive_path()),
        )),
        "case 传输进虚拟机",
    )?;
    runner.run(
        &HostCommand::new(vm_logic::bash_script_args(
            VmProviderKind::Multipass,
            &vm_logic::vm_case_extract_from_archive_command(&vm_case),
        )),
        "case 解压进虚拟机",
    )?;
    Ok(vm_case)
}

/// 结果回传四步：列 VM 内时间目录 → VM 内打包 → `transfer` 回宿主 → 宿主解压到 case 目录。
/// 返回回传的时间目录名（求解没产生结果时报错，不做「空回传」）。
pub fn copy_results(
    runner: &dyn HostRunner,
    archive: &Path,
    case_dir: &str,
    vm_case: &str,
) -> Result<Vec<String>> {
    let listing = runner.capture(
        &HostCommand::new(vm_logic::bash_script_args(
            VmProviderKind::Multipass,
            &vm_logic::vm_results_list_command(vm_case),
        )),
        "读取结果目录清单",
    )?;
    let names = results::time_dir_names(&listing);
    if names.is_empty() {
        return Err(KairosError::io(
            "虚拟机内没有可回传的结果时间目录（求解未产生输出）。",
        ));
    }
    runner.run(
        &HostCommand::new(vm_logic::bash_script_args(
            VmProviderKind::Multipass,
            &vm_logic::vm_results_pack_command(vm_case, &names),
        )),
        "求解结果打包",
    )?;
    runner.run(
        &HostCommand::new(vm_logic::transfer_args(
            &vm_logic::vm_transfer_endpoint(&vm_logic::vm_archive_path()),
            &archive.to_string_lossy(),
        )),
        "求解结果传输回宿主",
    )?;
    runner.run(
        &host_unpack_command(archive, case_dir),
        "求解结果解压到宿主",
    )?;
    Ok(names)
}

/// 一次日志回读的产物：求解是否已出现错误标记、新的读取偏移、退出码、本轮新增日志行。
#[derive(Debug)]
pub struct LogPoll {
    /// 日志里出现求解器错误标记（强特征行），调用方据此把作业判为失败。
    pub aborted: bool,
    /// 下一次回读应从哪个字节继续。
    pub offset: u64,
    /// 退出码文件的内容；`None` = 仍在求解。
    pub exit_code: Option<i32>,
    /// 本轮新增的日志行（已按求解器标记判定过）。
    pub lines: Vec<String>,
}

/// 回读一次求解日志（`tail -c +<offset+1>` + 退出码文件，见 `vm_logic::detached_read_command`）。
/// 轮询节奏与取消由调用方决定：这里只做「读一次、解析、判定」。
pub fn poll_log_once(runner: &dyn HostRunner, vm_case: &str, offset: u64) -> Result<LogPoll> {
    let stdout = runner.capture(
        &HostCommand::new(vm_logic::bash_script_args(
            VmProviderKind::Multipass,
            &vm_logic::detached_read_command(vm_case, offset),
        )),
        "求解日志回读",
    )?;
    let (chunk, new_offset, exit_code) = vm_logic::parse_read_output(&stdout, offset);
    let mut aborted = false;
    let mut lines = Vec::new();
    for line in chunk.lines() {
        aborted |= crate::services::moldingfoam::is_abort_line(line);
        lines.push(line.to_string());
    }
    Ok(LogPoll {
        aborted,
        offset: new_offset,
        exit_code,
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::path::PathBuf;

    /// 跨平台的最小「shell 命令」构造：测试要让真进程按脚本说话（写 stderr、给退出码）。
    #[cfg(unix)]
    fn shell(script: &str) -> Vec<String> {
        vec!["sh".into(), "-c".into(), script.into()]
    }

    #[cfg(windows)]
    fn shell(script: &str) -> Vec<String> {
        vec!["cmd".into(), "/C".into(), script.into()]
    }

    /// 退出码 0 但 stderr 有输出的命令——tar 打印警告就是这个形态，曾经被判成失败。
    #[cfg(unix)]
    const NOISY_OK: &str = "echo 'tar: Ignoring unknown extended header keyword' >&2; exit 0";
    #[cfg(windows)]
    const NOISY_OK: &str = "echo tar: Ignoring unknown extended header keyword 1>&2 & exit 0";

    /// 跑得比超时久的命令：用来验证「超时杀掉并判失败」这条兜底。
    #[cfg(unix)]
    const SLOW: &str = "sleep 5";
    #[cfg(windows)]
    const SLOW: &str = "ping -n 6 127.0.0.1 > NUL";
    #[cfg(unix)]
    const FAIL: &str = "echo 'transfer failed: instance is stopped' >&2; exit 3";
    #[cfg(windows)]
    const FAIL: &str = "echo transfer failed: instance is stopped 1>&2 & exit /b 3";
    #[cfg(unix)]
    const SILENT_FAIL: &str = "exit 7";
    #[cfg(windows)]
    const SILENT_FAIL: &str = "exit /b 7";

    /// 生产 runner 的判定：退出码 0 即成功（哪怕 stderr 有警告），非 0 报 stderr 首行，
    /// 超时杀掉并给出超时原因，启动失败单独报——四种形态各一条，跨平台跑。
    #[test]
    fn process_runner_judges_by_exit_code_and_timeout() {
        let runner = ProcessRunner::new(None, ProcessRunner::DEFAULT_TIMEOUT_S);
        runner
            .run(&HostCommand::new(shell(NOISY_OK)), "命令")
            .expect("退出码 0 的命令即使 stderr 有输出也算成功");

        let failed = runner
            .run(&HostCommand::new(shell(FAIL)), "命令")
            .expect_err("非零退出码必须判失败");
        let message = failed.message().to_string();
        assert!(
            message.contains("transfer failed: instance is stopped"),
            "失败原因取 stderr：{message}"
        );

        let silent = runner
            .run(&HostCommand::new(shell(SILENT_FAIL)), "命令")
            .expect_err("无 stderr 的非零退出码也要报失败");
        let message = silent.message().to_string();
        assert!(
            message.contains("退出状态"),
            "无 stderr 时用退出状态兜底：{message}"
        );

        let missing = runner
            .run(
                &HostCommand::new(vec!["kairos-no-such-binary".into()]),
                "命令",
            )
            .expect_err("不存在的可执行文件必须报启动失败");
        let message = missing.message().to_string();
        assert!(message.contains("启动失败"), "启动失败要单独报：{message}");

        // 超时：0 秒的等待窗口对 SLOW 一定不够
        let impatient = ProcessRunner::new(None, 0);
        let timeout = impatient
            .run(&HostCommand::new(shell(SLOW)), "命令")
            .expect_err("跑不完的命令必须超时判失败");
        let message = timeout.message().to_string();
        assert!(message.contains("超时"), "超时原因必须可读：{message}");
    }

    /// 补 PATH 前缀：子进程拿到的 PATH 以注入的前缀开头（GUI 进程靠它找到
    /// multipass / brew；终端里跑则传 None 走原样 PATH）。
    #[test]
    fn process_runner_prepends_path_prefix_for_the_child() {
        let prefix = "/opt/kairos-prefix";
        let runner = ProcessRunner::new(Some(prefix), ProcessRunner::DEFAULT_TIMEOUT_S);
        #[cfg(unix)]
        let script = "printf '%s' \"$PATH\"";
        #[cfg(windows)]
        let script = "echo %PATH%";
        let path = runner
            .capture(&HostCommand::new(shell(script)), "命令")
            .expect("回读 PATH");
        assert!(
            path.trim().starts_with(prefix),
            "子进程 PATH 应以注入前缀开头：{path}"
        );
        // 已经带前缀时不重复叠加（同一目录进两次会让 PATH 越来越长）
        let path_again = runner
            .capture(&HostCommand::new(shell(script)), "命令")
            .expect("回读 PATH");
        assert_eq!(path_again.matches(prefix).count(), 1, "{path_again}");
    }

    /// capture 取回 stdout 且不把 stderr 混进来；环境变量按 HostCommand 注入。
    #[test]
    fn process_runner_captures_stdout_and_injects_env() {
        let runner = ProcessRunner::new(None, ProcessRunner::DEFAULT_TIMEOUT_S);
        #[cfg(unix)]
        let script = "echo 'KAIROS-TAG:v1.0.0'; echo 'noise' >&2";
        #[cfg(windows)]
        let script = "echo KAIROS-TAG:v1.0.0 & echo noise 1>&2";
        let stdout = runner
            .capture(&HostCommand::new(shell(script)), "命令")
            .expect("回读 stdout");
        assert_eq!(stdout.trim(), "KAIROS-TAG:v1.0.0");

        #[cfg(unix)]
        let env_script = "printf '%s' \"$COPYFILE_DISABLE\"";
        #[cfg(windows)]
        let env_script = "echo %COPYFILE_DISABLE%";
        let value = runner
            .capture(
                &HostCommand::new(shell(env_script)).with_env("COPYFILE_DISABLE", "1"),
                "命令",
            )
            .expect("环境变量应传给子进程");
        assert_eq!(value.trim(), "1");
    }

    /// PATH 前缀：没带前缀时补在最前，已经带了就原样返回（不重复叠加）。
    #[test]
    fn prefixed_path_does_not_stack_the_same_prefix_twice() {
        assert_eq!(
            prefixed_path("/usr/bin:/bin", "/opt/homebrew/bin:/usr/local/bin"),
            "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin"
        );
        assert_eq!(
            prefixed_path("/opt/homebrew/bin:/usr/bin", "/opt/homebrew/bin"),
            "/opt/homebrew/bin:/usr/bin"
        );
    }

    /// 记账用假 runner：记下每条命令与 stdout 应答，不碰任何进程。
    #[derive(Default)]
    struct FakeRunner {
        commands: RefCell<Vec<(HostCommand, String)>>,
        replies: RefCell<Vec<String>>,
    }

    impl FakeRunner {
        fn with_replies(replies: &[&str]) -> Self {
            Self {
                commands: RefCell::new(Vec::new()),
                replies: RefCell::new(replies.iter().map(|r| r.to_string()).collect()),
            }
        }

        fn taken(&self) -> Vec<HostCommand> {
            self.commands
                .borrow()
                .iter()
                .map(|(command, _)| command.clone())
                .collect()
        }
    }

    impl HostRunner for FakeRunner {
        fn run(&self, command: &HostCommand, what: &str) -> Result<()> {
            self.commands
                .borrow_mut()
                .push((command.clone(), what.to_string()));
            Ok(())
        }

        fn capture(&self, command: &HostCommand, what: &str) -> Result<String> {
            self.commands
                .borrow_mut()
                .push((command.clone(), what.to_string()));
            let mut replies = self.replies.borrow_mut();
            // 应答按顺序取；取完即空输出（与真实命令「没有新内容」同形）。
            Ok(if replies.is_empty() {
                String::new()
            } else {
                replies.remove(0)
            })
        }
    }

    /// 判定口径：退出码为 0 就是成功——stderr 上有警告（tar 的 pax 扩展头关键字、
    /// multipass 的进度提示）不改判；非 0 才拿 stderr 首行非空内容当原因，
    /// 没有内容时用兜底描述（超时之类只有调用方知道的处置信息）。
    #[test]
    fn judge_decides_by_exit_code_and_quotes_stderr_only_on_failure() {
        judge(true, "case 解压进虚拟机", "", "不会用到").expect("退出码 0 即成功");
        judge(
            true,
            "case 解压进虚拟机",
            "tar: Ignoring unknown extended header keyword 'LIBARCHIVE.xattr.com.apple.provenance'\n",
            "不会用到",
        )
        .expect("stderr 有警告但退出码 0：不能判失败");

        let error = judge(
            false,
            "case 传输进虚拟机",
            "\ntransfer failed: instance is stopped\n更多细节\n",
            "不会用到",
        )
        .expect_err("非零退出码必须判失败");
        assert_eq!(
            error.message(),
            "case 传输进虚拟机失败：transfer failed: instance is stopped",
            "原因取 stderr 首个非空行"
        );
        let silent =
            judge(false, "case 打包", "   \n", "退出码 1").expect_err("无 stderr 也报失败");
        assert_eq!(silent.message(), "case 打包失败：退出码 1");
        assert_eq!(
            failure("求解日志回读", "", "超时（600 秒无响应），已中止").message(),
            "求解日志回读失败：超时（600 秒无响应），已中止"
        );
    }

    /// `with_env` 是 upsert：同名项只留最后写入的值，其余项保持追加顺序。
    #[test]
    fn with_env_replaces_same_key_in_place() {
        let command = HostCommand::new(vec!["multipass".into()])
            .with_env("COPYFILE_DISABLE", "1")
            .with_env("COPYFILE_DISABLE", "0")
            .with_env("TMPDIR", "/tmp");
        assert_eq!(
            command.env,
            [
                ("COPYFILE_DISABLE".to_string(), "0".to_string()),
                ("TMPDIR".to_string(), "/tmp".to_string()),
            ]
        );
    }

    /// 打包命令：归档条目自带叶子名前缀（解压目标可以是 case 根），
    /// 且带上去掉 macOS 私有元数据的两个开关。
    #[test]
    fn pack_command_keeps_leaf_and_drops_mac_metadata() {
        let command = case_pack_command(Path::new("/tmp/kairos-x.tgz"), "/data/cases/study-1");
        assert_eq!(
            command.args,
            [
                "tar",
                "--no-xattrs",
                "-C",
                "/data/cases",
                "-czf",
                "/tmp/kairos-x.tgz",
                "study-1"
            ]
        );
        assert_eq!(
            command.env,
            [("COPYFILE_DISABLE".to_string(), "1".to_string())]
        );
        // 相对路径（无父目录）时不 panic，父目录退化为 `.`
        assert_eq!(
            case_pack_command(Path::new("/tmp/a.tgz"), "case").args[3],
            "."
        );
        // 取不到叶子名（空路径）时条目名回退 `case`——与 `vm_case_dir` 的回退名同源，
        // 否则解压到 case 根后 `cd <叶子>` 找不到目录。
        let leafless = case_pack_command(Path::new("/tmp/a.tgz"), "");
        assert_eq!(
            leafless.args,
            [
                "tar",
                "--no-xattrs",
                "-C",
                ".",
                "-czf",
                "/tmp/a.tgz",
                "case"
            ]
        );
        assert!(
            vm_logic::vm_case_dir("").ends_with(&leafless.args[6]),
            "归档条目名与 VM 内 case 目录名必须同源：{leafless:?}"
        );
    }

    /// 进 VM 三步的顺序与目标：打包 → transfer（目标是 VM 内中转路径）→ VM 内解压；
    /// 解压脚本的归档名与中转路径同源（否则解压读到旧文件）。
    #[test]
    fn stage_case_runs_pack_transfer_extract_in_order() {
        let runner = FakeRunner::default();
        let archive = PathBuf::from("/tmp/kairos-transfer.tgz");
        let vm_case = stage_case(&runner, &archive, "/data/cases/study-7").expect("三步都成功");
        assert_eq!(vm_case, "/home/ubuntu/study-7");
        let commands = runner.taken();
        assert_eq!(commands.len(), 3, "进 VM 的步骤数不对：{commands:?}");
        assert_eq!(commands[0].args[0], "tar");
        assert_eq!(commands[1].args[0..2], ["multipass", "transfer"]);
        assert_eq!(
            commands[1].args[3],
            vm_logic::vm_transfer_endpoint(&vm_logic::vm_archive_path())
        );
        assert_eq!(
            commands[2].args[0..5],
            ["multipass", "exec", "kairos", "--", "bash"]
        );
        assert!(
            commands[2].args[6].contains(&vm_logic::vm_archive_path()),
            "解压的来源必须与传输目标同一个中转路径"
        );
    }

    /// 结果回传四步：清单 → VM 内打包（只打清单里的目录）→ transfer 回宿主 → 宿主解压。
    #[test]
    fn copy_results_lists_packs_transfers_and_unpacks() {
        let runner = FakeRunner::with_replies(&["0\n0.5\n1\n"]);
        let archive = PathBuf::from("/tmp/kairos-back.tgz");
        let names = copy_results(
            &runner,
            &archive,
            "/data/cases/study-7",
            "/home/ubuntu/study-7",
        )
        .expect("回传成功");
        assert_eq!(names, ["0", "0.5", "1"]);
        let commands = runner.taken();
        assert_eq!(commands.len(), 4);
        assert!(commands[0].args[6].contains("ls -d [0-9]*"), "第一步是清单");
        let pack_script = commands[1].args[6].clone();
        assert!(
            pack_script.contains("tar -czf") && pack_script.ends_with("0 0.5 1"),
            "第二步只打包清单里的时间目录：{pack_script}"
        );
        assert_eq!(commands[2].args[0..2], ["multipass", "transfer"]);
        assert_eq!(
            commands[3].args,
            [
                "tar",
                "-xzf",
                "/tmp/kairos-back.tgz",
                "-C",
                "/data/cases/study-7"
            ]
        );
    }

    /// 某一步失败即中断：后续步骤不再跑，错误带上「哪一步」（排查就靠这一句）。
    /// 同时覆盖各步骤 `?` 的失败落点——成功的路径永远走不到那里。
    #[test]
    fn steps_stop_at_first_failure_and_name_the_step() {
        struct FailingRunner {
            fail_at: usize,
            seen: std::cell::Cell<usize>,
        }

        impl FailingRunner {
            fn new(fail_at: usize) -> Self {
                Self {
                    fail_at,
                    seen: std::cell::Cell::new(0),
                }
            }

            /// 推进到下一步：命中失败位即报「哪一步失败」，否则返回该步的应答。
            fn step(&self, what: &str, reply: &str) -> Result<String> {
                let index = self.seen.get() + 1;
                self.seen.set(index);
                if index == self.fail_at {
                    return Err(KairosError::io(format!("{what}失败：模拟")));
                }
                Ok(reply.to_string())
            }
        }

        impl HostRunner for FailingRunner {
            fn run(&self, _command: &HostCommand, what: &str) -> Result<()> {
                self.step(what, "").map(|_| ())
            }

            // 清单步骤要有真实的目录名，否则会被「没有可回传的结果」拦在打包之前。
            fn capture(&self, _command: &HostCommand, what: &str) -> Result<String> {
                self.step(what, "0\n1\n")
            }
        }

        for (fail_at, what) in [
            (1, "case 打包"),
            (2, "case 传输进虚拟机"),
            (3, "case 解压进虚拟机"),
        ] {
            let runner = FailingRunner::new(fail_at);
            let error = stage_case(&runner, Path::new("/tmp/x.tgz"), "/data/cases/study-1")
                .expect_err("第 {fail_at} 步失败必须冒泡");
            let message = error.message();
            assert!(message.contains(what), "错误必须点出失败的步骤：{message}");
            assert_eq!(runner.seen.get(), fail_at, "失败之后不应再跑后续步骤");
        }

        for (fail_at, what) in [
            (1, "读取结果目录清单"),
            (2, "求解结果打包"),
            (3, "求解结果传输回宿主"),
            (4, "求解结果解压到宿主"),
        ] {
            let runner = FailingRunner::new(fail_at);
            let error = copy_results(
                &runner,
                Path::new("/tmp/x.tgz"),
                "/data/cases/study-1",
                "/home/ubuntu/study-1",
            )
            .expect_err("第 {fail_at} 步失败必须冒泡");
            let message = error.message();
            // 清单失败与「清单为空」是两条路径：前者必须带上失败步骤名。
            if fail_at == 1 {
                assert!(message.contains(what), "错误必须点出失败的步骤：{message}");
            }
            assert_eq!(runner.seen.get(), fail_at, "失败之后不应再跑后续步骤");
        }

        let runner = FailingRunner::new(1);
        let error = poll_log_once(&runner, "/home/ubuntu/study-1", 0)
            .expect_err("回读失败必须冒泡（由调用方决定重试）");
        assert!(error.message().contains("求解日志回读"));
    }

    /// 求解没产生时间目录：报「没有可回传的结果」，不做空回传（也不继续打包传输）。
    #[test]
    fn copy_results_rejects_empty_time_dir_listing() {
        let runner = FakeRunner::with_replies(&["\n"]);
        let error = copy_results(
            &runner,
            Path::new("/tmp/x.tgz"),
            "/data/cases/study-7",
            "/home/ubuntu/study-7",
        )
        .expect_err("空清单必须报错");
        assert!(error.message().contains("没有可回传的结果时间目录"));
        assert_eq!(runner.taken().len(), 1, "空清单后不应再跑打包 / 传输");
    }

    /// 日志回读：哨兵之前的字节是新增日志，之后是退出码；出现求解器错误标记即置 aborted。
    #[test]
    fn poll_log_splits_log_from_exit_code_and_flags_abort() {
        let steady = FakeRunner::with_replies(&[&format!(
            "Time = 0.1\nTime = 0.2\n\n{}\n-\n",
            vm_logic::STREAM_MARK
        )]);
        let poll = poll_log_once(&steady, "/home/ubuntu/study-7", 0).expect("回读成功");
        assert_eq!(poll.lines, ["Time = 0.1", "Time = 0.2"]);
        assert_eq!(poll.exit_code, None, "退出码文件未出现 = 仍在求解");
        assert!(!poll.aborted);
        assert_eq!(poll.offset, "Time = 0.1\nTime = 0.2\n".len() as u64);

        let failed = FakeRunner::with_replies(&[&format!(
            "FOAM FATAL ERROR: something went wrong\n\n{}\n3\n",
            vm_logic::STREAM_MARK
        )]);
        let poll = poll_log_once(&failed, "/home/ubuntu/study-7", 0).expect("回读成功");
        assert!(poll.aborted, "求解器错误标记必须置 aborted：{poll:?}");
        assert_eq!(poll.exit_code, Some(3));

        // 应答用尽（真实场景里的「这一轮没有新内容」）：空输出、偏移不变、仍无退出码
        let quiet = poll_log_once(&steady, "/home/ubuntu/study-7", poll.offset).expect("回读成功");
        assert!(quiet.lines.is_empty());
        assert_eq!(quiet.exit_code, None);
        assert_eq!(quiet.offset, poll.offset);
    }
}
