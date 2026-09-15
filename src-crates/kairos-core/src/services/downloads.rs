//! 组件下载的**纯规则**：官方源白名单与落盘文件名推导。
//!
//! 规则住 core 的理由：白名单是安全边界、文件名推导编码了 codeload / release 直链的
//! 形态知识，两者都属「容易写错且值得逐例锁定」的逻辑，放这里才能被单测覆盖。
//! 网络取数、流式落盘、解压起进程仍在适配层（那里无覆盖率门槛，也不该把 HTTP
//! 客户端引进 core）。

use crate::error::{KairosError, Result};

/// 允许下载的官方源前缀白名单（防任意 URL 下载）。
pub const ALLOWED_PREFIXES: &[&str] = &[
    "https://gmsh.info/",
    "https://github.com/Yuki-Nagori/moldingFoam/releases/",
];

/// 校验下载源在受信前缀内；否则报 `validation`。
pub fn ensure_allowed_source(url: &str) -> Result<()> {
    if ALLOWED_PREFIXES
        .iter()
        .any(|prefix| url.starts_with(prefix))
    {
        return Ok(());
    }
    Err(KairosError::validation(format!(
        "下载源不在白名单内：{url}"
    )))
}

/// 从 URL 推导落盘文件名。
///
/// 规则：URL 末段是分支名形态（`master.tar.gz` / `master` 等，codeload 直链的典型
/// 样子，落盘完全没法用）时改用「组件 id + 扩展名」；其余保留官方原始文件名。
/// 仍剥离 query/hash 并拒绝相对路径段。
pub fn derive_file_name(component_id: &str, url: &str) -> String {
    let last_segment = url.rsplit('/').next().unwrap_or_default();
    let stem = last_segment.split(['?', '#']).next().unwrap_or_default();
    let branch_like = stem.is_empty()
        || stem == "."
        || stem == ".."
        || stem.starts_with("master")
        || stem.starts_with("main")
        || !stem.contains('.');
    if !branch_like {
        return stem.to_string();
    }
    // 扩展名判定顺序：.tar.gz / .tgz 双段扩展优先于最后一个点（否则
    // master.tar.gz 会被截成 .gz）；相对路径段（. / ..）视为无扩展名。
    let ext = if stem == "." || stem == ".." {
        String::new()
    } else if stem.ends_with(".tar.gz") || url.contains(".tar.gz") || url.contains("/tar.gz/") {
        ".tar.gz".to_string()
    } else if stem.ends_with(".tgz") || url.contains(".tgz") {
        ".tgz".to_string()
    } else if let Some(dot) = stem.rfind('.') {
        stem[dot..].to_string()
    } else if url.contains(".zip") || url.contains("/zip/") {
        ".zip".to_string()
    } else {
        String::new()
    };
    format!("{component_id}{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    #[test]
    fn allowed_source_is_accepted() {
        assert!(ensure_allowed_source("https://gmsh.info/bin/Linux/gmsh.zip").is_ok());
        assert!(
            ensure_allowed_source(
                "https://github.com/Yuki-Nagori/moldingFoam/releases/download/v0.1.1/x.tar.xz"
            )
            .is_ok()
        );
    }

    #[test]
    fn other_sources_are_rejected() {
        for url in [
            "https://evil.example.com/gmsh.zip",
            // 前缀必须是完整来源，不能靠子串蒙混
            "http://gmsh.info/bin/Linux/gmsh.zip",
            "https://gmsh.info.evil.example.com/x.zip",
            "https://github.com/other/repo/releases/download/v1/x.tar.gz",
            "",
        ] {
            let error = ensure_allowed_source(url).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Validation, "url: {url}");
            assert!(error.message().contains("不在白名单内"), "url: {url}");
        }
    }

    #[test]
    fn derive_file_name_covers_branch_rules() {
        let cases = [
            // release 资产：官方原始文件名保留（.tar.xz 双段扩展不受最后一个点影响）
            (
                "moldingfoam",
                "https://github.com/Yuki-Nagori/moldingFoam/releases/download/v0.1.1/moldingFoam-openfoam14-linuxArm64GccDPInt32Opt-20260909.tar.xz",
                "moldingFoam-openfoam14-linuxArm64GccDPInt32Opt-20260909.tar.xz",
            ),
            // 分支归档形态 → 组件 id + 扩展名（.tar.gz 双段扩展优先于最后一个点）
            (
                "gmsh",
                "https://github.com/example/example/archive/refs/heads/master.tar.gz",
                "gmsh.tar.gz",
            ),
            // 官方原始文件名保留
            (
                "gmsh",
                "https://gmsh.info/bin/macOS/gmsh-4.15.2-MacOSARM-sdk.tgz",
                "gmsh-4.15.2-MacOSARM-sdk.tgz",
            ),
            // 剥离 query；拒绝空段与相对路径段
            (
                "gmsh",
                "https://gmsh.info/bin/Linux/gmsh.zip?query=1#hash",
                "gmsh.zip",
            ),
            ("solver", "https://openfoam.org/master", "solver"),
            ("solver", "https://openfoam.org/..", "solver"),
            ("solver", "https://openfoam.org/", "solver"),
            // 分支归档但仍带扩展名（codeload 的 zip / tgz 形态）：按实际扩展名
            (
                "gmsh",
                "https://github.com/example/example/archive/refs/heads/main.zip",
                "gmsh.zip",
            ),
            (
                "solver",
                "https://gmsh.info/archive/master.tgz",
                "solver.tgz",
            ),
        ];
        for (id, url, expected) in cases {
            assert_eq!(derive_file_name(id, url), expected, "url: {url}");
        }
    }

    #[test]
    fn derive_file_name_keeps_extension_for_zip_archives() {
        // 末段没有扩展名但 URL 里出现归档标记：按标记补扩展名
        assert_eq!(
            derive_file_name("gmsh", "https://gmsh.info/download/zip/latest"),
            "gmsh.zip"
        );
        assert_eq!(
            derive_file_name("src", "https://gmsh.info/archive/tar.gz/head"),
            "src.tar.gz"
        );
        // 既无扩展名也无归档标记：只用组件 id
        assert_eq!(
            derive_file_name("plain", "https://gmsh.info/latest"),
            "plain"
        );
    }
}
