//! 运行时依赖目录：外部工具的许可分级、安装策略与就绪判定。
//!
//! 分策略规则：
//! - **MIT 组件**：`DirectDownload`——点击直接下载到应用数据目录（MIT 允许自由再分发）；
//! - **GPL 组件**：`GuidedInstall`——打开官方页由用户自行安装（Kairos 不分发 GPL 二进制，
//!   规避源码附带义务；与合规文档 ai-docs/decisions/openfoam-gpl-compliance.md 一致）。
//!
//! 当前目录中没有 MIT 运行时组件；未来新增 MIT 组件时按上表选择 DirectDownload 即可。

use crate::models::dependencies::{InstallStrategy, LicenseKind, RuntimeDependency};

/// 运行时依赖目录（求解链路 + 网格升级路线）。
pub fn catalog() -> Vec<RuntimeDependency> {
    vec![
        RuntimeDependency {
            id: "openfoam".into(),
            name: "OpenFOAM 7（.org）".into(),
            license: "GPL-3.0".into(),
            license_kind: LicenseKind::Gpl,
            strategy: InstallStrategy::GuidedInstall,
            page_url: "https://openfoam.org/download/".into(),
            required: true,
            check_command: "blockMesh".into(),
            hint: "命令 blockMesh 可用即视为就绪。".into(),
            download: None,
        },
        RuntimeDependency {
            id: "openinjmoldsim".into(),
            name: "openInjMoldSim 求解器".into(),
            license: "GPL-3.0".into(),
            license_kind: LicenseKind::Gpl,
            strategy: InstallStrategy::GuidedInstall,
            page_url: "https://github.com/krebeljk/openInjMoldSim".into(),
            required: true,
            check_command: "openInjMoldSim".into(),
            hint: "命令 openInjMoldSim 可用即视为就绪（./Allwmake 编译后加入 PATH）。".into(),
            download: None,
        },
        RuntimeDependency {
            id: "gmsh".into(),
            name: "Gmsh 网格引擎".into(),
            license: "GPL-2.0-or-later".into(),
            license_kind: LicenseKind::Gpl,
            strategy: InstallStrategy::DirectDownload,
            page_url: "https://gmsh.info/#Download".into(),
            required: false,
            check_command: "gmsh".into(),
            hint: "T22 Delaunay 网格升级路线（条件触发，可选）。".into(),
            download: Some(crate::models::dependencies::DownloadSpec {
                macos: "https://gmsh.info/bin/macOSX/gmsh-4.12.2-MacOSX-sdk.tgz".into(),
                windows: "https://gmsh.info/bin/Windows/gmsh-4.12.2-Windows64.zip".into(),
                linux: "https://gmsh.info/bin/Linux/gmsh-4.12.2-Linux64-sdk.tgz".into(),
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::dependencies::{InstallStrategy, LicenseKind};

    #[test]
    fn solver_chain_dependencies_are_required_and_gpl() {
        let catalog = catalog();
        let openfoam = catalog.iter().find(|d| d.id == "openfoam").unwrap();
        let solver = catalog.iter().find(|d| d.id == "openinjmoldsim").unwrap();
        assert!(openfoam.required && solver.required);
        assert_eq!(openfoam.license_kind, LicenseKind::Gpl);
        assert_eq!(openfoam.strategy, InstallStrategy::GuidedInstall);
    }

    #[test]
    fn gmsh_is_optional_with_direct_download() {
        let catalog = catalog();
        let gmsh = catalog.iter().find(|d| d.id == "gmsh").unwrap();
        assert!(!gmsh.required);
        assert!(gmsh.download.is_some(), "Gmsh 应提供应用内直接下载地址");
    }

    #[test]
    fn every_dependency_has_page_and_check() {
        for dep in catalog() {
            assert!(
                dep.page_url.starts_with("https://"),
                "{} 页面应为 https",
                dep.id
            );
            assert!(!dep.check_command.is_empty());
            assert!(!dep.hint.is_empty());
        }
    }
}
