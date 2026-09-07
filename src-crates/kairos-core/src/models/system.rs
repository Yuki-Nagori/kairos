//! 系统 / 应用信息的 IPC 模型。

use serde::Serialize;

/// 应用基本信息：版本与平台展示于标题栏，同时充当 IPC 连通性探测的载荷。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub name: String,
    pub version: String,
    /// 运行平台（`std::env::consts::OS`），如 "macos" / "windows" / "linux"。
    pub os: String,
}
