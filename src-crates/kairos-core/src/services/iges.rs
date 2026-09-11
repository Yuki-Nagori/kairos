//! IGES 文件导入：镶嵌表示子集。
//! 支持实体 106（Copious Data，forms 0 / 1 / 2 / 11 / 12）与实体 63
//! （Compact Plane Subfigure）的封闭多边形，以及实体 124（变换矩阵）
//! 的空间定位；封闭环扇形三角化成面。开放折线与点列不构成曲面，跳过；
//! G 节自定义分隔符不解析（按标准 `,` `;` 处理）；NURBS 等 B-rep
//! 几何不在支持范围，请在 CAD 中以镶嵌 / 网格形式导出。

use std::collections::HashMap;

use crate::error::{KairosError, Result};
use crate::models::geometry::{Triangle, TriangleMesh};

type Point = [f64; 3];

/// 目录条目（D 节）：解析参数所需的最小字段。
#[derive(Debug, Clone, Copy)]
struct DirectoryEntry {
    entity_type: i64,
    form: i64,
    /// 变换矩阵（实体 124）的目录序号，0 表示恒等变换。
    transform: i64,
}

/// 候选环：点列与封闭性；只有封闭环可三角化成面。
struct Ring {
    points: Vec<Point>,
    closed: bool,
}

/// 导入统计：被跳过的几何实体数量（未知类型 / 不可解析 / 未成封闭环）。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportStats {
    pub skipped_entities: usize,
}

/// 解析 IGES 文本为三角网格（106 / 63 镶嵌子集）。
pub fn parse_iges(text: &str) -> Result<TriangleMesh> {
    parse_iges_with_stats(text).map(|(mesh, _)| mesh)
}

/// 解析 IGES 并返回跳过统计（供上层提示不完整导入）。
pub fn parse_iges_with_stats(text: &str) -> Result<(TriangleMesh, ImportStats)> {
    // 分节：只关心目录节 D 与参数节 P（固定 80 列行格式，第 73 列为节字母）。
    let mut d_lines: Vec<&str> = Vec::new();
    let mut p_lines: Vec<&str> = Vec::new();
    for line in text.lines() {
        let bytes = line.as_bytes();
        if bytes.len() < 73 {
            continue;
        }
        match bytes[72] {
            b'D' => d_lines.push(line),
            b'P' => p_lines.push(line),
            _ => {}
        }
    }
    if d_lines.is_empty() {
        return Err(KairosError::validation(
            "IGES 缺少目录节（D 节），无法识别任何实体。",
        ));
    }

    let entries = parse_directory(&d_lines);
    let params = collect_parameter_data(&p_lines);

    // 先收集变换矩阵（实体 124），几何实体按各自目录指向引用。
    let mut matrices: HashMap<i64, [f64; 12]> = HashMap::new();
    for (seq, entry) in &entries {
        if entry.entity_type != 124 {
            continue;
        }
        if let Some(text) = params.get(seq)
            && let Some(matrix) = parse_matrix(&tokenize(text))
        {
            matrices.insert(*seq, matrix);
        }
    }

    let mut triangles: Vec<Triangle> = Vec::new();
    let mut stats = ImportStats::default();
    for (seq, entry) in &entries {
        if entry.entity_type == 124 {
            continue;
        }
        let Some(text) = params.get(seq) else {
            continue;
        };
        let tokens = tokenize(text);
        let matrix = matrices.get(&entry.transform);
        let rings = match entry.entity_type {
            63 => parse_entity_63(&tokens, entry.form, matrix),
            106 => parse_entity_106(&tokens, entry.form, matrix),
            _ => {
                stats.skipped_entities += 1;
                continue;
            }
        };
        let mut produced = 0usize;
        for ring in rings.unwrap_or_default().iter().filter(|ring| ring.closed) {
            let before = triangles.len();
            fan_triangulate(&ring.points, &mut triangles);
            produced += triangles.len() - before;
        }
        if produced == 0 {
            stats.skipped_entities += 1;
        }
    }

    if triangles.is_empty() {
        return Err(KairosError::validation(
            "IGES 中未找到可导入的镶嵌几何（支持实体 106 Copious Data 与 63 \
             封闭多边形；开放折线 / 点列 / B-rep 实体不构成可导入曲面）。",
        ));
    }
    Ok((TriangleMesh { triangles }, stats))
}

/// 解析 IGES 文件为三角网格。
pub fn parse_iges_file(path: &std::path::Path) -> Result<TriangleMesh> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| KairosError::io(format!("读取 IGES 文件失败：{e}")))?;
    parse_iges(&text)
}

/// 目录节：每 2 行一组，各含 9 个 8 字符右对齐字段（列 1–72），
/// 序号在列 74–80。取实体类型（字段 1）、变换指向（字段 7）、
/// 窗体号（字段 15）与目录序号。
fn parse_directory(lines: &[&str]) -> Vec<(i64, DirectoryEntry)> {
    // 只用索引 0–6：切片必然落在列 1–72（进入本函数的行已保证 ≥ 73 列）。
    let field = |line: &str, index: usize| -> String {
        let start = index * 8;
        let end = (start + 8).min(72);
        String::from_utf8_lossy(&line.as_bytes()[start..end])
            .trim()
            .to_string()
    };
    let sequence = |line: &str| -> i64 {
        let bytes = line.as_bytes();
        if bytes.len() < 74 {
            return 0;
        }
        String::from_utf8_lossy(&bytes[73..])
            .trim()
            .parse()
            .unwrap_or(0)
    };
    let mut entries = Vec::new();
    for pair in lines.chunks(2) {
        if pair.len() < 2 {
            break;
        }
        let entry = DirectoryEntry {
            entity_type: field(pair[0], 0).parse().unwrap_or(0),
            form: field(pair[1], 4).parse().unwrap_or(0),
            transform: field(pair[0], 6).parse().unwrap_or(0),
        };
        // 目录序号取条目第一行的奇数号（与参数节的属主指向一致）。
        let seq = sequence(pair[0]);
        if seq > 0 {
            entries.push((seq, entry));
        }
    }
    entries
}

/// 参数节：按属主目录序号（列 65–72）聚合列 1–64 的参数文本；
/// 同一实体的参数折行时按出现顺序拼接。
fn collect_parameter_data(lines: &[&str]) -> HashMap<i64, String> {
    let mut params: HashMap<i64, String> = HashMap::new();
    for line in lines {
        // 进入本函数的参数行已保证 ≥ 73 列，列 1–64 与 65–72 可安全切片。
        let bytes = line.as_bytes();
        let owner: i64 = String::from_utf8_lossy(&bytes[64..72])
            .trim()
            .parse()
            .unwrap_or(0);
        if owner <= 0 {
            continue;
        }
        let data = String::from_utf8_lossy(&bytes[..64]);
        params
            .entry(owner)
            .and_modify(|text| text.push_str(&data))
            .or_insert_with(|| data.to_string());
    }
    params
}

/// 参数 token 流：默认分隔符 `,` 与 `;`；Hollerith 字符串（nH…）整体
/// 作为一个 token，其内容里的分隔符不破坏切分；`;` 结束当前记录。
fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(&ch) = chars.peek() {
        match ch {
            ',' | ';' => {
                chars.next();
                let token = current.trim().to_string();
                if !token.is_empty() {
                    tokens.push(token);
                }
                current.clear();
                if ch == ';' {
                    break;
                }
            }
            '0'..='9' => {
                // 可能是 Hollerith 长度前缀：数字后紧跟 H 时按字符串吞掉。
                let mut lookahead = chars.clone();
                let mut digits = String::new();
                while let Some(&digit) = lookahead.peek() {
                    if digit.is_ascii_digit() {
                        digits.push(digit);
                        lookahead.next();
                    } else {
                        break;
                    }
                }
                if lookahead.peek() == Some(&'H') {
                    lookahead.next();
                    let count: usize = digits.parse().unwrap_or(0);
                    let mut string = String::new();
                    for _ in 0..count {
                        match lookahead.next() {
                            Some(c) => string.push(c),
                            None => break,
                        }
                    }
                    chars = lookahead;
                    if !string.is_empty() {
                        tokens.push(string);
                    }
                } else {
                    current.push(ch);
                    chars.next();
                }
            }
            _ => {
                current.push(ch);
                chars.next();
            }
        }
    }
    let token = current.trim().to_string();
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

fn parse_number(token: &str) -> Option<f64> {
    token.trim().parse::<f64>().ok()
}

/// 实体 124（变换矩阵）：参数为 12 个实数 R11..R34。
fn parse_matrix(tokens: &[String]) -> Option<[f64; 12]> {
    if tokens.len() < 13 {
        return None;
    }
    let mut matrix = [0.0; 12];
    for (slot, token) in tokens[1..13].iter().enumerate() {
        matrix[slot] = parse_number(token)?;
    }
    Some(matrix)
}

/// 变换作用于点：无矩阵时恒等。
fn apply_transform(point: Point, matrix: Option<&[f64; 12]>) -> Point {
    match matrix {
        None => point,
        Some(m) => [
            m[0] * point[0] + m[1] * point[1] + m[2] * point[2] + m[3],
            m[4] * point[0] + m[5] * point[1] + m[6] * point[2] + m[7],
            m[8] * point[0] + m[9] * point[1] + m[10] * point[2] + m[11],
        ],
    }
}

/// 二维定义空间点提升为三维（z = 0）后施加实体变换。
fn lift_2d(x: f64, y: f64, matrix: Option<&[f64; 12]>) -> Point {
    apply_transform([x, y, 0.0], matrix)
}

/// 读出 tokens[from..] 的前 count 个数；数量不足或出现非数返回 None。
fn read_numbers(tokens: &[String], from: usize, count: usize) -> Option<Vec<f64>> {
    let mut values = Vec::with_capacity(count);
    for token in tokens.iter().skip(from).take(count) {
        values.push(parse_number(token)?);
    }
    (values.len() == count).then_some(values)
}

/// 首尾点是否重合（按坐标量级的相对容差判断）。
fn endpoints_coincide(points: &[Point]) -> bool {
    const RELATIVE_TOLERANCE: f64 = 1e-9;
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return false;
    };
    let scale = first
        .iter()
        .chain(last.iter())
        .map(|value| value.abs())
        .fold(0.0, f64::max);
    let tolerance = scale * RELATIVE_TOLERANCE + 1e-24;
    distance_sq(*first, *last) <= tolerance * tolerance
}

/// 实体 63（Compact Plane Subfigure）：IP、点数 N、N 个 (x, y)。
/// form 0 为封闭多边形，form 1 为开放多边形；其他 form 不认识。
fn parse_entity_63(tokens: &[String], form: i64, matrix: Option<&[f64; 12]>) -> Option<Vec<Ring>> {
    if form != 0 && form != 1 {
        return None;
    }
    let count = parse_number(tokens.get(2)?)? as usize;
    let values = read_numbers(tokens, 3, count * 2)?;
    let points = values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| lift_2d(pair[0], pair[1], matrix))
        .collect();
    Some(vec![Ring {
        points,
        closed: form == 0,
    }])
}

/// 实体 106（Copious Data）按 form 分派：
/// - form 0：重复「点数 M + M 个 (x,y)」块，每块为封闭多边形；
/// - form 1 / 2：IP、点数 N、N 个 (x,y,z)，线性路径 / 点列（首尾重合视为封闭）；
/// - form 11 / 12：IP、块数，每块「M + M 个 (x,y) / (x,y,z)」线性路径。
///
/// 未知 form 或参数不完整返回 None。
fn parse_entity_106(tokens: &[String], form: i64, matrix: Option<&[f64; 12]>) -> Option<Vec<Ring>> {
    let ip = parse_number(tokens.get(1)?)?;
    if !(0.0..=2.0).contains(&ip) {
        return None;
    }
    match form {
        0 => {
            let mut rings = Vec::new();
            let mut cursor = 2usize;
            while cursor < tokens.len() {
                let count = parse_number(tokens.get(cursor)?)? as usize;
                let values = read_numbers(tokens, cursor + 1, count * 2)?;
                rings.push(Ring {
                    points: values
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|pair| lift_2d(pair[0], pair[1], matrix))
                        .collect(),
                    closed: true,
                });
                cursor += 1 + count * 2;
            }
            Some(rings)
        }
        1 | 2 => {
            let count = parse_number(tokens.get(2)?)? as usize;
            let values = read_numbers(tokens, 3, count * 3)?;
            let points: Vec<Point> = values
                .as_chunks::<3>()
                .0
                .iter()
                .map(|triple| apply_transform([triple[0], triple[1], triple[2]], matrix))
                .collect();
            let closed = form == 1 && points.len() >= 3 && endpoints_coincide(&points);
            Some(vec![Ring { points, closed }])
        }
        11 | 12 => {
            let stride = if form == 11 { 2 } else { 3 };
            let groups = parse_number(tokens.get(2)?)? as usize;
            let mut rings = Vec::new();
            let mut cursor = 3usize;
            for _ in 0..groups {
                let count = parse_number(tokens.get(cursor)?)? as usize;
                let values = read_numbers(tokens, cursor + 1, count * stride)?;
                let points: Vec<Point> = values
                    .chunks_exact(stride)
                    .map(|chunk| {
                        if stride == 2 {
                            lift_2d(chunk[0], chunk[1], matrix)
                        } else {
                            apply_transform([chunk[0], chunk[1], chunk[2]], matrix)
                        }
                    })
                    .collect();
                let closed = points.len() >= 3 && endpoints_coincide(&points);
                rings.push(Ring { points, closed });
                cursor += 1 + count * stride;
            }
            Some(rings)
        }
        _ => None,
    }
}

/// 封闭环扇形三角化；显式重复的尾点（与首点重合）去掉后不足 3 点则不成面。
fn fan_triangulate(points: &[Point], triangles: &mut Vec<Triangle>) {
    let mut ring = points.to_vec();
    if ring.len() >= 2 && endpoints_coincide(&ring) {
        ring.pop();
    }
    if ring.len() < 3 {
        return;
    }
    for index in 1..ring.len() - 1 {
        triangles.push(Triangle {
            a: ring[0],
            b: ring[index],
            c: ring[index + 1],
            normal: [0.0, 0.0, 0.0],
        });
    }
}

fn distance_sq(a: Point, b: Point) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 组装 80 列行：内容 + 节字母 + 7 位序号。
    fn section_line(content: &str, section: char, seq: usize) -> String {
        let mut line = content.to_string();
        while line.chars().count() < 72 {
            line.push(' ');
        }
        line.push(section);
        line.push_str(&format!("{seq:>7}"));
        line
    }

    /// 目录条目两行：类型 / 变换指向 / 窗体号，序号取目录序号。
    fn directory_lines(seq: usize, entity_type: i64, transform: i64, form: i64) -> [String; 2] {
        let line1 = format!(
            "{entity_type:>8}{seq:>8}       0       0       0       0{transform:>8}       0       0"
        );
        let line2 = format!(
            "{entity_type:>8}       0       0       1{form:>8}       0       0               0"
        );
        [
            section_line(&line1, 'D', seq * 2 - 1),
            section_line(&line2, 'D', seq * 2),
        ]
    }

    /// 参数行：语句内容（列 1–64）+ 属主目录序号（列 65–72）。
    fn parameter_lines(owner: usize, statement: &str) -> Vec<String> {
        let mut line = statement.to_string();
        while line.chars().count() < 64 {
            line.push(' ');
        }
        line.push_str(&format!("{:>8}", owner * 2 - 1));
        vec![section_line(&line, 'P', owner * 2 - 1)]
    }

    fn iges_file(d: Vec<String>, p: Vec<String>) -> String {
        let mut text = String::from("S      1\n");
        text.extend(d.iter().map(|line| format!("{line}\n")));
        text.extend(p.iter().map(|line| format!("{line}\n")));
        text.push_str("T      1\n");
        text
    }

    /// 组装完整用例：目录条目 (类型, 变换, form) 与参数语句按目录序号配对。
    fn fixture(entries: &[(usize, i64, i64, i64, &str)]) -> String {
        let mut d = Vec::new();
        let mut p = Vec::new();
        for (seq, entity_type, transform, form, statement) in entries {
            d.extend(directory_lines(*seq, *entity_type, *transform, *form));
            p.extend(parameter_lines(*seq, statement));
        }
        iges_file(d, p)
    }

    const SHIFT_124: &str = "124,1.,0.,0.,10.,0.,1.,0.,20.,0.,0.,1.,30.;";

    #[test]
    fn entity_63_closed_polygon_triangulates_with_transform() {
        let text = iges_file(
            {
                let mut d = Vec::new();
                d.extend(directory_lines(1, 124, 0, 0));
                d.extend(directory_lines(2, 63, 1, 0));
                d
            },
            {
                let mut p = Vec::new();
                p.extend(parameter_lines(1, SHIFT_124));
                p.extend(parameter_lines(2, "63,0,4,0.,0.,4.,0.,4.,4.,0.,4.;"));
                p
            },
        );
        let mesh = parse_iges(&text).unwrap();
        assert_eq!(mesh.triangle_count(), 2);
        // 变换已生效：顶点平移到 (10..14, 20..24, 30)。
        for triangle in &mesh.triangles {
            for point in [triangle.a, triangle.b, triangle.c] {
                assert!((point[2] - 30.0).abs() < 1e-12);
                assert!((10.0..=14.0).contains(&point[0]));
                assert!((20.0..=24.0).contains(&point[1]));
            }
        }
    }

    #[test]
    fn entity_63_without_transform_stays_in_definition_plane() {
        let text = fixture(&[(1, 63, 0, 0, "63,0,3,0.,0.,1.,0.,0.,1.;")]);
        let mesh = parse_iges(&text).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.triangles[0].a, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn mixed_import_counts_skipped_entities() {
        // 有效 63 + 未知 110 + 开放 63（form 1）→ 1 个三角形、2 个跳过。
        let text = fixture(&[
            (1, 63, 0, 0, "63,0,3,0.,0.,1.,0.,0.,1.;"),
            (2, 110, 0, 0, "110,0.,0.,0.,1.,1.,1.;"),
            (3, 63, 0, 1, "63,1,3,0.,5.,1.,5.,0.,5.;"),
        ]);
        let (mesh, stats) = parse_iges_with_stats(&text).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(
            stats,
            ImportStats {
                skipped_entities: 2
            }
        );
    }

    #[test]
    fn entity_63_unknown_form_is_skipped() {
        let text = fixture(&[(1, 63, 0, 7, "63,0,3,0.,0.,1.,0.,0.,1.;")]);
        assert!(parse_iges(&text).is_err());
    }

    #[test]
    fn entity_106_form0_blocks_make_closed_polygons() {
        let text = fixture(&[(
            1,
            106,
            0,
            0,
            "106,0,3,0.,0.,1.,0.,0.,1.,3,2.,0.,3.,0.,3.,1.;",
        )]);
        let mesh = parse_iges(&text).unwrap();
        assert_eq!(mesh.triangle_count(), 2);
        assert!(mesh.triangles[1].a[0] >= 2.0);
    }

    #[test]
    fn entity_106_form1_closed_path_triangulates_open_path_skipped() {
        let closed = fixture(&[(
            1,
            106,
            0,
            1,
            "106,1,5,0.,0.,0.,1.,0.,0.,1.,1.,0.,0.,1.,0.,0.,0.,0.;",
        )]);
        assert_eq!(parse_iges(&closed).unwrap().triangle_count(), 2);

        let open = fixture(&[(1, 106, 0, 1, "106,1,3,0.,0.,0.,1.,0.,0.,2.,0.,0.;")]);
        assert!(parse_iges(&open).is_err());
    }

    #[test]
    fn entity_106_form2_point_series_never_imports() {
        let text = fixture(&[(1, 106, 0, 2, "106,2,4,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.;")]);
        assert!(parse_iges(&text).is_err());
    }

    #[test]
    fn entity_106_forms_11_and_12_group_paths() {
        let form11 = fixture(&[(1, 106, 0, 11, "106,1,1,5,0.,0.,1.,0.,1.,1.,0.,1.,0.,0.;")]);
        assert_eq!(parse_iges(&form11).unwrap().triangle_count(), 2);

        let form12 = fixture(&[(
            1,
            106,
            0,
            12,
            "106,1,1,5,0.,0.,1.,1.,0.,1.,1.,1.,1.,0.,1.,1.,0.,0.,1.,0.,0.,1.;",
        )]);
        let mesh = parse_iges(&form12).unwrap();
        assert_eq!(mesh.triangle_count(), 2);
        assert!((mesh.triangles[0].a[2] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn missing_d_section_is_rejected() {
        let error = parse_iges("S      1\nT      1\n").unwrap_err();
        assert!(error.to_string().contains("缺少目录节"));
    }

    #[test]
    fn entity_without_parameter_data_is_ignored() {
        // 目录声明了 63 与 124 但参数节为空（属主列空白）：跳过而不是崩溃。
        let text = iges_file(
            {
                let mut d = Vec::new();
                d.extend(directory_lines(1, 124, 0, 0));
                d.extend(directory_lines(2, 63, 1, 0));
                d
            },
            vec![section_line("", 'P', 1)],
        );
        assert!(parse_iges(&text).is_err());
    }

    #[test]
    fn short_lines_and_foreign_sections_are_ignored() {
        let mut text = String::from("short\n");
        text.push_str(&section_line("1H,,1H;,4HKairos;", 'G', 1));
        text.push('\n');
        text.push_str(&fixture(&[(1, 63, 0, 0, "63,0,3,0.,0.,1.,0.,0.,1.;")]));
        assert_eq!(parse_iges(&text).unwrap().triangle_count(), 1);
    }

    #[test]
    fn truncated_directory_line_is_dropped() {
        // 截断到 73 列：序号缺失按 0 处理；截断到 74 列：序号非数字按 0 处理。
        // 两种情况条目都因序号 ≤ 0 被丢弃。
        let mut short = section_line("63", 'D', 1);
        short.truncate(73);
        let mut bad_seq = section_line("63", 'D', 1);
        bad_seq.truncate(74);
        let text = format!("{short}\n{bad_seq}\nS      1\n");
        assert!(parse_iges(&text).is_err());
    }

    #[test]
    fn hollerith_strings_are_swallowed_whole() {
        let tokens = tokenize("63,0,4HAB,C,1.,2.;,9H尾部垃圾;7");
        assert_eq!(tokens, vec!["63", "0", "AB,C", "1.", "2."]);
    }

    #[test]
    fn tokenizer_stops_at_record_delimiter() {
        let tokens = tokenize("124,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0. ; 999");
        assert_eq!(tokens.len(), 13);
        assert_eq!(tokens.last().unwrap(), "0.");
        // 无终止符的残句保留为最后一个 token。
        assert_eq!(
            tokenize("no-delimiter-at-all").as_slice(),
            ["no-delimiter-at-all"]
        );
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn hollerith_count_exceeding_line_end_is_clamped() {
        // 越界长度前缀把行内剩余字符整体吞掉（畸形数据不 panic、不破坏切分）。
        assert_eq!(tokenize("99Honly-this;rest").as_slice(), ["only-this;rest"]);
        assert!(tokenize("0H;").is_empty());
    }

    #[test]
    fn matrix_requires_thirteen_tokens_and_numbers() {
        assert!(parse_matrix(&tokenize("124,1.,0.;")).is_none());
        assert!(parse_matrix(&tokenize("124,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,x.;")).is_none());
        assert!(parse_matrix(&tokenize(SHIFT_124)).is_some());
    }

    #[test]
    fn entity_63_with_bad_parameters_is_skipped() {
        for statement in [
            "63,0",                      // 缺点数
            "63,0,x,0.,0.,1.,0.,0.,1.;", // 点数非数
            "63,0,3,0.,0.,1.,0.,0.;",    // 数值不足
        ] {
            let text = fixture(&[(1, 63, 0, 0, statement)]);
            assert!(parse_iges(&text).is_err(), "应跳过：{statement}");
        }
    }

    #[test]
    fn entity_106_bad_ip_or_missing_tokens_are_rejected() {
        for (statement, form) in [
            ("106", 0),                // 缺 IP
            ("106,7,1,1.,1.;", 0),     // IP 越界
            ("106,0,1,1.,1.;", 9),     // 未知 form
            ("106,0,2,0.,0.,", 0),     // form 0 块内数值不足
            ("106,1", 1),              // form 1 缺点数
            ("106,1,2,0.,0.,0.,", 1),  // form 1 数值不足
            ("106,11,2,1,1.,0.;", 11), // form 11 声明 2 组只提供 1 组
        ] {
            let text = fixture(&[(1, 106, 0, form, statement)]);
            assert!(parse_iges(&text).is_err(), "应跳过：{statement}");
        }
    }

    #[test]
    fn entity_106_group_with_degenerate_point_count_is_skipped() {
        // 组内只有 2 个点：无法成环，实体被跳过。
        let text = fixture(&[(1, 106, 0, 12, "106,1,1,2,0.,0.,1.,0.,0.;")]);
        assert!(parse_iges(&text).is_err());
    }

    #[test]
    fn triangle_fan_drops_duplicate_tail_and_rejects_tiny_rings() {
        // 尾点与首点显式重合：4 点环去重后 3 点 → 1 个三角形而非退化扇面。
        let mut triangles = Vec::new();
        fan_triangulate(
            &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 0.]],
            &mut triangles,
        );
        assert_eq!(triangles.len(), 1);
        // 两点环不成面。
        let mut triangles = Vec::new();
        fan_triangulate(&[[0., 0., 0.], [0., 0., 0.]], &mut triangles);
        assert!(triangles.is_empty());
    }

    #[test]
    fn odd_trailing_directory_line_is_dropped() {
        // 目录节行数为奇数：不成对的尾行直接丢弃。
        let mut d = Vec::new();
        d.extend(directory_lines(1, 63, 0, 0));
        d.extend(directory_lines(2, 63, 0, 0));
        let [first, _] = directory_lines(3, 63, 0, 0);
        d.push(first);
        let text = iges_file(d, parameter_lines(1, "63,0,3,0.,0.,1.,0.,0.,1.;"));
        assert_eq!(parse_iges(&text).unwrap().triangle_count(), 1);
    }

    #[test]
    fn parameter_statement_split_across_lines_is_concatenated() {
        // 同一实体的参数折到第二条参数行：按属主序号拼接后照常解析。
        let text = iges_file(
            directory_lines(1, 63, 0, 0).to_vec(),
            [
                parameter_lines(1, "63,0,3,0.,0.,1.,0."),
                parameter_lines(1, ",0.,1.;"),
            ]
            .concat(),
        );
        let mesh = parse_iges(&text).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.triangles[0].c, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn endpoints_coincide_rejects_empty_input() {
        // 空切片无法取首尾点：返回 false；单点首尾同点，视为重合。
        assert!(!endpoints_coincide(&[]));
        assert!(endpoints_coincide(&[[1.0, 2.0, 3.0]]));
    }

    #[test]
    fn parse_iges_file_reads_disk_and_reports_io_error() {
        let path = std::env::temp_dir().join(format!("kairos-iges-{}.igs", std::process::id()));
        std::fs::write(
            &path,
            fixture(&[(1, 63, 0, 0, "63,0,3,0.,0.,1.,0.,0.,1.;")]),
        )
        .unwrap();
        assert_eq!(parse_iges_file(&path).unwrap().triangle_count(), 1);
        std::fs::remove_file(&path).ok();
        assert!(
            parse_iges_file(&path)
                .unwrap_err()
                .to_string()
                .contains("读取 IGES 文件失败")
        );
    }
}
