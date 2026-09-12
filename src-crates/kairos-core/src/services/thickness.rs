//! 壁厚探测：三角面网格沿法向的最近命中距离统计，用于「体素尺寸 vs 最小特征」
//! 提示——体素化会抹掉薄于约两个体素的壁，或把薄壁压成单层单元（实测：5 mm
//! 体素把 3 mm 底座压成一层，流动退化为薄板，润滑压降按 1/h² 放大）。
//!
//! 复用双域管线的焊接节点与 `TriangleGrid` 射线加速，不引入第二套几何工具。

use crate::models::geometry::TriangleMesh;
use crate::services::dualdomain::{TriangleGrid, weld_surface};

/// 抽样上限：面数很大时按等距抽样，保证探测耗时可控（真实件 46.7 万面）。
pub const MAX_THICKNESS_SAMPLES: usize = 4000;

/// 壁厚探测结果（mm）。样本只覆盖「双向或单向有背面命中」的抽样面。
///
/// 真实装配件（多个零件拼装）的厚度分布是双峰的：实体区 ~10 mm 量级、件间
/// 间隙与细特征 < 1 mm。单看最薄值或某个分位数都会被离群点带偏，所以判据用
/// 「薄于阈值的**表面面积占比**」——它对离群点稳健，且直接回答「有多大比例的
/// 表面分辨率不足」。
#[derive(Debug, Clone, PartialEq)]
pub struct ThicknessReport {
    /// 升序样本（mm），供分位与占比计算。
    samples: Vec<f64>,
}

impl ThicknessReport {
    /// 无有效样本（开放网格/退化输入）。
    pub fn empty() -> Self {
        Self {
            samples: Vec::new(),
        }
    }

    /// 有效样本数（有命中的抽样面数）。
    pub fn count(&self) -> usize {
        self.samples.len()
    }

    /// 最薄处。
    pub fn min_mm(&self) -> f64 {
        self.samples.first().copied().unwrap_or(0.0)
    }

    /// 分位数（0 ≤ fraction ≤ 1）。
    pub fn quantile_mm(&self, fraction: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let index = ((self.samples.len() - 1) as f64 * fraction.clamp(0.0, 1.0)).round() as usize;
        self.samples[index]
    }

    /// 中位壁厚。
    pub fn median_mm(&self) -> f64 {
        self.quantile_mm(0.5)
    }

    /// 壁厚薄于 `threshold_mm` 的样本占比（0~1）。
    pub fn thin_fraction(&self, threshold_mm: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let thin = self.samples.partition_point(|value| *value < threshold_mm);
        thin as f64 / self.samples.len() as f64
    }
}

/// 探测壁厚：对（抽样）三角面沿 ±法向取最近命中距离的较小值。
pub fn probe_thickness(mesh: &TriangleMesh) -> ThicknessReport {
    let (nodes, triangles) = weld_surface(mesh);
    if triangles.is_empty() {
        return ThicknessReport::empty();
    }
    let (min, max) = bounds(&nodes);
    let diagonal = {
        let extent = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
        (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2])
            .sqrt()
            .max(1e-12)
    };
    let grid = TriangleGrid::new(&nodes, &triangles, min, max);
    let stride = triangles.len().div_ceil(MAX_THICKNESS_SAMPLES).max(1);
    let mut thicknesses: Vec<f64> = Vec::new();
    for indices in triangles.iter().step_by(stride) {
        let (a, b, c) = (nodes[indices[0]], nodes[indices[1]], nodes[indices[2]]);
        let centroid = [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ];
        // weld_surface 已过滤零面积面，这里的回退只是不让除零发生。
        let normal = unit_normal(&a, &b, &c).unwrap_or([0.0, 0.0, 1.0]);
        // 先沿内向（-n）找对面壁的背面命中；网格朝向可能不一致，再用外向兜底。
        // 背面命中过滤是必须的：擦边命中相邻同向面会给出 0.0x mm 的伪近距离。
        let inward = [-normal[0], -normal[1], -normal[2]];
        let hit = grid
            .first_opposing_hit(&nodes, &triangles, centroid, inward, diagonal)
            .or_else(|| grid.first_opposing_hit(&nodes, &triangles, centroid, normal, diagonal));
        let Some(value) = hit else {
            continue;
        };
        if value.is_finite() {
            thicknesses.push(value);
        }
    }
    if thicknesses.is_empty() {
        return ThicknessReport::empty();
    }
    thicknesses.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    ThicknessReport {
        samples: thicknesses,
    }
}

/// 薄壁占比告警线：薄于 2×目标尺寸的表面占比超过该值即提示。
const THIN_FRACTION_LIMIT: f64 = 0.05;

/// 网格尺寸提示：目标尺寸分辨不了的表面（壁厚薄于 2×目标尺寸）占比超线时告警
/// （空 = 通过）。
pub fn thin_feature_hints(target_size_mm: f64, report: &ThicknessReport) -> Vec<String> {
    if report.count() == 0 || target_size_mm <= 0.0 {
        return Vec::new();
    }
    let threshold = 2.0 * target_size_mm;
    let thin_fraction = report.thin_fraction(threshold);
    if thin_fraction < THIN_FRACTION_LIMIT {
        return Vec::new();
    }
    let p05 = report.quantile_mm(0.05);
    vec![format!(
        "约 {:.0}% 的表面所在壁厚薄于 2×目标尺寸（最薄 {:.2} mm、5% 分位 {:.2} mm、中位 {:.2} mm）：这些薄壁 / 窄间隙会被体素化抹掉或压成单层单元，参与流动时结果失真；若需分辨，目标尺寸应 ≤ {:.2} mm，或改用保形网格引擎（gmsh）。",
        thin_fraction * 100.0,
        report.min_mm(),
        p05,
        report.median_mm(),
        p05 / 2.0
    )]
}

fn bounds(nodes: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    nodes.iter().fold(
        ([f64::MAX; 3], [f64::MIN; 3]),
        |(mut min, mut max), node| {
            for axis in 0..3 {
                min[axis] = min[axis].min(node[axis]);
                max[axis] = max[axis].max(node[axis]);
            }
            (min, max)
        },
    )
}

fn unit_normal(a: &[f64; 3], b: &[f64; 3], c: &[f64; 3]) -> Option<[f64; 3]> {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    (length > 0.0).then(|| [cross[0] / length, cross[1] / length, cross[2] / length])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::geometry::Triangle;

    /// 轴对齐长方体（x/y/z 尺寸 mm）的 12 个三角面（外法向逆时针）。
    fn box_mesh(size: [f64; 3]) -> TriangleMesh {
        let [x, y, z] = size;
        let vertices = [
            [0.0, 0.0, 0.0],
            [x, 0.0, 0.0],
            [x, y, 0.0],
            [0.0, y, 0.0],
            [0.0, 0.0, z],
            [x, 0.0, z],
            [x, y, z],
            [0.0, y, z],
        ];
        let quads = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [0, 4, 7, 3],
            [1, 2, 6, 5],
        ];
        let mut triangles = Vec::new();
        for [a, b, c, d] in quads {
            // 法向由探测自身按顶点计算，夹具里留零值
            triangles.push(Triangle {
                a: vertices[a],
                b: vertices[b],
                c: vertices[c],
                normal: [0.0, 0.0, 0.0],
            });
            triangles.push(Triangle {
                a: vertices[a],
                b: vertices[c],
                c: vertices[d],
                normal: [0.0, 0.0, 0.0],
            });
        }
        TriangleMesh { triangles }
    }

    #[test]
    fn probe_measures_plate_thickness() {
        // 20×20×2 mm 薄板：上下表面法向命中 ≈ 2 mm，四周侧壁 ≈ 20 mm
        let report = probe_thickness(&box_mesh([20.0, 20.0, 2.0]));
        assert!(report.count() > 0);
        // 单行断言：多行 assert! 的格式化分支会被 llvm-cov 记成未覆盖行
        assert!((report.min_mm() - 2.0).abs() < 1e-6, "最薄值应为 2 mm");
        assert!((report.quantile_mm(0.05) - 2.0).abs() < 1e-6);
        // 中位含四周侧壁的 20 mm 命中，故不小于最薄值
        assert!(report.median_mm() >= report.min_mm());
        // 薄壁占比：阈值 2 mm 时上下面（恰好 2 mm）不计入
        assert_eq!(report.thin_fraction(2.0), 0.0);
        // 阈值 3 mm 时上下面（12 个样本里的 4 个）计入
        assert!((report.thin_fraction(3.0) - 4.0 / 12.0).abs() < 1e-9);
    }

    #[test]
    fn probe_skips_open_mesh_without_hits() {
        // 单个三角面：双向都无命中 → 无有效样本
        let mesh = TriangleMesh {
            triangles: vec![Triangle {
                a: [0.0, 0.0, 0.0],
                b: [1.0, 0.0, 0.0],
                c: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            }],
        };
        assert_eq!(probe_thickness(&mesh).count(), 0);
        // 零面积三角面同样不产生样本（法向未定义）
        let degenerate = TriangleMesh {
            triangles: vec![Triangle {
                a: [0.0, 0.0, 0.0],
                b: [1.0, 0.0, 0.0],
                c: [2.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            }],
        };
        assert_eq!(probe_thickness(&degenerate).count(), 0);
    }

    #[test]
    fn probe_accepts_single_sided_hit() {
        // 两片同向平行三角面（z=0 与 z=5）：只有下片沿 +z 能命中对面的背面，
        // 上片两个方向都没有合法背面命中 → 「单向命中」分支取该值。
        let mesh = TriangleMesh {
            triangles: vec![
                Triangle {
                    a: [0.0, 0.0, 0.0],
                    b: [10.0, 0.0, 0.0],
                    c: [0.0, 10.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                },
                Triangle {
                    a: [0.0, 0.0, 5.0],
                    b: [10.0, 0.0, 5.0],
                    c: [0.0, 10.0, 5.0],
                    normal: [0.0, 0.0, 1.0],
                },
            ],
        };
        let report = probe_thickness(&mesh);
        assert_eq!(report.count(), 1, "只有下片有合法命中");
        assert!((report.min_mm() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn empty_report_accessors_stay_neutral() {
        let empty = ThicknessReport::empty();
        assert_eq!(empty.count(), 0);
        assert_eq!(empty.min_mm(), 0.0);
        assert_eq!(empty.quantile_mm(0.5), 0.0);
        assert_eq!(empty.median_mm(), 0.0);
        assert_eq!(empty.thin_fraction(1.0), 0.0);
    }

    #[test]
    fn thin_feature_hints_trigger_on_insufficient_resolution() {
        let report = probe_thickness(&box_mesh([20.0, 20.0, 2.0]));
        // 目标 1.0 mm：2 mm 壁恰好 2 个体素 → 薄壁占比 0 → 不提示
        assert!(thin_feature_hints(1.0, &report).is_empty());
        // 目标 1.5 mm：阈值 3 mm，上下面（12 个样本里的 4 个）计入 → 提示
        let hints = thin_feature_hints(1.5, &report);
        assert_eq!(hints.len(), 1, "{hints:?}");
        assert!(hints[0].contains("约 33% 的表面"), "{}", hints[0]);
        assert!(hints[0].contains("目标尺寸应 ≤ 1.00 mm"), "{}", hints[0]);
        assert!(hints[0].contains("gmsh"), "{}", hints[0]);
        // 无样本 / 非正目标尺寸：不提示
        assert!(thin_feature_hints(1.5, &ThicknessReport::empty()).is_empty());
        assert!(thin_feature_hints(0.0, &report).is_empty());
    }
}
