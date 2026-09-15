//! DOE / 正交试验编排：参数矩阵生成与批次汇总表。
//!
//! 只做编排与汇总（纯逻辑）：矩阵怎么排、每次运行的参数与指标怎么记。
//! 真正的求解由调用方按 `DoeRun` 串行提交（VM 单实例 / 本机单实例），
//! 失败运行显式标记、批次继续——静默丢弃失败点会让结论建立在缺失样本上。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{KairosError, Result};

/// 一个试验因子：名称 + 各水平取值。
#[derive(Debug, Clone, PartialEq)]
pub struct DoeFactor {
    pub name: String,
    pub values: Vec<f64>,
}

/// 矩阵编排方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoePlan {
    /// 正交表 L9（3 水平、最多 4 因子、9 次运行）。
    OrthogonalL9,
    /// 全因子（维度 ≤ 3，总次数上限见 `FULL_FACTORIAL_LIMIT`）。
    FullFactorial,
}

/// 全因子运行的次数上限：超过就不是「小规模摸底」而是批量生产，改用正交表。
pub const FULL_FACTORIAL_LIMIT: usize = 64;

/// L9(3^4) 正交表：9 行 × 4 列，每列 3 个水平各出现 3 次。
pub const L9_TABLE: [[usize; 4]; 9] = [
    [0, 0, 0, 0],
    [0, 1, 1, 1],
    [0, 2, 2, 2],
    [1, 0, 1, 2],
    [1, 1, 2, 0],
    [1, 2, 0, 1],
    [2, 0, 2, 1],
    [2, 1, 0, 2],
    [2, 2, 1, 0],
];

/// 单次运行的记录：参数 + 状态 + 指标 + 耗时（指标与耗时由执行方回填）。
#[derive(Debug, Clone, PartialEq)]
pub struct DoeRun {
    pub index: usize,
    pub parameters: BTreeMap<String, f64>,
    pub status: DoeStatus,
    /// 指标名 → 值（如填充时间、V/P 切换压力、最大注射压力、入口压力）。
    pub metrics: BTreeMap<String, f64>,
    /// 墙钟耗时（秒）；未执行为 None。
    pub elapsed_s: Option<f64>,
}

/// 运行状态：失败必须带原因，汇总表里如实呈现。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoeStatus {
    Pending,
    Done,
    Failed(String),
}

impl DoeStatus {
    fn label(&self) -> &'static str {
        match self {
            DoeStatus::Pending => "pending",
            DoeStatus::Done => "done",
            DoeStatus::Failed(_) => "failed",
        }
    }

    fn reason(&self) -> String {
        match self {
            DoeStatus::Failed(reason) => reason.clone(),
            _ => String::new(),
        }
    }
}

/// 本次运行在 `cases/<方案 id>/` 下的目录名：`run-001`（三位补零，按序可读可排序）。
pub fn run_dir_name(index: usize) -> String {
    format!("run-{index:03}")
}

/// 单次运行的 case 目录：`<工作区>/cases/<方案 id>/run-XXX`。
///
/// 每次运行独立目录是硬要求：共用目录会让上一次的时间目录被当成这一次的结果
/// （求解与回传都按目录扫描，混在一起无法区分）。
pub fn run_case_dir(root: &Path, study_id: &str, index: usize) -> PathBuf {
    crate::services::workspace::cases_dir(root, study_id).join(run_dir_name(index))
}

/// 批次汇总目录：`<工作区>/doe/<批次名>`。
pub fn batch_dir(root: &Path, batch: &str) -> PathBuf {
    root.join("doe").join(batch)
}

/// 可用的因子名（与 `ProcessSettings` 字段一一对应）——未知因子名**必须报错**：
/// 静默忽略会让整批运行的参数完全一样，而汇总表看起来「跑过了」。
pub const FACTOR_NAMES: [&str; 9] = [
    "熔体温度",
    "模具温度",
    "顶出温度",
    "注射时间",
    "V/P 切换",
    "保压压力",
    "保压时间",
    "冷却时间",
    "介质温度",
];

/// 把一次运行的参数应用到工艺设置上（其余字段沿用基准值）。
pub fn apply_factors(
    base: &crate::models::process::ProcessSettings,
    run: &DoeRun,
) -> Result<crate::models::process::ProcessSettings> {
    let mut settings = base.clone();
    for (name, value) in &run.parameters {
        if !value.is_finite() {
            return Err(KairosError::validation(format!(
                "因子「{name}」的水平不是有限数（{value}）。"
            )));
        }
        match name.as_str() {
            "熔体温度" => settings.melt_temp_c = *value,
            "模具温度" => settings.mold_temp_c = *value,
            "顶出温度" => settings.ejection_temp_c = *value,
            "注射时间" => settings.injection_time_s = *value,
            "V/P 切换" => settings.vp_switch_volume_percent = *value,
            // 保压是曲线：单值因子落成「从 0 起恒定」的曲线（DOE 调的是保压压力水平）
            "保压压力" => settings.packing_pressure_mpa_curve = vec![(0.0, *value)],
            "保压时间" => settings.packing_time_s = *value,
            "冷却时间" => settings.cooling_time_s = *value,
            "介质温度" => settings.coolant_temp_c = *value,
            other => {
                return Err(KairosError::validation(format!(
                    "未知因子「{other}」，可用：{}。",
                    FACTOR_NAMES.join(" / ")
                )));
            }
        }
    }
    Ok(settings)
}

/// 标记运行成功：指标与耗时一起写入（指标缺失允许，缺失即空单元格）。
pub fn mark_done(run: &mut DoeRun, metrics: BTreeMap<String, f64>, elapsed_s: f64) {
    run.status = DoeStatus::Done;
    run.metrics = metrics;
    run.elapsed_s = Some(elapsed_s);
}

/// 标记运行失败：原因必填——静默丢弃失败点会让结论建立在缺失样本上。
pub fn mark_failed(run: &mut DoeRun, reason: &str, elapsed_s: Option<f64>) {
    run.status = DoeStatus::Failed(reason.to_string());
    run.elapsed_s = elapsed_s;
}

/// 生成参数矩阵：每行一次运行，参数按因子名索引。
pub fn build_matrix(plan: DoePlan, factors: &[DoeFactor]) -> Result<Vec<DoeRun>> {
    if factors.is_empty() {
        return Err(KairosError::validation("DOE 至少需要一个因子。"));
    }
    for factor in factors {
        if factor.values.is_empty() {
            return Err(KairosError::validation(format!(
                "因子「{}」没有水平取值。",
                factor.name
            )));
        }
    }
    let indices: Vec<Vec<usize>> = match plan {
        DoePlan::OrthogonalL9 => l9_rows(factors)?,
        DoePlan::FullFactorial => full_factorial_rows(factors)?,
    };
    Ok(indices
        .into_iter()
        .enumerate()
        .map(|(index, row)| DoeRun {
            index: index + 1,
            parameters: factors
                .iter()
                .zip(row)
                .map(|(factor, level)| (factor.name.clone(), factor.values[level]))
                .collect(),
            status: DoeStatus::Pending,
            metrics: BTreeMap::new(),
            elapsed_s: None,
        })
        .collect())
}

/// L9 行：最多 4 个因子、每因子恰好 3 个水平（正交表的定义使然，不能满足就明确报错，
/// 而不是用前 3 个水平凑合——那会让用户以为做了正交设计）。
fn l9_rows(factors: &[DoeFactor]) -> Result<Vec<Vec<usize>>> {
    if factors.len() > 4 {
        return Err(KairosError::validation(format!(
            "正交表 L9 最多支持 4 个因子，当前 {} 个。",
            factors.len()
        )));
    }
    for factor in factors {
        if factor.values.len() != 3 {
            return Err(KairosError::validation(format!(
                "正交表 L9 要求每个因子恰好 3 个水平，因子「{}」有 {} 个。",
                factor.name,
                factor.values.len()
            )));
        }
    }
    Ok(L9_TABLE
        .iter()
        .map(|row| row[..factors.len()].to_vec())
        .collect())
}

/// 全因子行：笛卡尔积，按维度与总次数设上限。
fn full_factorial_rows(factors: &[DoeFactor]) -> Result<Vec<Vec<usize>>> {
    if factors.len() > 3 {
        return Err(KairosError::validation(format!(
            "全因子仅支持 ≤ 3 个维度，当前 {} 个（更高维请用正交表）。",
            factors.len()
        )));
    }
    let total: usize = factors.iter().map(|factor| factor.values.len()).product();
    if total > FULL_FACTORIAL_LIMIT {
        return Err(KairosError::validation(format!(
            "全因子共 {total} 次运行，超过上限 {FULL_FACTORIAL_LIMIT}（改用正交表）。"
        )));
    }
    let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
    for factor in factors {
        let mut next = Vec::with_capacity(rows.len() * factor.values.len());
        for row in &rows {
            for level in 0..factor.values.len() {
                let mut extended = row.clone();
                extended.push(level);
                next.push(extended);
            }
        }
        rows = next;
    }
    Ok(rows)
}

/// 批次汇总表（CSV）：每行一次运行，列为「序号 + 因子 + 状态 + 失败原因 + 指标 + 耗时」。
///
/// 列顺序在整批内固定：先按首个运行里出现的顺序排因子与指标，缺值的运行留空单元格，
/// 这样同一批次的不同运行可以直接对比（表头一致）。
pub fn summary_csv(runs: &[DoeRun]) -> String {
    let (factor_names, metric_names) = column_names(runs);
    let mut lines = vec![header(&factor_names, &metric_names)];
    for run in runs {
        let mut cells = vec![run.index.to_string()];
        for name in &factor_names {
            cells.push(format_value(run.parameters.get(name)));
        }
        cells.push(run.status.label().to_string());
        cells.push(run.status.reason());
        for name in &metric_names {
            cells.push(format_value(run.metrics.get(name)));
        }
        cells.push(format_value(run.elapsed_s.as_ref()));
        lines.push(cells.join(","));
    }
    lines.join("\n") + "\n"
}

/// 批次汇总（JSON）：数组形式，字段名与 CSV 列一致，便于脚本消费。
pub fn summary_json(runs: &[DoeRun]) -> String {
    let mut items = Vec::with_capacity(runs.len());
    for run in runs {
        let parameters = run
            .parameters
            .iter()
            .map(|(name, value)| format!("\"{}\":{}", escape(name), number(*value)))
            .collect::<Vec<_>>()
            .join(",");
        let metrics = run
            .metrics
            .iter()
            .map(|(name, value)| format!("\"{}\":{}", escape(name), number(*value)))
            .collect::<Vec<_>>()
            .join(",");
        items.push(format!(
            "{{\"index\":{},\"parameters\":{{{parameters}}},\"status\":\"{}\",\"reason\":\"{}\",\"metrics\":{{{metrics}}},\"elapsedS\":{}}}",
            run.index,
            run.status.label(),
            escape(&run.status.reason()),
            run.elapsed_s.map(number).unwrap_or_else(|| "null".to_string())
        ));
    }
    format!("[{}]\n", items.join(","))
}

/// 列顺序：按运行里首次出现的顺序收集（BTreeMap 已按名排序，跨运行稳定）。
fn column_names(runs: &[DoeRun]) -> (Vec<String>, Vec<String>) {
    let mut factors: Vec<String> = Vec::new();
    let mut metrics: Vec<String> = Vec::new();
    for run in runs {
        for name in run.parameters.keys() {
            if !factors.contains(name) {
                factors.push(name.clone());
            }
        }
        for name in run.metrics.keys() {
            if !metrics.contains(name) {
                metrics.push(name.clone());
            }
        }
    }
    (factors, metrics)
}

fn header(factors: &[String], metrics: &[String]) -> String {
    let mut cells = vec!["run".to_string()];
    cells.extend(factors.iter().cloned());
    cells.extend(["status".to_string(), "reason".to_string()]);
    cells.extend(metrics.iter().cloned());
    cells.push("elapsed_s".to_string());
    cells.join(",")
}

/// 数值单元格：空值留空；用固定小数位避免 `1e-7` 这类科学计数进入 CSV 汇总。
fn format_value(value: Option<&f64>) -> String {
    value.map(|value| number(*value)).unwrap_or_default()
}

fn number(value: f64) -> String {
    let text = format!("{value:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// JSON 字符串转义：因子名与失败原因来自用户输入，不能直接拼进 JSON。
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_level(name: &str) -> DoeFactor {
        DoeFactor {
            name: name.to_string(),
            values: vec![1.0, 2.0, 3.0],
        }
    }

    fn run(index: usize, status: DoeStatus, metric: Option<f64>) -> DoeRun {
        let mut parameters = BTreeMap::new();
        parameters.insert("熔体温度".to_string(), 200.0 + index as f64);
        let mut metrics = BTreeMap::new();
        if let Some(value) = metric {
            metrics.insert("填充时间".to_string(), value);
        }
        DoeRun {
            index,
            parameters,
            status,
            metrics,
            elapsed_s: Some(12.5),
        }
    }

    #[test]
    fn factors_map_onto_process_settings_and_reject_unknown_names() {
        use crate::models::process::ProcessSettings;
        let base = ProcessSettings {
            melt_temp_c: 230.0,
            mold_temp_c: 40.0,
            ejection_temp_c: 90.0,
            injection_time_s: 1.5,
            vp_switch_volume_percent: 96.0,
            packing_pressure_mpa_curve: vec![(0.0, 50.0), (8.0, 40.0)],
            packing_time_s: 8.0,
            cooling_time_s: 15.0,
            coolant_temp_c: 25.0,
        };
        let mut run = DoeRun {
            index: 1,
            parameters: BTreeMap::new(),
            status: DoeStatus::Pending,
            metrics: BTreeMap::new(),
            elapsed_s: None,
        };
        run.parameters.insert("熔体温度".to_string(), 210.0);
        run.parameters.insert("保压压力".to_string(), 60.0);
        let settings = apply_factors(&base, &run).unwrap();
        assert_eq!(settings.melt_temp_c, 210.0);
        assert_eq!(settings.packing_pressure_mpa_curve, vec![(0.0, 60.0)]);
        // 未涉及的字段沿用基准（不能因为一次运行就把其他参数清空）
        assert_eq!(settings.mold_temp_c, 40.0);
        assert_eq!(settings.cooling_time_s, 15.0);

        // 未知因子名必须报错：静默忽略会让整批参数一样，而汇总表看起来「跑过了」
        let mut bad = run.clone();
        bad.parameters.insert("熔体黏度".to_string(), 1.0);
        let error = apply_factors(&base, &bad).unwrap_err();
        assert!(error.message().contains("未知因子"), "{}", error.message());
        assert!(error.message().contains("熔体温度"), "{}", error.message());

        // 非有限水平也拒绝（NaN 会被写进 case 字典）
        let mut nan = run;
        nan.parameters.insert("熔体温度".to_string(), f64::NAN);
        assert!(apply_factors(&base, &nan).is_err());
    }

    /// 表驱动逐名验证：**每个因子名都要真的落到自己的字段上**。
    ///
    /// 覆盖不到就得靠这条用例兜——行口径看不见这种缺口：因子臂所在的那一行同时含
    /// 「模式比较」区域（遍历每个候选名都会求值、计数非 0），整行因此被判为已执行，
    /// 而臂体本身可能一次都没跑过。漏一个臂的后果是整批运行的该参数悄悄保持基准值，
    /// 汇总表却看起来「跑过了」。
    #[test]
    fn every_factor_name_lands_on_its_own_field() {
        use crate::models::process::ProcessSettings;

        let base = ProcessSettings {
            melt_temp_c: 230.0,
            mold_temp_c: 40.0,
            ejection_temp_c: 90.0,
            injection_time_s: 1.5,
            vp_switch_volume_percent: 96.0,
            packing_pressure_mpa_curve: vec![(0.0, 50.0), (8.0, 40.0)],
            packing_time_s: 8.0,
            cooling_time_s: 15.0,
            coolant_temp_c: 25.0,
        };
        let pending = || DoeRun {
            index: 1,
            parameters: BTreeMap::new(),
            status: DoeStatus::Pending,
            metrics: BTreeMap::new(),
            elapsed_s: None,
        };

        // 注入值全部与基准值不同（基准 + 1 量级）：臂没执行时字段会保持基准值，断言即失败。
        type ReadField = fn(&ProcessSettings) -> f64;
        let cases: [(&str, f64, ReadField); 8] = [
            ("熔体温度", 231.0, |s| s.melt_temp_c),
            ("模具温度", 41.0, |s| s.mold_temp_c),
            ("顶出温度", 91.0, |s| s.ejection_temp_c),
            ("注射时间", 2.5, |s| s.injection_time_s),
            ("V/P 切换", 97.0, |s| s.vp_switch_volume_percent),
            ("保压时间", 9.0, |s| s.packing_time_s),
            ("冷却时间", 16.0, |s| s.cooling_time_s),
            ("介质温度", 26.0, |s| s.coolant_temp_c),
        ];
        for (name, value, read) in cases {
            let mut subject = pending();
            subject.parameters.insert(name.to_string(), value);
            let settings = apply_factors(&base, &subject).unwrap();
            assert_eq!(read(&settings), value, "因子「{name}」没落到对应字段");
        }

        // 保压是曲线而非标量：单值因子落成「从 0 起恒定」的曲线
        let mut pressure = pending();
        pressure.parameters.insert("保压压力".to_string(), 60.0);
        let settings = apply_factors(&base, &pressure).unwrap();
        assert_eq!(settings.packing_pressure_mpa_curve, vec![(0.0, 60.0)]);

        // 表与 FACTOR_NAMES 双向对齐：新增因子名时这里失败，提醒同时补 match 臂与断言
        // （只加 FACTOR_NAMES 不补臂会让新名字直接报「未知因子」）。
        let mut covered: Vec<&str> = cases.iter().map(|(name, _, _)| *name).collect();
        covered.push("保压压力");
        for name in FACTOR_NAMES {
            assert!(
                covered.contains(&name),
                "FACTOR_NAMES 的「{name}」没被本用例覆盖"
            );
        }
        assert_eq!(
            covered.len(),
            FACTOR_NAMES.len(),
            "本用例列出了 FACTOR_NAMES 之外的因子"
        );
    }

    #[test]
    fn run_layout_and_status_transitions() {
        use std::path::Path;
        assert_eq!(run_dir_name(1), "run-001");
        assert_eq!(run_dir_name(12), "run-012");
        assert_eq!(
            run_case_dir(Path::new("/w"), "study-1", 7),
            PathBuf::from("/w/cases/study-1/run-007")
        );
        assert_eq!(
            batch_dir(Path::new("/w"), "L9-温度压力"),
            PathBuf::from("/w/doe/L9-温度压力")
        );

        let mut matrix = build_matrix(DoePlan::OrthogonalL9, &[three_level("温度")]).unwrap();
        let mut metrics = BTreeMap::new();
        metrics.insert("填充时间".to_string(), 1.04);
        mark_done(&mut matrix[0], metrics, 42.5);
        assert_eq!(matrix[0].status, DoeStatus::Done);
        assert_eq!(matrix[0].elapsed_s, Some(42.5));

        mark_failed(&mut matrix[1], "参数超出量程", Some(3.0));
        assert_eq!(
            matrix[1].status,
            DoeStatus::Failed("参数超出量程".to_string())
        );
        // 失败行不继承上一次的指标（否则汇总表会把别人的指标算在失败点上）
        assert!(matrix[1].metrics.is_empty());
        mark_failed(&mut matrix[2], "未启动", None);
        assert_eq!(matrix[2].elapsed_s, None);
    }

    #[test]
    fn l9_matrix_is_balanced_and_bounded() {
        let matrix = build_matrix(
            DoePlan::OrthogonalL9,
            &[three_level("温度"), three_level("压力")],
        )
        .unwrap();
        assert_eq!(matrix.len(), 9);
        assert_eq!(matrix[0].index, 1);
        // L9 只用两列时，前两列的 9 行组合仍是标准表的前两列（每水平各 3 次）
        for column in ["温度", "压力"] {
            // 每列三个水平各出现 3 次（正交表的平衡性）
            let mut counts = [0usize; 3];
            for run in &matrix {
                let level = run.parameters[column];
                counts[(level - 1.0) as usize] += 1;
            }
            assert_eq!(counts, [3, 3, 3]);
        }
        assert!(matrix.iter().all(|run| run.status == DoeStatus::Pending));
    }

    #[test]
    fn l9_rejects_wrong_shape_instead_of_silently_trimming() {
        let too_many = vec![
            three_level("a"),
            three_level("b"),
            three_level("c"),
            three_level("d"),
            three_level("e"),
        ];
        assert!(build_matrix(DoePlan::OrthogonalL9, &too_many).is_err());
        let two_levels = vec![DoeFactor {
            name: "温度".to_string(),
            values: vec![1.0, 2.0],
        }];
        assert!(build_matrix(DoePlan::OrthogonalL9, &two_levels).is_err());
        assert!(build_matrix(DoePlan::OrthogonalL9, &[]).is_err());
        let empty_values = vec![DoeFactor {
            name: "温度".to_string(),
            values: Vec::new(),
        }];
        assert!(build_matrix(DoePlan::OrthogonalL9, &empty_values).is_err());
    }

    #[test]
    fn full_factorial_covers_every_combination_within_limits() {
        let factors = vec![
            DoeFactor {
                name: "温度".to_string(),
                values: vec![200.0, 220.0],
            },
            DoeFactor {
                name: "压力".to_string(),
                values: vec![50.0, 60.0, 70.0],
            },
        ];
        let matrix = build_matrix(DoePlan::FullFactorial, &factors).unwrap();
        assert_eq!(matrix.len(), 6);
        // 六种组合齐全
        let combos: Vec<(f64, f64)> = matrix
            .iter()
            .map(|run| (run.parameters["温度"], run.parameters["压力"]))
            .collect();
        assert!(combos.contains(&(200.0, 70.0)));
        assert!(combos.contains(&(220.0, 50.0)));
        // 维度与次数上限
        let four = vec![
            DoeFactor {
                name: "a".to_string(),
                values: vec![1.0, 2.0],
            },
            DoeFactor {
                name: "b".to_string(),
                values: vec![1.0, 2.0],
            },
            DoeFactor {
                name: "c".to_string(),
                values: vec![1.0, 2.0],
            },
            DoeFactor {
                name: "d".to_string(),
                values: vec![1.0, 2.0],
            },
        ];
        assert!(build_matrix(DoePlan::FullFactorial, &four).is_err());
        let big = vec![
            DoeFactor {
                name: "a".to_string(),
                values: (0..5).map(|i| i as f64).collect(),
            },
            DoeFactor {
                name: "b".to_string(),
                values: (0..5).map(|i| i as f64).collect(),
            },
            DoeFactor {
                name: "c".to_string(),
                values: (0..5).map(|i| i as f64).collect(),
            },
        ];
        assert!(build_matrix(DoePlan::FullFactorial, &big).is_err()); // 125 > 64
    }

    #[test]
    fn summary_csv_keeps_columns_aligned_and_marks_failures() {
        let runs = vec![
            run(1, DoeStatus::Done, Some(0.9607489)),
            run(2, DoeStatus::Failed("参数超出量程".to_string()), None),
            run(3, DoeStatus::Pending, None),
        ];
        let csv = summary_csv(&runs);
        let lines: Vec<&str> = csv.trim_end().split('\n').collect();
        assert_eq!(lines[0], "run,熔体温度,status,reason,填充时间,elapsed_s");
        assert_eq!(lines[1].split(',').nth(2), Some("done"));
        assert_eq!(lines[2].split(',').nth(3), Some("参数超出量程"));
        // 失败行的指标留空但列数一致——静默丢弃会让行错位
        assert_eq!(lines[2].split(',').count(), lines[1].split(',').count());
        assert_eq!(lines[3].split(',').nth(2), Some("pending"));
    }

    #[test]
    fn summary_json_escapes_user_text() {
        let mut runs = vec![run(
            1,
            DoeStatus::Failed("含\"引号\"与\\反斜杠".to_string()),
            None,
        )];
        runs[0].metrics.insert("V/P 切换压力".to_string(), 5.9e6);
        let json = summary_json(&runs);
        assert!(json.starts_with('['));
        assert!(json.contains("\\\"引号\\\""), "{json}");
        assert!(json.contains("\\\\反斜杠"), "{json}");
        assert!(json.contains("\"V/P 切换压力\":5900000"), "{json}");
        // 无耗时运行写 null 而不是 0（0 会被当成「瞬时完成」）
        let mut pending = runs.clone();
        pending[0].elapsed_s = None;
        assert!(summary_json(&pending).contains("\"elapsedS\":null"));
    }
}
