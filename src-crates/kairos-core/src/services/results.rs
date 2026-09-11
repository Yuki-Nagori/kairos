//! 结果服务：扫描 OpenFOAM 时间目录、解析 internalField、不完整结果容错。

use std::fs;
use std::path::Path;

use crate::error::{KairosError, Result};
use crate::models::results::{ResultCatalog, ScalarField, TimeStepMeta};

/// 已知场名 → 是否为矢量场（模量读取）。
fn is_vector_field(field: &str) -> bool {
    matches!(field, "U" | "V" | "gradU")
}

/// 判断目录名是否为时间目录（可解析为非负有限浮点数）。
fn parse_time_dir_name(name: &str) -> Option<f64> {
    let value = name.parse::<f64>().ok()?;
    (value >= 0.0 && value.is_finite()).then_some(value)
}

/// 扫描 case 目录下的时间步与场文件；解析失败的时间步跳过（不完整结果容错）。
pub fn scan_times(case_dir: &Path) -> Result<ResultCatalog> {
    if !case_dir.exists() {
        return Err(KairosError::not_found(format!(
            "结果目录不存在：{}",
            case_dir.display()
        )));
    }
    let mut times: Vec<TimeStepMeta> = Vec::new();
    let entries =
        fs::read_dir(case_dir).map_err(|e| KairosError::io(format!("读取结果目录失败：{e}")))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // 非 UTF-8 目录名与非时间目录名一并不解析（链式 None 合流到同一跳过分支）。
        let Some((time_s, dir_name)) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| parse_time_dir_name(name).map(|time_s| (time_s, name)))
        else {
            continue;
        };
        // read_dir 失败（极罕见）等价于无字段文件：两层 flatten 剥掉 Result 与目录迭代器。
        let mut fields: Vec<String> = fs::read_dir(&path)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_str()?.to_string();
                entry.path().is_file().then_some(name)
            })
            .collect();
        fields.sort();
        times.push(TimeStepMeta {
            dir_name: dir_name.to_string(),
            time_s,
            fields,
        });
    }
    times.sort_by(|a, b| a.time_s.total_cmp(&b.time_s));
    Ok(ResultCatalog {
        case_dir: case_dir.to_string_lossy().to_string(),
        times,
    })
}

/// 提取 `internalField` 之后的内容到配平的列表右括号（uniform 则到行尾分号）。
fn extract_internal_field(content: &str) -> Option<&str> {
    let start = content.find("internalField")? + "internalField".len();
    let rest = content[start..].trim_start();
    if rest.starts_with("uniform") {
        let end = rest.find(';')?;
        return Some(&rest[..=end]);
    }
    let open = rest.find('(')?;
    let bytes = rest.as_bytes();
    let mut depth = 0usize;
    let mut close = None;
    for (offset, &byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(offset);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    Some(rest[..=close].trim_end())
}

/// 解析 internalField 为标量值列表：uniform 与 nonuniform 均支持。
/// 声明数量与实际不符时返回 (实际值, false)——求解中途取消的不完整结果照读但标注。
pub fn parse_internal_scalar(content: &str) -> (Vec<f64>, bool) {
    let Some(internal) = extract_internal_field(content) else {
        return (Vec::new(), false);
    };
    let trimmed = internal.trim_start();
    if let Some(rest) = trimmed.strip_prefix("uniform") {
        let value: f64 = rest
            .trim()
            .trim_end_matches(';')
            .trim()
            .parse()
            .unwrap_or(f64::NAN);
        return (vec![value], value.is_finite());
    }
    // 列表形式：数值位于首个左括号之后。非 uniform 时 extract_internal_field
    // 已保证存在配对括号（否则返回 None），故这里必然是 Some。
    let open = trimmed
        .find('(')
        .expect("extract_internal_field 已保证非 uniform 形式含左括号");
    let declared = declared_count(trimmed[..open].trim_start());
    let numbers: Vec<f64> = trimmed[open..]
        .split(['(', ')', ';', ','])
        .flat_map(|part| part.split_whitespace())
        .filter_map(|token| token.parse::<f64>().ok())
        .collect();
    let complete = declared.is_none_or(|count| numbers.len() >= count);
    (numbers, complete)
}

/// 从 internalField 头部提取「声明的元素数量」，无则 None。
fn declared_count(internal: &str) -> Option<usize> {
    internal
        .split(['(', ')'])
        .next()?
        .split_whitespace()
        .find_map(|token| token.parse::<usize>().ok())
}

/// 解析 internalField 为矢量模量列表（每 3 个分量一组）。
pub fn parse_internal_vector_magnitudes(content: &str) -> (Vec<f64>, bool) {
    let (values, complete) = parse_internal_scalar(content);
    if values.len() % 3 != 0 {
        // 分量数不是 3 的倍数：不完整的矢量数据，按可用分量截断后取模量
        let usable = values.len() - values.len() % 3;
        let magnitudes: Vec<f64> = values[..usable]
            .chunks(3)
            .map(|group| group.iter().map(|v| v * v).sum::<f64>().sqrt())
            .collect();
        return (magnitudes, false);
    }
    let magnitudes: Vec<f64> = values
        .chunks(3)
        .map(|group| group.iter().map(|v| v * v).sum::<f64>().sqrt())
        .collect();
    (magnitudes, complete)
}

/// 读取指定时间步的场文件：标量场直读，矢量场返回模量。
pub fn read_field(case_dir: &Path, time_dir: &str, field: &str) -> Result<ScalarField> {
    let path = case_dir.join(time_dir).join(field);
    let content =
        fs::read_to_string(&path).map_err(|e| KairosError::io(format!("读取场文件失败：{e}")))?;
    let time_s = parse_time_dir_name(time_dir)
        .ok_or_else(|| KairosError::validation(format!("时间目录名无法解析：{time_dir}")))?;
    let (values, complete) = if is_vector_field(field) {
        parse_internal_vector_magnitudes(&content)
    } else {
        parse_internal_scalar(&content)
    };
    Ok(ScalarField {
        field: field.to_string(),
        time_dir: time_dir.to_string(),
        time_s,
        values,
        is_magnitude: is_vector_field(field),
        complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCALAR_UNIFORM: &str = r#"FoamFile
{
    version 2.0;
    format ascii;
    class "volScalarField";
    object T;
}
dimensions [0 0 0 0 0 1 0];
internalField uniform 300;
boundaryField
{
    walls { type zeroGradient; }
}
"#;

    const SCALAR_NONUNIFORM: &str = r#"FoamFile
{
    version 2.0;
    format ascii;
    class "volScalarField";
    object T;
}
dimensions [0 0 0 0 0 1 0];
internalField nonuniform List<scalar>
4
(
300
301.5
303
305.25
)
;
boundaryField
{
    walls { type zeroGradient; }
}
"#;

    const VECTOR_NONUNIFORM: &str = r#"FoamFile
{
    version 2.0;
    format ascii;
    class "volVectorField";
    object U;
}
dimensions [0 1 -1 0 0 0 0];
internalField nonuniform List<vector>
2
(
(3 4 0)
(1 0 0)
)
;
boundaryField
{
    walls { type noSlip; }
}
"#;

    #[test]
    fn parses_uniform_scalar() {
        let (values, complete) = parse_internal_scalar(SCALAR_UNIFORM);
        assert_eq!(values, vec![300.0]);
        assert!(complete);
    }

    #[test]
    fn parses_nonuniform_scalars() {
        let (values, complete) = parse_internal_scalar(SCALAR_NONUNIFORM);
        assert_eq!(values, vec![300.0, 301.5, 303.0, 305.25]);
        assert!(complete);
    }

    #[test]
    fn parses_vector_magnitudes() {
        let (values, complete) = parse_internal_vector_magnitudes(VECTOR_NONUNIFORM);
        assert_eq!(values, vec![5.0, 1.0]);
        assert!(complete);
    }

    #[test]
    fn truncated_list_is_flagged_incomplete() {
        let truncated = SCALAR_NONUNIFORM.replace("305.25\n", "");
        let (values, complete) = parse_internal_scalar(&truncated);
        assert_eq!(values, vec![300.0, 301.5, 303.0]);
        assert!(!complete);
    }

    #[test]
    fn scan_times_lists_and_sorts() {
        let dir = std::env::temp_dir().join(format!("kairos-t13-{}", std::process::id()));
        for (name, content) in [
            ("0", SCALAR_UNIFORM),
            ("0.5", SCALAR_NONUNIFORM),
            ("constant", SCALAR_UNIFORM),
        ] {
            let time_dir = dir.join(name);
            fs::create_dir_all(&time_dir).unwrap();
            fs::write(time_dir.join("T"), content).unwrap();
        }
        let catalog = scan_times(&dir).unwrap();
        assert_eq!(catalog.times.len(), 2, "constant 目录应被排除");
        assert_eq!(catalog.times[0].dir_name, "0");
        assert_eq!(catalog.times[1].dir_name, "0.5");
        assert!(catalog.times[0].fields.contains(&"T".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_times_rejects_unreadable_case_dir() {
        // case 路径存在但不是目录：read_dir 失败走 io 错误分支。
        let path = std::env::temp_dir().join(format!("kairos-t13f-{}", std::process::id()));
        fs::write(&path, "不是目录").unwrap();
        let error = scan_times(&path).unwrap_err();
        assert!(error.to_string().contains("读取结果目录失败"));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn read_field_reports_missing_file_and_bad_time_dir() {
        let dir = std::env::temp_dir().join(format!("kairos-t13g-{}", std::process::id()));
        let time_dir = dir.join("abc"); // 非时间目录名
        fs::create_dir_all(&time_dir).unwrap();
        // 场文件缺失：io 失败先于目录名解析报出。
        let missing = read_field(&dir, "1.0", "T").unwrap_err();
        assert!(missing.to_string().contains("读取场文件失败"));
        // 目录名无法解析为时间步。
        fs::write(time_dir.join("T"), SCALAR_UNIFORM).unwrap();
        let error = read_field(&dir, "abc", "T").unwrap_err();
        assert!(error.to_string().contains("时间目录名无法解析"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_field_returns_scalar_and_magnitude() {
        let dir = std::env::temp_dir().join(format!("kairos-t13b-{}", std::process::id()));
        fs::create_dir_all(dir.join("0.5")).unwrap();
        fs::write(dir.join("0.5").join("T"), SCALAR_NONUNIFORM).unwrap();
        fs::write(dir.join("0.5").join("U"), VECTOR_NONUNIFORM).unwrap();

        let t = read_field(&dir, "0.5", "T").unwrap();
        assert_eq!(t.values.len(), 4);
        assert!(!t.is_magnitude);
        assert!(t.complete);

        let u = read_field(&dir, "0.5", "U").unwrap();
        assert!(u.is_magnitude);
        assert!((u.values[0] - 5.0).abs() < 1e-12);
        fs::remove_dir_all(&dir).ok();
    }
    #[test]
    fn scan_times_skips_plain_files() {
        let dir = std::env::temp_dir().join(format!("kairos-t13file-{}", std::process::id()));
        let time_dir = dir.join("0");
        fs::create_dir_all(&time_dir).unwrap();
        fs::write(time_dir.join("T"), SCALAR_UNIFORM).unwrap();
        fs::write(dir.join("README.md"), "不是时间步目录").unwrap();
        let catalog = scan_times(&dir).unwrap();
        assert_eq!(catalog.times.len(), 1);
        fs::remove_dir_all(&dir).ok();
    }
}

/// 二进制场编码的魔数与元数据结构（大体积数据走 IPC 原始字节通道）。
/// 二进制场编码 / 解码：[magic 4B][meta_len u32 LE][meta JSON][f64 值 × n LE]。
/// 大结果走原始字节而非 JSON 数组，体积与解析开销都显著降低。
pub mod field_binary {
    use crate::error::{KairosError, Result};
    use crate::models::results::ScalarField;

    const MAGIC: [u8; 4] = *b"KF1\x00";

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Meta {
        field: String,
        time_dir: String,
        time_s: f64,
        is_magnitude: bool,
        complete: bool,
        count: usize,
    }

    pub fn encode(field: &ScalarField) -> Vec<u8> {
        let meta = serde_json::to_vec(&Meta {
            field: field.field.clone(),
            time_dir: field.time_dir.clone(),
            time_s: field.time_s,
            is_magnitude: field.is_magnitude,
            complete: field.complete,
            count: field.values.len(),
        })
        .expect("元数据为纯标量结构，序列化不会失败");
        let mut bytes = Vec::with_capacity(8 + meta.len() + field.values.len() * 8);
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&(meta.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&meta);
        for value in &field.values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<ScalarField> {
        let invalid =
            |reason: &str| KairosError::validation(format!("二进制场数据无效：{reason}。"));
        if bytes.len() < 8 || bytes[..4] != MAGIC {
            return Err(invalid("魔数不匹配"));
        }
        let meta_len = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        // 先比较后加法：避免 32 位平台上 8 + meta_len 溢出。
        if meta_len > bytes.len() - 8 {
            return Err(invalid("元数据被截断"));
        }
        let meta_end = 8 + meta_len;
        let meta: Meta = serde_json::from_slice(&bytes[8..meta_end])
            .map_err(|e| invalid(&format!("元数据解析失败（{e}）")))?;
        let values_bytes = bytes.len() - meta_end;
        if values_bytes < meta.count * 8 {
            return Err(invalid("值区被截断"));
        }
        let mut values = Vec::with_capacity(meta.count);
        for index in 0..meta.count {
            let start = meta_end + index * 8;
            // 值区数量已在上方与字节长度核对，切片必然完整。
            let mut raw = [0u8; 8];
            raw.copy_from_slice(&bytes[start..start + 8]);
            values.push(f64::from_le_bytes(raw));
        }
        Ok(ScalarField {
            field: meta.field,
            time_dir: meta.time_dir,
            time_s: meta.time_s,
            values,
            is_magnitude: meta.is_magnitude,
            complete: meta.complete,
        })
    }
}

/// 有界场缓存（FIFO 淘汰）：命中时跳过磁盘读取，容量上限防御内存膨胀。
pub struct FieldCache {
    capacity: usize,
    entries: std::collections::HashMap<String, ScalarField>,
    order: Vec<String>,
}

impl FieldCache {
    /// 容量 1..=64。
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity > 64 {
            return Err(KairosError::validation("场缓存容量必须在 1..=64 之间。"));
        }
        Ok(Self {
            capacity,
            entries: std::collections::HashMap::new(),
            order: Vec::new(),
        })
    }

    pub fn get(&mut self, key: &str) -> Option<&ScalarField> {
        self.entries.get(key)
    }

    /// 命中计数 0 的最旧条目淘汰：插入时若超容量则移除队首。
    pub fn put(&mut self, key: String, field: ScalarField) {
        if !self.entries.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.entries.insert(key, field);
        while self.order.len() > self.capacity {
            let oldest = self.order.remove(0);
            self.entries.remove(&oldest);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 派生场：按请求对主场做归一化 / 阈值掩码 / 线性映射；差值走 derive_difference。
pub fn derive_scalar_field(
    field: &crate::models::results::ScalarField,
    request: &crate::models::results::DeriveRequest,
) -> Result<crate::models::results::ScalarField> {
    use crate::error::KairosError;
    use crate::models::results::{DeriveRequest, ScalarField};

    if field.values.is_empty() {
        // 空场原样返回：上层保持名称与状态不变
        return Ok(field.clone());
    }
    let min = field.values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = field
        .values
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    let (suffix, derived): (String, Vec<f64>) = match request {
        DeriveRequest::Normalize => {
            let values = if range > 0.0 {
                field.values.iter().map(|v| (v - min) / range).collect()
            } else {
                vec![0.0; field.values.len()]
            };
            ("归一化".into(), values)
        }
        DeriveRequest::Threshold => {
            let threshold = (min + max) / 2.0;
            let values = field
                .values
                .iter()
                .map(|v| if *v >= threshold { 1.0 } else { 0.0 })
                .collect();
            ("阈值掩码".into(), values)
        }
        DeriveRequest::Linear { scale, offset } => (
            format!("线性映射 ×{scale} {offset:+}"),
            field.values.iter().map(|v| v * scale + offset).collect(),
        ),
        // 差值需要主场与对比场两份数据，单场入口不受理。
        DeriveRequest::Difference => {
            return Err(KairosError::validation(
                "两场差值请使用 derive_difference 命令（需要主场与对比场）。",
            ));
        }
    };

    Ok(ScalarField {
        field: format!("{} · {suffix}", field.field),
        time_dir: field.time_dir.clone(),
        time_s: field.time_s,
        values: derived,
        is_magnitude: false,
        complete: field.complete,
    })
}

/// 两场差值：主场 − 对比场，逐值相减。长度不一致时报验证错误（不静默截断）；
/// 结果继承主场的时间步与完整性标记，命名记为「主场 - 对比场」。
pub fn derive_difference(
    primary: &crate::models::results::ScalarField,
    compare: &crate::models::results::ScalarField,
) -> Result<crate::models::results::ScalarField> {
    use crate::error::KairosError;
    use crate::models::results::ScalarField;

    if primary.values.len() != compare.values.len() {
        return Err(KairosError::validation(format!(
            "两场长度不一致：{} 有 {} 个值，{} 有 {} 个值。",
            primary.field,
            primary.values.len(),
            compare.field,
            compare.values.len()
        )));
    }
    let values = primary
        .values
        .iter()
        .zip(compare.values.iter())
        .map(|(a, b)| a - b)
        .collect();
    Ok(ScalarField {
        field: format!("{} - {}", primary.field, compare.field),
        time_dir: primary.time_dir.clone(),
        time_s: primary.time_s,
        values,
        is_magnitude: false,
        complete: primary.complete && compare.complete,
    })
}

#[cfg(test)]
mod derive_tests {
    use super::*;
    use crate::models::results::{DeriveRequest, ScalarField};

    fn field(values: Vec<f64>) -> ScalarField {
        ScalarField {
            field: "T".into(),
            time_dir: "0.001".into(),
            time_s: 0.001,
            values,
            is_magnitude: false,
            complete: true,
        }
    }

    #[test]
    fn normalize_maps_to_zero_one_and_renames() {
        let derived =
            derive_scalar_field(&field(vec![1.0, 2.0, 3.0]), &DeriveRequest::Normalize).unwrap();
        assert_eq!(derived.values, vec![0.0, 0.5, 1.0]);
        assert_eq!(derived.field, "T · 归一化");
        assert!(!derived.is_magnitude);
    }

    #[test]
    fn normalize_flat_field_maps_to_zeros() {
        let derived =
            derive_scalar_field(&field(vec![5.0, 5.0, 5.0]), &DeriveRequest::Normalize).unwrap();
        assert_eq!(derived.values, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn threshold_midpoint_inclusive() {
        let derived =
            derive_scalar_field(&field(vec![1.0, 3.0, 2.0]), &DeriveRequest::Threshold).unwrap();
        assert_eq!(derived.values, vec![0.0, 1.0, 1.0]);
        assert_eq!(derived.field, "T · 阈值掩码");
    }

    #[test]
    fn empty_field_returns_unchanged() {
        let derived = derive_scalar_field(&field(vec![]), &DeriveRequest::Normalize).unwrap();
        assert!(derived.values.is_empty());
        assert_eq!(derived.field, "T");
    }

    #[test]
    fn linear_map_applies_scale_and_offset_with_named_suffix() {
        let derived = derive_scalar_field(
            &field(vec![1.0, 2.0]),
            &DeriveRequest::Linear {
                scale: 2.0,
                offset: -1.0,
            },
        )
        .unwrap();
        assert_eq!(derived.values, vec![1.0, 3.0]);
        assert_eq!(derived.field, "T · 线性映射 ×2 -1");
    }

    #[test]
    fn difference_subtracts_compare_field_value_wise() {
        let primary = field(vec![3.0, 5.0]);
        let mut compare = field(vec![1.0, 2.0]);
        compare.field = "Tamb".into();
        let derived = derive_difference(&primary, &compare).unwrap();
        assert_eq!(derived.values, vec![2.0, 3.0]);
        assert_eq!(derived.field, "T - Tamb");
    }

    #[test]
    fn difference_marks_completeness_only_when_both_complete() {
        let primary = field(vec![1.0]);
        let mut compare = field(vec![1.0]);
        compare.complete = false;
        assert!(!derive_difference(&primary, &compare).unwrap().complete);
    }

    #[test]
    fn difference_rejects_length_mismatch() {
        let error = derive_difference(&field(vec![1.0]), &field(vec![1.0, 2.0])).unwrap_err();
        assert!(error.to_string().contains("长度不一致"));
    }

    #[test]
    fn difference_request_in_single_field_entry_is_rejected() {
        let error = derive_scalar_field(&field(vec![1.0]), &DeriveRequest::Difference).unwrap_err();
        assert!(error.to_string().contains("derive_difference"));
    }
}

#[cfg(test)]
mod field_chain_tests {
    use super::*;

    #[test]
    fn binary_round_trip_preserves_field() {
        let field = ScalarField {
            field: "T".into(),
            time_dir: "0.100".into(),
            time_s: 0.1,
            values: vec![1.5, -2.25, f64::INFINITY, 0.0],
            is_magnitude: true,
            complete: false,
        };
        let bytes = field_binary::encode(&field);
        assert_eq!(&bytes[..4], b"KF1\x00");
        let decoded = field_binary::decode(&bytes).unwrap();
        assert_eq!(decoded.field, "T");
        assert_eq!(decoded.time_dir, "0.100");
        assert_eq!(decoded.time_s, 0.1);
        assert_eq!(decoded.values, field.values);
        assert!(decoded.is_magnitude);
        assert!(!decoded.complete);
    }

    #[test]
    fn binary_decode_rejects_garbage() {
        // 长度不足（<8B）与魔数错误统一按「魔数不匹配」报错。
        let error = field_binary::decode(b"XXXX").unwrap_err();
        assert!(error.to_string().contains("魔数不匹配"));
        // 长度足够但魔数错误。
        let error = field_binary::decode(b"XXXX\0\0\0\0").unwrap_err();
        assert!(error.to_string().contains("魔数不匹配"));
        // 长度过短。
        assert!(field_binary::decode(b"KF1\x00").is_err());
        // 元数据被截断（截到值区以内，且不足 meta 声明长度）。
        let mut truncated = field_binary::encode(&ScalarField {
            field: "T".into(),
            time_dir: "0".into(),
            time_s: 0.0,
            values: vec![1.0, 2.0],
            is_magnitude: false,
            complete: true,
        });
        truncated.truncate(truncated.len() - 9);
        assert!(field_binary::decode(&truncated).is_err());
        // 元数据 JSON 损坏（值区一起被改写，但解码在元数据阶段即失败）。
        let mut broken_meta = field_binary::encode(&ScalarField {
            field: "T".into(),
            time_dir: "0".into(),
            time_s: 0.0,
            values: vec![1.0],
            is_magnitude: false,
            complete: true,
        });
        for byte in &mut broken_meta[8..] {
            *byte = 0x78;
        }
        let error = field_binary::decode(&broken_meta).unwrap_err();
        assert!(error.to_string().contains("元数据解析失败"));

        // 头 + 部分元数据：连值区起始都未到达 → 元数据被截断。
        let mut values_only = field_binary::encode(&ScalarField {
            field: "T".into(),
            time_dir: "0".into(),
            time_s: 0.0,
            values: vec![1.0, 2.0],
            is_magnitude: false,
            complete: true,
        });
        let meta_end = values_only.len() - 16;
        values_only.truncate(meta_end - 1);
        let error = field_binary::decode(&values_only).unwrap_err();
        assert!(error.to_string().contains("元数据被截断"));
    }

    #[test]
    fn cache_evicts_oldest_beyond_capacity() {
        let mut cache = FieldCache::new(2).unwrap();
        let field = |name: &str| ScalarField {
            field: name.into(),
            time_dir: "0".into(),
            time_s: 0.0,
            values: vec![],
            is_magnitude: false,
            complete: true,
        };
        cache.put("a".into(), field("a"));
        cache.put("b".into(), field("b"));
        assert!(!cache.is_empty());
        cache.put("c".into(), field("c"));
        // a 最旧被淘汰。
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
        assert!(FieldCache::new(0).is_err());
        assert!(FieldCache::new(65).is_err());
    }

    #[test]
    fn cache_put_same_key_does_not_duplicate_order() {
        let mut cache = FieldCache::new(2).unwrap();
        let field = |name: &str| ScalarField {
            field: name.into(),
            time_dir: "0".into(),
            time_s: 0.0,
            values: vec![],
            is_magnitude: false,
            complete: true,
        };
        cache.put("a".into(), field("a"));
        cache.put("a".into(), field("a-v2"));
        assert!(!cache.is_empty());
        assert_eq!(cache.get("a").unwrap().field, "a-v2");
    }
}
