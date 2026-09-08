//! 运行时依赖目录：外部工具的许可分级、安装策略与就绪判定。
//!
//! 分策略规则：
//! - **DirectDownload**：组件有官方单文件直链（预编译包或源码包，已实测可达）——
//!   应用内点击下载到受管目录。Kairos 只转发官方直链，不是分发方，
//!   GPL 组件亦不触发再分发义务；
//! - **GuidedInstall**：组件无单文件可下（纯在线安装流程）——打开官方页引导，
//!   装好后重新探测。
//!
//! 许可分级（LicenseKind）只决定徽标颜色与合规口径，与安装策略正交。
//! 注意：DirectDownload 只负责「拿到文件」；OpenFOAM 的编译安装步骤
//! 仍需在应用内编译流程或终端完成（hint 字段向用户说明）。

use crate::models::dependencies::{DownloadSpec, InstallStrategy, LicenseKind, RuntimeDependency};

/// 三平台同源的官方单文件直链。
fn source_download(url: &str) -> Option<DownloadSpec> {
    Some(DownloadSpec {
        macos: url.into(),
        windows: url.into(),
        linux: url.into(),
    })
}

/// 运行时依赖目录（求解链路 + 网格升级路线）。
pub fn catalog() -> Vec<RuntimeDependency> {
    vec![
        // 求解链路只依赖 OpenFOAM 本体：其 foamRun 模块化框架自 11 起内置
        // compressibleVoF 模块（compressibleInterFoam 的原生后继），无需第三方求解器。
        RuntimeDependency {
            id: "openfoam".into(),
            name: "OpenFOAM 14（.org）".into(),
            license: "GPL-3.0".into(),
            license_kind: LicenseKind::Gpl,
            strategy: InstallStrategy::DirectDownload,
            page_url: "https://openfoam.org/download/".into(),
            required: true,
            check_command: "blockMesh".into(),
            hint: "官方源码包（自动解压）。编译安装后 blockMesh / foamRun 进入 PATH 即就绪：                   Linux 推荐 apt 直接装预编译包（openfoam.org/download/14-ubuntu）；                   macOS 需源码编译（较耗时）。"
                .into(),
            download: source_download(
                "https://github.com/OpenFOAM/OpenFOAM-14/archive/refs/heads/master.tar.gz",
            ),
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
                // 注意：macOS 目录是 bin/macOS（ARM 版），且旧版本文件会被官方
                // 移除——升级版本号时三条 URL 必须同步更新。
                macos: "https://gmsh.info/bin/macOS/gmsh-4.15.2-MacOSARM-sdk.tgz".into(),
                windows: "https://gmsh.info/bin/Windows/gmsh-4.15.2-Windows64-sdk.zip".into(),
                linux: "https://gmsh.info/bin/Linux/gmsh-4.15.2-Linux64-sdk.tgz".into(),
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::dependencies::{InstallStrategy, LicenseKind};

    #[test]
    fn solver_chain_dependency_is_required_and_gpl() {
        let catalog = catalog();
        let openfoam = catalog.iter().find(|d| d.id == "openfoam").unwrap();
        assert!(openfoam.required);
        assert_eq!(openfoam.license_kind, LicenseKind::Gpl);
        // foamRun 模块化求解器随 OpenFOAM 本体分发，不单列第三方求解器依赖。
        assert!(!catalog.iter().any(|d| d.id == "openinjmoldsim"));
    }

    #[test]
    fn every_dependency_has_direct_download() {
        for dep in catalog() {
            assert!(
                dep.strategy == InstallStrategy::DirectDownload && dep.download.is_some(),
                "{} 应提供应用内直链下载",
                dep.id
            );
        }
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
