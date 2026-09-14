//! KF1 字段载荷与网格存储的编解码基准：换 `bytemuck` 之前先留基线。
//!
//! 这两条路径是结果加载的热路径（解码直接进 IPC / 渲染），换实现前必须先量化，
//! 换完用同一基准对照——性能退步就不换。
//!
//! 跑法：`cargo bench -p kairos-core --bench kf1_codec`

use criterion::{Criterion, criterion_group, criterion_main};
use kairos_core::models::mesh::VolumeMesh;
use kairos_core::models::results::ScalarField;
use kairos_core::services::{mesh_store, results};

/// 造一个规模接近真实结果的标量场（十万节点量级）。
fn sample_field(values: usize) -> ScalarField {
    ScalarField {
        field: "T".to_string(),
        time_dir: "1.2".to_string(),
        time_s: 1.2,
        is_magnitude: false,
        complete: true,
        values: (0..values).map(|i| 300.0 + (i as f64) * 0.001).collect(),
    }
}

/// 造一个等规模体网格（节点 + 四面体）。
fn sample_mesh(nodes: usize) -> VolumeMesh {
    let positions = (0..nodes)
        .map(|i| {
            [
                i as f64 * 0.01,
                (i % 97) as f64 * 0.02,
                (i % 53) as f64 * 0.03,
            ]
        })
        .collect::<Vec<_>>();
    let elements = (0..nodes.saturating_sub(4))
        .map(|i| [i, i + 1, i + 2, i + 3])
        .collect::<Vec<_>>();
    VolumeMesh {
        nodes: positions,
        tets: elements,
        surface_faces: Vec::new(),
    }
}

fn codec_benchmark(c: &mut Criterion) {
    let field = sample_field(100_000);
    let field_bytes = results::field_binary::encode(&field);
    let mesh = sample_mesh(50_000);
    let mesh_bytes = mesh_store::encode(&mesh);

    let mut group = c.benchmark_group("kf1_codec");
    group.bench_function("field_encode_100k", |b| {
        b.iter(|| results::field_binary::encode(&field))
    });
    group.bench_function("field_decode_100k", |b| {
        b.iter(|| results::field_binary::decode(&field_bytes).expect("解码"))
    });
    group.bench_function("mesh_encode_50k", |b| b.iter(|| mesh_store::encode(&mesh)));
    group.bench_function("mesh_decode_50k", |b| {
        b.iter(|| mesh_store::decode(&mesh_bytes).expect("解码"))
    });
    group.finish();
}

criterion_group!(benches, codec_benchmark);
criterion_main!(benches);
