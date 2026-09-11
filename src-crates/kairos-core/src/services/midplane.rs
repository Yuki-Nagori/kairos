//! 中面网格服务：顶点配对法（1D/2.5D 快速分析路线）。
//!
//! 每个焊接后的表面顶点沿其相邻三角形的内法向发射射线，取最近对面
//! 命中（多射线取近可缓解棱边顶点平均法向的斜穿问题）；中面节点 =
//! 顶点与命中点的中点，单元继承表面三角形连接，厚度为三顶点配对
//! 厚度均值。射线求交复用双域网格的均匀格加速（见 [`super::dualdomain`]）。
//! 杆系梁端点捕捉中面节点；捕捉失败的端点成为自由节点，完全重合的
//! 自由端按位置复用。

use crate::error::{KairosError, Result};
use crate::models::geometry::TriangleMesh;
use crate::models::mesh::{BeamCoupling, MidplaneMesh, MidplaneReport, ShellBeam};
use crate::models::runners::RunnerElement;
use crate::services::dualdomain::{TriangleGrid, nearest_node, normalized_normal, weld_surface};

type Point = [f64; 3];

/// 中面网格参数（与双域网格同口径的捕捉容差）。
#[derive(Debug, Clone, Copy, Default)]
pub struct MidplaneParams {
    /// 梁端点捕捉中面节点的最大距离；None 按包围盒对角线 1e-3 自动取值。
    pub snap_tolerance: Option<f64>,
}

impl MidplaneParams {
    pub fn validate(&self) -> Result<()> {
        if let Some(tol) = self.snap_tolerance
            && (!tol.is_finite() || tol < 0.0)
        {
            return Err(KairosError::validation("捕捉容差必须为非负有限数。"));
        }
        Ok(())
    }
}

/// 由表面网格与杆系生成中面网格，返回网格与统计报告。
pub fn generate(
    mesh: &TriangleMesh,
    runners: &[RunnerElement],
    params: &MidplaneParams,
) -> Result<(MidplaneMesh, MidplaneReport)> {
    params.validate()?;
    let (min, max) = mesh.bounding_box();
    let diagonal = mesh.diagonal();
    if mesh.triangles.is_empty() || diagonal <= 0.0 || !diagonal.is_finite() {
        return Err(KairosError::validation(
            "几何为空或包围盒退化，无法生成中面网格。",
        ));
    }

    let (nodes, triangles) = weld_surface(mesh);
    if triangles.is_empty() {
        return Err(KairosError::validation(
            "焊接后没有有效三角形，无法生成中面网格。",
        ));
    }

    // 每顶点沿相邻面内法向的多射线配对：取最近命中，中点为中面节点。
    let grid = TriangleGrid::new(&nodes, &triangles, min, max);
    let mut mid_nodes: Vec<Point> = Vec::with_capacity(nodes.len());
    // 顶点序号 → (中面节点序号, 配对厚度)；未配对为 None。
    let mut pairing: Vec<Option<(usize, f64)>> = Vec::with_capacity(nodes.len());
    let mut unpaired_vertices = 0usize;
    for (vertex, position) in nodes.iter().enumerate() {
        let mut best: Option<(Point, f64)> = None;
        for indices in &triangles {
            if indices.contains(&vertex) {
                let (a, b, c) = (nodes[indices[0]], nodes[indices[1]], nodes[indices[2]]);
                // 退化面已在焊接阶段剔除；法向求不出时命中记为无（and_then 内跳过）。
                let hit = normalized_normal(&a, &b, &c).and_then(|normal| {
                    let inward = [-normal[0], -normal[1], -normal[2]];
                    grid.first_hit(&nodes, &triangles, *position, inward, diagonal)
                        .map(|distance| (normal, distance))
                });
                if let Some((normal, distance)) = hit
                    && best.is_none_or(|(_, current)| distance < current)
                {
                    best = Some((normal, distance));
                }
            }
        }
        match best {
            Some((normal, distance)) => {
                pairing.push(Some((mid_nodes.len(), distance)));
                mid_nodes.push([
                    position[0] - normal[0] * distance * 0.5,
                    position[1] - normal[1] * distance * 0.5,
                    position[2] - normal[2] * distance * 0.5,
                ]);
            }
            None => {
                unpaired_vertices += 1;
                pairing.push(None);
            }
        }
    }

    // 单元继承表面连接；任一顶点未配对即丢弃。
    let mut elements = Vec::with_capacity(triangles.len());
    let mut thickness = Vec::with_capacity(triangles.len());
    let mut dropped_elements = 0usize;
    for indices in &triangles {
        let Some(corners) = indices
            .iter()
            .map(|&v| pairing[v].map(|(mid, _)| mid))
            .collect::<Option<Vec<usize>>>()
        else {
            dropped_elements += 1;
            continue;
        };
        let pair_thickness: f64 = indices
            .iter()
            .map(|&v| pairing[v].map_or(0.0, |(_, d)| d))
            .sum::<f64>()
            / 3.0;
        elements.push([corners[0], corners[1], corners[2]]);
        thickness.push(pair_thickness);
    }
    if elements.is_empty() {
        return Err(KairosError::validation(
            "没有顶点能配对到对面，无法生成中面网格（开放 / 实体厚件请用 3D 路线）。",
        ));
    }

    // 杆系耦合：梁端点捕捉最近中面节点；失败端成为自由节点（完全重合的
    // 自由端按位置复用，如两条流道的共享结点）。
    let snap_tolerance = params.snap_tolerance.unwrap_or(diagonal * 1e-3);
    let mut couplings = Vec::new();
    let mut beams = Vec::with_capacity(runners.len());
    let mut free_nodes: Vec<Point> = Vec::new();
    for (index, runner) in runners.iter().enumerate() {
        let mut endpoints = [0usize; 2];
        for (slot, point) in [(0usize, runner.start), (1usize, runner.end)] {
            if let Some((node, distance)) = nearest_node(&mid_nodes, point)
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
            // 自由端索引接在中面节点之后（最后统一并入节点数组）。
            match free_nodes.iter().position(|candidate| candidate == &point) {
                Some(free_index) => endpoints[slot] = mid_nodes.len() + free_index,
                None => {
                    endpoints[slot] = mid_nodes.len() + free_nodes.len();
                    free_nodes.push(point);
                }
            }
        }
        beams.push(ShellBeam {
            nodes: endpoints,
            diameter: runner.diameter_mm,
            kind: runner.kind,
        });
    }

    // 自由端节点并入节点数组（梁端点索引按「中面节点 + 自由节点」连续编号）。
    mid_nodes.extend(free_nodes);

    let mut min = f64::INFINITY;
    let mut max = 0.0f64;
    let mut sum = 0.0f64;
    for value in &thickness {
        min = min.min(*value);
        max = max.max(*value);
        sum += value;
    }
    let stats = MidplaneReport {
        node_count: mid_nodes.len(),
        element_count: elements.len(),
        beam_count: beams.len(),
        coupling_count: couplings.len(),
        uncoupled_endpoints: beams.len() * 2 - couplings.len(),
        unpaired_vertices,
        dropped_elements,
        // elements 非空在上方已保证，厚度统计直接取值。
        thickness_min: min,
        thickness_max: max,
        thickness_avg: sum / thickness.len() as f64,
    };
    Ok((
        MidplaneMesh {
            nodes: mid_nodes,
            elements,
            thickness,
            beams,
            couplings,
        },
        stats,
    ))
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
            MidplaneParams {
                snap_tolerance: Some(-1.0)
            }
            .validate()
            .is_err()
        );
        assert!(
            MidplaneParams {
                snap_tolerance: Some(f64::NAN)
            }
            .validate()
            .is_err()
        );
        assert!(
            MidplaneParams {
                snap_tolerance: Some(0.5)
            }
            .validate()
            .is_ok()
        );
        assert!(
            MidplaneParams {
                snap_tolerance: None
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn degenerate_or_empty_meshes_are_rejected() {
        assert!(generate(&TriangleMesh::default(), &[], &MidplaneParams::default()).is_err());
        let collapsed = TriangleMesh {
            triangles: vec![Triangle {
                a: [1., 1., 1.],
                b: [1., 1., 1.],
                c: [1., 1., 1.],
                normal: [0.; 3],
            }],
        };
        assert!(generate(&collapsed, &[], &MidplaneParams::default()).is_err());
    }

    #[test]
    fn collinear_faces_only_are_rejected_after_weld() {
        let collinear = TriangleMesh {
            triangles: vec![Triangle {
                a: [0., 0., 0.],
                b: [1., 0., 0.],
                c: [2., 0., 0.],
                normal: [0.; 3],
            }],
        };
        let error = generate(&collinear, &[], &MidplaneParams::default()).unwrap_err();
        assert!(error.to_string().contains("焊接后没有有效三角形"));
    }

    #[test]
    fn slab_mid_nodes_sit_at_half_thickness() {
        // 10×10×2 平板（仅顶底面）：中面节点应落在 z = 1，厚度 ≈ 2。
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
        let (mesh, stats) =
            generate(&TriangleMesh { triangles }, &[], &MidplaneParams::default()).unwrap();
        assert_eq!(mesh.nodes.len(), 8);
        assert_eq!(mesh.elements.len(), 4);
        for node in &mesh.nodes {
            assert!((node[2] - 1.0).abs() < 1e-9, "中面 z {node:?}");
        }
        for value in &mesh.thickness {
            assert!((value - 2.0).abs() < 1e-9, "厚度 {value}");
        }
        assert_eq!(stats.element_count, 4);
        assert_eq!(stats.unpaired_vertices, 0);
        assert_eq!(stats.dropped_elements, 0);
        assert!((stats.thickness_avg - 2.0).abs() < 1e-9);
    }

    #[test]
    fn open_surface_drops_all_elements_and_reports() {
        let mesh = TriangleMesh {
            triangles: vec![Triangle {
                a: [0., 0., 0.],
                b: [1., 0., 0.],
                c: [0., 1., 0.],
                normal: [0.; 3],
            }],
        };
        assert!(generate(&mesh, &[], &MidplaneParams::default()).is_err());
    }

    #[test]
    fn partially_paired_surface_drops_adjacent_elements() {
        // 顶面 + 侧壁 + 底面：侧壁顶点配到对面侧壁（厚度大但可配对），
        // 全部顶点都能配对 → 不丢单元；改为「顶面 + 突出小面」制造部分配对。
        let mut triangles = vec![];
        // 顶面（可配对到底面）
        triangles.extend(quad(
            [0., 0., 2.],
            [10., 0., 2.],
            [10., 10., 2.],
            [0., 10., 2.],
        ));
        // 底面
        triangles.extend(quad(
            [0., 10., 0.],
            [10., 10., 0.],
            [10., 0., 0.],
            [0., 0., 0.],
        ));
        // 突出的孤立小面（其顶点在面法向上没有对面）
        triangles.extend(quad(
            [20., 0., 0.],
            [21., 0., 0.],
            [21., 1., 0.],
            [20., 1., 0.],
        ));
        let (mesh, stats) =
            generate(&TriangleMesh { triangles }, &[], &MidplaneParams::default()).unwrap();
        // 顶底面 4 单元保留；突出面 2 单元丢弃。
        assert_eq!(mesh.elements.len(), 4);
        assert_eq!(stats.dropped_elements, 2);
        assert!(stats.unpaired_vertices > 0);
        // 中面节点只含配对顶点（8 个）+ 自由端（无）→ 小面顶点不产生节点。
        assert_eq!(mesh.nodes.len(), 8);
    }

    #[test]
    fn box_vertex_thickness_uses_multi_ray_minimum() {
        // 立方体棱边顶点：多射线取近后仍配到对面（厚度 = 边长）。
        let (mesh, stats) = generate(&cube(10.0), &[], &MidplaneParams::default()).unwrap();
        assert_eq!(mesh.nodes.len(), 8);
        assert_eq!(mesh.elements.len(), 12);
        for value in &mesh.thickness {
            assert!((value - 10.0).abs() < 1e-9, "厚度 {value}");
        }
        assert_eq!(stats.unpaired_vertices, 0);
    }

    #[test]
    fn runner_endpoints_snap_to_mid_nodes_or_become_free() {
        // 立方体中面是 z=5 的缩位面：梁端点设在 (10, 5, 5)（顶点 (10,0,0) 与
        // (10,10,0) 的中面节点中点 x=10? 中面节点 = 顶点 - n*d/2 → x=10-5=5。
        // 直接验证：自由端共享结点复用 + 捕捉计数。
        let (mesh, stats) = generate(
            &cube(10.0),
            &[
                runner([0., 0., 5.], [50., 0., 5.]),
                runner([50., 0., 5.], [40., 0., 5.]),
            ],
            &MidplaneParams::default(),
        )
        .unwrap();
        // (0,0,5) 在棱 (0,0,*) 中面节点附近吗？中面节点 x≥5 → 捕捉失败为自由端。
        // 两条流道的 (50,0,5) / (50,0,5) 共享同一位置 → 复用同一自由节点。
        assert_eq!(mesh.beams.len(), 2);
        assert_eq!(mesh.beams[0].nodes[1], mesh.beams[1].nodes[0]);
        assert!(stats.uncoupled_endpoints >= 3);
        assert!(mesh.nodes.len() > 8);
    }

    #[test]
    fn zero_snap_tolerance_still_couples_exact_mid_node() {
        // 构造一个端点恰好落在中面节点上的梁：顶点 (0,0,10) 的中面节点 =
        // (0,0,10) - (0,0,-1)*10/2 = (0,0,5)。
        let (mesh, stats) = generate(
            &cube(10.0),
            &[runner([0., 0., 5.], [50., 0., 5.])],
            &MidplaneParams {
                snap_tolerance: Some(0.0),
            },
        )
        .unwrap();
        let mid_node = mesh
            .nodes
            .iter()
            .position(|n| {
                (n[0] - 0.0).abs() < 1e-9 && (n[1] - 0.0).abs() < 1e-9 && (n[2] - 5.0).abs() < 1e-9
            })
            .expect("应存在中面节点 (0,0,5)");
        assert!(
            mesh.couplings
                .iter()
                .any(|c| c.node == mid_node && c.distance == 0.0)
        );
        assert_eq!(stats.coupling_count, 1);
    }
}
