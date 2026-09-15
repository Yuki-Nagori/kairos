//! IPC 契约测试：锁定前端（`src-web/types.ts`、`src-web/lib/ipc.ts`）与 Rust serde 结构
//! 之间的序列化形状。这些测试失败意味着前端联调会爆错，而不是上线后才发现。

use kairos_core::error::{ErrorKind, KairosError};
use kairos_core::models::geometry::{Triangle, TriangleMesh};
use kairos_core::models::system::SystemInfo;
use kairos_core::services::system;
use serde_json::json;

use kairos_core::models::material::{BlowingGroup, Material};
use kairos_core::models::mesh::{
    DualDomainReport, DualDomainSolverInput, MeshEstimate, MeshQuality, MeshRefinement,
    MeshingReport, MidplaneReport, RefineRegion,
};
use kairos_core::models::project::{GeometryRef, Project, Study};
use kairos_core::models::results::{
    DeriveRequest, ResultCatalog, ScalarField, TimeStepMeta, VectorField,
};
use kairos_core::models::runners::{CoolingChannel, RunnerElement, RunnerKind, RunnerMedium};
use kairos_core::models::solver::{CaseOutcome, EnvironmentCheck};
use kairos_core::services::moldingfoam::{CaseReport, GateInlet, PatchAreas};
use kairos_core::services::{geometry, material};

/// ImportOutcome 的形状：导入摘要 + 日志行，前端几何面板日志区与之对应。
#[test]
fn import_outcome_serializes_with_camel_case() {
    let mesh = TriangleMesh::sample_box(1.0);
    let summary = geometry::summarize("g-1".into(), "盒.stl".into(), &mesh);
    let outcome = kairos_core::models::geometry::ImportOutcome {
        log: vec!["导入 STL：盒.stl".into()],
        summary,
    };
    let json = serde_json::to_value(&outcome).unwrap();
    assert_eq!(json["summary"]["geometryId"], "g-1");
    assert_eq!(json["summary"]["triangleCount"], 12);
    assert_eq!(json["log"][0], "导入 STL：盒.stl");
}

/// SystemInfo 的形状：camelCase 字段，前端 `src-web/types.ts` 的 SystemInfo 与之对应。
#[test]
fn system_info_serializes_with_camel_case() {
    let info: SystemInfo = system::system_info("kairos", "0.1.0");
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(
        json,
        json!({
            "name": "kairos",
            "version": "0.1.0",
            "os": std::env::consts::OS,
        })
    );
}

/// 错误契约：`{ code, message }` 两字段，前端 CommandError 按 code 分类。
#[test]
fn error_serializes_to_code_message_contract() {
    let error = KairosError::new(ErrorKind::Validation, "参数超出量程");
    let json = serde_json::to_value(&error).unwrap();
    assert_eq!(
        json,
        json!({ "code": "validation", "message": "参数超出量程" })
    );

    let solver_error = KairosError::solver("迭代不收敛");
    let json = serde_json::to_value(&solver_error).unwrap();
    assert_eq!(json["code"], "solver");
}

/// Project 的形状：camelCase + schemaVersion，前端 `src-web/types.ts` 与之对应。
#[test]
fn project_serializes_with_camel_case() {
    let mut project = Project::new("p-1".into(), "演示项目".into(), 1000);
    project.studies.push(Study {
        id: "s-1".into(),
        name: "填充分析".into(),
        created_ms: 1001,
        runner_elements: Vec::new(),
        cooling_channels: vec![CoolingChannel {
            id: "cc-1".into(),
            diameter_mm: 8.0,
            start: [0.0, 0.0, 0.0],
            end: [10.0, 0.0, 0.0],
            inlet_temp_c: 25.0,
            mass_flow_rate_kg_s: 0.05,
            specific_heat_j_kg_k: 4180.0,
        }],
        process: None,
        material_id: None,
    });
    let json = serde_json::to_value(&project).unwrap();
    assert_eq!(json["schemaVersion"], 5);
    // v5：工作区几何引用（相对路径），空工程为空数组
    assert_eq!(json["geometries"], json!([]));
    assert!(json["studies"][0]["process"].is_null());
    assert_eq!(json["name"], "演示项目");
    assert_eq!(json["studies"][0]["name"], "填充分析");
    assert_eq!(json["studies"][0]["createdMs"], 1001);
    // 冷却水路：模壁 1D 通道 BC 的口径（质量流量 / 比热）随工程文件持久化
    assert_eq!(
        json["studies"][0]["coolingChannels"][0]["massFlowRateKgS"],
        0.05
    );
    assert_eq!(
        json["studies"][0]["coolingChannels"][0]["specificHeatJKgK"],
        4180.0
    );
}

#[test]
fn runner_medium_serializes_with_snake_case() {
    let runner = RunnerElement {
        id: "gate-1".into(),
        kind: RunnerKind::Gate,
        diameter_mm: 2.0,
        start: [0.0; 3],
        end: [1.0, 0.0, 0.0],
        medium: Some(RunnerMedium::Gas),
    };
    let value = serde_json::to_value(runner).unwrap();
    assert_eq!(value["kind"], "gate");
    assert_eq!(value["medium"], "gas");
    assert_eq!(value["diameterMm"], 2.0);
}

/// BlowingGroup 的形状：camelCase 微发泡近似参数，前端材料面板与之对应。
#[test]
fn blowing_group_serializes_with_camel_case() {
    let group = BlowingGroup {
        kind: "N₂".into(),
        mass_fraction_percent: 2.0,
        density_reduction_percent: 12.0,
        viscosity_reduction_percent: 20.0,
        note: "工程默认量级".into(),
    };
    let json = serde_json::to_value(&group).unwrap();
    assert_eq!(
        json,
        json!({
            "kind": "N₂",
            "massFractionPercent": 2.0,
            "densityReductionPercent": 12.0,
            "viscosityReductionPercent": 20.0,
            "note": "工程默认量级",
        })
    );
}

/// GeometryRef 的形状：camelCase 相对路径引用，前端工程树与之对应。
#[test]
fn geometry_ref_serializes_with_camel_case() {
    let reference = GeometryRef {
        id: "geom-1".into(),
        file_name: "part.stl".into(),
        relative_path: "geometry/part.stl".into(),
    };
    let json = serde_json::to_value(&reference).unwrap();
    assert_eq!(
        json,
        json!({
            "id": "geom-1",
            "fileName": "part.stl",
            "relativePath": "geometry/part.stl",
        })
    );
}

/// Material 的形状：camelCase 参数组，前端 `src-web/types.ts` 与之对应。
#[test]
fn material_serializes_with_camel_case() {
    let material: Material = material::builtin_materials().remove(0);
    let json = serde_json::to_value(&material).unwrap();
    assert_eq!(json["family"], "PP");
    assert_eq!(json["rheology"]["tauStar"], 2.0e4);
    assert_eq!(json["pvt"]["b1m"], 1.28e-3);
    assert_eq!(json["specificHeat"][0], json!([300.0, 1900.0]));
    assert_eq!(json["mechanics"]["elasticModulus"], 1.5e9);
    assert_eq!(json["filler"], serde_json::Value::Null);
    assert_eq!(json["blowing"], serde_json::Value::Null);
    assert!(json["dataNote"].is_string());
}

/// GeometrySummary 的形状：摘要字段 camelCase，issues 子对象对应前端 MeshIssues。
#[test]
fn geometry_summary_serializes_with_camel_case() {
    let triangles = vec![Triangle {
        a: [0.0, 0.0, 0.0],
        b: [1.0, 0.0, 0.0],
        c: [0.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
    }];
    let mesh = TriangleMesh { triangles };
    let summary = geometry::summarize("g-1".into(), "demo.stl".into(), &mesh);
    let json = serde_json::to_value(&summary).unwrap();
    assert_eq!(json["geometryId"], "g-1");
    assert_eq!(json["fileName"], "demo.stl");
    assert_eq!(json["triangleCount"], 1);
    assert_eq!(json["suggestedUnit"], "mm");
    assert_eq!(json["issues"]["degenerate"], 0);
    assert_eq!(json["issues"]["openEdges"], 3);
}

/// RepairOutcome 的形状：summary + report 两级 camelCase，前端修复面板与之对应。
#[test]
fn repair_outcome_serializes_with_camel_case() {
    let triangles = vec![Triangle {
        a: [0.0, 0.0, 0.0],
        b: [1.0, 0.0, 0.0],
        c: [0.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
    }];
    let summary = geometry::summarize("g-1".into(), "demo.stl".into(), &TriangleMesh { triangles });
    let outcome = kairos_core::models::repair::RepairOutcome {
        summary,
        report: kairos_core::models::repair::RepairReport {
            merged_vertices: 3,
            removed_degenerate: 1,
            filled_holes: 2,
            filled_triangles: 4,
            flipped_faces: 5,
            self_intersections: 0,
        },
    };
    let json = serde_json::to_value(&outcome).unwrap();
    assert_eq!(json["summary"]["geometryId"], "g-1");
    assert_eq!(json["summary"]["issues"]["openEdges"], 3);
    assert_eq!(json["report"]["mergedVertices"], 3);
    assert_eq!(json["report"]["removedDegenerate"], 1);
    assert_eq!(json["report"]["filledHoles"], 2);
    assert_eq!(json["report"]["filledTriangles"], 4);
    assert_eq!(json["report"]["flippedFaces"], 5);
    assert_eq!(json["report"]["selfIntersections"], 0);
}

/// MeshingReport 的形状：统计字段 camelCase，前端网格面板与之对应。
#[test]
fn meshing_report_serializes_with_camel_case() {
    let report = MeshingReport {
        engine: "voxel".into(),
        node_count: 27,
        element_count: 40,
        surface_face_count: 48,
        total_volume: 1.0,
        quality: MeshQuality {
            min_edge_ratio: 1.0,
            avg_edge_ratio: 1.3,
            max_edge_ratio: 1.73,
            min_volume: 0.01,
        },
        aspect_max: 2.4,
        aspect_avg: 1.5,
        thin_feature_hints: vec![
            "目标尺寸 5.00 mm 超过最薄特征的 1/2（5% 分位壁厚 3.00 mm）。".into(),
        ],
    };
    let restored = kairos_core::models::mesh::RestoredStudyMesh {
        geometry_id: "geo-2".into(),
        report: report.clone(),
    };
    let restored_json = serde_json::to_value(restored).unwrap();
    assert_eq!(restored_json["geometryId"], "geo-2");
    assert_eq!(restored_json["report"]["nodeCount"], 27);
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["engine"], "voxel");
    assert_eq!(json["nodeCount"], 27);
    assert_eq!(json["elementCount"], 40);
    assert_eq!(json["surfaceFaceCount"], 48);
    assert_eq!(json["totalVolume"], 1.0);
    assert_eq!(json["quality"]["minEdgeRatio"], 1.0);
    assert_eq!(json["quality"]["minVolume"], 0.01);
    assert_eq!(json["aspectMax"], 2.4);
    assert_eq!(json["aspectAvg"], 1.5);
    assert_eq!(
        json["thinFeatureHints"][0],
        "目标尺寸 5.00 mm 超过最薄特征的 1/2（5% 分位壁厚 3.00 mm）。"
    );
}

/// MeshEstimate 的形状：估算字段 camelCase，前端几何面板的生成前预览与之对应。
#[test]
fn mesh_estimate_serializes_with_camel_case() {
    let estimate = MeshEstimate {
        engine: "voxel".into(),
        cell_count: 8,
        element_count: 40,
        over_limit: false,
        basis: "包围盒上限".into(),
        cell_limit: 2_000_000,
    };
    let json = serde_json::to_value(&estimate).unwrap();
    assert_eq!(
        json,
        json!({
            "engine": "voxel",
            "cellCount": 8,
            "elementCount": 40,
            "overLimit": false,
            "basis": "包围盒上限",
            "cellLimit": 2_000_000,
        })
    );
}

/// 浇口入口口径回显的形状：camelCase，前端工艺面板的「请求 vs 实际」行与之对应。
#[test]
fn gate_inlet_report_serializes_with_camel_case() {
    let gate = GateInlet {
        index: 1,
        requested_radius_mm: 4.0,
        requested_area_mm2: 50.26548245743669,
        actual_area_mm2: 1200.5,
        face_count: 7,
        equivalent_diameter_mm: 39.1,
        area_ratio: 23.9,
        expressible: false,
        min_face_area_mm2: 493.0,
    };
    let json = serde_json::to_value(&gate).unwrap();
    assert_eq!(json["index"], 1);
    assert_eq!(json["requestedRadiusMm"], 4.0);
    assert_eq!(json["actualAreaMm2"], 1200.5);
    assert_eq!(json["faceCount"], 7);
    assert_eq!(json["areaRatio"], 23.9);
    assert_eq!(json["expressible"], false);
    assert!(json["minFaceAreaMm2"].is_number());

    let report = CaseReport::new(
        PatchAreas {
            inlet_m2: 1.2e-3,
            vent_m2: 2.5e-5,
            walls_m2: 3.0e-3,
        },
        vec![gate],
    );
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["inletAreaM2"], 1.2e-3);
    assert_eq!(json["ventAreaM2"], 2.5e-5);
    assert_eq!(json["gates"][0]["index"], 1);
    assert_eq!(json["warnings"].as_array().map(Vec::len), Some(1));
}

#[test]
fn solver_dtos_serialize_with_camel_case() {
    let environment = serde_json::to_value(EnvironmentCheck {
        moldingfoam: true,
        solver: false,
        hint: "install solver".into(),
    })
    .unwrap();
    assert_eq!(environment["moldingfoam"], true);
    assert_eq!(environment["solver"], false);

    let outcome = serde_json::to_value(CaseOutcome {
        case_dir: "case".into(),
        inlet_area_m2: 1.0,
        inlet_equivalent_diameter_mm: 2.0,
        gates: Vec::new(),
        warnings: vec!["warning".into()],
    })
    .unwrap();
    assert_eq!(outcome["caseDir"], "case");
    assert_eq!(outcome["inletEquivalentDiameterMm"], 2.0);
}

/// VectorField 的形状：三分量与 camelCase，前端结果面板的矢量行与之对应。
#[test]
fn vector_field_serializes_with_camel_case() {
    let field = VectorField {
        field: "D".into(),
        time_dir: "2".into(),
        time_s: 2.0,
        components: vec![[0.001, -0.002, 0.0]],
        complete: true,
    };
    let json = serde_json::to_value(&field).unwrap();
    assert_eq!(json["field"], "D");
    assert_eq!(json["timeDir"], "2");
    assert_eq!(json["timeS"], 2.0);
    assert_eq!(json["components"][0], json!([0.001, -0.002, 0.0]));
    assert_eq!(json["complete"], true);
}

/// DualDomainReport 的形状：统计字段 camelCase，前端几何面板与之对应。
#[test]
fn dual_domain_report_serializes_with_camel_case() {
    let report = DualDomainReport {
        node_count: 9,
        triangle_count: 12,
        beam_count: 2,
        coupling_count: 3,
        uncoupled_endpoints: 1,
        unpaired_triangles: 0,
        match_ratio: 0.75,
        thickness_min: 1.8,
        thickness_max: 2.2,
        thickness_avg: 2.0,
    };
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(
        json,
        json!({
            "nodeCount": 9,
            "triangleCount": 12,
            "beamCount": 2,
            "couplingCount": 3,
            "uncoupledEndpoints": 1,
            "unpairedTriangles": 0,
            "matchRatio": 0.75,
            "thicknessMin": 1.8,
            "thicknessMax": 2.2,
            "thicknessAvg": 2.0,
        })
    );
}

#[test]
fn dual_domain_solver_input_serializes_with_schema_and_units() {
    let input = DualDomainSolverInput {
        schema_version: "dual-domain/v1".into(),
        length_unit: "mm".into(),
        thickness_unit: "mm".into(),
        nodes: vec![[0.0, 0.0, 0.0]],
        triangles: vec![],
        thickness: vec![],
        beams: vec![],
        couplings: vec![],
    };
    let json = serde_json::to_value(input).unwrap();
    assert_eq!(json["schemaVersion"], "dual-domain/v1");
    assert_eq!(json["lengthUnit"], "mm");
    assert_eq!(json["thicknessUnit"], "mm");
}

/// MidplaneReport 的形状：统计字段 camelCase，前端几何面板与之对应。
#[test]
fn midplane_report_serializes_with_camel_case() {
    let report = MidplaneReport {
        node_count: 8,
        element_count: 12,
        beam_count: 1,
        coupling_count: 1,
        uncoupled_endpoints: 1,
        unpaired_vertices: 2,
        dropped_elements: 3,
        thickness_min: 1.9,
        thickness_max: 2.1,
        thickness_avg: 2.0,
    };
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["nodeCount"], 8);
    assert_eq!(json["elementCount"], 12);
    assert_eq!(json["beamCount"], 1);
    assert_eq!(json["couplingCount"], 1);
    assert_eq!(json["uncoupledEndpoints"], 1);
    assert_eq!(json["unpairedVertices"], 2);
    assert_eq!(json["droppedElements"], 3);
    assert_eq!(json["thicknessMin"], 1.9);
    assert_eq!(json["thicknessMax"], 2.1);
    assert_eq!(json["thicknessAvg"], 2.0);
}

/// 结果模型形状：时间步与标量场（complete 标记不完整结果）。
#[test]
fn result_models_serialize_with_camel_case() {
    let meta = TimeStepMeta {
        dir_name: "0.5".into(),
        time_s: 0.5,
        fields: vec!["T".into(), "U".into()],
    };
    let json = serde_json::to_value(&meta).unwrap();
    assert_eq!(json["dirName"], "0.5");
    assert_eq!(json["timeS"], 0.5);
    assert_eq!(json["fields"], serde_json::json!(["T", "U"]));

    let field = ScalarField {
        field: "T".into(),
        time_dir: "0.5".into(),
        time_s: 0.5,
        values: vec![300.0],
        is_magnitude: false,
        complete: true,
    };
    let json = serde_json::to_value(&field).unwrap();
    assert_eq!(json["timeDir"], "0.5");
    assert_eq!(json["isMagnitude"], false);
    assert_eq!(json["complete"], true);

    let catalog = ResultCatalog {
        case_dir: "/c".into(),
        times: vec![meta],
    };
    let json = serde_json::to_value(&catalog).unwrap();
    assert_eq!(json["caseDir"], "/c");
}

/// VmStatus 的形状：camelCase 字段 + snake_case 枚举值，
/// 前端 `src-web/types.ts` 的 VmStatus / VmState 与之对应。
#[test]
fn vm_status_serializes_with_camel_case() {
    use kairos_core::models::vm::{VmProviderKind, VmState, VmStatus};
    let status = VmStatus {
        provider: VmProviderKind::Multipass,
        tool_installed: true,
        instance_name: "kairos".into(),
        instance_state: VmState::Running,
        hint: "虚拟机运行中".into(),
    };
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(
        json,
        json!({
            "provider": "multipass",
            "toolInstalled": true,
            "instanceName": "kairos",
            "instanceState": "running",
            "hint": "虚拟机运行中",
        })
    );
}

/// UpdateCheck 的形状：camelCase 字段 + Option 的 null 语义，
/// 前端 `src-web/types.ts` 的 UpdateCheck 与之对应。
#[test]
fn update_check_serializes_with_camel_case() {
    use kairos_core::models::dependencies::UpdateCheck;
    let check = UpdateCheck {
        component_id: "moldingfoam".into(),
        installed_tag: Some("v0.1.1".into()),
        latest_tag: Some("v0.2.0".into()),
        update_available: true,
    };
    let json = serde_json::to_value(&check).unwrap();
    assert_eq!(
        json,
        json!({
            "componentId": "moldingfoam",
            "installedTag": "v0.1.1",
            "latestTag": "v0.2.0",
            "updateAvailable": true,
        })
    );
}

/// MeshRefinement 的形状：内部标记 tag = mode，字段 camelCase，
/// 前端以可辨识联合类型与之对应。
#[test]
fn mesh_refinement_serializes_with_mode_tag() {
    let layers = MeshRefinement::BoundaryLayers {
        layers: 2,
        ratio: 0.5,
    };
    assert_eq!(
        serde_json::to_value(layers).unwrap(),
        json!({ "mode": "boundaryLayers", "layers": 2, "ratio": 0.5 })
    );
    let region = MeshRefinement::Region {
        region: RefineRegion {
            min: [0.0, 0.0, 0.0],
            max: [4.0, 4.0, 4.0],
        },
        levels: 2,
    };
    let json = serde_json::to_value(region).unwrap();
    assert_eq!(json["mode"], "region");
    assert_eq!(json["levels"], 2);
    assert_eq!(json["region"]["min"], json!([0.0, 0.0, 0.0]));
    assert_eq!(json["region"]["max"], json!([4.0, 4.0, 4.0]));
}

/// DeriveRequest 的形状：内部标记 tag = kind，字段 camelCase，
/// 前端以可辨识联合类型与之对应。
#[test]
fn derive_request_serializes_with_kind_tag() {
    let linear = DeriveRequest::Linear {
        scale: 2.0,
        offset: -1.0,
    };
    assert_eq!(
        serde_json::to_value(linear).unwrap(),
        json!({ "kind": "linear", "scale": 2.0, "offset": -1.0 })
    );
    assert_eq!(
        serde_json::to_value(DeriveRequest::Normalize).unwrap(),
        json!({ "kind": "normalize" })
    );
    assert_eq!(
        serde_json::to_value(DeriveRequest::Difference).unwrap(),
        json!({ "kind": "difference" })
    );
}

/// RenderMeshData 的形状：camelCase 字段，前端视口上传用（云图按 faceCells 着色）。
#[test]
fn render_mesh_data_serializes_with_camel_case() {
    let data = kairos_core::models::render::RenderMeshData {
        positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        indices: vec![0, 1, 2],
        face_cells: vec![7],
    };
    let json = serde_json::to_value(&data).unwrap();
    assert_eq!(json["positions"].as_array().map(Vec::len), Some(9));
    assert_eq!(json["indices"], json!([0, 1, 2]));
    assert_eq!(json["faceCells"], json!([7]));
}

#[test]
fn probe_series_contract_uses_camel_case() {
    use kairos_core::models::results::{Probe, ProbeSample, ProbeTimeSeries};
    let probe: Probe = serde_json::from_value(serde_json::json!({"id":2,"nodeIndex":7})).unwrap();
    let series = ProbeTimeSeries {
        probe_id: probe.id,
        node_index: probe.node_index,
        samples: vec![ProbeSample {
            time_s: 0.5,
            value: 42.0,
        }],
    };
    assert_eq!(
        serde_json::to_value(series).unwrap(),
        serde_json::json!({"probeId":2,"nodeIndex":7,"samples":[{"timeS":0.5,"value":42.0}]})
    );
}
