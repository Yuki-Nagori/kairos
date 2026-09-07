//! 运行时依赖模型：外部工具的许可分级与安装策略（应用内下载管理器，T09/T18）。

use serde::{Deserialize, Serialize};

/// 许可分级：决定安装策略与 UI 徽标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseKind {
    /// MIT：可点击直接下载（无合规负担）。
    Mit,
    /// GPL：只走引导安装（打开官方页，用户自行安装），Kairos 不分发。
    Gpl,
}

/// 安装策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStrategy {
    /// MIT 组件：点击直接下载到应用数据目录。
    DirectDownload,
    /// GPL 组件：打开官方下载/编译页，用户自行安装后 Kairos 重新探测。
    GuidedInstall,
}

/// 运行时依赖目录项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDependency {
    pub id: String,
    pub name: String,
    /// 许可证 SPDX 标识。
    pub license: String,
    pub license_kind: LicenseKind,
    pub strategy: InstallStrategy,
    /// 官方下载 / 编译说明页（引导安装的落地页）。
    pub page_url: String,
    /// 求解链路是否强依赖（缺失时求解不可用）。
    pub required: bool,
    /// 就绪判定所依赖的命令名。
    pub check_command: String,
    pub hint: String,
}
