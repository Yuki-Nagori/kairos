//! OpenFOAM case 生成与进度解析（纯函数，无进程操作）。
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
use crate::models::solver::AnalysisStage;

/// 求解入口单点：foamRun 框架的求解模块名。bundle 自带
/// libmoldingFoamSolver.so 探测链接，libs 行同时显式加载（双保险）。
pub const SOLVER_MODULE: &str = "moldingFoam";

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

/// 网格包围盒 z 高度带分类（v1 轴向模具启发式）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum BoundaryBand {
    Inlet,
    Vent,
    Walls,
}

fn classify_face(z_centroid: f64, z_min: f64, z_max: f64) -> BoundaryBand {
    let height = (z_max - z_min).max(1e-9);
    if z_centroid <= z_min + INLET_BAND * height {
        BoundaryBand::Inlet
    } else if z_centroid >= z_max - VENT_BAND * height {
        BoundaryBand::Vent
    } else {
        BoundaryBand::Walls
    }
}

/// 从体积网格写出 constant/polyMesh（points/faces/owner/neighbour/boundary）。
/// 四面体绕向已在生成时保证正体积；面法向按 owner 外法向定向。
/// 边界面按 z 分带重排为 inlet / vent / walls 三个连续 patch 区段。
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

    // 面法向按 owner 外法向定向（点积判定，反向则交换后两点）。
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
    let n_internal = neighbour_lines.len();

    // 边界面按 z 分带分类，并重排为「内部面 → inlet → vent → walls」连续区段
    // （boundary 文件的 startFace/nFaces 要求每个 patch 的面连续）。
    let (z_min, z_max) = mesh.nodes.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p[2]), hi.max(p[2]))
    });
    let mut band_faces: [Vec<usize>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for (index, face) in faces.iter().enumerate().skip(n_internal) {
        let z_centroid =
            (mesh.nodes[face[0]][2] + mesh.nodes[face[1]][2] + mesh.nodes[face[2]][2]) / 3.0;
        let slot = match classify_face(z_centroid, z_min, z_max) {
            BoundaryBand::Inlet => 0,
            BoundaryBand::Vent => 1,
            BoundaryBand::Walls => 2,
        };
        band_faces[slot].push(index);
    }

    let mut ordered_face_lines = face_lines[..n_internal].to_vec();
    let mut ordered_owner_lines = owner_lines[..n_internal].to_vec();
    let mut boundary_patches: Vec<(&str, usize, usize)> = Vec::new();
    let mut start_face = n_internal;
    for (slot, patch_name) in ["inlet", "vent", "walls"].iter().enumerate() {
        let count = band_faces[slot].len();
        for &index in &band_faces[slot] {
            ordered_face_lines.push(face_lines[index].clone());
            ordered_owner_lines.push(owner_lines[index].clone());
        }
        boundary_patches.push((patch_name, start_face, count));
        start_face += count;
    }

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
        faces.len(),
        ordered_face_lines.join("\n")
    );
    let owner_content = format!(
        "FoamFile\n{{\n    version 2.0;\n    format ascii;\n    class \"labelList\";\n    object owner;\n}}\n{}\n(\n{})\n",
        faces.len(),
        ordered_owner_lines.join("\n")
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
    for (patch_name, patch_start, patch_count) in &boundary_patches {
        boundary_content.push_str(&format!(
            "    {patch_name}\n    {{\n        type wall;\n        inGroups 1(wall);\n        nFaces {patch_count};\n        startFace {patch_start};\n    }}\n"
        ));
    }
    boundary_content.push_str(")\n");

    write(&poly.join("points"), &points_content)?;
    write(&poly.join("faces"), &faces_content)?;
    write(&poly.join("owner"), &owner_content)?;
    write(&poly.join("neighbour"), &neighbour_content)?;
    write(&poly.join("boundary"), &boundary_content)?;
    Ok(())
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

    write(&constant.join("moldingDict"), &molding_dict(process))?;
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
    // 注射流量 = 型腔体积 / 注射时间（moldingInletVelocity 的体积流量口径）。
    let flow_rate = mesh_volume(mesh) / process.injection_time_s.max(1e-9);
    write(&zero.join("alpha.melt"), ALPHA_MELT)?;
    write(&zero.join("U"), &u_dict(flow_rate))?;
    write(&zero.join("p"), P_DICT)?;
    write(&zero.join("p_rgh"), P_RGH_DICT)?;
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
) -> Result<()> {
    write_poly_mesh(case_dir, mesh)?;
    write_case_files(case_dir, mesh, material, process, stage, cores)
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

/// 工艺字典：V/P 切换分数、保压压力表（首点大气压防阶跃）、顶出判据。
fn molding_dict(process: &ProcessSettings) -> String {
    let mut curve = String::new();
    curve.push_str("            (0 1e5)\n");
    for (time_s, pressure_mpa) in &process.packing_pressure_mpa_curve {
        curve.push_str(&format!(
            "            ({time_s:.4} {:.6e})\n",
            pressure_mpa * 1e6
        ));
    }
    foam_header("dictionary", "moldingDict")
        + &format!(
            "injection\n{{\n    meltTemperature  {:.4};\n}}\npacking\n{{\n    switchFraction   {:.4};\n    pressure\n    {{\n        type            table;\n        values\n        (\n{}        );\n    }}\n}}\ncooling\n{{\n    ejectionTemperature  {:.4};\n    releasePressure  1e5;\n}}\n",
            kelvin(process.melt_temp_c),
            process.vp_switch_volume_percent / 100.0,
            curve,
            kelvin(process.ejection_temp_c)
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
    Ok(foam_header("dictionary", "physicalProperties.melt")
        + &format!(
            "thermoType\n{{\n    type            heRhoThermo;\n    mixture         pureMixture;\n    transport       const;\n    thermo          hMelt;\n    equationOfState Tait;\n    specie          specie;\n    energy          sensibleInternalEnergy;\n}}\n\nmixture\n{{\n    specie\n    {{\n        molWeight   1;\n    }}\n\n    equationOfState\n    {{\n        b1m         {:.6e};\n        b2m         {:.6e};\n        b1s         {:.6e};\n        b2s         {:.6e};\n        b3          {:.6e};\n        b4          {:.6e};\n        b5          {:.4};\n        b6          0;\n        C           0.0894;\n        smoothBand  0.5;\n    }}\n\n    thermodynamics\n    {{\n        Cp          {:.4};\n        latentHeat  0;\n        hf          0;\n    }}\n\n    transport\n    {{\n        mu          100;\n        Pr          4;\n    }}\n}}\n",
            pvt.b1m, pvt.b2m, pvt.b1s, pvt.b2s, pvt.b3, pvt.b4m, pvt.b5, cp
        ))
}

/// 注入流量随浇口位置出现在 U 的 moldingInletVelocity 边界里。
fn u_dict(flow_rate: f64) -> String {
    foam_header("volVectorField", "U")
        + &format!(
            "dimensions      [0 1 -1 0 0 0 0];\n\ninternalField   uniform (0 0 0);\n\nboundaryField\n{{\n    #includeEtc \"caseDicts/setConstraintTypes\"\n\n    inlet\n    {{\n        type                moldingInletVelocity;\n        volumetricFlowRate  {flow_rate:.6e};\n        value               uniform (0 0 0);\n    }}\n\n    vent\n    {{\n        type            pressureInletOutletVelocity;\n        value           uniform (0 0 0);\n    }}\n\n    walls\n    {{\n        type            noSlip;\n    }}\n}}\n"
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

const P_RGH_DICT: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class \"volScalarField\";\n    object p_rgh;\n}\ndimensions      [1 -1 -2 0 0 0 0];\ninternalField   uniform 1e5;\nboundaryField\n{\n    #includeEtc \"caseDicts/setConstraintTypes\"\n\n    inlet\n    {\n        type            moldingPrghPressure;\n        refValue        uniform 1e5;\n        refGradient     uniform 0;\n        valueFraction   uniform 0;\n        value           uniform 1e5;\n    }\n\n    vent\n    {\n        type            prghTotalPressure;\n        p0              uniform 1e5;\n        value           $internalField;\n    }\n\n    walls\n    {\n        type            fixedFluxPressure;\n        value           $internalField;\n    }\n}\n";

const PHASE_PROPERTIES: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class dictionary;\n    location \"constant\";\n    object phaseProperties;\n}\nphases (melt air);\n\nsigma\n{\n    type    constant;\n    sigma   [1 0 -2 0 0 0 0] 0.025;\n}\n";

const G_DICT: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class \"uniformDimensionedVectorField\";\n    object g;\n}\ndimensions      [0 1 -2 0 0 0 0];\nvalue           (0 -9.81 0);\n";

const FV_MODELS: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class dictionary;\n    location \"constant\";\n    object fvModels;\n}\n";

const PHYSICAL_PROPERTIES_AIR: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class dictionary;\n    location \"constant\";\n    object physicalProperties.air;\n}\n\nthermoType\n{\n    type            heRhoThermo;\n    mixture         pureMixture;\n    transport       const;\n    thermo          hConst;\n    equationOfState perfectGas;\n    specie          specie;\n    energy          sensibleInternalEnergy;\n}\n\nmixture\n{\n    specie\n    {\n        molWeight   28.9;\n    }\n\n    thermodynamics\n    {\n        Cp          1007;\n        hf          0;\n    }\n\n    transport\n    {\n        mu          1.84e-05;\n        Pr          0.7;\n    }\n}\n";

const FV_SCHEMES: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class dictionary;\n    location \"system\";\n    object fvSchemes;\n}\nddtSchemes\n{\n    default         Euler;\n}\n\ngradSchemes\n{\n    default         Gauss linear;\n}\n\ndivSchemes\n{\n    div(phi,alpha)  Gauss interfaceCompression vanLeer 1;\n    div(rhoPhi,U)   Gauss linearUpwind grad(U);\n    div(alphaRhoPhi,e) Gauss upwind;\n    div(alphaRhoPhi,T) Gauss upwind;\n    div(rhoPhi,K)   Gauss upwind;\n    div(phi,p)      Gauss upwind;\n    div(((rho*nuEff)*dev2(T(grad(U))))) Gauss linear;\n}\n\nlaplacianSchemes\n{\n    default         Gauss linear uncorrected;\n}\n\ninterpolationSchemes\n{\n    default         linear;\n}\n\nsnGradSchemes\n{\n    default         uncorrected;\n}\n";

const FV_SOLUTION: &str = "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class dictionary;\n    location \"system\";\n    object fvSolution;\n}\nsolvers\n{\n    \"alpha.melt.*\"\n    {\n        nCorrectors     2;\n        nSubCycles      6;\n        MULESCorr       no;\n        solver          smoothSolver;\n        smoother        symGaussSeidel;\n        tolerance       1e-8;\n        relTol          0;\n    }\n\n    \"pcorr.*\"\n    {\n        solver          PCG;\n        preconditioner  DIC;\n        tolerance       1e-5;\n        relTol          0;\n    }\n\n    p_rgh\n    {\n        solver          PCG;\n        preconditioner  DIC;\n        tolerance       1e-07;\n        relTol          0.05;\n    }\n\n    p_rghFinal\n    {\n        $p_rgh;\n        relTol          0;\n    }\n\n    \"(U|e|T).*\"\n    {\n        solver          smoothSolver;\n        smoother        symGaussSeidel;\n        tolerance       1e-06;\n        relTol          0;\n    }\n}\n\nPIMPLE\n{\n    momentumPredictor no;\n    nOuterCorrectors 1;\n    nCorrectors     3;\n    nNonOrthogonalCorrectors 0;\n}\n";

/// 各分析阶段预期产出的结果场目录（供 T13 结果模型与后处理面板消费）。
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

    #[test]
    fn physical_properties_melt_requires_specific_heat_table() {
        let mut material = crate::services::material::builtin_materials()[0].clone();
        material.specific_heat = Vec::new();
        let error = physical_properties_melt(&material, &process()).unwrap_err();
        assert!(error.to_string().contains("材料比热表为空"));
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
        let material = crate::services::material::builtin_materials()[0].clone();
        let dict = molding_dict(&process());
        assert!(dict.contains("switchFraction   0.96"));
        assert!(dict.contains("(0 1e5)"));
        // MPa → Pa
        assert!(dict.contains("6.000000e7"));
        assert!(dict.contains("ejectionTemperature  363.15"));
        let momentum = momentum_transport_dict(&material);
        assert!(momentum.contains("viscosityModel  CrossWlf;"));
        let melt = physical_properties_melt(&material, &process()).unwrap();
        assert!(melt.contains("equationOfState Tait;"));
        assert!(melt.contains("latentHeat  0;"));
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
