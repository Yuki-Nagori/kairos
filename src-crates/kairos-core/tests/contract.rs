//! IPC 契约测试：锁定前端（`src-web/types.ts`、`src-web/lib/ipc.ts`）与 Rust serde 结构
//! 之间的序列化形状。这些测试失败意味着前端联调会爆错，而不是上线后才发现。

use kairos_core::error::{ErrorKind, KairosError};
use kairos_core::models::system::SystemInfo;
use kairos_core::services::system;
use serde_json::json;

use kairos_core::models::geometry::{Triangle, TriangleMesh};
use kairos_core::models::material::Material;
use kairos_core::models::project::{Project, Study};
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
    });
    let json = serde_json::to_value(&project).unwrap();
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["name"], "演示项目");
    assert_eq!(json["studies"][0]["name"], "填充分析");
    assert_eq!(json["studies"][0]["createdMs"], 1001);
}

/// Material 的形状：camelCase 参数组，前端 `src-web/types.ts` 与之对应。
#[test]
fn material_serializes_with_camel_case() {
    let material: Material = material::builtin_materials().unwrap().remove(0);
    let json = serde_json::to_value(&material).unwrap();
    assert_eq!(json["family"], "PP");
    assert_eq!(json["rheology"]["tauStar"], 2.0e4);
    assert_eq!(json["pvt"]["b1m"], 1.28e-3);
    assert_eq!(json["specificHeat"][0], json!([300.0, 1900.0]));
    assert_eq!(json["mechanics"]["elasticModulus"], 1.5e9);
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
