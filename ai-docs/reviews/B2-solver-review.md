# B2 求解器（Kairos 侧）评审 · 第 1 轮（T61 / T62）

- 日期：2026-09-12
- 范围：`architecture-status.md` B2/C4 下**非 moldingFoam** 的两项新工作
  ——T61 浇口几何落地、T62 工艺参数合理性校验；外加本日 e2e 真机验证的
  连带发现。
- 方法：按 [T21 清单](../tasks/T21-milestone-review.md) 八项逐项核对，
  消融抽查 6 点（T21 第 5 项要求）。

## 结论摘要

八项无「未检查」项；消融 6 点全部被测试当场捕获（其中 1 点暴露了集成用例的
断言缺口，已当场加固并复验「先红后绿」）。分类：**立即修 2 项**（已在本轮完成）、
**回流 0 项**、**接受 3 项**（阈值口径、模型边界、性能口径，均写明重评条件）。

## 八项清单

1. **架构一致性** ✓：新增的 `GatePortal` / `gate_portals` / `fill_load_hints`
   全部住 `kairos-core`（纯函数，无进程副作用）；`src-tauri` 命令只做入参拼装
   （`check_process` 合并 core 校验结果、`generate_moldingfoam_case` 调
   `gate_portals`）；core 无 tauri 依赖（grep 仅命中注释）；前端经
   `api → store → view` 分层传递上下文。本批未新增 `models/` DTO，故契约测试
   无需扩展（`FillLoadContext` 是前端 api 层局部入参类型，与既有
   `GenerateCaseInput` 同构）；`bun run verify` 全绿（含契约测试）。
2. **性能预算对照**：本批只改 case 生成阶段——边界分类由 O(B) 变为
   O(B log B)（仅在有浇口时对边界面排序；B = 边界面数，实测样例盒 1200、
   真实件 7956），占比相对网格生成可忽略（真实件 5 mm 档全流程 18 s，
   体素化占绝大部分；3 mm 档 46 s）。该路径不在 T01 基准套件覆盖范围
   （非热点路径），无需对照数字；**接受并记录**（重评条件：T01 增加 case 生成
   基准项时补测）。
3. **代码与注释质量** ✓：新增文件无 TODO/FIXME；注释解释「为什么」（如
   `NORMAL_Z_MIN` 说明阶梯侧向面的来源与实测 39% 的误差、`GATE_MIN_RADIUS_MM`
   说明「入口不能为空」的原因、`MACHINE_FLOW_LIMIT_CM3_S` 写明实测依据）；
   命名与既有风格一致（常量 SCREAMING_SNAKE、测试名描述行为）。
4. **文档同步** ✓：`architecture-status.md` C4 两条目改为已完成并回填实测数字；
   任务索引增 T61/T62；`ARCHITECTURE.md` 不涉及 case 的 inlet/浇口设计（grep
   无命中），无需同步；模块头注释（`moldingfoam.rs`）已写明 inlet 的两种来源。
5. **测试覆盖缺口** ✓ + **消融抽查**（下表）：行覆盖 100%（`coverage:rust` 门禁
   通过），消融 6 点全部捕获。
6. **依赖健康** ✓：无新增依赖（`knip` 全绿；core/tauri/前端未引入任何新 crate
   或 npm 包）；许可证面无变化（未引入第三方代码）。
7. **安全** ✓：`check_process` 仅新增两个可选数值入参（无新权限面）；capabilities、
   CSP 未改动；无新增网络/文件访问。
8. **上轮遗留追踪**：M6 评审「接受」项里的 **`Duplicate entry … in runtime
selection table` 警告**，2026-09-12 已证明不是「非致命噪音」而是 bundle
   打包缺陷（同一求解模块两份 `.so` 同时加载）的表征——退出期堆破坏的根因即在
   此处（详见 [e2e 报告](e2e-solve-report.md) §4），bundle v0.2.1 已修复。
   M6 的其余接受项（`style-src unsafe-inline`、CLI 单测以冒烟替代）状态不变。

## 消融抽查（6 点，全部捕获）

| #   | 消融点                                | 期望变红的用例                                                                                                   | 结果             |
| --- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------- |
| 1   | `gate_portals` 取 `start` 端（错端）  | `gate_portals_take_cavity_end_of_gate_units_only`                                                                | ✅ 红 → 还原后绿 |
| 2   | `NORMAL_Z_MIN` 置 0（关掉外法向过滤） | `boundary_face_classification_needs_outward_normal`、`degenerate_face_normal_stays_zero_and_classifies_as_walls` | ✅ 2 例红        |
| 3   | 去掉浇口等效面积补足（只认半径内）    | `gate_inlet_faces_fill_up_to_equivalent_area`                                                                    | ✅ 红            |
| 4   | 流量包络阈值放宽 10 倍                | `fill_load_hints_flag_order_of_magnitude_mismatch`                                                               | ✅ 红            |
| 5   | 浇口剪切公式丢掉 4/π                  | `estimate_fill_load_scales_with_volume_time_and_gate`                                                            | ✅ 红            |
| 6   | 给了浇口仍让底带算入口                | `write_poly_mesh_puts_gate_faces_on_inlet_patch`                                                                 | ✅ 红            |

**缺口与加固**：第 3 点起初只被单元用例捕获，集成用例（写出的 polyMesh）没有
断言「入口等效面积 ≥ πr²」——补上该断言后复验：消融 3 同样使集成用例变红
（`入口等效面积 4 小于 πr²`），还原后绿。加固随本轮入库。

## 发现分类

- **立即修（2 项，已随本轮入库）**：
  1. 集成用例补「入口等效面积 ≥ πr²」断言（消融 3 暴露）；
  2. T21 清单新增第 9 项「外部产物信号必须跟进到根因」（由第 8 项教训固化）。
- **回流任务**：无。
- **接受并记录（3 项）**：
  1. 阈值为量级口径（500 cm³/s、5×10⁴ 1/s），非机型精确包络——重评条件：接入
     真实机型参数库（锁模力/螺杆/最大注射速率）后按机型判定；
  2. 不估填充压力（简单润滑模型无法可靠预测体素化薄通道下的 10² MPa 量级）——
     重评条件：moldingFoam 侧给出压力/稳定性的可解析判据；
  3. case 生成的性能不在 T01 基准覆盖内（见第 2 项）。

## 与下轮的衔接

第 2 轮（T63/T64 完成后）需复评：本轮「接受」三项的重评条件是否触发；T63 的
最小特征提示与 T62 的工况校验是否有重叠口径需要合并；T64 引入视口形变后的
渲染性能与 T48 的关系。
