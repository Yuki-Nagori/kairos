//! IPC 契约测试：锁定前端（`src-web/types.ts`、`src-web/lib/ipc.ts`）与 Rust serde 结构
//! 之间的序列化形状。这些测试失败意味着前端联调会爆错，而不是上线后才发现。

use kairos_core::error::{ErrorKind, KairosError};
use kairos_core::models::geometry::{Triangle, TriangleMesh};
use kairos_core::models::system::SystemInfo;
use kairos_core::services::system;
use serde_json::json;

use kairos_core::models::material::Material;
use kairos_core::models::mesh::{
    DualDomainReport, MeshQuality, MeshRefinement, MeshingReport, MidplaneReport, RefineRegion,
};
use kairos_core::models::project::{Project, Study};
use kairos_core::models::results::{DeriveRequest, ResultCatalog, ScalarField, TimeStepMeta};
use kairos_core::services::{geometry, material};

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
        cooling_channels: Vec::new(),
        process: None,
        material_id: None,
    });
    let json = serde_json::to_value(&project).unwrap();
    assert_eq!(json["schemaVersion"], 4);
    assert!(json["studies"][0]["process"].is_null());
    assert_eq!(json["name"], "演示项目");
    assert_eq!(json["studies"][0]["name"], "填充分析");
    assert_eq!(json["studies"][0]["createdMs"], 1001);
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
    };
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["engine"], "voxel");
    assert_eq!(json["nodeCount"], 27);
    assert_eq!(json["elementCount"], 40);
    assert_eq!(json["surfaceFaceCount"], 48);
    assert_eq!(json["totalVolume"], 1.0);
    assert_eq!(json["quality"]["minEdgeRatio"], 1.0);
    assert_eq!(json["quality"]["minVolume"], 0.01);
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
            "thicknessMin": 1.8,
            "thicknessMax": 2.2,
            "thicknessAvg": 2.0,
        })
    );
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
