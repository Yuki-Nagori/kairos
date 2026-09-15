# T103 · GUI 网格与工艺入口对齐

- 阶段：E3
- 状态：进行中
- 目标：让 GUI 暴露 CLI 已稳定的网格路线和 Mug 基线工艺入口，避免只能通过 CLI 才能复现实验。

## 已完成

- 工艺面板支持完整保压曲线输入（`时间=压力`，逗号分隔）。
- 内置 Mug 基线预设：220°C 熔体、50°C 模具、5.5 s 注射、20 s 冷却。
- 几何面板可生成体积网格、Dual Domain 网格和中面网格，并显示质量摘要。
- 方案任务面板可选择 `fill`、`fill-pack`、`fill-pack-cool` 阶段。

## 待完成

1. GUI 提供体积网格与 Dual Domain 输入 JSON 的导出入口，复用保存对话框和 Rust 序列化契约。
2. GUI 显示当前网格类型、schema 版本、节点/单元数量和单位，提交求解前阻止网格类型与 solver 能力不匹配。
3. Mug 基线预设与 `ai-docs/report/mug-baseline.md` 做单一来源校验，避免界面默认值漂移。
4. Web 端回归覆盖上述入口；真实 Tauri 文件对话框只补一条桌面冒烟。

## 验收

- GUI 可生成并导出两种 Mug 网格输入；导出 JSON 通过 Rust 契约解析。
- 选择 Dual Domain 时明确提示当前 moldingFoam 只支持三维体网格；不得静默降级。
- Mug 基线工艺从 GUI 应用后，与 CLI 生成的 `ProcessSettings` 逐字段一致。
