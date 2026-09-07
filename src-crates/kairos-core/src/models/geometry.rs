//! 几何模型：三角网格、统计信息与网格健康检查结果。
//! STL 的三角形单独携带顶点（无共享索引），网格检查时按坐标容差焊接。

use serde::Serialize;

/// 单个三角形：三顶点 + 文件中声明的法向。
#[derive(Debug, Clone, PartialEq)]
pub struct Triangle {
    pub a: [f64; 3],
    pub b: [f64; 3],
    pub c: [f64; 3],
    pub normal: [f64; 3],
}

impl Triangle {
    /// 面积 = |(b-a) × (c-a)| / 2。
    pub fn area(&self) -> f64 {
        let ab = [
            self.b[0] - self.a[0],
            self.b[1] - self.a[1],
            self.b[2] - self.a[2],
        ];
        let ac = [
            self.c[0] - self.a[0],
            self.c[1] - self.a[1],
            self.c[2] - self.a[2],
        ];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt()
    }
}

/// 三角网格（STL 导入产物）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TriangleMesh {
    pub triangles: Vec<Triangle>,
}

impl TriangleMesh {
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// 包围盒 (min, max)；空网格返回 (零点, 零点)。
    pub fn bounding_box(&self) -> ([f64; 3], [f64; 3]) {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for triangle in &self.triangles {
            for vertex in [&triangle.a, &triangle.b, &triangle.c] {
                for axis in 0..3 {
                    min[axis] = min[axis].min(vertex[axis]);
                    max[axis] = max[axis].max(vertex[axis]);
                }
            }
        }
        if self.triangles.is_empty() {
            ([0.0; 3], [0.0; 3])
        } else {
            (min, max)
        }
    }

    /// 包围盒对角线长度。
    pub fn diagonal(&self) -> f64 {
        let (min, max) = self.bounding_box();
        let dx = max[0] - min[0];
        let dy = max[1] - min[1];
        let dz = max[2] - min[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// 总表面积。
    pub fn surface_area(&self) -> f64 {
        self.triangles.iter().map(|t| t.area()).sum()
    }

    /// 有符号体积（四面体累加；开放网格下仅供参考）。
    pub fn signed_volume(&self) -> f64 {
        self.triangles
            .iter()
            .map(|t| {
                let dot = t.a[0] * (t.b[1] * t.c[2] - t.b[2] * t.c[1])
                    + t.a[1] * (t.b[2] * t.c[0] - t.b[0] * t.c[2])
                    + t.a[2] * (t.b[0] * t.c[1] - t.b[1] * t.c[0]);
                dot / 6.0
            })
            .sum()
    }
}

/// 网格健康检查结果（数量均为边或三角形的计数）。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshIssues {
    /// 退化三角形（面积近零）。
    pub degenerate: usize,
    /// 边界边（只被 1 个三角形使用——开放网格，STL 常见）。
    pub open_edges: usize,
    /// 非流形边（被超过 2 个三角形使用）。
    pub non_manifold_edges: usize,
    /// 法向不一致的共享边。
    pub normal_inconsistent_edges: usize,
}

impl MeshIssues {
    pub fn is_clean(&self) -> bool {
        self.degenerate == 0
            && self.open_edges == 0
            && self.non_manifold_edges == 0
            && self.normal_inconsistent_edges == 0
    }
}

/// 导入摘要返回给前端；全量网格保留在 Rust 侧会话缓存。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeometrySummary {
    pub geometry_id: String,
    pub file_name: String,
    pub triangle_count: usize,
    /// 包围盒尺寸 (dx, dy, dz)，单位为推断单位。
    pub size: [f64; 3],
    pub surface_area: f64,
    pub signed_volume: f64,
    pub suggested_unit: String,
    pub issues: MeshIssues,
}
