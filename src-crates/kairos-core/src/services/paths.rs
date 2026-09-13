//! 路径工具：全仓唯一的路径拼装 / 清洗 / 存储格式转换入口。
//!
//! 约定（凡是路径都走 `std::path`，不手写分隔符）：
//! - **拼装**用 `Path::join` / `PathBuf::push`，本模块不给字符串拼接留口子；
//! - **存储**（工程文件、DTO）里的相对路径统一用 `/`：`Path` 在 Windows 上 join 出 `\`，
//!   直接写进工程文件后换到 Unix 会被当成普通字符而不是分隔符，因此跨机器落盘前
//!   一律经 [`to_storage`] 归一（Windows 能同时识别 `/`，读取侧无需区分平台）；
//! - **名字 ≠ 路径**：用户输入的「工程名 / 文件名」是**单个路径段**，不做路径解析
//!   （[`path_segment`] 直接把保留字符换成下划线）——同一段文本当路径解析会在
//!   Windows 上变味（`x:y` 被当成盘符 + 名字、`C:\a` 被拆开），而 Unix 不会；
//! - **真路径取最后一段**用 [`file_name_of`]（`Path::file_name`，输入是某个平台
//!   产出的真实文件路径）；
//! - 唯一的分隔符处理是 [`normalized`]（把平台不认识的那一种换成 `MAIN_SEPARATOR`），
//!   其余全部交给 `std::path`。
//!
//! 上层（workspace / 命令层）只调这里的函数，平台差异（分隔符、盘符、保留字符）
//! 全部收敛在本文件。

use std::path::{Component, MAIN_SEPARATOR_STR, Path, PathBuf};

use crate::error::{KairosError, Result};

/// 跨平台保留字符（Windows 最严）：文件名 / 目录名里统一替换为下划线。
/// `/`、`\\` 之类的分隔符不出现在 `Path::file_name` 的结果里，这里只列字符集。
const RESERVED_CHARS: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// 分隔符归一（对外版本）：跨平台文本里可能带另一种分隔符，先统一成 `MAIN_SEPARATOR`
/// 再交给 `std::path`。全仓唯一做这件事的地方。
pub fn normalize_separators(text: &str) -> String {
    normalized(text)
}

/// Windows 路径 → WSL（`/mnt` 形态）路径；没有盘符（已是 Unix 形态 / 相对路径）返回 None。
///
/// 盘符判定在组件层做：Windows 上 `Path::components()` 给 `Component::Prefix`，
/// Unix 上则解析成普通组件 `C:`——两种形态都要认，否则同一段逻辑在 macOS 上测不了、
/// 只能靠 Windows CI 才发现问题。
pub fn wsl_path(windows_path: &str) -> Option<String> {
    let normalized = normalized(windows_path);
    // 盘符取「第一个组件」的文本：`Component::as_os_str` 对所有变体都可用，
    // 因此 Unix（`C:` 解析成普通组件）与 Windows（解析成 `Prefix`）走同一条代码路径——
    // 不需要按平台分叉，也就能在任意平台上测到。
    let first = Path::new(&normalized).components().next()?;
    let drive = drive_letter(&first.as_os_str().to_string_lossy())?;
    // 其余组件取「可作文件名的那部分」：根（`/`、`\`）、`..` 之类结构性组件由
    // `Path::file_name()` 判为 None 直接丢弃——不写按平台分叉的 match 分支
    // （Windows 的 `C:\` 会多出一个 RootDir 组件，Unix 解析不出，分叉必然在一边漏测）。
    let rest: Vec<String> = Path::new(&normalized)
        .components()
        .skip(1)
        .filter_map(|component| {
            Path::new(component.as_os_str())
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .collect();
    let mut segments = vec!["mnt".to_string(), drive.to_string()];
    segments.extend(rest);
    let parts: Vec<&str> = segments.iter().map(String::as_str).collect();
    // WSL 侧是 Unix 绝对路径：根 + 归一后的相对段
    Some(format!("/{}", to_storage(&join(&parts))))
}

/// 「盘符」文本判定：恰为 `X:` 形态（`Prefix` 在 Windows 上的 `as_os_str` 也带冒号）
/// → 返回小写字母；其余（普通目录名、空、多字符）都不是盘符。
fn drive_letter(component: &str) -> Option<char> {
    let mut chars = component.chars();
    let letter = chars.next()?;
    if chars.next() != Some(':') || chars.next().is_some() {
        return None;
    }
    letter
        .is_ascii_alphabetic()
        .then(|| letter.to_ascii_lowercase())
}

/// 分隔符归一：把「平台不认识的那个分隔符」换成 `MAIN_SEPARATOR`，交给 `Path` 解析。
///
/// 全仓唯一需要做这一步的地方——`Path` 按设计只认本平台分隔符（Unix 上
/// `C:\models\part.stl` 是一个普通文件名，Windows 上 `a/b` 反而能解析），
/// 而外部字符串（对话框、手工编辑的工程文件、跨平台文本）可能带另一种写法。
/// 归一之后所有解析都交给 `std::path`，本函数不带任何路径语义。
fn normalized(name: &str) -> String {
    // 两种分隔符都换成主分隔符：native 换成自己是幂等操作，因此不需要按平台分叉
    // （分叉会让另一边的分支在本地永远覆盖不到）。
    name.trim().replace(['/', '\\'], MAIN_SEPARATOR_STR)
}

/// 把若干段拼成路径（库函数 `join`，不手写分隔符）。
pub fn join(parts: &[&str]) -> PathBuf {
    parts
        .iter()
        .fold(PathBuf::new(), |path, part| path.join(part))
}

/// 相对路径 → **存储形态**（分隔符归一为 `/`，供工程文件 / DTO 使用）。
/// 非 UTF-8 分量按有损转换（工程内路径来自我们自己生成的名称，不会触发）。
pub fn to_storage(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// 存储形态（`/` 分隔）→ 平台路径；带根 / 盘符 / `..` 一律拒绝。
pub fn from_storage(relative: &str) -> Result<PathBuf> {
    validate_relative(relative)?;
    Ok(PathBuf::from(relative))
}

/// 相对路径安全校验：拒绝绝对路径、盘符 / 根、`..` 与空路径。
///
/// Windows 上 `C:\x` 与 `/etc/passwd`（无盘符的 rooted 路径）`is_absolute()` 都为
/// false 或平台相关，因此这里同时看 `has_root()`；盘符另由冒号显式判定兜住
/// （工程文件可能被手工编辑）。
pub fn validate_relative(relative: &str) -> Result<()> {
    if relative.trim().is_empty() {
        return Err(KairosError::validation("工程内相对路径不能为空。"));
    }
    let normalized = normalized(relative);
    let path = Path::new(&normalized);
    if path.is_absolute() || path.has_root() {
        return Err(KairosError::validation(format!(
            "工程内相对路径必须是相对路径：{relative}"
        )));
    }
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(KairosError::validation(format!(
            "工程内相对路径不得越出工作区：{relative}"
        )));
    }
    // 盘符 / 备用数据流：按组件迭代判定（Unix 解析不出 `Prefix` 组件，
    // 因此不能用 `path.prefix()`），逐组件不让路径语义泄漏到字符串层。
    if path
        .components()
        .any(|component| component.as_os_str().to_string_lossy().contains(':'))
    {
        return Err(KairosError::validation(format!(
            "工程内相对路径不得含盘符或冒号：{relative}"
        )));
    }
    Ok(())
}

/// 单个路径段清洗（**用户输入的名字**，如工程名 / 文件名 / 报告名）：
/// 把保留字符（含分隔符与盘符冒号）换成下划线，空名回退 `fallback`。
///
/// 故意**不**做路径解析：同一段文本当路径解析在 Windows 与 Unix 上结果不同
/// （`x:y` 在 Windows 是「盘符 x + 名字 y」，Unix 只是普通字符），而名字本来
/// 就没有目录语义。
pub fn path_segment(name: &str, fallback: &str) -> String {
    let cleaned = replace_reserved(name);
    if cleaned.trim().trim_matches('.').trim().is_empty() {
        fallback.to_string()
    } else {
        cleaned.trim().trim_matches('.').trim().to_string()
    }
}

/// 真实文件路径 → 最后一段文件名（`Path::file_name`，适合来自文件对话框 / 工作区的路径），
/// 再对该段做保留字符清洗；取不到文件名时回退 `fallback`。
pub fn file_name_of(path: &str, fallback: &str) -> String {
    Path::new(&normalized(path))
        .file_name()
        .map(|file| replace_reserved(&file.to_string_lossy()))
        .filter(|file| !file.trim().trim_matches('.').trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

/// 名字主干（`Path::file_stem`）：`mold.kairos` → `mold`，`a.b.stl` → `a.b`。
/// 先按 [`path_segment`] 清洗成单个路径段再做主干提取——清洗后已无分隔符 / 冒号，
/// `Path` 的语义在各平台一致。主干为空（纯扩展名 / 空白）时回退 `fallback`。
pub fn file_stem(name: &str, fallback: &str) -> String {
    let segment = path_segment(name, fallback);
    // `path_segment` 已保证是「非空且非纯点」的单个路径段，`file_stem` 必有值；
    // 兜底返回该段本身（不留不可达的闭包分支）。
    Path::new(&segment)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or(segment)
}

/// 替换保留字符为下划线（只处理字符集，不解析路径结构）。
fn replace_reserved(name: &str) -> String {
    name.trim()
        .chars()
        .map(|character| {
            if RESERVED_CHARS.contains(&character) {
                '_'
            } else {
                character
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::path::MAIN_SEPARATOR;

    use super::*;

    /// 盘符路径 → WSL `/mnt` 形态：两种分隔符写法都认，无盘符返回 None。
    #[test]
    fn wsl_path_maps_drive_letters() {
        assert_eq!(wsl_path(r"C:\a\b").as_deref(), Some("/mnt/c/a/b"));
        assert_eq!(wsl_path("D:/cases").as_deref(), Some("/mnt/d/cases"));
        assert_eq!(
            wsl_path(r"c:\x y\件.stl").as_deref(),
            Some("/mnt/c/x y/件.stl")
        );
        assert_eq!(wsl_path("/already/unix"), None);
        assert_eq!(wsl_path("relative/dir"), None);
        assert_eq!(wsl_path(""), None);
        assert_eq!(wsl_path("C:"), Some("/mnt/c".to_string()));
        // 单字母目录名不是盘符（判据要求 `X:` 形态）
        assert_eq!(wsl_path("a/b"), None);
        assert_eq!(wsl_path("dir/c"), None);
        // 盘符根目录（`C:/` 与 `C:\` 两种写法）：根组件被丢弃，只留 /mnt/c
        assert_eq!(wsl_path("C:/").as_deref(), Some("/mnt/c"));
        assert_eq!(wsl_path("D:\\").as_deref(), Some("/mnt/d"));
    }

    /// 分隔符归一：另一种写法统一成平台主分隔符（解析交给 std::path）。
    #[test]
    fn separator_normalization_is_platform_stable() {
        let expected = format!("a{0}b{0}c", MAIN_SEPARATOR);
        assert_eq!(normalize_separators("a/b/c"), expected);
        assert_eq!(normalize_separators(r"a\b\c"), expected);
        assert_eq!(
            normalize_separators("  a/b  "),
            format!("a{0}b", MAIN_SEPARATOR)
        );
    }

    #[test]
    fn join_uses_library_and_keeps_components() {
        let path = join(&["mesh", "study-1", "mesh.json"]);
        assert_eq!(
            path,
            PathBuf::from("mesh").join("study-1").join("mesh.json")
        );
        assert_eq!(path.components().count(), 3);
        // 空段等于不追加（`Path::join("")` 语义）
        assert_eq!(
            join(&["cases", "", "s"]),
            PathBuf::from("cases").join("").join("s")
        );
        assert_eq!(join(&[]), PathBuf::new());
    }

    /// 存储形态始终是 `/` 分隔：Windows 上 join 出的 `\` 必须在落盘前归一。
    #[test]
    fn storage_form_is_slash_normalized() {
        let path = join(&["geometry", "part.stl"]);
        assert_eq!(to_storage(&path), "geometry/part.stl");
        assert_eq!(to_storage(&PathBuf::from("mesh").join("s-1")), "mesh/s-1");
        // 反向：`/` 形态在任意平台都能解析回同样的分量
        let parsed = from_storage("mesh/s-1/mesh.json").unwrap();
        assert_eq!(parsed.components().count(), 3);
        assert_eq!(to_storage(&parsed), "mesh/s-1/mesh.json");
    }

    #[test]
    fn relative_paths_reject_absolute_and_escape() {
        assert!(validate_relative("geometry/part.stl").is_ok());
        assert!(from_storage("geometry/part.stl").is_ok());
        assert!(validate_relative("").is_err());
        assert!(validate_relative("   ").is_err());
        // 根路径：Unix 的 `/etc/passwd` 与 Windows 的 `\etc\passwd` 都拒绝
        assert!(validate_relative("/etc/passwd").is_err());
        assert!(validate_relative("\\etc\\passwd").is_err());
        assert!(validate_relative("geometry/../../etc/passwd").is_err());
        assert!(validate_relative("../outside.stl").is_err());
        // 盘符 / 冒号：Windows 形态可能先被 rooted 判定拦下，这里只要求报错；
        // 相对路径里的冒号由显式判定拦下。
        assert!(validate_relative("C:/Users/x/part.stl").is_err());
        assert!(
            validate_relative("geometry/a:b.stl")
                .unwrap_err()
                .message()
                .contains("不得含盘符")
        );
    }

    /// 名字（单个路径段）：保留字符一律替换，**不做路径解析**——`x:y` 在 Windows
    /// 也不能被当成盘符（否则同一输入在两平台结果不同）。
    #[test]
    fn path_segment_replaces_reserved_without_parsing() {
        assert_eq!(path_segment("part.stl", "geometry.stl"), "part.stl");
        assert_eq!(path_segment("支架分析", "kairos"), "支架分析");
        assert_eq!(path_segment("x:y", "kairos"), "x_y");
        assert_eq!(path_segment("a/b", "kairos"), "a_b");
        assert_eq!(path_segment(r"C:\models", "kairos"), "C__models");
        assert_eq!(path_segment("a b?c*d", "kairos"), "a b_c_d");
        assert_eq!(path_segment("   ", "geometry.stl"), "geometry.stl");
        assert_eq!(path_segment(".", "geometry.stl"), "geometry.stl");
        assert_eq!(path_segment("..", "geometry.stl"), "geometry.stl");
    }

    /// 真路径取末段：目录成分被 `Path::file_name` 丢弃，末段再做保留字符替换。
    #[test]
    fn file_name_of_takes_last_component_then_cleans() {
        assert_eq!(file_name_of("/models/part.stl", "geometry.stl"), "part.stl");
        assert_eq!(file_name_of("models/part.stl", "geometry.stl"), "part.stl");
        // 两种分隔符写法都归一到平台主分隔符后再取末段
        assert_eq!(
            file_name_of(r"C:\models\part.stl", "geometry.stl"),
            "part.stl"
        );
        assert_eq!(file_name_of("a/b:c?.stl", "geometry.stl"), "b_c_.stl");
        assert_eq!(file_name_of("/models/", "geometry.stl"), "models");
        assert_eq!(file_name_of("", "geometry.stl"), "geometry.stl");
        assert_eq!(file_name_of("/", "geometry.stl"), "geometry.stl");
    }

    /// 主干取 `Path::file_stem`（多扩展名只去掉最后一段；`kairos` 不需要手工 strip）。
    #[test]
    fn file_stem_follows_path_semantics() {
        assert_eq!(file_stem("mold.kairos", "project"), "mold");
        assert_eq!(file_stem("mold", "project"), "mold");
        assert_eq!(file_stem("a.b.stl", "project"), "a.b");
        assert_eq!(file_stem(" 支架分析.kairos ", "project"), "支架分析");
        // 首尾的点先被 path_segment 去掉（`.kairos` → `kairos`），只剩扩展形态时回退
        assert_eq!(file_stem(".kairos", "project"), "kairos");
        assert_eq!(file_stem("...", "project"), "project");
        assert_eq!(file_stem("   ", "project"), "project");
    }
}
