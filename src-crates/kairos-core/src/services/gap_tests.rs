//! 覆盖率缺口补测（T28 后的 100% 门槛任务）：集中覆盖各服务此前未触达的
//! 校验分支、错误路径与边界情况。全部通过公开 API 驱动，不依赖私有实现；
//! 作为 crate 内单元测试纳入 lib 覆盖率统计。

use crate::error::{ErrorKind, KairosError};
use crate::models::geometry::{Triangle, TriangleMesh};
use crate::models::jobs::{Job, JobStatus};
use crate::models::material::Material;
use crate::models::mesh::VolumeMesh;
use crate::models::process::ProcessSettings;
use crate::models::project::Study;
use crate::models::runners::{RunnerElement, RunnerKind};
use crate::models::solver::AnalysisStage;
use crate::services::system::system_info;
use crate::services::{
    geometry, gmsh, jobs as job_service, material as material_service, meshing, openfoam,
    process as process_service, project as project_service, results as results_service,
    runners as runners_service,
};

// ---------- 共享构造 ----------

fn valid_material() -> Material {
    let mut material = material_service::builtin_materials().remove(0);
    material.id = "gap-test".into();
    material
}

fn valid_process() -> ProcessSettings {
    ProcessSettings {
        melt_temp_c: 230.0,
        mold_temp_c: 40.0,
        ejection_temp_c: 90.0,
        injection_time_s: 1.5,
        vp_switch_volume_percent: 96.0,
        packing_pressure_mpa_curve: vec![(0.0, 60.0), (1.0, 70.0)],
        packing_time_s: 5.0,
        cooling_time_s: 20.0,
        coolant_temp_c: 25.0,
    }
}

fn sample_mesh() -> TriangleMesh {
    TriangleMesh::sample_box(10.0)
}

fn binary_stl_bytes() -> Vec<u8> {
    let mut bytes = vec![0u8; 80];
    bytes.extend_from_slice(&1u32.to_le_bytes());
    let mut triangle = Vec::with_capacity(50);
    for value in [
        0.0f32, 0.0, 0.0, // 法向
        0.0, 0.0, 0.0, //
        1.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, //
    ] {
        triangle.extend_from_slice(&value.to_le_bytes());
    }
    triangle.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend(triangle);
    bytes
}

// ---------- error.rs ----------

#[test]
fn error_kind_as_code_covers_all_variants() {
    assert_eq!(ErrorKind::Validation.as_code(), "validation");
    assert_eq!(ErrorKind::NotFound.as_code(), "not_found");
    assert_eq!(ErrorKind::Io.as_code(), "io");
    assert_eq!(ErrorKind::Solver.as_code(), "solver");
    assert_eq!(ErrorKind::Internal.as_code(), "internal");
}

#[test]
fn io_error_converts_into_io_kind() {
    let error = KairosError::from(std::io::Error::other("disk full"));
    assert_eq!(error.kind(), ErrorKind::Io);
    assert!(error.message().contains("disk full"));
}

// ---------- models/jobs.rs ----------

#[test]
fn job_transition_records_timestamps() {
    let mut job = Job::new("j1".into(), None, "case".into(), 2, 100);
    assert!(job.transition(JobStatus::Running, 150).is_some());
    assert_eq!(job.started_ms, Some(150));
    assert!(job.transition(JobStatus::Done, 300).is_some());
    assert_eq!(job.finished_ms, Some(300));
    // Done → Failed 非法
    assert!(job.transition(JobStatus::Failed, 400).is_none());
}

// ---------- services/jobs.rs ----------

fn two_queued_jobs(jobs: &mut Vec<Job>, cores_a: u32, cores_b: u32) {
    job_service::submit(jobs, "a".into(), None, "case-a".into(), cores_a, 10).unwrap();
    job_service::submit(jobs, "b".into(), None, "case-b".into(), cores_b, 20).unwrap();
}

#[test]
fn submit_rejects_zero_cores() {
    let mut jobs = Vec::new();
    let error = job_service::submit(&mut jobs, "j".into(), None, "case".into(), 0, 10).unwrap_err();
    assert!(error.message().contains("核数"));
}

#[test]
fn promote_breaks_at_concurrency_limit() {
    let mut jobs = Vec::new();
    two_queued_jobs(&mut jobs, 2, 2);
    let limits = job_service::SchedulerLimits::new(1, 8);
    let started = job_service::promote_ready(&mut jobs, &limits, 100);
    assert_eq!(started, vec!["a"]);
    let started_second_round = job_service::promote_ready(&mut jobs, &limits, 200);
    assert!(started_second_round.is_empty(), "并发已满，应触发 break");
}

#[test]
fn promote_skips_jobs_over_core_budget() {
    let mut jobs = Vec::new();
    two_queued_jobs(&mut jobs, 4, 4);
    let limits = job_service::SchedulerLimits::new(4, 6);
    let started = job_service::promote_ready(&mut jobs, &limits, 100);
    assert_eq!(started, vec!["a"], "b 超出核数预算，应 continue 跳过");
}

#[test]
fn mark_running_lifecycle_and_illegal_repeat() {
    let mut jobs = Vec::new();
    two_queued_jobs(&mut jobs, 2, 2);
    job_service::mark_running(&mut jobs, "a", 100).unwrap();
    assert_eq!(jobs[0].status, JobStatus::Running);
    // running → running 非法
    assert!(job_service::mark_running(&mut jobs, "a", 150).is_err());
}

#[test]
fn update_progress_requires_running_state() {
    let mut jobs = Vec::new();
    two_queued_jobs(&mut jobs, 2, 2);
    assert!(job_service::update_progress(&mut jobs, "a", 1.0).is_err());
    job_service::mark_running(&mut jobs, "a", 100).unwrap();
    job_service::update_progress(&mut jobs, "a", 2.5).unwrap();
    assert_eq!(jobs[0].last_time_s, Some(2.5));
}

// ---------- services/geometry.rs ----------

#[test]
fn parse_stl_rejects_unrecognized_short_file() {
    let error = geometry::parse_stl(b"short").unwrap_err();
    assert!(error.message().contains("无法识别"));
}

#[test]
fn parse_stl_short_ascii_reports_no_triangles() {
    // 长度 < 84 且带 solid 头：走 ASCII 路径且无 facet
    let error = geometry::parse_stl(b"solid empty\nendsolid empty\n").unwrap_err();
    assert!(error.message().contains("没有解析到任何三角形"));
}

#[test]
fn parse_ascii_rejects_non_finite_numbers() {
    let content = "solid bad\nfacet normal 0 0 0\n  outer loop\n    vertex nan 0 0\n    \
                   vertex 1 0 0\n    vertex 0 1 0\n  endloop\nendfacet\nendsolid bad\n";
    let error = geometry::parse_stl(content.as_bytes()).unwrap_err();
    assert!(error.message().contains("不是有限数"));
}

#[test]
fn parse_ascii_rejects_wrong_vertex_count() {
    let content = "solid bad\nfacet normal 0 0 0\n  outer loop\n    vertex 0 0 0\n    \
                   vertex 1 0 0\n  endloop\nendfacet\nendsolid bad\n";
    let error = geometry::parse_stl(content.as_bytes()).unwrap_err();
    assert!(error.message().contains("facet 顶点数为 2"));
}

#[test]
fn parse_stl_file_reads_from_disk() {
    let dir = std::env::temp_dir().join(format!("kairos-stl-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.stl");
    std::fs::write(&path, binary_stl_bytes()).unwrap();
    let mesh = geometry::parse_stl_file(&path).unwrap();
    assert_eq!(mesh.triangles.len(), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_mesh_flags_self_loop_as_non_manifold() {
    // 焊接容差 = 对角线 × 1e-6，退化阈值 = 对角线 × 1e-9：
    // 用一个巨型三角形撑大对角线，另一个近重合顶点的三角形
    // 面积高于退化阈值、顶点却落入焊接容差 → 自环 → 非流形。
    let mesh = TriangleMesh {
        triangles: vec![
            Triangle {
                a: [0.0, 0.0, 0.0],
                b: [0.001, 0.0, 0.0],
                c: [0.0005, 0.02, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            Triangle {
                a: [5000.0, 0.0, 0.0],
                b: [0.0, 5000.0, 0.0],
                c: [0.0, 0.0, 5000.0],
                normal: [1.0, 1.0, 1.0],
            },
        ],
    };
    let issues = geometry::check_mesh(&mesh);
    assert!(issues.non_manifold_edges > 0, "{issues:?}");
}

#[test]
fn infer_unit_reports_unknown_for_non_positive() {
    assert_eq!(geometry::infer_unit(0.0), "未知");
    assert_eq!(geometry::infer_unit(f64::NAN), "未知");
}

// ---------- services/gmsh.rs ----------

fn valid_tet_msh() -> String {
    [
        "$MeshFormat",
        "2.2 0 8",
        "$EndMeshFormat",
        "$Nodes",
        "4",
        "1 0 0 0",
        "2 1 0 0",
        "3 0 1 0",
        "4 0 0 1",
        "$EndNodes",
        "$Elements",
        "2",
        "1 1 2 1 2 1 2",
        "2 4 4 1 2 3 4 1 2 3 4",
        "$EndElements",
    ]
    .join("\n")
}

#[test]
fn parse_msh_skips_unparseable_element_type_tokens() {
    // 第一行类型 token 非数字 → continue；第二行是合法四面体
    let mesh =
        gmsh::parse_msh_v2(&valid_tet_msh().replace("1 1 2 1 2 1 2", "1 x 2 1 2 1 2")).unwrap();
    assert_eq!(mesh.tets.len(), 1);
}

#[test]
fn parse_msh_rejects_short_tet_row() {
    let broken = valid_tet_msh().replace("2 4 4 1 2 3 4 1 2 3 4", "2 4 2");
    let error = gmsh::parse_msh_v2(&broken).unwrap_err();
    assert!(error.message().contains("字段不足"));
}

// ---------- services/material.rs ----------

/// 校验用例：变更函数 + 期望的错误信息片段。
type MutationCase<'a, M> = (Box<dyn Fn(&mut M)>, &'a str);

#[test]
fn material_validate_rejects_each_bad_field() {
    let cases: Vec<MutationCase<'_, Material>> = vec![
        (Box::new(|m: &mut Material| m.name = "  ".into()), "牌号"),
        (Box::new(|m: &mut Material| m.family = "".into()), "材料族"),
        (
            Box::new(|m: &mut Material| m.rheology.tau_star = 0.0),
            "Cross-WLF",
        ),
        (
            Box::new(|m: &mut Material| m.rheology.d1 = -1.0),
            "Cross-WLF",
        ),
        (Box::new(|m: &mut Material| m.rheology.d2 = 0.0), "D2"),
        (Box::new(|m: &mut Material| m.pvt.b1m = 0.0), "Tait PVT"),
        (
            Box::new(|m: &mut Material| {
                m.pvt.b1s = m.pvt.b1m;
            }),
            "b1s",
        ),
        (Box::new(|m: &mut Material| m.pvt.b5 = 0.0), "b5"),
        (
            Box::new(|m: &mut Material| m.specific_heat = vec![(300.0, -1.0)]),
            "非法数值",
        ),
    ];
    for (mutate, expected) in cases {
        let mut material = valid_material();
        mutate(&mut material);
        let error = material_service::validate(&material).unwrap_err();
        assert!(
            error.message().contains(expected),
            "{error:?} 应包含 {expected}"
        );
    }
}

#[test]
fn material_merge_pushes_entries_with_new_ids() {
    let existing = vec![valid_material()];
    let mut incoming = valid_material();
    incoming.id = "brand-new".into();
    let merged = material_service::merge_custom(existing, vec![incoming.clone()]);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[1].id, "brand-new");
}

// ---------- services/meshing.rs ----------

#[test]
fn meshing_rejects_target_size_that_yields_no_voxels() {
    // 两个分离的封闭立方体共享一个大包围盒：超大目标尺寸下唯一体素
    // 中心落在两立方体之间的空洞里，射线无交点 → 零体素。
    let mut shifted = TriangleMesh::sample_box(2.0);
    for triangle in &mut shifted.triangles {
        for vertex in [&mut triangle.a, &mut triangle.b, &mut triangle.c] {
            *vertex = [vertex[0] + 8.0, vertex[1] + 8.0, vertex[2] + 8.0];
        }
    }
    let mut triangles = TriangleMesh::sample_box(2.0).triangles;
    triangles.extend(shifted.triangles);
    let hollow = TriangleMesh { triangles };

    let params = meshing::VolumeMeshParams {
        refinement: None,
        target_size: 5.0,
    };
    let error = meshing::generate(&hollow, &params).unwrap_err();
    assert!(error.message().contains("未生成任何体素"));
}

#[test]
fn meshing_report_handles_empty_mesh() {
    let report = meshing::report(&VolumeMesh::default());
    assert_eq!(report.element_count, 0);
}

// ---------- services/process.rs ----------

#[test]
fn process_validate_reports_each_issue_branch() {
    let cases: Vec<MutationCase<'_, ProcessSettings>> = vec![
        (
            Box::new(|p: &mut ProcessSettings| p.packing_pressure_mpa_curve = Vec::new()),
            "保压压力曲线不能为空",
        ),
        (
            Box::new(|p: &mut ProcessSettings| {
                p.packing_pressure_mpa_curve = vec![(-1.0, 50.0), (2.0, 60.0)];
            }),
            "保压曲线时间必须为非负数",
        ),
        (
            Box::new(|p: &mut ProcessSettings| {
                p.packing_pressure_mpa_curve = vec![(0.0, 600.0)];
            }),
            "保压压力须在 0 ~ 500",
        ),
        (
            Box::new(|p: &mut ProcessSettings| p.mold_temp_c = -5.0),
            "模具温度须在 0 ~ 250",
        ),
        (
            Box::new(|p: &mut ProcessSettings| p.ejection_temp_c = 999.0),
            "顶出温度不能高于熔体温度",
        ),
        (
            Box::new(|p: &mut ProcessSettings| p.injection_time_s = 700.0),
            "注射时间须在 0 ~ 600",
        ),
        (
            Box::new(|p: &mut ProcessSettings| p.vp_switch_volume_percent = 150.0),
            "V/P 切换点须在 0 ~ 100",
        ),
        (
            Box::new(|p: &mut ProcessSettings| p.packing_time_s = -1.0),
            "保压时间不能为负数",
        ),
        (
            Box::new(|p: &mut ProcessSettings| p.cooling_time_s = -1.0),
            "冷却时间不能为负数",
        ),
        (
            Box::new(|p: &mut ProcessSettings| p.coolant_temp_c = 500.0),
            "冷却介质温度须在 -20 ~ 200",
        ),
    ];
    for (mutate, expected) in cases {
        let mut settings = valid_process();
        mutate(&mut settings);
        let issues = process_service::validate(&settings);
        assert!(
            issues.iter().any(|issue| issue.contains(expected)),
            "{issues:?} 应包含 {expected}"
        );
    }
}

#[test]
fn process_validate_accepts_reference_settings() {
    assert!(process_service::validate(&valid_process()).is_empty());
}

// ---------- services/project.rs ----------

#[test]
fn project_create_rejects_blank_name() {
    let error = project_service::create("   ", 0).unwrap_err();
    assert!(error.message().contains("不能为空"));
}

#[test]
fn project_validate_rejects_duplicate_study_names() {
    let mut project = project_service::create("p", 0).unwrap();
    for _ in 0..2 {
        project.studies.push(Study {
            id: format!("s-{}", project.studies.len()),
            name: "同名".into(),
            created_ms: 0,
            runner_elements: vec![],
            cooling_channels: vec![],
            process: None,
            material_id: None,
        });
    }
    let error = project_service::validate(&project).unwrap_err();
    assert!(error.message().contains("研究名称重复"));
}

#[test]
fn write_atomic_reports_rename_failure() {
    let dir = std::env::temp_dir().join(format!("kairos-write-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // 目标路径是一个目录：rename 到既有目录必然失败，覆盖错误分支
    let target = dir.join("x.kairos");
    std::fs::create_dir_all(&target).unwrap();
    let error = project_service::write_atomic(&target, "content").unwrap_err();
    assert!(error.message().contains("工程文件替换失败"));
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---------- services/results.rs ----------

#[test]
fn scan_times_rejects_missing_dir() {
    let error =
        results_service::scan_times(std::path::Path::new("/nonexistent-case-xyz")).unwrap_err();
    assert!(error.message().contains("结果目录不存在"));
}

#[test]
fn scan_times_skips_files_and_non_numeric_dirs() {
    let dir = std::env::temp_dir().join(format!("kairos-scan-{}", std::process::id()));
    let time_a = dir.join("0.05");
    let time_b = dir.join("0.1");
    std::fs::create_dir_all(&time_a).unwrap();
    std::fs::create_dir_all(&time_b).unwrap();
    std::fs::create_dir_all(dir.join("system")).unwrap();
    std::fs::write(dir.join("notes.txt"), "not a timestep").unwrap();
    // APFS（macOS）强制 UTF-8 文件名，只有 Linux 能创建非法 UTF-8 目录名。
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        let bad = std::ffi::OsStr::from_bytes(b"\xff\xfe-bad");
        std::fs::create_dir_all(dir.join(bad)).unwrap();
    }
    std::fs::write(time_a.join("p"), "internalField uniform 0;").unwrap();
    std::fs::write(time_b.join("U"), "internalField uniform 0;").unwrap();

    let catalog = results_service::scan_times(&dir).unwrap();
    assert_eq!(catalog.times.len(), 2);
    assert_eq!(catalog.times[0].dir_name, "0.05");
    assert_eq!(catalog.times[0].fields, vec!["p"]);
    assert_eq!(catalog.times[1].fields, vec!["U"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn parse_scalar_without_internal_field_is_incomplete() {
    let (values, complete) = results_service::parse_internal_scalar("boundaryField { }");
    assert!(values.is_empty());
    assert!(!complete);
}

#[test]
fn parse_scalar_uniform_without_list_is_incomplete() {
    let (values, complete) = results_service::parse_internal_scalar("internalField uniform;");
    assert!(values.is_empty() || values.iter().any(|v| v.is_nan()));
    assert!(!complete);
}

#[test]
fn parse_scalar_uniform_without_paren_is_incomplete() {
    // internalField 在花括号内声明 uniform 但没有列表括号
    let (values, complete) = results_service::parse_internal_scalar("internalField { uniform x; }");
    assert!(values.is_empty());
    assert!(!complete);
}

#[test]
fn parse_vector_magnitudes_truncates_partial_vectors() {
    let content = "internalField nonuniform List<vector>\n4\n(1 0 0 2 0 0 3 0)\n;";
    let (magnitudes, complete) = results_service::parse_internal_vector_magnitudes(content);
    assert_eq!(magnitudes.len(), 2, "4 个分量取前 3 个为一组");
    assert!(!complete);
}

// ---------- services/runners.rs ----------

#[test]
fn runner_check_flags_nan_coordinates() {
    let runner = RunnerElement {
        id: "r1".into(),
        kind: RunnerKind::Runner,
        diameter_mm: 5.0,
        start: [f64::NAN, 0.0, 0.0],
        end: [1.0, 0.0, 0.0],
    };
    let issues = runners_service::check_mold_network(&[runner], &[]);
    assert!(issues.iter().any(|issue| issue.contains("非法数值")));
}

// ---------- services/openfoam.rs ----------

#[test]
fn generate_case_reports_write_failure_when_target_is_directory() {
    let dir = std::env::temp_dir().join(format!("kairos-case-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("system").join("fvSchemes")).unwrap();
    let params = meshing::VolumeMeshParams {
        refinement: None,
        target_size: 2.5,
    };
    let volume_mesh = meshing::generate(&sample_mesh(), &params).unwrap();
    let error = openfoam::generate_case(
        &dir,
        &volume_mesh,
        &valid_material(),
        &valid_process(),
        &AnalysisStage::Fill,
        2,
    )
    .unwrap_err();
    assert!(error.message().contains("写入") || error.message().contains("创建目录"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn generate_case_reports_write_failure_at_constant_dictionaries() {
    // 逐个把 constant 下的字典文件预埋成目录，验证各 write()? 的失败传播：
    // momentumTransport / physicalProperties.melt / physicalProperties.air。
    let params = meshing::VolumeMeshParams {
        refinement: None,
        target_size: 2.5,
    };
    let volume_mesh = meshing::generate(&sample_mesh(), &params).unwrap();
    for target in [
        "constant/momentumTransport",
        "constant/physicalProperties.melt",
        "constant/physicalProperties.air",
    ] {
        let dir = std::env::temp_dir().join(format!(
            "kairos-case-{}-{}",
            target.replace(['/', '.'], "-"),
            std::process::id()
        ));
        std::fs::create_dir_all(dir.join(target)).unwrap();
        let error = openfoam::generate_case(
            &dir,
            &volume_mesh,
            &valid_material(),
            &valid_process(),
            &AnalysisStage::Fill,
            2,
        )
        .unwrap_err();
        assert!(error.message().contains("写入"), "{target}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn cancel_reports_missing_job() {
    let mut jobs: Vec<Job> = Vec::new();
    let error = job_service::cancel(&mut jobs, "ghost", 1_000).unwrap_err();
    assert!(error.to_string().contains("作业不存在"));
}

#[test]
fn kairos_error_serializes_to_code_message() {
    let json = serde_json::to_string(&KairosError::validation("boom")).unwrap();
    assert_eq!(json, r#"{"code":"validation","message":"boom"}"#);
}

#[test]
fn system_info_passes_name_and_version_through() {
    let info = system_info("kairos", "9.9.9");
    assert_eq!(info.name, "kairos");
    assert_eq!(info.version, "9.9.9");
}
