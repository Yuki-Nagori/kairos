//! 应用内下载：有官方单文件直链的组件（Gmsh 预编译包、OpenFOAM 源码包）
//! 点击下载到受管目录（Kairos 只转发官方直链，不是分发方）；其余组件走
//! 引导安装。存放位置固定为 `<应用数据目录>/downloads/`，面板展示路径并支持打开。

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use kairos_core::error::{KairosError, Result};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

/// 允许下载的官方源前缀白名单（防任意 URL 下载）。
const ALLOWED_PREFIXES: &[&str] = &[
    "https://gmsh.info/",
    "https://github.com/Yuki-Nagori/moldingFoam/releases/",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDownload {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    /// 压缩包自动解压后的目录（非压缩包为 None）。
    pub extract_dir: Option<String>,
}

pub fn downloads_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| KairosError::io(format!("无法定位应用数据目录：{e}")))?;
    Ok(dir.join("downloads"))
}

/// 白名单校验：只允许目录里登记过的官方源。
fn ensure_allowed(url: &str) -> Result<()> {
    if ALLOWED_PREFIXES
        .iter()
        .any(|prefix| url.starts_with(prefix))
    {
        Ok(())
    } else {
        Err(KairosError::validation(format!(
            "下载源不在白名单内：{url}"
        )))
    }
}

/// 从 URL 推导落盘文件名。
///
/// 规则：URL 末段是分支名形态（`master.tar.gz` / `master` 等，codeload 直链
/// 的典型样子，落盘完全没法用）时改用「组件 id + 扩展名」；其余保留官方
/// 原始文件名。仍剥离 query/hash 并拒绝相对路径段。
fn derive_file_name(component_id: &str, url: &str) -> String {
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

/// moldingFoam 仓库：bundle 由其 CI 按 release 发布。
const BUNDLE_REPO: &str = "Yuki-Nagori/moldingFoam";

#[derive(serde::Deserialize)]
struct ReleaseAssets {
    assets: Vec<ReleaseAsset>,
}

#[derive(serde::Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

/// `releases/latest` 形态的 URL 在下载时动态解析为具体资产：资产名含日期，
/// 固定文件名的 latest/download 直链会随下个版本失效。按宿主架构挑资产，
/// 找不到匹配时明确报错而不是猜。其余 URL 原样返回。
fn resolve_release_asset(url: &str) -> Result<String> {
    if !url.ends_with("/releases/latest") {
        return Ok(url.to_string());
    }
    let agent: ureq::Agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("kairos-dependency-manager/1.0")
        .build();
    let response = agent
        .get(&format!(
            "https://api.github.com/repos/{BUNDLE_REPO}/releases/latest"
        ))
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| KairosError::io(format!("查询最新 release 失败：{e}")))?;
    let release: ReleaseAssets = serde_json::from_reader(response.into_reader())
        .map_err(|e| KairosError::io(format!("release 数据解析失败：{e}")))?;
    let asset = release
        .assets
        .iter()
        .find(|asset| {
            kairos_core::services::dependencies::bundle_asset_matches_arch(
                &asset.name,
                std::env::consts::ARCH,
            )
        })
        .ok_or_else(|| {
            KairosError::not_found(format!(
                "最新 release 中没有适配 {} 架构的资产",
                std::env::consts::ARCH
            ))
        })?;
    Ok(asset.browser_download_url.clone())
}

/// 下载文件到受管目录：流式写盘并按百分比回传进度（Channel<u64>）；
/// 成功后登记进 manifest.json（跨会话记住「已下载」状态）。
#[tauri::command]
pub async fn download_file(
    app: AppHandle,
    component_id: String,
    url: String,
    progress: Channel<u64>,
) -> Result<SavedDownload> {
    ensure_allowed(&url)?;
    let url = resolve_release_asset(&url)?;
    let file_name = derive_file_name(&component_id, &url);

    tauri::async_runtime::spawn_blocking(move || {
        let dir = downloads_dir(&app)?;
        fs::create_dir_all(&dir)?;
        let dest = dir.join(&file_name);

        // 带超时与 UA 的共享 agent：部分官方站点对无 UA 请求或无限挂起不友好。
        let agent: ureq::Agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(30))
            .timeout(std::time::Duration::from_secs(1800))
            .user_agent("kairos-dependency-manager/1.0")
            .build();
        let response = agent
            .get(&url)
            .call()
            .map_err(|e| KairosError::io(format!("下载请求失败：{e}")))?;
        let total: u64 = response
            .header("Content-Length")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);

        let mut reader = response.into_reader();
        let mut file = fs::File::create(&dest)?;
        let mut buffer = [0u8; 65_536];
        let mut downloaded: u64 = 0;
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])?;
            downloaded += read as u64;
            // checked_div 满足 clippy::manual_checked_ops；不用 let-chain 写法，
            // 兼容尚未支持 let-chains 的 rust-analyzer 版本（total = 0 时为 None 不发送）。
            if let Some(percent) = (downloaded * 100).checked_div(total) {
                let _ = progress.send(percent);
            }
        }
        file.flush()?;
        drop(file);
        // 压缩包自动解压到组件子目录（downloads/<组件 id>/）。
        let extract_dir = extract_if_archive(&dir, &component_id, &dest, &file_name)?;
        let saved = SavedDownload {
            path: dest.to_string_lossy().to_string(),
            file_name: file_name.clone(),
            size_bytes: downloaded,
            extract_dir: extract_dir.map(|p| p.to_string_lossy().to_string()),
        };
        register_in_manifest(&dir, &component_id, &saved)?;
        Ok(saved)
    })
    .await
    .map_err(|e| KairosError::internal(format!("下载任务失败：{e}")))?
}

/// 跨会话的下载清单（manifest.json，key = 组件 id）。
pub type DownloadManifest = std::collections::HashMap<String, ManifestEntry>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub file_name: String,
    pub size_bytes: u64,
    pub downloaded_at_ms: u64,
    pub extract_dir: Option<String>,
}

fn manifest_path(dir: &Path) -> PathBuf {
    dir.join("manifest.json")
}

fn read_manifest(dir: &Path) -> DownloadManifest {
    fs::read_to_string(manifest_path(dir))
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// 成功下载后把条目写进清单（读改写，原子替换）。
fn register_in_manifest(dir: &Path, component_id: &str, saved: &SavedDownload) -> Result<()> {
    let mut manifest = read_manifest(dir);
    manifest.insert(
        component_id.to_string(),
        ManifestEntry {
            file_name: saved.file_name.clone(),
            size_bytes: saved.size_bytes,
            downloaded_at_ms: now_ms(),
            extract_dir: saved.extract_dir.clone(),
        },
    );
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| KairosError::internal(format!("清单序列化失败：{e}")))?;
    let tmp = manifest_path(dir).with_extension("json.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, manifest_path(dir))?;
    Ok(())
}

/// 返回已下载组件清单（前端启动时恢复「已下载」徽标）。
#[tauri::command]
pub fn list_downloads(app: AppHandle) -> Result<DownloadManifest> {
    let dir = downloads_dir(&app)?;
    Ok(read_manifest(&dir))
}

/// 压缩包则解压到 downloads/<组件 id>/；非压缩包返回 None。
fn extract_if_archive(
    dir: &Path,
    component_id: &str,
    archive: &Path,
    file_name: &str,
) -> Result<Option<PathBuf>> {
    let lower = file_name.to_lowercase();
    let is_archive = lower.ends_with(".zip")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.xz");
    if !is_archive {
        return Ok(None);
    }
    let dest = dir.join(component_id);
    extract_archive(archive, &dest)?;
    Ok(Some(dest))
}

/// 调系统 tar/unzip 解压（不引入 Rust 侧压缩依赖；zip 在 Linux 用 unzip，
/// macOS / Windows 的 tar 是 bsdtar，可直接解 zip）。
fn extract_archive(archive: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest)?;
    let name = archive
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let is_zip = name.ends_with(".zip");

    #[cfg(target_os = "windows")]
    let mut command = Command::new("tar"); // Windows 的 tar 是 bsdtar，可直接解 zip
    #[cfg(not(target_os = "windows"))]
    let mut command = Command::new(if is_zip { "unzip" } else { "tar" });

    if is_zip {
        // Linux / macOS：unzip <archive> -d <dest>；
        // Windows：bsdtar -xf <archive> -C <dest>——必须显式给解压模式位，
        // 裸位置参数 bsdtar 只打用法提示不干活（Windows CI 真机踩过）。
        if cfg!(target_os = "windows") {
            command.arg("-xf");
        }
        command.arg(archive);
        command.arg(if cfg!(target_os = "windows") {
            "-C"
        } else {
            "-d"
        });
    } else {
        // GNU tar：-z 是 gzip、-J 是 xz（moldingFoam bundle 为 tar.xz）。
        let compress = if name.ends_with(".tar.xz") {
            "-xJf"
        } else {
            "-xzf"
        };
        command.arg(compress).arg(archive).arg("-C");
    }
    command.arg(dest);
    // 捕获 stderr：解压失败时把工具的最后一条报错带给用户，便于定位。
    let output = command
        .output()
        .map_err(|e| KairosError::io(format!("解压启动失败：{e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let detail = stderr.lines().last().unwrap_or("无 stderr 输出");
        return Err(KairosError::io(format!(
            "压缩包解压失败（原始文件已保留，可手动解压）：{detail}"
        )));
    }
    Ok(())
}

/// 收集受管目录下的 bin 目录（求解器运行 / 网格生成的 PATH 前缀）。
pub fn managed_bin_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(dir) = downloads_dir(app) else {
        return out;
    };
    for component in ["openfoam", "gmsh"] {
        collect_bin_dirs(&dir.join(component), 0, 4, &mut out);
    }
    out
}

fn collect_bin_dirs(dir: &Path, depth: u8, max_depth: u8, out: &mut Vec<PathBuf>) {
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
        if path.file_name().and_then(|n| n.to_str()) == Some("bin") {
            out.push(path.clone());
        }
        collect_bin_dirs(&path, depth + 1, max_depth, out);
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 返回下载目录路径（前端展示「文件存放在哪里」）。
#[tauri::command]
pub fn get_downloads_dir(app: AppHandle) -> Result<String> {
    let dir = downloads_dir(&app)?;
    Ok(dir.to_string_lossy().to_string())
}

/// 在系统文件管理器中打开下载目录。
#[tauri::command]
pub fn open_downloads_dir(app: AppHandle) -> Result<String> {
    let dir = downloads_dir(&app)?;
    fs::create_dir_all(&dir)?;
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&dir)
            .status()
            .map_err(|e| KairosError::io(format!("打开目录失败：{e}")))?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&dir)
            .status()
            .map_err(|e| KairosError::io(format!("打开目录失败：{e}")))?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&dir)
            .status()
            .map_err(|e| KairosError::io(format!("打开目录失败：{e}")))?;
    }
    Ok(dir.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip_and_upsert() {
        let dir = std::env::temp_dir().join(format!("kairos-manifest-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        assert!(read_manifest(&dir).is_empty());

        let saved = SavedDownload {
            path: dir.join("a.tgz").to_string_lossy().to_string(),
            file_name: "a.tgz".into(),
            size_bytes: 123,
            extract_dir: None,
        };
        register_in_manifest(&dir, "gmsh", &saved).unwrap();
        register_in_manifest(&dir, "openfoam", &saved).unwrap();

        let manifest = read_manifest(&dir);
        assert_eq!(manifest.len(), 2);
        assert_eq!(manifest["gmsh"].file_name, "a.tgz");
        assert_eq!(manifest["gmsh"].size_bytes, 123);

        // 同组件重复下载：upsert 不产生重复条目
        register_in_manifest(&dir, "gmsh", &saved).unwrap();
        assert_eq!(read_manifest(&dir).len(), 2);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn derive_file_name_covers_branch_rules() {
        let cases = [
            // release 资产：官方原始文件名保留（.tar.xz 双段扩展不受最后一个点影响）
            (
                "openfoam",
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
            ("openfoam", "https://openfoam.org/master", "openfoam"),
            ("openfoam", "https://openfoam.org/..", "openfoam"),
            ("openfoam", "https://openfoam.org/", "openfoam"),
        ];
        for (id, url, expected) in cases {
            assert_eq!(derive_file_name(id, url), expected, "url: {url}");
        }
    }

    #[test]
    fn extract_archive_handles_zip() {
        let dir = std::env::temp_dir().join(format!("kairos-zip-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("pkg.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file("hello.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"kairos").unwrap();
        writer.finish().unwrap();

        let dest = dir.join("out");
        extract_archive(&zip_path, &dest).unwrap();
        assert_eq!(
            fs::read_to_string(dest.join("hello.txt")).unwrap(),
            "kairos"
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn extract_archive_roundtrip() {
        let dir = std::env::temp_dir().join(format!("kairos-extract-test-{}", std::process::id()));
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("hello.txt"), "kairos").unwrap();

        // 用系统 tar 制作 gzip 包（三平台自带）
        let archive = dir.join("pkg.tar.gz");
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(&src)
            .arg("hello.txt")
            .status()
            .unwrap();
        assert!(status.success());

        let dest = dir.join("out");
        extract_archive(&archive, &dest).unwrap();
        assert_eq!(
            fs::read_to_string(dest.join("hello.txt")).unwrap(),
            "kairos"
        );

        // 重复解压：先清空再解，不残留不报错
        extract_archive(&archive, &dest).unwrap();
        assert!(dest.join("hello.txt").exists());

        fs::remove_dir_all(&dir).unwrap();
    }
}
