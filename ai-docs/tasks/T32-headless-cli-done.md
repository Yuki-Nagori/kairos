# T32 · 无头 CLI 与批处理入口（门面式）

- 阶段：已完成（kairos-cli crate + pipeline run 编排；CI 内端到端求解依赖 OpenFOAM 环境，按 T29 约定随真机验证推进）
- 依赖：T25 研究（建议 #1 采纳）
- 优先级：P2

## 目标

提供无 GUI 的 `kairos-cli` 二进制（复用 kairos-core，零 Tauri 依赖）：单门面 + 按域子命令（project / mesh / solve / results），支撑批处理与自动化冒烟——这是 T25 对照研究识别的最大缺口（多研究编排入口）。

## 范围

- `src-crates/kairos-cli`（新 crate，workspace 成员）：clap 子命令
  `project new/open`、`mesh generate --engine voxel|gmsh`、`solve submit --cores --stage`、`results list/dump`；
- 编排子命令 `pipeline run`：单条命令跑完 样例/指定 STL → 网格 → case → 求解 → 结果汇总（T25 对照表的批处理缺口）；
- 错误输出沿用 `{code, message}` 结构化契约（T25 结论：原样穿透）；
- JSON 输出模式（`--json`）供脚本消费。

## 非目标

- Python 绑定（CLI 稳定后再评估）;
- 交互式 TUI。

## 交付物

- kairos-cli crate + pipeline run 编排命令；
- README 补 CLI 用法段。

## 验收标准

- `kairos-cli pipeline run --sample-box` 在无 GUI 环境（CI Ubuntu）端到端跑通；
- 子命令错误均输出结构化 code；
- kairos-core 不新增 tauri 依赖（GPL 隔离与分层铁律不破）。
