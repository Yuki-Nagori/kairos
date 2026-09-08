# Kairos 任务索引

- 已完成任务文件名以 `-done` 结尾，评审报告见 `ai-docs/reviews/`，时间线见 `ai-docs/timeline.md`；
- 任务文件只写任务本身（目标 / 范围 / 非目标 / 交付物 / 验收标准），实现方案在开工时再定。

## 已完成任务（M0–M5）

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

## 后续完成任务

| ID  | 文件                                                             | 任务                            |
| --- | ---------------------------------------------------------------- | ------------------------------- |
| T22 | [T22-gmsh-evaluation-done.md](T22-gmsh-evaluation-done.md)       | Gmsh 网格引擎评估与解析原型     |
| T23 | [T23-time-animation-done.md](T23-time-animation-done.md)         | 体素→表面场映射与时间步动画通道 |
| T26 | [T26-cae-workbench-ui-done.md](T26-cae-workbench-ui-done.md)     | Moldflow 风格工作台 UI + 主题   |
| T27 | [T27-component-download-done.md](T27-component-download-done.md) | 应用内组件下载                  |
| T24 | [T24-pinn-research-done.md](T24-pinn-research-done.md)           | PINN 熔融前沿预测研究 spike     |
| T25 | [T25-workflow-api-study-done.md](T25-workflow-api-study-done.md) | 第三方工作流 API 分层概念研究   |
| T28 | [T28-repo-structure-done.md](T28-repo-structure-done.md)         | 仓库结构优化（state/WGSL/面板） |

> T26/T27 落地后的增量打磨（系统窗口按钮、原生应用菜单、主题注入统一、
> 品牌图标、emoji 清理、启动窗口位置）见 `ai-docs/timeline.md` 对应条目，
> 不单独立任务。

## 待开工任务

| ID  | 文件                                               | 任务                           | 依赖     |
| --- | -------------------------------------------------- | ------------------------------ | -------- |
| T21 | [T21-milestone-review.md](T21-milestone-review.md) | 里程碑评审循环（随里程碑触发） | 随里程碑 |

> 任务池已清空：T24/T25/T28 于 2026-09-08 完成；后续新任务（如
> CLI/批处理入口、网格质量度量）见 `ai-docs/research/` 两份结论文档的建议清单。

## 全局原则（不变）

- **性能是一等约束**：新任务带明确性能验收标准，预算源自 T01；
- **GPL 隔离红线**：GPL 代码只走子进程 + 文件交换；
- **GPU 统一走 wgpu**：跨 NVIDIA / AMD / Intel / Apple，无 GPU 环境回退 CPU；
- 领域代码进 `src-crates/kairos-core`，适配进 `src-tauri`，UI 进 `src-web`；
- 任务完成即改名加 `-done` 并同步本索引。
