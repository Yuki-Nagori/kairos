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
pub fn tetrahedralize_args(stl: &Path, out_msh: &Path) -> Vec<String> {
    vec![
        stl.to_string_lossy().to_string(),
        "-3".into(),
        "-format".into(),
        "msh2".into(),
        "-order".into(),
        "1".into(),
        "-o".into(),
        out_msh.to_string_lossy().to_string(),
    ]
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
            &crate::services::meshing::VolumeMeshParams { target_size: 0.5 },
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
        let args = tetrahedralize_args(Path::new("part.stl"), Path::new("out.msh"));
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
}
