# T66 · case 单位制与排气边界对齐契约（SI + moldingVent）

- 阶段：B2/C4（Kairos 侧 case 生成）
- 依赖：T36（case 契约）、T61（浇口几何落地）
- 优先级：**P1**（对齐修复：旧配置下样例 case 填充停滞且求解失稳）

## 目标

把 case 生成从「mm 数值直写 + 通用排气边界」改成契约与上游参考用例的形态：

- 坐标写米（mm × 1e-3）——契约的 `blockMeshDict` 是「mm 顶点 + `scale 0.001`」，
  Kairos 直接写 polyMesh，换算必须自己做；
- 注射流量用 SI（型腔体积 m³ / 注射时间）；
- 排气口用 `moldingVentVelocity`（U）与 `moldingVentPressure`（p_rgh，`p0 1e5` +
  `CdA` = 排气 patch 几何面积），配合 `ventSealAlpha` 构成密封逻辑；
- `moldingDict` 打开质量预算（逐 patch 打印熔体通量，填充停滞/逃逸可见）。

## 依据

旧配置的两处偏差（诊断来自上游 [#038]）：

1. 排气口是通用开放边界（`pressureInletOutletVelocity` + `prghTotalPressure`），
   求解器的 `ventSealAlpha` 密封逻辑从不生效 → 熔体前沿触达后大量排出（实测
   入口流量的 92% 从排气口逃逸），填充分数卡在 0.75、V/P 切换永不触发；
2. mm 数值直写让 10 mm 件在求解器里成为 10 m 立方体，与 SI 物性组合后整体
   放大千倍（静水压主导，前沿/逃逸都不是目标件的物理）。

上游参考用例（SI + moldingVent + `ventSealAlpha`）实测：V/P 切换 t=1.177 s
（填充 0.9614）、最终填充 0.9786、无 NaN。

## 交付

core（`services/moldingfoam.rs`）：

- `MM_TO_M` 与 `write_poly_mesh` 写米坐标，并返回 `PatchAreas`
  （inlet/vent/walls 的 SI 面积）；
- `u_dict`：vent → `moldingVentVelocity`；流量 = V_SI / 注射时间；
- `p_rgh_dict(cda_m2)`（原 const 改函数）：vent → `moldingVentPressure` +
  `p0 1e5` + `CdA`；
- `molding_dict`：新增 `ventSealAlpha 0.9` 与 `massBudget true`
  （`massBudgetInterval 200`）；
- `generate_case` / `write_case_files` 串上 `PatchAreas`。

## 验收标准

- 单测：写出点为米、patch 面积为 SI、浇口入口按米比对、vent BC 与
  `ventSealAlpha` 出现在字典里 ✅；
- 真机（bundle v0.2.1，样例方盒）：`checkMesh` 包围盒 0 → 0.01 m 且 Mesh OK；
  求解 nan=0、V/P 切换在 t ≈ 1.16 s 触发（填充 0.95）、t = 2 s 填充 0.9731、
  整链 exit 0 ✅；
- 与上游参考用例对照：切换填充分数 0.95 vs 0.9614、最终填充 0.9731 vs 0.9786
  （差异来自网格与配置细节）✅；
- `bun run verify` 全绿。

## 非目标

- 排气口几何：仍按分带（无浇口时）/浇口回退的启发式确定 patch，`CdA` 取该
  patch 的几何面积，不做物理排气口建模；
- 真实件的工艺参数匹配（由工况校验提示，见 T62）；
- 网格加密策略（上游提示 10³ 粗网格会有数值膜逃逸，加密属 T43/T57 范围）。
