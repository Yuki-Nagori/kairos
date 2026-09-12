//! 工艺设置校验：参数量程与曲线合法性，返回问题清单（空 = 通过）。

use crate::models::process::{PressureCurve, ProcessSettings};

fn check_curve(curve: &PressureCurve, max_mpa: f64, issues: &mut Vec<String>) {
    if curve.is_empty() {
        issues.push("保压压力曲线不能为空。".into());
        return;
    }
    let mut previous_time: Option<f64> = None;
    for (time, pressure) in curve {
        if !time.is_finite() || *time < 0.0 {
            issues.push("保压曲线时间必须为非负数。".into());
            continue;
        }
        if let Some(previous_time) = previous_time
            && *time <= previous_time
        {
            issues.push("保压曲线时间必须严格递增。".into());
            continue;
        }
        if !pressure.is_finite() || *pressure < 0.0 || *pressure > max_mpa {
            issues.push(format!("保压压力须在 0 ~ {max_mpa} MPa 之间。"));
        }
        previous_time = Some(*time);
    }
}

/// 常规注塑机体积流量包络（cm³/s）。
///
/// 小件（1 cm³、1 s 注射）约 1 cm³/s；880 cm³ 的件用 1 s 注射则达 880 cm³/s，
/// 这种量级不匹配会让填充压力远超常规注塑机能力——该检查拦的就是它，
/// 不替代求解稳定性判断。
const MACHINE_FLOW_LIMIT_CM3_S: f64 = 500.0;

/// 浇口表观剪切速率上限（1/s）：常见聚合物建议不超过 5×10⁴。
const GATE_SHEAR_LIMIT_S: f64 = 5.0e4;

/// 填充工况量级（由件体积、注射时间与浇口半径估算）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillLoad {
    /// 体积流量（cm³/s）。
    pub flow_rate_cm3_s: f64,
    /// 浇口表观剪切速率（1/s）；未给浇口半径时为 None。
    pub gate_shear_rate_s: Option<f64>,
}

/// 估算填充工况：体积流量 Q = V/t，浇口表观剪切速率 γ̇ = 4Q/(πR³)（圆管）。
/// 体积单位 mm³、时间 s、半径 mm，故 Q 需换算成 cm³/s 输出。
pub fn estimate_fill_load(
    volume_mm3: f64,
    injection_time_s: f64,
    gate_radius_mm: Option<f64>,
) -> FillLoad {
    let flow_rate_mm3_s = volume_mm3 / injection_time_s.max(1e-9);
    FillLoad {
        flow_rate_cm3_s: flow_rate_mm3_s / 1000.0,
        gate_shear_rate_s: gate_radius_mm
            .filter(|radius| *radius > 0.0)
            .map(|radius| 4.0 * flow_rate_mm3_s / (std::f64::consts::PI * radius.powi(3))),
    }
}

/// 填充工况提示：量级明显不匹配时给出建议值（空 = 通过）。
///
/// `volume_mm3` 为件体积（来自网格报告）；`gate_radius_mm` 为浇口半径
/// （来自模具网络的 Gate 单元），没有浇口时跳过剪切速率检查。
pub fn fill_load_hints(
    volume_mm3: f64,
    settings: &ProcessSettings,
    gate_radius_mm: Option<f64>,
) -> Vec<String> {
    let mut hints = Vec::new();
    if volume_mm3 <= 0.0 {
        return hints;
    }
    let load = estimate_fill_load(volume_mm3, settings.injection_time_s, gate_radius_mm);
    if load.flow_rate_cm3_s > MACHINE_FLOW_LIMIT_CM3_S {
        let minimum_time_s = volume_mm3 / 1000.0 / MACHINE_FLOW_LIMIT_CM3_S;
        hints.push(format!(
            "注射 {:.2} s 对应体积流量 {:.0} cm³/s，超出常规注塑机包络（≈{:.0} cm³/s）；件体积 {:.0} cm³，建议注射时间 ≥ {:.1} s。",
            settings.injection_time_s,
            load.flow_rate_cm3_s,
            MACHINE_FLOW_LIMIT_CM3_S,
            volume_mm3 / 1000.0,
            minimum_time_s
        ));
    }
    if let Some(shear_rate) = load.gate_shear_rate_s
        && shear_rate > GATE_SHEAR_LIMIT_S
    {
        hints.push(format!(
            "浇口表观剪切速率 {:.1e} 1/s 超过常见聚合物上限（{:.0e} 1/s）；建议加大浇口直径或延长注射时间。",
            shear_rate, GATE_SHEAR_LIMIT_S
        ));
    }
    hints
}

/// 校验工艺设置，返回问题清单（空 = 通过）。
pub fn validate(settings: &ProcessSettings) -> Vec<String> {
    let mut issues = Vec::new();

    if !(20.0..=400.0).contains(&settings.melt_temp_c) {
        issues.push("熔体温度须在 20 ~ 400 °C 之间。".into());
    }
    if !(0.0..=250.0).contains(&settings.mold_temp_c) {
        issues.push("模具温度须在 0 ~ 250 °C 之间。".into());
    }
    if settings.ejection_temp_c < settings.mold_temp_c {
        issues.push("顶出温度不能低于模具温度。".into());
    }
    if !(0.0..=settings.melt_temp_c).contains(&settings.ejection_temp_c) {
        issues.push("顶出温度不能高于熔体温度。".into());
    }
    if !(0.0..=600.0).contains(&settings.injection_time_s) {
        issues.push("注射时间须在 0 ~ 600 s 之间。".into());
    }
    if !(0.0..=100.0).contains(&settings.vp_switch_volume_percent) {
        issues.push("V/P 切换点须在 0 ~ 100 之间。".into());
    }
    check_curve(&settings.packing_pressure_mpa_curve, 500.0, &mut issues);
    if settings.packing_time_s < 0.0 {
        issues.push("保压时间不能为负数。".into());
    }
    if settings.cooling_time_s < 0.0 {
        issues.push("冷却时间不能为负数。".into());
    }
    if !(-20.0..=200.0).contains(&settings.coolant_temp_c) {
        issues.push("冷却介质温度须在 -20 ~ 200 °C 之间。".into());
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::process::ProcessSettings;

    fn valid() -> ProcessSettings {
        ProcessSettings {
            melt_temp_c: 230.0,
            mold_temp_c: 40.0,
            ejection_temp_c: 90.0,
            injection_time_s: 1.5,
            vp_switch_volume_percent: 96.0,
            packing_pressure_mpa_curve: vec![(0.0, 60.0), (1.0, 55.0), (8.0, 40.0)],
            packing_time_s: 8.0,
            cooling_time_s: 15.0,
            coolant_temp_c: 25.0,
        }
    }

    #[test]
    fn valid_settings_pass() {
        assert!(validate(&valid()).is_empty());
    }

    #[test]
    fn rejects_out_of_range_temperatures() {
        let mut settings = valid();
        settings.melt_temp_c = 500.0;
        let issues = validate(&settings);
        assert!(issues.iter().any(|issue| issue.contains("熔体温度")));
    }

    #[test]
    fn rejects_ejection_below_mold_temp() {
        let mut settings = valid();
        settings.mold_temp_c = 100.0;
        let issues = validate(&settings);
        assert!(issues.iter().any(|issue| issue.contains("顶出温度")));
    }

    #[test]
    fn estimate_fill_load_scales_with_volume_time_and_gate() {
        // 样例方盒：1 cm³、1 s → 1 cm³/s；未给浇口 → 无剪切速率
        let sample = estimate_fill_load(1000.0, 1.0, None);
        assert!((sample.flow_rate_cm3_s - 1.0).abs() < 1e-12);
        assert_eq!(sample.gate_shear_rate_s, None);
        // 880 cm³、1 s → 880 cm³/s；浇口半径 8 mm → 4Q/(πR³) ≈ 547 1/s
        let big = estimate_fill_load(880_000.0, 1.0, Some(8.0));
        assert!((big.flow_rate_cm3_s - 880.0).abs() < 1e-9);
        let shear = big.gate_shear_rate_s.unwrap();
        assert!((shear - 4.0 * 880_000.0 / (std::f64::consts::PI * 512.0)).abs() < 1e-6);
        // 半径为 0 / 负值 / 时间为 0 都不产生除零
        assert_eq!(
            estimate_fill_load(1000.0, 1.0, Some(0.0)).gate_shear_rate_s,
            None
        );
        assert_eq!(
            estimate_fill_load(1000.0, 1.0, Some(-1.0)).gate_shear_rate_s,
            None
        );
        assert!(
            estimate_fill_load(1000.0, 0.0, None)
                .flow_rate_cm3_s
                .is_finite()
        );
    }

    #[test]
    fn fill_load_hints_flag_order_of_magnitude_mismatch() {
        let mut settings = valid();
        settings.injection_time_s = 1.0;
        // 样例量级：1 cm³、1 s → 无提示
        assert!(fill_load_hints(1000.0, &settings, Some(1.0)).is_empty());
        // 体积为 0 或负 → 跳过（没有网格信息时不提示）
        assert!(fill_load_hints(0.0, &settings, None).is_empty());
        assert!(fill_load_hints(-5.0, &settings, None).is_empty());
        // 880 cm³、1 s → 流量超包络，建议值 ≈ 1.8 s
        let hints = fill_load_hints(880_000.0, &settings, Some(8.0));
        assert_eq!(hints.len(), 1, "{hints:?}");
        assert!(hints[0].contains("880 cm³/s"), "{}", hints[0]);
        assert!(hints[0].contains("≥ 1.8 s"), "{}", hints[0]);
        // 细浇口 + 高流量 → 剪切速率提示（880 cm³/s、半径 1 mm → 1.1e6 1/s）
        let sheared = fill_load_hints(880_000.0, &settings, Some(1.0));
        assert_eq!(sheared.len(), 2, "{sheared:?}");
        assert!(sheared[1].contains("浇口表观剪切速率"), "{}", sheared[1]);
        // 注射时间拉长到 2 s → 流量回到包络内，只剩剪切提示（半径 1 mm）
        settings.injection_time_s = 2.0;
        let relaxed = fill_load_hints(880_000.0, &settings, Some(1.0));
        assert_eq!(relaxed.len(), 1, "{relaxed:?}");
        assert!(relaxed[0].contains("浇口表观剪切速率"), "{}", relaxed[0]);
    }

    #[test]
    fn rejects_non_increasing_packing_curve() {
        let mut settings = valid();
        settings.packing_pressure_mpa_curve = vec![(0.0, 60.0), (0.0, 50.0)];
        let issues = validate(&settings);
        assert!(issues.iter().any(|issue| issue.contains("严格递增")));
    }
}
