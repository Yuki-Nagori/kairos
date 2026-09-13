//! IPC DTO 序列化基准：命令层往返的热路径，预算见 ai-docs/perf-budget.md。

use criterion::{Criterion, criterion_group, criterion_main};
use kairos_core::error::KairosError;
use kairos_core::models::geometry::TriangleMesh;
use kairos_core::models::system::SystemInfo;
use kairos_core::services::fill_preview;
use kairos_core::services::gate_location::{self, GateLocationParams};
use kairos_core::services::{meshing, system};

fn bench_system_info_serialize(c: &mut Criterion) {
    let info: SystemInfo = system::system_info("kairos", env!("CARGO_PKG_VERSION"));
    c.bench_function("system_info_serialize", |b| {
        b.iter(|| serde_json::to_string(&info).expect("serialize SystemInfo"))
    });
}

fn bench_error_serialize(c: &mut Criterion) {
    let error = KairosError::solver("迭代不收敛");
    c.bench_function("kairos_error_serialize", |b| {
        b.iter(|| serde_json::to_string(&error).expect("serialize KairosError"))
    });
}

/// 浇口位置分析：62.5 万四面体网格上的候选遍历耗时（预算见 ai-docs/perf-budget.md）。
fn bench_gate_location(c: &mut Criterion) {
    let mesh = meshing::generate(
        &TriangleMesh::sample_box(50.0),
        &meshing::VolumeMeshParams {
            refinement: None,
            target_size: 1.0,
        },
    )
    .expect("mesh");
    c.bench_function("gate_location_625k_tets", |b| {
        b.iter(|| gate_location::analyze(&mesh, &GateLocationParams::default()).expect("analyze"))
    });
}

/// 填充预览：同一网格上的多源最短路（预算见 ai-docs/perf-budget.md）。
fn bench_fill_preview(c: &mut Criterion) {
    let mesh = meshing::generate(
        &TriangleMesh::sample_box(50.0),
        &meshing::VolumeMeshParams {
            refinement: None,
            target_size: 1.0,
        },
    )
    .expect("mesh");
    c.bench_function("fill_preview_625k_tets", |b| {
        b.iter(|| fill_preview::preview(&mesh, &[[25.0, 25.0, 0.0]]).expect("preview"))
    });
}

criterion_group!(
    benches,
    bench_system_info_serialize,
    bench_error_serialize,
    bench_gate_location,
    bench_fill_preview
);
criterion_main!(benches);
