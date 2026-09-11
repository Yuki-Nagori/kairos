//! 体数据重采样：四面体网格 + 单元关联场 → 结构化体素网格。
//!
//! 这是三维体渲染（体绘制）的数据前置：把非结构四面体场重采样到规则
//! 网格，供 GPU 光线步进按体素直接采样。采样按四面体包围盒栅格化——
//! 对每个四面体只测试其包围盒内的网格点（重心坐标同号判定），
//! 复杂度 O(Σ 四面体包围盒内网格点数)，与全场体素数解耦。
//! 未被任何四面体覆盖的网格点值记 0，覆盖率在诊断字段呈现。

use crate::error::{KairosError, Result};
use crate::models::mesh::VolumeMesh;

/// 重采样参数：三轴统一的体素分辨率。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResampleParams {
    /// 每轴体素数（8..=256）。
    pub resolution: usize,
}

impl ResampleParams {
    pub fn validate(&self) -> Result<()> {
        if !(8..=256).contains(&self.resolution) {
            return Err(KairosError::validation("体素分辨率必须在 8..=256 之间。"));
        }
        Ok(())
    }
}

/// 结构化体素场：值按 x 主序（y 次之、z 再次）排布，总长 = nx × ny × nz。
#[derive(Debug, Clone, PartialEq)]
pub struct VolumeFieldGrid {
    pub dims: [usize; 3],
    /// 网格原点（包围盒 min，即体素 [0,0,0] 的角点）。
    pub origin: [f64; 3],
    /// 体素间距（包围盒尺寸 / 各轴体素数）。
    pub spacing: [f64; 3],
    /// 体素中心处的场值；未被四面体覆盖的点为 0。
    pub values: Vec<f64>,
    /// 被至少一个四面体覆盖的网格点比例（0..1，体素化覆盖诊断）。
    pub covered_ratio: f64,
}

/// 由体积网格与单元场重采样出结构化体素场。
pub fn resample(
    volume: &VolumeMesh,
    field: &crate::models::results::ScalarField,
    params: &ResampleParams,
) -> Result<VolumeFieldGrid> {
    params.validate()?;
    if volume.tets.is_empty() || volume.nodes.is_empty() {
        return Err(KairosError::validation("体积网格为空，无法重采样体数据。"));
    }
    if field.values.len() != volume.tets.len() {
        return Err(KairosError::validation(format!(
            "场值数量（{}）与四面体数量（{}）不一致。",
            field.values.len(),
            volume.tets.len()
        )));
    }

    let (min, max) = mesh_bounds(volume);
    let dims = [params.resolution; 3];
    let spacing = [
        (max[0] - min[0]) / dims[0] as f64,
        (max[1] - min[1]) / dims[1] as f64,
        (max[2] - min[2]) / dims[2] as f64,
    ];

    let mut values = vec![0.0f64; dims[0] * dims[1] * dims[2]];
    let mut covered = vec![false; values.len()];
    let mut covered_count = 0usize;

    for (cell, tet) in volume.tets.iter().enumerate() {
        let p = [
            volume.nodes[tet[0]],
            volume.nodes[tet[1]],
            volume.nodes[tet[2]],
            volume.nodes[tet[3]],
        ];
        // 包围盒栅格化：只测试四面体包围盒覆盖的网格点。
        let (lo, hi) = tet_bounds(&p);
        let ix_lo = index_of(lo[0], min[0], spacing[0], dims[0]);
        let ix_hi = index_of(hi[0], min[0], spacing[0], dims[0]);
        let iy_lo = index_of(lo[1], min[1], spacing[1], dims[1]);
        let iy_hi = index_of(hi[1], min[1], spacing[1], dims[1]);
        let iz_lo = index_of(lo[2], min[2], spacing[2], dims[2]);
        let iz_hi = index_of(hi[2], min[2], spacing[2], dims[2]);
        for ix in ix_lo..=ix_hi {
            for iy in iy_lo..=iy_hi {
                for iz in iz_lo..=iz_hi {
                    let flat = flat_index(ix, iy, iz, dims);
                    if covered[flat] {
                        continue;
                    }
                    let point = grid_point(ix, iy, iz, min, spacing);
                    if point_in_tet(point, &p) {
                        values[flat] = field.values[cell];
                        covered[flat] = true;
                        covered_count += 1;
                    }
                }
            }
        }
    }

    let covered_ratio = covered_count as f64 / values.len() as f64;
    Ok(VolumeFieldGrid {
        dims,
        origin: min,
        spacing,
        values,
        covered_ratio,
    })
}

fn mesh_bounds(volume: &VolumeMesh) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for node in &volume.nodes {
        for axis in 0..3 {
            min[axis] = min[axis].min(node[axis]);
            max[axis] = max[axis].max(node[axis]);
        }
    }
    (min, max)
}

fn tet_bounds(p: &[[f64; 3]; 4]) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for vertex in p {
        for axis in 0..3 {
            lo[axis] = lo[axis].min(vertex[axis]);
            hi[axis] = hi[axis].max(vertex[axis]);
        }
    }
    (lo, hi)
}

/// 世界坐标 → 体素索引（体素 [i] 覆盖 [origin + i·h, origin + (i+1)·h)，夹取到网格内）。
fn index_of(coord: f64, origin: f64, spacing: f64, dims: usize) -> usize {
    (((coord - origin) / spacing).floor() as isize).clamp(0, dims as isize - 1) as usize
}

fn flat_index(ix: usize, iy: usize, iz: usize, dims: [usize; 3]) -> usize {
    ix + dims[0] * (iy + dims[1] * iz)
}

fn grid_point(ix: usize, iy: usize, iz: usize, origin: [f64; 3], spacing: [f64; 3]) -> [f64; 3] {
    [
        origin[0] + (ix as f64 + 0.5) * spacing[0],
        origin[1] + (iy as f64 + 0.5) * spacing[1],
        origin[2] + (iz as f64 + 0.5) * spacing[2],
    ]
}

/// 重心坐标同号判定：点在四面体内 ⇔ 四个体积分量同号（非负或非正）。
fn point_in_tet(point: [f64; 3], p: &[[f64; 3]; 4]) -> bool {
    let det0 = signed_volume(p[0], p[1], p[2], p[3]);
    if det0.abs() < 1e-15 {
        return false;
    }
    let dets = [
        signed_volume(point, p[1], p[2], p[3]),
        signed_volume(p[0], point, p[2], p[3]),
        signed_volume(p[0], p[1], point, p[3]),
        signed_volume(p[0], p[1], p[2], point),
    ];
    // 容差：体积分量允许微小负值（浮点）。
    let tolerance = det0.abs() * 1e-9 + 1e-15;
    dets.iter()
        .all(|d| *d >= -tolerance && *d <= det0.abs() + tolerance)
        || dets
            .iter()
            .all(|d| *d <= tolerance && *d >= -(det0.abs() + tolerance))
}

fn signed_volume(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> f64 {
    let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let e3 = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
    let cross = [
        e2[1] * e3[2] - e2[2] * e3[1],
        e2[2] * e3[0] - e2[0] * e3[2],
        e2[0] * e3[1] - e2[1] * e3[0],
    ];
    (e1[0] * cross[0] + e1[1] * cross[1] + e1[2] * cross[2]) / 6.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::mesh::VolumeMesh;
    use crate::models::results::ScalarField;

    /// 单位直角四面体：(0,0,0),(1,0,0),(0,1,0),(0,0,1)。
    fn unit_tet() -> VolumeMesh {
        VolumeMesh {
            nodes: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            tets: vec![[0, 1, 2, 3]],
            surface_faces: Vec::new(),
        }
    }

    fn scalar_field(values: Vec<f64>) -> ScalarField {
        ScalarField {
            field: "T".into(),
            time_dir: "0".into(),
            time_s: 0.0,
            values,
            is_magnitude: false,
            complete: true,
        }
    }

    #[test]
    fn params_reject_resolution_out_of_range() {
        assert!(ResampleParams { resolution: 7 }.validate().is_err());
        assert!(ResampleParams { resolution: 257 }.validate().is_err());
        assert!(ResampleParams { resolution: 8 }.validate().is_ok());
        assert!(ResampleParams { resolution: 256 }.validate().is_ok());
    }

    #[test]
    fn empty_mesh_and_count_mismatch_are_rejected() {
        let error = resample(
            &VolumeMesh::default(),
            &scalar_field(vec![]),
            &ResampleParams { resolution: 8 },
        )
        .unwrap_err();
        assert!(error.to_string().contains("体积网格为空"));

        let mesh = unit_tet();
        let error = resample(
            &mesh,
            &scalar_field(vec![1.0, 2.0]),
            &ResampleParams { resolution: 8 },
        )
        .unwrap_err();
        assert!(error.to_string().contains("不一致"));
    }

    #[test]
    fn tet_interior_is_sampled_and_exterior_left_zero() {
        let grid = resample(
            &unit_tet(),
            &scalar_field(vec![5.0]),
            &ResampleParams { resolution: 8 },
        )
        .unwrap();
        assert_eq!(grid.dims, [8, 8, 8]);
        assert_eq!(grid.values.len(), 512);
        assert!(grid.covered_ratio > 0.0 && grid.covered_ratio < 1.0);
        // 靠近直角顶点的体素中心在四面体内；远离对角的体素为 0（未覆盖）。
        let center_low = grid.values[flat_index(0, 0, 0, grid.dims)];
        assert!((center_low - 5.0).abs() < 1e-12);
        let far = grid.values[flat_index(7, 7, 7, grid.dims)];
        assert_eq!(far, 0.0);
    }

    #[test]
    fn two_tet_box_carries_distinct_cell_values() {
        // 单位立方体拆成两个四面体：值分别为 1 与 2，网格同时含两类值。
        let mesh = VolumeMesh {
            nodes: vec![
                [0., 0., 0.],
                [1., 0., 0.],
                [1., 1., 0.],
                [0., 1., 0.],
                [0., 0., 1.],
                [1., 0., 1.],
                [1., 1., 1.],
                [0., 1., 1.],
            ],
            tets: vec![
                [0, 1, 2, 6],
                [0, 2, 3, 6],
                [0, 1, 6, 4],
                [0, 4, 6, 7],
                [0, 7, 6, 3],
                [1, 5, 6, 4],
            ],
            surface_faces: Vec::new(),
        };
        let grid = resample(
            &mesh,
            &scalar_field(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
            &ResampleParams { resolution: 8 },
        )
        .unwrap();
        let has = |value: f64| grid.values.iter().any(|v| (*v - value).abs() < 1e-12);
        assert!(has(1.0));
        assert!(has(2.0));
        assert!((grid.covered_ratio - 1.0).abs() < 1e-9);
    }

    #[test]
    fn degenerate_tet_covers_nothing() {
        // 共面（零体积）四面体：包围盒非退化但无内部点，覆盖率 0。
        let degenerate = VolumeMesh {
            nodes: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
            tets: vec![[0, 1, 2, 3]],
            surface_faces: Vec::new(),
        };
        let grid = resample(
            &degenerate,
            &scalar_field(vec![5.0]),
            &ResampleParams { resolution: 8 },
        )
        .unwrap();
        assert_eq!(grid.covered_ratio, 0.0);
        assert!(grid.values.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn resample_is_deterministic() {
        let a = resample(
            &unit_tet(),
            &scalar_field(vec![5.0]),
            &ResampleParams { resolution: 8 },
        )
        .unwrap();
        let b = resample(
            &unit_tet(),
            &scalar_field(vec![5.0]),
            &ResampleParams { resolution: 8 },
        )
        .unwrap();
        assert_eq!(a, b);
    }
}
