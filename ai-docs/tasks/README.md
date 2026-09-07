# Kairos 路线图与任务索引

Kairos 的目标是做一款注塑成型 CAE 仿真软件：功能对标 Autodesk Moldflow 并在体验与性能上优化。本文是全部任务的总索引；**每个任务一个独立 md 文件，只写任务本身**（目标 / 范围 / 非目标 / 交付物 / 验收标准），不写实现方案——实现方案在任务开工时再定。

## 求解器选型事实（影响全局）

求解器采用开源的 **openInjMoldSim**（OpenFOAM 7 上的注塑求解器，GPL-3.0，有 V&V 论文）。四个直接影响计划的事实：

1. **GPL-3.0** → Kairos 只能以独立子进程方式调用它，不链接、不分发修改版源码，保持本仓库 Apache-2.0 干净；
2. **3D 求解器** → 网格路线是**三维体积网格**，不复制 Moldflow 特有的中面 / 双域（Dual Domain）技术；
3. **覆盖填充 / 保压 / 冷却** → 翘曲（Warpage）与纤维取向不在其能力内，列为远期评估；
4. **运行时依赖 OpenFOAM 7（.org 版）** → 分发策略（捆绑或引导安装）是交付阶段的一等任务；
5. **求解器本身是 CPU 代码**（OpenFOAM 7 无 GPU 路线）→ GPU 辅助计算覆盖前处理/后处理/渲染（T19、T20），求解内核 GPU 化列为远期评估。

> 注：不存在名为 "OpenForm" 的求解器；此处按 OpenFOAM 生态（openInjMoldSim）对齐。如另有所指，先修订本节再动任务。

## 里程碑

| 里程碑          | 内容                                                                                                                | 任务                              |
| --------------- | ------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| M0 地基         | 性能预算与 CI                                                                                                       | T01、T02                          |
| M1 数据底座     | 项目模型、材料库                                                                                                    | T03、T04                          |
| M2 前处理       | 几何、网格、浇口/水路、工艺                                                                                         | T05、T06、T07、T08                |
| M3 求解         | OpenFOAM 集成、调度器、填充闭环、保压冷却                                                                           | T09、T10、T11、T12                |
| M4 后处理与体验 | 结果读取、3D 视口、GPU 加速算子、图表、设计系统、报告                                                               | T13、T14、T15、T16、T17、T19、T20 |
| M5 交付         | 打包分发与自动更新                                                                                                  | T18                               |
| 评审节点        | **每个里程碑结束强制执行 T21 整体评审与优化**（架构 / 性能 / 质量 / 文档 / 安全八项清单），未通过不得开启下一里程碑 | T21（循环）                       |

关键里程碑：**T11 填充分析端到端闭环**是整个计划的分水岭——完成即证明「前处理 → 求解 → 后处理」全链路成立。

## 任务索引

| ID  | 文件                                                       | 任务                                  | 依赖               | 里程碑       |
| --- | ---------------------------------------------------------- | ------------------------------------- | ------------------ | ------------ |
| T01 | [T01-perf-baseline.md](T01-perf-baseline.md)               | 性能预算与基准框架                    | —                  | M0           |
| T02 | [T02-ci-cd.md](T02-ci-cd.md)                               | CI/CD 流水线                          | —                  | M0           |
| T03 | [T03-project-model.md](T03-project-model.md)               | 仿真项目模型与持久化                  | —                  | M1           |
| T04 | [T04-material-library.md](T04-material-library.md)         | 材料数据库                            | T03                | M1           |
| T05 | [T05-geometry-import.md](T05-geometry-import.md)           | 几何导入（STL）                       | T03                | M2           |
| T06 | [T06-volume-meshing.md](T06-volume-meshing.md)             | 3D 体积网格生成                       | T05                | M2           |
| T07 | [T07-runner-cooling.md](T07-runner-cooling.md)             | 浇口流道与冷却水路建模                | T05                | M2           |
| T08 | [T08-process-settings.md](T08-process-settings.md)         | 成型工艺设置                          | T04                | M2           |
| T09 | [T09-openfoam-integration.md](T09-openfoam-integration.md) | OpenFOAM 运行时与 openInjMoldSim 集成 | T06、T08           | M3           |
| T10 | [T10-job-scheduler.md](T10-job-scheduler.md)               | 求解作业调度器                        | T09                | M3           |
| T11 | [T11-fill-e2e.md](T11-fill-e2e.md)                         | 填充分析端到端闭环（分水岭）          | T09、T10、T13、T14 | M3           |
| T12 | [T12-pack-cool.md](T12-pack-cool.md)                       | 保压与冷却分析                        | T11                | M3           |
| T13 | [T13-result-model.md](T13-result-model.md)                 | 结果数据模型与流式读取                | T09                | M4           |
| T14 | [T14-viewport-engine.md](T14-viewport-engine.md)           | 3D 视口渲染引擎                       | T06                | M4           |
| T15 | [T15-xy-charts.md](T15-xy-charts.md)                       | XY 曲线与探针                         | T13                | M4           |
| T16 | [T16-design-system.md](T16-design-system.md)               | UI 设计系统与工作台布局               | —                  | M4           |
| T17 | [T17-report-export.md](T17-report-export.md)               | 仿真报告导出                          | T13、T15           | M4           |
| T18 | [T18-distribution.md](T18-distribution.md)                 | 打包分发与自动更新                    | T09、T11           | M5           |
| T19 | [T19-gpu-infrastructure.md](T19-gpu-infrastructure.md)     | GPU 计算基础设施（wgpu 跨厂商层）     | —                  | M4           |
| T20 | [T20-gpu-postprocessing.md](T20-gpu-postprocessing.md)     | GPU 加速后处理算子                    | T19、T13、T14      | M4           |
| T21 | [T21-milestone-review.md](T21-milestone-review.md)         | 里程碑评审与整体优化（每里程碑循环）  | 当前里程碑         | 每个里程碑末 |

## 全局原则

- **性能是一等约束**：每个涉及计算与渲染的任务都带明确性能验收标准，预算源自 T01；
- **GPL 隔离红线**：任何 GPL 代码（含 OpenFOAM 宏头展开物）不得进入本仓库，集成只走子进程与文件交换；
- **GPU 统一走 wgpu**：NVIDIA / AMD / Intel / Apple 四类显卡一套代码（Vulkan/DX12/Metal），禁止引入 CUDA 等单厂商 SDK；无 GPU 环境必须回退 CPU（T19）；
- **UI 对标 Moldflow 信息架构并现代化**：研究树 / 功能区 / 视口 / 结果图层 + 现代设计语言，详见 T16；
- 领域代码一律进 `src-crates/kairos-core`，适配进 `src-tauri`，UI 进 `src-web`（见 ARCHITECTURE.md）；
- **任务完成自查**：每个任务关闭前逐条核对其验收标准、`bun run verify` 全绿、注释与文档同步、性能埋点就位（涉及性能的任务）；
- **评审穿插**：整体优化与 review 由 T21 在每个里程碑末强制执行，评审发现按「立即修 / 回流任务 / 接受并记录」三类闭环；
