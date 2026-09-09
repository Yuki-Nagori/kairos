//! 虚拟机适配的纯逻辑：provider 判定、命令行参数构造与输出解析。
//! 进程副作用（安装 / 启动 / Shell 子进程）全部住在 src-tauri 适配层；
//! 这里只产出参数与解析结果，保证可测（覆盖率门槛适用）。

use crate::models::vm::{VmProviderKind, VmState, VmStatus};

/// 受管实例名：multipass 虚拟机与 WSL 发行版共用一个标识口径。
pub const INSTANCE_NAME: &str = "kairos";
/// WSL 受管发行版（`wsl --install -d` 的目标；v1 固定，自装发行版不受管）。
pub const WSL_DISTRO: &str = "Ubuntu-24.04";

/// 平台 → provider 的唯一映射；Linux 原生不需要虚拟机，返回 None。
pub fn provider_for(os: &str) -> Option<VmProviderKind> {
    match os {
        "macos" => Some(VmProviderKind::Multipass),
        "windows" => Some(VmProviderKind::Wsl),
        _ => None,
    }
}

/// 工具版本探测命令（成功退出即视为已安装）。
pub fn version_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec!["multipass".into(), "--version".into()],
        VmProviderKind::Wsl => vec!["wsl".into(), "--status".into()],
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
    }
}

/// 实例清单 / 状态查询命令。
pub fn instance_info_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec!["multipass".into(), "info".into(), INSTANCE_NAME.into()],
        VmProviderKind::Wsl => vec!["wsl".into(), "-l".into(), "-v".into()],
    }
}

/// 首次创建实例（multipass 拉起 Ubuntu 镜像；WSL 安装发行版）。
pub fn launch_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec![
            "multipass".into(),
            "launch".into(),
            "--name".into(),
            INSTANCE_NAME.into(),
            "--cpus".into(),
            "4".into(),
            "--mem".into(),
            "8G".into(),
            "--disk".into(),
            "40G".into(),
        ],
        VmProviderKind::Wsl => vec![
            "wsl".into(),
            "--install".into(),
            "-d".into(),
            WSL_DISTRO.into(),
        ],
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
        VmProviderKind::Wsl => None,
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
    }
}

/// 停止受管实例（退出联动时以 detached 方式派发，不等其退出）。
pub fn stop_args(provider: VmProviderKind) -> Vec<String> {
    match provider {
        VmProviderKind::Multipass => vec!["multipass".into(), "stop".into(), INSTANCE_NAME.into()],
        VmProviderKind::Wsl => vec!["wsl".into(), "--terminate".into(), WSL_DISTRO.into()],
    }
}

/// 解析 `multipass info <name>` 输出（命令失败由适配层先行判 Missing）。
pub fn parse_multipass_state(text: &str) -> VmState {
    for line in text.lines() {
        if let Some(value) = line.trim().strip_prefix("State:") {
            return match value.trim() {
                "RUNNING" => VmState::Running,
                "STARTING" | "DELAYING SHUTDOWN" => VmState::Starting,
                "STOPPED" | "SUSPENDED" => VmState::Stopped,
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

/// 组装状态视图与用户提示。
pub fn build_status(
    provider: VmProviderKind,
    tool_installed: bool,
    instance_state: VmState,
) -> VmStatus {
    let instance_name = match provider {
        VmProviderKind::Multipass => INSTANCE_NAME.to_string(),
        VmProviderKind::Wsl => WSL_DISTRO.to_string(),
    };
    let hint = if !tool_installed {
        match provider {
            VmProviderKind::Multipass => {
                "未检测到 Multipass。点击「安装虚拟机」（约需数分钟，依赖 Homebrew）。".to_string()
            }
            VmProviderKind::Wsl => {
                "未检测到 WSL2。点击「安装虚拟机」（需管理员授权，完成后可能要求重启）。"
                    .to_string()
            }
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
    fn provider_maps_by_os_and_rejects_native_linux() {
        assert_eq!(provider_for("macos"), Some(VmProviderKind::Multipass));
        assert_eq!(provider_for("windows"), Some(VmProviderKind::Wsl));
        assert_eq!(provider_for("linux"), None);
    }

    #[test]
    fn args_builders_cover_both_providers() {
        for (provider, bin) in [
            (VmProviderKind::Multipass, "multipass"),
            (VmProviderKind::Wsl, "wsl"),
        ] {
            assert_eq!(version_args(provider)[0], bin);
            assert_eq!(instance_info_args(provider)[0], bin);
            assert_eq!(launch_args(provider)[0], bin);
            assert_eq!(shell_args(provider)[0], bin);
            assert_eq!(stop_args(provider)[0], bin);
        }
        // multipass：实例名贯穿 launch/start/shell/stop；WSL：无独立 start 步骤
        assert!(launch_args(VmProviderKind::Multipass).contains(&INSTANCE_NAME.to_string()));
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
        let info = "Name: kairos\nState: RUNNING\nIPv4: 192.168.64.3\n";
        assert_eq!(parse_multipass_state(info), VmState::Running);
        assert_eq!(parse_multipass_state("State: STOPPED\n"), VmState::Stopped);
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
