//! DOE / 正交试验编排：参数矩阵生成与批次汇总表。
//!
//! 只做编排与汇总（纯逻辑）：矩阵怎么排、每次运行的参数与指标怎么记。
//! 真正的求解由调用方按 `DoeRun` 串行提交（VM 单实例 / 本机单实例），
//! 失败运行显式标记、批次继续——静默丢弃失败点会让结论建立在缺失样本上。

use std::collections::BTreeMap;

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
