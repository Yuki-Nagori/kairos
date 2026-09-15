use kairos_core::models::results::ScalarField;
use kairos_core::services::results::{field_stats, scalar_field_csv};

#[test]
#[ignore = "显式基准入口：cargo test -p kairos-core --test large_result_benchmark -- --ignored --nocapture"]
fn large_result_stats_benchmark() {
    let values: Vec<f64> = (0..1_000_000)
        .map(|index| (index as f64 * 0.001).sin())
        .collect();
    let field = ScalarField {
        field: "T".into(),
        time_dir: "1".into(),
        time_s: 1.0,
        values,
        is_magnitude: false,
        complete: true,
    };
    let stats_start = std::time::Instant::now();
    let stats = field_stats(&field.values);
    let stats_ms = stats_start.elapsed().as_secs_f64() * 1_000.0;
    let csv_start = std::time::Instant::now();
    let csv_bytes = scalar_field_csv(&field).len();
    let csv_ms = csv_start.elapsed().as_secs_f64() * 1_000.0;
    assert_eq!(stats.count, 1_000_000);
    println!(
        "large_result_stats count={} stats_ms={stats_ms:.2} csv_ms={csv_ms:.2} csv_bytes={csv_bytes}",
        stats.count
    );
}
