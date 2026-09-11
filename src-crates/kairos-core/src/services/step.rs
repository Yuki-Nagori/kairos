//! STEP 文件导入：AP242 镶嵌表示子集。
//! 支持 TRIANGULATED_FACE_SET + COORDINATES_LIST（CAD「镶嵌 STEP」导出）
//! 与面化 B-rep 子集（FACE 上的 POLY_LOOP 多边形面，扇形三角化）。
//! NURBS 曲面等精确 B-rep 几何不在支持范围，遇到时明确报错，
//! 请在 CAD 中以镶嵌 / 网格形式重新导出。

use std::collections::HashMap;

use crate::error::{KairosError, Result};
use crate::models::geometry::{Triangle, TriangleMesh};

type Point = [f64; 3];

/// 解析 STEP 文本为三角网格（镶嵌子集）。
pub fn parse_step(text: &str) -> Result<TriangleMesh> {
    // 实例语句以 ';' 分隔（容忍导出器的换行折行）。
    let mut points: HashMap<String, Point> = HashMap::new();
    let mut coord_lists: HashMap<String, Vec<Point>> = HashMap::new();
    // 镶嵌面集：坐标表引用 → 三角形顶点索引（1-based，相对坐标表）。
    let mut tessellations: Vec<(String, Vec<[usize; 3]>)> = Vec::new();
    // 多边形环：顶点引用序列（面化 B-rep）。
    let mut poly_loops: Vec<Vec<String>> = Vec::new();

    for statement in text.split(';') {
        let statement = statement.trim();
        let Some(rest) = statement.strip_prefix('#') else {
            continue;
        };
        let Some(eq_pos) = rest.find('=') else {
            continue;
        };
        let id = rest[..eq_pos].trim().to_string();
        let call = rest[eq_pos + 1..].trim();
        let Some(paren) = call.find('(') else {
            continue;
        };
        let entity = call[..paren].trim().to_uppercase();
        let args = &call[paren..];
        match entity.as_str() {
            "CARTESIAN_POINT" => {
                let triples = scan_float_triples(args);
                if let Some(point) = triples.first() {
                    points.insert(id, *point);
                }
            }
            "COORDINATES_LIST" => {
                coord_lists.insert(id, scan_float_triples(args));
            }
            "TRIANGULATED_FACE_SET" => {
                let coords_ref = extract_ref(args, 0);
                let triangles = scan_int_triples(args)
                    .into_iter()
                    .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
                    .collect();
                tessellations.push((coords_ref, triangles));
            }
            "POLY_LOOP" => {
                poly_loops.push(extract_refs(args));
            }
            _ => {}
        }
    }

    let mut triangles: Vec<Triangle> = Vec::new();

    // 镶嵌面集：顶点索引为 1-based，相对各坐标表。
    for (coords_ref, index_triangles) in &tessellations {
        let coords = coord_lists.get(coords_ref).ok_or_else(|| {
            KairosError::validation(format!("STEP 镶嵌面集引用的坐标表 #{coords_ref} 不存在。"))
        })?;
        for triangle in index_triangles {
            let indices = [
                triangle[0].checked_sub(1),
                triangle[1].checked_sub(1),
                triangle[2].checked_sub(1),
            ];
            let mut face = [[0.0f64; 3]; 3];
            for (slot, index) in indices.iter().enumerate() {
                let Some(index) = index else {
                    continue;
                };
                face[slot] = *coords
                    .get(*index)
                    .ok_or_else(|| KairosError::validation("STEP 镶嵌面集顶点索引越界。"))?;
            }
            triangles.push(Triangle {
                a: face[0],
                b: face[1],
                c: face[2],
                normal: [0.0, 0.0, 0.0],
            });
        }
    }

    // 多边形环：全部顶点引用须可解析为点，扇形三角化。
    for loop_refs in &poly_loops {
        if loop_refs.len() < 3 {
            continue;
        }
        let Some(loop_points) = loop_refs
            .iter()
            .map(|reference| points.get(reference).copied())
            .collect::<Option<Vec<Point>>>()
        else {
            continue;
        };
        for index in 1..loop_points.len() - 1 {
            triangles.push(Triangle {
                a: loop_points[0],
                b: loop_points[index],
                c: loop_points[index + 1],
                normal: [0.0, 0.0, 0.0],
            });
        }
    }

    if triangles.is_empty() {
        return Err(KairosError::validation(
            "STEP 中未找到可导入的镶嵌几何（支持 TRIANGULATED_FACE_SET 与 POLY_LOOP 面）；\
             NURBS B-rep 请先在 CAD 中以镶嵌 / 网格形式导出。",
        ));
    }
    Ok(TriangleMesh { triangles })
}

/// 解析 STEP 文件为三角网格。
pub fn parse_step_file(path: &std::path::Path) -> Result<TriangleMesh> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| KairosError::io(format!("读取 STEP 文件失败：{e}")))?;
    parse_step(&text)
}

/// 收集文本中所有最内层的纯数字括号组（如坐标三元组、三角形索引三元组）。
/// 引用（#n）、字符串（''）等非数字内容会使所在组被丢弃。
fn numeric_groups(text: &str) -> Vec<Vec<f64>> {
    let bytes = text.as_bytes();
    let mut groups: Vec<Vec<f64>> = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'(' {
            index += 1;
            continue;
        }
        // 收集到匹配的 ')' 为止；遇到嵌套 '(' 时放弃本组、
        // 从嵌套处重新收集（最内层组才是有效数据）。
        let mut numbers: Vec<f64> = Vec::new();
        let mut number = String::new();
        let mut numeric = true;
        let mut closed = false;
        let mut restart: Option<usize> = None;
        let mut cursor = index + 1;
        while cursor < bytes.len() {
            match bytes[cursor] {
                b'(' => {
                    numeric = false;
                    restart = Some(cursor);
                    break;
                }
                b')' => {
                    closed = true;
                    break;
                }
                b',' | b' ' | b'\t' => {
                    if !number.is_empty() {
                        match number.parse::<f64>() {
                            Ok(value) => numbers.push(value),
                            Err(_) => numeric = false,
                        }
                        number.clear();
                    }
                }
                _ => number.push(bytes[cursor] as char),
            }
            cursor += 1;
        }
        if let Some(pos) = restart {
            index = pos;
            continue;
        }
        if !closed {
            break;
        }
        if numeric && !number.is_empty() {
            if let Ok(value) = number.parse::<f64>() {
                numbers.push(value);
            } else {
                numeric = false;
            }
        }
        if numeric && !numbers.is_empty() {
            groups.push(numbers);
        }
        index = cursor + 1;
    }
    groups
}

/// 从实体参数文本中提取形如「(浮点,浮点,浮点)」的三元组列表。
fn scan_float_triples(text: &str) -> Vec<Point> {
    numeric_groups(text)
        .iter()
        .filter(|group| group.len() == 3)
        .map(|group| [group[0], group[1], group[2]])
        .collect()
}

/// 从实体参数文本中提取形如「(整数,整数,整数)」的三元组列表。
fn scan_int_triples(text: &str) -> Vec<[i64; 3]> {
    numeric_groups(text)
        .iter()
        .filter(|group| group.len() == 3)
        .map(|group| [group[0] as i64, group[1] as i64, group[2] as i64])
        .collect()
}

/// 从实体参数文本中提取全部「#数字」引用（按出现顺序）。
fn extract_refs(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut refs = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'#' {
            let mut number = String::new();
            let mut cursor = index + 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                number.push(bytes[cursor] as char);
                cursor += 1;
            }
            if !number.is_empty() {
                refs.push(number);
                index = cursor;
                continue;
            }
        }
        index += 1;
    }
    refs
}

/// 提取文本中的第一个「#数字」引用。
fn extract_ref(text: &str, fallback: usize) -> String {
    extract_refs(text)
        .first()
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tessellated_face_set_parses_into_triangles() {
        let text = r#"
ISO-10303-21;
#1=COORDINATES_LIST('',3,((0.E0,0.E0,0.E0),(1.E0,0.E0,0.E0),(0.E0,1.E0,0.E0)));
#2=TRIANGULATED_FACE_SET('',#1,(''),(1,2,3));
#3=TRIANGULATED_FACE_SET('',#1,(''),(1,3,2));
ENDSEC;
"#;
        let mesh = parse_step(text).unwrap();
        assert_eq!(mesh.triangle_count(), 2);
    }

    #[test]
    fn poly_loop_face_fans_into_triangles() {
        let text = r#"
#10=CARTESIAN_POINT('',(0.E0,0.E0,0.E0));
#11=CARTESIAN_POINT('',(1.E0,0.E0,0.E0));
#12=CARTESIAN_POINT('',(1.E0,1.E0,0.E0));
#13=CARTESIAN_POINT('',(0.E0,1.E0,0.E0));
#14=POLY_LOOP('',(#10,#11,#12,#13));
"#;
        let mesh = parse_step(text).unwrap();
        // 四点环扇形三角化为 2 个三角形
        assert_eq!(mesh.triangle_count(), 2);
        assert!((mesh.triangles[0].a[0] - 0.0).abs() < 1e-9);
    }

    #[test]
    fn pure_nurbs_step_reports_clear_error() {
        let text = "#5=B_SPLINE_SURFACE_WITH_KNOTS('',3,(#1,#2),.UNSPECIFIED.,...);";
        let error = parse_step(text).unwrap_err();
        assert!(error.to_string().contains("未找到可导入的镶嵌几何"));
    }
}
