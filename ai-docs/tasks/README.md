# Kairos 任务索引

- `-done` 表示该任务已有实现提交，**不等同于所有验收标准已经闭环**。功能、集成、
  性能与外部阻塞的权威状态见 [architecture-status.md](../architecture-status.md)；评审
  报告见 `ai-docs/reviews/`，时间线见 `ai-docs/timeline.md`。
- 任务文件写目标、范围与验收标准；其中的“当前实现边界”用于防止文件名与实际能力
  脱节。实现方案在开工时再定。

## 已实现任务（M0–M5；验收状态另见完成度清单）

| ID  | 文件                                                                 | 任务                   |
| --- | -------------------------------------------------------------------- | ---------------------- |
| T01 | [T01-perf-baseline-done.md](T01-perf-baseline-done.md)               | 性能预算与基准框架     |
| T02 | [T02-ci-cd-done.md](T02-ci-cd-done.md)                               | CI/CD 流水线           |
| T03 | [T03-project-model-done.md](T03-project-model-done.md)               | 仿真项目模型与持久化   |
| T04 | [T04-material-library-done.md](T04-material-library-done.md)         | 材料数据库             |
| T05 | [T05-geometry-import-done.md](T05-geometry-import-done.md)           | 几何导入（STL）        |
| T06 | [T06-volume-meshing-done.md](T06-volume-meshing-done.md)             | 3D 体积网格生成        |
| T07 | [T07-runner-cooling-done.md](T07-runner-cooling-done.md)             | 浇口流道与冷却水路建模 |
| T08 | [T08-process-settings-done.md](T08-process-settings-done.md)         | 成型工艺设置           |
| T09 | [T09-openfoam-integration-done.md](T09-openfoam-integration-done.md) | 求解器运行时集成       |
| T10 | [T10-job-scheduler-done.md](T10-job-scheduler-done.md)               | 求解作业调度器         |
| T11 | [T11-fill-e2e-done.md](T11-fill-e2e-done.md)                         | 填充分析端到端闭环     |
| T12 | [T12-pack-cool-done.md](T12-pack-cool-done.md)                       | 保压与冷却分析         |
| T13 | [T13-result-model-done.md](T13-result-model-done.md)                 | 结果数据模型与流式读取 |
| T14 | [T14-viewport-engine-done.md](T14-viewport-engine-done.md)           | 3D 视口渲染引擎        |
| T15 | [T15-xy-charts-done.md](T15-xy-charts-done.md)                       | XY 曲线与探针          |
| T16 | [T16-design-system-done.md](T16-design-system-done.md)               | 设计系统与工作台布局   |
| T17 | [T17-report-export-done.md](T17-report-export-done.md)               | 仿真报告导出           |
| T18 | [T18-distribution-done.md](T18-distribution-done.md)                 | 打包分发与发布流水线   |
| T19 | [T19-gpu-infrastructure-done.md](T19-gpu-infrastructure-done.md)     | GPU 计算基础设施       |
| T20 | [T20-gpu-postprocessing-done.md](T20-gpu-postprocessing-done.md)     | GPU 加速后处理算子     |

## 后续增量实现（部分任务仍待验收）

| ID  | 文件                                                                           | 任务                              |
| --- | ------------------------------------------------------------------------------ | --------------------------------- |
| T22 | [T22-gmsh-evaluation-done.md](T22-gmsh-evaluation-done.md)                     | Gmsh 网格引擎评估与解析原型       |
| T23 | [T23-time-animation-done.md](T23-time-animation-done.md)                       | 体素→表面场映射与时间步动画通道   |
| T26 | [T26-cae-workbench-ui-done.md](T26-cae-workbench-ui-done.md)                   | Moldflow 风格工作台 UI + 主题     |
| T27 | [T27-component-download-done.md](T27-component-download-done.md)               | 应用内组件下载                    |
| T24 | [T24-pinn-research-done.md](T24-pinn-research-done.md)                         | PINN 熔融前沿预测研究 spike       |
| T25 | [T25-workflow-api-study-done.md](T25-workflow-api-study-done.md)               | 第三方工作流 API 分层概念研究     |
| T28 | [T28-repo-structure-done.md](T28-repo-structure-done.md)                       | 仓库结构优化（state/WGSL/面板）   |
| T30 | [T30-gmsh-integration-done.md](T30-gmsh-integration-done.md)                   | Gmsh 网格引擎正式集成             |
| T31 | [T31-derived-fields-done.md](T31-derived-fields-done.md)                       | 派生结果算子（基础实现）          |
| T32 | [T32-headless-cli-done.md](T32-headless-cli-done.md)                           | 无头 CLI 与批处理入口             |
| T34 | [T34-openfoam14-fork-entry-done.md](T34-openfoam14-fork-entry-done.md)         | OpenFOAM-14 求解入口收口          |
| T35 | [T35-vm-adapter-done.md](T35-vm-adapter-done.md)                               | 虚拟机适配层                      |
| T36 | [T36-contract-case-and-vm-exec-done.md](T36-contract-case-and-vm-exec-done.md) | case 契约与 VM 执行链路           |
| T37 | [T37-vue3-migration.md](T37-vue3-migration.md)                                 | Vue 3 迁移                        |
| T38 | [T38-frontend-architecture.md](T38-frontend-architecture.md)                   | 前端架构重构                      |
| T40 | [T40-iges-import-done.md](T40-iges-import-done.md)                             | IGES 导入（106/63 镶嵌子集）      |
| T41 | [T41-dual-domain-mesh-done.md](T41-dual-domain-mesh-done.md)                   | 双域网格（表面 + 杆系耦合）       |
| T42 | [T42-midplane-mesh-done.md](T42-midplane-mesh-done.md)                         | 中面网格（1D/2.5D 快速路线）      |
| T43 | [T43-voxel-graded-refinement-done.md](T43-voxel-graded-refinement-done.md)     | 局部加密 / 边界层（体素分级）     |
| T44 | [T44-derive-pipeline-done.md](T44-derive-pipeline-done.md)                     | 派生算子管线补全（线性/差值）     |
| T45 | [T45-viewport-picking-done.md](T45-viewport-picking-done.md)                   | 视口空间拾取与探针场关联          |
| T46 | [T46-probe-time-series-done.md](T46-probe-time-series-done.md)                 | 探针时间曲线与时间轴联动          |
| T47 | [T47-full-clipping-done.md](T47-full-clipping-done.md)                         | 视口完整剖切（三轴平面）          |
| T49 | [T49-multi-viewport-done.md](T49-multi-viewport-done.md)                       | 多视口联动（布局/相机/时间轴）    |
| T50 | [T50-volume-rendering-poc-done.md](T50-volume-rendering-poc-done.md)           | 三维体渲染 POC（重采样+光线步进） |
| T51 | [T51-result-binary-chain-done.md](T51-result-binary-chain-done.md)             | 大结果二进制通道 + 有界缓存       |
| T52 | [T52-report-content-done.md](T52-report-content-done.md)                       | 分析报告内容增强（几何/探针表）   |
| T53 | [T53-report-template-done.md](T53-report-template-done.md)                     | 模板化自定义报告                  |
| T54 | [T54-vm-deploy-version-hint-done.md](T54-vm-deploy-version-hint-done.md)       | 求解环境"更新未部署"提醒          |

> T26/T27 落地后的增量打磨（系统窗口按钮、原生应用菜单、主题注入统一、
> 品牌图标、emoji 清理、启动窗口位置）见 `ai-docs/timeline.md` 对应条目，
> 不单独立任务。

## 待开工 / 待验收任务

| ID  | 文件                                                                 | 任务                            | 依赖     |
| --- | -------------------------------------------------------------------- | ------------------------------- | -------- |
| T21 | [T21-milestone-review.md](T21-milestone-review.md)                   | 里程碑评审循环（随里程碑触发）  | 随里程碑 |
| T29 | [T29-real-solve-e2e.md](T29-real-solve-e2e.md)                       | 真实求解端到端验证（OpenFOAM）  | T09–T11  |
| T33 | [T33-updater-hardening.md](T33-updater-hardening.md)                 | 发布加固：updater + CSP 收窄    | T18      |
| T39 | [T39-postprocess-renderer.md](T39-postprocess-renderer.md)           | 后处理渲染后端 POC 与技术决策   | T13/T14  |
| T48 | [T48-renderer-bench-realdevice.md](T48-renderer-bench-realdevice.md) | 渲染后端三端真机验收与 FPS 回填 | T39      |

> T29 优先级最高：真实求解仍被 moldingFoam bundle 的 SIGILL 阻塞。T13/T14/T20/
> T23/T31 的实现已经存在，但大结果、GPU 管线与性能验收尚未闭环，不能推进为
> “后处理已完成”。T39 在不引入生产 VTK.js 依赖的前提下，为渲染后端保留可验证的
> 决策出口。

## 全局原则（不变）

- **性能是一等约束**：新任务带明确性能验收标准，预算源自 T01；
- **GPL 隔离红线**：GPL 代码只走子进程 + 文件交换；
- **GPU 统一走 wgpu**：跨 NVIDIA / AMD / Intel / Apple；后处理以硬件 GPU 为前提，
  无可用 GPU 时明确显示不受支持，CPU 仅作正确性基准；
- 领域代码进 `src-crates/kairos-core`，适配进 `src-tauri`，UI 进 `src-web`；
- 任务有实现提交才可加 `-done`；验收是否闭环仍须同步完成度清单和性能证据。
