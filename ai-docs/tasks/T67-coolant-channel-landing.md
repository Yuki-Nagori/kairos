# T67 · 冷却水路落地（模壁 1D 通道 BC → case）

- 阶段：B2/C5（求解器 · 冷却分析的 Kairos 侧）
- 依赖：T07（水路网络模型与面板）、T66（case 单位制与 vent 对齐）
- 优先级：**P2**
- 状态：**待开工**（契约已定，见下）

## 目标

把研究的冷却水路（`CoolingChannel`：直径 / 起止 / 介质温度）落进 case，
使冷却阶段用上游的**模壁 1D 通道模型**求解瞬态传热，而不是现在的恒温模壁。

## 契约（2026-09-13 上游确认）

模壁 patch 的 T 边界改用 `moldingMoldTemperature`，通道参数写在 `coolant`
子字典：

```
coolant
{
    massFlowRate     0.05;      // [kg/s]
    cp               4180;      // [J/kg/K]
    inletTemperature 293.15;    // [K] 介质入口温度（面板字段）
    direction        (1 0 0);   // 通道轴向
    htc              5000;      // [W/m^2/K]
    // 或改用 Nu 相关式：Nu { C 0.023; m 0.8; n 0.4; Re 6000; Pr 7; k 0.6; D 0.008; }
}
```

该 BC 逐步求解 1D 活塞流能量平衡（上游 `validation/coolantMold`、`moldCHT`
套件已验收）；水路取热的瞬态演化天然包含。三维水路 CHT 是另一条路径
（独立 `moldingCoolantFluid` 区域 + `coupledTemperature`），需要真实水流场时再上。

## 范围

1. **模型补字段**：`CoolingChannel` 目前只有直径 / 起止 / 介质温度，契约还需要
   `massFlowRate` 与 `cp`（水默认 4180）——补 DTO（双端镜像 + 契约测试）与
   模具网络面板字段；
2. **case 生成**：有通道时把「润湿的模壁面」的 T BC 写成 `moldingMoldTemperature`
   - `coolant` 子字典（通道轴向由起止点给出，`D` 取直径，`htc` 或 Nu 参数由
     面板/默认给出）；无通道时保持现行恒定模温。
     **待定设计**：Kairos 的网格只有制品（没有模具域），"润湿模壁 patch" 只能用
     通道附近的制品壁面近似——需要决定影响半径与 patch 归属（按通道到面心距离
     分区，落到 `walls` 面集上再拆分 patch），或先做"整壁共用一条通道参数"的简化；
3. **校验**：通道与制品的连通性/覆盖（无通道、通道离件过远、参数缺项）；
4. 文档与状态：architecture-status 的 C5 条目、任务索引、timeline。

## 非目标

- 三维水路 CHT（需要模具域网格与水场，属更大范围）；
- 冷却时间的工艺优化。

## 验收标准

- 有通道时 case 的模壁 T BC 为 `moldingMoldTemperature` + `coolant`（字段齐全、
  单位 SI），无通道时行为不变；
- 真机跑一轮冷却阶段（Fill+Pack+Cool），结果可读且无 NaN；
- DTO 双端镜像 + 契约测试、面板字段测试；
- `bun run verify` 全绿。

## 当前实现边界

- 制品域近似模壁（同一几何），不建模模具本体；
- `htc` 与 Nu 参数先给工程默认值，材料/机型库接入后细化。
