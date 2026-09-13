//! IPC DTO 序列化基准：命令层往返的热路径，预算见 ai-docs/perf-budget.md。

use criterion::{Criterion, criterion_group, criterion_main};
use kairos_core::error::KairosError;
use kairos_core::models::geometry::TriangleMesh;
use kairos_core::models::results::{ScalarField, VectorField};
use kairos_core::models::system::SystemInfo;
use kairos_core::services::fill_preview;
use kairos_core::services::gate_location::{self, GateLocationParams};
use kairos_core::services::results::{field_binary, parse_internal_scalar};
use kairos_core::services::{meshing, system};

/// 确定性大结果（1e7 个值，混合温度 / 压力量级）：基准资产每次构造一致。
fn scalar_field_1e7() -> ScalarField {
    let count = 10_000_000;
    let values = (0..count)
        .map(|index| {
            let t = index as f64 * 1e-3;
            300.0 + (t * 0.7).sin() * 40.0 + 1.0e5 * (t * 0.03).cos() * 1e-4
        })
        .collect();
    ScalarField {
        field: "T".into(),
        time_dir: "1".into(),
        time_s: 1.0,
        values,
        is_magnitude: false,
        complete: true,
    }
}

/// 与 `scalar_field_1e7` 同规模的 OpenFOAM ascii 场文本（解析链路基准）。
fn field_ascii_1e7() -> String {
    let count = 10_000_000usize;
    let mut content = String::with_capacity(count * 12 + 256);
    content.push_str(
        "FoamFile\n{\n    version 2.0;\n    format ascii;\n    class volScalarField;\n    object T;\n}\n",
    );
    content.push_str("dimensions [0 0 0 1 0 0 0];\n");
    content.push_str(&format!(
        "internalField nonuniform List<scalar>\n{count}\n(\n"
    ));
    for index in 0..count {
        let t = index as f64 * 1e-3;
        let value = 300.0 + (t * 0.7).sin() * 40.0;
        content.push_str(&format!("{value:.6}\n"));
    }
    content.push_str(")\n;\nboundaryField {}\n");
    content
}

/// 位移矢量场（三分量，1e6 个单元）：矢量通道基准。
fn vector_field_1e6() -> VectorField {
    let count = 1_000_000;
    let components = (0..count)
        .map(|index| {
            let t = index as f64 * 1e-3;
            [t.sin() * 1e-4, (t * 0.5).cos() * 1e-4, t * 1e-7]
        })
        .collect();
    VectorField {
        field: "D".into(),
        time_dir: "2".into(),
        time_s: 2.0,
        components,
        complete: true,
    }
}

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

/// 大结果二进制链路：1e7 值的 f32（默认）与 f64 编码 / 解码耗时与体积。
fn bench_field_binary(c: &mut Criterion) {
    let field = scalar_field_1e7();
    let f32_bytes = field_binary::encode_scalar(&field, field_binary::ValueFormat::F32);
    let f64_bytes = field_binary::encode_scalar(&field, field_binary::ValueFormat::F64);
    println!(
        "field_binary 1e7 值：f32 {} MB / f64 {} MB",
        f32_bytes.len() / 1_000_000,
        f64_bytes.len() / 1_000_000
    );
    c.bench_function("field_binary_encode_f32_1e7", |b| {
        b.iter(|| field_binary::encode_scalar(&field, field_binary::ValueFormat::F32))
    });
    c.bench_function("field_binary_encode_f64_1e7", |b| {
        b.iter(|| field_binary::encode_scalar(&field, field_binary::ValueFormat::F64))
    });
    c.bench_function("field_binary_decode_f32_1e7", |b| {
        b.iter(|| field_binary::decode(&f32_bytes).expect("decode"))
    });
    c.bench_function("field_binary_decode_f64_1e7", |b| {
        b.iter(|| field_binary::decode(&f64_bytes).expect("decode"))
    });

    let vectors = vector_field_1e6();
    let vector_bytes = field_binary::encode_vector(&vectors);
    println!(
        "field_binary 1e6 矢量：{} MB",
        vector_bytes.len() / 1_000_000
    );
    c.bench_function("field_binary_encode_vector_1e6", |b| {
        b.iter(|| field_binary::encode_vector(&vectors))
    });
    c.bench_function("field_binary_decode_vector_1e6", |b| {
        b.iter(|| field_binary::decode_vector(&vector_bytes).expect("decode"))
    });
}

/// 大结果文本解析（OpenFOAM ascii 场 → 值表）：加载链路的真正大头。
fn bench_field_parse(c: &mut Criterion) {
    let content = field_ascii_1e7();
    println!("field 文本 1e7 值：{} MB", content.len() / 1_000_000);
    c.bench_function("field_parse_ascii_1e7", |b| {
        b.iter(|| parse_internal_scalar(&content))
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
    bench_fill_preview,
    bench_field_binary,
    bench_field_parse
);
criterion_main!(benches);
