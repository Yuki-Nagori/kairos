//! moldingFoam case 生成与进度解析（纯函数，无进程操作）。
//! 输出为标准 OpenFOAM case 目录：constant/polyMesh + 0/ 场 + system/ 字典。
//! 字典布局对齐 moldingFoam 的 `case-contract/` v1.1 对接规范（关键字冻结）：
//! 求解入口为 foamRun + moldingFoam 模块（libs 加载 libmoldingFoam.so）。
//! 边界面按包围盒 z 分带启发式分类：底带 = inlet（浇口）、顶带 = vent（排气）、
//! 其余 = walls（模壁）——轴向模具假设，浇口几何落地后替换为精确分类。

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::{KairosError, Result};
use crate::models::material::Material;
use crate::models::mesh::VolumeMesh;
use crate::models::process::ProcessSettings;
use crate::models::runners::{RunnerElement, RunnerKind};
use crate::models::solver::AnalysisStage;

/// 求解入口单点：foamRun 框架的求解模块名。bundle 自带
/// libmoldingFoamSolver.so 探测链接，libs 行同时显式加载（双保险）。
pub const SOLVER_MODULE: &str = "moldingFoam";

/// 网格坐标单位 → SI 换算。网格与工艺侧一律用 mm；case 必须写米——契约的
/// blockMeshDict 是「mm 顶点 + scale 0.001」，Kairos 直接写 polyMesh，
/// 换算在这里落地（否则求解器把 10 mm 读成 10 m，与 SI 物性组合后量纲整体
/// 放大，静水压与前沿/逃逸行为都不是目标件的物理）。
const MM_TO_M: f64 = 1e-3;

/// 排气口密封判据：排气口熔体体积分数达到该值后 vent 密封（契约的
/// ventSealAlpha；密封期间闸口封冻、型腔停止排料）。
const VENT_SEAL_ALPHA: f64 = 0.9;

/// 质量预算诊断间隔（步）：逐 patch 打印熔体通量，填充停滞或逃逸时可直接
/// 看到 `patch vent: alphaPhi1` 的量级。
const MASS_BUDGET_INTERVAL: u32 = 200;

/// const 输运的熔体动力黏度占位 [Pa·s]：黏度的物理模型由 momentumTransport 的
/// CrossWlf 提供，这里只作为 `Pr` 的换算基准——**产物 `k = mu·Cp/Pr` 必须等于
/// 材料导热系数的真值**（见 physical_properties_melt）。
const MELT_TRANSPORT_MU: f64 = 100.0;

/// 冻死短射守卫的可动熔体占比阈值（与求解器缺省一致）：低于该占比即判短射，
/// 封闸并把阶段切到保压，避免冻结区把解推到非有限。
const FREEZE_OFF_FRACTION: f64 = 0.01;

/// 保压压力斜坡：V/P 切换后闸口压力从实测值线性升到保压曲线，避免单步阶跃
/// （大件实测切换瞬间 p_gate 仅 5.4 MPa、曲线起点 60 MPa，阶跃会把熔体推到
/// 不可解）。斜坡时长按注射时间的固定比例给，再夹到工程区间。
const PACKING_RAMP_OF_INJECTION: f64 = 0.05;
const PACKING_RAMP_MIN_S: f64 = 0.05;
const PACKING_RAMP_MAX_S: f64 = 0.5;

/// 边界分带比例：底带 = 浇口（inlet），顶带 = 排气（vent），其余 = 模壁。
const INLET_BAND: f64 = 0.05;
const VENT_BAND: f64 = 0.05;

/// 摄氏度 → 开尔文。
fn kelvin(celsius: f64) -> f64 {
    celsius + 273.15
}

/// 从温度物性表（温度 K 升序）取值：区间内线性插值，超界取最近端点。
fn table_value_at(table: &[(f64, f64)], temperature: f64) -> Option<f64> {
    let first = table.first()?;
    if temperature <= first.0 {
        return Some(first.1);
    }
    for window in table.windows(2) {
        let (t0, v0) = window[0];
        let (t1, v1) = window[1];
        if temperature <= t1 {
            let w = (temperature - t0) / (t1 - t0);
            return Some(v0 + w * (v1 - v0));
        }
    }
    table.last().map(|(_, v)| *v)
}

/// 网格包围盒 z 高度带分类（无浇口时的回退启发式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundaryBand {
    Inlet,
    Vent,
    Walls,
}

/// 边界面外法向 z 分量的下限：低于它不算「朝下/朝上」的面。
/// 体素网格的边界是阶梯面，底/顶带里混着大量侧向面；把它们计入 inlet/vent
/// 会让熔体从零件外壁注入（真实件实测该误差占入口面的 39%）。
const NORMAL_Z_MIN: f64 = 0.5;

/// 边界面 → patch（无浇口回退）：底带且朝下 = inlet，顶带且朝上 = vent，
/// 其余（含阶梯侧向面）归 walls。
fn classify_boundary_face(z_centroid: f64, normal_z: f64, z_min: f64, z_max: f64) -> BoundaryBand {
    let height = (z_max - z_min).max(1e-9);
    if z_centroid <= z_min + INLET_BAND * height && normal_z <= -NORMAL_Z_MIN {
        BoundaryBand::Inlet
    } else if z_centroid >= z_max - VENT_BAND * height && normal_z >= NORMAL_Z_MIN {
        BoundaryBand::Vent
    } else {
        BoundaryBand::Walls
    }
}

/// 浇口入口：与型腔相接的位置（网格坐标 mm）与等效半径（mm）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GatePortal {
    pub center: [f64; 3],
    pub radius_mm: f64,
}

/// 写出的三个 patch 的面积（m²，SI）：排气口用等效流通面积 `CdA`，其余供
/// 诊断与后续边界条件使用。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatchAreas {
    pub inlet_m2: f64,
    pub vent_m2: f64,
    pub walls_m2: f64,
}

/// 浇口半径下限：体素网格上浇口常小于一个单元，靠「最近面补齐」保底，
/// 下限只用于避免半径为 0。
const GATE_MIN_RADIUS_MM: f64 = 0.5;

/// 流道网络 → 浇口入口表：取 `kind == Gate` 的单元，入口位置取 `end`
/// （约定：Gate 的 `start` 接流道、`end` 接型腔），半径 = 直径 / 2。
pub fn gate_portals(runners: &[RunnerElement]) -> Vec<GatePortal> {
    runners
        .iter()
        .filter(|runner| runner.kind == RunnerKind::Gate)
        .map(|runner| GatePortal {
            center: runner.end,
            radius_mm: (runner.diameter_mm / 2.0).max(GATE_MIN_RADIUS_MM),
        })
        .collect()
}

fn distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    ((left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2) + (left[2] - right[2]).powi(2))
        .sqrt()
}

/// 按浇口圈定入口面：半径内的边界面全取；等效面积（πr²）不足时按距离从近到远
/// 继续补足——体素网格上浇口常小于一个单元，没有补足就会出现「入口为空」的
/// case（求解器直接没有进料口）。
fn gate_inlet_faces(
    boundary: &[usize],
    centres: &[[f64; 3]],
    areas: &[f64],
    gates: &[GatePortal],
) -> Vec<usize> {
    let mut inlet: Vec<usize> = Vec::new();
    for gate in gates {
        let target_area = std::f64::consts::PI * gate.radius_mm * gate.radius_mm;
        let mut candidates: Vec<(f64, usize)> = boundary
            .iter()
            .map(|&index| (distance(centres[index], gate.center), index))
            .collect();
        candidates.sort_by(|left, right| {
            left.0
                .partial_cmp(&right.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut area = 0.0;
        for (distance, index) in candidates {
            if distance > gate.radius_mm && area >= target_area {
                break;
            }
            inlet.push(index);
            area += areas[index];
        }
    }
    inlet.sort_unstable();
    inlet.dedup();
    inlet
}

/// 边界面分带：先由浇口圈定 inlet，其余按分带回退（给了浇口时底带不再是入口）。
/// 返回 [inlet, vent, walls] 三组面序号。
fn classify_boundary_faces(
    neighbour: &[Option<usize>],
    face_centres: &[[f64; 3]],
    face_areas: &[f64],
    face_normal_z: &[f64],
    (z_min, z_max): (f64, f64),
    gates: &[GatePortal],
) -> [Vec<usize>; 3] {
    let boundary: Vec<usize> = (0..neighbour.len())
        .filter(|index| neighbour[*index].is_none())
        .collect();
    let mut is_gate_face = vec![false; neighbour.len()];
    for index in gate_inlet_faces(&boundary, face_centres, face_areas, gates) {
        is_gate_face[index] = true;
    }
    let mut band_faces: [Vec<usize>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for &index in &boundary {
        if is_gate_face[index] {
            band_faces[0].push(index);
            continue;
        }
        let band =
            classify_boundary_face(face_centres[index][2], face_normal_z[index], z_min, z_max);
        band_faces[match band {
            // 给了浇口时底带不再是入口（避免「浇口 + 整条底带」双重入口）
            BoundaryBand::Inlet if gates.is_empty() => 0,
            BoundaryBand::Inlet => 2,
            BoundaryBand::Vent => 1,
            BoundaryBand::Walls => 2,
        }]
        .push(index);
    }
    band_faces
}

/// 从体积网格写出 constant/polyMesh（points/faces/owner/neighbour/boundary）。
/// 四面体绕向已在生成时保证正体积；面法向按 owner 外法向定向。
///
/// 边界面重排为 inlet / vent / walls 三个连续 patch 区段：
/// - 给了浇口（`gates` 非空）→ inlet 只取浇口圈定的面；
/// - 没给浇口 → 回退 z 分带启发式，且只认朝下/朝上的面（阶梯侧向面归 walls）。
pub fn write_poly_mesh(
    case_dir: &Path,
    mesh: &VolumeMesh,
    gates: &[GatePortal],
) -> Result<PatchAreas> {
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

    // 面法向按 owner 外法向定向（点积判定，反向则交换后两点）。
    let mut face_lines = Vec::with_capacity(faces.len());
    let mut face_centres: Vec<[f64; 3]> = Vec::with_capacity(faces.len());
    let mut face_areas: Vec<f64> = Vec::with_capacity(faces.len());
    let mut face_normal_z: Vec<f64> = Vec::with_capacity(faces.len());
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
        // 定向下标后按最小节点开头循环（OpenFOAM 的 upper-triangular 约定，
        // 循环不改变绕向）：违反该约定 checkMesh 报 "Faces not in upper
        // triangular order"。
        let mut ordered_face = if outward {
            [face[0], face[1], face[2]]
        } else {
            [face[0], face[2], face[1]]
        };
        if ordered_face[1] < ordered_face[0] && ordered_face[1] < ordered_face[2] {
            ordered_face.rotate_left(1);
        } else if ordered_face[2] < ordered_face[0] {
            ordered_face.rotate_right(1);
        }
        // |cross| = 2 × 面积；法向符号已由 outward 定向，朝下时取负 z 分量。
        let length = (normal[0].powi(2) + normal[1].powi(2) + normal[2].powi(2)).sqrt();
        let unit_z = if length > 0.0 {
            (if outward { normal[2] } else { -normal[2] }) / length
        } else {
            0.0
        };
        face_centres.push(face_centre);
        face_areas.push(length / 2.0);
        face_normal_z.push(unit_z);
        face_lines.push(format!(
            "3({} {} {})",
            ordered_face[0], ordered_face[1], ordered_face[2]
        ));
    }

    // 输出顺序：内部面 → inlet → vent → walls。三个数组必须**逐项对齐**——
    // faces/owner/neighbour 是并行列表，任何单独重排都会让 owner 与面错位，
    // 单元体积随即出现负值（checkMesh: zero or negative cell volume）。
    // 边界面按 z 分带分类，每个 patch 的面保持连续（boundary 的 startFace）。
    let (z_min, z_max) = mesh.nodes.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p[2]), hi.max(p[2]))
    });
    let mut order: Vec<usize> = Vec::with_capacity(faces.len());
    for (index, slot) in neighbour.iter().enumerate() {
        if slot.is_some() {
            order.push(index);
        }
    }
    // OpenFOAM 要求内部面按 (owner, neighbour) 升序（"upper triangular order"）：
    // 每个单元的面按其邻居升序排列，否则 checkMesh 报 faces not in upper
    // triangular order。
    order.sort_by_key(|&index| (owner[index], neighbour[index].unwrap_or(0)));
    let n_internal = order.len();
    let band_faces = classify_boundary_faces(
        &neighbour,
        &face_centres,
        &face_areas,
        &face_normal_z,
        (z_min, z_max),
        gates,
    );
    let mut boundary_patches: Vec<(&str, &str, usize, usize)> = Vec::new();
    let mut start_face = n_internal;
    for (slot, (name, patch_type)) in ["inlet", "vent", "walls"]
        .iter()
        .zip(["wall", "patch", "wall"])
        .enumerate()
    {
        order.extend_from_slice(&band_faces[slot]);
        boundary_patches.push((name, patch_type, start_face, band_faces[slot].len()));
        start_face += band_faces[slot].len();
    }

    let patch_area_m2 = |slot: usize| -> f64 {
        band_faces[slot]
            .iter()
            .map(|&index| face_areas[index])
            .sum::<f64>()
            * MM_TO_M
            * MM_TO_M
    };
    let areas = PatchAreas {
        inlet_m2: patch_area_m2(0),
        vent_m2: patch_area_m2(1),
        walls_m2: patch_area_m2(2),
    };

    let ordered = |pick: &dyn Fn(usize) -> String| -> Vec<String> {
        order.iter().map(|&index| pick(index)).collect()
    };
    let owner_lines = ordered(&|index| owner[index].to_string());
    let neighbour_lines: Vec<String> = order[..n_internal]
        .iter()
        .map(|&index| neighbour[index].map(|n| n.to_string()).unwrap_or_default())
        .collect();

    let points_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"vectorField\";\n    object points;\n}}\n{}\n(\n{})\n",
        mesh.nodes.len(),
        mesh.nodes
            .iter()
            // 坐标换算到米：求解器把数值当 SI 读，写 mm 数值会让整件事放大千倍
            .map(|p| {
                format!(
                    "({:.6} {:.6} {:.6})",
                    p[0] * MM_TO_M,
                    p[1] * MM_TO_M,
                    p[2] * MM_TO_M
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    );
    let faces_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"faceList\";\n    object faces;\n}}\n{}\n(\n{})\n",
        faces.len(),
        ordered(&|index| face_lines[index].clone()).join("\n")
    );
    let owner_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"labelList\";\n    object owner;\n}}\n{}\n(\n{})\n",
        faces.len(),
        owner_lines.join("\n")
    );
    let neighbour_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"labelList\";\n    object neighbour;\n}}\n{}\n(\n{})\n",
        n_internal,
        neighbour_lines.join("\n")
    );
    let mut boundary_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"polyBoundaryMesh\";\n    object boundary;\n}}\n{}\n(\n",
        boundary_patches.len()
    );
    for (patch_name, patch_type, patch_start, patch_count) in &boundary_patches {
        // inlet/vent 是进出口而非壁面（vent 与契约 case 同为 patch 类型），
        // 只有 walls 进 wall 组。
        let in_groups = if *patch_type == "wall" {
            "        inGroups 1(wall);\n"
        } else {
            ""
        };
        boundary_content.push_str(&format!(
            "    {patch_name}\n    {{\n        type {patch_type};\n{in_groups}        nFaces {patch_count};\n        startFace {patch_start};\n    }}\n"
        ));
    }
    boundary_content.push_str(")\n");

    write(&poly.join("points"), &points_content)?;
    write(&poly.join("faces"), &faces_content)?;
    write(&poly.join("owner"), &owner_content)?;
    write(&poly.join("neighbour"), &neighbour_content)?;
    write(&poly.join("boundary"), &boundary_content)?;
    Ok(areas)
}

/// 网格体积（m³）：四面体有向体积求和（绕向已保证正值）。
pub fn mesh_volume(mesh: &VolumeMesh) -> f64 {
    mesh.tets
        .iter()
        .map(|tet| {
            let (a, b, c, d) = (
                mesh.nodes[tet[0]],
                mesh.nodes[tet[1]],
                mesh.nodes[tet[2]],
                mesh.nodes[tet[3]],
            );
            let (ab, ac, ad) = (
                [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
                [d[0] - a[0], d[1] - a[1], d[2] - a[2]],
            );
            (ab[0] * (ac[1] * ad[2] - ac[2] * ad[1]) - ab[1] * (ac[0] * ad[2] - ac[2] * ad[0])
                + ab[2] * (ac[0] * ad[1] - ac[1] * ad[0]))
                .abs()
                / 6.0
        })
        .sum()
}

/// 写出 0/ 场与 system/、constant/ 字典（case-contract v1.1 布局）。
pub fn write_case_files(
    case_dir: &Path,
    mesh: &VolumeMesh,
    material: &Material,
    process: &ProcessSettings,
    stage: &AnalysisStage,
    cores: usize,
    areas: &PatchAreas,
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
    write(&system.join("controlDict"), &control_dict(end_time))?;
    write(&system.join("decomposeParDict"), &decompose_dict(cores))?;
    write(&system.join("fvSchemes"), FV_SCHEMES)?;
    write(&system.join("fvSolution"), FV_SOLUTION)?;

    let dict = molding_dict(material, process);
    write(&constant.join("moldingDict"), &dict)?;
    write(
        &constant.join("momentumTransport"),
        &momentum_transport_dict(material),
    )?;
    write(&constant.join("phaseProperties"), PHASE_PROPERTIES)?;
    write(&constant.join("g"), G_DICT)?;
    write(&constant.join("fvModels"), FV_MODELS)?;
    write(
        &constant.join("physicalProperties.melt"),
        &physical_properties_melt(material, process)?,
    )?;
    write(
        &constant.join("physicalProperties.air"),
        PHYSICAL_PROPERTIES_AIR,
    )?;

    let melt_k = kelvin(process.melt_temp_c);
    let mold_k = kelvin(process.mold_temp_c);
    // 注射流量 = 型腔体积 / 注射时间（moldingInletVelocity 体积流量口径，m³/s）。
    let flow_rate = mesh_volume(mesh) * MM_TO_M.powi(3) / process.injection_time_s.max(1e-9);
    write(&zero.join("alpha.melt"), ALPHA_MELT)?;
    write(&zero.join("U"), &u_dict(flow_rate))?;
    write(&zero.join("p"), P_DICT)?;
    write(&zero.join("p_rgh"), &p_rgh_dict(areas.vent_m2))?;
    write(&zero.join("T"), &t_dict(melt_k, mold_k))?;
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
    gates: &[GatePortal],
) -> Result<PatchAreas> {
    let areas = write_poly_mesh(case_dir, mesh, gates)?;
    write_case_files(case_dir, mesh, material, process, stage, cores, &areas)?;
    Ok(areas)
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
    fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|e| KairosError::io(format!("创建目录失败：{e}")))?;
    fs::write(path, content).map_err(|e| KairosError::io(format!("写入 {path:?} 失败：{e}")))
}

/// 求解命令（不含 cd 与环境 source）：分解网格后以 mpirun 拉起并行 foamRun，
/// 最后重建 case 级结果目录。
///
/// `foamRun` 不会自行调用 mpirun——直接 `foamRun -parallel` 会以
/// "attempt to run parallel on 1 processor" 退出；`-np` 必须与
/// decomposeParDict 的 numberOfSubdomains（同一 `cores`）一致。
///
/// 并行求解把结果写在 `processor*/` 下，必须 `reconstructPar` 之后宿主侧的
/// results 服务才读得到 case 级时间目录；用 `;` 而非 `&&` 串接，使求解器退出
/// 码异常时仍尽力重建已写出的部分结果（部分结果对排查有用）。
pub fn solve_command(cores: u32) -> String {
    format!("decomposePar -force && mpirun -np {cores} foamRun -parallel; reconstructPar")
}

/// 求解器输出行是否表示异常退出路径。
///
/// 求解器（含 OpenFOAM 的 argList 级失败）报错时必打印 `FOAM FATAL ERROR` /
/// `FOAM exiting` / `FOAM aborting`，据此把「求解器主动报错退出」与「进程被
/// 终止 / 崩溃（无任何求解器错误标记）」在作业失败信息里区分开。
pub fn is_abort_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains("FOAM FATAL")
        || trimmed.contains("FOAM exiting")
        || trimmed.contains("FOAM aborting")
}

/// FoamFile 头（无横幅注释的精简形态，OpenFOAM 原生接受）。
fn foam_header(class: &str, object: &str) -> String {
    format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"{class}\";\n    object {object};\n}}\n"
    )
}

fn control_dict(end_time: f64) -> String {
    foam_header("dictionary", "controlDict")
        + &format!(
            "application     foamRun;\nsolver          {SOLVER_MODULE};\nlibs            (\"libmoldingFoam.so\");\nstartFrom       startTime;\nstartTime       0;\nstopAt          endTime;\nendTime         {end_time:.6};\ndeltaT          1e-05;\nwriteControl    adjustableRunTime;\nwriteInterval   0.1;\npurgeWrite      0;\nwriteFormat     ascii;\nwritePrecision  7;\nrunTimeModifiable yes;\nadjustTimeStep  on;\nmaxCo           0.5;\nmaxAlphaCo      0.1;\nmaxDeltaT       1;\n"
        )
}

fn decompose_dict(cores: usize) -> String {
    foam_header("dictionary", "decomposeParDict")
        + &format!("numberOfSubdomains {cores};\nmethod          scotch;\n")
}

/// 工艺字典：V/P 切换分数与切换压力、保压压力表、顶出判据。
///
/// 保压曲线相对 V/P 切换时刻计时（契约的保压阶段）：
/// 切换压力取曲线起点，使闸口压力在切换瞬间连续、无压力阶跃。不能再额外
/// 写一个大气压首点——曲线起点在 t=0 时会出现重复横坐标，被求解器的
/// `Function1s::Table::check` 判为 out-of-order 而拒绝启动。
/// 保压压力斜坡时长 [s]：注射时间的 5%，夹在 [0.05, 0.5]。
fn packing_ramp_s(injection_time_s: f64) -> f64 {
    (injection_time_s * PACKING_RAMP_OF_INJECTION).clamp(PACKING_RAMP_MIN_S, PACKING_RAMP_MAX_S)
}

fn molding_dict(material: &Material, process: &ProcessSettings) -> String {
    let curve = &process.packing_pressure_mpa_curve;
    let mut table = String::new();
    // 空曲线兜底：services::process::validate 会拦下，但生成器被直接调用时
    // 仍要写出求解器可接受的表（单点）。
    if curve.is_empty() {
        table.push_str("            (0 1e5)\n");
    }
    for (time_s, pressure_mpa) in curve {
        table.push_str(&format!(
            "            ({time_s:.4} {:.6e})\n",
            pressure_mpa * 1e6
        ));
    }
    let switch_pressure = curve
        .first()
        .map(|(_, pressure_mpa)| format!("    switchPressure  {:.6e};\n", pressure_mpa * 1e6))
        .unwrap_or_default();
    // 冻死短射守卫：以 Tait 转变温度 b5 作无流温度代理（材料库暂无显式字段）。
    // 开启后冷模/慢充工况在填充早期即判短射并封闸收尾，而不是把冻结区推到非有限。
    // 键位即求解器的读取位置（moldingDict 顶层）；日志里回显的
    // gateFreezeTemperature 是另一个旧键（闸口冻结封闸），不代表本开关的状态。
    foam_header("dictionary", "moldingDict")
        + &format!(
            "injection\n{{\n    meltTemperature  {:.4};\n}}\npacking\n{{\n    switchFraction   {:.4};\n    pressureRamp  {ramp:.4};\n{switch_pressure}    pressure\n    {{\n        type            table;\n        values\n        (\n{table}        );\n    }}\n}}\nventSealAlpha  {VENT_SEAL_ALPHA:.4};\nmassBudget  true;\nmassBudgetInterval  {MASS_BUDGET_INTERVAL};\nfreezeOffTemperature  {:.4};\nfreezeOffFraction  {FREEZE_OFF_FRACTION:.4};\ncooling\n{{\n    ejectionTemperature  {:.4};\n    releasePressure  1e5;\n}}\n",
            kelvin(process.melt_temp_c),
            process.vp_switch_volume_percent / 100.0,
            material.pvt.b5,
            kelvin(process.ejection_temp_c),
            ramp = packing_ramp_s(process.injection_time_s)
        )
}

/// 混合物动量输运：层流广义牛顿 + CrossWlf（系数来自材料库）。
fn momentum_transport_dict(material: &Material) -> String {
    let r = &material.rheology;
    foam_header("dictionary", "momentumTransport")
        + &format!(
            "simulationType  laminar;\n\nlaminar\n{{\n    model           generalisedNewtonian;\n\n    viscosityModel  CrossWlf;\n\n    CrossWlfCoeffs\n    {{\n        n           {:.6};\n        tauStar     {:.6e};\n        D1          {:.6e};\n        D2          {:.4};\n        D3          {:.6e};\n        A1          {:.4};\n        A2          {:.4};\n        etaMin      5;\n        etaMax      1e6;\n        gammaDotMin 1e-06;\n    }}\n}}\n",
            r.n, r.tau_star, r.d1, r.d2, r.d3, r.a1, r.a2
        )
}

/// 熔体相热物理：Tait 双域 PVT（b4 取熔体域 b4m；latentHeat 必须 0，
/// 非零会使过渡带 Cv 为负而发散——moldingFoam README §6）。
fn physical_properties_melt(material: &Material, process: &ProcessSettings) -> Result<String> {
    let pvt = &material.pvt;
    let melt_k = kelvin(process.melt_temp_c);
    let cp = table_value_at(&material.specific_heat, melt_k)
        .ok_or_else(|| KairosError::validation("材料比热表为空，无法生成熔体热物性。"))?;
    let conductivity = table_value_at(&material.conductivity, melt_k)
        .ok_or_else(|| KairosError::validation("材料导热系数表为空，无法生成熔体热物性。"))?;
    if conductivity <= 0.0 || conductivity.is_nan() {
        return Err(KairosError::validation(
            "材料导热系数必须为正，无法生成熔体热物性。",
        ));
    }
    // const 输运吃 (mu, Pr)，导热率由求解器按 k = mu·Cp/Pr 反推：Pr 必须由材料的
    // 导热真值算出来。写死常数会让 k 与真值差若干数量级（聚合物 k ~0.2 W/m·K，
    // 写死 Pr=4 时 k = 62500），熔体入模即冻、必然短射。
    let pr = MELT_TRANSPORT_MU * cp / conductivity;
    Ok(foam_header("dictionary", "physicalProperties.melt")
        + &format!(
            "thermoType\n{{\n    type            heRhoThermo;\n    mixture         pureMixture;\n    transport       const;\n    thermo          hMelt;\n    equationOfState Tait;\n    specie          specie;\n    energy          sensibleInternalEnergy;\n}}\n\nmixture\n{{\n    specie\n    {{\n        molWeight   1;\n    }}\n\n    equationOfState\n    {{\n        b1m         {:.6e};\n        b2m         {:.6e};\n        b1s         {:.6e};\n        b2s         {:.6e};\n        b3          {:.6e};\n        b4          {:.6e};\n        b5          {:.4};\n        b6          0;\n        C           0.0894;\n        smoothBand  0.5;\n    }}\n\n    thermodynamics\n    {{\n        Cp          {:.4};\n        latentHeat  0;\n        hf          0;\n    }}\n\n    transport\n    {{\n        mu          {MELT_TRANSPORT_MU:.4};\n        Pr          {pr:.6e};\n    }}\n}}\n",
            pvt.b1m, pvt.b2m, pvt.b1s, pvt.b2s, pvt.b3, pvt.b4m, pvt.b5, cp
        ))
}

/// 注入流量随浇口位置出现在 U 的 moldingInletVelocity 边界里。
fn u_dict(flow_rate: f64) -> String {
    foam_header("volVectorField", "U")
        + &format!(
            "dimensions      [0 1 -1 0 0 0 0];\n\ninternalField   uniform (0 0 0);\n\nboundaryField\n{{\n    #includeEtc \"caseDicts/setConstraintTypes\"\n\n    inlet\n    {{\n        type                moldingInletVelocity;\n        volumetricFlowRate  {flow_rate:.6e};\n        value               uniform (0 0 0);\n    }}\n\n    vent\n    {{\n        type            moldingVentVelocity;\n        value           uniform (0 0 0);\n    }}\n\n    walls\n    {{\n        type            noSlip;\n    }}\n}}\n"
        )
}

/// 模温作用于 walls 的 fixedValue；浇口 fixedValue 熔温；vent 进出流切换。
fn t_dict(melt_k: f64, mold_k: f64) -> String {
    foam_header("volScalarField", "T")
        + &format!(
            "dimensions      [0 0 0 1 0 0 0];\n\ninternalField   uniform 300;\n\nboundaryField\n{{\n    #includeEtc \"caseDicts/setConstraintTypes\"\n\n    inlet\n    {{\n        type            fixedValue;\n        value           uniform {melt_k:.2};\n    }}\n\n    vent\n    {{\n        type            inletOutlet;\n        inletValue      uniform 300;\n        value           uniform 300;\n    }}\n\n    walls\n    {{\n        type            fixedValue;\n        value           uniform {mold_k:.2};\n    }}\n}}\n"
        )
}

const ALPHA_MELT: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class \"volScalarField\";\n    object alpha.melt;\n}\ndimensions      [];\ninternalField   uniform 0;\nboundaryField\n{\n    #includeEtc \"caseDicts/setConstraintTypes\"\n\n    inlet\n    {\n        type            fixedValue;\n        value           uniform 1;\n    }\n\n    vent\n    {\n        type            inletOutlet;\n        inletValue      uniform 0;\n        value           uniform 0;\n    }\n\n    walls\n    {\n        type            zeroGradient;\n    }\n}\n";

const P_DICT: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class \"volScalarField\";\n    object p;\n}\ndimensions      [1 -1 -2 0 0 0 0];\ninternalField   uniform 1e5;\nboundaryField\n{\n    #includeEtc \"caseDicts/setConstraintTypes\"\n\n    inlet\n    {\n        type            calculated;\n        value           $internalField;\n    }\n\n    vent\n    {\n        type            calculated;\n        value           $internalField;\n    }\n\n    walls\n    {\n        type            calculated;\n        value           $internalField;\n    }\n}\n";

/// 压力场：浇口与排气口都用 moldingFoam 的双模式边界（排气口带等效流通面积
/// `CdA`，与 `ventSealAlpha` 一起构成密封逻辑）。
fn p_rgh_dict(cda_m2: f64) -> String {
    foam_header("volScalarField", "p_rgh")
        + &format!(
            "dimensions      [1 -1 -2 0 0 0 0];\ninternalField   uniform 1e5;\nboundaryField\n{{\n    #includeEtc \"caseDicts/setConstraintTypes\"\n\n    inlet\n    {{\n        type            moldingPrghPressure;\n        refValue        uniform 1e5;\n        refGradient     uniform 0;\n        valueFraction   uniform 0;\n        value           uniform 1e5;\n    }}\n\n    vent\n    {{\n        type            moldingVentPressure;\n        p0              1e5;\n        CdA             {cda_m2:.6e};\n        value           $internalField;\n    }}\n\n    walls\n    {{\n        type            fixedFluxPressure;\n        value           $internalField;\n    }}\n}}\n"
        )
}

const PHASE_PROPERTIES: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class dictionary;\n    location \"constant\";\n    object phaseProperties;\n}\nphases (melt air);\n\nsigma\n{\n    type    constant;\n    sigma   [1 0 -2 0 0 0 0] 0.025;\n}\n";

const G_DICT: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class \"uniformDimensionedVectorField\";\n    object g;\n}\ndimensions      [0 1 -2 0 0 0 0];\nvalue           (0 -9.81 0);\n";

const FV_MODELS: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class dictionary;\n    location \"constant\";\n    object fvModels;\n}\n";

const PHYSICAL_PROPERTIES_AIR: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class dictionary;\n    location \"constant\";\n    object physicalProperties.air;\n}\n\nthermoType\n{\n    type            heRhoThermo;\n    mixture         pureMixture;\n    transport       const;\n    thermo          hConst;\n    equationOfState perfectGas;\n    specie          specie;\n    energy          sensibleInternalEnergy;\n}\n\nmixture\n{\n    specie\n    {\n        molWeight   28.9;\n    }\n\n    thermodynamics\n    {\n        Cp          1007;\n        hf          0;\n    }\n\n    transport\n    {\n        mu          1.84e-05;\n        Pr          0.7;\n    }\n}\n";

const FV_SCHEMES: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class dictionary;\n    location \"system\";\n    object fvSchemes;\n}\nddtSchemes\n{\n    default         Euler;\n}\n\ngradSchemes\n{\n    default         Gauss linear;\n}\n\ndivSchemes\n{\n    div(phi,alpha)  Gauss interfaceCompression vanLeer 1;\n    div(rhoPhi,U)   Gauss linearUpwind grad(U);\n    div(alphaRhoPhi,e) Gauss upwind;\n    div(alphaRhoPhi,T) Gauss upwind;\n    div(rhoPhi,K)   Gauss upwind;\n    div(phi,p)      Gauss upwind;\n    div(((rho*nuEff)*dev2(T(grad(U))))) Gauss linear;\n}\n\nlaplacianSchemes\n{\n    default         Gauss linear uncorrected;\n}\n\ninterpolationSchemes\n{\n    default         linear;\n}\n\nsnGradSchemes\n{\n    default         uncorrected;\n}\n";

const FV_SOLUTION: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class dictionary;\n    location \"system\";\n    object fvSolution;\n}\nsolvers\n{\n    \"alpha.melt.*\"\n    {\n        nCorrectors     2;\n        nSubCycles      6;\n        MULESCorr       no;\n        solver          smoothSolver;\n        smoother        symGaussSeidel;\n        tolerance       1e-8;\n        relTol          0;\n    }\n\n    \"pcorr.*\"\n    {\n        solver          PCG;\n        preconditioner  DIC;\n        tolerance       1e-5;\n        relTol          0;\n    }\n\n    p_rgh\n    {\n        solver          PCG;\n        preconditioner  DIC;\n        tolerance       1e-07;\n        relTol          0.05;\n    }\n\n    p_rghFinal\n    {\n        $p_rgh;\n        relTol          0;\n    }\n\n    \"(U|e|T).*\"\n    {\n        solver          smoothSolver;\n        smoother        symGaussSeidel;\n        tolerance       1e-06;\n        relTol          0;\n    }\n}\n\nPIMPLE\n{\n    momentumPredictor no;\n    nOuterCorrectors 1;\n    nCorrectors     3;\n    nNonOrthogonalCorrectors 0;\n}\n";

/// 各分析阶段预期产出的结果场目录（供结果模型与后处理面板消费）。
/// 场名为 moldingFoam 的真实写出名；填充 ⊂ 保压 ⊂ 冷却单调增长。
const FILL_FIELDS: &[&str] = &["alpha.melt", "p", "p_rgh", "T", "U"];
// 保压与冷却阶段场集合一致（顶出判据由求解器内部判定，不新增场）。
const FILL_PACK_FIELDS: &[&str] = &["alpha.melt", "p", "p_rgh", "T", "U", "rho"];

/// 各分析阶段预期产出的结果场名。
pub fn expected_fields(stage: &AnalysisStage) -> &'static [&'static str] {
    match stage {
        AnalysisStage::Fill => FILL_FIELDS,
        AnalysisStage::FillPack | AnalysisStage::FillPackCool => FILL_PACK_FIELDS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::process::ProcessSettings;
    use crate::models::runners::{RunnerElement, RunnerKind};

    #[test]
    fn boundary_bands_classify_bottom_inlet_top_vent() {
        // 分带方向锁定：底带 = 浇口 inlet、顶带 = 排气 vent、中段 = 模壁
        // （消融 B5 锁定——分带翻转或交换时该用例失败）。
        let (z_min, z_max) = (0.0_f64, 10.0_f64);
        assert!(matches!(
            classify_boundary_face(0.0, -1.0, z_min, z_max),
            BoundaryBand::Inlet
        ));
        assert!(matches!(
            classify_boundary_face(10.0, 1.0, z_min, z_max),
            BoundaryBand::Vent
        ));
        assert!(matches!(
            classify_boundary_face(5.0, 0.0, z_min, z_max),
            BoundaryBand::Walls
        ));
    }

    #[test]
    fn generate_case_reports_constant_and_stage_dir_collisions() {
        let material = &crate::services::material::builtin_materials()[0];
        // 预埋 case 目录本身为文件：polyMesh 目录创建失败（首个写点）。
        let as_file = std::env::temp_dir().join(format!("kairos-t36c-{}", std::process::id()));
        fs::write(&as_file, "占位").unwrap();
        let error = generate_case(
            &as_file,
            &two_tet_mesh(),
            material,
            &process(),
            &AnalysisStage::Fill,
            4,
            &[],
        )
        .unwrap_err();
        assert!(error.to_string().contains("创建 polyMesh 目录失败"));
        fs::remove_file(&as_file).ok();

        // 预埋 0/ 为文件：write_case_files 的目录准备失败。
        let staged = std::env::temp_dir().join(format!("kairos-t36d-{}", std::process::id()));
        let case = staged.join("case");
        fs::create_dir_all(case.join("constant")).unwrap();
        fs::write(case.join("0"), "占位").unwrap();
        let error = generate_case(
            &case,
            &two_tet_mesh(),
            material,
            &process(),
            &AnalysisStage::Fill,
            4,
            &[],
        )
        .unwrap_err();
        assert!(error.to_string().contains("创建"), "{error}");
        fs::remove_dir_all(&staged).ok();
    }

    #[test]
    fn write_reports_parent_dir_creation_failure() {
        let blocked = std::env::temp_dir().join(format!("kairos-t36e-{}", std::process::id()));
        fs::create_dir_all(&blocked).unwrap();
        fs::write(blocked.join("system"), "占位").unwrap();
        let error = write(&blocked.join("system").join("controlDict"), "x").unwrap_err();
        assert!(error.to_string().contains("创建目录失败"));
        fs::remove_dir_all(&blocked).ok();

        // 根路径没有父目录：走 Path::new(".") 兜底后，写入根目录仍失败。
        let error = write(std::path::Path::new("/"), "x").unwrap_err();
        assert!(error.to_string().contains("写入"));
    }

    /// 内置 PP：比热 / 导热 / PVT 齐全，用于生成器用例。
    fn sample_material() -> Material {
        crate::services::material::builtin_materials()[0].clone()
    }

    #[test]
    fn physical_properties_melt_requires_specific_heat_table() {
        let mut material = sample_material();
        material.specific_heat = Vec::new();
        let error = physical_properties_melt(&material, &process()).unwrap_err();
        assert!(error.to_string().contains("材料比热表为空"));
    }

    #[test]
    fn physical_properties_melt_derives_pr_from_conductivity() {
        let mut material = sample_material();
        material.specific_heat = vec![(500.0, 2500.0)];
        material.conductivity = vec![(500.0, 0.22)];
        let melt = physical_properties_melt(&material, &process()).unwrap();
        // Pr = mu·Cp/k = 100·2500/0.22 ≈ 1.136e6 → 求解器反推的 k 才是材料真值
        assert!(melt.contains("Pr          1.136364e6"), "{melt}");
        assert!(melt.contains("mu          100.0000"));
        assert!(melt.contains("Cp          2500.0000"));
    }

    #[test]
    fn physical_properties_melt_requires_positive_conductivity() {
        let mut material = sample_material();
        material.conductivity = Vec::new();
        let error = physical_properties_melt(&material, &process()).unwrap_err();
        assert!(error.to_string().contains("材料导热系数表为空"));

        material.conductivity = vec![(500.0, 0.0)];
        let error = physical_properties_melt(&material, &process()).unwrap_err();
        assert!(error.to_string().contains("材料导热系数必须为正"));
    }

    #[test]
    fn packing_ramp_scales_with_injection_time_and_clamps() {
        // 注射 1 s → 5% = 0.05 命中下限；4 s → 0.2；20 s → 夹到上限 0.5
        assert_eq!(packing_ramp_s(1.0), PACKING_RAMP_MIN_S);
        assert!((packing_ramp_s(4.0) - 0.2).abs() < 1e-12);
        assert_eq!(packing_ramp_s(20.0), PACKING_RAMP_MAX_S);
    }

    fn two_tet_mesh() -> VolumeMesh {
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
    fn case_generation_writes_contract_directory() {
        let dir = std::env::temp_dir().join(format!("kairos-t36-{}", std::process::id()));
        let case = dir.join("case");
        generate_case(
            &case,
            &two_tet_mesh(),
            &crate::services::material::builtin_materials()[0],
            &process(),
            &AnalysisStage::Fill,
            4,
            &[],
        )
        .unwrap();

        for path in [
            "constant/polyMesh/boundary",
            "0/alpha.melt",
            "0/U",
            "0/p",
            "0/p_rgh",
            "0/T",
            "system/controlDict",
            "system/decomposeParDict",
            "system/fvSchemes",
            "system/fvSolution",
            "constant/moldingDict",
            "constant/momentumTransport",
            "constant/phaseProperties",
            "constant/g",
            "constant/fvModels",
            "constant/physicalProperties.melt",
            "constant/physicalProperties.air",
        ] {
            assert!(case.join(path).exists(), "缺少 {path}");
        }

        // 边界三 patch（小网格分带下 inlet/vent 可为空面数）
        let boundary = fs::read_to_string(case.join("constant/polyMesh/boundary")).unwrap();
        for patch in ["inlet", "vent", "walls"] {
            assert!(boundary.contains(patch), "boundary 缺少 {patch}");
        }
        // 求解入口契约：foamRun + moldingFoam + libs
        let control = fs::read_to_string(case.join("system/controlDict")).unwrap();
        assert!(control.contains("application     foamRun;"));
        assert!(control.contains("solver          moldingFoam;"));
        assert!(control.contains("libs            (\"libmoldingFoam.so\");"));
        // U 的浇口为 moldingInletVelocity（体积流量 = 型腔体积/注射时间）
        let u = fs::read_to_string(case.join("0/U")).unwrap();
        assert!(u.contains("moldingInletVelocity"));
        let volume = mesh_volume(&two_tet_mesh());
        assert!(u.contains(&format!("{:.6e}", volume / 1.0)));
        // T：熔温/模温开尔文化
        let t_field = fs::read_to_string(case.join("0/T")).unwrap();
        assert!(t_field.contains("uniform 503.15")); // 230C
        assert!(t_field.contains("uniform 313.15")); // 40C

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn molding_dict_reflects_process_settings() {
        let material = sample_material();
        let dict = molding_dict(&material, &process());
        assert!(dict.contains("switchFraction   0.96"));
        // 切换压力 = 曲线起点，避免切换瞬间压力阶跃（MPa → Pa）
        assert!(dict.contains("switchPressure  6.000000e7;"));
        assert!(dict.contains("(0.0000 6.000000e7)"));
        assert!(dict.contains("(8.0000 4.000000e7)"));
        assert!(dict.contains("ejectionTemperature  363.15"));
        // 冻死短射守卫：无流温度取 Tait 转变温度 b5（内置 PP 为 418 K）
        assert!(dict.contains("freezeOffTemperature  418.0000"), "{dict}");
        assert!(dict.contains("freezeOffFraction  0.0100"));
        // 保压斜坡：注射 1 s → 5% = 0.05（下限），避免切换瞬间单步阶跃
        assert!(dict.contains("pressureRamp  0.0500"), "{dict}");
        let momentum = momentum_transport_dict(&material);
        assert!(momentum.contains("viscosityModel  CrossWlf;"));
        let melt = physical_properties_melt(&material, &process()).unwrap();
        assert!(melt.contains("equationOfState Tait;"));
        assert!(melt.contains("latentHeat  0;"));
    }

    #[test]
    fn molding_dict_falls_back_on_empty_curve() {
        let mut settings = process();
        settings.packing_pressure_mpa_curve = vec![];
        let dict = molding_dict(&sample_material(), &settings);
        // 空曲线：单点大气压表且不写切换压力（求解器 Table 仍需至少一点）
        assert!(dict.contains("(0 1e5)"));
        assert!(!dict.contains("switchPressure"));
    }

    #[test]
    fn solve_command_decomposes_then_runs_under_mpirun() {
        let command = solve_command(6);
        assert_eq!(
            command,
            "decomposePar -force && mpirun -np 6 foamRun -parallel; reconstructPar"
        );
    }

    #[test]
    fn abort_line_matches_solver_error_markers_only() {
        assert!(is_abort_line("--> FOAM FATAL ERROR: "));
        assert!(is_abort_line("FOAM exiting"));
        assert!(is_abort_line("[1] FOAM aborting"));
        // 正常日志行与正常收尾标记都不是异常标记
        assert!(!is_abort_line("Time = 0.5s"));
        assert!(!is_abort_line("End"));
        assert!(!is_abort_line("ExecutionTime = 4 s"));
    }

    /// 写出的 polyMesh 四个列表：点、三角面、owner、neighbour。
    type RawPolyMesh = (Vec<[f64; 3]>, Vec<[usize; 3]>, Vec<usize>, Vec<usize>);

    /// 解析写出的 polyMesh（测试用）。
    fn parse_poly_mesh(poly: &Path) -> RawPolyMesh {
        let text = |name: &str| fs::read_to_string(poly.join(name)).unwrap();
        let numbers = |line: &str| -> Vec<f64> {
            line.split(|c: char| {
                !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E')
            })
            .filter(|token| !token.is_empty())
            .filter_map(|token| token.parse::<f64>().ok())
            .collect()
        };
        let points = text("points")
            .lines()
            .filter(|line| line.starts_with('('))
            .map(numbers)
            .filter(|values| values.len() >= 3)
            .map(|values| [values[0], values[1], values[2]])
            .collect();
        let faces = text("faces")
            .lines()
            .filter(|line| line.starts_with("3("))
            .map(|line| {
                // 首个 token 是面点数（3），节点编号从第二个 token 起
                let values = numbers(line);
                [values[1] as usize, values[2] as usize, values[3] as usize]
            })
            .collect();
        let labels = |name: &str| -> Vec<usize> {
            text(name)
                .lines()
                // 最后一项与列表收尾的 ")" 同行
                .filter_map(|line| line.trim().trim_end_matches(')').parse::<usize>().ok())
                // 首个可解析行是条目数（列表头），不是 label
                .skip(1)
                .collect()
        };
        (points, faces, labels("owner"), labels("neighbour"))
    }

    /// 闭合三角面集合的带符号体积（发散定理，用于校验单元体积为正）。
    fn closed_volume(points: &[[f64; 3]], faces: &[[usize; 3]]) -> f64 {
        faces
            .iter()
            .map(|face| {
                let a = points[face[0]];
                let b = points[face[1]];
                let c = points[face[2]];
                let cross = [
                    b[1] * c[2] - b[2] * c[1],
                    b[2] * c[0] - b[0] * c[2],
                    b[0] * c[1] - b[1] * c[0],
                ];
                a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]
            })
            .sum::<f64>()
            / 6.0
    }

    /// 写出的 polyMesh 必须自洽：faces/owner/neighbour 逐项对齐、内部面在前、
    /// 每个单元 4 个面且体积为正。此前 faces/owner 被单独重排而 neighbour 未
    /// 同步，导致 owner 与面错位、1968/5000 个单元体积为负（求解第一步即 NaN）。
    #[test]
    fn written_poly_mesh_is_consistent() {
        let dir = std::env::temp_dir().join(format!("kairos-mesh-{}", std::process::id()));
        let case = dir.join("case");
        let mesh = crate::services::meshing::generate(
            &crate::models::geometry::TriangleMesh::sample_box(2.0),
            &crate::services::meshing::VolumeMeshParams {
                refinement: None,
                target_size: 1.0,
            },
        )
        .unwrap();
        write_poly_mesh(&case, &mesh, &[]).unwrap();
        let (points, faces, owner, neighbour) = parse_poly_mesh(&case.join("constant/polyMesh"));

        assert_eq!(faces.len(), owner.len());
        assert!(neighbour.len() < faces.len());
        // 内部面在前：boundary 的 startFace 依赖该区段划分
        assert_eq!(neighbour.len() + boundary_face_count(&case), faces.len());

        // (面序号, 该单元是否为 owner)：neighbour 一侧的面朝向相反
        let mut cell_faces: Vec<Vec<(usize, bool)>> = vec![Vec::new(); mesh.tets.len()];
        for (index, cell) in owner.iter().enumerate() {
            cell_faces[*cell].push((index, true));
        }
        for (index, cell) in neighbour.iter().enumerate() {
            cell_faces[*cell].push((index, false));
        }
        for (cell, face_ids) in cell_faces.iter().enumerate() {
            assert_eq!(face_ids.len(), 4, "单元 {cell} 的面数不为 4");
            let tris: Vec<[usize; 3]> = face_ids
                .iter()
                .map(|&(face, is_owner)| {
                    let mut tri = faces[face];
                    if !is_owner {
                        tri.swap(1, 2);
                    }
                    tri
                })
                .collect();
            assert!(
                closed_volume(&points, &tris) > 0.0,
                "单元 {cell} 体积非正：faces={face_ids:?}"
            );
        }

        fs::remove_dir_all(&dir).ok();
    }

    /// 解析 boundary 文件的 patch 区段（名字 → startFace/nFaces）。
    /// 写出的格式里 patch 名与 `{` 分行，故用「上一个非空行」作为候选名。
    fn parse_patches(case: &Path) -> std::collections::HashMap<String, PatchSpan> {
        let text = fs::read_to_string(case.join("constant/polyMesh/boundary")).unwrap();
        let mut patches: std::collections::HashMap<String, PatchSpan> =
            std::collections::HashMap::new();
        let mut name: Option<String> = None;
        let mut previous = String::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed == "{" {
                if !previous.is_empty()
                    && previous != "FoamFile"
                    && !previous.contains([';', ':', '{', '}'])
                {
                    name = Some(previous.clone());
                }
            } else if let Some(value) = trimmed.strip_prefix("nFaces ") {
                if let (Some(current), Ok(count)) = (
                    name.clone(),
                    value.trim_end_matches(';').trim().parse::<usize>(),
                ) {
                    patches.insert(current, PatchSpan { start: 0, count });
                }
            } else if let Some(value) = trimmed.strip_prefix("startFace ")
                && let (Some(current), Ok(start)) = (
                    name.clone(),
                    value.trim_end_matches(';').trim().parse::<usize>(),
                )
                && let Some(span) = patches.get_mut(&current)
            {
                span.start = start;
            }
            if !trimmed.is_empty() {
                previous = trimmed.to_string();
            }
        }
        patches
    }

    /// patch 在 faces 列表里的区段。
    #[derive(Debug, Clone, Copy)]
    struct PatchSpan {
        start: usize,
        count: usize,
    }

    /// boundary 文件中所有 patch 的 nFaces 之和。
    fn boundary_face_count(case: &Path) -> usize {
        fs::read_to_string(case.join("constant/polyMesh/boundary"))
            .unwrap()
            .lines()
            .filter_map(|line| line.trim().strip_prefix("nFaces "))
            .filter_map(|value| value.trim_end_matches(';').parse::<usize>().ok())
            .sum()
    }

    #[test]
    fn degenerate_face_normal_stays_zero_and_classifies_as_walls() {
        // 共线三点构成零面积面：法向长度为 0 时 z 分量取 0，不参与朝下/朝上判定
        // （防御分支：真实网格里不出现，但写面时不应对零向量做除法）。
        let dir = std::env::temp_dir().join(format!("kairos-deg-{}", std::process::id()));
        let case = dir.join("case");
        let mesh = VolumeMesh {
            nodes: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            tets: vec![[0, 1, 2, 3]],
            surface_faces: Vec::new(),
        };
        write_poly_mesh(&case, &mesh, &[]).unwrap();
        let patches = parse_patches(&case);
        // 三个面落在 z=0 平面上（朝下 → inlet），零面积面法向 z 记 0 → walls
        assert_eq!(patches.get("inlet").map(|span| span.count), Some(3));
        assert_eq!(patches.get("walls").map(|span| span.count), Some(1));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn gate_portals_take_cavity_end_of_gate_units_only() {
        let runners = vec![
            RunnerElement {
                id: "r-1".into(),
                kind: RunnerKind::Runner,
                diameter_mm: 6.0,
                start: [0.0, 0.0, 5.0],
                end: [0.0, 0.0, 0.0],
            },
            RunnerElement {
                id: "g-1".into(),
                kind: RunnerKind::Gate,
                diameter_mm: 1.5,
                start: [0.0, 0.0, 0.0],
                end: [10.0, 20.0, 0.0],
            },
            RunnerElement {
                id: "g-2".into(),
                kind: RunnerKind::Gate,
                diameter_mm: 0.2,
                start: [0.0, 0.0, 0.0],
                end: [1.0, 2.0, 3.0],
            },
        ];
        let portals = gate_portals(&runners);
        // 只取 Gate 单元，入口在型腔侧的一端（end），半径 = 直径/2
        assert_eq!(portals.len(), 2);
        assert_eq!(portals[0].center, [10.0, 20.0, 0.0]);
        assert!((portals[0].radius_mm - 0.75).abs() < 1e-12);
        // 直径细小 → 半径下限保底（靠最近面补齐，避免入口为空）
        assert!((portals[1].radius_mm - GATE_MIN_RADIUS_MM).abs() < 1e-12);
    }

    #[test]
    fn boundary_face_classification_needs_outward_normal() {
        // 底带 + 朝下 = inlet；同带的阶梯侧向面归 walls（否则熔体从外壁注入）
        assert_eq!(
            classify_boundary_face(0.0, -1.0, 0.0, 10.0),
            BoundaryBand::Inlet
        );
        assert_eq!(
            classify_boundary_face(0.4, 0.0, 0.0, 10.0),
            BoundaryBand::Walls
        );
        // 顶带 + 朝上 = vent；同带的侧向面归 walls
        assert_eq!(
            classify_boundary_face(10.0, 1.0, 0.0, 10.0),
            BoundaryBand::Vent
        );
        assert_eq!(
            classify_boundary_face(9.7, 0.2, 0.0, 10.0),
            BoundaryBand::Walls
        );
        // 中段无论朝向都归 walls
        assert_eq!(
            classify_boundary_face(5.0, -1.0, 0.0, 10.0),
            BoundaryBand::Walls
        );
    }

    #[test]
    fn gate_inlet_faces_fill_up_to_equivalent_area() {
        // 面心沿 x 等距排列、面积各 0.4；半径 0.5 只圈到第一个面，
        // 等效面积 πr² ≈ 0.785 未达标 → 按距离补足一个面后停手。
        let centres: Vec<[f64; 3]> = (0..5).map(|i| [i as f64, 0.0, 0.0]).collect();
        let areas = vec![0.4; 5];
        let boundary: Vec<usize> = (0..5).collect();
        let gates = vec![GatePortal {
            center: [0.0, 0.0, 0.0],
            radius_mm: 0.5,
        }];
        assert_eq!(
            gate_inlet_faces(&boundary, &centres, &areas, &gates),
            vec![0, 1]
        );
    }

    #[test]
    fn write_poly_mesh_puts_gate_faces_on_inlet_patch() {
        let dir = std::env::temp_dir().join(format!("kairos-gate-{}", std::process::id()));
        let case = dir.join("case");
        let mesh = crate::services::meshing::generate(
            &crate::models::geometry::TriangleMesh::sample_box(4.0),
            &crate::services::meshing::VolumeMeshParams {
                refinement: None,
                target_size: 1.0,
            },
        )
        .unwrap();

        // 无浇口：回退分带，inlet = 底面朝下的那圈面
        write_poly_mesh(&case, &mesh, &[]).unwrap();
        let (_, _, _, _) = parse_poly_mesh(&case.join("constant/polyMesh"));
        let band_inlet = parse_patches(&case).remove("inlet").unwrap();

        // 有浇口：inlet 收缩到浇口邻域（面数变少、且都落在浇口附近）
        let gate = GatePortal {
            center: [2.0, 2.0, 0.0],
            radius_mm: 1.2,
        };
        write_poly_mesh(&case, &mesh, &[gate]).unwrap();
        let (points, faces, _, _) = parse_poly_mesh(&case.join("constant/polyMesh"));
        let gated_inlet = parse_patches(&case).remove("inlet").unwrap();
        assert!(gated_inlet.count > 0, "浇口圈定的入口不能为空");
        assert!(
            gated_inlet.count < band_inlet.count,
            "浇口入口面数应少于分带回退（{} vs {}）",
            gated_inlet.count,
            band_inlet.count
        );
        let max_distance = (gated_inlet.start..gated_inlet.start + gated_inlet.count)
            .map(|index| {
                let face = faces[index];
                let centroid = [
                    (points[face[0]][0] + points[face[1]][0] + points[face[2]][0]) / 3.0,
                    (points[face[0]][1] + points[face[1]][1] + points[face[2]][1]) / 3.0,
                    (points[face[0]][2] + points[face[1]][2] + points[face[2]][2]) / 3.0,
                ];
                // 写出的点是米：把浇口坐标一并换算后再比距离
                distance(
                    centroid,
                    [
                        gate.center[0] * MM_TO_M,
                        gate.center[1] * MM_TO_M,
                        gate.center[2] * MM_TO_M,
                    ],
                )
            })
            .fold(0.0_f64, f64::max);
        // 半径 1.2 mm + 单元对角线量级（补齐最近面）以内 → 米制 □ 2.5e-3
        assert!(max_distance < 2.5e-3, "入口面离浇口过远：{max_distance}");
        // 等效流通面积不低于 πr²（消融「按距离补足」时该断言先红）
        let inlet_area: f64 = (gated_inlet.start..gated_inlet.start + gated_inlet.count)
            .map(|index| {
                let face = faces[index];
                let (a, b, c) = (points[face[0]], points[face[1]], points[face[2]]);
                let (u, v) = (
                    [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                    [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
                );
                let cross = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt() / 2.0
            })
            .sum();
        let gate_area_m2 =
            std::f64::consts::PI * gate.radius_mm * gate.radius_mm * MM_TO_M * MM_TO_M;
        assert!(
            inlet_area >= gate_area_m2,
            "入口等效面积 {inlet_area} 小于 πr²（{gate_area_m2}）"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn written_mesh_is_si_with_patch_areas() {
        // 4 mm 立方体、1 mm 体素：写出的点应在米制（≤ 4e-3），
        // patch 面积为 SI（底面 16 个 1×1 mm 面元 = 16 mm² = 1.6e-5 m²）。
        let dir = std::env::temp_dir().join(format!("kairos-si-{}", std::process::id()));
        let case = dir.join("case");
        let mesh = crate::services::meshing::generate(
            &crate::models::geometry::TriangleMesh::sample_box(4.0),
            &crate::services::meshing::VolumeMeshParams {
                refinement: None,
                target_size: 1.0,
            },
        )
        .unwrap();
        let areas = write_poly_mesh(&case, &mesh, &[]).unwrap();

        let (points, _, _, _) = parse_poly_mesh(&case.join("constant/polyMesh"));
        let max_coordinate = points.iter().flatten().fold(f64::MIN, |acc, v| acc.max(*v));
        assert!(max_coordinate <= 4.0e-3, "坐标应为米：{max_coordinate}");

        let expected = 1.6e-5;
        assert!(
            (areas.inlet_m2 - expected).abs() < expected * 0.01,
            "inlet 面积 {:#e} 应为 {expected:#e}",
            areas.inlet_m2
        );
        assert!((areas.vent_m2 - expected).abs() < expected * 0.01);
        assert!(areas.walls_m2 > 0.0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn mesh_volume_sums_tet_volumes() {
        // 两个单位直角四面体各 1/6，合计 1/3
        assert!((mesh_volume(&two_tet_mesh()) - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn expected_fields_grow_and_use_real_names() {
        use crate::models::solver::AnalysisStage;
        let fill = expected_fields(&AnalysisStage::Fill);
        let cool = expected_fields(&AnalysisStage::FillPackCool);
        assert!(fill.contains(&"alpha.melt"));
        assert!(fill.iter().all(|f| cool.contains(f)));
    }
    #[test]
    fn table_value_at_interpolates_and_clamps_endpoints() {
        let table = [(300.0, 2.0), (400.0, 4.0)];
        assert_eq!(table_value_at(&table, 250.0), Some(2.0));
        assert_eq!(table_value_at(&table, 350.0), Some(3.0));
        assert_eq!(table_value_at(&table, 999.0), Some(4.0));
        assert_eq!(table_value_at(&[], 300.0), None);
    }

    #[test]
    fn parse_time_line_reads_solver_progress() {
        assert_eq!(parse_time_line("Time = 0.05"), Some(0.05));
        assert_eq!(parse_time_line("  Time = 1.25 s"), Some(1.25));
        assert_eq!(parse_time_line("Flow time = 0.5"), None);
        assert_eq!(parse_time_line("Time = abc"), None);
        assert_eq!(parse_time_line(""), None);
    }

    #[test]
    fn case_generation_covers_pack_cool_dicts() {
        let dir = std::env::temp_dir().join(format!("kairos-packcool-{}", std::process::id()));
        let case = dir.join("case");
        generate_case(
            &case,
            &two_tet_mesh(),
            &crate::services::material::builtin_materials()[0],
            &process(),
            &crate::models::solver::AnalysisStage::FillPackCool,
            4,
            &[],
        )
        .unwrap();

        for path in [
            "constant/momentumTransport",
            "constant/phaseProperties",
            "constant/g",
            "constant/fvModels",
            "constant/physicalProperties.melt",
            "constant/physicalProperties.air",
        ] {
            assert!(case.join(path).exists(), "缺少 {path}");
        }
        fs::remove_dir_all(&dir).ok();
    }
}
