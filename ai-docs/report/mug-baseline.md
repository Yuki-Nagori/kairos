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

## 6. 最终复现实验命令

该命令作为当前“三维体网格基线”实验的唯一入口，使用内置 `PP-REF-01`，固定 Mug 参考工艺，并分配 VM 五核：

```bash
cargo run -p kairos-cli -- doe run --plan full \
  --factor '熔体温度=220' \
  --factor '模具温度=50' \
  --factor '注射时间=5.5' \
  --stl report/mug-moldflow/mug.stl \
  --target-size 5.0 --cores 5 \
  --injection-time 5.5 \
  --packing-pressure-curve '0=0.9229,0.2=27.6282,315.0797=27.6282' \
  --cooling-time-s 20 \
  --stage fill-pack-cool \
  --out-dir /private/tmp/kairos-mug-baseline \
  --batch mug-baseline-220c-50c-5p5s-5c \
  --solve --vm --json
```

CLI 现已支持完整保压曲线、独立冷却时间和显式 `fill-pack-cool` 阶段。工艺输入与 `setting.txt` 一致；Kairos 当前使用 5.0 mm 目标尺寸的三维体网格（15,868 节点 / 48,605 四面体），参考结果为 Dual Domain（33,418 节点 / 66,830 三角形），网格和边界不同，结果只能做量级对照。

## 7. 已执行运行

| 运行 | 目的 | 状态 | 结论 |
| --- | --- | --- | --- |
| sample-box + PP-REF-01 + VM 单核 | 验证材料、字典和 solver 链路 | 通过，退出码 0，`End` | Cross-WLF/Tait 真实加载，推进到 2 s |
| Mug STL + VM 单核 | 性能摸底 | CLI 600 s 超时 | 原始日志保留，未作为结果验收 |
| Mug STL + VM 五核 | 并行链路 smoke run | 已启动并按要求停止 | `mpirun -np 5 foamRun -parallel` 正常拉起五个 rank；不作为数值结果 |

## 7.1 压缩网格性能 smoke（不作为基线）

为验证长工况能否快速进入 solver，使用相同 Mug、材料和工艺参数，仅将体网格目标尺寸从 5.0 mm 调整为 10.0 mm：

| 项目 | 压缩 smoke |
| --- | ---: |
| 目标尺寸 | 10.0 mm |
| 节点 / 四面体 | 2,901 / 6,640 |
| 五核 solver | 成功启动并正常推进 |
| 观测物理时间 | 1.401 s |
| 墙钟 | 47 s |
| 估算完整工况 | 仍需数小时，未等待完成 |

该批次只验证网格缩减后 case 能进入 moldingFoam，未产生完整结果，不替代 5.0 mm 基线，也不用于 Moldflow 数值对照。

## 8. 当前结论

- `PP-REF-01` 已作为 Kairos 内置 PP 默认模板，参数快照见第 3.1 节。
- moldingFoam v1.1.0 的材料导入、Tait/Cross-WLF 字典和 VM 求解链路已跑通。
- Mug 三维体网格实验已具备 `FillPackCool` 命令入口，可直接提交 moldingFoam；Dual Domain solver 消费延后，不阻塞当前实验。
- Moldflow 参考值只作为对照基线。当前已对齐几何文件、温度、注射时间、保压曲线、冷却时间和求解阶段；网格类型/分辨率、浇口边界和 OpenFOAM 离散仍不一致，不报告逐点准确或完全验证通过。

## 9. 更新记录

| 时间 | 变更 | 结论 |
| --- | --- | --- |
| 2026-09-15 | 建立报告并冻结 Mug 参考工况 | 形成 Moldflow/Kairos/实验统一对照口径 |
| 2026-09-15 | 完成 PP 曲线解析、拟合、残差和材料字典输出 | T101 材料核心闭环完成 |
| 2026-09-15 | sample-box 自定义材料 VM 实验 | solver 真实启动、Cross-WLF/Tait 生效、退出码 0 |
| 2026-09-15 | Mug 单核/五核 smoke run | 单核超时；五核并行链路拉起并停止，均不作为数值验收 |
