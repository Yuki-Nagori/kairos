# Mug 基线与实验对照

- 状态：进行中
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
