//! 浇口位置分析：不预设浇口的候选评分（轻量流动启发式，纯函数）。
//!
//! 口径：候选限**表面单元**（浇口落在制品表面），对抽样候选逐个估
//! 「最长流动距离」（到最远单元形心的距离）与「壁厚可达性」（单元最短高
//! 对全场中位数的比值，薄处冻死风险高），两项相乘得适合度；适合度场按
//! 「每个单元取最近候选的分值」铺开，与结果的云图口径一致（逐单元）。
//! 这是排序建议，不是求解结果。

use crate::error::{KairosError, Result};
use crate::models::analysis::{GateCandidate, GateLocationReport};
use crate::models::mesh::VolumeMesh;
use crate::services::vec3;

/// 候选上限：评分成本 ≈ 候选数 × 单元数，超出预算的网格按等距抽样取候选。
const MAX_CANDIDATES: usize = 400;
/// Top-N 上限（面板列表长度）。
const MAX_TOP_N: usize = 20;
/// 默认 Top-N 建议数。
pub const DEFAULT_TOP_N: usize = 5;
/// 薄壁折减下限：最薄处最多把适合度打到 0.5（不归零，避免出现全零场）。
const THIN_FLOOR: f64 = 0.5;

/// 浇口位置分析参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GateLocationParams {
    pub top_n: usize,
}

impl Default for GateLocationParams {
    fn default() -> Self {
        Self {
            top_n: DEFAULT_TOP_N,
        }
    }
}

impl GateLocationParams {
    pub fn validate(&self) -> Result<()> {
        if self.top_n == 0 || self.top_n > MAX_TOP_N {
            return Err(KairosError::validation(format!(
                "Top-N 建议数必须在 1..={MAX_TOP_N} 之间。"
            )));
        }
        Ok(())
    }
}

/// 单元几何：形心 + 最短高（厚度代理）+ 最靠近形心的顶点。
struct CellGeometry {
    centre: [f64; 3],
    thickness: f64,
    node: usize,
}

fn cell_geometry(mesh: &VolumeMesh, tet: &[usize; 4]) -> CellGeometry {
    let p: Vec<[f64; 3]> = tet.iter().map(|&index| mesh.nodes[index]).collect();
    let centre = [
        (p[0][0] + p[1][0] + p[2][0] + p[3][0]) / 4.0,
        (p[0][1] + p[1][1] + p[2][1] + p[3][1]) / 4.0,
        (p[0][2] + p[1][2] + p[2][2] + p[3][2]) / 4.0,
    ];
    // 最短高 = 3V / 最大面面积 = det / (2 · A_max)：薄壁单元的厚度代理。
    let det = {
        let ab = vec3::sub(p[1], p[0]);
        let ac = vec3::sub(p[2], p[0]);
        let ad = vec3::sub(p[3], p[0]);
        vec3::dot(vec3::cross(ab, ac), ad).abs()
    };
    let max_area = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)]
        .iter()
        .map(|(a, b, c)| {
            let cross = vec3::cross(vec3::sub(p[*b], p[*a]), vec3::sub(p[*c], p[*a]));
            0.5 * vec3::dot(cross, cross).sqrt()
        })
        .fold(0.0f64, f64::max);
    let thickness = if max_area > 0.0 {
        det / (2.0 * max_area)
    } else {
        0.0
    };
    let node = tet
        .iter()
        .copied()
        .min_by(|left, right| {
            let dl = vec3::distance_sq(mesh.nodes[*left], centre);
            let dr = vec3::distance_sq(mesh.nodes[*right], centre);
            dl.partial_cmp(&dr).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(tet[0]);
    CellGeometry {
        centre,
        thickness,
        node,
    }
}

/// 壁厚折减：薄于全场中位数的单元按比例打折，下限 THIN_FLOOR。
fn thickness_factor(thickness: f64, median_thickness: f64) -> f64 {
    if median_thickness <= 0.0 {
        return 1.0;
    }
    THIN_FLOOR + (1.0 - THIN_FLOOR) * (thickness / median_thickness).min(1.0)
}

/// 中位数（就地排副本；偶数取中间两值均值）。调用方保证非空（单元表已校验）。
fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    }
}

/// 运行浇口位置分析：返回逐单元适合度场与 Top-N 建议。
pub fn analyze(mesh: &VolumeMesh, params: &GateLocationParams) -> Result<GateLocationReport> {
    params.validate()?;
    if mesh.tets.is_empty() {
        return Err(KairosError::validation(
            "网格为空，无法做浇口位置分析：请先生成体积网格。",
        ));
    }
    let cells: Vec<CellGeometry> = mesh
        .tets
        .iter()
        .map(|tet| cell_geometry(mesh, tet))
        .collect();
    let (min, max) =
        mesh.nodes
            .iter()
            .fold(([f64::MAX; 3], [f64::MIN; 3]), |(mut lo, mut hi), point| {
                for axis in 0..3 {
                    lo[axis] = lo[axis].min(point[axis]);
                    hi[axis] = hi[axis].max(point[axis]);
                }
                (lo, hi)
            });
    let diagonal =
        ((max[0] - min[0]).powi(2) + (max[1] - min[1]).powi(2) + (max[2] - min[2]).powi(2))
            .sqrt()
            .max(1e-9);
    let mut thicknesses: Vec<f64> = cells.iter().map(|cell| cell.thickness).collect();
    let median_thickness = median(&mut thicknesses);

    // 候选限表面单元：浇口要落在制品表面，内部单元不可能是浇口位置。
    // 表面节点由边界面给出（面片顶点集合），单元只要含这样的顶点即视为表层。
    let mut surface_nodes = vec![false; mesh.nodes.len()];
    for face in &mesh.surface_faces {
        for &node in face {
            surface_nodes[node] = true;
        }
    }
    let surface_cells: Vec<usize> = (0..cells.len())
        .filter(|&index| mesh.tets[index].iter().any(|&node| surface_nodes[node]))
        .collect();
    let pool = if surface_cells.is_empty() {
        (0..cells.len()).collect::<Vec<usize>>()
    } else {
        surface_cells
    };
    // 候选抽样：等距取，保证覆盖整个件（候选数上限约束成本）。
    let stride = pool.len().div_ceil(MAX_CANDIDATES).max(1);
    let candidates: Vec<usize> = pool.iter().copied().step_by(stride).collect();

    // 一次 O(候选 × 单元) 扫描：候选的最长流动距离 + 每个单元的最近候选。
    let mut max_flow = vec![0.0f64; candidates.len()];
    let mut nearest = vec![0usize; cells.len()];
    let mut nearest_distance = vec![f64::INFINITY; cells.len()];
    for (slot, &candidate) in candidates.iter().enumerate() {
        let origin = cells[candidate].centre;
        let mut farthest = 0.0f64;
        for (index, cell) in cells.iter().enumerate() {
            let distance = vec3::distance(origin, cell.centre);
            farthest = farthest.max(distance);
            if distance < nearest_distance[index] {
                nearest_distance[index] = distance;
                nearest[index] = slot;
            }
        }
        max_flow[slot] = farthest;
    }

    let scores: Vec<f64> = candidates
        .iter()
        .enumerate()
        .map(|(slot, &cell)| {
            let balance = (1.0 - max_flow[slot] / diagonal).clamp(0.0, 1.0);
            balance * thickness_factor(cells[cell].thickness, median_thickness)
        })
        .collect();

    // 适合度场：每个单元取最近候选的分值（云图与求解结果同口径）。
    let field: Vec<f64> = (0..cells.len())
        .map(|index| scores[nearest[index]])
        .collect();

    let mut ranked: Vec<usize> = (0..candidates.len()).collect();
    ranked.sort_by(|left, right| {
        scores[*right]
            .partial_cmp(&scores[*left])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top: Vec<GateCandidate> = ranked
        .into_iter()
        .take(params.top_n)
        .map(|slot| GateCandidate {
            cell: candidates[slot],
            node: cells[candidates[slot]].node,
            center: cells[candidates[slot]].centre,
            score: scores[slot],
            max_flow_length_mm: max_flow[slot],
            thickness_mm: cells[candidates[slot]].thickness,
        })
        .collect();

    Ok(GateLocationReport {
        field,
        candidate_count: candidates.len(),
        cell_count: cells.len(),
        top,
        diagonal_mm: diagonal,
        basis: "流动长度均衡 × 壁厚可达性（候选限表面单元；启发式建议，非求解结果）".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::geometry::TriangleMesh;
    use crate::services::meshing::{self, VolumeMeshParams};

    fn box_mesh(size: f64, target: f64) -> VolumeMesh {
        meshing::generate(
            &TriangleMesh::sample_box(size),
            &VolumeMeshParams {
                target_size: target,
                refinement: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn params_reject_top_n_out_of_range() {
        assert!(GateLocationParams { top_n: 0 }.validate().is_err());
        assert!(
            GateLocationParams {
                top_n: MAX_TOP_N + 1
            }
            .validate()
            .is_err()
        );
        assert!(GateLocationParams { top_n: 1 }.validate().is_ok());
        assert!(GateLocationParams::default().validate().is_ok());
        assert_eq!(GateLocationParams::default().top_n, DEFAULT_TOP_N);
    }

    #[test]
    fn empty_mesh_is_rejected() {
        let error = analyze(&VolumeMesh::default(), &GateLocationParams::default()).unwrap_err();
        assert!(error.message().contains("网格为空"));
    }

    /// 立方体件：最均衡的浇口在中心附近（流动长度最短），且场逐单元铺满。
    #[test]
    fn best_candidate_sits_near_the_centre_of_a_box() {
        let mesh = box_mesh(10.0, 2.5);
        let report = analyze(&mesh, &GateLocationParams::default()).unwrap();
        assert_eq!(report.field.len(), mesh.tets.len());
        assert_eq!(report.cell_count, mesh.tets.len());
        assert!(report.candidate_count > 0 && report.candidate_count <= MAX_CANDIDATES);
        assert_eq!(report.top.len(), DEFAULT_TOP_N);
        assert!(report.basis.contains("启发式"));

        // 立方体的最佳浇口是某个面的中心：两轴居中、一轴贴面。
        let best = &report.top[0];
        let near_centre = (0..3)
            .filter(|&axis| (best.center[axis] - 5.0).abs() < 2.5)
            .count();
        let near_face = (0..3)
            .filter(|&axis| {
                let value = best.center[axis];
                value < 2.5 || value > 7.5
            })
            .count();
        assert_eq!(near_centre, 2, "最佳候选应居中两轴：{:?}", best.center);
        assert_eq!(near_face, 1, "最佳候选应有一轴贴面：{:?}", best.center);
        // 分值降序 + 场值域在 0..1
        for pair in report.top.windows(2) {
            assert!(pair[0].score >= pair[1].score);
        }
        assert!(report.field.iter().all(|value| (0.0..=1.0).contains(value)));
        // 中心候选的最长流动距离应小于包围盒对角线（立方体 √300 ≈ 17.32）
        assert!(best.max_flow_length_mm < report.diagonal_mm);
        assert!((report.diagonal_mm - 300.0f64.sqrt()).abs() < 1e-9);
        // 建议落点是真实节点：坐标与该节点一致
        assert_eq!(best.center, {
            let tet = mesh.tets[best.cell];
            let mut centre = [0.0; 3];
            for index in tet {
                for (axis, value) in centre.iter_mut().enumerate() {
                    *value += mesh.nodes[index][axis] / 4.0;
                }
            }
            centre
        });
        assert!(mesh.nodes[best.node].iter().all(|value| value.is_finite()));
    }

    /// 壁厚折减：薄于中位数按比例打折，下限 THIN_FLOOR；中位数缺失不打折。
    #[test]
    fn thickness_factor_discounts_thin_cells() {
        assert_eq!(thickness_factor(2.0, 2.0), 1.0);
        assert_eq!(thickness_factor(4.0, 2.0), 1.0);
        assert!((thickness_factor(1.0, 2.0) - 0.75).abs() < 1e-12);
        assert_eq!(thickness_factor(0.0, 2.0), THIN_FLOOR);
        assert_eq!(thickness_factor(1.0, 0.0), 1.0);
    }

    /// 候选必须落在表面单元上（浇口在制品表面，不能在体内部）。
    #[test]
    fn candidates_are_limited_to_surface_cells() {
        let mesh = box_mesh(10.0, 2.0);
        let mut surface_nodes = vec![false; mesh.nodes.len()];
        for face in &mesh.surface_faces {
            for &node in face {
                surface_nodes[node] = true;
            }
        }
        let report = analyze(&mesh, &GateLocationParams { top_n: MAX_TOP_N }).unwrap();
        assert!(report.top.len() > 5);
        for candidate in &report.top {
            assert!(
                mesh.tets[candidate.cell]
                    .iter()
                    .any(|&node| surface_nodes[node]),
                "候选单元不在表面"
            );
            assert!(candidate.thickness_mm > 0.0);
        }
        // 面片缺失（退化网格）时退回全单元候选，不 panic
        let mut broken = mesh.clone();
        broken.surface_faces.clear();
        assert!(analyze(&broken, &GateLocationParams::default()).is_ok());
    }

    #[test]
    fn top_n_is_honored() {
        let mesh = box_mesh(10.0, 5.0);
        let report = analyze(&mesh, &GateLocationParams { top_n: 3 }).unwrap();
        assert_eq!(report.top.len(), 3);
    }

    /// 退化单元（零面积面 / 零体积）不产生 NaN：厚度代理与折减都归 0/1。
    #[test]
    fn degenerate_cells_do_not_break_scoring() {
        // 四点共线：四个面面积全为 0、体积 0
        let degenerate = VolumeMesh {
            nodes: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
            ],
            tets: vec![[0, 1, 2, 3]],
            surface_faces: vec![[0, 1, 2]],
        };
        let report = analyze(&degenerate, &GateLocationParams::default()).unwrap();
        assert_eq!(report.top.len(), 1);
        assert_eq!(report.top[0].thickness_mm, 0.0);
        // 中位厚度为 0 → 不打折（thickness_factor 的 1.0 兜底），分值为有限数
        assert!(report.top[0].score.is_finite());
        assert!((0.0..=1.0).contains(&report.top[0].score));
        assert!(report.field.iter().all(|value| value.is_finite()));
    }

    /// 候选上限：单元数远超上限时按等距抽样，成本受控（结构性断言）。
    #[test]
    fn candidate_sampling_is_bounded_for_large_meshes() {
        let mesh = box_mesh(10.0, 1.25); // 8³ = 512 体素 → 2560 四面体
        assert!(mesh.tets.len() > MAX_CANDIDATES);
        let report = analyze(&mesh, &GateLocationParams::default()).unwrap();
        assert!(report.candidate_count <= MAX_CANDIDATES);
        assert_eq!(report.field.len(), mesh.tets.len());
    }
}
