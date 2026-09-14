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
        if !(channel.mass_flow_rate_kg_s.is_finite() && channel.mass_flow_rate_kg_s > 0.0) {
            issues.push(format!(
                "水路「{}」质量流量必须为正数（kg/s）——模壁 1D 通道 BC 需要它算取热。",
                channel.id
            ));
        }
        if !(channel.specific_heat_j_kg_k.is_finite() && channel.specific_heat_j_kg_k > 0.0) {
            issues.push(format!(
                "水路「{}」介质比热必须为正数（J/kg/K，水默认 4180）。",
                channel.id
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

    // 3. 截面匹配：浇口直径不应大于它所在的流道直径
    //    （反了说明参数填错：熔体先收缩进浇口再放大，填充会先在流道处失速）
    for gate in runners.iter().filter(|item| item.kind == RunnerKind::Gate) {
        let attached = runners.iter().find(|other| {
            other.kind == RunnerKind::Runner
                && ([other.start, other.end].contains(&gate.start)
                    || [other.start, other.end].contains(&gate.end))
        });
        if let Some(channel) = attached
            && gate.diameter_mm > channel.diameter_mm + WELD_TOLERANCE
        {
            issues.push(format!(
                "浇口「{}」直径 {:.2} mm 大于所在流道「{}」的 {:.2} mm：截面先收缩再放大，填充会先在这里失速。",
                gate.id, gate.diameter_mm, channel.id, channel.diameter_mm
            ));
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 问题清单里是否出现某个关键词（放在测试模块里，避免断言内联闭包被算作未覆盖）。
    fn has(issues: &[String], needle: &str) -> bool {
        issues.iter().any(|issue| issue.contains(needle))
    }

    fn runner(id: &str, start: [f64; 3], end: [f64; 3]) -> RunnerElement {
        RunnerElement {
            id: id.into(),
            kind: RunnerKind::Runner,
            diameter_mm: 6.0,
            start,
            end,
            medium: None,
        }
    }

    fn gate(id: &str, start: [f64; 3], end: [f64; 3]) -> RunnerElement {
        RunnerElement {
            id: id.into(),
            kind: RunnerKind::Gate,
            diameter_mm: 1.5,
            start,
            end,
            medium: None,
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
            ..CoolingChannel::default()
        }];
        let issues = check_mold_network(&[], &channels);
        assert!(issues.iter().any(|issue| issue.contains("超出合理范围")));
    }

    /// 冷却水路的口径校验：质量流量与比热缺项（旧文件迁移后为 0）必须提示。
    #[test]
    fn channel_missing_coolant_parameters_are_reported() {
        let channels = vec![CoolingChannel {
            id: "c1".into(),
            diameter_mm: 8.0,
            start: [0.0, 0.0, 0.0],
            end: [100.0, 0.0, 0.0],
            inlet_temp_c: 25.0,
            mass_flow_rate_kg_s: 0.0,
            specific_heat_j_kg_k: 0.0,
        }];
        let issues = check_mold_network(&[], &channels);
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("质量流量必须为正数")),
            "{issues:?}"
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("介质比热必须为正数")),
            "{issues:?}"
        );

        // 补齐口径后不再报这两项（默认构造即水口径）
        let fixed = vec![CoolingChannel::default()];
        let issues = check_mold_network(&[], &fixed);
        assert!(!issues.iter().any(|issue| issue.contains("质量流量")));
        assert!(!issues.iter().any(|issue| issue.contains("介质比热")));
    }

    #[test]
    fn gate_wider_than_its_runner_is_reported() {
        // 浇口截面大于所在流道：参数填错的典型形态，必须给出提示
        let mut wide = gate("g1", [0., 0., 0.], [0., 0., 5.]);
        wide.diameter_mm = 8.0; // 流道 6.0 mm
        let channel = runner("r1", [0., 0., 0.], [0., 0., -10.]);
        let issues = check_mold_network(&[channel.clone(), wide.clone()], &[]);
        assert!(has(&issues, "大于所在流道"), "{issues:?}");
        // 浇口从另一端接上流道（端点匹配走 gate.end 分支）
        let mut tail = gate("g2", [0., 0., -3.], [0., 0., -10.]);
        tail.diameter_mm = 8.0;
        let issues = check_mold_network(&[channel.clone(), tail], &[]);
        assert!(has(&issues, "大于所在流道"), "{issues:?}");
        // 截面不超限与孤立浇口：都不应误报截面问题
        let mut narrow = wide;
        narrow.diameter_mm = 4.0;
        let issues = check_mold_network(&[channel.clone(), narrow], &[]);
        assert!(!has(&issues, "大于所在流道"), "{issues:?}");
        let mut lonely = gate("g3", [50., 0., 0.], [50., 0., 5.]);
        lonely.diameter_mm = 9.0;
        let issues = check_mold_network(&[channel, lonely], &[]);
        assert!(!has(&issues, "大于所在流道"), "{issues:?}");
    }
}
