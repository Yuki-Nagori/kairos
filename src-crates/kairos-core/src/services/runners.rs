//! 模具网络领域服务：流道 / 浇口 / 冷却水路的校验与连通性检查。
//! 校验返回问题清单（而非首个错误），供前端错误面板整体呈现。

use std::collections::HashMap;

use crate::models::runners::{CoolingChannel, RunnerElement, RunnerKind};

/// 端点焊接容差（相对坐标尺度的固定值；模型单位为 mm，0.01 mm 足够）。
const WELD_TOLERANCE: f64 = 0.01;

fn element_issues(prefix: &str, diameter_mm: f64, start: &[f64; 3], end: &[f64; 3]) -> Vec<String> {
    let mut issues = Vec::new();
    if !(diameter_mm.is_finite() && diameter_mm > 0.0) {
        issues.push(format!("{prefix}直径必须为正数。"));
    }
    let len: f64 = start
        .iter()
        .zip(end.iter())
        .map(|(a, b)| (b - a) * (b - a))
        .sum::<f64>()
        .sqrt();
    if !len.is_finite() || len <= WELD_TOLERANCE {
        issues.push(format!("{prefix}起点与终点重合（单元长度为零）。"));
    }
    if start.iter().any(|v| !v.is_finite()) || end.iter().any(|v| !v.is_finite()) {
        issues.push(format!("{prefix}坐标含非法数值。"));
    }
    issues
}

/// 检查流道 / 浇口 / 冷却水路网络：单元参数 + 连通性（孤立单元即错误）。
/// 返回问题清单；空清单 = 校验通过。
pub fn check_mold_network(runners: &[RunnerElement], channels: &[CoolingChannel]) -> Vec<String> {
    let mut issues = Vec::new();

    // 1. 单元自身参数
    for element in runners {
        let kind_label = match element.kind {
            RunnerKind::Gate => "浇口",
            RunnerKind::Runner => "流道",
        };
        issues.extend(element_issues(
            &format!("{kind_label}「{}」", element.id),
            element.diameter_mm,
            &element.start,
            &element.end,
        ));
    }
    for channel in channels {
        issues.extend(element_issues(
            &format!("水路「{}」", channel.id),
            channel.diameter_mm,
            &channel.start,
            &channel.end,
        ));
        if !(-20.0..=200.0).contains(&channel.inlet_temp_c) {
            issues.push(format!(
                "水路「{}」入口温度 {}°C 超出合理范围（-20 ~ 200）。",
                channel.id, channel.inlet_temp_c
            ));
        }
    }

    // 2. 流道网络连通性：端点焊接成图，孤立单元（两端都只连自己）报错
    let mut degree: HashMap<[i64; 3], usize> = HashMap::new();
    let weld = |p: &[f64; 3]| -> [i64; 3] {
        [
            (p[0] / WELD_TOLERANCE).round() as i64,
            (p[1] / WELD_TOLERANCE).round() as i64,
            (p[2] / WELD_TOLERANCE).round() as i64,
        ]
    };
    let mut element_nodes: Vec<([[i64; 3]; 2], String)> = Vec::new();
    for element in runners {
        let (s, e) = (weld(&element.start), weld(&element.end));
        *degree.entry(s).or_insert(0) += 1;
        *degree.entry(e).or_insert(0) += 1;
        element_nodes.push(([s, e], element.id.clone()));
    }
    // 浇口被视为网络端点：浇口只要求单端连接流道网络
    for (nodes, id) in &element_nodes {
        let isolated = nodes.iter().all(|node| degree.get(node) == Some(&1));
        if isolated {
            issues.push(format!(
                "流道「{id}」是孤立单元，未与任何其他流道或浇口连通。"
            ));
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runner(id: &str, start: [f64; 3], end: [f64; 3]) -> RunnerElement {
        RunnerElement {
            id: id.into(),
            kind: RunnerKind::Runner,
            diameter_mm: 6.0,
            start,
            end,
        }
    }

    fn gate(id: &str, start: [f64; 3], end: [f64; 3]) -> RunnerElement {
        RunnerElement {
            id: id.into(),
            kind: RunnerKind::Gate,
            diameter_mm: 1.5,
            start,
            end,
        }
    }

    #[test]
    fn clean_network_has_no_issues() {
        // 主流道 → 分流道 → 浇口：手工连通
        let runners = vec![
            runner("r1", [0.0, 0.0, -10.0], [0.0, 0.0, 0.0]),
            runner("r2", [0.0, 0.0, 0.0], [20.0, 0.0, 0.0]),
            gate("g1", [20.0, 0.0, 0.0], [25.0, 0.0, 0.0]),
        ];
        assert!(check_mold_network(&runners, &[]).is_empty());
    }

    #[test]
    fn isolated_runner_is_reported() {
        let runners = vec![
            runner("r1", [0.0, 0.0, -10.0], [0.0, 0.0, 0.0]),
            runner("bad", [50.0, 50.0, 50.0], [60.0, 50.0, 50.0]),
        ];
        let issues = check_mold_network(&runners, &[]);
        assert!(issues.iter().any(|issue| issue.contains("bad")));
    }

    #[test]
    fn zero_length_and_bad_diameter_are_reported() {
        let mut bad = runner("r1", [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        bad.diameter_mm = -1.0;
        let runners = vec![bad];
        let issues = check_mold_network(&runners, &[]);
        assert!(issues.iter().any(|issue| issue.contains("直径")));
        assert!(issues.iter().any(|issue| issue.contains("重合")));
    }

    #[test]
    fn channel_temperature_range_is_checked() {
        let channels = vec![CoolingChannel {
            id: "c1".into(),
            diameter_mm: 8.0,
            start: [0.0, 0.0, 0.0],
            end: [100.0, 0.0, 0.0],
            inlet_temp_c: 500.0,
        }];
        let issues = check_mold_network(&[], &channels);
        assert!(issues.iter().any(|issue| issue.contains("超出合理范围")));
    }
}
