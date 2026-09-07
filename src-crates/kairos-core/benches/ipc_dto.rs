//! IPC DTO 序列化基准：命令层往返的热路径，预算见 ai-docs/perf-budget.md。

use criterion::{Criterion, criterion_group, criterion_main};
use kairos_core::error::KairosError;
use kairos_core::models::system::SystemInfo;
use kairos_core::services::system;

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

criterion_group!(benches, bench_system_info_serialize, bench_error_serialize);
criterion_main!(benches);
