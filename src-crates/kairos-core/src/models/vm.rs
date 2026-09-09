//! 虚拟机运行时模型：macOS 用 Multipass（Ubuntu 虚拟机）、Windows 用 WSL2
//! 承载 moldingFoam 的 Linux 求解包。DTO 形状是前后端契约，改动必须跑契约测试。

use serde::{Deserialize, Serialize};

/// 虚拟机提供方（按平台一对一：macOS → Multipass，Windows → WSL）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmProviderKind {
    Multipass,
    Wsl,
}

/// 受管实例的状态；Missing 表示实例尚未创建（工具可能已装）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmState {
    Missing,
    Stopped,
    Starting,
    Running,
    Unknown,
}

/// 虚拟机运行时状态视图（探测结果 + 面向用户的提示）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VmStatus {
    pub provider: VmProviderKind,
    /// 运行时工具（multipass / wsl）是否已安装。
    pub tool_installed: bool,
    /// 受管实例名（multipass 实例 / WSL 发行版）。
    pub instance_name: String,
    pub instance_state: VmState,
    /// 面向用户的就绪状态提示。
    pub hint: String,
}
