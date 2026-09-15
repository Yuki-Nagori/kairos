//! 渲染网格提取：体积网格边界面（带 owner 单元归属）与 STL 表面网格 →
//! 前端视口上传的顶点/索引形态。边界面判定与 OpenFOAM polyMesh / Gmsh
//! msh 的表面提取同源：只被一个四面体使用的面即边界面。

use crate::models::geometry::TriangleMesh;
use crate::models::mesh::VolumeMesh;
use crate::models::render::RenderMeshData;

/// 体积网格 → 渲染网格：边界面三角扇展开，face_cells 记录 owner 单元
/// （云图按单元值着色）。
pub fn from_volume_mesh(volume: &VolumeMesh) -> RenderMeshData {
    let mut positions = Vec::with_capacity(volume.nodes.len() * 3);
    for node in &volume.nodes {
        positions.push(node[0] as f32);
        positions.push(node[1] as f32);
        positions.push(node[2] as f32);
    }
    // 边界面归属：重算面计数后，只保留边界三角面并记录 owner 单元。
    let mut face_count: std::collections::HashMap<[usize; 3], usize> =
        std::collections::HashMap::new();
    for tet in &volume.tets {
        for face in tet_faces(tet) {
            let mut key = face;
            key.sort_unstable();
            *face_count.entry(key).or_insert(0) += 1;
        }
    }
    let mut indices = Vec::new();
    let mut face_cells = Vec::new();
    for (cell_index, tet) in volume.tets.iter().enumerate() {
        for (face_index, face) in tet_faces(tet).into_iter().enumerate() {
            let mut key = face;
            key.sort_unstable();
            if face_count.get(&key) == Some(&1) {
                // 体网格四面体统一为正体积，但固定的面枚举并不保证每个面
                // 都朝外。视口法向取决于绕向；这里在导出边界时按对顶点
                // 重新定向，避免体网格显示出翻面、明暗断裂或局部发黑。
                let face = orient_boundary_face(tet, face_index, face, &volume.nodes);
                indices.extend_from_slice(&(face.map(|i| i as u32)));
                face_cells.push(cell_index as u32);
            }
        }
    }
    RenderMeshData {
        positions,
        indices,
        face_cells,
    }
}

/// 将四面体边界面定向为向外法向。
///
/// `tet_faces` 的面顺序只用于拓扑枚举，不能直接当作渲染绕向。若面法向
/// 指向四面体内部，则交换后两个顶点；这样所有边界面都与外部半空间同向。
fn orient_boundary_face(
    tet: &[usize; 4],
    face_index: usize,
    face: [usize; 3],
    nodes: &[[f64; 3]],
) -> [usize; 3] {
    let opposite = tet[[3, 2, 1, 0][face_index]];
    let a = nodes[face[0]];
    let b = nodes[face[1]];
    let c = nodes[face[2]];
    let d = nodes[opposite];
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let ad = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
    let normal = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    if normal[0] * ad[0] + normal[1] * ad[1] + normal[2] * ad[2] > 0.0 {
        [face[0], face[2], face[1]]
    } else {
        face
    }
}

/// 四面体的 4 个三角面（保持绕向，供法向计算）。
fn tet_faces(tet: &[usize; 4]) -> [[usize; 3]; 4] {
    [
        [tet[0], tet[1], tet[2]],
        [tet[0], tet[1], tet[3]],
        [tet[0], tet[2], tet[3]],
        [tet[1], tet[2], tet[3]],
    ]
}

/// STL 表面网格 → 渲染网格：三角形汤展开为独立顶点（无索引共享）。
pub fn from_surface_mesh(mesh: &TriangleMesh) -> RenderMeshData {
    let mut positions = Vec::with_capacity(mesh.triangles.len() * 9);
    let mut indices = Vec::with_capacity(mesh.triangles.len() * 3);
    let mut face_cells = Vec::new();
    for (face, triangle) in mesh.triangles.iter().enumerate() {
        let base = (face * 3) as u32;
        for vertex in [triangle.a, triangle.b, triangle.c] {
            positions.push(vertex[0] as f32);
            positions.push(vertex[1] as f32);
            positions.push(vertex[2] as f32);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);
        face_cells.push(face as u32);
    }
    RenderMeshData {
        positions,
        indices,
        face_cells,
    }
}

/// 渲染网格二进制：KRM1 + 三段元素数（LE u32）+ 位置 f32 / 索引 u32 / 单元 u32。
pub fn encode(mesh: &RenderMeshData) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        16 + 4 * (mesh.positions.len() + mesh.indices.len() + mesh.face_cells.len()),
    );
    bytes.extend_from_slice(b"KRM1");
    for count in [
        mesh.positions.len(),
        mesh.indices.len(),
        mesh.face_cells.len(),
    ] {
        bytes.extend_from_slice(&(count as u32).to_le_bytes());
    }
    for value in &mesh.positions {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for values in [&mesh.indices, &mesh.face_cells] {
        for value in values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    #[test]
    fn render_binary_has_little_endian_arrays_and_counts() {
        let mesh = crate::models::render::RenderMeshData {
            positions: vec![1.0, -2.0, 3.0],
            indices: vec![0, 1, 2],
            face_cells: vec![7],
        };
        let bytes = super::encode(&mesh);
        assert_eq!(&bytes[..4], b"KRM1");
        assert_eq!(&bytes[4..16], &[3, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0]);
        assert_eq!(&bytes[16..20], &1.0f32.to_le_bytes());
        assert_eq!(&bytes[40..44], &7u32.to_le_bytes());
    }

    use super::*;
    use crate::models::geometry::Triangle;

    fn tri(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Triangle {
        Triangle {
            a,
            b,
            c,
            normal: [0.0, 0.0, 1.0],
        }
    }

    #[test]
    fn volume_boundary_faces_carry_owner_cells() {
        // 2 四面体共享面 1-2-3（5 节点）：唯一面 7 个，边界 6 个；
        // 共享面被两个单元使用，不是边界面。
        let volume = VolumeMesh {
            nodes: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -1.0],
            ],
            tets: vec![[0, 1, 2, 3], [0, 1, 3, 4]],
            surface_faces: Vec::new(),
        };
        let render = from_volume_mesh(&volume);
        assert_eq!(render.positions.len(), 5 * 3);
        assert_eq!(render.indices.len(), 6 * 3);
        assert_eq!(render.face_cells.len(), 6);
        // 共享面（节点 0-1-3）不出现在边界面索引里
        let boundary: Vec<Vec<u32>> = render
            .indices
            .chunks(3)
            .map(|chunk| chunk.to_vec())
            .collect();
        assert!(!boundary.iter().any(|face| face == &[0, 1, 3]));
        // 共享面两侧单元的其余面 owner 各自正确
        assert!(render.face_cells.contains(&0));
        assert!(render.face_cells.contains(&1));
    }

    #[test]
    fn volume_boundary_faces_point_outward() {
        let volume = VolumeMesh {
            nodes: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            tets: vec![[0, 1, 2, 3]],
            surface_faces: Vec::new(),
        };
        let render = from_volume_mesh(&volume);
        for face in render.indices.chunks(3) {
            assert_eq!(face.len(), 3);
            let a = volume.nodes[face[0] as usize];
            let b = volume.nodes[face[1] as usize];
            let c = volume.nodes[face[2] as usize];
            let normal = [
                (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]),
                (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]),
                (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]),
            ];
            let centroid = [
                (a[0] + b[0] + c[0]) / 3.0,
                (a[1] + b[1] + c[1]) / 3.0,
                (a[2] + b[2] + c[2]) / 3.0,
            ];
            let tet_centroid = [0.25, 0.25, 0.25];
            let away = [
                centroid[0] - tet_centroid[0],
                centroid[1] - tet_centroid[1],
                centroid[2] - tet_centroid[2],
            ];
            assert!(normal[0] * away[0] + normal[1] * away[1] + normal[2] * away[2] > 0.0);
        }
    }

    #[test]
    fn surface_mesh_expands_to_unshared_vertices() {
        let mesh = TriangleMesh {
            triangles: vec![
                tri([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
                tri([0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]),
            ],
        };
        let render = from_surface_mesh(&mesh);
        // 三角形汤：每三角形 3 个独立顶点
        assert_eq!(render.positions.len(), 2 * 9);
        assert_eq!(render.indices, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(render.face_cells, vec![0, 1]);
    }

    #[test]
    fn f64_positions_are_narrowed_to_f32() {
        let mesh = TriangleMesh {
            triangles: vec![tri([0.1, 0.2, 0.3], [0.4, 0.5, 0.6], [0.7, 0.8, 0.9])],
        };
        let render = from_surface_mesh(&mesh);
        assert_eq!(render.positions[0], 0.1_f32);
        assert_eq!(render.positions[8], 0.9_f32);
    }
}
