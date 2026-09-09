# T34 · 求解入口收口：移除 openInjMoldSim，对接 OpenFOAM 原生 foamRun（fork 预留）

- 阶段：进行中（移除部分本次落地；fork 对接待外部就绪）
- 依赖：T09–T12（求解集成代码）
- 优先级：**P1**

## 背景与决策

- openInjMoldSim（krebeljk）是 compressibleInterFoam 的第三方改版，上游 README
  明确要求 OpenFOAM 7，无法跟随 .org 新版的模块化框架（11+ 引入 foamRun，
  当前最新为 14，2026-07 发布）。
- 决策（2026-09-08）：**Kairos 删去 openInjMoldSim 依赖**；注塑求解物理
  （Cross-WLF 黏度、Tait PVT、保压/冷却字典、注射边界条件）改由
  **用户自维护的 OpenFOAM-14 fork** 承担，Kairos 侧只保留求解入口。
- OpenFOAM 依赖从 7 升到 14：controlDict 固定 `application foamRun;` +
  `solver <模块>;`，上游阶段用原生 `compressibleVoF`（compressibleInterFoam
  的官方后继模块），fork 就绪后在 `openfoam.rs::SOLVER_MODULE` 单点替换。

## 本次范围（移除与入口）

- 依赖目录删除 openinjmoldsim 条目；OpenFOAM 条目升到 14
  （下载源 `github.com/OpenFOAM/OpenFOAM-14` master.tar.gz，名称 / 提示 /
  apt 包名同步）；
- 下载顺序守卫（求解器需先下 OpenFOAM）随条目删除；白名单移除 krebeljk 源；
- 编译脚本只剩 openfoam 全量构建分支；
- 就绪探测：blockMesh（工具链）+ foamRun（模块化运行器，11+ 才有，
  旧版本给出升级提示）；
- 作业脚本 `decomposePar -force && foamRun -parallel`（求解模块由
  controlDict 提供）；case 生成把模块名收进 `SOLVER_MODULE` 常量；
- web 侧自动编译钩子、编译按钮显示条件、面板与服务层文案同步。

## fork 就绪后的对接清单（待办）

1. ~~依赖目录下载直链换成 fork 仓库~~（已完成：`openfoam` 条目现指向
   `Yuki-Nagori/moldingFoam` 的 `releases/latest`，下载时按宿主架构解析
   资产并 tar.xz 解压，应用内副本直接可用，无需编译）；
2. `openfoam.rs::SOLVER_MODULE` 换成 `moldingFoam`——bundle 已自带
   `libmoldingFoamSolver.so` 探测链接，需与第 3 条 case 字典布局切换同步做；
3. case 生成器按 moldingFoam 的 `case-contract/` v1.1 字典布局重写
   （packingDict/coolingDict/transportProperties → moldingDict +
   physicalProperties.<相> 等）；
4. T29 端到端在 bundle 环境跑通填充 / 保压 / 冷却并复核 case 模板字段。

## 非目标

- fork 本身的开发（外部仓库进行）；
- 注塑物理字典的字段重设计（沿用既有口径，等 T29 实测反馈）。

## 验收标准

- 仓库内无 openInjMoldSim 残留（代码 grep 为零；历史任务 / 评审文档除外）；
- `bun run verify` 全绿；依赖面板只剩 OpenFOAM 14 与 Gmsh；
- 求解入口（foamRun + SOLVER_MODULE）保持单点可换。
