//! 虚拟机适配的纯逻辑：provider 判定、命令行参数构造与输出解析。
//! 进程副作用（安装 / 启动 / Shell 子进程）全部住在 src-tauri 适配层；
//! 这里只产出参数与解析结果，保证可测（覆盖率门槛适用）。

use crate::models::vm::{VmProviderKind, VmState, VmStatus};

/// 受管实例名：Kairos 自己管理的 multipass 虚拟机，与 moldingFoam README
/// 的 of14 开发虚拟机相互独立（规格与镜像仍对齐其验证配方）。
pub const INSTANCE_NAME: &str = "kairos";
/// WSL 受管发行版（`wsl --install -d` 的目标；v1 固定，自装发行版不受管）。
pub const WSL_DISTRO: &str = "Ubuntu-24.04";

/// 平台 → provider 的唯一映射；Linux 原生即目标环境，无需虚拟机。
pub fn provider_for(os: &str) -> Option<VmProviderKind> {
    match os {
        "macos" => Some(VmProviderKind::Multipass),
        "windows" => Some(VmProviderKind::Wsl),
        "linux" => Some(VmProviderKind::Native),
        _ => None,
    }
}

/// 工具版本探测命令（成功退出即视为已安装）。
pub fn version_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec!["multipass".into(), "--version".into()],
        VmProviderKind::Wsl => vec!["wsl".into(), "--status".into()],
        // 原生环境恒就绪：探测命令用 /usr/bin/true（恒成功），无需安装。
        VmProviderKind::Native => vec!["true".into()],
    }
}

/// 一键安装命令。Windows 走 UAC 提权（输出不可回传）；macOS 依赖 Homebrew。
pub fn install_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec![
            "brew".into(),
            "install".into(),
            "--cask".into(),
            "multipass".into(),
        ],
        VmProviderKind::Wsl => vec![
            "powershell".into(),
            "-Command".into(),
            "Start-Process wsl -ArgumentList '--install' -Verb RunAs".into(),
        ],
        VmProviderKind::Native => vec![],
    }
}

/// 实例清单 / 状态查询命令。
pub fn instance_info_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec!["multipass".into(), "info".into(), INSTANCE_NAME.into()],
        VmProviderKind::Wsl => vec!["wsl".into(), "-l".into(), "-v".into()],
        VmProviderKind::Native => vec!["true".into()],
    }
}

/// 虚拟机资源规格（multipass 用；wsl/原生环境由系统自管，不消费此结构）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmResources {
    pub cpus: u32,
    pub memory_gib: u32,
    pub disk_gib: u32,
}

/// 由宿主机规格推导虚拟机默认规格。
/// - CPU：宿主核数的一半（给宿主留余量），收敛到 [2, 8]；
/// - 内存：宿主内存的一半，收敛到 [4, 16] GiB；
/// - 磁盘：multipass 磁盘是稀疏分配、按需增长，固定 80G 足够 openfoam14 + bundle。
pub fn plan_resources(host_cpus: u32, host_memory_gib: u32) -> VmResources {
    VmResources {
        cpus: (host_cpus / 2).clamp(2, 8),
        memory_gib: (host_memory_gib / 2).clamp(4, 16),
        disk_gib: 80,
    }
}

/// 视为中国大陆时区的 IANA / Windows 时区名（本机判断，不发起网络请求）。
pub const CN_TIMEZONES: &[&str] = &[
    "Asia/Shanghai",
    "Asia/Urumqi",
    "Asia/Chongqing",
    "Asia/Harbin",
    "Asia/Kashgar",
    "China Standard Time",
];

/// 中国大陆时区 → 云镜像走国内镜像源（multipassd 直连上游下载慢）。
pub fn is_china_timezone(tz: &str) -> bool {
    CN_TIMEZONES.contains(&tz)
}

/// 平台架构 → Ubuntu 云镜像文件名（noble / 24.04）。
pub fn image_file_name(arch: &str) -> Option<&'static str> {
    match arch {
        "aarch64" => Some("noble-server-cloudimg-arm64.img"),
        "x86_64" => Some("noble-server-cloudimg-amd64.img"),
        _ => None,
    }
}

/// 国内镜像源候选（清华 TUNA，实测可达；按序尝试）。
pub fn image_mirror_urls(arch: &str) -> Vec<String> {
    let Some(name) = image_file_name(arch) else {
        return Vec::new();
    };
    vec![format!(
        "https://mirrors.tuna.tsinghua.edu.cn/ubuntu-cloud-images/noble/current/{name}"
    )]
}

/// 首次创建实例（multipass 按宿主规格拉起 Ubuntu；WSL 安装发行版）。
/// `image` 为本地云镜像路径（国内镜像源预下载后 file:// 导入）；
/// None 时由 multipass 直接拉取 `24.04`。
pub fn launch_args(
    provider: VmProviderKind,
    resources: &VmResources,
    image: Option<&str>,
) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => {
            let source = match image {
                Some(path) => format!("file://{path}"),
                None => "24.04".into(),
            };
            vec![
                "multipass".into(),
                "launch".into(),
                "--name".into(),
                INSTANCE_NAME.into(),
                "--cpus".into(),
                resources.cpus.to_string(),
                // --mem 已被 multipass 弃用（真机警告），新脚本一律用 --memory。
                "--memory".into(),
                format!("{}G", resources.memory_gib),
                "--disk".into(),
                format!("{}G", resources.disk_gib),
                source,
            ]
        }
        VmProviderKind::Wsl => vec![
            "wsl".into(),
            "--install".into(),
            "-d".into(),
            WSL_DISTRO.into(),
        ],
        VmProviderKind::Native => vec!["true".into()],
    }
}

/// 把已存在但停止的实例拉起来（WSL 无此步骤，实例随首次执行自启）。
pub fn start_args(provider: VmProviderKind) -> Option<Vec<String>> {
    match provider {
        VmProviderKind::Multipass => Some(vec![
            "multipass".into(),
            "start".into(),
            INSTANCE_NAME.into(),
        ]),
        VmProviderKind::Wsl | VmProviderKind::Native => None,
    }
}

/// 应用内 Shell：multipass 走 exec bash（管道友好，规避非 TTY 限制）；
/// wsl 本身就是管道友好的 Linux 进程桥。
pub fn shell_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => {
            vec![
                "multipass".into(),
                "exec".into(),
                INSTANCE_NAME.into(),
                "--".into(),
                "bash".into(),
                "--login".into(),
            ]
        }
        VmProviderKind::Wsl => vec!["wsl".into(), "-d".into(), WSL_DISTRO.into()],
        // 原生环境：Shell 就是本地登录 bash。
        VmProviderKind::Native => vec!["bash".into(), "--login".into()],
    }
}

/// 停止受管实例（退出联动时以 detached 方式派发，不等其退出）。
pub fn stop_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec!["multipass".into(), "stop".into(), INSTANCE_NAME.into()],
        VmProviderKind::Wsl => vec!["wsl".into(), "--terminate".into(), WSL_DISTRO.into()],
        VmProviderKind::Native => vec!["true".into()],
    }
}

/// 解析 `multipass info <name>` 输出（命令失败由适配层先行判 Missing）。
pub fn parse_multipass_state(text: &str) -> VmState {
    for line in text.lines() {
        if let Some(value) = line.trim().strip_prefix("State:") {
            // 新版 multipass 打印 Running（首字母大写），旧版全大写：统一小写匹配。
            return match value.trim().to_lowercase().as_str() {
                "running" => VmState::Running,
                "starting" | "delaying shutdown" => VmState::Starting,
                "stopped" | "suspended" => VmState::Stopped,
                _ => VmState::Unknown,
            };
        }
    }
    VmState::Unknown
}

/// 解析 `wsl -l -v` 输出（UTF-16LE 解码后传入）。
/// 行形如 `  Ubuntu-24.04     Running           2`；缺发行版行即 Missing。
pub fn parse_wsl_list(text: &str) -> VmState {
    let mut found = VmState::Missing;
    for line in text.lines() {
        // 行首的 `*` 是「默认发行版」标记，先剥掉再对列。
        let line = line.trim_start();
        let line = line.strip_prefix("* ").unwrap_or(line);
        let mut tokens = line.split_whitespace();
        if tokens.next() == Some(WSL_DISTRO) {
            found = match tokens.next() {
                Some("Running") => VmState::Running,
                Some("Installing") | Some("Converting") => VmState::Starting,
                Some("Stopped") => VmState::Stopped,
                _ => VmState::Unknown,
            };
        }
    }
    found
}

/// 解码 wsl 系列命令的输出：Windows 下是 UTF-16LE（带 BOM 与 NUL 填充），
/// 其余平台按 UTF-8 处理。统一在本函数内归一成干净的多行文本。
pub fn decode_wsl_output(bytes: &[u8]) -> String {
    let looks_utf16 = bytes.len() >= 2 && bytes.get(1) == Some(&0);
    let text = if looks_utf16 {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    text.replace('\0', "")
}

/// 清洗终端行：剥掉 ANSI 转义序列与全部控制字符（multipass 的转圈动画
/// 既有 CSI 也有退格 \x08 重绘、夹 NUL），并把 \r / \x08 分段折叠为
/// 最后一帧（spinner 语义：只有最新帧有意义）。
pub fn clean_terminal_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => {
                // CSI 序列：跳过到结束字母；其他转义跳过单字符
                if chars.next() == Some('[') {
                    for follow in chars.by_ref() {
                        if follow.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            }
            // 回退重绘（回车 / 退格）：清空已收字符，只保留最后一帧
            '\r' | '\u{8}' => out.clear(),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.trim_end().to_string()
}

/// 组装状态视图与用户提示。
pub fn build_status(
    provider: VmProviderKind,
    tool_installed: bool,
    instance_state: VmState,
) -> VmStatus {
    let instance_name = match provider {
        VmProviderKind::Multipass => INSTANCE_NAME.to_string(),
        VmProviderKind::Wsl => WSL_DISTRO.to_string(),
        VmProviderKind::Native => "localhost".to_string(),
    };
    // 原生环境无需虚拟机：恒就绪、恒可进 Shell，安装/启动/停止语义全部短路。
    let (tool_installed, instance_state) = if provider == VmProviderKind::Native {
        (true, VmState::Running)
    } else {
        (tool_installed, instance_state)
    };
    let hint = if provider == VmProviderKind::Native {
        "Linux 原生环境，无需虚拟机，可直接进入 Shell。".to_string()
    } else if !tool_installed {
        match provider {
            VmProviderKind::Multipass => {
                "未检测到 Multipass。点击「安装虚拟机」（约需数分钟，依赖 Homebrew）。".to_string()
            }
            VmProviderKind::Wsl => {
                "未检测到 WSL2。点击「安装虚拟机」（需管理员授权，完成后可能要求重启）。"
                    .to_string()
            }
            VmProviderKind::Native => unreachable!(),
        }
    } else {
        match instance_state {
            VmState::Missing => match provider {
                VmProviderKind::Multipass => {
                    "Multipass 已就绪，虚拟机尚未创建。点击「启动虚拟机」开始拉起 Ubuntu 实例。"
                        .to_string()
                }
                VmProviderKind::Wsl => {
                    "WSL2 已就绪，Ubuntu-24.04 尚未安装。点击「启动虚拟机」安装发行版。".to_string()
                }
                VmProviderKind::Native => unreachable!(),
            },
            VmState::Stopped => "虚拟机已创建但未运行。点击「启动虚拟机」。".to_string(),
            VmState::Starting => "虚拟机启动中…".to_string(),
            VmState::Running => "虚拟机运行中，可进入 Shell。".to_string(),
            VmState::Unknown => "虚拟机状态未知，请重新探测。".to_string(),
        }
    };
    VmStatus {
        provider,
        tool_installed,
        instance_name,
        instance_state,
        hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_maps_by_os() {
        assert_eq!(provider_for("macos"), Some(VmProviderKind::Multipass));
        assert_eq!(provider_for("windows"), Some(VmProviderKind::Wsl));
        assert_eq!(provider_for("linux"), Some(VmProviderKind::Native));
        assert_eq!(provider_for("freebsd"), None);
    }

    #[test]
    fn native_provider_needs_no_vm() {
        let status = build_status(VmProviderKind::Native, false, VmState::Missing);
        assert!(status.tool_installed);
        assert_eq!(status.instance_state, VmState::Running);
        assert_eq!(status.instance_name, "localhost");
        assert!(status.hint.contains("无需虚拟机"));
        // Shell 就是本地登录 bash；安装/启动/停止全部为无操作短路。
        assert_eq!(shell_args(VmProviderKind::Native), vec!["bash", "--login"]);
        assert!(start_args(VmProviderKind::Native).is_none());
        assert!(install_args(VmProviderKind::Native).is_empty());
        for args in [
            version_args(VmProviderKind::Native),
            instance_info_args(VmProviderKind::Native),
            launch_args(VmProviderKind::Native, &plan_resources(8, 16), None),
            stop_args(VmProviderKind::Native),
        ] {
            assert_eq!(args, vec!["true"]);
        }
    }

    #[test]
    fn china_timezone_uses_mirror_image_for_launch() {
        assert!(is_china_timezone("Asia/Shanghai"));
        assert!(is_china_timezone("China Standard Time"));
        assert!(!is_china_timezone("America/New_York"));
        assert!(!is_china_timezone(""));

        assert_eq!(
            image_file_name("aarch64"),
            Some("noble-server-cloudimg-arm64.img")
        );
        assert_eq!(image_file_name("riscv"), None);
        let urls = image_mirror_urls("x86_64");
        assert_eq!(urls.len(), 1);
        assert!(urls[0].starts_with(
            "https://mirrors.tuna.tsinghua.edu.cn/ubuntu-cloud-images/noble/current/"
        ));
        assert!(urls[0].ends_with("noble-server-cloudimg-amd64.img"));

        // file:// 导入：本地镜像替换 24.04 位置参数
        let launch = launch_args(
            VmProviderKind::Multipass,
            &plan_resources(8, 16),
            Some("/tmp/noble.img"),
        );
        assert!(launch.contains(&"file:///tmp/noble.img".to_string()));
        assert!(!launch.contains(&"24.04".to_string()));
        let direct = launch_args(VmProviderKind::Multipass, &plan_resources(8, 16), None);
        assert_eq!(direct.last().unwrap(), "24.04");
    }

    #[test]
    fn resources_plan_scales_with_host_and_clamps() {
        // 大机器：封顶 8 核 / 16G（moldingFoam 的推荐规格），磁盘恒 80G。
        assert_eq!(
            plan_resources(16, 64),
            VmResources {
                cpus: 8,
                memory_gib: 16,
                disk_gib: 80
            }
        );
        // 中等机器：宿主的一半。
        assert_eq!(plan_resources(12, 32).cpus, 6);
        assert_eq!(plan_resources(12, 32).memory_gib, 16);
        // 小机器：保底 2 核 / 4G，宿主与虚拟机各占一半。
        assert_eq!(plan_resources(4, 8).cpus, 2);
        assert_eq!(plan_resources(4, 8).memory_gib, 4);
    }

    #[test]
    fn args_builders_cover_both_providers() {
        let resources = plan_resources(8, 16);
        for (provider, bin) in [
            (VmProviderKind::Multipass, "multipass"),
            (VmProviderKind::Wsl, "wsl"),
        ] {
            assert_eq!(version_args(provider)[0], bin);
            assert_eq!(instance_info_args(provider)[0], bin);
            assert_eq!(launch_args(provider, &resources, None)[0], bin);
            assert_eq!(shell_args(provider)[0], bin);
            assert_eq!(stop_args(provider)[0], bin);
        }
        // multipass：实例名贯穿 launch/start/shell/stop；WSL：无独立 start 步骤
        let launch = launch_args(VmProviderKind::Multipass, &resources, None);
        assert!(launch.contains(&INSTANCE_NAME.to_string()));
        // 规格来自宿主探测结果：4 核 / 8G / 80G + Ubuntu 24.04（plan_resources(8,16)）
        let cpus_pos = launch.iter().position(|a| a == "--cpus").unwrap();
        assert_eq!(launch[cpus_pos + 1], "4");
        let mem_pos = launch.iter().position(|a| a == "--memory").unwrap();
        assert_eq!(launch[mem_pos + 1], "8G");
        assert_eq!(launch.last().unwrap(), "24.04");
        assert_eq!(
            start_args(VmProviderKind::Multipass).as_deref(),
            Some(
                &[
                    "multipass".to_string(),
                    "start".to_string(),
                    INSTANCE_NAME.to_string()
                ][..]
            )
        );
        assert!(start_args(VmProviderKind::Wsl).is_none());
        assert!(install_args(VmProviderKind::Multipass).contains(&"--cask".to_string()));
        assert!(
            install_args(VmProviderKind::Wsl)
                .iter()
                .any(|a| a.contains("RunAs"))
        );
        assert_eq!(shell_args(VmProviderKind::Wsl).last().unwrap(), WSL_DISTRO);
    }

    #[test]
    fn multipass_state_parser_covers_all_lines() {
        // 新版 multipass 打印 Running（首字母大写），旧版全大写
        let info = "Name: kairos\nState: Running\nIPv4: 192.168.64.3\n";
        assert_eq!(parse_multipass_state(info), VmState::Running);
        assert_eq!(parse_multipass_state("State: RUNNING\n"), VmState::Running);
        assert_eq!(parse_multipass_state("State: Stopped\n"), VmState::Stopped);
        assert_eq!(
            parse_multipass_state("State: STARTING\n"),
            VmState::Starting
        );
        assert_eq!(parse_multipass_state("State: DELETED\n"), VmState::Unknown);
        assert_eq!(parse_multipass_state("no state here"), VmState::Unknown);
    }

    #[test]
    fn wsl_list_parser_requires_distro_row() {
        let list =
            "  NAME            STATE           VERSION\n* Ubuntu-24.04    Running         2\n";
        assert_eq!(parse_wsl_list(list), VmState::Running);
        assert_eq!(
            parse_wsl_list("  Ubuntu-24.04    Stopped         2"),
            VmState::Stopped
        );
        assert_eq!(
            parse_wsl_list("  Ubuntu-24.04    Installing      2"),
            VmState::Starting
        );
        assert_eq!(
            parse_wsl_list("  Debian          Running         2"),
            VmState::Missing
        );
        assert_eq!(parse_wsl_list(""), VmState::Missing);
        assert_eq!(
            parse_wsl_list("* Ubuntu-24.04    Weird           2"),
            VmState::Unknown
        );
    }

    #[test]
    fn wsl_output_decoder_handles_utf16_and_utf8() {
        // `wsl -l -v` 在 Windows 上输出 UTF-16LE：逐字符低位在前
        let mut utf16 = Vec::new();
        for unit in "  Ubuntu-24.04\tRunning\r\n".encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(decode_wsl_output(&utf16), "  Ubuntu-24.04\tRunning\r\n");
        assert_eq!(decode_wsl_output(b"plain utf8"), "plain utf8");
        // 非法 UTF-8 落入替换字符而非 panic
        assert!(decode_wsl_output(&[0xff, 0xfe]).contains('\u{fffd}'));
    }

    #[test]
    fn terminal_line_cleaner_strips_ansi_and_collapses_carriage_returns() {
        assert_eq!(
            clean_terminal_line("\u{1b}[2K\u{1b}[0A\u{1b}[0EStarting kairos  / "),
            "Starting kairos  /"
        );
        // \r 折叠：spinner 多帧只保留最后一帧
        assert_eq!(
            clean_terminal_line("frame one\rframe two\rframe three"),
            "frame three"
        );
        // 退格重绘 + 夹 NUL（multipass 真机形态）
        assert_eq!(
            clean_terminal_line("Starting kairos  0/0-0\\0|0/0"),
            "Starting kairos  0/0-0\\0|0/0"
        );
        assert_eq!(clean_terminal_line("a\u{8}b\u{8}c"), "c");
        assert_eq!(clean_terminal_line("plain\u{0}line"), "plainline");
        assert_eq!(clean_terminal_line("plain line"), "plain line");
        // 纯控制序列清洗后为空
        assert_eq!(clean_terminal_line("\u{1b}[?25l"), "");
    }

    #[test]
    fn status_hint_branches_cover_every_combination() {
        for provider in [VmProviderKind::Multipass, VmProviderKind::Wsl] {
            let missing_tool = build_status(provider, false, VmState::Missing);
            assert!(!missing_tool.tool_installed);
            assert!(missing_tool.hint.contains("安装虚拟机"));
            assert!(!missing_tool.instance_name.is_empty());

            let no_instance = build_status(provider, true, VmState::Missing);
            assert!(no_instance.hint.contains("启动虚拟机"));
            assert!(
                build_status(provider, true, VmState::Stopped)
                    .hint
                    .contains("启动")
            );
            assert!(
                build_status(provider, true, VmState::Starting)
                    .hint
                    .contains("启动中")
            );
            assert!(
                build_status(provider, true, VmState::Running)
                    .hint
                    .contains("Shell")
            );
            assert!(
                build_status(provider, true, VmState::Unknown)
                    .hint
                    .contains("重新探测")
            );
        }
    }
}
