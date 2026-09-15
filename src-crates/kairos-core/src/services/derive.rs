//! 派生场的**共享规则**：命名后缀、结果信封与错误文案的唯一出处。
//!
//! 数值实现按精度分两处——CPU 在 [`crate::services::results`] 用 f64，GPU 在适配层
//! 走 f32 着色器。**但命名与信封必须同源**：同一个请求在两条路径上若得出不同的场名，
//! 前端图例、场对比与报告就会跟着漂。本模块只放这些「与精度无关、两条路径都依赖」的
//! 部分，不放数值核。

use crate::error::{KairosError, Result};
use crate::models::results::ScalarField;

/// 归一化场名后缀。
pub const NAME_NORMALIZE: &str = "归一化";
/// 阈值掩码场名后缀。
pub const NAME_THRESHOLD: &str = "阈值掩码";

/// 线性映射的场名后缀（含实际参数，便于在界面上区分多次映射）。
pub fn linear_name(scale: f64, offset: f64) -> String {
    format!("线性映射 ×{scale} {offset:+}")
}

/// 单场入口收到「两场差值」请求时的错误。
///
/// 差值需要主场与对比场两份数据，单场入口无法受理——两条路径共用同一句文案。
pub fn difference_request_error() -> KairosError {
    KairosError::validation("两场差值请使用 derive_difference 命令（需要主场与对比场）。")
}

/// 两场差值前的长度校验（不静默截断）。
pub fn validate_difference_lengths(primary: &ScalarField, compare: &ScalarField) -> Result<()> {
    if primary.values.len() == compare.values.len() {
        return Ok(());
    }
    Err(KairosError::validation(format!(
        "两场长度不一致：{} 有 {} 个值，{} 有 {} 个值。",
        primary.field,
        primary.values.len(),
        compare.field,
        compare.values.len()
    )))
}

/// 单场派生结果的信封：场名追加后缀，其余字段沿用基场（含时间步与完整性）。
///
/// `is_magnitude` 置 false：派生结果不再是「矢量模量」，而是对值本身做的变换。
pub fn derived_field(base: &ScalarField, suffix: &str, values: Vec<f64>) -> ScalarField {
    ScalarField {
        field: format!("{} · {suffix}", base.field),
        time_dir: base.time_dir.clone(),
        time_s: base.time_s,
        values,
        is_magnitude: false,
        complete: base.complete,
    }
}

/// 两场差值结果的信封：命名「主场 - 对比场」，时间步继承主场，完整性取两者之与
/// （任一侧不完整则结果不完整）。
pub fn difference_field(
    primary: &ScalarField,
    compare: &ScalarField,
    values: Vec<f64>,
) -> ScalarField {
    ScalarField {
        field: format!("{} - {}", primary.field, compare.field),
        time_dir: primary.time_dir.clone(),
        time_s: primary.time_s,
        values,
        is_magnitude: false,
        complete: primary.complete && compare.complete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    fn field(name: &str, values: Vec<f64>) -> ScalarField {
        ScalarField {
            field: name.to_string(),
            time_dir: "2".to_string(),
            time_s: 2.0,
            values,
            is_magnitude: true,
            complete: true,
        }
    }

    #[test]
    fn linear_name_carries_parameters_with_sign() {
        assert_eq!(linear_name(2.0, -1.0), "线性映射 ×2 -1");
        assert_eq!(linear_name(0.5, 3.0), "线性映射 ×0.5 +3");
    }

    #[test]
    fn difference_request_error_names_the_right_command() {
        let error = difference_request_error();
        assert_eq!(error.kind(), ErrorKind::Validation);
        assert!(error.message().contains("derive_difference"));
    }

    #[test]
    fn length_mismatch_is_reported_with_both_field_names() {
        let error = validate_difference_lengths(&field("T", vec![1.0]), &field("T0", vec![1.0; 3]))
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Validation);
        assert!(error.message().contains("T 有 1 个值"));
        assert!(error.message().contains("T0 有 3 个值"));
    }

    #[test]
    fn matching_lengths_pass_validation() {
        assert!(
            validate_difference_lengths(&field("T", vec![1.0]), &field("T0", vec![2.0])).is_ok()
        );
    }

    #[test]
    fn derived_field_appends_suffix_and_resets_magnitude() {
        let derived = derived_field(&field("T", vec![1.0]), NAME_NORMALIZE, vec![0.5]);
        assert_eq!(derived.field, "T · 归一化");
        assert_eq!(derived.time_dir, "2");
        assert_eq!(derived.time_s, 2.0);
        assert_eq!(derived.values, vec![0.5]);
        // 基场是模量，派生结果不再是模量
        assert!(!derived.is_magnitude);
        assert!(derived.complete);
    }

    #[test]
    fn derived_field_keeps_incomplete_marker() {
        let mut base = field("T", vec![1.0]);
        base.complete = false;
        assert!(!derived_field(&base, NAME_THRESHOLD, vec![0.0]).complete);
    }

    #[test]
    fn difference_field_combines_completeness() {
        let complete = field("T", vec![1.0]);
        let mut incomplete = field("T0", vec![1.0]);
        incomplete.complete = false;
        let derived = difference_field(&complete, &incomplete, vec![0.0]);
        assert_eq!(derived.field, "T - T0");
        assert!(!derived.complete);
        assert!(!derived.is_magnitude);
    }

    #[test]
    fn difference_field_is_complete_when_both_are() {
        let derived = difference_field(&field("T", vec![3.0]), &field("T0", vec![1.0]), vec![2.0]);
        assert_eq!(derived.field, "T - T0");
        assert!(derived.complete);
    }
}
