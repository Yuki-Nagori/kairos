//! 几何服务：STL 解析（ASCII / Binary 自动识别）、单位推断与网格健康检查。

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::{KairosError, Result};
use crate::models::geometry::{GeometrySummary, MeshIssues, Triangle, TriangleMesh};

const BINARY_HEADER: usize = 84;
const BINARY_TRIANGLE: usize = 50;

/// 解析 STL：按「长度匹配二进制 → solid 前缀 ASCII → 二进制兜底」识别格式。
pub fn parse_stl(bytes: &[u8]) -> Result<TriangleMesh> {
    let looks_binary = looks_like_binary(bytes);
    if looks_binary {
        return parse_binary(bytes);
    }
    if bytes.starts_with(b"solid") {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| KairosError::validation("ASCII STL 含非法 UTF-8 字节。"))?;
        return parse_ascii(text);
    }
    if bytes.len() >= BINARY_HEADER {
        return parse_binary(bytes);
    }
    Err(KairosError::validation(
        "无法识别的 STL 文件（过短且缺少 solid 头）。",
    ))
}

/// 二进制判定：文件长度恰好等于 84 + 50 × 三角形数。
fn looks_like_binary(bytes: &[u8]) -> bool {
    if bytes.len() < BINARY_HEADER {
        return false;
    }
    let count = u32::from_le_bytes(bytes[80..84].try_into().expect("长度已检查"));
    bytes.len() == BINARY_HEADER + count as usize * BINARY_TRIANGLE
}

fn parse_binary(bytes: &[u8]) -> Result<TriangleMesh> {
    // 不变量：两条调用路径（looks_like_binary / 长度兜底）都保证 len >= BINARY_HEADER。
    let count = u32::from_le_bytes(bytes[80..84].try_into().expect("长度已检查")) as usize;
    let expected = BINARY_HEADER + count * BINARY_TRIANGLE;
    if bytes.len() < expected {
        return Err(KairosError::validation(format!(
            "二进制 STL 被截断：声明 {count} 个三角形，实际数据不足（{expected} 字节需要）。"
        )));
    }
    let mut triangles = Vec::with_capacity(count);
    for index in 0..count {
        let offset = BINARY_HEADER + index * BINARY_TRIANGLE;
        let float = |at: usize| -> f64 {
            f32::from_le_bytes(
                bytes[offset + at..offset + at + 4]
                    .try_into()
                    .expect("窗口内"),
            ) as f64
        };
        triangles.push(Triangle {
            normal: [float(0), float(4), float(8)],
            a: [float(12), float(16), float(20)],
            b: [float(24), float(28), float(32)],
            c: [float(36), float(40), float(44)],
        });
    }
    Ok(TriangleMesh { triangles })
}

fn parse_f64(token: &str, line: usize) -> Result<f64> {
    token
        .parse::<f64>()
        .map_err(|_| {
            KairosError::validation(format!("第 {} 行：数值「{token}」无法解析。", line + 1))
        })
        .and_then(|value| {
            if value.is_finite() {
                Ok(value)
            } else {
                Err(KairosError::validation(format!(
                    "第 {} 行：数值「{token}」不是有限数。",
                    line + 1
                )))
            }
        })
}

fn parse_ascii(text: &str) -> Result<TriangleMesh> {
    let mut triangles = Vec::new();
    let mut normal = [0.0f64; 3];
    let mut vertices: Vec<[f64; 3]> = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        match tokens.as_slice() {
            ["facet", "normal", nx, ny, nz] => {
                normal = [
                    parse_f64(nx, index)?,
                    parse_f64(ny, index)?,
                    parse_f64(nz, index)?,
                ];
                vertices.clear();
            }
            ["vertex", x, y, z] => {
                vertices.push([
                    parse_f64(x, index)?,
                    parse_f64(y, index)?,
                    parse_f64(z, index)?,
                ]);
            }
            ["endfacet"] => {
                if vertices.len() != 3 {
                    return Err(KairosError::validation(format!(
                        "第 {} 行：facet 顶点数为 {}（应为 3）。",
                        index + 1,
                        vertices.len()
                    )));
                }
                triangles.push(Triangle {
                    a: vertices[0],
                    b: vertices[1],
                    c: vertices[2],
                    normal,
                });
                vertices = Vec::new();
            }
            _ => {}
        }
    }

    if triangles.is_empty() {
        return Err(KairosError::validation(
            "ASCII STL 中没有解析到任何三角形。",
        ));
    }
    Ok(TriangleMesh { triangles })
}

/// 网格健康检查：焊接顶点（按包围盒对角线的 1e-6 容差）后统计
/// 退化三角形、边界边、非流形边与法向不一致边。
pub fn check_mesh(mesh: &TriangleMesh) -> MeshIssues {
    let diagonal = mesh.diagonal().max(1.0);
    let tolerance = diagonal * 1e-6;
    let key = |p: &[f64; 3]| -> [i64; 3] {
        [
            (p[0] / tolerance).round() as i64,
            (p[1] / tolerance).round() as i64,
            (p[2] / tolerance).round() as i64,
        ]
    };

    // 无向边（按字典序的顶点键对）→ (正向次数, 反向次数)。
    // 一致定向的封闭网格中，共享边必被相邻两三角形一正一反各用一次。
    let mut edges: HashMap<([i64; 3], [i64; 3]), (usize, usize)> = HashMap::new();
    let mut issues = MeshIssues::default();
    let degenerate_limit = diagonal * 1e-9;

    for triangle in &mesh.triangles {
        if triangle.area() < degenerate_limit {
            issues.degenerate += 1;
            continue;
        }
        let keys = [key(&triangle.a), key(&triangle.b), key(&triangle.c)];
        for (first, second) in [(0usize, 1usize), (1, 2), (2, 0)] {
            let (from, to) = (keys[first], keys[second]);
            if from == to {
                // 焊接后退化为自环的边：按非流形计。
                issues.non_manifold_edges += 1;
                continue;
            }
            let undirected = if from <= to { (from, to) } else { (to, from) };
            let entry = edges.entry(undirected).or_insert((0, 0));
            if from <= to {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
    }

    for &(forward, backward) in edges.values() {
        let total = forward + backward;
        match total {
            1 => issues.open_edges += 1,
            2 => {
                if forward != 1 || backward != 1 {
                    issues.normal_inconsistent_edges += 1;
                }
            }
            _ => issues.non_manifold_edges += 1,
        }
    }

    issues
}

/// 按包围盒最大尺寸推断单位；注塑制件常为毫米量级。
pub fn infer_unit(max_dimension: f64) -> &'static str {
    if !max_dimension.is_finite() || max_dimension <= 0.0 {
        "未知"
    } else if max_dimension < 1.0 {
        "m"
    } else if max_dimension <= 10_000.0 {
        "mm"
    } else {
        "可疑（过大，请确认单位）"
    }
}

/// 解析 STL 并生成导入摘要（网格本体由调用方存入会话缓存）。
pub fn summarize(geometry_id: String, file_name: String, mesh: &TriangleMesh) -> GeometrySummary {
    let (min, max) = mesh.bounding_box();
    let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let suggested_unit = infer_unit(size.iter().cloned().fold(0.0, f64::max)).to_string();
    GeometrySummary {
        geometry_id,
        file_name,
        triangle_count: mesh.triangle_count(),
        size,
        surface_area: mesh.surface_area(),
        signed_volume: mesh.signed_volume(),
        suggested_unit,
        issues: check_mesh(mesh),
    }
}

/// 读取 STL 文件并解析。
/// 写出二进制 STL（Gmsh 引擎的文件输入；80 字节头 + 三角形逐个落盘）。
pub fn write_stl_binary(mesh: &TriangleMesh, path: &Path) -> Result<()> {
    use std::io::Write;
    let mut file =
        std::fs::File::create(path).map_err(|e| KairosError::io(format!("创建 STL 失败：{e}")))?;
    file.write_all(&[0u8; 80])
        .and_then(|_| file.write_all(&(mesh.triangles.len() as u32).to_le_bytes()))
        .map_err(|e| KairosError::io(format!("STL 头写入失败：{e}")))?;
    for tri in &mesh.triangles {
        // 法向取自 STL 解析时保存的三角形法向（只读记录，不参与几何）
        let mut record = Vec::with_capacity(50);
        for v in &tri.normal {
            record.extend_from_slice(&v.to_le_bytes());
        }
        for vertex in [&tri.a, &tri.b, &tri.c] {
            for component in vertex {
                record.extend_from_slice(&component.to_le_bytes());
            }
        }
        record.extend_from_slice(&0u16.to_le_bytes());
        file.write_all(&record)
            .map_err(|e| KairosError::io(format!("STL 三角形写入失败：{e}")))?;
    }
    Ok(())
}

pub fn parse_stl_file(path: &Path) -> Result<TriangleMesh> {
    let bytes = fs::read(path).map_err(|e| KairosError::io(format!("读取 STL 文件失败：{e}")))?;
    parse_stl(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 单位立方体 [0,1]³ 的 12 个三角形（封闭、法向朝外、绕向一致）。
    type Quad = ([f64; 3], [f64; 3], [f64; 3], [f64; 3]);

    fn unit_cube() -> Vec<Triangle> {
        let mut triangles = Vec::new();
        // 每个面的 4 个顶点按外法向的逆时针绕向给出。
        let faces: [Quad; 6] = [
            (
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
            ),
            (
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
            ),
            (
                [1.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
            ),
            (
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
            ),
            (
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
            ),
            (
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ),
        ];
        for (p0, p1, p2, p3) in faces {
            triangles.push(Triangle {
                a: p0,
                b: p1,
                c: p2,
                normal: [0.0; 3],
            });
            triangles.push(Triangle {
                a: p0,
                b: p2,
                c: p3,
                normal: [0.0; 3],
            });
        }
        triangles
    }

    #[test]
    fn cube_mesh_is_clean() {
        let mesh = TriangleMesh {
            triangles: unit_cube(),
        };
        assert_eq!(mesh.triangle_count(), 12);
        let issues = check_mesh(&mesh);
        assert!(issues.is_clean(), "立方体应为封闭一致网格：{issues:?}");
        assert!((mesh.surface_area() - 6.0).abs() < 1e-9);
        assert!((mesh.signed_volume() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn detects_open_and_degenerate() {
        let mut triangles = unit_cube();
        // 去掉顶面两个三角形 → 4 条开放边
        triangles.truncate(10);
        // 加一个退化三角形
        triangles.push(Triangle {
            a: [2.0, 0.0, 0.0],
            b: [2.0, 0.0, 0.0],
            c: [2.0, 1.0, 0.0],
            normal: [0.0; 3],
        });
        let issues = check_mesh(&TriangleMesh { triangles });
        assert_eq!(issues.degenerate, 1);
        assert_eq!(issues.open_edges, 4);
    }

    #[test]
    fn detects_non_manifold_edge() {
        let mut triangles = unit_cube();
        // 复制一个三角形：其 3 条边各被 3 个三角形使用
        triangles.push(triangles[0].clone());
        let issues = check_mesh(&TriangleMesh { triangles });
        assert_eq!(issues.non_manifold_edges, 3);
    }

    #[test]
    fn detects_inconsistent_orientation() {
        // 单三角形网格：每条边都是边界边；翻转其绕向不改变边界计数，
        // 但把两个共边三角形改为同向绕行 → 该边法向不一致。
        let mut triangles = Vec::new();
        let n = [0.0, 0.0, 1.0];
        triangles.push(Triangle {
            a: [0.0, 0.0, 0.0],
            b: [1.0, 0.0, 0.0],
            c: [1.0, 1.0, 0.0],
            normal: n,
        });
        // 第二个三角形绕向与前一个在共享边上同向 → 法向不一致
        triangles.push(Triangle {
            a: [0.0, 0.0, 0.0],
            b: [0.0, 1.0, 0.0],
            c: [1.0, 1.0, 0.0],
            normal: n,
        });
        let issues = check_mesh(&TriangleMesh { triangles });
        assert_eq!(issues.normal_inconsistent_edges, 1);
        assert_eq!(issues.open_edges, 4);
    }

    fn stl_ascii(triangles: &str) -> String {
        format!("solid demo\n{triangles}endsolid demo\n")
    }

    #[test]
    fn parses_ascii_stl() {
        let content = stl_ascii(
            "facet normal 0 0 1\n  outer loop\n    vertex 0 0 0\n    vertex 1 0 0\n    vertex 1 1 0\n  endloop\nendfacet\n",
        );
        let mesh = parse_stl(content.as_bytes()).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.triangles[0].a, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn parses_binary_stl() {
        let mut bytes = vec![0u8; 80];
        bytes.extend_from_slice(&1u32.to_le_bytes());
        let mut triangle = vec![0u8; 50];
        let value = 1.0f32;
        // 第一个顶点 x = 1.0，其余为 0
        triangle[12..16].copy_from_slice(&value.to_le_bytes());
        bytes.extend_from_slice(&triangle);
        let mesh = parse_stl(&bytes).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.triangles[0].a, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn rejects_truncated_binary() {
        let mut bytes = vec![0u8; 84];
        bytes[80..84].copy_from_slice(&2u32.to_le_bytes()); // 声明 2 个三角形
        bytes.extend_from_slice(&[0u8; 50]); // 只带 1 个三角形的数据
        let error = parse_stl(&bytes).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Validation);
    }

    #[test]
    fn rejects_bad_ascii_numbers() {
        let content = stl_ascii(
            "facet normal 0 0 1\n  outer loop\n    vertex abc 0 0\n    vertex 1 0 0\n    vertex 1 1 0\n  endloop\nendfacet\n",
        );
        let error = parse_stl(content.as_bytes()).unwrap_err();
        assert!(error.message().contains("无法解析"));
    }

    #[test]
    fn infers_units_by_size() {
        assert_eq!(infer_unit(0.05), "m");
        assert_eq!(infer_unit(50.0), "mm");
        assert!(infer_unit(50_000.0).contains("可疑"));
    }

    #[test]
    fn sample_box_is_watertight_and_volume_matches() {
        let mesh = TriangleMesh::sample_box(10.0);
        let issues = crate::services::geometry::check_mesh(&mesh);
        assert!(issues.is_clean(), "样例立方体应封闭一致：{issues:?}");
        assert!((mesh.signed_volume() - 1000.0).abs() < 1e-9);
        assert!((mesh.surface_area() - 600.0).abs() < 1e-9);
    }

    #[test]
    fn summarize_fills_report() {
        let mesh = TriangleMesh {
            triangles: unit_cube(),
        };
        let summary = summarize("g-1".into(), "cube.stl".into(), &mesh);
        assert_eq!(summary.geometry_id, "g-1");
        assert_eq!(summary.suggested_unit, "mm");
        assert_eq!(summary.triangle_count, 12);
        assert_eq!(summary.issues.degenerate, 0);
    }
}
