//! 体积网格落盘：工作区 `mesh/<studyId>/` 下的二进制网格 + JSON 清单。
//!
//! 网格是「几何 + 参数」的确定产物，可重建；落盘只为打开工程时免于重算。
//! 二进制布局（小端）：
//! `[magic KMS\0][u32 node_count][u32 tet_count][u32 face_count]`
//! `[f64 × 3·node_count][u32 × 4·tet_count][u32 × 3·face_count]`。

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{KairosError, Result};
use crate::models::mesh::{MeshRefinement, MeshingReport, VolumeMesh};

const MAGIC: [u8; 4] = *b"KMS\0";
/// 网格二进制文件名。
pub const MESH_BINARY_NAME: &str = "mesh.bin";
/// 网格清单文件名（报告 + 生成参数）。
pub const MESH_MANIFEST_NAME: &str = "mesh.json";

/// 网格清单：报告 + 生成参数（重算或溯源用）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshManifest {
    /// 落盘格式版本（未来不兼容变更时递增）。
    pub version: u32,
    /// 来源几何 id（会话内 id，跨会话由工程文件保持一致）。
    pub geometry_id: String,
    pub target_size: f64,
    pub refinement: Option<MeshRefinement>,
    pub report: MeshingReport,
}

/// 读取结果：清单 + 体积网格。
#[derive(Debug, Clone, PartialEq)]
pub struct StoredMesh {
    pub manifest: MeshManifest,
    pub mesh: VolumeMesh,
}

/// 体积网格 → 二进制。
pub fn encode(mesh: &VolumeMesh) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        16 + mesh.nodes.len() * 24 + mesh.tets.len() * 16 + mesh.surface_faces.len() * 12,
    );
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&(mesh.nodes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(mesh.tets.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(mesh.surface_faces.len() as u32).to_le_bytes());
    for node in &mesh.nodes {
        for value in node {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for tet in &mesh.tets {
        for index in tet {
            bytes.extend_from_slice(&(*index as u32).to_le_bytes());
        }
    }
    for face in &mesh.surface_faces {
        for index in face {
            bytes.extend_from_slice(&(*index as u32).to_le_bytes());
        }
    }
    bytes
}

/// 二进制 → 体积网格（长度与魔数校验，越界即报错不读脏数据）。
pub fn decode(bytes: &[u8]) -> Result<VolumeMesh> {
    let invalid = |reason: &str| KairosError::validation(format!("网格文件无效：{reason}。"));
    if bytes.len() < 16 || bytes[..4] != MAGIC {
        return Err(invalid("魔数不匹配"));
    }
    let read_u32 = |offset: usize| -> u32 {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    };
    let node_count = read_u32(4) as usize;
    let tet_count = read_u32(8) as usize;
    let face_count = read_u32(12) as usize;
    let expected = 16 + node_count * 24 + tet_count * 16 + face_count * 12;
    if bytes.len() < expected {
        return Err(invalid("长度与头部声明不符"));
    }
    let mut offset = 16;
    let mut nodes = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        let mut node = [0.0f64; 3];
        for value in &mut node {
            let mut raw = [0u8; 8];
            raw.copy_from_slice(&bytes[offset..offset + 8]);
            *value = f64::from_le_bytes(raw);
            offset += 8;
        }
        nodes.push(node);
    }
    let mut tets = Vec::with_capacity(tet_count);
    for _ in 0..tet_count {
        let mut tet = [0usize; 4];
        for index in &mut tet {
            *index = read_u32(offset) as usize;
            offset += 4;
        }
        tets.push(tet);
    }
    let mut surface_faces = Vec::with_capacity(face_count);
    for _ in 0..face_count {
        let mut face = [0usize; 3];
        for index in &mut face {
            *index = read_u32(offset) as usize;
            offset += 4;
        }
        surface_faces.push(face);
    }
    Ok(VolumeMesh {
        nodes,
        tets,
        surface_faces,
    })
}

/// 写入工作区网格目录：`mesh.bin` + `mesh.json`。
pub fn write(dir: &Path, manifest: &MeshManifest, mesh: &VolumeMesh) -> Result<()> {
    fs::create_dir_all(dir).map_err(|e| KairosError::io(format!("创建网格目录失败：{e}")))?;
    fs::write(dir.join(MESH_BINARY_NAME), encode(mesh))
        .map_err(|e| KairosError::io(format!("写入网格文件失败：{e}")))?;
    let json =
        serde_json::to_string_pretty(manifest).expect("网格清单为纯数据结构，序列化不会失败");
    fs::write(dir.join(MESH_MANIFEST_NAME), json)
        .map_err(|e| KairosError::io(format!("写入网格清单失败：{e}")))
}

/// 读取工作区网格目录；目录或文件缺失返回 None（可重建，不算错误）。
pub fn read(dir: &Path) -> Result<Option<StoredMesh>> {
    let binary = dir.join(MESH_BINARY_NAME);
    let manifest_path = dir.join(MESH_MANIFEST_NAME);
    if !binary.exists() || !manifest_path.exists() {
        return Ok(None);
    }
    let manifest_json = fs::read_to_string(&manifest_path)
        .map_err(|e| KairosError::io(format!("读取网格清单失败：{e}")))?;
    let manifest: MeshManifest = serde_json::from_str(&manifest_json)
        .map_err(|e| KairosError::validation(format!("网格清单无法解析：{e}")))?;
    let bytes = fs::read(&binary).map_err(|e| KairosError::io(format!("读取网格文件失败：{e}")))?;
    Ok(Some(StoredMesh {
        manifest,
        mesh: decode(&bytes)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::mesh::{MeshQuality, RefineRegion};

    fn report() -> MeshingReport {
        MeshingReport {
            engine: "voxel".into(),
            node_count: 4,
            element_count: 1,
            surface_face_count: 4,
            total_volume: 1.0 / 6.0,
            quality: MeshQuality {
                min_edge_ratio: 1.0,
                avg_edge_ratio: 1.414,
                max_edge_ratio: 1.732,
                min_volume: 1.0 / 6.0,
            },
            aspect_max: 2.449,
            aspect_avg: 2.449,
            thin_feature_hints: vec![],
        }
    }

    fn mesh() -> VolumeMesh {
        VolumeMesh {
            nodes: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            tets: vec![[0, 1, 2, 3]],
            surface_faces: vec![[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]],
        }
    }

    #[test]
    fn round_trip_preserves_mesh() {
        let bytes = encode(&mesh());
        // 头部 16B + 4 节点 × 24B + 1 单元 × 16B + 4 面 × 12B
        assert_eq!(bytes.len(), 16 + 4 * 24 + 16 + 4 * 12);
        assert_eq!(decode(&bytes).unwrap(), mesh());
    }

    #[test]
    fn decode_rejects_garbage_and_truncated() {
        assert!(decode(b"XXXX").is_err());
        let bytes = encode(&mesh());
        assert!(decode(&bytes[..bytes.len() - 1]).is_err());
        let mut wrong_magic = bytes.clone();
        wrong_magic[0] = 0;
        assert!(decode(&wrong_magic).unwrap_err().message().contains("魔数"));
        // 只够头部：声明了节点但长度不足
        assert!(
            decode(&bytes[..16])
                .unwrap_err()
                .message()
                .contains("长度与头部声明不符")
        );
    }

    #[test]
    fn write_and_read_disk_round_trip() {
        let dir = std::env::temp_dir().join(format!("kairos-mesh-store-{}", std::process::id()));
        let manifest = MeshManifest {
            version: 1,
            geometry_id: "g-1".into(),
            target_size: 2.5,
            refinement: Some(MeshRefinement::Region {
                region: RefineRegion {
                    min: [0.0; 3],
                    max: [5.0; 3],
                },
                levels: 1,
            }),
            report: report(),
        };
        write(&dir, &manifest, &mesh()).unwrap();
        let stored = read(&dir).unwrap().unwrap();
        assert_eq!(stored.manifest, manifest);
        assert_eq!(stored.mesh, mesh());

        // 缺文件 → None（可重建，不算错误）
        let empty = std::env::temp_dir().join(format!("kairos-mesh-empty-{}", std::process::id()));
        assert!(read(&empty).unwrap().is_none());

        // 清单损坏 → 明确报错（不静默重算）
        std::fs::write(dir.join(MESH_MANIFEST_NAME), "{ not json").unwrap();
        assert!(read(&dir).unwrap_err().message().contains("无法解析"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_and_read_report_io_failures() {
        let base = std::env::temp_dir().join(format!("kairos-mesh-io-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let manifest = MeshManifest {
            version: 1,
            geometry_id: "g-1".into(),
            target_size: 1.0,
            refinement: None,
            report: report(),
        };

        // 目标路径是文件而不是目录 → 创建目录失败
        let blocked = base.join("blocked");
        std::fs::write(&blocked, "x").unwrap();
        assert!(
            write(&blocked, &manifest, &mesh())
                .unwrap_err()
                .message()
                .contains("创建网格目录失败")
        );

        // 网格二进制位置被目录占住 → 写入失败
        let occupied = base.join("occupied");
        std::fs::create_dir_all(&occupied).unwrap();
        std::fs::create_dir_all(occupied.join(MESH_BINARY_NAME)).unwrap();
        assert!(
            write(&occupied, &manifest, &mesh())
                .unwrap_err()
                .message()
                .contains("写入网格文件失败")
        );

        // 清单位置被目录占住 → 清单写入失败
        let manifest_blocked = base.join("manifest-blocked");
        std::fs::create_dir_all(manifest_blocked.join(MESH_MANIFEST_NAME)).unwrap();
        assert!(
            write(&manifest_blocked, &manifest, &mesh())
                .unwrap_err()
                .message()
                .contains("写入网格清单失败")
        );

        // 清单是目录 → 读取清单失败
        let manifest_dir = base.join("manifest-dir");
        std::fs::create_dir_all(manifest_dir.join(MESH_MANIFEST_NAME)).unwrap();
        std::fs::write(manifest_dir.join(MESH_BINARY_NAME), encode(&mesh())).unwrap();
        assert!(
            read(&manifest_dir)
                .unwrap_err()
                .message()
                .contains("读取网格清单失败")
        );

        // 读取：mesh.bin 是目录 → 读取失败；二进制内容损坏 → 解码报错
        let corrupt = base.join("corrupt");
        write(&corrupt, &manifest, &mesh()).unwrap();
        std::fs::remove_file(corrupt.join(MESH_BINARY_NAME)).unwrap();
        std::fs::create_dir_all(corrupt.join(MESH_BINARY_NAME)).unwrap();
        assert!(
            read(&corrupt)
                .unwrap_err()
                .message()
                .contains("读取网格文件失败")
        );
        std::fs::remove_dir_all(corrupt.join(MESH_BINARY_NAME)).unwrap();
        std::fs::write(corrupt.join(MESH_BINARY_NAME), b"XXXX__").unwrap();
        assert!(read(&corrupt).unwrap_err().message().contains("魔数"));
        std::fs::remove_dir_all(&base).ok();
    }
}
