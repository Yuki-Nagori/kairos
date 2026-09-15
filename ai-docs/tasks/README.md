# Kairos 任务索引

- `-done` 表示该任务已有实现提交，**不等同于所有验收标准已经闭环**。功能、集成、
  性能与外部阻塞的权威状态见 [architecture-status.md](../architecture-status.md)；评审
  报告见 `ai-docs/reviews/`（每个任务一份 `Txx-review.md`，含八项清单结论、消融抽查
  与处理记录），时间线见 [ai-docs/timeline.md](../timeline.md)。
- 任务文件写目标、范围与验收标准；其中的“当前实现边界”用于防止文件名与实际能力
  脱节。实现方案在开工时再定。
- **真机 / 外部条件相关的验收**（求解器复跑、三端渲染数字、桌面截图、跨机器拷贝）
  统一归 T29 / T48 与各任务的「未闭环项」记录，不在实现任务里假装完成。

## 已实现任务

| ID  | 文件                                                                                       | 任务                                                                            |
| --- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- |
| T01 | [T01-perf-baseline-done.md](T01-perf-baseline-done.md)                                     | 性能预算与基准框架                                                              |
| T02 | [T02-ci-cd-done.md](T02-ci-cd-done.md)                                                     | CI/CD 流水线                                                                    |
| T03 | [T03-project-model-done.md](T03-project-model-done.md)                                     | 仿真项目模型与持久化                                                            |
| T04 | [T04-material-library-done.md](T04-material-library-done.md)                               | 材料数据库                                                                      |
| T05 | [T05-geometry-import-done.md](T05-geometry-import-done.md)                                 | 几何导入（STL）                                                                 |
| T06 | [T06-volume-meshing-done.md](T06-volume-meshing-done.md)                                   | 3D 体积网格生成                                                                 |
| T07 | [T07-runner-cooling-done.md](T07-runner-cooling-done.md)                                   | 浇口流道与冷却水路建模                                                          |
| T08 | [T08-process-settings-done.md](T08-process-settings-done.md)                               | 成型工艺设置                                                                    |
| T09 | [T09-openfoam-integration-done.md](T09-openfoam-integration-done.md)                       | OpenFOAM 运行时与 openInjMoldSim 集成                                           |
| T10 | [T10-job-scheduler-done.md](T10-job-scheduler-done.md)                                     | 求解作业调度器                                                                  |
| T11 | [T11-fill-e2e-done.md](T11-fill-e2e-done.md)                                               | 填充分析端到端闭环                                                              |
| T12 | [T12-pack-cool-done.md](T12-pack-cool-done.md)                                             | 保压与冷却分析                                                                  |
| T13 | [T13-result-model-done.md](T13-result-model-done.md)                                       | 结果数据模型与流式读取                                                          |
| T14 | [T14-viewport-engine-done.md](T14-viewport-engine-done.md)                                 | 3D 视口渲染引擎                                                                 |
| T15 | [T15-xy-charts-done.md](T15-xy-charts-done.md)                                             | XY 曲线与探针                                                                   |
| T16 | [T16-design-system-done.md](T16-design-system-done.md)                                     | UI 设计系统与工作台布局                                                         |
| T17 | [T17-report-export-done.md](T17-report-export-done.md)                                     | 仿真报告导出                                                                    |
| T18 | [T18-distribution-done.md](T18-distribution-done.md)                                       | 打包分发与自动更新                                                              |
| T19 | [T19-gpu-infrastructure-done.md](T19-gpu-infrastructure-done.md)                           | GPU 计算基础设施（wgpu 跨厂商层）                                               |
| T20 | [T20-gpu-postprocessing-done.md](T20-gpu-postprocessing-done.md)                           | GPU 加速后处理算子                                                              |
| T22 | [T22-gmsh-evaluation-done.md](T22-gmsh-evaluation-done.md)                                 | Gmsh 网格引擎集成评估与原型                                                     |
| T23 | [T23-time-animation-done.md](T23-time-animation-done.md)                                   | 体素→表面场映射与时间步动画通道                                                 |
| T24 | [T24-pinn-research-done.md](T24-pinn-research-done.md)                                     | PINN 熔融前沿预测研究 spike                                                     |
| T25 | [T25-workflow-api-study-done.md](T25-workflow-api-study-done.md)                           | 第三方工作流 API 分层概念研究                                                   |
| T26 | [T26-cae-workbench-ui-done.md](T26-cae-workbench-ui-done.md)                               | UI 重构为 Moldflow 风格专业 CAE 工作台                                          |
| T27 | [T27-component-download-done.md](T27-component-download-done.md)                           | 应用内组件下载                                                                  |
| T28 | [T28-repo-structure-done.md](T28-repo-structure-done.md)                                   | 仓库结构优化                                                                    |
| T29 | [T29-real-solve-e2e.md](T29-real-solve-e2e.md)                                             | 真实求解端到端验证（v1.0.0，剩余真机复测记录在未闭环项）                        |
| T30 | [T30-gmsh-integration-done.md](T30-gmsh-integration-done.md)                               | Gmsh 网格引擎正式集成                                                           |
| T31 | [T31-derived-fields-done.md](T31-derived-fields-done.md)                                   | 派生结果算子（对标 data_transform）                                             |
| T32 | [T32-headless-cli-done.md](T32-headless-cli-done.md)                                       | 无头 CLI 与批处理入口（门面式）                                                 |
| T34 | [T34-openfoam14-fork-entry-done.md](T34-openfoam14-fork-entry-done.md)                     | 求解入口收口：移除 openInjMoldSim，对接 moldingFoam（OpenFOAM-14 foamRun 框架） |
| T35 | [T35-vm-adapter-done.md](T35-vm-adapter-done.md)                                           | 虚拟机适配层：Multipass / WSL2 一键安装 + 内嵌 Shell + 退出联动关机             |
| T36 | [T36-contract-case-and-vm-exec-done.md](T36-contract-case-and-vm-exec-done.md)             | moldingFoam 契约对接 + bundle 进虚拟机执行                                      |
| T37 | [T37-vue3-migration-done.md](T37-vue3-migration-done.md)                                   | 前端迁移 Vue 3 + Composition API                                                |
| T38 | [T38-frontend-architecture-done.md](T38-frontend-architecture-done.md)                     | 前端架构重构：views / composables / Pinia 分层                                  |
| T39 | [T39-postprocess-renderer-done.md](T39-postprocess-renderer-done.md)                       | 后处理渲染后端 POC 与技术决策                                                   |
| T40 | [T40-iges-import-done.md](T40-iges-import-done.md)                                         | IGES 导入（106 / 63 镶嵌子集）                                                  |
| T41 | [T41-dual-domain-mesh-done.md](T41-dual-domain-mesh-done.md)                               | 双域网格（表面 + 杆系耦合）                                                     |
| T42 | [T42-midplane-mesh-done.md](T42-midplane-mesh-done.md)                                     | 中面网格（1D / 2.5D 快速分析路线）                                              |
| T43 | [T43-voxel-graded-refinement-done.md](T43-voxel-graded-refinement-done.md)                 | 局部加密 / 边界层（体素引擎分级加密）                                           |
| T44 | [T44-derive-pipeline-done.md](T44-derive-pipeline-done.md)                                 | 派生结果算子产品管线补全（线性映射 + 两场差值）                                 |
| T45 | [T45-viewport-picking-done.md](T45-viewport-picking-done.md)                               | 视口空间拾取与探针场关联                                                        |
| T46 | [T46-probe-time-series-done.md](T46-probe-time-series-done.md)                             | 探针时间曲线与时间轴联动                                                        |
| T47 | [T47-full-clipping-done.md](T47-full-clipping-done.md)                                     | 视口完整剖切（三轴平面 + 位置 + 反向）                                          |
| T49 | [T49-multi-viewport-done.md](T49-multi-viewport-done.md)                                   | 多视口联动；2026-09-15 修复导入自动显示、GPU 着色器与模型取景                   |
| T50 | [T50-volume-rendering-poc-done.md](T50-volume-rendering-poc-done.md)                       | 三维体渲染 POC（体绘制）                                                        |
| T51 | [T51-result-binary-chain-done.md](T51-result-binary-chain-done.md)                         | 大结果数据链第一步（二进制通道 + 会话缓存淘汰）                                 |
| T52 | [T52-report-content-done.md](T52-report-content-done.md)                                   | 分析报告内容增强（几何摘要 / 探针数值 / 时间序列）                              |
| T53 | [T53-report-template-done.md](T53-report-template-done.md)                                 | 模板化自定义报告（分区选择 + 标题 / 备注）                                      |
| T54 | [T54-vm-deploy-version-hint-done.md](T54-vm-deploy-version-hint-done.md)                   | 求解环境"更新未部署"提醒（VM 部署版本比对）                                     |
| T55 | [T55-gate-location-analysis-done.md](T55-gate-location-analysis-done.md)                   | 浇口位置分析（Gate Location 分析序列）                                          |
| T56 | [T56-mesh-aspect-match-rate-done.md](T56-mesh-aspect-match-rate-done.md)                   | 网格纵横比 + 双域匹配率进质量报告                                               |
| T57 | [T57-mesh-estimate-preview-done.md](T57-mesh-estimate-preview-done.md)                     | 网格预估单元数（生成前预览）                                                    |
| T58 | [T58-viewport-gate-picking-done.md](T58-viewport-gate-picking-done.md)                     | 视口拾取放置浇口 + 吸附最近节点                                                 |
| T59 | [T59-fill-preview-done.md](T59-fill-preview-done.md)                                       | 填充预览（轻量充填覆盖估计）                                                    |
| T61 | [T61-gate-geometry-landing-done.md](T61-gate-geometry-landing-done.md)                     | 浇口几何落地（模具网络 → case inlet）                                           |
| T62 | [T62-fill-load-validation-done.md](T62-fill-load-validation-done.md)                       | 工艺参数合理性校验（填充工况量级）                                              |
| T63 | [T63-thin-feature-hint-done.md](T63-thin-feature-hint-done.md)                             | 网格最小特征提示（体素尺寸 vs 壁厚）                                            |
| T64 | [T64-warpage-field-consumption-done.md](T64-warpage-field-consumption-done.md)             | 翘曲结果场消费（位移场口径）                                                    |
| T65 | [T65-warpage-deformation-view-done.md](T65-warpage-deformation-view-done.md)               | 翘曲变形可视化（视口位移显示）                                                  |
| T85 | [T85-linux-native-channel-done.md](T85-linux-native-channel-done.md)                       | Linux 原生执行通道                                                              |
| T90 | [T90-mucell-pvt-approximation-done.md](T90-mucell-pvt-approximation-done.md)               | 微发泡近似（PVT 修正）                                                          |
| T91 | [T91-fiber-orientation-chain.md](T91-fiber-orientation-chain.md)                           | 纤维取向链路（等上游取向场；第一步已落地）                                      |
| T66 | [T66-case-si-and-vent-alignment-done.md](T66-case-si-and-vent-alignment-done.md)           | case 单位制与排气边界对齐契约（SI + moldingVent）                               |
| T67 | [T67-coolant-channel-landing-done.md](T67-coolant-channel-landing-done.md)                 | 冷却水路落地（模壁 1D 通道 BC → case）                                          |
| T68 | [T68-study-creation-entry-done.md](T68-study-creation-entry-done.md)                       | 方案创建入口缺失（新建工程自带默认方案 + 工程面板「＋ 新建方案」）              |
| T69 | [T69-terminology-unification-done.md](T69-terminology-unification-done.md)                 | 术语统一：「方案 / 研究」混用收口                                               |
| T70 | [T70-latex-a11y-flatten-done.md](T70-latex-a11y-flatten-done.md)                           | 材料公式刷屏可访问性树（LaTeX 按单张图像暴露）                                  |
| T71 | [T71-card-collapse-a11y-done.md](T71-card-collapse-a11y-done.md)                           | 面板折叠入口的可访问性（UiCard 标题栏按按钮暴露 + 长面板可折叠）                |
| T72 | [T72-inlet-portal-area-done.md](T72-inlet-portal-area-done.md)                             | 浇口入口面口径与有效面积回显                                                    |
| T73 | [T73-download-version-display-done.md](T73-download-version-display-done.md)               | 下载面板显示 release 版本号（同名资产歧义）                                     |
| T74 | [T74-gate-velocity-si-recalibration-done.md](T74-gate-velocity-si-recalibration-done.md)   | 浇口速度阈值按 SI 重标定对齐（fillVelocityWarn 5 → 20）                         |
| T75 | [T75-workspace-layout-done.md](T75-workspace-layout-done.md)                               | 工作区布局（工程自包含：几何 / 网格 / case / 结果随工程走）                     |
| T76 | [T76-melt-transport-and-freeze-guard-done.md](T76-melt-transport-and-freeze-guard-done.md) | case 材料热物性口径：Pr 由导热真值反算 + 冻死短射守卫开启                       |
| T77 | [T77-step-import-done.md](T77-step-import-done.md)                                         | STEP 镶嵌网格导入（补档）                                                       |
| T78 | [T78-geometry-repair-done.md](T78-geometry-repair-done.md)                                 | 几何修复工具（补档）                                                            |
| T79 | [T79-sample-geometry-done.md](T79-sample-geometry-done.md)                                 | 内置样例几何（补档）                                                            |
| T80 | [T80-material-catalog-csv-done.md](T80-material-catalog-csv-done.md)                       | 牌号数据集扩充与 21 列 CSV 批量导入（补档）                                     |
| T81 | [T81-filler-parameters-done.md](T81-filler-parameters-done.md)                             | 纤维 / 填料参数组（补档）                                                       |
| T84 | [T84-result-data-chain-remainder-done.md](T84-result-data-chain-remainder-done.md)         | 大结果数据链剩余（分块 / 压缩 / 完整矢量场）                                    |
| T86 | [T86-special-process-modules-done.md](T86-special-process-modules-done.md)                 | 扩展工艺模块规划（GAIM / 双色多组分 / 微发泡）                                  |
| T87 | [T87-multiphysics-coupling-done.md](T87-multiphysics-coupling-done.md)                     | 多物理场耦合规划（纤维取向 / 流固耦合）                                         |
| T88 | [T88-e3-optimization-planning-done.md](T88-e3-optimization-planning-done.md)               | 优化与自动化规划（DOE 编排 / 工艺参数寻优）                                     |
| T92 | [T92-doe-orchestration.md](T92-doe-orchestration.md)                                       | DOE / 正交试验编排（矩阵、串行运行、汇总与失败标记）                            |
| T60 | [T60-import-log-and-pptx.md](T60-import-log-and-pptx.md)                                   | 导入日志流 + Rust PPTX 报告导出（含视口/曲线快照）                              |
| T96 | [T96-full-flow-e2e-done.md](T96-full-flow-e2e-done.md)                                     | 全流程集成测试（正常使用：几何 → 网格 → case → 求解 → 结果）                    |

## 循环任务（不进待办清单）

- **[T21 里程碑评审与整体优化](T21-milestone-review.md)**（P0）：每个里程碑结束时强制执行
  一轮（八项清单 + 消融抽查 + 性能对照），产出 `ai-docs/reviews/M<n>-review.md`。
  它是**触发式循环任务**，不是一次性交付项，因此不列在待办表里；当前一轮的结论与
  遗留项以最近一份评审报告为准。

> **等上游的事项**汇总在 [上游问题清单](../reviews/upstream-questions.md)（含证据与影响面，可直接转给上游）。

> **三处状态面的分工**：[architecture-status.md](../architecture-status.md) 按**能力**（B/C/D/E 分节）
> 看完成度与缺口；本表按**任务**看谁在做什么、卡在哪；[timeline.md](../timeline.md) 按**时间**记每次提交
> 做了什么。三者互补，不重复表述同一结论——发现不一致时以时间线与提交为准，并回填另两处。

## 待开工 / 待验收任务

> 端到端验证默认走 **CLI 通道**（`kairos-cli`：生成 case → 送进 VM → 求解 → 结果回传 → 结果扫描），
> 可脚本化、可重放；只有验证目标本身是界面 / 作业编排行为时才走 GUI（见 T94）。

| ID   | 文件                                                                                         | 任务                                            | 依赖 / 状态                                                                                             |
| ---- | -------------------------------------------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| T33  | [T33-updater-hardening.md](T33-updater-hardening.md)                                         | 发布加固：CSP 已加固 + updater / 签名待密钥证书 | CSP 零行为变化加固已落地；`style-src 'unsafe-inline'` 移除需真机回归，updater 与三平台签名需密钥 / 证书 |
| T48  | [T48-renderer-bench-realdevice.md](T48-renderer-bench-realdevice.md)                         | 渲染后端三端真机验收与 FPS 回填                 | 等三端真机（macOS/Windows/Linux WebView）                                                               |
| T82  | [T82-dualdomain-midplane-solve-consumption.md](T82-dualdomain-midplane-solve-consumption.md) | 双域 / 中面网格的求解消费                       | 延后：当前实验统一使用三维体网格；保留契约，等待上游降维能力                                            |
| T83  | [T83-multi-cavity-runner-fill.md](T83-multi-cavity-runner-fill.md)                           | 多型腔与流道系统参与填充                        | 流道体进网格 + 多腔分配，依赖 T29 闭环                                                                  |
| T89  | [T89-gaim-integration.md](T89-gaim-integration.md)                                           | 气体辅助注塑（GAIM）集成                        | 进行中：第一步气体介质数据位与 DTO 契约已完成；第二步等上游三相 / 气芯场                                |
| T93  | [T93-process-optimization.md](T93-process-optimization.md)                                   | 工艺参数自动寻优                                | 已实现：自适应回填、重试、约束过滤、失败中止与汇总完成；真实 VM 复测按 T100 记录                        |
| T99  | [T99-runtime-and-result-hardening.md](T99-runtime-and-result-hardening.md)                   | 运行生命周期与大结果处理优化                    | 已实现：生命周期、缓存、统计、并发、基准与 T93 回填闭环完成                                             |
| T100 | [T100-solver-runtime-v110-validation.md](T100-solver-runtime-v110-validation.md)             | 求解器运行时 v1.1.0 真实 CLI 验证与原始留档     | 进行中：VM 真实 DOE 与 moldingFoam v1.1.0 已验证；完整 Mug 对照与单点入口待补                           |
| T101 | [T101-generic-pp-material-fit.md](T101-generic-pp-material-fit.md)                           | Generic PP 自定义材料拟合与 Mug baseline        | 核心闭环完成：曲线拟合、残差、默认 PP 模板与 moldingFoam VM 验证已完成；GUI 材料面板后续补齐            |
| T102 | [T102-dualdomain-mug-exact-baseline.md](T102-dualdomain-mug-exact-baseline.md)               | Dual Domain Mug 完全一致基线                    | 延后：Mug 基线统一走三维体网格；Dual Domain 契约与样例保留，后续再做 solver 消费                        |
| T94  | [T94-gui-e2e-verification.md](T94-gui-e2e-verification.md)                                   | GUI 端到端验证（提交 → 回传 → VM 自动关闭）     | 待开工；需人在界面展开求解面板后跑（AX 拿不到折叠面板的输入框）；求解链路本身先走 CLI 通道              |
| T95  | [T95-dependency-audit.md](T95-dependency-audit.md)                                           | 依赖取向审查：用现成库 vs 自造轮子              | 进行中：repair 向量与色标基础收编已完成；剩图表/前端数学评估、摘要展示与证据补齐                        |
| T97  | [T97-solver-lib-duplication-guard.md](T97-solver-lib-duplication-guard.md)                   | 求解环境库重复守卫 + 部署前清理                 | 进行中：部署清理与 core/原生库探测已完成；真实 VM 坏状态验收待下一批                                    |

## 整体评审闭环（2026-09-16）

| 任务                                       | 状态   | 范围                                                             |
| ------------------------------------------ | ------ | ---------------------------------------------------------------- |
| [T98](T98-review-fixes-done.md)            | 已实现 | F1–F12、并发忙碌/日志加固、回归与文档                            |
| [T99](T99-runtime-and-result-hardening.md) | 已实现 | 生命周期服务、场共享/统计/导出、DTO、并发故障注入与 T93 回填闭环 |

T98 的本机测试不替代 T94/T96 的 Windows/WSL、Linux 与真实 VM 验收。

## 执行顺序（2026-09-16 更新）

核对范围：全部未完成任务逐条对照文件内声明的依赖、剩余范围与外部条件；已完成任务不再重复列入执行顺序。
每批只在依赖满足、验收证据明确后开工。结论分三档：

**档 A · 现在就能开工（无外部条件）**——按建议顺序：

1. **T95 补漏**（P2）：repair 向量消重、色标统一、摘要展示与准入证据补齐；P2-a 已完成，不重复开工。
2. **T95 对照评估**（P2/P3）：XY 图表与 gl-matrix 分别做视觉/性能/包体取舍；解压保持系统工具，按重审条件触发。
3. **T97**（P1）：求解环境库重复守卫与部署前清理；优先于新求解功能。
4. **T93 CLI 寻优循环**（P3）：核心实现已完成；真实 VM 结果按 T100 运行记录补充。
5. **T83 多型腔与流道参与填充**（P1）：文件依赖 T07/T61 均已满足（README 原先写的「依赖 T29 闭环」
   与文件不符，已改正）；验收在 VM 跑两腔填充，v0.2.5 通道已验证可用。
6. **T33 可做部分**（P3）：updater 插件接入 + `style-src 'unsafe-inline'` 改造（nonce / 文件化）；
   真机回归与三平台签名属档 B。
7. **T94**（P1）：GUI 端到端验证；需人在界面展开求解面板后跑。

**档 B · 等外部条件**：T29 剩余（上游 046 发版后复测 np=4）、T82（上游降维支持口径）、
T91 剩余（上游取向求解）、T89 第二步（上游三相 / 气芯）、T48（三端真机 WebView）、
T33 收尾（真机回归 + 签名证书密钥）、T94（需人展开求解面板一次）。

**档 C · 被未完成内部任务阻塞**：无。T92/T93 的核心 CLI 已完成，剩余事项属于 T100、T94 或外部求解器验收。

## 2026-09-13 收口批次（本轮完成）

按「先非求解器、后求解链路」的顺序完成，每条都有独立提交 + 评审记录 + timeline 行。
批次收口后按 T21 跑一轮整体评审（见 §循环任务）。

| 任务 | 内容                                   | 关键证据                                                             |
| ---- | -------------------------------------- | -------------------------------------------------------------------- |
| T55  | 浇口位置分析序列（免求解器启发式）     | 62.5 万四面体 / 399 候选 = 296 ms；[评审](../reviews/T55-review.md)  |
| T56  | 网格纵横比 + 双域匹配率进质量报告      | 解析值 √6 / √6⁄2 锁定；[评审](../reviews/T56-review.md)              |
| T57  | 网格预估单元数（生成前预览）           | 估算 == 实际（填满包围盒三档对拍）；[评审](../reviews/T57-review.md) |
| T58  | 视口拾取放置浇口 + 节点吸附            | 吸附到网格节点 / 单元中心回退；[评审](../reviews/T58-review.md)      |
| T59  | 填充预览（免求解器覆盖估计）           | 62.5 万四面体 = 168 ms；[评审](../reviews/T59-review.md)             |
| T65  | 翘曲变形可视化                         | 共享顶点取均值（防撕裂）；[评审](../reviews/T65-review.md)           |
| T67  | 冷却水路 → 模壁 1D 通道 BC             | 子字典字段 + SI 单位快照；[评审](../reviews/T67-review.md)           |
| T69  | 术语统一（方案 / 研究）                | `grep 研究` 全仓源码为 0；[评审](../reviews/T69-review.md)           |
| T72  | 浇口入口面口径与有效面积回显           | 粗网格不再吞入口带；[评审](../reviews/T72-review.md)                 |
| T75  | 工作区布局（工程自包含）               | 文档目录 / kairos / 工程目录；[评审](../reviews/T75-review.md)       |
| T84  | 大结果数据链收口（f32 / LRU / 矢量场） | 1e7 值 40 MB、解码 12 ms；[评审](../reviews/T84-review.md)           |
| T86  | 特殊工艺可行性（GAIM/双色/微发泡）     | 结论 + 拆解 T89/T90；[评审](../reviews/T86-review.md)                |
| T87  | 多物理场耦合可行性（纤维/流固）        | 结论 + 拆解 T91 / 流固不做；[评审](../reviews/T87-review.md)         |
| T88  | 优化与自动化可行性（DOE/寻优）         | 结论 + 拆解 T92/T93；[评审](../reviews/T88-review.md)                |

> T26/T27 落地后的增量打磨（系统窗口按钮、原生应用菜单、主题注入统一、
> 品牌图标、emoji 清理、启动窗口位置）见 `ai-docs/timeline.md` 对应条目，
> 不单独立任务。

## 测试分层约定

前端 GUI 的日常回归优先走 Web 测试（`bun run test:web`）和 Vite 预览，避免每次修改都重新打包 Tauri。涉及原生窗口、菜单、文件对话框、真实 IPC 或发布配置时，再补一次桌面冒烟；这两个层次不能互相替代。

## 全局原则（不变）

- **性能是一等约束**：新任务带明确性能验收标准，预算源自 T01（实测数字统一回填
  [perf-budget.md](../perf-budget.md)）；
- **GPL 隔离红线**：GPL 代码只走子进程 + 文件交换；
- **GPU 统一走 wgpu**：跨 NVIDIA / AMD / Intel / Apple；后处理以硬件 GPU 为前提，
  无可用 GPU 时明确显示不受支持，CPU 仅作正确性基准；
- 领域代码进 `src-crates/kairos-core`，适配进 `src-tauri`，UI 进 `src-web`；
- 任务有实现提交才可加 `-done`；验收是否闭环仍须同步完成度清单和性能证据。
