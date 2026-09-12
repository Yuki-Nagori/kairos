# T61 · 浇口几何落地（模具网络 → case inlet）

- 阶段：B2/C4（求解器 · 流动分析的 Kairos 侧）
- 依赖：T07（流道/浇口网络模型与面板）、T36（case 契约与生成器）
- 优先级：**P1**（C4「多型腔 / 流道系统参与填充」的前置，也是 T55 建议浇口
  可用化的前置）

## 目标

让 case 的 `inlet` 由**浇口几何**决定，而不是包围盒 z 分带启发式：

- 研究上已有浇口（`RunnerElement{kind: Gate}`）时，`inlet` patch 取浇口
  与型腔相接处（约定 `end` 端）邻域的边界面，等效流通面积按 `πr²` 保底；
- 没有浇口时保留分带回退，但**只认朝下/朝上的面**——体素网格的边界是阶梯面，
  底/顶带里混着大量侧向面，把它们计入 inlet/vent 会让熔体从零件外壁注入
  （真实件实测：入口面 39% 是侧向面，见 e2e 报告 §5）。

## 范围

1. core（`services/moldingfoam.rs`）：
   - `GatePortal { center, radius_mm }` + `gate_portals(&[RunnerElement])`
     （纯函数：只取 Gate 单元、入口取 `end` 端、半径 = 直径/2 且有下限）；
   - `gate_inlet_faces(...)`：半径内的边界面全取，等效面积不足时按距离从近到远
     补足（体素网格上浇口常小于一个单元，没有补足会出现「入口为空」的 case）；
   - `classify_boundary_face(z, normal_z, z_min, z_max)`：分带回退加外法向
     过滤（`|n_z| ≥ 0.5` 才算朝下/朝上）；
   - `write_poly_mesh` / `generate_case` 接受 `gates: &[GatePortal]`；
     给了浇口时底带不再算入口（避免「浇口 + 整条底带」双重入口）。
2. 适配层：`generate_moldingfoam_case` 新增 `runner_elements` 入参，
   经 `gate_portals` 落成 case；前端 `submitPipeline` 透传研究的
   `runnerElements`。
3. CLI（无头验证入口）：`pipeline run --gate x,y,z[,半径]`（可重复，
   缺省半径 2 mm），供真实件按浇口跑。
4. 文档：本任务文件、任务索引、architecture-status 的 C4 条目与 timeline。

## 非目标

- 流道**体**参与网格（把浇口/流道几何切成单元）——本次只做 inlet patch
  的定位，流道体积网格化留给后续（依赖更完整的流道几何与网格策略）；
- 多型腔自动分配（每个腔一个浇口）——`gates` 已支持多个浇口，分配策略
  与工艺侧的平衡设计不在本次范围；
- 浇口位置**分析**（推荐浇口）——那是 T55。

## 验收标准

- 有浇口时 `inlet` patch 只含浇口邻域的面（面数显著少于分带回退、
  且面心都在浇口附近），等效面积不小于 `πr²`；
- 无浇口时 `inlet`/`vent` 不再包含阶梯侧向面（样例方盒底面 240 面 → 200 面）；
- 样例方盒：checkMesh 通过（inlet 200 / vent 200 / walls 800，阶梯侧向面
  按设计归 walls）✅；**填充回归在 bundle v0.2.1 上出现求解侧失稳**（见下
  「真机回归发现」），按约定记录并移交 moldingFoam 侧，不计为本任务缺陷；
- 真实件（Thingiverse 控制器支架，5 mm 体素）带浇口时 `inlet` 收缩为
  浇口邻域（实测 382 面 → 21 面）；
- 纯函数单测覆盖浇口映射、法向过滤、面积补足与写出一致性；
  `bun run verify` 全绿；契约测试不受影响（无新 DTO，`RunnerElement` 已存在）。

## 真机回归发现（2026-09-12，bundle v0.2.1）

改动后样例方盒在 t ≈ 0.5 s 起失稳（能量方程先出 NaN：
`smoothSolver: Solving for T … Final residual = nan`，随后全场 NaN）。判别实验：

| 配置（同一 VM / bundle v0.2.1）            | 结果                        |
| ------------------------------------------ | --------------------------- |
| 改动前 case（inlet = 底带含侧向面 240 面） | 跑满 1.97 s，exit 0、nan=0  |
| 改动后 case（inlet = 底面 200 面）         | t ≈ 0.49 s 失稳             |
| 改动后 + 真实浇口（16 面 inlet）           | t ≈ 0.64 s 失稳             |
| 改动后 + 旧 vent 行为（只改 inlet 过滤）   | t ≈ 0.51 s 失稳（同样 NaN） |
| 改动前 case + `maxAlphaCo` 0.03            | t ≈ 0.46 s 失稳             |
| 改动前 case + endTime 6 s                  | t ≈ 2.05 s 失稳             |

结论：这个 case（10 单元高方块、1 s 注满、整面进料）本就坐在稳定性边界附近——
同一族配置在不同微扰下分别在 0.46 / 0.49 / 0.64 / 2.05 s 失稳。本次改动把底带
的 40 个阶梯侧向面从 inlet 移到 walls，同时把这些面的 T 边界从熔温（503 K）
换成模温（313 K）——浇口附近的冷壁面积显著增加，与「能量方程先崩」的现象一致。
角点高速区（~25 m/s）在改动前后的两个 case 里都存在，不是本次引入。

处置：**记录并移交 moldingFoam 侧**（求解器鲁棒性：整面/小浇口 + 冷壁的
能量方程稳定性），Kairos 侧不改物理正确的分带过滤。样本 case 的稳定填充回归
待上游修复后用 `--gate 5,5,0,1.5` 配置复跑确认。

## 当前实现边界

- 浇口只影响 `inlet` patch 的位置与面积，**不改变网格**：流道/浇口几何本身
  仍不在网格里，熔体从浇口所在的面进入型腔（等价于「浇口贴着型腔表面」）；
- 分带回退仍是轴向模具假设（`classify_boundary_face`），浇口与排气口的
  精确位置只有给了网络才有意义；
- `end` 端的语义是约定（Gate 的 `start` 接流道、`end` 接型腔），面板文案
  与模型注释同步说明。
