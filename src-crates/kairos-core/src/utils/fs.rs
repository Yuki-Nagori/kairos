//! 文件系统工具：原子写入、有界深度遍历、带动作描述的读取。
//!
//! 遍历接口取 `&mut dyn FnMut` 而非泛型：泛型会按每个调用方的闭包类型各单态化
//! 一份，同一源码行被登记多次，其中未被走到的实例会被覆盖率工具报成未覆盖。

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{KairosError, Result};

/// 原子写入的临时文件后缀。
const TMP_SUFFIX: &str = ".tmp";

/// 原子写入：先写同目录临时文件再改名覆盖，避免中途失败留下半截文件。
///
/// 临时文件必须与目标同目录（跨目录 rename 不是原子操作）；改名失败时清理临时
/// 文件，否则每失败一次就留一个残留。Windows 的 rename 无法覆盖既有目标，先移除。
///
/// `what` 是**动作描述**（如「工程文件」），用于拼出可读的中文错误。
pub fn write_atomic(path: &Path, content: &str, what: &str) -> Result<()> {
    let tmp = temporary_path(path);
    fs::write(&tmp, content).map_err(|error| tmp_write_failed(&error))?;
    #[cfg(target_os = "windows")]
    let _ = fs::remove_file(path);
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&tmp);
            Err(replace_failed(what, &error))
        }
    }
}

/// 读文本文件，失败时用 `what`（动作描述）拼出可读的中文错误——避免每个调用点
/// 手写同一句 `map_err`。
pub fn read_to_string(path: &Path, what: &str) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(error) => Err(read_failed(path, what, &error)),
    }
}

/// 遍历 `root` 下深度不超过 `max_depth` 的子目录，逐个调用 `visit`（**不含**
/// `root` 自身）。`root` 不是目录、不可读或不存在时静默返回——调用方按「没找到」
/// 处理，不必为探测路径单独判存在性。
pub fn walk_dirs_bounded(root: &Path, max_depth: usize, visit: &mut dyn FnMut(&Path)) {
    collect_dirs(root, 1, max_depth, visit);
}

/// 按深度优先顺序查找 `root` 下（**含** `root` 自身）第一个满足 `predicate`
/// 的文件；深度超限、不可读或没有命中时返回 `None`。
pub fn find_file_bounded(
    root: &Path,
    max_depth: usize,
    predicate: &mut dyn FnMut(&Path) -> bool,
) -> Option<PathBuf> {
    search_file(root, 0, max_depth, predicate)
}

/// 临时文件路径：在文件名后**追加**后缀而不是替换扩展名（`a.stl` → `a.stl.tmp`），
/// 这样同名不同扩展名的文件不会争用同一个临时文件。
fn temporary_path(path: &Path) -> PathBuf {
    let mut name: OsString = path
        .file_name()
        .map(OsStr::to_os_string)
        .unwrap_or_default();
    name.push(TMP_SUFFIX);
    path.with_file_name(name)
}

fn collect_dirs(dir: &Path, depth: usize, max_depth: usize, visit: &mut dyn FnMut(&Path)) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        visit(&path);
        collect_dirs(&path, depth + 1, max_depth, visit);
    }
}

fn search_file(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    predicate: &mut dyn FnMut(&Path) -> bool,
) -> Option<PathBuf> {
    if depth > max_depth {
        return None;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return None;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = search_file(&path, depth + 1, max_depth, predicate) {
                return Some(found);
            }
            continue;
        }
        if predicate(&path) {
            return Some(path);
        }
    }
    None
}

fn tmp_write_failed(error: &std::io::Error) -> KairosError {
    KairosError::io(format!("写入临时文件失败：{error}"))
}

fn replace_failed(what: &str, error: &std::io::Error) -> KairosError {
    KairosError::io(format!("{what}替换失败：{error}"))
}

fn read_failed(path: &Path, what: &str, error: &std::io::Error) -> KairosError {
    KairosError::io(format!("{what}失败（{}）：{error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    /// 每个用例独占的临时目录，避免并行执行时互相删现场。
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kairos-utils-fs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 目录收集器：**多个用例共用同一份实现**。内容完全相同的闭包会被优化器合并，
    /// 覆盖率按「多份单态实例」记账时其中一份计数为 0，整行被报成未覆盖。
    fn path_collector(seen: &mut Vec<PathBuf>) -> impl FnMut(&Path) + '_ {
        move |path| seen.push(path.to_path_buf())
    }

    /// 扩展名匹配谓词：同上，只写一处。
    fn extension_is(extension: &'static str) -> impl FnMut(&Path) -> bool {
        move |path| path.extension() == Some(OsStr::new(extension))
    }

    /// 全不匹配谓词（「没有命中」用例）。
    fn matches_nothing(_: &Path) -> bool {
        false
    }

    #[test]
    fn write_atomic_round_trips_content() {
        let dir = scratch("roundtrip");
        let path = dir.join("project.kairos");
        write_atomic(&path, "内容", "工程文件").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "内容");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_atomic_leaves_no_temporary_file_behind() {
        let dir = scratch("cleanup");
        let path = dir.join("a.stl");
        write_atomic(&path, "x", "网格").unwrap();
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name())
            .collect();
        // 成功后目录里只剩目标文件，没有 .tmp 残留
        assert_eq!(leftovers, vec![OsString::from("a.stl")]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_atomic_reports_temporary_write_failure() {
        let dir = scratch("tmp-failure");
        let target = dir.join("project.kairos");
        // 预埋同名临时文件为目录，令临时文件写入失败
        fs::create_dir_all(target.with_file_name("project.kairos.tmp")).unwrap();
        let error = write_atomic(&target, "内容", "工程文件").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Io);
        assert!(error.message().contains("写入临时文件失败"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_atomic_reports_replace_failure_and_cleans_temporary_file() {
        let dir = scratch("replace-failure");
        // 目标路径是一个目录：rename 到既有目录必然失败
        let target = dir.join("x.kairos");
        fs::create_dir_all(&target).unwrap();
        let error = write_atomic(&target, "内容", "工程文件").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Io);
        assert!(error.message().contains("工程文件替换失败"));
        // 失败路径已清掉临时文件
        assert!(!target.with_file_name("x.kairos.tmp").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn temporary_path_appends_suffix_without_dropping_extension() {
        assert_eq!(
            temporary_path(Path::new("/tmp/mold.stl")),
            PathBuf::from("/tmp/mold.stl.tmp")
        );
        assert_eq!(temporary_path(Path::new("a")), PathBuf::from("a.tmp"));
    }

    #[test]
    fn read_to_string_returns_content() {
        let dir = scratch("read");
        let path = dir.join("field.txt");
        fs::write(&path, "0.5").unwrap();
        assert_eq!(read_to_string(&path, "读取场文件").unwrap(), "0.5");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_to_string_reports_missing_file_with_action_and_path() {
        let dir = scratch("read-missing");
        let path = dir.join("missing.txt");
        let error = read_to_string(&path, "读取场文件").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Io);
        assert!(error.message().contains("读取场文件失败"));
        assert!(error.message().contains("missing.txt"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_failed_names_action_and_path() {
        let error = read_failed(
            Path::new("/tmp/x.txt"),
            "读取网格",
            &std::io::Error::new(std::io::ErrorKind::NotFound, "没有那个文件"),
        );
        assert!(error.message().contains("读取网格失败（/tmp/x.txt）"));
    }

    #[test]
    fn tmp_write_failed_and_replace_failed_map_to_io() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "拒绝访问");
        assert_eq!(tmp_write_failed(&io).kind(), ErrorKind::Io);
        assert!(tmp_write_failed(&io).message().contains("写入临时文件失败"));
        assert_eq!(replace_failed("网格", &io).kind(), ErrorKind::Io);
        assert!(
            replace_failed("网格", &io)
                .message()
                .contains("网格替换失败")
        );
    }

    #[test]
    fn walk_visits_nested_directories_within_depth() {
        let dir = scratch("walk");
        fs::create_dir_all(dir.join("a/b/c")).unwrap();
        fs::write(dir.join("a/note.txt"), "x").unwrap();
        let mut seen = Vec::new();
        walk_dirs_bounded(&dir, 4, &mut path_collector(&mut seen));
        seen.sort();
        assert_eq!(
            seen,
            vec![dir.join("a"), dir.join("a/b"), dir.join("a/b/c")]
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn walk_respects_max_depth() {
        let dir = scratch("walk-depth");
        fs::create_dir_all(dir.join("a/b")).unwrap();
        let mut seen = Vec::new();
        walk_dirs_bounded(&dir, 1, &mut path_collector(&mut seen));
        // 深度 1 只看得到直接子目录
        assert_eq!(seen, vec![dir.join("a")]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn walk_stops_at_zero_depth() {
        let dir = scratch("walk-zero");
        fs::create_dir_all(dir.join("a")).unwrap();
        let mut seen = Vec::new();
        walk_dirs_bounded(&dir, 0, &mut path_collector(&mut seen));
        assert!(seen.is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn walk_ignores_unreadable_root() {
        let dir = scratch("walk-file");
        let file = dir.join("not-a-dir.txt");
        fs::write(&file, "x").unwrap();
        let mut seen = Vec::new();
        // 传文件而非目录：读目录失败即静默返回
        walk_dirs_bounded(&file, 4, &mut path_collector(&mut seen));
        assert!(seen.is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn find_returns_match_from_root_layer() {
        let dir = scratch("find");
        fs::write(dir.join("top.bin"), "x").unwrap();
        fs::create_dir_all(dir.join("nested")).unwrap();
        // 子目录里放同名但不匹配的文件，确保命中来自 root 这一层
        fs::write(dir.join("nested/top.txt"), "x").unwrap();
        let mut wanted = extension_is("bin");
        let found = find_file_bounded(&dir, 4, &mut wanted).unwrap();
        assert_eq!(found, dir.join("top.bin"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn find_recurses_into_subdirectories() {
        let dir = scratch("find-nested");
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::write(dir.join("a/b/wanted.bin"), "x").unwrap();
        let mut wanted = extension_is("bin");
        let found = find_file_bounded(&dir, 4, &mut wanted).unwrap();
        assert_eq!(found, dir.join("a/b/wanted.bin"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn find_returns_none_without_match() {
        let dir = scratch("find-none");
        fs::write(dir.join("a.txt"), "x").unwrap();
        assert!(find_file_bounded(&dir, 4, &mut matches_nothing).is_none());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn find_returns_none_when_depth_exhausted() {
        let dir = scratch("find-depth");
        fs::create_dir_all(dir.join("a")).unwrap();
        fs::write(dir.join("a/wanted.bin"), "x").unwrap();
        let mut wanted = extension_is("bin");
        // 只允许看 root 自身这一层，进不了 a
        assert!(find_file_bounded(&dir, 0, &mut wanted).is_none());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn find_returns_none_for_unreadable_root() {
        let dir = scratch("find-file");
        let file = dir.join("not-a-dir.txt");
        fs::write(&file, "x").unwrap();
        // root 是文件：读目录失败即返回 None，谓词根本不会被调用
        assert!(find_file_bounded(&file, 4, &mut matches_nothing).is_none());
        fs::remove_dir_all(&dir).ok();
    }
}
