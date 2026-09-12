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
//! 注意：DirectDownload 只负责「拿到文件」；moldingFoam 走官方 release 的
//! 预编译 bundle（OpenFOAM-14 完整环境树 + 求解模块），解压即用、无需编译，
//! 虚拟机内的部署见 src-tauri 的 vm_deploy_bundle。

use crate::models::dependencies::{DownloadSpec, InstallStrategy, LicenseKind, RuntimeDependency};

/// 三平台同源的官方单文件直链。
fn source_download(url: &str) -> Option<DownloadSpec> {
    Some(DownloadSpec {
        macos: url.into(),
        windows: url.into(),
        linux: url.into(),
    })
}

/// 组件是否来自可在线检查更新的 release 流（`releases/latest` 直链）。
/// 静态直链组件（如 Gmsh 固定版本文件）没有可查询的版本源，不支持。
pub fn is_release_updatable(dep: &RuntimeDependency) -> bool {
    dep.download
        .as_ref()
        .map(|spec| spec.linux.ends_with("/releases/latest"))
        .unwrap_or(false)
}

/// release 资产名是否匹配宿主架构（moldingFoam bundle 按 linux64 / linuxArm64
/// 分包；匹配对大小写不敏感）。无法识别的架构一律不匹配。
pub fn bundle_asset_matches_arch(asset_name: &str, arch: &str) -> bool {
    let lower = asset_name.to_lowercase();
    match arch {
        "aarch64" => lower.contains("arm64"),
        "x86_64" => lower.contains("linux64"),
        _ => false,
    }
}

/// 运行时依赖目录（求解链路 + 网格升级路线）。
pub fn catalog() -> Vec<RuntimeDependency> {
    vec![
        // 注塑求解环境 = moldingFoam 仓库发布的 bundle：基于 OpenFOAM-14
        // 官方环境树构建（内含 libmoldingFoam），但它是一个独立项目，不等于
        // OpenFOAM 本体（GPL-3.0，Kairos 只转发官方直链）。下载
        // `releases/latest` 形态的 URL，适配层在下载时按宿主架构解析
        // 具体资产（资产名含日期，无法用固定 latest/download 文件名）。
        // 自动解压即用：无需编译，解压目录内 platforms/*/bin 会被求解时
        // 自动加入 PATH 前缀。
        RuntimeDependency {
            id: "moldingfoam".into(),
            name: "moldingFoam（基于 OpenFOAM-14）".into(),
            license: "GPL-3.0".into(),
            license_kind: LicenseKind::Gpl,
            strategy: InstallStrategy::DirectDownload,
            page_url: "https://github.com/Yuki-Nagori/moldingFoam/releases".into(),
            required: true,
            check_command: "blockMesh".into(),
            hint: "官方 release 预编译包（OpenFOAM-14 完整环境树 + 注塑求解模块，约 120MB）。                    自动解压即用，无需编译。".into(),
            download: source_download(
                "https://github.com/Yuki-Nagori/moldingFoam/releases/latest",
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
        let moldingfoam = catalog.iter().find(|d| d.id == "moldingfoam").unwrap();
        assert!(moldingfoam.required);
        assert_eq!(moldingfoam.license_kind, LicenseKind::Gpl);
        // 求解环境独立成条目，不再单列第三方求解器依赖。
        assert!(!catalog.iter().any(|d| d.id == "openinjmoldsim"));
        assert!(!catalog.iter().any(|d| d.id == "openfoam"));
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
    fn only_release_stream_components_are_updatable() {
        let catalog = catalog();
        let moldingfoam = catalog
            .iter()
            .find(|d| d.id == "moldingfoam")
            .expect("moldingfoam entry");
        assert!(is_release_updatable(moldingfoam));
        let gmsh = catalog.iter().find(|d| d.id == "gmsh").expect("gmsh entry");
        assert!(!is_release_updatable(gmsh));
    }

    #[test]
    fn bundle_asset_matches_host_arch() {
        assert!(bundle_asset_matches_arch(
            "moldingFoam-openfoam14-linuxArm64GccDPInt32Opt-20260909.tar.xz",
            "aarch64"
        ));
        assert!(!bundle_asset_matches_arch(
            "moldingFoam-openfoam14-linuxArm64GccDPInt32Opt-20260909.tar.xz",
            "x86_64"
        ));
        assert!(bundle_asset_matches_arch(
            "moldingFoam-openfoam14-linux64GccDPInt32Opt-20260909.tar.xz",
            "x86_64"
        ));
        assert!(!bundle_asset_matches_arch(
            "moldingFoam-openfoam14-linux64GccDPInt32Opt-20260909.tar.xz",
            "aarch64"
        ));
        // 未知架构一律不匹配（不猜）
        assert!(!bundle_asset_matches_arch(
            "moldingFoam-openfoam14-linux64GccDPInt32Opt-20260909.tar.xz",
            "riscv64"
        ));
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
