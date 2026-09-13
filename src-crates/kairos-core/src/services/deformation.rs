//! 变形显示：按单元位移场把渲染网格顶点整体偏移（纯函数）。
//!
//! 渲染网格的顶点被多个三角面共享，而位移是**逐单元**的（与场值同域）。
//! 直接按面位移会让共享顶点被多次拉动、网格撕裂，因此先把「面 → 单元」的位移
//! 聚合成「顶点 → 平均位移」，再按倍数偏移顶点。这是显示级重构，不做高阶插值。

use crate::models::render::RenderMeshData;

/// 位移分量（m，SI）→ 渲染网格坐标单位（mm）：显示按 mm ×1000。
pub const DISPLACEMENT_TO_MM: f64 = 1000.0;

/// 生成变形后的渲染网格：`变形 = 原位 + 倍数 × 顶点平均位移`。
/// `displacements` 与 `face_cells` 同域（每个单元一个三分量位移，单位 m）。
/// 倍数 0、位移缺失或分量数不匹配时返回原位网格（不改动顶点）。
pub fn deformed_render_mesh(
    render: &RenderMeshData,
    displacements: &[[f64; 3]],
    scale: f64,
) -> RenderMeshData {
    if scale == 0.0 || displacements.is_empty() || render.face_cells.is_empty() {
        return render.clone();
    }
    let positions = deform_positions(
        &render.positions,
        &render.indices,
        &render.face_cells,
        displacements,
        scale * DISPLACEMENT_TO_MM,
    );
    RenderMeshData {
        positions,
        indices: render.indices.clone(),
        face_cells: render.face_cells.clone(),
    }
}

/// 顶点位移聚合 + 偏移：每个顶点取其所属各面单元的位移均值。
/// 顶点总数取 `positions` 长度 / 3；越界的单元索引按 0 位移处理（不 panic）。
fn deform_positions(
    positions: &[f32],
    indices: &[u32],
    face_cells: &[u32],
    displacements: &[[f64; 3]],
    scale_mm: f64,
) -> Vec<f32> {
    let vertex_count = positions.len() / 3;
    let mut sums = vec![[0.0f64; 3]; vertex_count];
    let mut counts = vec![0u32; vertex_count];
    for (face, &cell) in face_cells.iter().enumerate() {
        let Some(displacement) = displacements.get(cell as usize) else {
            continue;
        };
        for corner in 0..3 {
            let Some(&index) = indices.get(face * 3 + corner) else {
                continue;
            };
            let Some(vertex) = (index as usize).lt(&vertex_count).then_some(index as usize) else {
                continue;
            };
            for axis in 0..3 {
                sums[vertex][axis] += displacement[axis];
            }
            counts[vertex] += 1;
        }
    }
    let mut deformed = Vec::with_capacity(positions.len());
    for vertex in 0..vertex_count {
        let count = counts[vertex];
        for axis in 0..3 {
            let base = positions[vertex * 3 + axis];
            if count == 0 {
                deformed.push(base);
                continue;
            }
            let offset = sums[vertex][axis] / f64::from(count) * scale_mm;
            deformed.push((f64::from(base) + offset) as f32);
        }
    }
    deformed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 两个共享一条边的三角形（单元 0 与单元 1），顶点 1/2 被两个单元共享。
    fn two_face_mesh() -> RenderMeshData {
        RenderMeshData {
            // 顶点 0..3：三角形 A(0,1,2) 与 B(1,3,2)
            positions: vec![
                0.0, 0.0, 0.0, // 0
                1.0, 0.0, 0.0, // 1（共享）
                0.0, 1.0, 0.0, // 2（共享）
                1.0, 1.0, 0.0, // 3
            ],
            indices: vec![0, 1, 2, 1, 3, 2],
            face_cells: vec![0, 1],
        }
    }

    #[test]
    fn shared_vertices_take_each_cell_displacement_average() {
        let mesh = two_face_mesh();
        // 单元 0 位移 +1 mm（写 0.001 m，显示按 mm 放大 1000 倍）
        // 单元 1 位移 +3 mm
        let displacements = vec![[0.001, 0.0, 0.0], [0.003, 0.0, 0.0]];
        let deformed = deformed_render_mesh(&mesh, &displacements, 1.0);

        // 独享顶点：顶点 0（仅单元 0）→ +1；顶点 3（仅单元 1）→ +3
        assert!((deformed.positions[0] - 1.0).abs() < 1e-6);
        assert!((deformed.positions[9] - 4.0).abs() < 1e-6);
        // 共享顶点 1 / 2：平均 (1 + 3) / 2 = +2
        assert!((deformed.positions[3] - 3.0).abs() < 1e-6);
        assert!((deformed.positions[6] - 2.0).abs() < 1e-6);
        // 索引与单元归属不变（云图仍按原位单元着色）
        assert_eq!(deformed.indices, mesh.indices);
        assert_eq!(deformed.face_cells, mesh.face_cells);
    }

    /// 索引越界（面引用了不存在的顶点）不 panic，也不影响其它顶点。
    #[test]
    fn out_of_range_vertex_indices_are_skipped() {
        let mesh = RenderMeshData {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            // 面 0 引用越界顶点 99 与不存在的面角（面数多于索引）
            indices: vec![0, 1, 99],
            face_cells: vec![0, 0],
        };
        let deformed = deformed_render_mesh(&mesh, &[[0.001, 0.0, 0.0]], 1.0);
        // 顶点 0/1 各吃到一次位移（面 0 的第三个角越界被跳过；面 1 无索引）
        assert!((deformed.positions[0] - 1.0).abs() < 1e-6);
        assert!((deformed.positions[3] - 2.0).abs() < 1e-6);
        // 顶点 2 未被任何面引用 → 保持原位
        assert!((deformed.positions[6] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn scale_zero_or_empty_displacements_keep_original_positions() {
        let mesh = two_face_mesh();
        let displacements = vec![[0.01, 0.02, 0.03], [0.0, 0.0, 0.0]];
        let zero = deformed_render_mesh(&mesh, &displacements, 0.0);
        assert_eq!(zero.positions, mesh.positions);
        let empty = deformed_render_mesh(&mesh, &[], 2.0);
        assert_eq!(empty.positions, mesh.positions);
        // 没有面归属信息（STL 表面汤的空 face_cells）也原样返回
        let no_cells = deformed_render_mesh(
            &RenderMeshData {
                face_cells: vec![],
                ..two_face_mesh()
            },
            &displacements,
            2.0,
        );
        assert_eq!(no_cells.positions, mesh.positions);
    }

    #[test]
    fn scale_multiplies_and_missing_cells_are_ignored() {
        let mesh = two_face_mesh();
        let displacements = vec![[0.002, 0.0, 0.0]];
        // 单元 1 没有位移（越界）→ 顶点 3 保持原位、共享顶点只吃到单元 0 的位移
        let deformed = deformed_render_mesh(&mesh, &displacements, 2.5);
        assert!((deformed.positions[0] - 5.0).abs() < 1e-5); // 2 mm × 2.5
        assert!((deformed.positions[9] - 1.0).abs() < 1e-6); // 顶点 3 原位
        assert!((deformed.positions[3] - 6.0).abs() < 1e-5); // 顶点 1 只吃单元 0
        // 法向重算由渲染后端负责（用变形后的位置）
        assert_eq!(deformed.indices.len(), mesh.indices.len());
    }
}
