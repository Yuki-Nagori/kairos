# Mug 基线与实验对照

- 状态：T101 核心材料闭环已完成；Mug 对照持续进行中
- 关联任务：[T100](../tasks/T100-solver-runtime-v110-validation.md)、[T101](../tasks/T101-generic-pp-material-fit.md)
- 模型：`report/mug-moldflow/mug.stl`（本地受控资产）
- 报告建立时间：2026-09-15

## 1. 目标

建立同一 Mug 模型在 Moldflow、Kairos 和实物实验之间的可追溯比较。第一阶段验证填充和保压量级，第二阶段再验证冷却、收缩和翘曲。网格不同或边界条件不同的结果只做量级比较，不宣称逐点等价。

## 2. 已冻结的参考工况

| 项目 | 参考值 |
| --- | ---: |
| 熔体温度 | 220 °C |
| 模具表面温度 | 50 °C |
| 注射时间 | 5.5 s |
| 名义流量 | 211.5792 cm³/s |
| 冷却时间 | 20 s |
| 参考模型体积 | 1163.6855 cm³ |
| Moldflow 网格 | Dual Domain；33418 nodes；66830 triangles |

参考报告中的观测量：填充结束 5.7916 s、最大注射压力 2.5106 MPa、制件质量 899.7875 g、最大锁模力 6.5605 metric tons、平均填充温度 218.9642 °C、最大剪切应力 0.1552 MPa、最大剪切速率 1.0061×10⁴ 1/s。以上数值是比较基线，不代表 Kairos 已通过验证。

## 3. 材料参数覆盖与缺口

### 已覆盖的主要输入

- Cross-WLF：`n`、`tauStar`、`D1`、`D2`、`D3`、`A1`、`A2`。
- PVT/Tait：熔态/固态比容系数、压力项和转变温度。
- 热性能：比热容表、导热系数表。
- 机械性能：弹性模量、泊松比字段可进入材料 DTO，为翘曲分析预留。

### 不能由这些参数单独确定的因素

- 真实浇口、流道、排气和注射机边界。
- 网格类型、单元尺寸、厚度方向分辨率和接触换热。
- PP 的结晶动力学、结晶收缩、取向冻结和材料批次差异。
- 模具温控、脱模约束、后收缩时间及测量基准。

因此，热/机械/PVT/Cross-WLF 数据足以建立工程级第一版材料模型，但不能保证尺寸或翘曲结果自动准确。需要通过实验对照校准工艺和模型假设。

## 3.1 Kairos 自定义材料快照

本次本地拟合输出的材料版本为 `PP-REF-01`（仅记录结果参数，原始曲线文件不入库）：

| 组别 | 参数 | 值 | 单位/说明 |
| --- | --- | ---: | --- |
| Cross-WLF | `n` | 0.32 | 无量纲 |
| Cross-WLF | `tauStar` | 20,000 | Pa |
| Cross-WLF | `D1` | 3.6231730884×10¹³ | Pa·s |
| Cross-WLF | `D2` | 263.0 | K |
| Cross-WLF | `D3` | 0 | K |
| Cross-WLF | `A1` | 31.4 | 无量纲 |
| Cross-WLF | `A2` | 51.6 | K |
| Tait | `b1m / b1s` | 1.3871717803×10⁻³ / 1.2145486190×10⁻³ | m³/kg |
| Tait | `b2m / b2s` | 1.0×10⁻⁷ / 5.0×10⁻⁸ | m³/(kg·K) |
| Tait | `b3 / b3s` | 1.0×10⁸ / 1.0×10⁸ | Pa |
| Tait | `b4 / b4s` | 3.0×10⁻³ / 1.5×10⁻³ | 1/K |
| Tait | `b5 / b6` | 400 / 0 | K；K/Pa |
| Tait | `C / smoothBand` | 0.0894 / 0.5 | 无量纲；K |
| 热性能 | 比热 | (300,1900)、(400,2300)、(500,2500) | K；J/(kg·K) |
| 热性能 | 导热率 | (300,0.22)、(400,0.20)、(500,0.18) | K；W/(m·K) |
| 机械 | 弹性模量 / 泊松比 | 1.5×10⁹ / 0.35 | Pa；无量纲 |

该快照用于验证材料导入、字典生成和 solver 接线；它不等同于 Moldflow 专有材料数据库，也不代表已完成 Mug 逐点精度验证。

该 `PP-REF-01` 快照现已同步为 Kairos 内置 PP 默认模板；内置资产只保存拟合结果参数，不包含原始受控曲线。

## 4. 对照矩阵

| 层级 | Kairos 输出 | 实验观测 | 判定方式 |
| --- | --- | --- | --- |
| 填充 | 填充结束时间、压力、温度、剪切速率 | 压力传感器、短射重量/充模影像 | 先看趋势和量级，再定容差 |
| 保压 | V/P 切换、保压压力、质量 | 注射机曲线、制件称重 | 同工艺曲线下比较 |
| 冷却 | 模具/制件温度、冷却时间 | 热电偶或红外测温 | 记录测量位置和响应时间 |
| 尺寸 | 关键尺寸、椭圆度、厚度 | 三坐标/卡尺/扫描 | 固定测量基准和时间点 |
| 翘曲 | 位移场、最大变形 | 三维扫描 | 说明约束、基准面和符号方向 |

## 5. 每次运行留档模板

```text
timestamp: YYYYMMDDTHHMMSS+0800
git_commit:
solver_version:
material_id:
material_revision:
mesh_type:
mesh_target_size:
process:
  melt_temperature_c:
  mold_temperature_c:
  injection_time_s:
  flow_rate_cm3_s:
  packing_curve:
  cooling_time_s:
outputs:
  fill_end_time_s:
  max_injection_pressure_mpa:
  part_mass_g:
  max_clamp_force_t:
  max_shear_stress_mpa:
  max_shear_rate_per_s:
archive:
  raw_log:
  manifest:
  sha256:
assessment:
  material_error:
  mesh_error:
  process_error:
  solver_error:
  conclusion:
```

## 6. 当前结论

- Moldflow 参考工况已冻结，Kairos 的第一次 Mug 试跑尚未形成有效逐点对照。
- T101 正在补齐通用曲线输入、拟合、材料版本和自定义材料选择入口。
- 在真实实验数据进入前，只能报告可复现性和与参考报告的量级差异，不能写成“准确”或“验证通过”。

## 7. 更新记录

| 时间 | 变更 | 结论 |
| --- | --- | --- |
| 2026-09-15 | 建立报告骨架，冻结 Mug 参考工况与实验对照矩阵 | 等待 T101 材料闭环和 T100 有效 baseline |
| 2026-09-15 | 本地受控曲线插入测试：250 个 PVT 点、200 个黏度点完成解析；原始内置 Tait 模板因压力尺度不兼容被拒绝，随后用明确标注的合成 Tait 模板验证 `material fit` 输出链路 | 插入链路通过；不能把合成模板结果当作 Generic PP 材料结论 |
| 2026-09-15 | 使用本地拟合材料生成未求解 moldingFoam case，核对 `physicalProperties.melt` 与 `momentumTransport` 的 Cross-WLF/Tait 键和值；记录两个字典 SHA-256 | case 配置接线通过，尚未代表 solver 运行时数值通过 |


### 2026-09-15 · Kairos 自定义材料 VM baseline

- 由 CLI 生成 sample-box case 并用自定义材料运行；实际 case 根目录提交到 Multipass VM。
- moldingFoam v1.1.0 arm64 / OpenFOAM 14，退出码 `0`，日志以 `End` 收尾并达到 `Time = 2s`。
- 日志确认 `CrossWlf` 与 `Tait` 均被选中；Tait 使用 `b4/b4s/b6/C/smoothBand` 字段。
- 该运行验证材料导入、case 字典接线和 solver 启动闭环，不等同于 Moldflow 逐点精度验证；后续仍需把完整 Mug 网格和实验观测量接入同一矩阵。

### Mug 与参考工况一致的最终复现实验命令

下面命令固定 Moldflow 参考工况中的工艺输入，并使用仓库内置的 `PP-REF-01` 默认材料。Kairos 根据导入 STL 的实际体积和 `5.5 s` 注射时间计算名义流量；不会手写覆盖流量值。

```bash
cargo run -p kairos-cli -- doe run --plan full \
  --factor '熔体温度=220' \
  --factor '模具温度=50' \
  --factor '注射时间=5.5' \
  --stl report/mug-moldflow/mug.stl \
  --target-size 5.0 \
  --cores 5 \
  --injection-time 5.5 \
  --packing-pressure-mpa 27.6282 \
  --packing-time-s 20 \
  --out-dir /private/tmp/kairos-mug-baseline \
  --batch mug-baseline-220c-50c-5p5s-5c \
  --solve --vm --json
```

对应参考输入：熔体 `220 °C`、模具 `50 °C`、注射 `5.5 s`、保压 `27.6282 MPa / 20 s`、冷却参考 `20 s`。当前 CLI case 的求解终止时间由生成器控制，完整 Mug 结果以原始 `log.foamRun`、时间目录和 CLI JSON 汇总为准。网格仍是 Kairos 体积网格，不能与参考 Dual Domain 结果宣称逐点等价。

与临时五核命令的差异：

| 项目 | 临时命令 | 最终命令 |
| --- | --- | --- |
| 材料 | 显式传入本地拟合 JSON | 使用仓库内置 `PP-REF-01` |
| 因子 | 熔体温度、注射时间 | 另固定模具温度 50°C |
| 输出目录 | `/private/tmp/t100-mug-full-5c` | `/private/tmp/kairos-mug-baseline` |
| 批次名 | `mug-baseline-full-5c` | `mug-baseline-220c-50c-5p5s-5c` |

两条命令的几何、注射时间、保压压力和五核 VM 设置相同；最终命令用于仓库文档复现。

### moldingFoam 实验结果摘要（2026-09-15）

Kairos 生成的 `PP-REF-01` case 已在 moldingFoam v1.1.0 arm64 / OpenFOAM 14 VM 中真实运行：日志选择 `CrossWlf` 与 `Tait`，时间推进到 `2 s`，以 `End` 收尾，退出码为 `0`。该实验验证了默认 PP 材料、Tait 字典和 solver 的运行链路；由于当前 case 是 Kairos sample-box 工况，不能把它当作 Moldflow Mug 的逐点数值结论。

### Mug STL 单点实验命令（运行中）

```bash
cargo run -p kairos-cli -- doe run --plan full \
  --factor '熔体温度=220' --factor '注射时间=5.5' \
  --stl report/mug-moldflow/mug.stl \
  --target-size 5.0 --cores 1 \
  --injection-time 5.5 \
  --packing-pressure-mpa 27.6282 --packing-time-s 20 \
  --out-dir /private/tmp/t100-mug-full \
  --batch mug-baseline-full --solve --vm \
  --material report/mug-moldflow/material-input/generic-pp-fitted-v3.json --json
```

该命令使用 `full` 计划执行单点（`orthogonal` 计划要求每个因子三个水平），GUI 同源 Multipass VM 中的 moldingFoam v1.1.0 负责实际求解。case 的 `endTime=11 s`；截至记录时物理时间约 `1.45 s`、墙钟约 `235 s`，当前运行预计总耗时约 27 分钟。完成后补写退出码、填充时间、压力和质量预算结果。

### 五核重跑（2026-09-15）

单核运行因 CLI 600 秒超时中止，保留其原始日志作为性能记录。按要求使用 5 核重新提交同一工况：

```bash
cargo run -p kairos-cli -- doe run --plan full \
  --factor '熔体温度=220' --factor '注射时间=5.5' \
  --stl report/mug-moldflow/mug.stl --target-size 5.0 --cores 5 \
  --injection-time 5.5 --packing-pressure-mpa 27.6282 --packing-time-s 20 \
  --out-dir /private/tmp/t100-mug-full-5c --batch mug-baseline-full-5c \
  --solve --vm --material report/mug-moldflow/material-input/generic-pp-fitted-v3.json --json
```

当前 VM 已启动 `mpirun -np 5 foamRun -parallel`，五个 solver rank 均在工作；完成后补写实际退出码和指标。
