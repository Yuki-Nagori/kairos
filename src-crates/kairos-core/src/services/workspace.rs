//! 工作区布局：工程自包含目录的路径规则与几何归档（纯函数）。
//!
//! ```text
//! <workspace>/
//!   project.kairos        # 结构 + 相对路径引用
//!   geometry/             # 导入的原始几何（原样拷贝，保留来源文件名）
//!   mesh/<studyId>/       # 体积网格（节点 / 单元 / 表面 + 生成参数）
//!   cases/<studyId>/      # 求解 case（生成物，可重建）
//!   reports/              # 生成的报告
//! ```
//!
//! 工作区根由工程文件位置推导：**位于默认根（用户文档目录下的 `kairos/`）之下、
//! 且不是直接放在根目录里**的工程，其所在目录即工作区；其它位置（散装 `.kairos`）
//! 视为「无工作区」，数据留在应用数据目录（旧行为），保存时可一键转为工程目录。
//!
//! 文档目录由平台 API 解析（macOS / Linux / Windows 与各语言的目录名都不同，
//! 如 `~/Documents`、`~/文档`、`~/Dokumente`），core 只接收解析结果做纯路径运算。
//! 工程文件只存相对路径——绝对路径换机器 / 换盘即失效。

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::services::paths;

/// 默认工作区根：用户文档目录下的 `kairos/`（每个工程一个子目录）。
pub const DEFAULT_ROOT_DIR_NAME: &str = "kairos";
/// 工程文件默认名（用户在新建工程时可改）。
pub const DEFAULT_PROJECT_FILE_NAME: &str = "project.kairos";
/// 工程文件扩展名（`Path::with_extension` 使用，不带点）。
pub const PROJECT_EXTENSION: &str = "kairos";
/// 几何归档文件名清洗失败时的回退名。
const DEFAULT_ARCHIVE_NAME: &str = "geometry.stl";
/// 几何归档子目录。
pub const GEOMETRY_DIR: &str = "geometry";
/// 网格子目录（其下按方案 id 分目录）。
pub const MESH_DIR: &str = "mesh";
/// case 子目录（其下按方案 id 分目录）。
pub const CASES_DIR: &str = "cases";
/// 报告输出子目录。
pub const REPORTS_DIR: &str = "reports";
/// 方案网格文件名（写在工作区 `mesh/<studyId>/` 下）。
pub const MESH_FILE_NAME: &str = "mesh.json";

/// 默认工作区根：`<文档目录>/kairos`。
pub fn default_root(documents_dir: &Path) -> PathBuf {
    documents_dir.join(DEFAULT_ROOT_DIR_NAME)
}

/// 工作区根：工程文件位于 `<文档目录>/kairos/<工程目录>/` 之内时返回该目录，
/// 否则 None（散装工程：直接放在默认根下、放在别处、或只有文件名没有目录）。
pub fn workspace_root(project_file: &Path, documents_dir: &Path) -> Option<PathBuf> {
    let root = default_root(documents_dir);
    let parent = project_file.parent()?;
    if parent.as_os_str().is_empty() || parent == root {
        return None;
    }
    if !parent.starts_with(&root) {
        return None;
    }
    Some(parent.to_path_buf())
}

/// 工程目录：`<文档目录>/kairos/<工程名>`。
/// 名称清洗走 [`paths::sanitize_file_name`]：保留字符替换为下划线，名称里的目录成分
/// 一律丢弃（只取最后一段），空名回退 `kairos`。
pub fn project_dir(documents_dir: &Path, project_name: &str) -> PathBuf {
    default_root(documents_dir).join(sanitize_component(project_name))
}

/// 工程文件路径：`<文档目录>/kairos/<工程名>/<文件名>.kairos`。
/// 用户可能连扩展名一起填——主干由 `Path::file_stem` 取，再补回 `.kairos`。
pub fn project_file_path(documents_dir: &Path, project_name: &str, file_name: &str) -> PathBuf {
    let stem = sanitize_component(&paths::file_stem(file_name, DEFAULT_ROOT_DIR_NAME));
    project_dir(documents_dir, project_name)
        .join(stem)
        .with_extension(PROJECT_EXTENSION)
}

/// 目录 / 文件名清洗：保留字符替换为下划线，空名回退默认目录名。
fn sanitize_component(name: &str) -> String {
    paths::sanitize_file_name(name, DEFAULT_ROOT_DIR_NAME)
}

/// `<root>/geometry`。
pub fn geometry_dir(root: &Path) -> PathBuf {
    root.join(GEOMETRY_DIR)
}

/// `<root>/mesh/<studyId>`。
pub fn mesh_dir(root: &Path, study_id: &str) -> PathBuf {
    root.join(MESH_DIR).join(study_id)
}

/// `<root>/cases/<studyId>`。
pub fn cases_dir(root: &Path, study_id: &str) -> PathBuf {
    root.join(CASES_DIR).join(study_id)
}

/// `<root>/reports`。
pub fn reports_dir(root: &Path) -> PathBuf {
    root.join(REPORTS_DIR)
}

/// 相对路径安全校验（工程文件可能被手工编辑）：委托 [`paths::validate_relative`]。
pub fn validate_relative(relative: &str) -> Result<()> {
    paths::validate_relative(relative)
}

/// 拼接工作区内的绝对路径（先做相对路径校验）。
pub fn resolve(root: &Path, relative: &str) -> Result<PathBuf> {
    Ok(root.join(paths::from_storage(relative)?))
}

/// 归档文件名：优先保留来源文件名；同名已被占用时加 `<id>-` 前缀避免覆盖。
/// `taken` 为工作区内已存在的文件名集合（小写归一比较，兼容大小写不敏感文件系统）。
pub fn archive_file_name(source_name: &str, id: &str, taken: &[String]) -> String {
    let sanitized = paths::sanitize_file_name(source_name, DEFAULT_ARCHIVE_NAME);
    let lower = sanitized.to_lowercase();
    if !taken.iter().any(|name| name.to_lowercase() == lower) {
        return sanitized;
    }
    format!("{id}-{sanitized}")
}

/// 几何归档的相对路径：`geometry/<文件名>`（存储形态固定 `/`）。
pub fn geometry_relative(file_name: &str) -> String {
    paths::to_storage(&paths::join(&[GEOMETRY_DIR, file_name]))
}

/// 方案网格文件的相对路径：`mesh/<studyId>/mesh.json`（存储形态固定 `/`）。
pub fn mesh_relative(study_id: &str) -> String {
    paths::to_storage(&paths::join(&[MESH_DIR, study_id, MESH_FILE_NAME]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_root_requires_default_root_subdirectory() {
        let documents = Path::new("/home/u/Documents");
        // 默认根下的工程目录 → 工作区
        assert_eq!(
            workspace_root(
                Path::new("/home/u/Documents/kairos/part/part.kairos"),
                documents
            ),
            Some(PathBuf::from("/home/u/Documents/kairos/part"))
        );
        // 文件名任意（用户在新建时可改），只看位置
        assert_eq!(
            workspace_root(
                Path::new("/home/u/Documents/kairos/part/mold.kairos"),
                documents
            ),
            Some(PathBuf::from("/home/u/Documents/kairos/part"))
        );
        // 直接放在默认根下：不是工程目录（避免把数据写进共享的 kairos/）
        assert_eq!(
            workspace_root(Path::new("/home/u/Documents/kairos/part.kairos"), documents),
            None
        );
        // 默认根之外（散装 / 桌面 / 其它盘）
        assert_eq!(
            workspace_root(Path::new("/home/u/Desktop/demo.kairos"), documents),
            None
        );
        assert_eq!(workspace_root(Path::new("demo.kairos"), documents), None);
    }

    #[test]
    fn project_paths_default_under_documents_kairos() {
        let documents = Path::new("/home/u/Documents");
        assert_eq!(
            default_root(documents),
            PathBuf::from("/home/u/Documents/kairos")
        );
        assert_eq!(
            project_dir(documents, "控制器支架"),
            PathBuf::from("/home/u/Documents/kairos/控制器支架")
        );
        assert_eq!(
            project_file_path(documents, "控制器支架", "支架分析.kairos"),
            PathBuf::from("/home/u/Documents/kairos/控制器支架/支架分析.kairos")
        );
        // 用户连扩展名一起填 / 填了非法字符 / 留空
        assert_eq!(
            project_file_path(documents, "p", "mold.kairos"),
            PathBuf::from("/home/u/Documents/kairos/p/mold.kairos")
        );
        assert_eq!(
            project_file_path(documents, "a/b", "x:y"),
            PathBuf::from("/home/u/Documents/kairos/b/x_y.kairos")
        );
        assert_eq!(
            project_file_path(documents, "  ", "  "),
            PathBuf::from("/home/u/Documents/kairos/kairos/kairos.kairos")
        );
    }

    #[test]
    fn standard_directories_follow_layout() {
        let root = Path::new("/data/part");
        assert_eq!(geometry_dir(root), Path::new("/data/part/geometry"));
        assert_eq!(mesh_dir(root, "s-1"), Path::new("/data/part/mesh/s-1"));
        assert_eq!(cases_dir(root, "s-1"), Path::new("/data/part/cases/s-1"));
        assert_eq!(reports_dir(root), Path::new("/data/part/reports"));
        assert_eq!(geometry_relative("part.stl"), "geometry/part.stl");
        assert_eq!(mesh_relative("s-1"), "mesh/s-1/mesh.json");
    }

    #[test]
    fn relative_paths_reject_escape_and_absolute() {
        assert!(validate_relative("geometry/part.stl").is_ok());
        assert!(validate_relative("").is_err());
        assert!(validate_relative("   ").is_err());
        assert!(validate_relative("/etc/passwd").is_err());
        // 绝对路径在 is_absolute 之前就被组件级校验拦下（含 Windows 盘符形态）
        assert!(
            validate_relative("C:/Users/x/part.stl")
                .unwrap_err()
                .message()
                .contains("不得含盘符")
        );
        assert!(validate_relative("geometry/a:b.stl").is_err());
        assert!(validate_relative("geometry/../../etc/passwd").is_err());
        assert!(validate_relative("../outside.stl").is_err());
        // 拼接结果始终落在工作区内
        let resolved = resolve(Path::new("/data/part"), "geometry/part.stl").unwrap();
        assert_eq!(resolved, Path::new("/data/part/geometry/part.stl"));
        assert!(resolve(Path::new("/data/part"), "../x").is_err());
    }

    #[test]
    fn archive_name_keeps_source_and_dedupes_by_id() {
        assert_eq!(archive_file_name("part.stl", "geom-1", &[]), "part.stl");
        // 同名已被占用 → 加 id 前缀
        assert_eq!(
            archive_file_name("part.stl", "geom-1", &["part.stl".to_string()]),
            "geom-1-part.stl"
        );
        // 大小写不敏感比较（macOS / Windows 文件系统）
        assert_eq!(
            archive_file_name("Part.STL", "geom-2", &["part.stl".to_string()]),
            "geom-2-Part.STL"
        );
        // 路径分隔与非法字符清洗
        assert_eq!(archive_file_name("a/b:c?.stl", "geom-3", &[]), "b_c_.stl");
        assert_eq!(archive_file_name("   ", "geom-4", &[]), "geometry.stl");
        assert_eq!(archive_file_name(".", "geom-5", &[]), "geometry.stl");
    }
}
