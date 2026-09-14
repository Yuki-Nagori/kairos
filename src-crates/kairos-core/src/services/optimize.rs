//! 工艺参数自动寻优：目标函数、约束承载与搜索驱动。
//!
//! 只做**决策**（下一步试哪组参数、哪组最好、什么时候放弃），不自己跑求解：
//! 调用方把 `next_candidates` 给出的参数交给 DOE 批次执行，再把结果经 `record`
//! 回灌。边界这样切是为了可测——搜索策略用合成目标函数就能完整覆盖，
//! 与求解执行解耦。
//!
//! 失败处置（任务要求）：非物理 / 无指标的结果按**高惩罚**参与比较但不算可行；
//! 同一候选重试一次；连续 3 次失败即中止并报出失败参数点。

use std::collections::BTreeMap;

use crate::error::{KairosError, Result};

/// 优化目标：先支持两项（求解日志里直接可读的指标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Objective {
    /// 填充时间最短（越小越好）。
    MinFillTime,
    /// V/P 切换时刻的浇口压力最低（越小越好）。
    MinInjectionPressure,
}

impl Objective {
    /// 目标对应的指标名（与 `moldingfoam::parse_metrics` 的输出同源）。
    pub fn metric_name(self) -> &'static str {
        match self {
            Objective::MinFillTime => crate::services::moldingfoam::METRIC_FILL_TIME_S,
            Objective::MinInjectionPressure => crate::services::moldingfoam::METRIC_VP_PRESSURE_PA,
        }
    }
}

/// 一个因子的取值范围与粗搜层数。
#[derive(Debug, Clone, PartialEq)]
pub struct FactorRange {
    pub name: String,
    pub min: f64,
    pub max: f64,
    /// 粗搜层数（≥2：至少能覆盖两端）。
    pub levels: usize,
}

/// 一次评估的结果：参数 + 目标值 + 约束违反项。
///
/// `score` 为 None 表示这一跑没有可用指标（求解失败 / 结果非物理）——它仍进入
/// 历史（用于失败计数），但不参与「最优」评选。
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub parameters: BTreeMap<String, f64>,
    pub score: Option<f64>,
    pub violations: Vec<String>,
    pub attempts: usize,
}

impl Evaluation {
    /// 可行 = 有指标且无约束违反。
    pub fn feasible(&self) -> bool {
        self.score.is_some() && self.violations.is_empty()
    }
}

/// 从求解指标里取目标值（缺指标返回 None，不猜、不填零）。
pub fn score_of(objective: Objective, metrics: &BTreeMap<String, f64>) -> Option<f64> {
    metrics
        .get(objective.metric_name())
        .copied()
        .filter(|value| value.is_finite())
}

/// 搜索结果：下一批候选参数（粗搜一次给整张网格，随后给局部细化点）。
pub struct Optimizer {
    ranges: Vec<FactorRange>,
    objective: Objective,
    /// 允许的总评估次数（含重试）。
    budget: usize,
    history: Vec<Evaluation>,
    /// 待评估队列：粗搜网格 → 局部细化点。
    pending: Vec<BTreeMap<String, f64>>,
    /// 当前局部细化的中心（上一轮最优）。
    center: Option<BTreeMap<String, f64>>,
    /// 已经细化过的轮数（收敛到范围下限后停止细化）。
    refine_rounds: usize,
}

/// 局部细化最多几轮：每轮把步长减半，超过后靠粗搜结果收口。
const MAX_REFINE_ROUNDS: usize = 4;
/// 同一候选的重试上限（含首次）：任务要求「重试一次」。
pub const MAX_ATTEMPTS: usize = 2;
/// 连续失败达到该值即中止批次（任务要求 3 次）。
pub const CONSECUTIVE_FAILURE_LIMIT: usize = 3;

impl Optimizer {
    /// 构建优化器：校验范围（min < max、levels ≥ 2、维度 ≤ 6）。
    pub fn new(ranges: Vec<FactorRange>, objective: Objective, budget: usize) -> Result<Self> {
        if ranges.is_empty() {
            return Err(KairosError::validation("寻优至少需要一个因子。"));
        }
        if ranges.len() > 6 {
            return Err(KairosError::validation(format!(
                "寻优维度上限为 6，当前 {} 个（高维搜索样本需求会爆炸式增长）。",
                ranges.len()
            )));
        }
        for range in &ranges {
            if !(range.min.is_finite() && range.max.is_finite()) {
                return Err(KairosError::validation(format!(
                    "因子「{}」的取值范围含非有限值。",
                    range.name
                )));
            }
            if range.max <= range.min {
                return Err(KairosError::validation(format!(
                    "因子「{}」的取值范围必须 min < max（当前 {} ~ {}）。",
                    range.name, range.min, range.max
                )));
            }
            if range.levels < 2 {
                return Err(KairosError::validation(format!(
                    "因子「{}」的层数至少为 2（当前 {}）。",
                    range.name, range.levels
                )));
            }
        }
        let pending = grid_points(&ranges);
        Ok(Self {
            ranges,
            objective,
            budget,
            history: Vec::new(),
            pending,
            center: None,
            refine_rounds: 0,
        })
    }

    /// 目标（供报告展示）。
    pub fn objective(&self) -> Objective {
        self.objective
    }

    /// 已用评估次数与预算。
    pub fn used(&self) -> usize {
        self.history.len()
    }

    /// 下一批候选：粗搜网格一次给全部点；粗搜结束后按上一轮最优做局部细化。
    pub fn next_candidates(&mut self) -> Vec<BTreeMap<String, f64>> {
        if self.history.len() >= self.budget {
            return Vec::new();
        }
        if !self.pending.is_empty() {
            return std::mem::take(&mut self.pending);
        }
        let Some(center) = self.center.clone() else {
            return Vec::new();
        };
        if self.refine_rounds >= MAX_REFINE_ROUNDS {
            return Vec::new();
        }
        self.refine_rounds += 1;
        // 每轮步长减半：粗搜步长的 1/2、1/4…
        let scale = 1.0 / (1 << self.refine_rounds) as f64;
        let points = refine_around(&center, &self.ranges, scale);
        self.history
            .len()
            .checked_add(points.len())
            .filter(|total| *total <= self.budget)
            .map(|_| points.clone())
            .unwrap_or_default()
    }

    /// 记录一次评估；连续失败达到上限时返回中止提示（批次应停止并报出失败点）。
    pub fn record(&mut self, evaluation: Evaluation) -> Option<String> {
        self.history.push(evaluation);
        // 细化中心**只跟随当前全局最优**：失败的评估（无指标）或违反约束的解
        // 都不能成为中心，否则下一轮会围着一次失败去细化。
        let best_parameters = self.best().map(|item| item.parameters.clone());
        if best_parameters != self.center {
            self.center = best_parameters;
        }
        if self.consecutive_failures() >= CONSECUTIVE_FAILURE_LIMIT {
            let worst = self
                .history
                .iter()
                .rev()
                .take(CONSECUTIVE_FAILURE_LIMIT)
                .map(|item| format_parameters(&item.parameters))
                .collect::<Vec<_>>()
                .join("；");
            return Some(format!(
                "连续 {CONSECUTIVE_FAILURE_LIMIT} 次评估无可用指标，已中止寻优。失败参数点：{worst}"
            ));
        }
        None
    }

    /// 当前最优（仅可行解；无可行解返回 None）。
    pub fn best(&self) -> Option<&Evaluation> {
        self.history
            .iter()
            .filter(|item| item.feasible())
            .min_by(|left, right| {
                left.score
                    .partial_cmp(&right.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// 已完成的评估历史。
    pub fn history(&self) -> &[Evaluation] {
        &self.history
    }

    /// 末尾连续「无可用指标」的次数。
    pub fn consecutive_failures(&self) -> usize {
        self.history
            .iter()
            .rev()
            .take_while(|item| item.score.is_none())
            .count()
    }
}

/// 粗搜网格：各因子按 levels 等分取点（笛卡尔积）。
pub fn grid_points(ranges: &[FactorRange]) -> Vec<BTreeMap<String, f64>> {
    let mut points: Vec<BTreeMap<String, f64>> = vec![BTreeMap::new()];
    for range in ranges {
        let mut next = Vec::with_capacity(points.len() * range.levels);
        for point in &points {
            for level in 0..range.levels {
                let ratio = level as f64 / (range.levels - 1) as f64;
                let mut extended = point.clone();
                extended.insert(
                    range.name.clone(),
                    range.min + (range.max - range.min) * ratio,
                );
                next.push(extended);
            }
        }
        points = next;
    }
    points
}

/// 局部细化：以 `center` 为中心、按粗搜步长的 `scale` 倍在范围内取邻点（不含中心本身）。
pub fn refine_around(
    center: &BTreeMap<String, f64>,
    ranges: &[FactorRange],
    scale: f64,
) -> Vec<BTreeMap<String, f64>> {
    let mut points = vec![center.clone()];
    for range in ranges {
        let Some(value) = center.get(&range.name).copied() else {
            continue;
        };
        let step = (range.max - range.min) / (range.levels - 1) as f64 * scale;
        for direction in [-1.0, 1.0] {
            let shifted = (value + direction * step).clamp(range.min, range.max);
            if (shifted - value).abs() <= f64::EPSILON {
                continue; // 贴边时该方向没有新点
            }
            let mut point = center.clone();
            point.insert(range.name.clone(), shifted);
            points.push(point);
        }
    }
    points
}

fn format_parameters(parameters: &BTreeMap<String, f64>) -> String {
    parameters
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(name: &str, min: f64, max: f64, levels: usize) -> FactorRange {
        FactorRange {
            name: name.to_string(),
            min,
            max,
            levels,
        }
    }

    fn metrics(fill_time: f64, pressure: f64) -> BTreeMap<String, f64> {
        BTreeMap::from([
            (
                crate::services::moldingfoam::METRIC_FILL_TIME_S.to_string(),
                fill_time,
            ),
            (
                crate::services::moldingfoam::METRIC_VP_PRESSURE_PA.to_string(),
                pressure,
            ),
        ])
    }

    #[test]
    fn score_comes_from_named_metrics_only() {
        let values = metrics(1.04, 3.5e6);
        assert_eq!(score_of(Objective::MinFillTime, &values), Some(1.04));
        assert_eq!(
            score_of(Objective::MinInjectionPressure, &values),
            Some(3.5e6)
        );
        // 缺指标 → None（不猜、不填零）
        assert_eq!(score_of(Objective::MinFillTime, &BTreeMap::new()), None);
        // 非有限值同样当作缺指标（NaN 参与比较会污染「最优」评选）
        let nan = BTreeMap::from([(
            crate::services::moldingfoam::METRIC_FILL_TIME_S.to_string(),
            f64::NAN,
        )]);
        assert_eq!(score_of(Objective::MinFillTime, &nan), None);
    }

    #[test]
    fn grid_covers_ends_and_every_combination() {
        let points = grid_points(&[range("温度", 200.0, 240.0, 3), range("压力", 50.0, 70.0, 2)]);
        assert_eq!(points.len(), 6);
        assert!(points.iter().any(|point| point["温度"] == 200.0));
        assert!(points.iter().any(|point| point["温度"] == 240.0));
        assert!(points.iter().any(|point| point["压力"] == 70.0));
        assert!(
            points
                .iter()
                .any(|point| point["温度"] == 220.0 && point["压力"] == 50.0)
        );
    }

    #[test]
    fn refinement_shrinks_step_and_stays_in_range() {
        let ranges = [range("温度", 200.0, 240.0, 5)];
        let center = BTreeMap::from([("温度".to_string(), 210.0)]);
        let wide = refine_around(&center, &ranges, 0.5);
        // 中心 + 两个方向
        assert_eq!(wide.len(), 3);
        assert!(wide.iter().any(|point| point["温度"] == 205.0));
        assert!(wide.iter().any(|point| point["温度"] == 215.0));
        // 贴边时没有越界点（只保留中心与另一侧）
        let edge = BTreeMap::from([("温度".to_string(), 200.0)]);
        let points = refine_around(&edge, &ranges, 0.5);
        assert_eq!(points.len(), 2);
        assert!(points.iter().all(|point| point["温度"] >= 200.0));
    }

    #[test]
    fn search_refines_around_the_best_and_finds_the_optimum() {
        // 合成目标：fill_time = (温度-220)²/1000 + 1 —— 最优点在 220
        let ranges = vec![range("温度", 200.0, 240.0, 5)];
        let mut optimizer = Optimizer::new(ranges, Objective::MinFillTime, 64).unwrap();
        for _ in 0..8 {
            let candidates = optimizer.next_candidates();
            if candidates.is_empty() {
                break;
            }
            for candidate in candidates {
                let temperature = candidate["温度"];
                let score = (temperature - 220.0).powi(2) / 1000.0 + 1.0;
                optimizer.record(Evaluation {
                    parameters: candidate,
                    score: Some(score),
                    violations: Vec::new(),
                    attempts: 1,
                });
            }
        }
        let best = optimizer.best().expect("应有可行解");
        assert!((best.parameters["温度"] - 220.0).abs() < 0.5, "{best:?}");
    }

    #[test]
    fn infeasible_and_failed_runs_are_tracked_but_not_selected() {
        let ranges = vec![range("温度", 200.0, 240.0, 3)];
        let mut optimizer = Optimizer::new(ranges, Objective::MinFillTime, 16).unwrap();
        let mut candidates = optimizer.next_candidates();
        // 第一条：指标很好但违反约束 → 不算可行
        let violating = candidates.remove(0);
        assert!(
            optimizer
                .record(Evaluation {
                    parameters: violating,
                    score: Some(0.1),
                    violations: vec!["浇口速度超限".to_string()],
                    attempts: 1,
                })
                .is_none()
        );
        assert!(optimizer.best().is_none(), "违反约束的解不能成为最优");
        // 第二条：无指标（求解失败）
        let failed = candidates.remove(0);
        assert!(
            optimizer
                .record(Evaluation {
                    parameters: failed,
                    score: None,
                    violations: Vec::new(),
                    attempts: MAX_ATTEMPTS,
                })
                .is_none()
        );
        // 第三条：可行解 → 成为最优
        let good = candidates.remove(0);
        assert!(
            optimizer
                .record(Evaluation {
                    parameters: good,
                    score: Some(1.2),
                    violations: Vec::new(),
                    attempts: 1,
                })
                .is_none()
        );
        assert_eq!(optimizer.best().unwrap().score, Some(1.2));
        assert_eq!(optimizer.consecutive_failures(), 0);
    }

    #[test]
    fn three_consecutive_failures_abort_with_the_offending_parameters() {
        let ranges = vec![range("温度", 200.0, 240.0, 3)];
        let mut optimizer = Optimizer::new(ranges, Objective::MinFillTime, 16).unwrap();
        let candidates = optimizer.next_candidates();
        let mut abort = None;
        for (index, candidate) in candidates.into_iter().enumerate() {
            abort = optimizer.record(Evaluation {
                parameters: candidate,
                score: None,
                violations: Vec::new(),
                attempts: MAX_ATTEMPTS,
            });
            if abort.is_some() {
                break;
            }
            let _ = index;
        }
        let message = abort.expect("连续失败应中止");
        assert!(message.contains("连续 3 次"), "{message}");
        // 中止信息里必须能读到失败参数点（否则用户不知道去查哪一组）
        assert!(message.contains("温度="), "{message}");
    }

    #[test]
    fn accessors_and_exhaustion_paths_are_observable() {
        let ranges = vec![range("温度", 200.0, 240.0, 3)];
        let mut optimizer =
            Optimizer::new(ranges.clone(), Objective::MinInjectionPressure, 3).unwrap();
        assert_eq!(optimizer.objective(), Objective::MinInjectionPressure);
        assert_eq!(optimizer.used(), 0);
        assert_eq!(optimizer.history().len(), 0);
        // 预算 3：粗搜网格恰好 3 个点，用完即无候选
        let candidates = optimizer.next_candidates();
        assert_eq!(candidates.len(), 3);
        let mut abort = None;
        for candidate in candidates {
            abort = optimizer.record(Evaluation {
                parameters: candidate,
                score: Some(1.0),
                violations: Vec::new(),
                attempts: 1,
            });
        }
        assert!(abort.is_none());
        assert_eq!(optimizer.used(), 3);
        assert_eq!(optimizer.history().len(), 3);
        assert!(optimizer.next_candidates().is_empty(), "预算用尽后无候选");

        // 无任何可行解时（中心为空）也不产出细化点
        let mut no_center = Optimizer::new(ranges, Objective::MinFillTime, 8).unwrap();
        let candidates = no_center.next_candidates();
        for candidate in candidates {
            no_center.record(Evaluation {
                parameters: candidate,
                score: None,
                violations: Vec::new(),
                attempts: MAX_ATTEMPTS,
            });
        }
        assert!(no_center.next_candidates().is_empty());
    }

    #[test]
    fn refinement_skips_factors_missing_from_the_center() {
        let ranges = [range("温度", 200.0, 240.0, 5), range("压力", 50.0, 70.0, 5)];
        // 中心只带一个因子：缺键的因子直接跳过，不 panic
        let center = BTreeMap::from([("温度".to_string(), 210.0)]);
        let points = refine_around(&center, &ranges, 0.5);
        assert_eq!(points.len(), 3);
        assert!(points.iter().all(|point| !point.contains_key("压力")));
    }

    #[test]
    fn constructors_reject_unusable_ranges() {
        let objective = Objective::MinFillTime;
        assert!(Optimizer::new(Vec::new(), objective, 8).is_err());
        assert!(Optimizer::new(vec![range("a", 1.0, 1.0, 3)], objective, 8).is_err());
        assert!(Optimizer::new(vec![range("a", 0.0, 1.0, 1)], objective, 8).is_err());
        assert!(Optimizer::new(vec![range("a", f64::NAN, 1.0, 3)], objective, 8).is_err());
        let seven = (0..7)
            .map(|index| range(&format!("f{index}"), 0.0, 1.0, 2))
            .collect::<Vec<_>>();
        assert!(Optimizer::new(seven, objective, 8).is_err());
    }
}
