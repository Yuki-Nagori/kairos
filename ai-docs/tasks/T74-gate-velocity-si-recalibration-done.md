# T74 · 浇口速度阈值按 SI 重标定对齐（fillVelocityWarn 5 → 20）

- 阶段：B2（求解器 · 工艺判据）
- 依赖：T62（填充工况量级校验）、T66（case 单位制改 SI）
- 优先级：**P2**（误报会让用户去调本来正确的浇口/流量）
- 状态：**已完成**

## 现象

上游 2026-09-13 通告：`fillVelocityWarn` 缺省由 5 m/s 改为 **20 m/s**，
理由是旧值在 mm-as-m 口径下标定；SI 口径下真实浇口速度 ≤ 数 m/s，
只有 >20 m/s 才指示浇口面积 / 流量错配。

Kairos 侧 `services/process.rs` 仍是旧口径：

```rust
const GATE_VELOCITY_CAUTION_M_S: f64 = 5.0;   // 谨慎
const GATE_VELOCITY_LIMIT_M_S: f64 = 20.0;    // 不可行
```

即「>5 m/s 谨慎」这条在 SI 下会误报（本轮真实件的名义入口速度 0.18 m/s、
点浇口样例盒也在数 m/s 量级，本不该提示）。

## 范围（已完成）

1. 阈值收敛为**单档告警**：`GATE_VELOCITY_LIMIT_M_S = 20`（与上游
   `fillVelocityWarn` 缺省同口径），删除 5 m/s 的「谨慎」档——SI 下真实浇口速度
   ≤ 数 m/s，旧档会误报；
2. 提示文案改为「浇口面积 / 流量错配」口径，并点出「网格没表达浇口时名义速度会
   被拉低」（与 T72 的有效面积回显呼应）；
3. `FillLoad.inlet_velocity_m_s` 仍用**实测**入口面积（CLI 传 case 里 inlet patch
   面积），口径未变；
4. 单测：小浇口（8.8 m/s）不再报警、20 m/s 两侧边界（19.5 静默 / 20.5 报警）。

## 非目标

- 浇口剪切速率阈值（另有量纲，不受本次重标定影响）；
- 上游求解器侧的实现。

## 验收标准

- 阈值与上游 `fillVelocityWarn` 一致，边界用例覆盖；
- `bun run verify` 全绿。
