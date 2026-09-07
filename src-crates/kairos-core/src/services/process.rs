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
    fn rejects_non_increasing_packing_curve() {
        let mut settings = valid();
        settings.packing_pressure_mpa_curve = vec![(0.0, 60.0), (0.0, 50.0)];
        let issues = validate(&settings);
        assert!(issues.iter().any(|issue| issue.contains("严格递增")));
    }
}
