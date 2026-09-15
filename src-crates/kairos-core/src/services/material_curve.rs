//! 通用材料曲线输入：CSV 解析、单位归一和数据质量校验。

use csv::StringRecord;
use serde::{Deserialize, Serialize};

use crate::error::{KairosError, Result};
use crate::models::material::CrossWlf;

#[derive(Debug, Clone, PartialEq)]
pub struct PvtPoint {
    pub pressure_pa: f64,
    pub temperature_k: f64,
    pub specific_volume_m3_per_kg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViscosityPoint {
    pub temperature_k: f64,
    pub shear_rate_per_s: f64,
    pub viscosity_pa_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidualSummary {
    pub count: usize,
    pub max_absolute_pa_s: f64,
    pub rmse_pa_s: f64,
    pub max_absolute_log10: f64,
    pub rmse_log10: f64,
}

/// 使用 Cross-WLF 参数计算零压力下的表观黏度（Pa·s）。
pub fn cross_wlf_viscosity(
    model: &CrossWlf,
    temperature_k: f64,
    shear_rate_per_s: f64,
) -> Result<f64> {
    if !temperature_k.is_finite() || temperature_k <= 0.0 {
        return Err(KairosError::validation("Cross-WLF 温度必须为正有限数值。"));
    }
    if !shear_rate_per_s.is_finite() || shear_rate_per_s <= 0.0 {
        return Err(KairosError::validation(
            "Cross-WLF 剪切速率必须为正有限数值。",
        ));
    }
    let denominator = model.a2 + temperature_k - model.d2;
    if !denominator.is_finite() || denominator.abs() < f64::EPSILON {
        return Err(KairosError::validation("Cross-WLF 温度移位分母不能为零。"));
    }
    let shift = (-model.a1 * (temperature_k - model.d2) / denominator).exp();
    let zero_shear = model.d1 * shift;
    if !zero_shear.is_finite() || zero_shear <= 0.0 {
        return Err(KairosError::validation(
            "Cross-WLF 零剪切黏度必须为正有限数值。",
        ));
    }
    let ratio = zero_shear * shear_rate_per_s / model.tau_star;
    let viscosity = zero_shear / (1.0 + ratio.powf(1.0 - model.n));
    if !viscosity.is_finite() || viscosity <= 0.0 {
        return Err(KairosError::validation(
            "Cross-WLF 计算结果必须为正有限数值。",
        ));
    }
    Ok(viscosity)
}

pub fn evaluate_cross_wlf(model: &CrossWlf, points: &[ViscosityPoint]) -> Result<ResidualSummary> {
    if points.is_empty() {
        return Err(KairosError::validation(
            "Cross-WLF 残差评估至少需要一个数据点。",
        ));
    }
    let mut max_absolute_pa_s: f64 = 0.0;
    let mut sum_squared = 0.0;
    let mut max_absolute_log10: f64 = 0.0;
    let mut sum_squared_log10 = 0.0;
    for point in points {
        if !point.viscosity_pa_s.is_finite() || point.viscosity_pa_s <= 0.0 {
            return Err(KairosError::validation(
                "残差评估中的实测黏度必须为正有限数值。",
            ));
        }
        let predicted = cross_wlf_viscosity(model, point.temperature_k, point.shear_rate_per_s)?;
        let absolute = (predicted - point.viscosity_pa_s).abs();
        let log10_error = (predicted.log10() - point.viscosity_pa_s.log10()).abs();
        max_absolute_pa_s = max_absolute_pa_s.max(absolute);
        max_absolute_log10 = max_absolute_log10.max(log10_error);
        sum_squared += absolute * absolute;
        sum_squared_log10 += log10_error * log10_error;
    }
    let count = points.len();
    Ok(ResidualSummary {
        count,
        max_absolute_pa_s,
        rmse_pa_s: (sum_squared / count as f64).sqrt(),
        max_absolute_log10,
        rmse_log10: (sum_squared_log10 / count as f64).sqrt(),
    })
}

const PVT_HEADER: [&str; 6] = [
    "pressure",
    "pressureUnit",
    "temperature",
    "temperatureUnit",
    "specificVolume",
    "specificVolumeUnit",
];
const VISCOSITY_HEADER: [&str; 6] = [
    "temperature",
    "temperatureUnit",
    "shearRate",
    "shearRateUnit",
    "viscosity",
    "viscosityUnit",
];

fn header(reader: &mut csv::Reader<&[u8]>, expected: &[&str]) -> Result<()> {
    let actual = reader.headers().ok().cloned().unwrap_or_default();
    if actual.iter().collect::<Vec<_>>() != expected {
        return Err(KairosError::validation(format!(
            "材料曲线 CSV 表头不匹配，期望列：{}。",
            expected.join(",")
        )));
    }
    Ok(())
}

fn number(row: &StringRecord, index: usize, row_no: usize, label: &str) -> Result<f64> {
    let value = row[index].trim().parse::<f64>().map_err(|error| {
        KairosError::validation(format!("第 {row_no} 行 {label} 不是数字：{error}"))
    })?;
    if !value.is_finite() {
        return Err(KairosError::validation(format!(
            "第 {row_no} 行 {label} 必须是有限数值。"
        )));
    }
    Ok(value)
}

fn positive(value: f64, row_no: usize, label: &str) -> Result<f64> {
    if value <= 0.0 {
        return Err(KairosError::validation(format!(
            "第 {row_no} 行 {label} 必须为正数。"
        )));
    }
    Ok(value)
}

fn temperature_kelvin(value: f64, unit: &str, row_no: usize) -> Result<f64> {
    let kelvin = match unit.trim().to_ascii_lowercase().as_str() {
        "k" | "kelvin" => value,
        "c" | "°c" | "degc" | "celsius" => value + 273.15,
        _ => {
            return Err(KairosError::validation(format!(
                "第 {row_no} 行温度单位不支持：{unit}。"
            )));
        }
    };
    if !kelvin.is_finite() || kelvin <= 0.0 {
        return Err(KairosError::validation(format!(
            "第 {row_no} 行温度换算后必须为正数。"
        )));
    }
    Ok(kelvin)
}

fn pressure_pa(value: f64, unit: &str, row_no: usize) -> Result<f64> {
    let factor = match unit.trim().to_ascii_lowercase().as_str() {
        "pa" => 1.0,
        "kpa" => 1_000.0,
        "mpa" => 1_000_000.0,
        _ => {
            return Err(KairosError::validation(format!(
                "第 {row_no} 行压力单位不支持：{unit}。"
            )));
        }
    };
    positive(value * factor, row_no, "压力")
}

fn specific_volume(value: f64, unit: &str, row_no: usize) -> Result<f64> {
    let factor = match unit.trim().to_ascii_lowercase().as_str() {
        "m3/kg" | "m^3/kg" => 1.0,
        "cm3/g" | "cm^3/g" => 1.0e-3,
        _ => {
            return Err(KairosError::validation(format!(
                "第 {row_no} 行比容单位不支持：{unit}。"
            )));
        }
    };
    positive(value * factor, row_no, "比容")
}

fn shear_rate(value: f64, unit: &str, row_no: usize) -> Result<f64> {
    if !matches!(
        unit.trim().to_ascii_lowercase().as_str(),
        "1/s" | "s^-1" | "s-1"
    ) {
        return Err(KairosError::validation(format!(
            "第 {row_no} 行剪切速率单位不支持：{unit}。"
        )));
    }
    positive(value, row_no, "剪切速率")
}

fn viscosity(value: f64, unit: &str, row_no: usize) -> Result<f64> {
    let factor = match unit.trim().to_ascii_lowercase().as_str() {
        "pa.s" | "pa·s" | "pas" => 1.0,
        "mpa.s" | "mpa·s" | "mpas" => 1.0e-3,
        _ => {
            return Err(KairosError::validation(format!(
                "第 {row_no} 行黏度单位不支持：{unit}。"
            )));
        }
    };
    positive(value * factor, row_no, "黏度")
}

pub fn parse_pvt_csv(content: &str) -> Result<Vec<PvtPoint>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(content.as_bytes());
    header(&mut reader, &PVT_HEADER)?;
    let mut points = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let row = record.unwrap_or_default();
        if row.len() != PVT_HEADER.len() {
            return Err(KairosError::validation(format!(
                "材料曲线第 {} 行列数不正确。",
                index + 2
            )));
        }
        let row_no = index + 2;
        let pressure = number(&row, 0, row_no, "压力")?;
        let temperature = number(&row, 2, row_no, "温度")?;
        let specific_volume_value = number(&row, 4, row_no, "比容")?;
        points.push(PvtPoint {
            pressure_pa: pressure_pa(pressure, &row[1], row_no)?,
            temperature_k: temperature_kelvin(temperature, &row[3], row_no)?,
            specific_volume_m3_per_kg: specific_volume(specific_volume_value, &row[5], row_no)?,
        });
    }
    validate_pvt(&mut points)?;
    Ok(points)
}

pub fn parse_viscosity_csv(content: &str) -> Result<Vec<ViscosityPoint>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(content.as_bytes());
    header(&mut reader, &VISCOSITY_HEADER)?;
    let mut points = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let row = record.unwrap_or_default();
        if row.len() != VISCOSITY_HEADER.len() {
            return Err(KairosError::validation(format!(
                "材料曲线第 {} 行列数不正确。",
                index + 2
            )));
        }
        let row_no = index + 2;
        let temperature = number(&row, 0, row_no, "温度")?;
        let shear = number(&row, 2, row_no, "剪切速率")?;
        let viscosity_value = number(&row, 4, row_no, "黏度")?;
        points.push(ViscosityPoint {
            temperature_k: temperature_kelvin(temperature, &row[1], row_no)?,
            shear_rate_per_s: shear_rate(shear, &row[3], row_no)?,
            viscosity_pa_s: viscosity(viscosity_value, &row[5], row_no)?,
        });
    }
    validate_viscosity(&mut points)?;
    Ok(points)
}

fn validate_pvt(points: &mut [PvtPoint]) -> Result<()> {
    if points.is_empty() {
        return Err(KairosError::validation("PVT 曲线至少需要一个数据点。"));
    }
    points.sort_by(|left, right| {
        left.pressure_pa
            .total_cmp(&right.pressure_pa)
            .then(left.temperature_k.total_cmp(&right.temperature_k))
    });
    for pair in points.windows(2) {
        if pair[0].pressure_pa == pair[1].pressure_pa
            && pair[0].temperature_k == pair[1].temperature_k
        {
            return Err(KairosError::validation("PVT 曲线包含重复的压力/温度点。"));
        }
    }
    Ok(())
}

fn validate_viscosity(points: &mut [ViscosityPoint]) -> Result<()> {
    if points.is_empty() {
        return Err(KairosError::validation("黏度曲线至少需要一个数据点。"));
    }
    points.sort_by(|left, right| {
        left.temperature_k
            .total_cmp(&right.temperature_k)
            .then(left.shear_rate_per_s.total_cmp(&right.shear_rate_per_s))
    });
    match points.windows(2).find(|pair| {
        pair[0].temperature_k == pair[1].temperature_k
            && pair[0].shear_rate_per_s == pair[1].shear_rate_per_s
    }) {
        Some(_) => Err(KairosError::validation(
            "黏度曲线包含重复的温度/剪切速率点。",
        )),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes_pvt_units_and_order() {
        let csv = "pressure,pressureUnit,temperature,temperatureUnit,specificVolume,specificVolumeUnit\n50,MPa,100,C,1.2,cm3/g\n1,MPa,25,C,1.4,cm3/g\n";
        let points = parse_pvt_csv(csv).expect("valid pvt");
        assert_eq!(points[0].pressure_pa, 1_000_000.0);
        assert!((points[0].temperature_k - 298.15).abs() < 1e-10);
        assert!((points[0].specific_volume_m3_per_kg - 0.0014).abs() < 1e-12);
    }

    #[test]
    fn parses_viscosity_and_normalizes_mpas() {
        let csv = "temperature,temperatureUnit,shearRate,shearRateUnit,viscosity,viscosityUnit\n220,C,10,1/s,100,mPa.s\n";
        let points = parse_viscosity_csv(csv).expect("valid viscosity");
        assert!((points[0].temperature_k - 493.15).abs() < 1e-10);
        assert_eq!(points[0].shear_rate_per_s, 10.0);
        assert!((points[0].viscosity_pa_s - 0.1).abs() < 1e-12);
    }

    #[test]
    fn rejects_empty_and_duplicate_curves() {
        let empty =
            "pressure,pressureUnit,temperature,temperatureUnit,specificVolume,specificVolumeUnit\n";
        assert!(parse_pvt_csv(empty).is_err());
        let duplicate = "temperature,temperatureUnit,shearRate,shearRateUnit,viscosity,viscosityUnit\n220,C,10,1/s,100,Pa.s\n220,C,10,1/s,90,Pa.s\n";
        assert!(parse_viscosity_csv(duplicate).is_err());
    }

    #[test]
    fn rejects_unknown_units_and_non_positive_values() {
        let bad_unit = "pressure,pressureUnit,temperature,temperatureUnit,specificVolume,specificVolumeUnit\n1,bar,25,C,1,cm3/g\n";
        assert!(parse_pvt_csv(bad_unit).is_err());
        let bad_value = "temperature,temperatureUnit,shearRate,shearRateUnit,viscosity,viscosityUnit\n220,C,0,1/s,100,Pa.s\n";
        assert!(parse_viscosity_csv(bad_value).is_err());
    }

    #[test]
    fn exercises_unit_aliases_and_validation_errors() {
        assert_eq!(temperature_kelvin(300.0, "K", 1).unwrap(), 300.0);
        assert!(temperature_kelvin(1.0, "rankine", 1).is_err());
        assert!(temperature_kelvin(-300.0, "K", 1).is_err());
        assert_eq!(pressure_pa(1.0, "Pa", 1).unwrap(), 1.0);
        assert_eq!(pressure_pa(1.0, "kPa", 1).unwrap(), 1_000.0);
        assert!(pressure_pa(1.0, "bar", 1).is_err());
        assert_eq!(specific_volume(1.0, "m^3/kg", 1).unwrap(), 1.0);
        assert!(specific_volume(1.0, "cm", 1).is_err());
        assert!(shear_rate(1.0, "rpm", 1).is_err());
        assert_eq!(shear_rate(1.0, "s^-1", 1).unwrap(), 1.0);
        assert_eq!(viscosity(1.0, "Pa·s", 1).unwrap(), 1.0);
        assert_eq!(viscosity(1.0, "mPa·s", 1).unwrap(), 0.001);
        assert!(viscosity(1.0, "cP", 1).is_err());
        assert!(positive(0.0, 1, "x").is_err());
    }

    #[test]
    fn rejects_bad_headers_rows_numbers_and_csv_syntax() {
        assert!(parse_pvt_csv("wrong\n").is_err());
        let short = "pressure,pressureUnit,temperature,temperatureUnit,specificVolume,specificVolumeUnit\n1,MPa,25,C\n";
        assert!(parse_pvt_csv(short).is_err());
        let short_viscosity = "temperature,temperatureUnit,shearRate,shearRateUnit,viscosity,viscosityUnit\n220,C,1,1/s,\n";
        assert!(parse_viscosity_csv(short_viscosity).is_err());
        let bad_number = "pressure,pressureUnit,temperature,temperatureUnit,specificVolume,specificVolumeUnit\nno,MPa,25,C,1,cm3/g\n";
        assert!(parse_pvt_csv(bad_number).is_err());
        let bad_volume = "pressure,pressureUnit,temperature,temperatureUnit,specificVolume,specificVolumeUnit\n1,MPa,25,C,no,cm3/g\n";
        assert!(parse_pvt_csv(bad_volume).is_err());
        let non_finite = "temperature,temperatureUnit,shearRate,shearRateUnit,viscosity,viscosityUnit\nNaN,C,1,1/s,1,Pa.s\n";
        assert!(parse_viscosity_csv(non_finite).is_err());
        let malformed_pvt = "pressure,pressureUnit,temperature,temperatureUnit,specificVolume,specificVolumeUnit\n1\"bad,MPa,25,C,1,cm3/g\n";
        assert!(parse_pvt_csv(malformed_pvt).is_err());
        let malformed_viscosity = "temperature,temperatureUnit,shearRate,shearRateUnit,viscosity,viscosityUnit\n220\"bad,C,1,1/s,1,Pa.s\n";
        assert!(parse_viscosity_csv(malformed_viscosity).is_err());
    }

    #[test]
    fn accepts_all_documented_unit_aliases() {
        assert_eq!(temperature_kelvin(1.0, "degC", 1).unwrap(), 274.15);
        assert_eq!(temperature_kelvin(1.0, "celsius", 1).unwrap(), 274.15);
        assert_eq!(temperature_kelvin(1.0, "°C", 1).unwrap(), 274.15);
        assert_eq!(specific_volume(1.0, "cm^3/g", 1).unwrap(), 0.001);
        assert_eq!(shear_rate(1.0, "s-1", 1).unwrap(), 1.0);
        assert_eq!(viscosity(1.0, "Pa.s", 1).unwrap(), 1.0);
        assert_eq!(viscosity(1.0, "pas", 1).unwrap(), 1.0);
        assert_eq!(viscosity(1.0, "mpas", 1).unwrap(), 0.001);
    }

    #[test]
    fn validates_direct_curve_shapes() {
        assert!(validate_pvt(&mut []).is_err());
        assert!(validate_viscosity(&mut []).is_err());
        let mut duplicate_pvt = vec![
            PvtPoint {
                pressure_pa: 1.0,
                temperature_k: 300.0,
                specific_volume_m3_per_kg: 1.0,
            },
            PvtPoint {
                pressure_pa: 1.0,
                temperature_k: 300.0,
                specific_volume_m3_per_kg: 1.1,
            },
        ];
        assert!(validate_pvt(&mut duplicate_pvt).is_err());
        let mut duplicate_viscosity = vec![
            ViscosityPoint {
                temperature_k: 300.0,
                shear_rate_per_s: 1.0,
                viscosity_pa_s: 1.0,
            },
            ViscosityPoint {
                temperature_k: 300.0,
                shear_rate_per_s: 1.0,
                viscosity_pa_s: 2.0,
            },
        ];
        assert!(validate_viscosity(&mut duplicate_viscosity).is_err());
    }

    #[test]
    fn cross_wlf_prediction_and_residual_summary_are_deterministic() {
        let model = CrossWlf {
            n: 0.3,
            tau_star: 10_000.0,
            d1: 1_000.0,
            d2: 263.15,
            d3: 0.0,
            a1: 30.0,
            a2: 50.0,
        };
        let predicted = cross_wlf_viscosity(&model, 493.15, 10.0).unwrap();
        assert!(predicted.is_finite() && predicted > 0.0);
        let points = vec![ViscosityPoint {
            temperature_k: 493.15,
            shear_rate_per_s: 10.0,
            viscosity_pa_s: predicted,
        }];
        let summary = evaluate_cross_wlf(&model, &points).unwrap();
        assert_eq!(summary.count, 1);
        assert!(summary.max_absolute_pa_s.abs() < 1e-12);
        assert!(summary.rmse_log10.abs() < 1e-12);
    }

    #[test]
    fn cross_wlf_rejects_invalid_inputs_and_empty_residuals() {
        let model = CrossWlf {
            n: 0.3,
            tau_star: 1.0,
            d1: 1.0,
            d2: 1.0,
            d3: 0.0,
            a1: 1.0,
            a2: -1.0,
        };
        assert!(cross_wlf_viscosity(&model, 0.0, 1.0).is_err());
        assert!(cross_wlf_viscosity(&model, 2.0, 0.0).is_err());
        assert!(cross_wlf_viscosity(&model, 2.0, 1.0).is_err());
        let mut non_finite_zero_shear = model.clone();
        non_finite_zero_shear.d1 = f64::INFINITY;
        assert!(cross_wlf_viscosity(&non_finite_zero_shear, 3.0, 1.0).is_err());
        let mut non_finite_result = model.clone();
        non_finite_result.tau_star = -1.0;
        assert!(cross_wlf_viscosity(&non_finite_result, 3.0, 1.0).is_err());
        assert!(evaluate_cross_wlf(&model, &[]).is_err());
        let invalid_point = [ViscosityPoint {
            temperature_k: 300.0,
            shear_rate_per_s: 1.0,
            viscosity_pa_s: f64::NAN,
        }];
        assert!(evaluate_cross_wlf(&model, &invalid_point).is_err());
    }
}
