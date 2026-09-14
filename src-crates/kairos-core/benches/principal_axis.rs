//! 对称张量主方向的性能对照：现用 `nalgebra::SymmetricEigen` 与迁移前手写 Jacobi
//! 旋回（此处保留为基准参照）。换库不能只看代码行数——这条基准保证主方向计算
//! 不会因为换实现而变慢。
//!
//! 跑法：`cargo bench -p kairos-core --bench principal_axis`

use criterion::{Criterion, criterion_group, criterion_main};
use kairos_core::services::results::principal_axis;

/// 迁移前的手写实现（Jacobi 旋回，12 次扫描）——仅作基准对照，不参与产品路径。
fn jacobi_reference(components: &[f64; 6]) -> [f64; 3] {
    let [xx, xy, xz, yy, yz, zz] = *components;
    let mut matrix = [[xx, xy, xz], [xy, yy, yz], [xz, yz, zz]];
    let mut vectors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..12 {
        let mut p = 0;
        let mut q = 1;
        let mut largest = matrix[0][1].abs();
        for (i, j) in [(0, 2), (1, 2)] {
            if matrix[i][j].abs() > largest {
                largest = matrix[i][j].abs();
                p = i;
                q = j;
            }
        }
        if largest < 1e-12 {
            break;
        }
        let theta = 0.5 * (2.0 * matrix[p][q]).atan2(matrix[q][q] - matrix[p][p]);
        let (sin, cos) = theta.sin_cos();
        for row in &mut matrix {
            let (mkp, mkq) = (row[p], row[q]);
            row[p] = cos * mkp - sin * mkq;
            row[q] = sin * mkp + cos * mkq;
        }
        {
            let (before, after) = matrix.split_at_mut(q);
            let row_q = &mut after[0];
            let row_p = &mut before[p];
            for (value_p, value_q) in row_p.iter_mut().zip(row_q.iter_mut()) {
                let (old_p, old_q) = (*value_p, *value_q);
                *value_p = cos * old_p - sin * old_q;
                *value_q = sin * old_p + cos * old_q;
            }
        }
        for row in &mut vectors {
            let (vkp, vkq) = (row[p], row[q]);
            row[p] = cos * vkp - sin * vkq;
            row[q] = sin * vkp + cos * vkq;
        }
    }
    let mut best = 0usize;
    for index in [1, 2] {
        if matrix[index][index].abs() > matrix[best][best].abs() {
            best = index;
        }
    }
    let axis = [vectors[0][best], vectors[1][best], vectors[2][best]];
    let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2])
        .sqrt()
        .max(1e-12);
    let unit = [axis[0] / norm, axis[1] / norm, axis[2] / norm];
    let dominant = if unit[1].abs() > unit[0].abs() { 1 } else { 0 };
    let dominant = if unit[2].abs() > unit[dominant].abs() {
        2
    } else {
        dominant
    };
    let sign = unit[dominant].signum();
    [unit[0] * sign, unit[1] * sign, unit[2] * sign]
}

fn principal_axis_benchmark(c: &mut Criterion) {
    // 真实张量场的典型形态：各分量量级差异大且非对角项不为零
    let fixtures: Vec<[f64; 6]> = (0..512)
        .map(|i| {
            let t = i as f64 / 512.0;
            [
                1.0e6 + t * 3.0e5,
                -2.0e5 * t,
                5.0e4 * (1.0 - t),
                8.0e5 + t * 1.0e5,
                -3.0e4 * t,
                6.0e5 - t * 2.0e5,
            ]
        })
        .collect();

    let mut group = c.benchmark_group("principal_axis");
    group.bench_function("nalgebra_symmetric_eigen", |b| {
        b.iter(|| {
            let mut acc = 0.0;
            for fixture in &fixtures {
                let axis = principal_axis(fixture);
                acc += axis[0];
            }
            acc
        })
    });
    group.bench_function("jacobi_reference", |b| {
        b.iter(|| {
            let mut acc = 0.0;
            for fixture in &fixtures {
                let axis = jacobi_reference(fixture);
                acc += axis[0];
            }
            acc
        })
    });
    group.finish();
}

criterion_group!(benches, principal_axis_benchmark);
criterion_main!(benches);
