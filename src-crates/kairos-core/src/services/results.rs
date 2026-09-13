//! 结果服务：扫描 OpenFOAM 时间目录、解析 internalField、不完整结果容错。

use std::fs;
use std::io::Read;
use std::path::Path;

use crate::error::{KairosError, Result};
use crate::models::results::{ResultCatalog, ScalarField, TensorField, TimeStepMeta, VectorField};

/// 场类型：由 FoamFile 头的 `class` 行判定（不靠场名猜——求解侧的场名会随
/// 契约扩展，如位移 `D`、等效应力 `sigmaEq`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// `volScalarField`：逐单元标量。
    Scalar,
    /// `volVectorField`：逐单元矢量，读取时取模量。
    VectorMagnitude,
    /// `volSymmTensorField`：逐单元对称张量，读取时给出模量与主方向。
    SymmTensor,
    /// 其它类型（如 `volTensorField` 的非对称张量）：不支持读取，明确报错。
    Unsupported,
}

/// 从场文件内容解析类型（`class "volVectorField";` 与不带引号的写法都接受）。
pub fn field_kind(content: &str) -> FieldKind {
    let class = content
        .lines()
        .find_map(|line| {
            let value = line
                .trim()
                .strip_prefix("class")?
                .trim()
                .trim_end_matches(';')
                .trim();
            Some(value.trim_matches('"').to_string())
        })
        .unwrap_or_default();
    match class.as_str() {
        "volScalarField" => FieldKind::Scalar,
        "volVectorField" => FieldKind::VectorMagnitude,
        "volSymmTensorField" => FieldKind::SymmTensor,
        _ => FieldKind::Unsupported,
    }
}

/// 判断目录名是否为时间目录（可解析为非负有限浮点数）。
fn parse_time_dir_name(name: &str) -> Option<f64> {
    let value = name.parse::<f64>().ok()?;
    (value >= 0.0 && value.is_finite()).then_some(value)
}

/// 从 `ls -d [0-9]*` 一类的目录清单里筛出时间目录名，按时间升序去重。
///
/// 用途：VM 内求解结束后把结果时间目录回传宿主（见 src-tauri 作业层），
/// 筛查与 `scan_times` 用同一套目录名判定，避免回传宿主无法识别的东西。
pub fn time_dir_names(listing: &str) -> Vec<String> {
    let mut names: Vec<(f64, String)> = listing
        .split_whitespace()
        .filter_map(|name| parse_time_dir_name(name).map(|time_s| (time_s, name.to_string())))
        .collect();
    names.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    names.dedup_by(|left, right| left.1 == right.1);
    names.into_iter().map(|(_, name)| name).collect()
}

/// 场文件头部的读取上限（字节）：`class` 行总在 FoamFile 头里，读前缀即可
/// 判定类型，不必为一次筛选把整套结果读进内存。
const FIELD_HEADER_BYTES: usize = 512;

/// 时间目录下的文件是否是可读场（按 FoamFile 头的 `class` 判定）。
///
/// 求解器会在时间目录里写非场对象（如阶段记录 `moldingStage`）：列进场清单
/// 后用户在结果面板一点就是「暂不支持该场类型」，不如不列。打不开 / 读不出
/// 头部的文件同样按不可读处理。
fn is_readable_field(path: &Path) -> bool {
    let mut header = [0u8; FIELD_HEADER_BYTES];
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let read = file.read(&mut header).unwrap_or(0);
    field_kind(&String::from_utf8_lossy(&header[..read])) != FieldKind::Unsupported
}

/// 扫描 case 目录下的时间步与场文件；解析失败的时间步跳过（不完整结果容错），
/// 非场对象（class 不受支持）不进 `fields` 清单。
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
                let path = entry.path();
                let name = entry.file_name().to_str()?.to_string();
                (path.is_file() && is_readable_field(&path)).then_some(name)
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

/// 解析 internalField 为矢量分量列表（每 3 个标量一组）。
/// 分量数不是 3 的倍数时按可用分量截断并把 complete 置 false。
pub fn parse_internal_vectors(content: &str) -> (Vec<[f64; 3]>, bool) {
    let (values, complete) = parse_internal_scalar(content);
    let usable = values.len() - values.len() % 3;
    let complete = complete && values.len() % 3 == 0;
    let vectors = values[..usable]
        .chunks(3)
        .map(|group| [group[0], group[1], group[2]])
        .collect();
    (vectors, complete)
}

/// 解析 internalField 为矢量模量列表（每 3 个分量一组）。
pub fn parse_internal_vector_magnitudes(content: &str) -> (Vec<f64>, bool) {
    let (vectors, complete) = parse_internal_vectors(content);
    let magnitudes = vectors
        .iter()
        .map(|group| (group[0] * group[0] + group[1] * group[1] + group[2] * group[2]).sqrt())
        .collect();
    (magnitudes, complete)
}

/// 解析 internalField 为对称张量分量（每 6 个标量一组，(xx, xy, xz, yy, yz, zz)）。
/// 分量数不是 6 的倍数时按可用分量截断并把 complete 置 false。
pub fn parse_internal_tensors(content: &str) -> (Vec<[f64; 6]>, bool) {
    let (values, complete) = parse_internal_scalar(content);
    let usable = values.len() - values.len() % 6;
    let complete = complete && values.len() % 6 == 0;
    let tensors = values[..usable]
        .chunks(6)
        .map(|group| [group[0], group[1], group[2], group[3], group[4], group[5]])
        .collect();
    (tensors, complete)
}

/// 对称张量的模量（与 OpenFOAM `mag(symmTensor)` 同口径）。
pub fn symm_tensor_magnitude(components: &[f64; 6]) -> f64 {
    let [xx, xy, xz, yy, yz, zz] = *components;
    (xx * xx + yy * yy + zz * zz + 2.0 * (xy * xy + xz * xz + yz * yz)).sqrt()
}

/// 对称张量的主方向（特征值绝对值最大者对应的单位特征向量）。
///
/// 用一次 Jacobi 旋转把 3×3 对称矩阵对角化（6 次旋回即可收敛到机器精度），
/// 再取 |λ| 最大的那一列。退化（全零 / 非有限）返回零向量，不产生 NaN。
pub fn principal_axis(components: &[f64; 6]) -> [f64; 3] {
    let [xx, xy, xz, yy, yz, zz] = *components;
    if ![xx, xy, xz, yy, yz, zz]
        .iter()
        .all(|value| value.is_finite())
    {
        return [0.0; 3];
    }
    let mut matrix = [[xx, xy, xz], [xy, yy, yz], [xz, yz, zz]];
    let mut vectors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..12 {
        // 取绝对值最大的非对角元
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
        // 旋转矩阵 R(p,q,θ)：A ← Rᵀ A R
        // 第一步按列混合（A·R）：每一行的第 p/q 列配对旋转。
        for row in &mut matrix {
            let (mkp, mkq) = (row[p], row[q]);
            row[p] = cos * mkp - sin * mkq;
            row[q] = sin * mkp + cos * mkq;
        }
        // 第二步按行混合（Rᵀ·A）：第 p/q 两行逐列配对旋转（p < q，split_at_mut 取两行）。
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
    // 旋转矩阵的列恒为单位向量（正交性），下限只是数值兜底；入口已排除非有限值。
    let axis = [vectors[0][best], vectors[1][best], vectors[2][best]];
    let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2])
        .sqrt()
        .max(1e-12);
    let unit = [axis[0] / norm, axis[1] / norm, axis[2] / norm];
    // 符号规范化：绝对值最大的分量取正——特征向量的整体符号本无物理意义，
    // 固定约定才能让「同一场两次读取」「不同后端」给出一致的方向。
    let dominant = if unit[1].abs() > unit[0].abs() { 1 } else { 0 };
    let dominant = if unit[2].abs() > unit[dominant].abs() {
        2
    } else {
        dominant
    };
    let sign = unit[dominant].signum();
    [unit[0] * sign, unit[1] * sign, unit[2] * sign]
}

/// 读取指定时间步的对称张量场（模量 + 主方向）。
pub fn read_tensor_field(case_dir: &Path, time_dir: &str, field: &str) -> Result<TensorField> {
    let path = case_dir.join(time_dir).join(field);
    let content =
        fs::read_to_string(&path).map_err(|e| KairosError::io(format!("读取场文件失败：{e}")))?;
    let time_s = parse_time_dir_name(time_dir)
        .ok_or_else(|| KairosError::validation(format!("时间目录名无法解析：{time_dir}")))?;
    if field_kind(&content) != FieldKind::SymmTensor {
        return Err(KairosError::validation(format!(
            "该场不是对称张量场：{field}（张量读取仅支持 volSymmTensorField）"
        )));
    }
    let (components, complete) = parse_internal_tensors(&content);
    let magnitudes = components
        .iter()
        .map(symm_tensor_magnitude)
        .collect::<Vec<_>>();
    let principal_axes = components.iter().map(principal_axis).collect::<Vec<_>>();
    Ok(TensorField {
        field: field.to_string(),
        time_dir: time_dir.to_string(),
        time_s,
        components,
        magnitudes,
        principal_axes,
        complete,
    })
}

/// 读取指定时间步的矢量场三分量（矢量场文件专用）。
pub fn read_vector_field(case_dir: &Path, time_dir: &str, field: &str) -> Result<VectorField> {
    let path = case_dir.join(time_dir).join(field);
    let content =
        fs::read_to_string(&path).map_err(|e| KairosError::io(format!("读取场文件失败：{e}")))?;
    let time_s = parse_time_dir_name(time_dir)
        .ok_or_else(|| KairosError::validation(format!("时间目录名无法解析：{time_dir}")))?;
    if field_kind(&content) != FieldKind::VectorMagnitude {
        return Err(KairosError::validation(format!(
            "该场不是矢量场：{field}（分量读取仅支持 volVectorField）"
        )));
    }
    let (components, complete) = parse_internal_vectors(&content);
    Ok(VectorField {
        field: field.to_string(),
        time_dir: time_dir.to_string(),
        time_s,
        components,
        complete,
    })
}

/// 读取指定时间步的场文件：标量场直读，矢量场返回模量。
pub fn read_field(case_dir: &Path, time_dir: &str, field: &str) -> Result<ScalarField> {
    let path = case_dir.join(time_dir).join(field);
    let content =
        fs::read_to_string(&path).map_err(|e| KairosError::io(format!("读取场文件失败：{e}")))?;
    let time_s = parse_time_dir_name(time_dir)
        .ok_or_else(|| KairosError::validation(format!("时间目录名无法解析：{time_dir}")))?;
    let (values, complete, is_magnitude) = match field_kind(&content) {
        FieldKind::Scalar => {
            let (values, complete) = parse_internal_scalar(&content);
            (values, complete, false)
        }
        FieldKind::VectorMagnitude => {
            let (values, complete) = parse_internal_vector_magnitudes(&content);
            (values, complete, true)
        }
        // 对称张量走标量通道时取模量（与矢量场取模量的口径一致）；
        // 需要主方向时用 read_tensor_field（张力三分量不在此丢失）。
        FieldKind::SymmTensor => {
            let (tensors, complete) = parse_internal_tensors(&content);
            let magnitudes = tensors.iter().map(symm_tensor_magnitude).collect();
            (magnitudes, complete, true)
        }
        FieldKind::Unsupported => {
            return Err(KairosError::validation(format!(
                "暂不支持该场类型：{field}（当前支持 volScalarField / volVectorField / volSymmTensorField）"
            )));
        }
    };
    Ok(ScalarField {
        field: field.to_string(),
        time_dir: time_dir.to_string(),
        time_s,
        values,
        is_magnitude,
        complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_dir_names_filters_and_sorts_listing() {
        // `ls -d [0-9]*` 的典型输出：混入 processor 目录与非数字名时只留时间目录，
        // 且按时间数值排序——字典序会把 10 排到 9 前面，数值序不会。
        let listing = "0  0.05  0.9  10  9  processor0  postProcessing  log.foamRun";
        assert_eq!(time_dir_names(listing), vec!["0", "0.05", "0.9", "9", "10"]);
        assert!(time_dir_names("").is_empty());
    }

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

    /// 位移场（D）：矢量场口径与 U 相同，模量可直接用于变形量展示。
    const DISPLACEMENT_NONUNIFORM: &str = r#"FoamFile
{
    version 2.0;
    format ascii;
    class "volVectorField";
    object D;
}
dimensions [0 1 0 0 0 0 0];
internalField nonuniform List<vector>
2
(
(0 0 0.3)
(0.4 0 0)
)
;
boundaryField
{
    walls { type fixedValue; value uniform (0 0 0); }
}
"#;

    #[test]
    fn field_kind_follows_file_class() {
        assert_eq!(field_kind(SCALAR_UNIFORM), FieldKind::Scalar);
        assert_eq!(field_kind(VECTOR_NONUNIFORM), FieldKind::VectorMagnitude);
        // 对称张量场（残余应力 sigma / 取向张量）单独成一类；非对称张量与缺 class 仍不支持
        assert_eq!(
            field_kind("FoamFile\n{\n    class \"volSymmTensorField\";\n}\n"),
            FieldKind::SymmTensor
        );
        assert_eq!(
            field_kind("FoamFile\n{\n    class \"volTensorField\";\n}\n"),
            FieldKind::Unsupported
        );
        assert_eq!(field_kind("not a foam file"), FieldKind::Unsupported);
    }

    #[test]
    fn tensor_field_reads_magnitude_and_rejects_nonsymmetric() {
        let dir = std::env::temp_dir().join(format!("kairos-tensor-{}", std::process::id()));
        fs::create_dir_all(dir.join("2")).unwrap();
        // 单轴应力：xx = 100，其余为 0 → 模量 100
        fs::write(
            dir.join("2").join("sigma"),
            "FoamFile\n{\n    class \"volSymmTensorField\";\n    object sigma;\n}\n\ninternalField   nonuniform List<symmTensor>\n1\n(\n(100 0 0 0 0 0)\n)\n;\n",
        )
        .unwrap();
        // 标量入口取模量
        let field = read_field(&dir, "2", "sigma").unwrap();
        assert!(field.is_magnitude);
        assert_eq!(field.values, vec![100.0]);
        // 张量入口给分量 / 模量 / 主轴
        let tensor = read_tensor_field(&dir, "2", "sigma").unwrap();
        assert_eq!(tensor.components, vec![[100.0, 0.0, 0.0, 0.0, 0.0, 0.0]]);
        assert_eq!(tensor.magnitudes, vec![100.0]);
        assert_eq!(tensor.principal_axes, vec![[1.0, 0.0, 0.0]]);

        // 非对称张量（volTensorField）仍明确拒绝
        fs::write(
            dir.join("2").join("gradU"),
            "FoamFile\n{\n    class \"volTensorField\";\n    object gradU;\n}\n\ninternalField   uniform (0 0 0 0 0 0 0 0 0);\n",
        )
        .unwrap();
        let error = read_field(&dir, "2", "gradU").unwrap_err();
        assert!(error.to_string().contains("暂不支持该场类型"), "{error}");
        // 张量入口拒绝标量场
        fs::write(dir.join("2").join("T"), SCALAR_NONUNIFORM).unwrap();
        assert!(
            read_tensor_field(&dir, "2", "T")
                .unwrap_err()
                .message()
                .contains("不是对称张量场")
        );
        // 文件缺失 → IO 错误；非数字时间目录（文件在）→ 目录名解析错误
        assert!(read_tensor_field(&dir, "2", "missing").is_err());
        fs::create_dir_all(dir.join("latest")).unwrap();
        fs::write(dir.join("latest").join("sigma"), "FoamFile\n{\n    class \"volSymmTensorField\";\n    object sigma;\n}\n\ninternalField   uniform (0 0 0 0 0 0);\n").unwrap();
        assert!(
            read_tensor_field(&dir, "latest", "sigma")
                .unwrap_err()
                .message()
                .contains("时间目录名无法解析")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn displacement_field_reads_as_magnitude() {
        let dir = std::env::temp_dir().join(format!("kairos-t64-{}", std::process::id()));
        fs::create_dir_all(dir.join("2")).unwrap();
        fs::write(dir.join("2").join("D"), DISPLACEMENT_NONUNIFORM).unwrap();

        let d = read_field(&dir, "2", "D").unwrap();
        assert!(d.is_magnitude, "位移场按矢量口径取模量");
        assert!(d.complete);
        assert_eq!(d.values.len(), 2);
        assert!((d.values[0] - 0.3).abs() < 1e-12);
        assert!((d.values[1] - 0.4).abs() < 1e-12);
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

    #[test]
    fn scan_times_lists_only_readable_fields() {
        let dir = std::env::temp_dir().join(format!("kairos-t13kind-{}", std::process::id()));
        let time_dir = dir.join("1");
        fs::create_dir_all(&time_dir).unwrap();
        fs::write(time_dir.join("T"), SCALAR_UNIFORM).unwrap();
        fs::write(time_dir.join("U"), VECTOR_NONUNIFORM).unwrap();
        // 求解器写的阶段记录：不是场对象，不该进场清单
        fs::write(
            time_dir.join("moldingStage"),
            "FoamFile\n{\n    class       moldingStage;\n    object      moldingStage;\n}\n\n1 1.08 0 1 -1\n",
        )
        .unwrap();
        let catalog = scan_times(&dir).unwrap();
        assert_eq!(
            catalog.times[0].fields,
            vec!["T".to_string(), "U".to_string()]
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unreadable_field_files_are_not_listed() {
        // 打不开的路径（不存在的文件）与读不出头部的路径（目录）都不算可读场。
        let dir = std::env::temp_dir().join(format!("kairos-t13hdr-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        assert!(!is_readable_field(&dir.join("missing")));
        assert!(!is_readable_field(&dir));
        fs::remove_dir_all(&dir).ok();
    }
}

/// 二进制场编码的魔数与元数据结构（大体积数据走 IPC 原始字节通道）。
/// 二进制场编码 / 解码：[magic 4B][meta_len u32 LE][meta JSON][f64 值 × n LE]。
/// 大结果走原始字节而非 JSON 数组，体积与解析开销都显著降低。
pub mod field_binary {
    use crate::error::{KairosError, Result};
    use crate::models::results::{ScalarField, VectorField};

    const MAGIC: [u8; 4] = *b"KF1\x00";
    /// 压缩口径标识：值区按 f32 截断（相对误差 ≤ 2^-24 ≈ 6e-8）。
    /// 实测 1e7 值：载荷 80 MB → 40 MB；编解码耗时基本持平（31 / 12 ms），
    /// 省的是传输与内存，不是 CPU；解析 ascii 场（426 ms）才是加载链路大头。
    const FORMAT_F32: &str = "f32";
    const FORMAT_F64: &str = "f64";

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Meta {
        field: String,
        time_dir: String,
        time_s: f64,
        is_magnitude: bool,
        complete: bool,
        count: usize,
        /// 值区格式：f32（默认）/ f64；缺省视为 f64（兼容旧载荷）。
        #[serde(default = "default_format")]
        format: String,
        /// true = 值区是三分量矢量（每单元 3 个值）。
        #[serde(default)]
        vector: bool,
    }

    fn default_format() -> String {
        FORMAT_F64.to_string()
    }

    /// 值区格式：编码端按 f32；解码端兼容 f64 旧载荷（测试与历史缓存）。
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ValueFormat {
        F32,
        F64,
    }

    impl ValueFormat {
        fn tag(self) -> &'static str {
            match self {
                Self::F32 => FORMAT_F32,
                Self::F64 => FORMAT_F64,
            }
        }

        fn parse(tag: &str) -> Result<Self> {
            match tag {
                FORMAT_F32 => Ok(Self::F32),
                FORMAT_F64 => Ok(Self::F64),
                other => Err(KairosError::validation(format!(
                    "二进制场数据无效：未知的值格式 {other}。"
                ))),
            }
        }

        fn width(self) -> usize {
            match self {
                Self::F32 => 4,
                Self::F64 => 8,
            }
        }

        fn push(self, bytes: &mut Vec<u8>, value: f64) {
            match self {
                Self::F32 => bytes.extend_from_slice(&(value as f32).to_le_bytes()),
                Self::F64 => bytes.extend_from_slice(&value.to_le_bytes()),
            }
        }

        fn read(self, raw: &[u8]) -> f64 {
            match self {
                Self::F32 => f64::from(f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]])),
                Self::F64 => f64::from_le_bytes([
                    raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
                ]),
            }
        }
    }

    /// 编码标量场（值区 f32）。
    pub fn encode(field: &ScalarField) -> Vec<u8> {
        encode_scalar(field, ValueFormat::F32)
    }

    /// 编码标量场并指定值区格式（f64 用于无损对拍与回归）。
    pub fn encode_scalar(field: &ScalarField, format: ValueFormat) -> Vec<u8> {
        let meta = serde_json::to_vec(&Meta {
            field: field.field.clone(),
            time_dir: field.time_dir.clone(),
            time_s: field.time_s,
            is_magnitude: field.is_magnitude,
            complete: field.complete,
            count: field.values.len(),
            format: format.tag().to_string(),
            vector: false,
        })
        .expect("元数据为纯标量结构，序列化不会失败");
        let mut bytes = Vec::with_capacity(8 + meta.len() + field.values.len() * format.width());
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&(meta.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&meta);
        for value in &field.values {
            format.push(&mut bytes, *value);
        }
        bytes
    }

    /// 编码矢量场（三分量；值区 f32，按 x/y/z 顺序平铺）。
    pub fn encode_vector(field: &VectorField) -> Vec<u8> {
        let format = ValueFormat::F32;
        let meta = serde_json::to_vec(&Meta {
            field: field.field.clone(),
            time_dir: field.time_dir.clone(),
            time_s: field.time_s,
            is_magnitude: false,
            complete: field.complete,
            count: field.components.len(),
            format: format.tag().to_string(),
            vector: true,
        })
        .expect("元数据为纯标量结构，序列化不会失败");
        let mut bytes = Vec::with_capacity(8 + meta.len() + field.components.len() * 3 * 4);
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&(meta.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&meta);
        for group in &field.components {
            for value in group {
                format.push(&mut bytes, *value);
            }
        }
        bytes
    }

    struct Envelope {
        meta: Meta,
        format: ValueFormat,
        values_end: usize,
    }

    fn envelope(bytes: &[u8]) -> Result<Envelope> {
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
        let format = ValueFormat::parse(&meta.format)?;
        let width = format.width() * if meta.vector { 3 } else { 1 };
        if bytes.len() - meta_end < meta.count * width {
            return Err(invalid("值区被截断"));
        }
        Ok(Envelope {
            meta,
            format,
            values_end: meta_end,
        })
    }

    /// 解码标量场（矢量载荷报错，避免误读成标量）。
    pub fn decode(bytes: &[u8]) -> Result<ScalarField> {
        let envelope = envelope(bytes)?;
        if envelope.meta.vector {
            return Err(KairosError::validation(
                "二进制场数据无效：该载荷是矢量场（请用矢量解码）。",
            ));
        }
        let mut offset = envelope.values_end;
        let width = envelope.format.width();
        let mut values = Vec::with_capacity(envelope.meta.count);
        for _ in 0..envelope.meta.count {
            values.push(envelope.format.read(&bytes[offset..offset + width]));
            offset += width;
        }
        Ok(ScalarField {
            field: envelope.meta.field,
            time_dir: envelope.meta.time_dir,
            time_s: envelope.meta.time_s,
            values,
            is_magnitude: envelope.meta.is_magnitude,
            complete: envelope.meta.complete,
        })
    }

    /// 解码矢量场三分量。
    pub fn decode_vector(bytes: &[u8]) -> Result<VectorField> {
        let envelope = envelope(bytes)?;
        if !envelope.meta.vector {
            return Err(KairosError::validation(
                "二进制场数据无效：该载荷是标量场（请用标量解码）。",
            ));
        }
        let width = envelope.format.width();
        let mut offset = envelope.values_end;
        let mut components = Vec::with_capacity(envelope.meta.count);
        for _ in 0..envelope.meta.count {
            let mut group = [0.0f64; 3];
            for value in &mut group {
                *value = envelope.format.read(&bytes[offset..offset + width]);
                offset += width;
            }
            components.push(group);
        }
        Ok(VectorField {
            field: envelope.meta.field,
            time_dir: envelope.meta.time_dir,
            time_s: envelope.meta.time_s,
            components,
            complete: envelope.meta.complete,
        })
    }
}

/// 有界场缓存（LRU + 值数加权）：命中时跳过磁盘读取；容量按「缓存值总数」计，
/// 大场与小场一视同仁地占额，避免几个大场把内存顶爆。淘汰最久未使用项。
pub struct FieldCache {
    capacity: usize,
    /// 值数上限（缓存内所有场的 values 长度之和）。
    value_budget: usize,
    entries: std::collections::HashMap<String, ScalarField>,
    /// 最近使用顺序：末尾最新。
    order: Vec<String>,
    hits: usize,
    misses: usize,
}

impl FieldCache {
    /// 容量 1..=64；值数预算默认 2e7（f64 计约 160 MB）。
    pub fn new(capacity: usize) -> Result<Self> {
        Self::with_budget(capacity, DEFAULT_VALUE_BUDGET)
    }

    /// 指定值数预算（测试与小内存场景用）。
    pub fn with_budget(capacity: usize, value_budget: usize) -> Result<Self> {
        if capacity == 0 || capacity > 64 {
            return Err(KairosError::validation("场缓存容量必须在 1..=64 之间。"));
        }
        if value_budget == 0 {
            return Err(KairosError::validation("场缓存值数预算必须为正数。"));
        }
        Ok(Self {
            capacity,
            value_budget,
            entries: std::collections::HashMap::new(),
            order: Vec::new(),
            hits: 0,
            misses: 0,
        })
    }

    /// 命中即刷新为最近使用（LRU 语义）。
    pub fn get(&mut self, key: &str) -> Option<&ScalarField> {
        if self.entries.contains_key(key) {
            self.hits += 1;
            self.touch(key);
            return self.entries.get(key);
        }
        self.misses += 1;
        None
    }

    /// 插入并淘汰：先按条数、再按值数预算，从最久未使用端开始移除。
    pub fn put(&mut self, key: String, field: ScalarField) {
        self.touch(&key);
        self.entries.insert(key, field);
        while self.order.len() > self.capacity || self.total_values() > self.value_budget {
            if self.order.len() <= 1 {
                // 单个场就超预算：保留它（否则缓存永远空转），由调用方决定是否加载
                break;
            }
            // 循环条件保证此处至少两项：下标 0 一定不是最近使用项。
            let oldest = self.order.remove(0);
            self.entries.remove(&oldest);
        }
    }

    fn touch(&mut self, key: &str) {
        if let Some(position) = self.order.iter().position(|entry| entry == key) {
            self.order.remove(position);
        }
        self.order.push(key.to_string());
    }

    fn total_values(&self) -> usize {
        self.entries.values().map(|field| field.values.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 命中 / 未命中计数（性能与策略验证用）。
    pub fn stats(&self) -> (usize, usize) {
        (self.hits, self.misses)
    }

    /// 当前缓存的场数量与值总数（内存占用上界判据）。
    pub fn usage(&self) -> (usize, usize) {
        (self.entries.len(), self.total_values())
    }
}

/// 缓存值数预算：2e7 值（f64 计 ≈ 160 MB），覆盖数十个时间步的中等场。
const DEFAULT_VALUE_BUDGET: usize = 20_000_000;

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
        truncated.truncate(truncated.len() - 3);
        let error = field_binary::decode(&truncated).unwrap_err();
        assert!(error.message().contains("值区被截断"));
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

    const VECTOR_FIXTURE: &str = r#"FoamFile
{
    version     2.0;
    format      ascii;
    class       volVectorField;
    object      D;
}
dimensions      [0 1 0 0 0 0 0];
internalField   nonuniform List<vector>
3
(
(1e-05 0 0)
(2e-05 1e-06 0)
(-3e-05 0 1e-06)
)
;
boundaryField { }
"#;

    const SCALAR_FIXTURE: &str = r#"FoamFile
{
    version     2.0;
    format      ascii;
    class       volScalarField;
    object      T;
}
dimensions      [0 0 0 1 0 0 0];
internalField   nonuniform List<scalar>
3
(
300.5
301.25
302.75
)
;
boundaryField { }
"#;

    #[test]
    fn binary_f32_compression_halves_payload_within_tolerance() {
        // 物理量量级混合：温度 ~300、压力 1e5、速度 1e-1、时间 1e-3
        let values: Vec<f64> = (0..1000)
            .map(|index| {
                let t = index as f64;
                300.0 + t * 0.25 + (t * 0.7).sin() * 5.0 + 1.0e5 * (t * 0.001).cos() * 1e-5
            })
            .collect();
        let field = ScalarField {
            field: "T".into(),
            time_dir: "0.1".into(),
            time_s: 0.1,
            values: values.clone(),
            is_magnitude: false,
            complete: true,
        };
        let compressed = field_binary::encode_scalar(&field, field_binary::ValueFormat::F32);
        let lossless = field_binary::encode_scalar(&field, field_binary::ValueFormat::F64);
        // 值区字节数正好减半（元数据部分不变）
        let meta_overhead = lossless.len() - values.len() * 8;
        assert_eq!(compressed.len(), meta_overhead + values.len() * 4);
        assert!(compressed.len() * 2 <= lossless.len() + values.len() * 8 - values.len() * 4 + 1);

        let decoded = field_binary::decode(&compressed).unwrap();
        assert_eq!(decoded.values.len(), values.len());
        // 容差：f32 尾数 24 位 → 相对误差 ≤ 2^-24
        for (actual, expected) in decoded.values.iter().zip(&values) {
            let tolerance = 2f64.powi(-24) * expected.abs().max(1.0);
            assert!(
                (actual - expected).abs() <= tolerance,
                "{actual} vs {expected}"
            );
        }
        // f64 通道逐位一致（回归对拍用）
        assert_eq!(field_binary::decode(&lossless).unwrap().values, values);
    }

    #[test]
    fn binary_vector_round_trip_and_cross_format_errors() {
        let vectors = VectorField {
            field: "D".into(),
            time_dir: "2".into(),
            time_s: 2.0,
            components: vec![[0.001, -0.002, 0.0], [1.5e-4, 2.5e-5, -3.5e-5]],
            complete: true,
        };
        let bytes = field_binary::encode_vector(&vectors);
        let decoded = field_binary::decode_vector(&bytes).unwrap();
        assert_eq!(decoded.field, "D");
        assert_eq!(decoded.time_dir, "2");
        assert_eq!(decoded.components.len(), 2);
        for (actual, expected) in decoded.components.iter().zip(&vectors.components) {
            for axis in 0..3 {
                let tolerance = 2f64.powi(-24) * expected[axis].abs().max(1.0);
                assert!((actual[axis] - expected[axis]).abs() <= tolerance);
            }
        }

        // 交叉解码：矢量载荷不能被当成标量读，反之亦然
        assert!(
            field_binary::decode(&bytes)
                .unwrap_err()
                .message()
                .contains("该载荷是矢量场")
        );
        let scalar = ScalarField {
            field: "T".into(),
            time_dir: "0".into(),
            time_s: 0.0,
            values: vec![1.0],
            is_magnitude: false,
            complete: true,
        };
        assert!(
            field_binary::decode_vector(&field_binary::encode(&scalar))
                .unwrap_err()
                .message()
                .contains("该载荷是标量场")
        );

        // 未知值格式：手工构造载荷（编码端不会产出）→ 明确报错
        let meta = r#"{"field":"T","time_dir":"0","time_s":0.0,"is_magnitude":false,"complete":true,"count":1,"format":"f16","vector":false}"#;
        let mut crafted = Vec::new();
        crafted.extend_from_slice(b"KF1\x00");
        crafted.extend_from_slice(&(meta.len() as u32).to_le_bytes());
        crafted.extend_from_slice(meta.as_bytes());
        crafted.extend_from_slice(&1.0f32.to_le_bytes());
        assert!(
            field_binary::decode(&crafted)
                .unwrap_err()
                .message()
                .contains("未知的值格式")
        );
        // 缺 format 字段的历史载荷按 f64 读（向后兼容）
        let legacy_meta = r#"{"field":"T","time_dir":"0","time_s":0.0,"is_magnitude":false,"complete":true,"count":1}"#;
        let mut legacy = Vec::new();
        legacy.extend_from_slice(b"KF1\x00");
        legacy.extend_from_slice(&(legacy_meta.len() as u32).to_le_bytes());
        legacy.extend_from_slice(legacy_meta.as_bytes());
        legacy.extend_from_slice(&2.5f64.to_le_bytes());
        assert_eq!(field_binary::decode(&legacy).unwrap().values, vec![2.5]);
    }

    #[test]
    fn cache_is_lru_and_weighted_by_values() {
        let field = |name: &str, count: usize| ScalarField {
            field: name.into(),
            time_dir: "0".into(),
            time_s: 0.0,
            values: vec![0.0; count],
            is_magnitude: false,
            complete: true,
        };
        // 条数上限 2：get 命中会刷新最近使用，淘汰最久未用
        let mut cache = FieldCache::new(2).unwrap();
        cache.put("a".into(), field("a", 1));
        cache.put("b".into(), field("b", 1));
        assert!(cache.get("a").is_some());
        cache.put("c".into(), field("c", 1));
        assert!(cache.get("b").is_none(), "最久未用的 b 应被淘汰");
        assert!(cache.get("a").is_some());
        let (hits, misses) = cache.stats();
        assert!(hits >= 2 && misses >= 1);

        // 值数加权：预算 5，两个 3 值场 → 插入第二个时淘汰第一个
        let mut weighted = FieldCache::with_budget(8, 5).unwrap();
        weighted.put("small".into(), field("small", 3));
        assert_eq!(weighted.usage(), (1, 3));
        weighted.put("big".into(), field("big", 3));
        assert!(weighted.get("small").is_none());
        assert_eq!(weighted.usage(), (1, 3));

        // 单个场就超预算：保留（否则缓存永远空转）
        let mut oversize = FieldCache::with_budget(8, 2).unwrap();
        oversize.put("huge".into(), field("huge", 10));
        assert_eq!(oversize.usage(), (1, 10));
        assert!(oversize.get("huge").is_some());
        assert!(FieldCache::with_budget(8, 0).is_err());
    }

    /// 机械量级对拍：单轴 + 纯剪 + 各向同性压力下的模量与主轴。
    #[test]
    fn tensor_magnitude_and_principal_axis_match_analytic_values() {
        // 单轴 xx=100：模量 100、主轴 +x
        assert_eq!(
            symm_tensor_magnitude(&[100.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            100.0
        );
        assert_eq!(
            principal_axis(&[100.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            [1.0, 0.0, 0.0]
        );

        // 纯剪 xy=50：模量 sqrt(2·50²)=70.71，主轴 (1,1,0)/√2
        let shear = [0.0, 50.0, 0.0, 0.0, 0.0, 0.0];
        assert!((symm_tensor_magnitude(&shear) - 70.71067811865476).abs() < 1e-12);
        let axis = principal_axis(&shear);
        let expected = 0.5f64.sqrt();
        // 纯剪的两个特征值 ±50 等模，主轴在 x-y 平面内（符号已规范化）
        assert!((axis[0].abs() - expected).abs() < 1e-9);
        assert!((axis[1].abs() - expected).abs() < 1e-9);
        assert!(axis[2].abs() < 1e-9);
        assert!(axis[0] > 0.0, "符号规范化后主分量应为正：{axis:?}");

        // 静水压 p=10（xx=yy=zz=10）：模量 sqrt(3·100)=17.32；主轴退化（三轴等特征值）
        let pressure = [10.0, 0.0, 0.0, 10.0, 0.0, 10.0];
        assert!((symm_tensor_magnitude(&pressure) - 17.320508075688775).abs() < 1e-12);
        let axis = principal_axis(&pressure);
        assert!(axis.iter().all(|value| value.is_finite()));
        let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        assert!((norm - 1.0).abs() < 1e-9);

        // 对角张量按 |λ| 取主轴：对角 (1, -5, 3) 的 |λ| 最大者是 -5（y 方向）
        let diagonal = [1.0, 0.0, 0.0, -5.0, 0.0, 3.0];
        let axis = principal_axis(&diagonal);
        assert!(axis[0].abs() < 1e-9);
        assert!((axis[1].abs() - 1.0).abs() < 1e-9);
        assert!(axis[2].abs() < 1e-9);

        // xz / yz 剪切：最大非对角元不在 (0,1) 位置，主轴落在对应平面内
        let xz = principal_axis(&[0.0, 0.0, 50.0, 0.0, 0.0, 0.0]);
        assert!((xz[0].abs() - 0.5f64.sqrt()).abs() < 1e-9);
        assert!(xz[1].abs() < 1e-9);
        assert!((xz[2].abs() - 0.5f64.sqrt()).abs() < 1e-9);
        let yz = principal_axis(&[0.0, 0.0, 0.0, 0.0, 50.0, 0.0]);
        assert!(yz[0].abs() < 1e-9);
        assert!((yz[1].abs() - 0.5f64.sqrt()).abs() < 1e-9);
        assert!((yz[2].abs() - 0.5f64.sqrt()).abs() < 1e-9);

        // z 为主导方向（|λ| 最大在第三轴）：符号规范化取 z 分量
        let z_axis = principal_axis(&[1.0, 0.0, 0.0, 2.0, 0.0, 5.0]);
        assert!(z_axis[0].abs() < 1e-9);
        assert!(z_axis[1].abs() < 1e-9);
        assert!((z_axis[2] - 1.0).abs() < 1e-9);

        // 零张量与非法值 → 零向量（不产生 NaN）
        assert_eq!(principal_axis(&[0.0; 6]), [1.0, 0.0, 0.0]);
        assert_eq!(
            principal_axis(&[f64::NAN, 0.0, 0.0, 0.0, 0.0, 0.0]),
            [0.0; 3]
        );

        // 分量数不是 6 的倍数：截断并标记不完整
        let (tensors, complete) = parse_internal_tensors(
            "internalField   nonuniform List<symmTensor>\n2\n(\n(1 2 3 4 5 6)\n(1 2 3 4 5)\n)\n;\n",
        );
        assert_eq!(tensors.len(), 1);
        assert!(!complete);
    }

    #[test]
    fn vector_field_reads_components_and_rejects_scalars() {
        let dir = std::env::temp_dir().join(format!("kairos-vec-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("2")).unwrap();
        std::fs::write(dir.join("2/D"), VECTOR_FIXTURE).unwrap();
        std::fs::write(dir.join("2/T"), SCALAR_FIXTURE).unwrap();
        // 非数字时间目录（文件存在，仅目录名无法解析）
        std::fs::create_dir_all(dir.join("latest")).unwrap();
        std::fs::write(dir.join("latest/D"), VECTOR_FIXTURE).unwrap();

        let field = read_vector_field(&dir, "2", "D").unwrap();
        assert_eq!(field.field, "D");
        assert_eq!(field.time_s, 2.0);
        assert!(field.complete);
        assert!(field.components.iter().all(|group| group[0].is_finite()));

        let error = read_vector_field(&dir, "2", "T").unwrap_err();
        assert!(error.message().contains("不是矢量场"));
        assert!(read_vector_field(&dir, "2", "missing").is_err());
        let error = read_vector_field(&dir, "latest", "D").unwrap_err();
        assert!(error.message().contains("时间目录名无法解析"));
        std::fs::remove_dir_all(&dir).ok();
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
