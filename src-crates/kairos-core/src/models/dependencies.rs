//! 运行时依赖模型：外部工具的许可分级与安装策略（应用内下载管理器）。

use serde::{Deserialize, Serialize};

/// 许可分级：决定 UI 徽标的颜色与合规口径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseKind {
    /// MIT：无再分发合规负担。
    Mit,
    /// GPL：Kairos 不做再分发方，只转发官方直链或引导安装。
    Gpl,
}

/// 安装策略（与许可分级正交：有官方预编译单文件的 GPL 组件也可直链下载）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStrategy {
    /// 有官方单文件直链（预编译包或源码包）：应用内点击下载（只转发官方直链，Kairos 不是分发方）。
    DirectDownload,
    /// 无单文件可下（纯在线安装流程）：打开官方页引导，装好后重新探测。
    GuidedInstall,
}

/// 组件的按平台下载地址（缺失平台 = 该平台无直链，走引导安装）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSpec {
    pub macos: String,
    pub windows: String,
    pub linux: String,
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
    /// 应用内直接下载地址（有官方预编译单文件的组件）；否则 None（引导安装）。
    #[serde(default)]
    pub download: Option<DownloadSpec>,
}

/// 组件更新检查结果（前端提示用）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub component_id: String,
    /// 当前安装的版本标签；清单中无记录（旧版下载）时为 None。
    pub installed_tag: Option<String>,
    /// 线上最新版本标签。
    pub latest_tag: Option<String>,
    pub update_available: bool,
}
