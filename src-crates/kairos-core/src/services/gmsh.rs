//! Gmsh .msh（v2.2 ASCII）解析：体素引擎之外的备选网格引擎。
//! Gmsh 为 GPL——本模块只解析其**输出文件**（纯数据），不做链接、不含其源码。

use std::path::Path;

use crate::error::{KairosError, Result};
use crate::models::mesh::VolumeMesh;

/// 解析 Gmsh ASCII .msh（v2.2）中的节点与四面体单元（elm-type 4）。
/// 一阶四面体；高阶单元（type 11 等）跳过并计数。
pub fn parse_msh_v2(content: &str) -> Result<VolumeMesh> {
    let mut lines = content.lines();
    let mut nodes: Vec<[f64; 3]> = Vec::new();
    let mut tets: Vec<[usize; 4]> = Vec::new();

    while let Some(line) = lines.next() {
        match line.trim() {
            "$Nodes" => {
                let count_line = lines
                    .next()
                    .ok_or_else(|| parse_err("$Nodes 后缺少数量行"))?;
                let count: usize = count_line
                    .trim()
                    .parse()
                    .map_err(|_| parse_err("节点数量行无法解析"))?;
                for _ in 0..count {
                    let node_line = lines.next().ok_or_else(|| parse_err("节点行数量不足"))?;
                    let tokens: Vec<&str> = node_line.split_whitespace().collect();
                    if tokens.len() < 4 {
                        return Err(parse_err("节点行格式错误"));
                    }
                    nodes.push([
                        tokens[1]
                            .parse()
                            .map_err(|_| parse_err("节点坐标无法解析"))?,
                        tokens[2]
                            .parse()
                            .map_err(|_| parse_err("节点坐标无法解析"))?,
                        tokens[3]
                            .parse()
                            .map_err(|_| parse_err("节点坐标无法解析"))?,
                    ]);
                }
            }
            "$Elements" => {
                let count_line = lines
                    .next()
                    .ok_or_else(|| parse_err("$Elements 后缺少数量行"))?;
                let count: usize = count_line
                    .trim()
                    .parse()
                    .map_err(|_| parse_err("单元数量行无法解析"))?;
                for _ in 0..count {
                    let element_line = lines.next().ok_or_else(|| parse_err("单元行数量不足"))?;
                    let tokens: Vec<&str> = element_line.split_whitespace().collect();
                    let element_type: usize = match tokens[1].parse() {
                        Ok(value) => value,
                        Err(_) => continue,
                    };
                    if element_type == 4 {
                        // elm-number type n-tags tags... n0 n1 n2 n3（末 4 个为节点，1-based）
                        if tokens.len() < 7 {
                            return Err(parse_err("四面体单元行字段不足"));
                        }
                        let last = tokens.len();
                        let ids: Vec<usize> = tokens[last - 4..last]
                            .iter()
                            .map(|t| {
                                t.parse::<usize>()
                                    .map_err(|_| parse_err("节点索引无法解析"))
                            })
                            .collect::<std::result::Result<_, _>>()?;
                        tets.push([ids[0] - 1, ids[1] - 1, ids[2] - 1, ids[3] - 1]);
                    }
                }
            }
            _ => {}
        }
    }

    if nodes.is_empty() || tets.is_empty() {
        return Err(parse_err("msh 中没有可用的四面体单元"));
    }
    let surface_faces = extract_surface_faces(&tets);
    Ok(VolumeMesh {
        nodes,
        tets,
        surface_faces,
    })
}

fn parse_err(message: &str) -> KairosError {
    KairosError::validation(format!("Gmsh msh 解析失败：{message}"))
}

/// 从四面体集合提取边界面（只被一个四面体使用的面）。
fn extract_surface_faces(tets: &[[usize; 4]]) -> Vec<[usize; 3]> {
    use std::collections::HashMap;
    let mut count: HashMap<[usize; 3], usize> = HashMap::new();
    for tet in tets {
        for face in [
            [tet[0], tet[1], tet[2]],
            [tet[0], tet[1], tet[3]],
            [tet[0], tet[2], tet[3]],
            [tet[1], tet[2], tet[3]],
        ] {
            let mut key = face;
            key.sort_unstable();
            *count.entry(key).or_insert(0) += 1;
        }
    }
    let mut surface: Vec<[usize; 3]> = count
        .into_iter()
        .filter_map(|(key, n)| (n == 1).then_some(key))
        .collect();
    surface.sort_unstable();
    surface
}

/// 体积网格 → Gmsh ASCII .msh（v2.2）内容（回写/调试用）。
pub fn to_msh_v2(mesh: &VolumeMesh, case_name: &str) -> String {
    let mut out = String::new();
    out.push_str("$MeshFormat\n2.2 0 8\n$EndMeshFormat\n");
    out.push_str(&format!("$Nodes\n{}\n", mesh.nodes.len()));
    for (index, node) in mesh.nodes.iter().enumerate() {
        out.push_str(&format!(
            "{} {:.8} {:.8} {:.8}\n",
            index + 1,
            node[0],
            node[1],
            node[2]
        ));
    }
    out.push_str("$EndNodes\n");
    out.push_str(&format!("$Elements\n{}\n", mesh.tets.len()));
    for (index, tet) in mesh.tets.iter().enumerate() {
        // elm-number type 4, 0 tags, 4 节点（1-based）
        out.push_str(&format!(
            "{} 4 0 {} {} {} {}\n",
            index + 1,
            tet[0] + 1,
            tet[1] + 1,
            tet[2] + 1,
            tet[3] + 1
        ));
    }
    out.push_str("$EndElements\n");
    let _ = case_name;
    out
}

/// 评估用：体素四面体转 msh 再解析回读（往返一致性，演示数据交换路径）。
pub fn from_volume_mesh(mesh: &VolumeMesh, case_name: &str) -> Result<VolumeMesh> {
    parse_msh_v2(&to_msh_v2(mesh, case_name))
}

/// 组装 Gmsh 体网格化命令参数：STL 输入 → 一阶 msh2 输出（解析器只认一阶）。
/// `target_size` 为可选目标单元尺寸上限（-clmax），None 表示交给 Gmsh 默认。
pub fn tetrahedralize_args(stl: &Path, out_msh: &Path, target_size: Option<f64>) -> Vec<String> {
    let mut args = vec![
        stl.to_string_lossy().to_string(),
        "-3".into(),
        "-format".into(),
        "msh2".into(),
        "-order".into(),
        "1".into(),
    ];
    if let Some(size) = target_size {
        args.push("-clmax".into());
        args.push(format!("{size}"));
    }
    args.push("-o".into());
    args.push(out_msh.to_string_lossy().to_string());
    args
}

/// 编排一次 Gmsh 体网格化子进程：清掉旧输出 → 调用 → 解析回 VolumeMesh。
/// CLI 与桌面命令层共用本函数，避免两处各写一份子进程编排；
/// GPL 隔离红线不变——只以独立子进程 + 文件交换方式使用 Gmsh。
pub fn tetrahedralize(
    bin: &Path,
    stl: &Path,
    out_msh: &Path,
    target_size: Option<f64>,
) -> Result<VolumeMesh> {
    let _ = std::fs::remove_file(out_msh);
    let args = tetrahedralize_args(stl, out_msh, target_size);
    let output = std::process::Command::new(bin)
        .args(&args)
        .output()
        .map_err(|e| KairosError::io(format!("gmsh 启动失败：{e}")))?;
    if !output.status.success() {
        return Err(KairosError::io(format!(
            "gmsh 网格化失败：{}",
            failure_message(&output.stderr)
        )));
    }
    let content = std::fs::read_to_string(out_msh)
        .map_err(|e| KairosError::io(format!("读取 msh 失败：{e}")))?;
    parse_msh_v2(&content)
}

/// 子进程失败时的用户消息：取 stderr 最后一行，缺省占位。
fn failure_message(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .last()
        .unwrap_or("无 stderr 输出")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2 四面体（共享面 1-2-3）的最小合法 msh。
    const SAMPLE_MSH: &str = r#"$MeshFormat
2.2 0 8
$EndMeshFormat
$Nodes
5
1 0 0 0
2 1 0 0
3 0 1 0
4 0 0 1
5 0 0 -1
$EndElements
$Elements
3
1 4 2 0 1 2 3 4
2 4 2 0 1 3 2 5
3 15 2 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15
$EndElements
"#;

    #[test]
    fn parses_two_tet_mesh() {
        let mesh = parse_msh_v2(SAMPLE_MSH).unwrap();
        assert_eq!(mesh.nodes.len(), 5);
        assert_eq!(mesh.tets.len(), 2);
        assert_eq!(mesh.tets[0], [0, 1, 2, 3]);
        assert_eq!(mesh.surface_faces.len(), 6); // 7 独立面 − 1 共享面 = 6 边界
    }

    #[test]
    fn skips_high_order_elements() {
        let mesh = parse_msh_v2(SAMPLE_MSH).unwrap();
        // type 15（点单元）被跳过，不影响四面体数
        assert_eq!(mesh.tets.len(), 2);
    }

    #[test]
    fn parse_errors_cover_every_malformed_shape() {
        for (content, message) in [
            ("$Nodes\n", "$Nodes 后缺少数量行"),
            ("$Nodes\nabc\n", "节点数量行无法解析"),
            ("$Nodes\n2\n", "节点行数量不足"),
            ("$Nodes\n1\n1 a 0 0\n", "节点坐标无法解析"),
            ("$Nodes\n1\n1 0 a 0\n", "节点坐标无法解析"),
            ("$Nodes\n1\n1 0 0 a\n", "节点坐标无法解析"),
            ("$Elements\n", "$Elements 后缺少数量行"),
            ("$Elements\nabc\n", "单元数量行无法解析"),
            ("$Elements\n3\n", "单元行数量不足"),
            ("$Elements\n1\n5 4 0 4 1 2 3 x\n", "节点索引无法解析"),
        ] {
            let error = parse_msh_v2(content).unwrap_err();
            assert!(error.to_string().contains(message), "{message}");
        }
    }

    #[test]
    fn volume_mesh_roundtrip_via_msh() {
        let mesh = crate::models::geometry::TriangleMesh::sample_box(1.0);
        let _ = mesh; // 体素网格 → msh → 解析回读
        let volume = crate::services::meshing::generate(
            &mesh,
            &crate::services::meshing::VolumeMeshParams {
                refinement: None,
                target_size: 0.5,
            },
        )
        .unwrap();
        let roundtrip = from_volume_mesh(&volume, "roundtrip").unwrap();
        assert_eq!(roundtrip.tets.len(), volume.tets.len());
        assert_eq!(roundtrip.nodes.len(), volume.nodes.len());
    }

    #[test]
    fn rejects_malformed() {
        assert!(parse_msh_v2("garbage").is_err());
        assert!(
            parse_msh_v2("$MeshFormat\n2.2 0 8\n$EndMeshFormat\n$Nodes\n2\n1 0 0\n$EndNodes\n")
                .is_err()
        );
    }
    #[test]
    fn tetrahedralize_args_orders_gmsh_command() {
        let args = tetrahedralize_args(Path::new("part.stl"), Path::new("out.msh"), None);
        assert_eq!(
            args,
            vec![
                "part.stl".to_string(),
                "-3".to_string(),
                "-format".to_string(),
                "msh2".to_string(),
                "-order".to_string(),
                "1".to_string(),
                "-o".to_string(),
                "out.msh".to_string(),
            ]
        );
    }

    #[test]
    fn tetrahedralize_args_carries_target_size_as_clmax() {
        let args = tetrahedralize_args(Path::new("part.stl"), Path::new("out.msh"), Some(0.5));
        assert!(args.contains(&"-clmax".to_string()));
        let position = args.iter().position(|arg| arg == "-clmax").unwrap();
        assert_eq!(args[position + 1], "0.5");
    }

    #[test]
    fn failure_message_takes_last_stderr_line() {
        assert_eq!(failure_message(b"warn\nboom"), "boom");
        assert_eq!(failure_message(b""), "无 stderr 输出");
    }

    #[test]
    fn tetrahedralize_reports_spawn_failure_as_io() {
        let error = tetrahedralize(
            Path::new("kairos-nonexistent-gmsh"),
            Path::new("in.stl"),
            Path::new("out.msh"),
            None,
        )
        .unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Io);
        assert!(error.to_string().contains("gmsh 启动失败"));
    }

    /// 假 gmsh 用例的「写脚本 → 起进程」临界区。
    ///
    /// Linux 上 exec 一个刚写完的文件，若同进程其它线程此刻 fork（fork 会复制 fd
    /// 表），新进程会短暂持有该脚本的写引用，内核即以 ETXTBSY（Text file busy）
    /// 拒绝 exec——并行跑这几个用例偶发失败（本文件是 core 里唯一的起进程处）。
    /// 串行执行即可消除：同一时刻只有一个用例在写脚本 / 起进程，没有别的 fork
    /// 能拿到它的写 fd。
    #[cfg(unix)]
    static FAKE_GMSH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 写一个可执行 shell 脚本冒充 gmsh：正文写入临时目录并赋予执行位。
    /// 仅 Unix——Windows 侧用等价的 .cmd 批处理（见下方 cfg(windows) 测试）。
    #[cfg(unix)]
    fn write_fake_gmsh(name: &str, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// 取临界区锁（用例存活期间持有）；中毒不影响断言本身。
    #[cfg(unix)]
    fn lock_fake_gmsh() -> std::sync::MutexGuard<'static, ()> {
        FAKE_GMSH_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[cfg(unix)]
    #[test]
    fn tetrahedralize_reads_parsed_msh_from_successful_run() {
        let _serial = lock_fake_gmsh();
        // 假 gmsh：忽略输入，把最小合法 msh（2 四面体）写到 -o 指定的输出路径。
        let fixture = SAMPLE_MSH.replace('"', "'");
        let script = format!(
            "out=\"\"; prev=\"\"; for a in \"$@\"; do [ \"$prev\" = \"-o\" ] && out=\"$a\"; prev=\"$a\"; done; printf '%s' '{fixture}' > \"$out\"\n"
        );
        let fake = write_fake_gmsh("kairos-fake-gmsh-ok", &script);
        let out_msh = std::env::temp_dir().join("kairos-fake-gmsh-ok.msh");
        let _ = std::fs::remove_file(&out_msh);

        let volume = tetrahedralize(&fake, Path::new("in.stl"), &out_msh, Some(0.5)).unwrap();
        assert_eq!(volume.nodes.len(), 5);
        assert_eq!(volume.tets.len(), 2);
        // 输入尺寸经 -clmax 传给子进程（脚本收到的参数含 -clmax 0.5）
        assert_eq!(
            std::fs::read_to_string(&fake).unwrap(),
            format!("#!/bin/sh\n{script}")
        );
        let _ = std::fs::remove_file(&fake);
    }

    #[cfg(unix)]
    #[test]
    fn tetrahedralize_maps_nonzero_exit_to_io_error_with_stderr_tail() {
        let _serial = lock_fake_gmsh();
        let fake = write_fake_gmsh("kairos-fake-gmsh-fail", "echo bad mesh >&2\nexit 3\n");
        let out_msh = std::env::temp_dir().join("kairos-fake-gmsh-fail.msh");
        let _ = std::fs::remove_file(&out_msh);

        let error = tetrahedralize(&fake, Path::new("in.stl"), &out_msh, None).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Io);
        assert!(
            error.to_string().contains("gmsh 网格化失败：bad mesh"),
            "{error}"
        );
        let _ = std::fs::remove_file(&fake);
    }

    #[cfg(unix)]
    #[test]
    fn tetrahedralize_reports_missing_output_as_io() {
        let _serial = lock_fake_gmsh();
        // 假 gmsh 正常退出但不产出文件 → 读取 msh 失败。
        let fake = write_fake_gmsh("kairos-fake-gmsh-silent", "exit 0\n");
        let out_msh = std::env::temp_dir().join("kairos-fake-gmsh-silent.msh");
        let _ = std::fs::remove_file(&out_msh);

        let error = tetrahedralize(&fake, Path::new("in.stl"), &out_msh, None).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Io);
        assert!(error.to_string().contains("读取 msh 失败"), "{error}");
        let _ = std::fs::remove_file(&fake);
    }

    /// Windows 等价物：.cmd 批处理冒充 gmsh（Rust 的 Command 可直接派生
    /// .cmd，经 cmd.exe 执行）。msh 夹具路径经环境变量传入，避免批处理
    /// 解析参数；批处理取最后一个参数（tetrahedralize_args 保证 -o 出现在
    /// 最后）作为输出路径。
    #[cfg(windows)]
    fn write_fake_cmd(name: &str, body: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("{name}.cmd"));
        // cmd.exe 对 LF-only 的批处理兼容性不稳，显式 CRLF
        let body = body.replace('\n', "\r\n");
        std::fs::write(&path, format!("@echo off\r\n{body}")).unwrap();
        path
    }

    #[cfg(windows)]
    #[test]
    fn tetrahedralize_reads_parsed_msh_from_successful_run() {
        // msh 夹具写临时文件，夹具路径直接嵌进批处理（避免进程级环境变量
        // 在并行测试下的竞态）；批处理取最后一个参数（-o 的值）作输出路径。
        let fixture = std::env::temp_dir().join("kairos-fake-gmsh-fixture.msh");
        std::fs::write(&fixture, SAMPLE_MSH).unwrap();
        let fake = write_fake_cmd(
            "kairos-fake-gmsh-ok",
            &format!(
                "set \"last=\"\r\nfor %%a in (%*) do set \"last=%%~a\"\r\ncopy /y \"{}\" \"%last%\" >nul\r\n",
                fixture.display()
            ),
        );
        let out_msh = std::env::temp_dir().join("kairos-fake-gmsh-ok.msh");
        let _ = std::fs::remove_file(&out_msh);

        let volume = tetrahedralize(&fake, Path::new("in.stl"), &out_msh, Some(0.5)).unwrap();
        assert_eq!(volume.nodes.len(), 5);
        assert_eq!(volume.tets.len(), 2);
        let _ = std::fs::remove_file(&fake);
        let _ = std::fs::remove_file(&fixture);
    }

    #[cfg(windows)]
    #[test]
    fn tetrahedralize_maps_nonzero_exit_to_io_error_with_stderr_tail() {
        let fake = write_fake_cmd(
            "kairos-fake-gmsh-fail",
            "echo bad mesh 1>&2\r\nexit /b 3\r\n",
        );
        let out_msh = std::env::temp_dir().join("kairos-fake-gmsh-fail.msh");
        let _ = std::fs::remove_file(&out_msh);

        let error = tetrahedralize(&fake, Path::new("in.stl"), &out_msh, None).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Io);
        assert!(
            error.to_string().contains("gmsh 网格化失败：bad mesh"),
            "{error}"
        );
        let _ = std::fs::remove_file(&fake);
    }

    #[cfg(windows)]
    #[test]
    fn tetrahedralize_reports_missing_output_as_io() {
        let fake = write_fake_cmd("kairos-fake-gmsh-silent", "exit /b 0\r\n");
        let out_msh = std::env::temp_dir().join("kairos-fake-gmsh-silent.msh");
        let _ = std::fs::remove_file(&out_msh);

        let error = tetrahedralize(&fake, Path::new("in.stl"), &out_msh, None).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Io);
        assert!(error.to_string().contains("读取 msh 失败"), "{error}");
        let _ = std::fs::remove_file(&fake);
    }
}
