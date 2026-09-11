//! 双域网格服务：表面三角形厚度配对 + 流道 / 浇口梁单元耦合。
//!
//! 厚度：三角形质心沿法向两个方向发射射线，取最近命中距离——
//! 双向取近可容忍法向不一致的导入网格。射线求交用均匀格加速
//! （3D DDA 遍历），复杂度近似 O(三角形 × 每射线穿越格数)。
//! 杆系：梁端点先按捕捉容差吸附最近表面节点成耦合，吸附失败则
//! 按焊接容差去重后自成为自由节点（未耦合端点在报告中计数）。

use std::collections::HashMap;

use crate::error::{KairosError, Result};
use crate::models::geometry::TriangleMesh;
use crate::models::mesh::{BeamCoupling, DualDomainBeam, DualDomainMesh, DualDomainReport};
use crate::models::runners::RunnerElement;

type Point = [f64; 3];

/// 双域网格参数。
#[derive(Debug, Clone, Copy, Default)]
pub struct DualDomainParams {
    /// 梁端点捕捉表面节点的最大距离；None 按包围盒对角线 1e-3 自动取值。
    pub snap_tolerance: Option<f64>,
}

impl DualDomainParams {
    pub fn validate(&self) -> Result<()> {
        if let Some(tol) = self.snap_tolerance
            && (!tol.is_finite() || tol < 0.0)
        {
            return Err(KairosError::validation("捕捉容差必须为非负有限数。"));
        }
        Ok(())
    }
}

/// 由表面网格与杆系生成双域网格。
pub fn generate(
    mesh: &TriangleMesh,
    runners: &[RunnerElement],
    params: &DualDomainParams,
) -> Result<DualDomainMesh> {
    params.validate()?;
    let (min, max) = mesh.bounding_box();
    let diagonal = mesh.diagonal();
    if mesh.triangles.is_empty() || diagonal <= 0.0 || !diagonal.is_finite() {
        return Err(KairosError::validation(
            "几何为空或包围盒退化，无法生成双域网格。",
        ));
    }

    // 顶点焊接（与网格健康检查同一容差口径），剔除零面积与焊接退化三角形。
    let weld_tolerance = diagonal * 1e-6;
    let mut welds: HashMap<[i64; 3], usize> = HashMap::new();
    let mut nodes: Vec<Point> = Vec::new();
    let weld =
        |point: Point, welds: &mut HashMap<[i64; 3], usize>, nodes: &mut Vec<Point>| -> usize {
            let key: [i64; 3] = point.map(|value| (value / weld_tolerance).round() as i64);
            *welds.entry(key).or_insert_with(|| {
                nodes.push(point);
                nodes.len() - 1
            })
        };
    let mut triangles: Vec<[usize; 3]> = Vec::with_capacity(mesh.triangles.len());
    let degenerate_limit = diagonal * 1e-12;
    for triangle in &mesh.triangles {
        if triangle.area() < degenerate_limit {
            continue;
        }
        let indices = [
            weld(triangle.a, &mut welds, &mut nodes),
            weld(triangle.b, &mut welds, &mut nodes),
            weld(triangle.c, &mut welds, &mut nodes),
        ];
        if indices[0] != indices[1] && indices[1] != indices[2] && indices[0] != indices[2] {
            triangles.push(indices);
        }
    }
    if triangles.is_empty() {
        return Err(KairosError::validation(
            "焊接后没有有效三角形，无法生成双域网格。",
        ));
    }

    // 厚度配对：质心沿 ±法向的最近命中，双向取近。
    let surface_nodes = nodes.len();
    let grid = TriangleGrid::new(&nodes, &triangles, min, max);
    let mut thickness = Vec::with_capacity(triangles.len());
    for indices in &triangles {
        let (a, b, c) = (nodes[indices[0]], nodes[indices[1]], nodes[indices[2]]);
        let centroid = [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ];
        let normal = normalized_normal(&a, &b, &c);
        let up = normal.and_then(|n| grid.first_hit(&nodes, &triangles, centroid, n, diagonal));
        let down = normal.and_then(|n| {
            grid.first_hit(
                &nodes,
                &triangles,
                centroid,
                [-n[0], -n[1], -n[2]],
                diagonal,
            )
        });
        thickness.push(match (up, down) {
            (Some(d1), Some(d2)) => d1.min(d2),
            (Some(d), None) | (None, Some(d)) => d,
            (None, None) => 0.0,
        });
    }

    // 杆系耦合：梁端点捕捉最近表面节点，失败则按焊接容差去重自成为节点。
    let snap_tolerance = params.snap_tolerance.unwrap_or(diagonal * 1e-3);
    let mut couplings = Vec::new();
    let mut beams = Vec::with_capacity(runners.len());
    for (index, runner) in runners.iter().enumerate() {
        let mut endpoints = [0usize; 2];
        for (slot, point) in [(0usize, runner.start), (1usize, runner.end)] {
            if let Some((node, distance)) = nearest_node(&nodes[..surface_nodes], point)
                && distance <= snap_tolerance
            {
                endpoints[slot] = node;
                couplings.push(BeamCoupling {
                    beam: index,
                    endpoint: slot,
                    node,
                    distance,
                });
                continue;
            }
            endpoints[slot] = weld(point, &mut welds, &mut nodes);
        }
        beams.push(DualDomainBeam {
            nodes: endpoints,
            diameter: runner.diameter_mm,
            kind: runner.kind,
        });
    }

    Ok(DualDomainMesh {
        nodes,
        triangles,
        thickness,
        beams,
        couplings,
    })
}

/// 双域网格统计报告。
pub fn report(mesh: &DualDomainMesh) -> DualDomainReport {
    let mut min = f64::INFINITY;
    let mut max = 0.0f64;
    let mut sum = 0.0f64;
    let mut paired = 0usize;
    for value in &mesh.thickness {
        if *value > 0.0 {
            paired += 1;
            min = min.min(*value);
            max = max.max(*value);
            sum += value;
        }
    }
    DualDomainReport {
        node_count: mesh.nodes.len(),
        triangle_count: mesh.triangles.len(),
        beam_count: mesh.beams.len(),
        coupling_count: mesh.couplings.len(),
        uncoupled_endpoints: mesh.beams.len() * 2 - mesh.couplings.len(),
        unpaired_triangles: mesh.thickness.len() - paired,
        thickness_min: if paired > 0 { min } else { 0.0 },
        thickness_max: max,
        thickness_avg: if paired > 0 { sum / paired as f64 } else { 0.0 },
    }
}

/// 单位化三角形法向；退化（零长度叉积）返回 None。
fn normalized_normal(a: &Point, b: &Point, c: &Point) -> Option<Point> {
    let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    (length > 1e-12).then(|| n.map(|value| value / length))
}

fn distance_sq(a: Point, b: Point) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

/// 最近节点线性搜索（杆系数量级小，表面节点一次扫描可接受）。
fn nearest_node(nodes: &[Point], point: Point) -> Option<(usize, f64)> {
    let mut best: Option<(usize, f64)> = None;
    for (index, candidate) in nodes.iter().enumerate() {
        let distance = distance_sq(*candidate, point);
        if best.is_none_or(|(_, current)| distance < current) {
            best = Some((index, distance));
        }
    }
    best.map(|(index, squared)| (index, squared.sqrt()))
}

/// Möller–Trumbore 射线 / 三角形求交，返回参数 t（> 0）。
fn ray_triangle(origin: Point, dir: Point, a: Point, b: Point, c: Point) -> Option<f64> {
    let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let p = [
        dir[1] * e2[2] - dir[2] * e2[1],
        dir[2] * e2[0] - dir[0] * e2[2],
        dir[0] * e2[1] - dir[1] * e2[0],
    ];
    let det = e1[0] * p[0] + e1[1] * p[1] + e1[2] * p[2];
    if det.abs() < 1e-15 {
        return None;
    }
    let inverse = 1.0 / det;
    let s = [origin[0] - a[0], origin[1] - a[1], origin[2] - a[2]];
    let u = (s[0] * p[0] + s[1] * p[1] + s[2] * p[2]) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = [
        s[1] * e1[2] - s[2] * e1[1],
        s[2] * e1[0] - s[0] * e1[2],
        s[0] * e1[1] - s[1] * e1[0],
    ];
    let v = (dir[0] * q[0] + dir[1] * q[1] + dir[2] * q[2]) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = (e2[0] * q[0] + e2[1] * q[1] + e2[2] * q[2]) * inverse;
    (t > 0.0).then_some(t)
}

/// 均匀格加速结构：三角形按包围盒落入格桶，射线用 3D DDA 逐格遍历。
struct TriangleGrid {
    origin: Point,
    cell: f64,
    dims: [i64; 3],
    cells: HashMap<(i64, i64, i64), Vec<u32>>,
}

impl TriangleGrid {
    /// 格数目标 64³：兼顾桶大小与遍历步数。
    fn new(nodes: &[Point], triangles: &[[usize; 3]], min: Point, max: Point) -> Self {
        let extent = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
        let diagonal = (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2])
            .sqrt()
            .max(1e-12);
        let cell = (diagonal / 64.0).max(1e-12);
        let dims: [i64; 3] =
            std::array::from_fn(|axis| ((extent[axis] / cell).floor() as i64 + 1).max(1));
        let mut cells: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
        for (index, indices) in triangles.iter().enumerate() {
            let (tri_min, tri_max) =
                indices
                    .iter()
                    .fold(([f64::MAX; 3], [f64::MIN; 3]), |mut bound, &node| {
                        for (axis, value) in nodes[node].iter().enumerate() {
                            bound.0[axis] = bound.0[axis].min(*value);
                            bound.1[axis] = bound.1[axis].max(*value);
                        }
                        bound
                    });
            let low: [i64; 3] = std::array::from_fn(|axis| {
                (((tri_min[axis] - min[axis]) / cell).floor() as i64).clamp(0, dims[axis] - 1)
            });
            let high: [i64; 3] = std::array::from_fn(|axis| {
                (((tri_max[axis] - min[axis]) / cell).floor() as i64).clamp(0, dims[axis] - 1)
            });
            for i in low[0]..=high[0] {
                for j in low[1]..=high[1] {
                    for k in low[2]..=high[2] {
                        cells.entry((i, j, k)).or_default().push(index as u32);
                    }
                }
            }
        }
        Self {
            origin: min,
            cell,
            dims,
            cells,
        }
    }

    /// 沿单位方向 dir 的最近命中距离；下一格入口距离超过当前最优命中时提前终止。
    fn first_hit(
        &self,
        nodes: &[Point],
        triangles: &[[usize; 3]],
        origin: Point,
        dir: Point,
        diagonal: f64,
    ) -> Option<f64> {
        // 起点位于自身三角形平面上的邻接面命中按此下限过滤。
        let min_hit = diagonal * 1e-9;
        let local: [f64; 3] = std::array::from_fn(|axis| origin[axis] - self.origin[axis]);
        let mut current: [i64; 3] = std::array::from_fn(|axis| {
            (local[axis] / self.cell)
                .floor()
                .clamp(0.0, (self.dims[axis] - 1) as f64) as i64
        });
        let mut step = [0i64; 3];
        let mut t_max = [f64::INFINITY; 3];
        let mut t_delta = [f64::INFINITY; 3];
        for axis in 0..3 {
            if dir[axis] > 0.0 {
                step[axis] = 1;
                t_max[axis] = ((current[axis] as f64 + 1.0) * self.cell - local[axis]) / dir[axis];
                t_delta[axis] = self.cell / dir[axis];
            } else if dir[axis] < 0.0 {
                step[axis] = -1;
                t_max[axis] = (current[axis] as f64 * self.cell - local[axis]) / dir[axis];
                t_delta[axis] = -self.cell / dir[axis];
            }
        }
        let mut best: Option<f64> = None;
        loop {
            if let Some(candidates) = self.cells.get(&(current[0], current[1], current[2])) {
                for &index in candidates {
                    let indices = &triangles[index as usize];
                    if let Some(t) = ray_triangle(
                        origin,
                        dir,
                        nodes[indices[0]],
                        nodes[indices[1]],
                        nodes[indices[2]],
                    ) && t >= min_hit
                        && best.is_none_or(|current_best| t < current_best)
                    {
                        best = Some(t);
                    }
                }
            }
            let next_axis = (0..3)
                .min_by(|&x, &y| t_max[x].total_cmp(&t_max[y]))
                .expect("轴数固定为 3");
            if t_max[next_axis] > best.unwrap_or(f64::INFINITY) {
                break;
            }
            current[next_axis] += step[next_axis];
            if current[next_axis] < 0 || current[next_axis] >= self.dims[next_axis] {
                break;
            }
            t_max[next_axis] += t_delta[next_axis];
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::geometry::Triangle;
    use crate::models::runners::RunnerKind;

    /// quad（外法向绕向）拆成两个三角形。
    fn quad(p0: Point, p1: Point, p2: Point, p3: Point) -> Vec<Triangle> {
        vec![
            Triangle {
                a: p0,
                b: p1,
                c: p2,
                normal: [0.0; 3],
            },
            Triangle {
                a: p0,
                b: p2,
                c: p3,
                normal: [0.0; 3],
            },
        ]
    }

    /// 封闭立方体 [0,size]³ 的 12 个三角形（外法向绕向）。
    fn cube(size: f64) -> TriangleMesh {
        let mut triangles = vec![];
        triangles.extend(quad(
            [0., 0., size],
            [size, 0., size],
            [size, size, size],
            [0., size, size],
        ));
        triangles.extend(quad(
            [0., size, 0.],
            [size, size, 0.],
            [size, 0., 0.],
            [0., 0., 0.],
        ));
        triangles.extend(quad(
            [size, 0., size],
            [size, 0., 0.],
            [size, size, 0.],
            [size, size, size],
        ));
        triangles.extend(quad(
            [0., 0., 0.],
            [0., 0., size],
            [0., size, size],
            [0., size, 0.],
        ));
        triangles.extend(quad(
            [0., size, 0.],
            [0., size, size],
            [size, size, size],
            [size, size, 0.],
        ));
        triangles.extend(quad(
            [0., 0., 0.],
            [size, 0., 0.],
            [size, 0., size],
            [0., 0., size],
        ));
        TriangleMesh { triangles }
    }

    fn runner(start: Point, end: Point) -> RunnerElement {
        RunnerElement {
            id: format!("r-{}", start[0] as i32),
            kind: RunnerKind::Runner,
            diameter_mm: 5.0,
            start,
            end,
        }
    }

    #[test]
    fn params_reject_negative_or_nonfinite_snap_tolerance() {
        assert!(
            DualDomainParams {
                snap_tolerance: Some(-1.0)
            }
            .validate()
            .is_err()
        );
        assert!(
            DualDomainParams {
                snap_tolerance: Some(f64::NAN)
            }
            .validate()
            .is_err()
        );
        assert!(
            DualDomainParams {
                snap_tolerance: Some(0.5)
            }
            .validate()
            .is_ok()
        );
        assert!(
            DualDomainParams {
                snap_tolerance: None
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn degenerate_or_empty_meshes_are_rejected() {
        assert!(generate(&TriangleMesh::default(), &[], &DualDomainParams::default()).is_err());
        // 全部顶点重合：包围盒退化。
        let collapsed = TriangleMesh {
            triangles: vec![Triangle {
                a: [1., 1., 1.],
                b: [1., 1., 1.],
                c: [1., 1., 1.],
                normal: [0.; 3],
            }],
        };
        assert!(generate(&collapsed, &[], &DualDomainParams::default()).is_err());
    }

    #[test]
    fn slab_thickness_is_measured_through_both_sides() {
        // 10×10×2 平板：顶底面三角形配对面厚 ≈ 2；侧壁三角形沿自身法向
        // 只能命中对面侧壁（≈ 10）——这是质心射线配厚的已知边界。
        let mut triangles = vec![];
        triangles.extend(quad(
            [0., 0., 2.],
            [10., 0., 2.],
            [10., 10., 2.],
            [0., 10., 2.],
        ));
        triangles.extend(quad(
            [0., 10., 0.],
            [10., 10., 0.],
            [10., 0., 0.],
            [0., 0., 0.],
        ));
        triangles.extend(quad(
            [10., 0., 2.],
            [10., 0., 0.],
            [10., 10., 0.],
            [10., 10., 2.],
        ));
        triangles.extend(quad(
            [0., 0., 0.],
            [0., 0., 2.],
            [0., 10., 2.],
            [0., 10., 0.],
        ));
        triangles.extend(quad(
            [0., 10., 0.],
            [0., 10., 2.],
            [10., 10., 2.],
            [10., 10., 0.],
        ));
        triangles.extend(quad(
            [0., 0., 0.],
            [10., 0., 0.],
            [10., 0., 2.],
            [0., 0., 2.],
        ));
        let mesh = generate(
            &TriangleMesh { triangles },
            &[],
            &DualDomainParams::default(),
        )
        .unwrap();
        assert_eq!(mesh.triangles.len(), 12);
        for value in &mesh.thickness[..4] {
            assert!((value - 2.0).abs() < 1e-9, "顶底面厚度 {value}");
        }
        for value in &mesh.thickness[4..] {
            assert!((value - 10.0).abs() < 1e-9, "侧壁厚度 {value}");
        }
        let stats = report(&mesh);
        assert_eq!(stats.unpaired_triangles, 0);
        assert!((stats.thickness_min - 2.0).abs() < 1e-9);
        assert!((stats.thickness_max - 10.0).abs() < 1e-9);
        assert_eq!(stats.beam_count, 0);
    }

    #[test]
    fn open_surface_has_unpaired_triangles_and_zero_stats() {
        let mesh = generate(
            &TriangleMesh {
                triangles: vec![Triangle {
                    a: [0., 0., 0.],
                    b: [1., 0., 0.],
                    c: [0., 1., 0.],
                    normal: [0.; 3],
                }],
            },
            &[],
            &DualDomainParams::default(),
        )
        .unwrap();
        assert_eq!(mesh.thickness, vec![0.0]);
        let stats = report(&mesh);
        assert_eq!(stats.unpaired_triangles, 1);
        assert_eq!(stats.thickness_min, 0.0);
        assert_eq!(stats.thickness_max, 0.0);
        assert_eq!(stats.thickness_avg, 0.0);
    }

    #[test]
    fn weld_collapses_shared_vertices_but_keeps_duplicate_faces() {
        // 复制一个三角形（面重复不剔除，属修复工具职责）+ 一个零面积三角形：
        // 节点焊接为 8 个，有效面 12 + 1 重复 = 13。
        let mut mesh = cube(4.0);
        mesh.triangles.push(mesh.triangles[0].clone());
        mesh.triangles.push(Triangle {
            a: [2., 2., 2.],
            b: [2., 2., 2.],
            c: [2., 3., 2.],
            normal: [0.; 3],
        });
        let dual = generate(&mesh, &[], &DualDomainParams::default()).unwrap();
        assert_eq!(dual.triangles.len(), 13);
        assert_eq!(dual.nodes.len(), 8);
        assert_eq!(dual.thickness.len(), 13);
    }

    #[test]
    fn degenerate_faces_only_are_rejected_after_weld() {
        // 全部顶点重合：包围盒退化在焊接之前就拒绝。
        let collapsed = TriangleMesh {
            triangles: vec![Triangle {
                a: [0., 0., 0.],
                b: [0., 0., 0.],
                c: [0., 0., 0.],
                normal: [0.; 3],
            }],
        };
        let error = generate(&collapsed, &[], &DualDomainParams::default()).unwrap_err();
        assert!(error.to_string().contains("包围盒退化"));

        // 共线三角形有包围盒但零面积：焊接阶段全部剔除后明确报错。
        let collinear = TriangleMesh {
            triangles: vec![Triangle {
                a: [0., 0., 0.],
                b: [1., 0., 0.],
                c: [2., 0., 0.],
                normal: [0.; 3],
            }],
        };
        let error = generate(&collinear, &[], &DualDomainParams::default()).unwrap_err();
        assert!(error.to_string().contains("焊接后没有有效三角形"));
    }

    #[test]
    fn runner_endpoints_snap_to_surface_nodes_or_become_free() {
        // 起点 (0,0,0) 恰为表面节点 → 耦合；终点远离表面 → 自由节点。
        let dual = generate(
            &cube(10.0),
            &[runner([0., 0., 0.], [50., 0., 0.])],
            &DualDomainParams::default(),
        )
        .unwrap();
        let corner = dual
            .nodes
            .iter()
            .position(|n| *n == [0.0, 0.0, 0.0])
            .unwrap();
        assert_eq!(dual.beams.len(), 1);
        assert_eq!(dual.beams[0].nodes, [corner, 8]);
        assert_eq!(dual.couplings.len(), 1);
        assert_eq!(dual.couplings[0].node, corner);
        assert_eq!(dual.couplings[0].distance, 0.0);
        let stats = report(&dual);
        assert_eq!(stats.coupling_count, 1);
        assert_eq!(stats.uncoupled_endpoints, 1);
    }

    #[test]
    fn shared_runner_junction_reuses_free_node() {
        // 两条流道共享端点 (50,0,0)：第二条按焊接容差复用第一条的自由节点。
        let dual = generate(
            &cube(10.0),
            &[
                runner([0., 0., 0.], [50., 0., 0.]),
                runner([50., 0., 0.], [40., 0., 0.]),
            ],
            &DualDomainParams::default(),
        )
        .unwrap();
        // 表面 8 节点 + 唯一自由节点 (50,0,0) + 耦合端 (40,0,0) 捕捉失败也成自由节点
        // ——(40,0,0) 在实体外：自由节点；故 8 + 2 = 10。
        assert_eq!(dual.nodes.len(), 10);
        assert_eq!(dual.beams[1].nodes[0], dual.beams[0].nodes[1]);
    }

    #[test]
    fn zero_snap_tolerance_still_couples_exact_hits() {
        let dual = generate(
            &cube(10.0),
            &[runner([0., 0., 0.], [50., 0., 0.])],
            &DualDomainParams {
                snap_tolerance: Some(0.0),
            },
        )
        .unwrap();
        assert_eq!(dual.couplings.len(), 1);
    }

    #[test]
    fn triangle_between_two_walls_takes_the_nearest_hit() {
        // 自由三角形夹在地板与天花板之间：双向都有命中，取最近（地板 3）。
        let mut triangles = vec![];
        triangles.extend(quad(
            [0., 0., 0.],
            [10., 0., 0.],
            [10., 10., 0.],
            [0., 10., 0.],
        ));
        triangles.extend(quad(
            [0., 0., 10.],
            [10., 0., 10.],
            [10., 10., 10.],
            [0., 10., 10.],
        ));
        triangles.push(Triangle {
            a: [4., 4., 3.],
            b: [6., 4., 3.],
            c: [5., 6., 3.],
            normal: [0.; 3],
        });
        let dual = generate(
            &TriangleMesh { triangles },
            &[],
            &DualDomainParams::default(),
        )
        .unwrap();
        assert!((dual.thickness[4] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn grid_ray_cast_matches_brute_force_on_cube() {
        let mesh = cube(8.0);
        let dual = generate(&mesh, &[], &DualDomainParams::default()).unwrap();
        let (min, max) = mesh.bounding_box();
        let grid = TriangleGrid::new(&dual.nodes, &dual.triangles, min, max);
        let probes = [
            ([4., 4., 8.], [0., 0., -1.]),
            ([4., 4., 0.], [0., 0., 1.]),
            ([8., 4., 4.], [-1., 0., 0.]),
            ([0., 4., 4.], [1., 0., 0.]),
            ([4., 0., 4.], [0., 1., 0.]),
            ([4., 8., 4.], [0., -1., 0.]),
            // 平行于某轴（分量为 0）的斜射线与完全脱离网格的射线。
            ([4., 4., 8.], [1., 1., 0.]),
            ([4., 4., 8.], [0., 0., 1.]),
        ];
        for (origin, dir) in probes {
            let grid_hit = grid.first_hit(&dual.nodes, &dual.triangles, origin, dir, 14.0);
            let brute = brute_force(&dual.nodes, &dual.triangles, origin, dir);
            assert_eq!(grid_hit, brute, "网格与暴力求交不一致：{origin:?} {dir:?}");
        }
    }

    /// 暴力全量扫描参照实现。
    fn brute_force(
        nodes: &[Point],
        triangles: &[[usize; 3]],
        origin: Point,
        dir: Point,
    ) -> Option<f64> {
        triangles
            .iter()
            .filter_map(|t| ray_triangle(origin, dir, nodes[t[0]], nodes[t[1]], nodes[t[2]]))
            .fold(None, |best: Option<f64>, t| {
                Some(best.map_or(t, |current: f64| current.min(t)))
            })
    }

    #[test]
    fn large_triangle_spans_multiple_grid_cells() {
        // 覆盖全包围盒的大三角形仍能被射线命中（跨格插入）。
        let mesh = TriangleMesh {
            triangles: vec![Triangle {
                a: [0., 0., 0.],
                b: [60., 0., 0.],
                c: [0., 60., 0.],
                normal: [0.; 3],
            }],
        };
        let dual = generate(&mesh, &[], &DualDomainParams::default()).unwrap();
        let grid = TriangleGrid::new(&dual.nodes, &dual.triangles, [0.; 3], [60., 60., 60.]);
        let hit = grid.first_hit(
            &dual.nodes,
            &dual.triangles,
            [10., 10., 30.],
            [0., 0., -1.],
            100.0,
        );
        assert!((hit.unwrap() - 30.0).abs() < 1e-9);
    }

    #[test]
    fn ray_triangle_rejects_parallel_misses_and_behind_hits() {
        let tri = ([0., 0., 0.], [1., 0., 0.], [0., 1., 0.]);
        // 与三角形平面平行。
        assert!(ray_triangle([0., 0., 1.], [1., 0., 0.], tri.0, tri.1, tri.2).is_none());
        // 命中点在 u 参数外。
        assert!(ray_triangle([5., 5., 1.], [0., 0., -1.], tri.0, tri.1, tri.2).is_none());
        // 命中点在 v 参数外（u+v > 1）。
        assert!(ray_triangle([0.9, 0.9, 1.], [0., 0., -1.], tri.0, tri.1, tri.2).is_none());
        // 三角形在射线反方向。
        assert!(ray_triangle([0.1, 0.1, -1.], [0., 0., -1.], tri.0, tri.1, tri.2).is_none());
        // 正常命中。
        assert_eq!(
            ray_triangle([0.1, 0.1, 1.], [0., 0., -1.], tri.0, tri.1, tri.2),
            Some(1.0)
        );
    }

    #[test]
    fn normalized_normal_rejects_collinear_points() {
        assert!(normalized_normal(&[0., 0., 0.], &[1., 1., 1.], &[2., 2., 2.]).is_none());
        assert!(normalized_normal(&[0., 0., 0.], &[1., 0., 0.], &[0., 1., 0.]).is_some());
    }

    #[test]
    fn nearest_node_prefers_closer_candidate() {
        let nodes = vec![[0., 0., 0.], [1., 0., 0.], [5., 0., 0.]];
        let (index, distance) = nearest_node(&nodes, [1.2, 0., 0.]).unwrap();
        assert_eq!(index, 1);
        assert!((distance - 0.2).abs() < 1e-12);
        assert!(nearest_node(&[], [0., 0., 0.]).is_none());
    }
}
