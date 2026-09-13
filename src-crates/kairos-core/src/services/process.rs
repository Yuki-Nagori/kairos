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

/// 浇口名义速度窗口（m/s，SI）：Q/A_in。1~10 m/s 是常见工艺区间，
/// 超过 5 m/s 需谨慎（粗网格 / 大流量易失稳），超过 20 m/s 视为不可行
/// （所需注塑压力多半超机台，且可压缩两相求解器在局部 Mach 接近 1 时失稳）。
const GATE_VELOCITY_CAUTION_M_S: f64 = 5.0;
/// 浇口速度不可行线（m/s）。
const GATE_VELOCITY_LIMIT_M_S: f64 = 20.0;

/// 填充工况量级（由件体积、注射时间与浇口流通面积估算）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillLoad {
    /// 体积流量（cm³/s）。
    pub flow_rate_cm3_s: f64,
    /// 浇口名义速度 Q/A_in（m/s）；未给浇口面积时为 None。
    pub inlet_velocity_m_s: Option<f64>,
    /// 浇口表观剪切速率 4Q/(πR³)（1/s，R 由面积反推的等效圆半径）。
    pub gate_shear_rate_s: Option<f64>,
}

/// 估算填充工况：体积流量 Q = V/t；给了浇口流通面积 A 时再算名义速度
/// U = Q/A 与表观剪切速率 γ̇ = 4Q/(πR³)（R = √(A/π)）。全部用 SI 计算，
/// 流量另以 cm³/s 输出便于阅读。
pub fn estimate_fill_load(
    volume_mm3: f64,
    injection_time_s: f64,
    inlet_area_m2: Option<f64>,
) -> FillLoad {
    let flow_rate_m3_s = volume_mm3 * 1e-9 / injection_time_s.max(1e-9);
    let area = inlet_area_m2.filter(|area| *area > 0.0);
    FillLoad {
        flow_rate_cm3_s: flow_rate_m3_s * 1e6,
        inlet_velocity_m_s: area.map(|area| flow_rate_m3_s / area),
        gate_shear_rate_s: area.map(|area| {
            let radius = (area / std::f64::consts::PI).sqrt();
            4.0 * flow_rate_m3_s / (std::f64::consts::PI * radius.powi(3))
        }),
    }
}

/// 填充工况提示：量级明显不匹配时给出建议值（空 = 通过）。
///
/// `volume_mm3` 为件体积（来自网格报告）；`inlet_area_m2` 为浇口流通面积
/// （模具网络的 Gate 单元等效面积，或 case 里 inlet patch 的实测面积），
/// 没给面积时只做流量包络检查。
pub fn fill_load_hints(
    volume_mm3: f64,
    settings: &ProcessSettings,
    inlet_area_m2: Option<f64>,
) -> Vec<String> {
    let mut hints = Vec::new();
    if volume_mm3 <= 0.0 {
        return hints;
    }
    let load = estimate_fill_load(volume_mm3, settings.injection_time_s, inlet_area_m2);
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
    if let Some(velocity) = load.inlet_velocity_m_s
        && velocity > GATE_VELOCITY_CAUTION_M_S
    {
        let judgement = if velocity > GATE_VELOCITY_LIMIT_M_S {
            "超过 20 m/s 视为工况不可行（所需注塑压力多半超机台，且求解器在浇口局部 Mach 接近 1 时失稳）"
        } else {
            "处于需谨慎区间（5~20 m/s），粗网格或大流量下易失稳"
        };
        hints.push(format!(
            "浇口名义速度 Q/A_in = {velocity:.1} m/s，{judgement}；请核对浇口面积与注射时间。",
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
    fn estimate_fill_load_uses_si_flow_velocity_and_shear() {
        // 样例量级：1 cm³、1 s、未给浇口面积 → 只有流量
        let sample = estimate_fill_load(1000.0, 1.0, None);
        assert!((sample.flow_rate_cm3_s - 1.0).abs() < 1e-12);
        assert_eq!(sample.inlet_velocity_m_s, None);
        assert_eq!(sample.gate_shear_rate_s, None);
        // 880 cm³、1 s、浇口 8 mm 半径（面积 πR² = 2.01e-4 m²）：
        // U = Q/A ≈ 4.4 m/s；γ̇ = 4Q/(πR³) ≈ 4370 1/s
        let area = std::f64::consts::PI * 0.008_f64.powi(2);
        let big = estimate_fill_load(880_000.0, 1.0, Some(area));
        assert!((big.flow_rate_cm3_s - 880.0).abs() < 1e-9);
        let velocity = big.inlet_velocity_m_s.unwrap();
        assert!((velocity - 8.8e-4 / area).abs() < 1e-9, "U={velocity}");
        let shear = big.gate_shear_rate_s.unwrap();
        assert!((shear - 4.0 * 8.8e-4 / (std::f64::consts::PI * 0.008_f64.powi(3))).abs() < 1e-6);
        // 面积为 0 / 负值 / 时间为 0 都不产生除零
        assert_eq!(
            estimate_fill_load(1000.0, 1.0, Some(0.0)).inlet_velocity_m_s,
            None
        );
        assert_eq!(
            estimate_fill_load(1000.0, 1.0, Some(-1.0)).inlet_velocity_m_s,
            None
        );
        assert!(
            estimate_fill_load(1000.0, 0.0, None)
                .flow_rate_cm3_s
                .is_finite()
        );
    }

    #[test]
    fn fill_load_hints_cover_flow_velocity_and_shear() {
        let mut settings = valid();
        settings.injection_time_s = 1.0;
        // 样例量级且无浇口面积 → 静默
        assert!(fill_load_hints(1000.0, &settings, None).is_empty());
        assert!(fill_load_hints(1000.0, &settings, Some(1e-4)).is_empty());
        // 体积为 0 或负 → 跳过
        assert!(fill_load_hints(0.0, &settings, None).is_empty());
        assert!(fill_load_hints(-5.0, &settings, None).is_empty());
        // 880 cm³、1 s：大浇口（1e-3 m² → U=0.88 m/s）只报流量包络
        let big_gate = fill_load_hints(880_000.0, &settings, Some(1e-3));
        assert_eq!(big_gate.len(), 1, "{big_gate:?}");
        assert!(big_gate[0].contains("880 cm³/s"), "{}", big_gate[0]);
        assert!(big_gate[0].contains("≥ 1.8 s"), "{}", big_gate[0]);
        // 小浇口（1e-4 m² → U≈8.8 m/s，谨慎区间）→ 流量 + 速度两条
        let small_gate = fill_load_hints(880_000.0, &settings, Some(1e-4));
        assert_eq!(small_gate.len(), 2, "{small_gate:?}");
        assert!(small_gate[1].contains("浇口名义速度"), "{}", small_gate[1]);
        assert!(small_gate[1].contains("5~20 m/s"), "{}", small_gate[1]);
        // 极细浇口（1e-6 m² → U≈880 m/s）→ 速度判为不可行
        let tiny_gate = fill_load_hints(880_000.0, &settings, Some(1e-6));
        assert!(
            tiny_gate.iter().any(|hint| hint.contains("不可行")),
            "{tiny_gate:?}"
        );
        // 极细且流量中等（1e-5 m² → γ̇≈2e5 1/s）→ 出现剪切速率提示
        let sheared = fill_load_hints(880_000.0, &settings, Some(1e-5));
        assert!(
            sheared.iter().any(|hint| hint.contains("剪切速率")),
            "{sheared:?}"
        );
    }

    #[test]
    fn rejects_non_increasing_packing_curve() {
        let mut settings = valid();
        settings.packing_pressure_mpa_curve = vec![(0.0, 60.0), (0.0, 50.0)];
        let issues = validate(&settings);
        assert!(issues.iter().any(|issue| issue.contains("严格递增")));
    }
}
