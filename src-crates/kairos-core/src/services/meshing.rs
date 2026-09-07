//! 网格化服务：体素占用 + 立方体 5-四面体保形分解。
//!
//! v1 策略（对接 openInjMoldSim 的 3D 求解）：
//! 1. 沿 x 轴做射线奇偶测试标记体内体素（扫描线按行求交，复杂度 O(行数 × 三角形)）；
//! 2. 每个体素按角点奇偶性选择「偶角点」或「奇角点」四面体 + 体心切成 5 个四面体，
//!    相邻体素共享面上的对角线自动一致（标准保形分解）；
//! 3. 只出现一次的四面体面即边界面。
//!
//! 局部加密与边界层尚未实现（见任务 T06 说明）。

use std::collections::HashMap;

use crate::error::{KairosError, Result};
use crate::models::geometry::TriangleMesh;
use crate::models::mesh::{MeshQuality, MeshingReport, VolumeMesh};

/// 单轴最大体素数与总体素上限：防御性上限，避免误填尺寸导致内存爆炸。
const MAX_CELLS_PER_AXIS: usize = 200;
const MAX_TOTAL_CELLS: usize = 2_000_000;

/// 网格化参数（v1）：目标体素尺寸（与几何同单位）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeMeshParams {
    pub target_size: f64,
}

impl VolumeMeshParams {
    pub fn validate(&self) -> Result<()> {
        if !self.target_size.is_finite() || self.target_size <= 0.0 {
            return Err(KairosError::validation("目标网格尺寸必须为正数。"));
        }
        Ok(())
    }
}

/// 由三角网格生成体积网格。
pub fn generate(mesh: &TriangleMesh, params: &VolumeMeshParams) -> Result<VolumeMesh> {
    params.validate()?;
    let (min, max) = mesh.bounding_box();
    let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if mesh.triangles.is_empty() || size.iter().any(|s| !s.is_finite() || *s <= 0.0) {
        return Err(KairosError::validation(
            "几何为空或包围盒退化，无法划分网格。",
        ));
    }
    let h = params.target_size;
    let counts: Vec<usize> = size
        .iter()
        .map(|s| (*s / h).ceil() as usize)
        .map(|n| n.clamp(1, MAX_CELLS_PER_AXIS))
        .collect();
    if counts[0] * counts[1] * counts[2] > MAX_TOTAL_CELLS {
        return Err(KairosError::validation(format!(
            "目标尺寸过小：体素数超过上限 {MAX_TOTAL_CELLS}，请调大目标网格尺寸。"
        )));
    }
    // 夹取后的实际步长（各轴独立，保证覆盖整个包围盒）。
    let hs = [
        size[0] / counts[0] as f64,
        size[1] / counts[1] as f64,
        size[2] / counts[2] as f64,
    ];

    // 扫描线：对每一行 (y, z) 求射线交点并标记内部体素。
    let mut inside: Vec<Vec<bool>> = Vec::with_capacity(counts[1] * counts[2]);
    for j in 0..counts[1] {
        for k in 0..counts[2] {
            // 轻微抖动 y，避免射线恰好穿过三角形边的歧义。
            let jitter = (k as f64 + 0.5) * hs[1] * 1e-6;
            let y = min[1] + (j as f64 + 0.5) * hs[1] + jitter;
            let z = min[2] + (k as f64 + 0.5) * hs[2];
            let crossings = row_crossings(mesh, y, z);
            inside.push(fill_row(&crossings, min[0], hs[0], counts[0]));
        }
    }

    let mut builder = TetBuilder::new(min, hs);
    // 行主序展开：cell_index = i + nx × (k + nz × j)
    for (row_index, row) in inside.iter().enumerate() {
        let j = row_index / counts[2];
        let k = row_index % counts[2];
        for (i, &cell_inside) in row.iter().enumerate() {
            if cell_inside {
                builder.tessellate_cell(i, j, k);
            }
        }
    }

    let volume_mesh = builder.finish();
    if volume_mesh.tets.is_empty() {
        return Err(KairosError::validation(
            "未生成任何体素：目标尺寸过大或几何过小，请调小目标网格尺寸。",
        ));
    }
    Ok(volume_mesh)
}

/// 体素 → 5 四面体的构建器：节点按网格角点 / 体心去重。
struct TetBuilder {
    min: [f64; 3],
    hs: [f64; 3],
    node_map: HashMap<(usize, usize, usize), usize>,
    nodes: Vec<[f64; 3]>,
    tets: Vec<[usize; 4]>,
}

impl TetBuilder {
    fn new(min: [f64; 3], hs: [f64; 3]) -> Self {
        Self {
            min,
            hs,
            node_map: HashMap::new(),
            nodes: Vec::new(),
            tets: Vec::new(),
        }
    }

    fn node(&mut self, i: usize, j: usize, k: usize) -> usize {
        *self.node_map.entry((i, j, k)).or_insert_with(|| {
            self.nodes.push([
                self.min[0] + i as f64 * self.hs[0],
                self.min[1] + j as f64 * self.hs[1],
                self.min[2] + k as f64 * self.hs[2],
            ]);
            self.nodes.len() - 1
        })
    }

    /// 一个体素切成 5 个四面体（标准保形分解，无体心节点）：
    /// - 偶体素：A = 4 个偶奇偶性角点 {000,110,101,011} 的四面体，
    ///   加 4 个「B 角点 + 相邻 3 个 A 角点」四面体；
    /// - 奇体素：A/B 角点集合互换。
    ///
    /// 相邻体素共享面上的对角线由奇偶性自动对齐，保证保形。
    fn tessellate_cell(&mut self, i: usize, j: usize, k: usize) {
        let even = (i + j + k).is_multiple_of(2);
        if even {
            let a0 = self.node(i, j, k);
            let a1 = self.node(i + 1, j + 1, k);
            let a2 = self.node(i + 1, j, k + 1);
            let a3 = self.node(i, j + 1, k + 1);
            let b0 = self.node(i + 1, j, k);
            let b1 = self.node(i, j + 1, k);
            let b2 = self.node(i, j, k + 1);
            let b3 = self.node(i + 1, j + 1, k + 1);
            for tet in [
                [a0, a1, a2, a3],
                [b0, a0, a1, a2],
                [b1, a0, a1, a3],
                [b2, a0, a2, a3],
                [b3, a1, a2, a3],
            ] {
                let mut tet = tet;
                ensure_positive(&mut tet, &self.nodes);
                self.tets.push(tet);
            }
        } else {
            let a0 = self.node(i + 1, j, k);
            let a1 = self.node(i, j + 1, k);
            let a2 = self.node(i, j, k + 1);
            let a3 = self.node(i + 1, j + 1, k + 1);
            let b0 = self.node(i, j, k);
            let b1 = self.node(i + 1, j + 1, k);
            let b2 = self.node(i + 1, j, k + 1);
            let b3 = self.node(i, j + 1, k + 1);
            for tet in [
                [a0, a1, a2, a3],
                [b0, a0, a1, a2],
                [b1, a0, a1, a3],
                [b2, a0, a2, a3],
                [b3, a1, a2, a3],
            ] {
                let mut tet = tet;
                ensure_positive(&mut tet, &self.nodes);
                self.tets.push(tet);
            }
        }
    }

    fn finish(self) -> VolumeMesh {
        let surface_faces = extract_surface(&self.tets);
        VolumeMesh {
            nodes: self.nodes,
            tets: self.tets,
            surface_faces,
        }
    }
}

/// 保证四面体绕向为正体积（行列式 < 0 时交换两点）。
fn ensure_positive(tet: &mut [usize; 4], nodes: &[[f64; 3]]) {
    let (a, b, c, d) = (nodes[tet[0]], nodes[tet[1]], nodes[tet[2]], nodes[tet[3]]);
    let det = dot3(&cross3(&sub(&b, &a), &sub(&c, &a)), &sub(&d, &a));
    if det < 0.0 {
        tet.swap(2, 3);
    }
}

fn cross3(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// 提取只被一个四面体使用的面（边界面）。
fn extract_surface(tets: &[[usize; 4]]) -> Vec<[usize; 3]> {
    let mut face_count: HashMap<[usize; 3], usize> = HashMap::new();
    for tet in tets {
        for face in [
            [tet[0], tet[1], tet[2]],
            [tet[0], tet[1], tet[3]],
            [tet[0], tet[2], tet[3]],
            [tet[1], tet[2], tet[3]],
        ] {
            let mut key = face;
            key.sort_unstable();
            *face_count.entry(key).or_insert(0) += 1;
        }
    }
    let mut surface: Vec<[usize; 3]> = face_count
        .into_iter()
        .filter_map(|(key, count)| (count == 1).then_some(key))
        .collect();
    surface.sort_unstable();
    surface
}

/// 计算一行 (y, z) 上射线 +x 与所有三角形的交点 x 坐标（升序）。
fn row_crossings(mesh: &TriangleMesh, y: f64, z: f64) -> Vec<f64> {
    let mut crossings = Vec::new();
    for triangle in &mesh.triangles {
        let (v0, v1, v2) = (triangle.a, triangle.b, triangle.c);
        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
        let det = e1[1] * e2[2] - e1[2] * e2[1];
        if det.abs() < 1e-12 {
            continue;
        }
        let dy = y - v0[1];
        let dz = z - v0[2];
        let a = (dy * e2[2] - dz * e2[1]) / det;
        let b = (e1[1] * dz - e1[2] * dy) / det;
        if a < 0.0 || b < 0.0 || a + b > 1.0 {
            continue;
        }
        // 保留全部交点（含 x=x_origin 的边界交点），奇偶填充按数量判断。
        crossings.push(v0[0] + a * e1[0] + b * e2[0]);
    }
    crossings.sort_by(|a, b| a.total_cmp(b));
    crossings
}

/// 按奇偶规则把交点区间翻译成该行的内部体素标记。
fn fill_row(crossings: &[f64], x_min: f64, h: f64, count: usize) -> Vec<bool> {
    let mut row = vec![false; count];
    let mut inside = false;
    let mut next = 0usize;
    for (i, row_cell) in row.iter_mut().enumerate() {
        let center = x_min + (i as f64 + 0.5) * h;
        while next < crossings.len() && crossings[next] < center {
            next += 1;
            inside = !inside;
        }
        *row_cell = inside;
    }
    row
}

/// 网格统计报告。
pub fn report(volume_mesh: &VolumeMesh) -> MeshingReport {
    let mut min_ratio = f64::INFINITY;
    let mut max_ratio = 0.0f64;
    let mut ratio_sum = 0.0f64;
    let mut min_volume = f64::INFINITY;
    let mut total_volume = 0.0f64;

    for tet in &volume_mesh.tets {
        let p: Vec<[f64; 3]> = tet.iter().map(|&i| volume_mesh.nodes[i]).collect();
        let mut longest = 0.0f64;
        let mut shortest = f64::INFINITY;
        for (i, j) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
            let d = sub(&p[i], &p[j]);
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            longest = longest.max(len);
            shortest = shortest.min(len);
        }
        let ratio = longest / shortest.max(1e-15);
        let volume = dot3(
            &cross3(&sub(&p[1], &p[0]), &sub(&p[2], &p[0])),
            &sub(&p[3], &p[0]),
        )
        .abs()
            / 6.0;
        min_ratio = min_ratio.min(ratio);
        max_ratio = max_ratio.max(ratio);
        ratio_sum += ratio;
        min_volume = min_volume.min(volume);
        total_volume += volume;
    }

    let count = volume_mesh.tets.len();
    let quality = MeshQuality {
        min_edge_ratio: if count > 0 { min_ratio } else { 0.0 },
        avg_edge_ratio: if count > 0 {
            ratio_sum / count as f64
        } else {
            0.0
        },
        max_edge_ratio: max_ratio,
        min_volume: if count > 0 { min_volume } else { 0.0 },
    };
    MeshingReport {
        node_count: volume_mesh.nodes.len(),
        element_count: volume_mesh.tets.len(),
        surface_face_count: volume_mesh.surface_faces.len(),
        total_volume,
        quality,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::geometry::Triangle;

    fn unit_cube_mesh() -> TriangleMesh {
        let mut triangles = Vec::new();
        // (起始角点, u, v)——外法向绕向，与 meshing 相同的构造方式。
        let quads = [
            ((0.0, 0.0, 1.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)),
            ((0.0, 1.0, 0.0), (1.0, 0.0, 0.0), (0.0, -1.0, 0.0)),
            ((1.0, 0.0, 1.0), (0.0, 0.0, -1.0), (0.0, 1.0, 0.0)),
            ((0.0, 0.0, 0.0), (0.0, 0.0, 1.0), (0.0, 1.0, 0.0)),
            ((0.0, 1.0, 0.0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0)),
            ((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 0.0, 1.0)),
        ];
        for (c0, u, v) in quads {
            let p0 = c0;
            let p1 = (c0.0 + u.0, c0.1 + u.1, c0.2 + u.2);
            let p2 = (c0.0 + u.0 + v.0, c0.1 + u.1 + v.1, c0.2 + u.2 + v.2);
            let p3 = (c0.0 + v.0, c0.1 + v.1, c0.2 + v.2);
            let as_array = |p: (f64, f64, f64)| [p.0, p.1, p.2];
            triangles.push(Triangle {
                a: as_array(p0),
                b: as_array(p1),
                c: as_array(p2),
                normal: [0.0; 3],
            });
            triangles.push(Triangle {
                a: as_array(p0),
                b: as_array(p2),
                c: as_array(p3),
                normal: [0.0; 3],
            });
        }
        TriangleMesh { triangles }
    }

    #[test]
    fn params_must_be_positive() {
        assert!(VolumeMeshParams { target_size: 0.0 }.validate().is_err());
        assert!(VolumeMeshParams { target_size: -1.0 }.validate().is_err());
        assert!(VolumeMeshParams { target_size: 0.25 }.validate().is_ok());
    }

    #[test]
    fn unit_cube_voxelize_volume_and_conformity() {
        let volume = generate(&unit_cube_mesh(), &VolumeMeshParams { target_size: 0.25 }).unwrap();
        let report = report(&volume);
        // 4×4×4 全内部：体积精确等于 1
        assert!(
            (report.total_volume - 1.0).abs() < 1e-9,
            "total={}",
            report.total_volume
        );
        assert_eq!(report.element_count, 4 * 4 * 4 * 5);
        // 边界面：6 面 × 16 方块 × 2 三角形
        assert_eq!(report.surface_face_count, 192);
        // 角点四面体为等边比（最长边/最短边 = √2h/√2h = 1），是理论上限质量
        assert!(report.quality.min_edge_ratio >= 1.0);
        assert!(report.quality.min_volume > 0.0);
    }

    #[test]
    fn tet_faces_are_conforming() {
        let volume = generate(&unit_cube_mesh(), &VolumeMeshParams { target_size: 0.34 }).unwrap();
        let mut face_count: HashMap<[usize; 3], usize> = HashMap::new();
        for tet in &volume.tets {
            for face in [
                [tet[0], tet[1], tet[2]],
                [tet[0], tet[1], tet[3]],
                [tet[0], tet[2], tet[3]],
                [tet[1], tet[2], tet[3]],
            ] {
                let mut key = face;
                key.sort_unstable();
                *face_count.entry(key).or_insert(0) += 1;
            }
        }
        // 保形网格中不存在被 3 个以上四面体共享的面
        assert!(face_count.values().all(|&count| count <= 2));
    }

    #[test]
    fn rejects_degenerate_bounds_and_huge_grids() {
        let empty = TriangleMesh::default();
        assert!(generate(&empty, &VolumeMeshParams { target_size: 0.1 }).is_err());
        let mesh = unit_cube_mesh();
        assert!(generate(&mesh, &VolumeMeshParams { target_size: 0.0 }).is_err());
        // 目标尺寸极小 → 触发总体素上限错误，不 panic、不爆内存
        assert!(generate(&mesh, &VolumeMeshParams { target_size: 1e-9 }).is_err());
    }

    #[test]
    fn report_counts_nodes_and_surface() {
        let volume = generate(&unit_cube_mesh(), &VolumeMeshParams { target_size: 0.5 }).unwrap();
        let report = report(&volume);
        // 纯角点 5-四面体分解：2×2×2 体素使用全部 3³ = 27 个网格角点
        assert_eq!(report.node_count, 27);
        assert_eq!(report.element_count, 2 * 2 * 2 * 5);
        assert!(
            (report.total_volume - 1.0).abs() < 1e-9,
            "total={}",
            report.total_volume
        );
    }
}
