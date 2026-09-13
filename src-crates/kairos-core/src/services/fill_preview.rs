//! 填充预览：从浇口出发的轻量充填覆盖估计（纯函数，不跑求解）。
//!
//! 口径：单元形心 + 共享面邻接构成无向图，边权取形心距离；从各浇口命中
//! 单元做多源最短路（Dijkstra），得到每个单元的「最短流动路径长度」。
//! 覆盖 = 可从浇口到达（图连通）；到达序归一化后作为云图场（0 = 浇口）。
//! 这是覆盖与先后顺序的估计，不含粘性/传热/冻层物理，不代表真实前沿。

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::error::{KairosError, Result};
use crate::models::analysis::FillPreviewReport;
use crate::models::mesh::VolumeMesh;

/// 落点偏差上限倍数：浇口到最近单元形心的距离超过「平均单元边长的该倍数」
/// 时判为「浇口未落在制品上」（坐标填错 / 落在件外）。
const GATE_OFF_PART_FACTOR: f64 = 5.0;

/// 单元形心。
fn centroid(mesh: &VolumeMesh, tet: &[usize; 4]) -> [f64; 3] {
    let mut centre = [0.0f64; 3];
    for &index in tet {
        for (axis, value) in centre.iter_mut().enumerate() {
            *value += mesh.nodes[index][axis] / 4.0;
        }
    }
    centre
}

fn distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    let dz = left[2] - right[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// 单元邻接表：共享面（无向）即相邻，边权为形心距离。
fn adjacency(centroids: &[[f64; 3]], tets: &[[usize; 4]]) -> Vec<Vec<(usize, f64)>> {
    // 面（排序后的三节点元组）→ 首个拥有它的单元。
    let mut owners: HashMap<[usize; 3], usize> = HashMap::new();
    let mut adjacency: Vec<Vec<(usize, f64)>> = vec![Vec::new(); tets.len()];
    for (cell, tet) in tets.iter().enumerate() {
        let faces = [
            [tet[0], tet[1], tet[2]],
            [tet[0], tet[1], tet[3]],
            [tet[0], tet[2], tet[3]],
            [tet[1], tet[2], tet[3]],
        ];
        for mut face in faces {
            face.sort_unstable();
            match owners.get(&face) {
                Some(&owner) => {
                    let weight = distance(centroids[owner], centroids[cell]);
                    adjacency[owner].push((cell, weight));
                    adjacency[cell].push((owner, weight));
                }
                None => {
                    owners.insert(face, cell);
                }
            }
        }
    }
    for list in &mut adjacency {
        list.sort_by_key(|(cell, _)| *cell);
    }
    adjacency
}

/// 每个浇口命中的单元：形心离浇口中心最近的那个。
fn gate_cells(centroids: &[[f64; 3]], gates: &[[f64; 3]]) -> Vec<usize> {
    gates
        .iter()
        .map(|gate| {
            let mut best = (f64::INFINITY, 0usize);
            for (cell, centre) in centroids.iter().enumerate() {
                let d = distance(*centre, *gate);
                if d < best.0 {
                    best = (d, cell);
                }
            }
            best.1
        })
        .collect()
}

/// 平均单元边长（mm）：包围盒对角线 / 单元数的立方根，用作「件外」判据的尺度。
fn cell_scale(mesh: &VolumeMesh) -> f64 {
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
    let diagonal = distance(min, max).max(1e-9);
    diagonal / (mesh.tets.len() as f64).cbrt().max(1.0)
}

/// 填充预览：返回覆盖场、未覆盖单元与告警。
pub fn preview(mesh: &VolumeMesh, gates: &[[f64; 3]]) -> Result<FillPreviewReport> {
    if mesh.tets.is_empty() {
        return Err(KairosError::validation(
            "网格为空，无法做填充预览：请先生成体积网格。",
        ));
    }
    if gates.is_empty() {
        return Err(KairosError::validation(
            "填充预览需要至少一个浇口：请在模具网络面板添加浇口单元。",
        ));
    }
    let centroids: Vec<[f64; 3]> = mesh.tets.iter().map(|tet| centroid(mesh, tet)).collect();
    let adjacency = adjacency(&centroids, &mesh.tets);
    let scale = cell_scale(mesh);
    let gates_cells = gate_cells(&centroids, gates);

    // 多源 Dijkstra：距离数组即「最短流动路径长度」。
    let mut arrival = vec![f64::INFINITY; centroids.len()];
    let mut heap: BinaryHeap<Reverse<(u64, usize)>> = BinaryHeap::new();
    for &cell in &gates_cells {
        arrival[cell] = 0.0;
        heap.push(Reverse((0, cell)));
    }
    while let Some(Reverse((key, cell))) = heap.pop() {
        let current = f64::from_bits(key);
        if current > arrival[cell] {
            continue;
        }
        for &(next, weight) in &adjacency[cell] {
            let candidate = current + weight;
            if candidate < arrival[next] {
                arrival[next] = candidate;
                heap.push(Reverse((candidate.to_bits(), next)));
            }
        }
    }

    let reachable: Vec<usize> = (0..arrival.len())
        .filter(|&i| arrival[i].is_finite())
        .collect();
    let uncovered: Vec<usize> = (0..arrival.len())
        .filter(|&i| !arrival[i].is_finite())
        .collect();
    let max_arrival = reachable.iter().map(|&i| arrival[i]).fold(0.0f64, f64::max);
    let field: Vec<f64> = arrival
        .iter()
        .map(|value| {
            if value.is_finite() && max_arrival > 0.0 {
                (value / max_arrival).clamp(0.0, 1.0)
            } else {
                1.0
            }
        })
        .collect();

    let mut warnings = Vec::new();
    let off_part: Vec<usize> = gates_cells
        .iter()
        .enumerate()
        .filter(|(index, cell)| {
            distance(centroids[**cell], gates[*index]) > GATE_OFF_PART_FACTOR * scale
        })
        .map(|(index, _)| index + 1)
        .collect();
    if !off_part.is_empty() {
        let list = off_part
            .iter()
            .map(|index| format!("#{index}"))
            .collect::<Vec<_>>()
            .join("、");
        warnings.push(format!(
            "浇口 {list} 离制品表面较远（超过平均单元尺寸的 {GATE_OFF_PART_FACTOR:.0} 倍），请核对落点坐标。"
        ));
    }
    if !uncovered.is_empty() {
        let ratio = uncovered.len() as f64 / centroids.len() as f64;
        warnings.push(format!(
            "存在无法从浇口充填的孤立区域：{} 个单元（占比 {:.1}%），请调整浇口位置或检查网格是否连通。",
            uncovered.len(),
            ratio * 100.0
        ));
    }

    Ok(FillPreviewReport {
        field,
        covered_count: reachable.len(),
        coverage_ratio: reachable.len() as f64 / centroids.len() as f64,
        uncovered_cells: uncovered,
        gate_cells: gates_cells,
        arrival_max_mm: max_arrival,
        warnings,
        basis: "图连通覆盖 + 最短流动路径到达序（启发式预览，非求解结果）".into(),
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
    fn empty_mesh_or_no_gates_are_rejected() {
        let mesh = box_mesh(10.0, 5.0);
        assert!(
            preview(&VolumeMesh::default(), &[[0.0, 0.0, 0.0]])
                .unwrap_err()
                .message()
                .contains("网格为空")
        );
        assert!(
            preview(&mesh, &[])
                .unwrap_err()
                .message()
                .contains("至少一个浇口")
        );
    }

    /// 单腔连通件：全部覆盖，浇口单元为 0、最远单元为 1。
    #[test]
    fn single_cavity_is_fully_covered() {
        let mesh = box_mesh(10.0, 2.5);
        let report = preview(&mesh, &[[2.0, 2.0, 0.0]]).unwrap();
        assert_eq!(report.covered_count, mesh.tets.len());
        assert_eq!(report.uncovered_cells.len(), 0);
        assert_eq!(report.coverage_ratio, 1.0);
        assert!(report.warnings.is_empty());
        assert_eq!(report.gate_cells.len(), 1);
        assert_eq!(report.field[report.gate_cells[0]], 0.0);
        assert!(report.field.iter().any(|value| (value - 1.0).abs() < 1e-12));
        assert!(report.field.iter().all(|value| (0.0..=1.0).contains(value)));
        assert!(report.arrival_max_mm > 0.0);
        // 到达序单调：更远的单元不会更早
        let gate_cell = report.gate_cells[0];
        let far = report
            .field
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.partial_cmp(right.1).unwrap())
            .unwrap()
            .0;
        assert_ne!(gate_cell, far);
        assert!(report.basis.contains("非求解结果"));
    }

    /// 两个分离体：只在一侧放浇口 → 另一侧整体未覆盖并给出告警。
    #[test]
    fn disconnected_region_is_reported_as_uncovered() {
        // 两个 10 mm 立方体相距 10 mm（各自独立网格）
        let single = box_mesh(10.0, 2.5);
        let mut merged = single.clone();
        let offset = 20.0;
        let node_offset = merged.nodes.len();
        for node in &single.nodes {
            merged.nodes.push([node[0] + offset, node[1], node[2]]);
        }
        for tet in &single.tets {
            merged.tets.push([
                tet[0] + node_offset,
                tet[1] + node_offset,
                tet[2] + node_offset,
                tet[3] + node_offset,
            ]);
        }
        let report = preview(&merged, &[[2.0, 2.0, 2.0]]).unwrap();
        assert_eq!(report.coverage_ratio, 0.5);
        assert_eq!(report.uncovered_cells.len(), single.tets.len());
        // 未覆盖单元在场里为 1.0，且与告警计数一致
        for &cell in &report.uncovered_cells {
            assert_eq!(report.field[cell], 1.0);
        }
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("无法从浇口充填的孤立区域"));
        assert!(report.warnings[0].contains(&single.tets.len().to_string()));
    }

    /// 浇口坐标落在件外：仍取最近单元保底，但给出「核对落点」告警。
    #[test]
    fn gate_far_from_part_produces_warning() {
        let mesh = box_mesh(10.0, 2.5);
        let report = preview(&mesh, &[[100.0, 100.0, 100.0]]).unwrap();
        // 件仍连通（浇口取最近单元），所以只有落点告警、没有未覆盖告警
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.coverage_ratio, 1.0);
        assert!(report.warnings[0].contains("离制品表面较远"));
        assert!(report.warnings[0].contains("#1"));
        assert_eq!(report.gate_cells.len(), 1);
    }

    /// 多浇口：两处都是 0，且覆盖并集 ≥ 单浇口的覆盖（这里全连通件都满覆盖）。
    #[test]
    fn multiple_gates_share_the_source_set() {
        let mesh = box_mesh(10.0, 2.5);
        let report = preview(&mesh, &[[1.0, 5.0, 5.0], [9.0, 5.0, 5.0]]).unwrap();
        assert_eq!(report.gate_cells.len(), 2);
        for &cell in &report.gate_cells {
            assert_eq!(report.field[cell], 0.0);
        }
        assert_ne!(report.gate_cells[0], report.gate_cells[1]);
        // 两侧都有源 → 最长到达距离不超过单浇口的
        let single = preview(&mesh, &[[1.0, 5.0, 5.0]]).unwrap();
        assert!(report.arrival_max_mm <= single.arrival_max_mm + 1e-9);
    }

    #[test]
    fn cell_scale_uses_bounding_box_and_cell_count() {
        let mesh = box_mesh(10.0, 2.5);
        // 10 mm 立方体、1000 个单元（4³=64 体素 × 5） → 对角线 √300 / 10 ≈ 1.73
        let scale = cell_scale(&mesh);
        assert!((scale - 300.0f64.sqrt() / (mesh.tets.len() as f64).cbrt()).abs() < 1e-12);
    }
}
