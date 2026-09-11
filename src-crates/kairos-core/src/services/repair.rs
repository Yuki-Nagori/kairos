//! 三角网格修复：顶点焊接、退化面移除、孔洞填充、法向一致化与自交检测。
//! 输入输出都是三角形汤（STL 风格），不引入索引网格中间结构。

use std::collections::HashMap;

use crate::error::{KairosError, Result};
use crate::models::geometry::{Triangle, TriangleMesh};

pub use crate::models::repair::RepairReport;

/// 顶点量化键（容差 = 模型对角线 × 1e-6，与网格检查一致）。
fn vertex_key(position: &[f64; 3], tolerance: f64) -> [i64; 3] {
    [
        (position[0] / tolerance).round() as i64,
        (position[1] / tolerance).round() as i64,
        (position[2] / tolerance).round() as i64,
    ]
}

/// 修复入口：焊接 → 去退化 → 填孔 → 一致化 → 自交检测。
pub fn repair_mesh(mesh: &TriangleMesh) -> Result<(TriangleMesh, RepairReport)> {
    let diagonal = mesh.diagonal();
    if diagonal <= 0.0 {
        return Err(KairosError::validation(
            "网格为空或没有空间尺寸，无法修复。",
        ));
    }
    let tolerance = diagonal * 1e-6;
    let mut report = RepairReport::default();

    // 1) 顶点焊接：量化坐标相同的顶点合并为同一 key；任两边重合的面即退化，丢弃。
    let mut weld: HashMap<[i64; 3], [f64; 3]> = HashMap::new();
    let mut raw_vertices = 0usize;
    let mut faces: Vec<([i64; 3], [i64; 3], [i64; 3])> = Vec::new();
    for triangle in &mesh.triangles {
        let mut key_face = ([0i64; 3], [0i64; 3], [0i64; 3]);
        for (index, position) in [&triangle.a, &triangle.b, &triangle.c].iter().enumerate() {
            raw_vertices += 1;
            let key = vertex_key(position, tolerance);
            weld.entry(key).or_insert(**position);
            if index == 0 {
                key_face.0 = key;
            } else if index == 1 {
                key_face.1 = key;
            } else {
                key_face.2 = key;
            }
        }
        if key_face.0 != key_face.1 && key_face.1 != key_face.2 && key_face.0 != key_face.2 {
            faces.push(key_face);
        }
    }
    report.merged_vertices = raw_vertices.saturating_sub(weld.len());
    report.removed_degenerate = mesh.triangles.len() - faces.len();

    // 2) 孔洞填充：无向边界边连通成环，质心扇形补面（绕行随后统一修正）。
    fill_holes(&mut faces, &mut weld, &mut report);

    // 3) 法向一致化：共享边两面的绕行方向必须相反，否则翻转后者。
    report.flipped_faces = orient_faces(&mut faces);

    let triangles = faces
        .iter()
        .map(|(a, b, c)| {
            let normal = face_normal(a, b, c, &weld);
            Triangle {
                a: weld[a],
                b: weld[b],
                c: weld[c],
                normal,
            }
        })
        .collect();
    let repaired = TriangleMesh { triangles };

    // 4) 自交检测（修复后网格上的计数报告）。
    report.self_intersections = count_self_intersections(&repaired);

    Ok((repaired, report))
}

fn face_normal(
    a: &[i64; 3],
    b: &[i64; 3],
    c: &[i64; 3],
    weld: &HashMap<[i64; 3], [f64; 3]>,
) -> [f64; 3] {
    let (pa, pb, pc) = (weld[a], weld[b], weld[c]);
    let ab = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
    let ac = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    if length > 0.0 {
        [cross[0] / length, cross[1] / length, cross[2] / length]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// 孔洞填充：无向边界边（只被 1 个面使用的边）连通成环后，以环质心
/// 扇形补面；补面绕行方向随后由法向一致化统一修正，此处不关心方向。
fn fill_holes(
    faces: &mut Vec<([i64; 3], [i64; 3], [i64; 3])>,
    weld: &mut HashMap<[i64; 3], [f64; 3]>,
    report: &mut RepairReport,
) {
    type Key = [i64; 3];
    type Edge = (Key, Key);

    // 无向边 → 出现次数；恰好出现 1 次的是边界边。
    let mut counts: HashMap<Edge, usize> = HashMap::new();
    for (a, b, c) in faces.iter() {
        for (from, to) in [(*a, *b), (*b, *c), (*c, *a)] {
            let key = if from < to { (from, to) } else { (to, from) };
            *counts.entry(key).or_insert(0) += 1;
        }
    }
    let mut boundary_neighbors: HashMap<Key, Vec<Key>> = HashMap::new();
    for ((from, to), count) in &counts {
        if *count == 1 {
            boundary_neighbors.entry(*from).or_default().push(*to);
            boundary_neighbors.entry(*to).or_default().push(*from);
        }
    }
    if boundary_neighbors.is_empty() {
        return;
    }

    // 环行走：每步取「来向之外」的当前顶点边界邻接点，回到起点即闭合；
    // 邻接数不是 2 的顶点属于分叉或断链，该环放弃填充（交由人工检查）。
    let mut consumed_edges: std::collections::HashSet<Edge> = std::collections::HashSet::new();
    let mut holes: Vec<Vec<Key>> = Vec::new();
    for (&start, start_neighbors) in &boundary_neighbors {
        if start_neighbors.len() != 2 {
            continue;
        }
        // 首步固定取第一个邻接点（环的两个方向等价），之后每步排除来向。
        let first = start_neighbors[0];
        consumed_edges.insert((start, first));
        consumed_edges.insert((first, start));
        let mut loop_vertices = vec![start, first];
        let mut previous = first;
        let mut current = first;
        let closed = loop {
            let candidates: Vec<Key> = boundary_neighbors[&current]
                .iter()
                .copied()
                .filter(|next| *next != previous && !consumed_edges.contains(&(current, *next)))
                .collect();
            match candidates.as_slice() {
                [next] => {
                    consumed_edges.insert((current, *next));
                    consumed_edges.insert((*next, current));
                    previous = current;
                    if *next == start {
                        break true;
                    }
                    loop_vertices.push(*next);
                    current = *next;
                }
                _ => break false,
            }
        };
        if closed && loop_vertices.len() >= 3 {
            holes.push(loop_vertices);
        }
    }

    for loop_vertices in holes {
        let count = loop_vertices.len();
        let mut centroid = [0.0f64; 3];
        for vertex in &loop_vertices {
            centroid[0] += vertex[0] as f64 / count as f64;
            centroid[1] += vertex[1] as f64 / count as f64;
            centroid[2] += vertex[2] as f64 / count as f64;
        }
        let centroid_key = [
            centroid[0].round() as i64,
            centroid[1].round() as i64,
            centroid[2].round() as i64,
        ];
        weld.entry(centroid_key).or_insert(centroid);
        for index in 0..count {
            let a = loop_vertices[index];
            let b = loop_vertices[(index + 1) % count];
            faces.push((a, b, centroid_key));
            report.filled_triangles += 1;
        }
        report.filled_holes += 1;
    }
}

/// 法向一致化：沿共享边做 BFS 方向传播——相邻两面绕行方向必须相反，
/// 同向则翻转其中一面（新面入队继续传播，直至整个连通片一致）。
fn orient_faces(faces: &mut [([i64; 3], [i64; 3], [i64; 3])]) -> usize {
    type Key = [i64; 3];
    type Edge = (Key, Key);

    // 无向边 → 拥有它的面（理论上 ≤ 2，非流形边跳过不一致化）。
    let mut edge_faces: HashMap<Edge, Vec<(usize, bool)>> = HashMap::new();
    for (index, (a, b, c)) in faces.iter().enumerate() {
        for (from, to) in [(*a, *b), (*b, *c), (*c, *a)] {
            let key = if from < to { (from, to) } else { (to, from) };
            let forward = from < to;
            edge_faces.entry(key).or_default().push((index, forward));
        }
    }

    let count = faces.len();
    let mut visited = vec![false; count];
    let mut flipped = 0usize;

    for seed in 0..count {
        if visited[seed] {
            continue;
        }
        visited[seed] = true;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(seed);
        while let Some(face) = queue.pop_front() {
            let (a, b, c) = faces[face];
            for (from, to) in [(a, b), (b, c), (c, a)] {
                let key = if from < to { (from, to) } else { (to, from) };
                // 边表由同一批面的归一化边构建，此处查找必然命中（不变量）。
                let owners = &edge_faces[&key];
                if owners.len() != 2 {
                    continue;
                }
                let neighbor = if owners[0].0 == face {
                    owners[1].0
                } else {
                    owners[0].0
                };
                if visited[neighbor] {
                    continue;
                }
                visited[neighbor] = true;
                // 一致性：共享边必须被两面「反向」绕行。当前面沿 key 正向
                // 绕行时，邻面也沿正向 = 不一致，翻转邻面并继续传播。
                // 邻面方向按其「当前」绕行实时读取（可能已被翻转过）。
                let face_forward = from < to;
                let (na, nb, nc) = faces[neighbor];
                let neighbor_forward = [(na, nb), (nb, nc), (nc, na)]
                    .iter()
                    .any(|(f, t)| *f == key.0 && *t == key.1);
                if face_forward == neighbor_forward {
                    let (a2, b2, c2) = faces[neighbor];
                    faces[neighbor] = (a2, c2, b2);
                    flipped += 1;
                }
                queue.push_back(neighbor);
            }
        }
    }
    flipped
}

/// 自交检测：AABB 预筛后的三角形对相交计数（共享顶点的相邻对不计）。
fn count_self_intersections(mesh: &TriangleMesh) -> usize {
    #[derive(Clone, Copy)]
    struct Box3 {
        min: [f64; 3],
        max: [f64; 3],
    }

    let boxes: Vec<(Box3, [[f64; 3]; 3])> = mesh
        .triangles
        .iter()
        .map(|t| {
            let mut min = t.a;
            let mut max = t.a;
            for vertex in [&t.b, &t.c] {
                for axis in 0..3 {
                    min[axis] = min[axis].min(vertex[axis]);
                    max[axis] = max[axis].max(vertex[axis]);
                }
            }
            (Box3 { min, max }, [t.a, t.b, t.c])
        })
        .collect();

    fn overlap(a: &Box3, b: &Box3) -> bool {
        (0..3).all(|axis| a.min[axis] <= b.max[axis] && b.min[axis] <= a.max[axis])
    }

    /// 三角形 - 三角形相交（Möller 区间法简化版）。
    fn triangles_intersect(
        p0: &[f64; 3],
        p1: &[f64; 3],
        p2: &[f64; 3],
        q0: &[f64; 3],
        q1: &[f64; 3],
        q2: &[f64; 3],
    ) -> bool {
        fn sub(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
            [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
        }
        fn cross(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        }
        fn dot(a: &[f64; 3], b: &[f64; 3]) -> f64 {
            a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
        }
        // P 三顶点相对 Q 平面的带符号距离。
        let q_normal = cross(&sub(q1, q0), &sub(q2, q0));
        let distances = [
            dot(&q_normal, &sub(p0, q0)),
            dot(&q_normal, &sub(p1, q0)),
            dot(&q_normal, &sub(p2, q0)),
        ];
        if distances.iter().all(|d| *d > 0.0) || distances.iter().all(|d| *d < 0.0) {
            return false;
        }
        let p_normal = cross(&sub(p1, p0), &sub(p2, p0));
        let p_distances = [
            dot(&p_normal, &sub(q0, p0)),
            dot(&p_normal, &sub(q1, p0)),
            dot(&p_normal, &sub(q2, p0)),
        ];
        if p_distances.iter().all(|d| *d > 0.0) || p_distances.iter().all(|d| *d < 0.0) {
            return false;
        }
        // 两三角形分别与相交线构成区间；区间在共享直线上投影重叠即相交。
        let direction = cross(&p_normal, &q_normal);
        fn interval(
            triangle: &[[f64; 3]; 3],
            distances: &[f64; 3],
            direction: &[f64; 3],
        ) -> (f64, f64) {
            let projection = |vertex: &[f64; 3]| dot(direction, vertex);
            let mut low = f64::INFINITY;
            let mut high = f64::NEG_INFINITY;
            for (index, vertex) in triangle.iter().enumerate() {
                // 只取跨平面边的端点投影，得到与相交线的交点区间。
                let other = (index + 1) % 3;
                let d0 = distances[index];
                let d1 = distances[other];
                if (d0 > 0.0) != (d1 > 0.0) || d0 == 0.0 {
                    let t = d0 / (d0 - d1);
                    let point = projection(&[
                        vertex[0] + (triangle[other][0] - vertex[0]) * t,
                        vertex[1] + (triangle[other][1] - vertex[1]) * t,
                        vertex[2] + (triangle[other][2] - vertex[2]) * t,
                    ]);
                    low = low.min(point);
                    high = high.max(point);
                }
            }
            (low, high)
        }
        let p_triangle = [*p0, *p1, *p2];
        let q_triangle = [*q0, *q1, *q2];
        let (pa, pb) = interval(&p_triangle, &distances, &direction);
        let (qa, qb) = interval(&q_triangle, &p_distances, &direction);
        pa <= qb && qa <= pb
    }

    let mut count = 0usize;
    for i in 0..boxes.len() {
        for j in (i + 1)..boxes.len() {
            let (box_a, tri_a) = &boxes[i];
            let (box_b, tri_b) = &boxes[j];
            if !overlap(box_a, box_b) {
                continue;
            }
            let shared = tri_a
                .iter()
                .any(|vertex| tri_b.iter().any(|other| vertex == other));
            if shared {
                continue;
            }
            if triangles_intersect(
                &tri_a[0], &tri_a[1], &tri_a[2], &tri_b[0], &tri_b[1], &tri_b[2],
            ) {
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::geometry::TriangleMesh;
    use crate::services::geometry::check_mesh;

    fn tri(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Triangle {
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        Triangle { a, b, c, normal }
    }

    #[test]
    fn clean_box_repair_is_identity() {
        let mesh = TriangleMesh::sample_box(10.0);
        let (repaired, report) = repair_mesh(&mesh).unwrap();
        for (index, t) in mesh.triangles.iter().enumerate() {
            println!("FACE {index}: {:?} -> {:?} -> {:?}", t.a, t.b, t.c);
        }
        println!(
            "DEBUG clean_box flipped={} issues={:?} tris={}",
            report.flipped_faces,
            check_mesh(&repaired),
            repaired.triangle_count()
        );
        assert_eq!(report.filled_holes, 0);
        assert_eq!(report.self_intersections, 0);
        assert_eq!(repaired.triangle_count(), mesh.triangle_count());
        assert!(check_mesh(&repaired).is_clean());
    }

    #[test]
    fn missing_face_is_filled_and_orientation_restored() {
        let mut mesh = TriangleMesh::sample_box(10.0);
        // 丢掉一个三角形制造孔洞，并把另一个面翻转。
        mesh.triangles.pop();
        mesh.triangles.last_mut().unwrap().b.swap(0, 2);
        let (repaired, report) = repair_mesh(&mesh).unwrap();
        println!(
            "DEBUG repaired_tris={} report={:?}",
            repaired.triangle_count(),
            report
        );
        assert!(report.flipped_faces >= 1);
        let issues = check_mesh(&repaired);
        assert_eq!(issues.open_edges, 0, "补面后不应有边界边：{issues:?}");
        assert_eq!(issues.normal_inconsistent_edges, 0, "{issues:?}");
    }

    #[test]
    fn duplicate_vertices_weld_and_degenerate_faces_drop() {
        // 两个三角形 + 一个零面积退化三角形（a = b）。
        let mut triangles = vec![
            tri([0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]),
            tri([0.0, 0.0, 0.0], [0.0, 10.0, 0.0], [10.0, 0.0, 0.0]),
        ];
        triangles.push(Triangle {
            a: [1.0, 1.0, 0.0],
            b: [1.0, 1.0, 0.0],
            c: [2.0, 1.0, 0.0],
            normal: [0.0, 0.0, 1.0],
        });
        let (repaired, report) = repair_mesh(&TriangleMesh { triangles }).unwrap();
        assert_eq!(report.removed_degenerate, 1);
        assert_eq!(repaired.triangle_count(), 2);
    }

    #[test]
    fn crossing_triangles_are_counted_as_self_intersection() {
        // 直接测检测函数：repair_mesh 会先填孔，交叉对外轮廓会被扇形补面。
        let triangles = vec![
            tri([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]),
            tri([0.5, 0.5, -1.0], [0.5, 0.5, 1.0], [2.0, 0.5, 0.0]),
        ];
        assert_eq!(count_self_intersections(&TriangleMesh { triangles }), 1);
    }

    #[test]
    fn face_normal_falls_back_to_z_axis_for_zero_area_face() {
        let weld: HashMap<[i64; 3], [f64; 3]> = HashMap::from([
            ([0, 0, 0], [0.0, 0.0, 0.0]),
            ([1, 0, 0], [1.0, 0.0, 0.0]),
            ([2, 0, 0], [2.0, 0.0, 0.0]),
        ]);
        // 三点共线叉积为零，法向回退 +Z。
        assert_eq!(
            face_normal(&[0, 0, 0], &[1, 0, 0], &[2, 0, 0], &weld),
            [0.0, 0.0, 1.0]
        );
    }

    #[test]
    fn fan_boundary_abandons_fill_and_stays_boundary() {
        // 三个三角形共享同一条边：边界图在共享顶点处邻接数为 3 ≠ 2，
        // 任何环行走都必然在共享顶点处断掉——孔洞放弃填充（结果与
        // boundary_neighbors 的迭代顺序无关），边界边保留到法向一致化。
        let mesh = TriangleMesh {
            triangles: vec![
                tri([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
                tri([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
                tri([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]),
            ],
        };
        let (repaired, report) = repair_mesh(&mesh).unwrap();
        assert_eq!(report.filled_holes, 0);
        assert_eq!(repaired.triangle_count(), 3);
    }

    #[test]
    fn coplanar_separated_triangles_short_circuit_on_first_plane() {
        // p 整体在 q 平面（x+y+z=1）下侧：第一层平面剔除直接判不相交。
        let triangles = vec![
            tri([0.0, 0.0, 0.0], [0.8, 0.0, 0.0], [0.0, 0.8, 0.0]),
            tri([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ];
        assert_eq!(count_self_intersections(&TriangleMesh { triangles }), 0);
    }

    #[test]
    fn perpendicularly_arranged_triangles_short_circuit_on_second_plane() {
        // q 穿过 p 所在平面（第一层剔除不成立），但 p 整体在 q 平面
        // （x+y=1）一侧：第二层平面剔除判不相交。
        let triangles = vec![
            tri([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 3.0]),
            tri([0.0, 1.0, 2.0], [1.0, 0.0, 2.0], [0.5, 0.5, 2.5]),
        ];
        assert_eq!(count_self_intersections(&TriangleMesh { triangles }), 0);
    }

    #[test]
    fn empty_mesh_is_rejected() {
        let error = repair_mesh(&TriangleMesh::default()).unwrap_err();
        assert!(error.to_string().contains("无法修复"));
    }
}
