//! OpenFOAM case 生成与进度解析（纯函数，无进程操作）。
//! 输出为标准 OpenFOAM case 目录：constant/polyMesh + 0/ 场 + system/ 字典。
//! 字典键名以 openInjMoldSim（v7.2，OpenFOAM 7 .org）为基准；
//! 求解器专属参数的精确校准在 T11 端到端时用官方 dogbone 样例复核。

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::{KairosError, Result};
use crate::models::material::Material;
use crate::models::mesh::VolumeMesh;
use crate::models::process::ProcessSettings;
use crate::models::solver::AnalysisStage;

/// 从体积网格写出 constant/polyMesh（points/faces/owner/neighbour/boundary）。
/// 四面体绕向已在生成时保证正体积；面法向按 owner 外法向定向。
pub fn write_poly_mesh(case_dir: &Path, mesh: &VolumeMesh) -> Result<()> {
    let poly = case_dir.join("constant/polyMesh");
    fs::create_dir_all(&poly)
        .map_err(|e| KairosError::io(format!("创建 polyMesh 目录失败：{e}")))?;

    // 收集所有面：键 = 排序节点三元组；值 = (owner, 有序顶点, neighbour: Option)
    let mut faces: Vec<[usize; 3]> = Vec::new();
    let mut face_index: HashMap<[usize; 3], usize> = HashMap::new();
    let mut owner: Vec<usize> = Vec::new();
    let mut neighbour: Vec<Option<usize>> = Vec::new();

    for (cell, tet) in mesh.tets.iter().enumerate() {
        for face in [
            [tet[0], tet[1], tet[2]],
            [tet[0], tet[1], tet[3]],
            [tet[0], tet[2], tet[3]],
            [tet[1], tet[2], tet[3]],
        ] {
            let mut key = face;
            key.sort_unstable();
            match face_index.get(&key) {
                Some(&existing) => {
                    neighbour[existing] = Some(cell);
                }
                None => {
                    face_index.insert(key, faces.len());
                    faces.push(face);
                    owner.push(cell);
                    neighbour.push(None);
                }
            }
        }
    }

    // 面 st 已按节点升序去重；按 owner 质心定向：若面法向指向 owner 内部则交换后两点。
    // 这里用重心比较实现：面中心到 owner 质心的向量与面法向点积 < 0 → 交换。
    let mut face_lines = Vec::with_capacity(faces.len());
    let mut owner_lines = Vec::with_capacity(faces.len());
    let mut neighbour_lines = Vec::new();
    for (index, face) in faces.iter().enumerate() {
        let (p0, p1, p2) = (
            mesh.nodes[face[0]],
            mesh.nodes[face[1]],
            mesh.nodes[face[2]],
        );
        let owner_cell = mesh.tets[owner[index]];
        let owner_centre = [
            (mesh.nodes[owner_cell[0]][0]
                + mesh.nodes[owner_cell[1]][0]
                + mesh.nodes[owner_cell[2]][0]
                + mesh.nodes[owner_cell[3]][0])
                / 4.0,
            (mesh.nodes[owner_cell[0]][1]
                + mesh.nodes[owner_cell[1]][1]
                + mesh.nodes[owner_cell[2]][1]
                + mesh.nodes[owner_cell[3]][1])
                / 4.0,
            (mesh.nodes[owner_cell[0]][2]
                + mesh.nodes[owner_cell[1]][2]
                + mesh.nodes[owner_cell[2]][2]
                + mesh.nodes[owner_cell[3]][2])
                / 4.0,
        ];
        let face_centre = [
            (p0[0] + p1[0] + p2[0]) / 3.0,
            (p0[1] + p1[1] + p2[1]) / 3.0,
            (p0[2] + p1[2] + p2[2]) / 3.0,
        ];
        let normal = [
            (p1[1] - p0[1]) * (p2[2] - p0[2]) - (p1[2] - p0[2]) * (p2[1] - p0[1]),
            (p1[2] - p0[2]) * (p2[0] - p0[0]) - (p1[0] - p0[0]) * (p2[2] - p0[2]),
            (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p1[1] - p0[1]) * (p2[0] - p0[0]),
        ];
        let to_owner = [
            owner_centre[0] - face_centre[0],
            owner_centre[1] - face_centre[1],
            owner_centre[2] - face_centre[2],
        ];
        let outward =
            normal[0] * to_owner[0] + normal[1] * to_owner[1] + normal[2] * to_owner[2] < 0.0;
        let (a, b, c) = if outward {
            (face[0], face[1], face[2])
        } else {
            (face[0], face[2], face[1])
        };
        face_lines.push(format!("3({a} {b} {c})"));
        owner_lines.push(owner[index].to_string());
        if let Some(n) = neighbour[index] {
            neighbour_lines.push(n.to_string());
        }
    }

    let n_faces = faces.len();
    let n_internal = neighbour_lines.len();
    let points_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"vectorField\";\n    object points;\n}}\n{}\n(\n{})\n",
        mesh.nodes.len(),
        mesh.nodes
            .iter()
            .map(|p| format!("({:.6} {:.6} {:.6})", p[0], p[1], p[2]))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let faces_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"faceList\";\n    object faces;\n}}\n{}\n(\n{})\n",
        n_faces,
        face_lines.join("\n")
    );
    let owner_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"labelList\";\n    object owner;\n}}\n{}\n(\n{})\n",
        n_faces,
        owner_lines.join("\n")
    );
    let neighbour_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"labelList\";\n    object neighbour;\n}}\n{}\n(\n{})\n",
        n_internal,
        neighbour_lines.join("\n")
    );
    let boundary_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"polyBoundaryMesh\";\n    object boundary;\n}}\n1\n(\n    walls\n    {{\n        type wall;\n        inGroups 1(wall);\n        nFaces {n_boundary};\n        startFace {n_internal};\n    }}\n)\n",
        n_boundary = n_faces - n_internal,
    );

    write(&poly.join("points"), &points_content)?;
    write(&poly.join("faces"), &faces_content)?;
    write(&poly.join("owner"), &owner_content)?;
    write(&poly.join("neighbour"), &neighbour_content)?;
    write(&poly.join("boundary"), &boundary_content)?;
    Ok(())
}

/// 写出 0/ 场与 system/、constant/ 字典（v1 模板；求解器专属校准在 T11 完成）。
pub fn write_case_files(
    case_dir: &Path,
    material: &Material,
    process: &ProcessSettings,
    stage: &AnalysisStage,
    cores: usize,
) -> Result<()> {
    let zero = case_dir.join("0");
    let system = case_dir.join("system");
    let constant = case_dir.join("constant");
    for dir in [&zero, &system, &constant] {
        fs::create_dir_all(dir).map_err(|e| KairosError::io(format!("创建 {dir:?} 失败：{e}")))?;
    }

    let end_time = stage.end_time_s(
        process.injection_time_s,
        process.packing_time_s,
        process.cooling_time_s,
    );
    write(&system.join("controlDict"), &control_dict(end_time, cores))?;
    write(&system.join("decomposeParDict"), &decompose_dict(cores))?;
    write(&system.join("fvSchemes"), FV_SCHEMES)?;
    write(&system.join("fvSolution"), FV_SOLUTION)?;
    write(
        &constant.join("transportProperties"),
        &transport_dict(material),
    )?;
    write(&constant.join("polyMesh").join(".keep"), "")?;

    let zero_gradient = |field: &str, dims: &str, value: &str| {
        format!(
            "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"volScalarField\";\n    object {field};\n}}\ndimensions [{dims}];\ninternalField uniform {value};\nboundaryField\n{{\n    walls\n    {{\n        type zeroGradient;\n    }}\n}}\n"
        )
    };
    let t_content = zero_gradient("T", "0 0 0 0 0 1 0", &format!("{:.2}", process.melt_temp_c));
    let p_content = zero_gradient("p", "1 -1 0 0 0 0 0", "1e5");
    write(&zero.join("T"), &t_content)?;
    write(&zero.join("p"), &p_content)?;
    let u_content = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class \"volVectorField\";\n    object U;\n}\ndimensions [0 1 -1 0 0 0 0];\ninternalField uniform (0 0 0);\nboundaryField\n{\n    walls\n    {\n        type noSlip;\n    }\n}\n";
    write(&zero.join("U"), u_content)?;
    Ok(())
}

/// 生成完整 case：polyMesh + 场 + 字典。
pub fn generate_case(
    case_dir: &Path,
    mesh: &VolumeMesh,
    material: &Material,
    process: &ProcessSettings,
    stage: &AnalysisStage,
    cores: usize,
) -> Result<()> {
    write_poly_mesh(case_dir, mesh)?;
    write_case_files(case_dir, material, process, stage, cores)
}

/// 解析求解器 stdout 中的时间步行（如 "Time = 0.05"），返回物理进度秒数。
pub fn parse_time_line(line: &str) -> Option<f64> {
    let trimmed = line.trim();
    trimmed
        .strip_prefix("Time = ")
        .and_then(|rest| rest.split(' ').next())
        .and_then(|value| value.parse::<f64>().ok())
}

fn write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| KairosError::io(format!("创建目录失败：{e}")))?;
    }
    fs::write(path, content).map_err(|e| KairosError::io(format!("写入 {path:?} 失败：{e}")))
}

fn control_dict(end_time: f64, cores: usize) -> String {
    format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"dictionary\";\n    object controlDict;\n}}\napplication     openInjMoldSim;\nstartFrom       latestTime;\nstopAt          endTime;\nendTime         {end_time:.6};\ndeltaT          1e-4;\nwriteControl    adjustableRunTime;\nwriteInterval   {end_time:.4};\npurgeWrite      3;\nwriteFormat     ascii;\nwritePrecision  8;\ntimeFormat      general;\nrunTimeModifiable false;\n// 并行核数提示：{cores}（由 decomposeParDict 生效）\n"
    )
}

fn decompose_dict(cores: usize) -> String {
    format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"dictionary\";\n    object decomposeParDict;\n}}\nnumberOfSubdomains {cores};\nmethod          scotch;\n"
    )
}

fn transport_dict(material: &Material) -> String {
    let r = &material.rheology;
    let t = &material.pvt;
    format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"dictionary\";\n    object transportProperties;\n}}\n// Cross-WLF 黏度参数（材料：{} / {}）\nrho0            rho0 [1 -3 0 0 0 0 0] 1000;\nn               n [0 0 0 0 0 0 0] {};\ntauStar         tauStar [1 -1 0 0 0 0 0] {};\nD1              D1 [1 -1 0 0 0 0 0] {};\nD2              D2 [0 0 0 0 0 0 0] {};\nD3              D3 [0 0 0 -1 0 0 0] {};\nA1              A1 [0 0 0 0 0 0 0] {};\nA2              A2 [0 0 0 0 0 0 0] {};\n// Tait PVT\nb1m             b1m [0 -3 0 0 0 0 0] {};\nb1s             b1s [0 -3 0 0 0 0 0] {};\nb2m             b2m [0 -3 0 0 0 0 0] {};\nb2s             b2s [0 -3 0 0 0 0 0] {};\nb3              b3 [1 -1 2 0 0 0 0] {};\nb4m             b4m [0 0 2 -1 0 0 0] {};\nb4s             b4s [0 0 2 -1 0 0 0] {};\nb5              b5 [0 0 0 0 0 0 0] {};\n",
        material.manufacturer,
        material.family,
        r.n,
        r.tau_star,
        r.d1,
        r.d2,
        r.d3,
        r.a1,
        r.a2,
        t.b1m,
        t.b1s,
        t.b2m,
        t.b2s,
        t.b3,
        t.b4m,
        t.b4s,
        t.b5
    )
}

const FV_SCHEMES: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class \"dictionary\";\n    object fvSchemes;\n}\nddtSchemes { default Euler; }\ngradSchemes { default Gauss linear; }\ndivSchemes { default none; div(phi,U) Gauss linearUpwind grad(U); div(phi,T) Gauss limitedLinear 1; }\nlaplacianSchemes { default Gauss linear corrected; }\ninterpolationSchemes { default linear; }\nsnGradSchemes { default corrected; }\n";

const FV_SOLUTION: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class \"dictionary\";\n    object fvSolution;\n}\nsolvers\n{\n    \"(U|p|T)\"\n    {\n        solver          PBiCGStab;\n        preconditioner  DILU;\n        tolerance       1e-7;\n        relTol          0.01;\n    }\n}\n";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::process::ProcessSettings;

    fn two_tet_mesh() -> VolumeMesh {
        // 共享面 (1,2,3) 的两个正四面体
        VolumeMesh {
            nodes: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -1.0],
            ],
            tets: vec![[0, 1, 2, 3], [0, 2, 1, 4]],
            surface_faces: vec![
                [0, 1, 3],
                [1, 2, 3],
                [0, 2, 3],
                [1, 2, 4],
                [0, 2, 4],
                [0, 1, 4],
            ],
        }
    }

    fn process() -> ProcessSettings {
        ProcessSettings {
            melt_temp_c: 230.0,
            mold_temp_c: 40.0,
            ejection_temp_c: 90.0,
            injection_time_s: 1.0,
            vp_switch_volume_percent: 96.0,
            packing_pressure_mpa_curve: vec![(0.0, 60.0), (8.0, 40.0)],
            packing_time_s: 8.0,
            cooling_time_s: 15.0,
            coolant_temp_c: 25.0,
        }
    }

    #[test]
    fn case_generation_writes_full_directory() {
        let dir = std::env::temp_dir().join(format!("kairos-t09-{}", std::process::id()));
        let case = dir.join("case");
        generate_case(
            &case,
            &two_tet_mesh(),
            &crate::services::material::builtin_materials().unwrap()[0],
            &process(),
            &AnalysisStage::Fill,
            4,
        )
        .unwrap();

        for path in [
            "constant/polyMesh/points",
            "constant/polyMesh/faces",
            "constant/polyMesh/owner",
            "constant/polyMesh/neighbour",
            "constant/polyMesh/boundary",
            "0/T",
            "0/U",
            "0/p",
            "system/controlDict",
            "system/decomposeParDict",
            "system/fvSchemes",
            "system/fvSolution",
            "constant/transportProperties",
        ] {
            assert!(case.join(path).exists(), "缺少 {path}");
        }

        // 双四面体：6 个面，其中 1 个内部面
        let faces = fs::read_to_string(case.join("constant/polyMesh/faces")).unwrap();
        assert!(faces.contains("\n7\n")); // 2 四面体：6 边界面 + 1 内部面
        let neighbour = fs::read_to_string(case.join("constant/polyMesh/neighbour")).unwrap();
        assert!(neighbour.contains("\n1\n"));
        // 控制字典 endTime = 注射×2（Fill 阶段）
        let control = fs::read_to_string(case.join("system/controlDict")).unwrap();
        assert!(control.contains("endTime         2.000000"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn progress_line_parses_time() {
        assert_eq!(parse_time_line("Time = 0.05"), Some(0.05));
        assert_eq!(parse_time_line("  Time = 12.5 seconds"), Some(12.5));
        assert_eq!(parse_time_line("no time here"), None);
    }
}
