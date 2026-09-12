# T34 · 求解入口收口：移除 openInjMoldSim，对接 moldingFoam（OpenFOAM-14 foamRun 框架）

- 阶段：已完成（移除与入口收口本次落地；fork 对接由 T35/T36/T54 接续完成，
  实测反馈见 T29）
- 依赖：T09–T12（求解集成代码）
- 优先级：**P1**

## 背景与决策

- openInjMoldSim（krebeljk）是 compressibleInterFoam 的第三方改版，上游 README
  明确要求 OpenFOAM 7，无法跟随 .org 新版的模块化框架（11+ 引入 foamRun，
  当前最新为 14）。
- 决策（2026-09-08）：**Kairos 删去 openInjMoldSim 依赖**；注塑求解物理
  （Cross-WLF 黏度、Tait PVT、保压/冷却字典、注射边界条件）改由
  **用户自维护的 moldingFoam 项目** 承担，Kairos 侧只保留求解入口。
- 求解入口 = foamRun 框架 + 模块名：`controlDict` 固定
  `application foamRun; solver <模块>;`，Kairos 侧收敛为
  `services/moldingfoam.rs::SOLVER_MODULE` 一个常量。
- 命名口径（2026-09-12 统一）：模块文件与模块名由 `openfoam` 更名为
  `moldingfoam`——生成的是 **moldingFoam 的 case**，OpenFOAM 只是承载框架，
  沿用 openfoam 命名会让「求解环境」与「求解模块」两个概念混在一个词里。

## 交付内容

1. **移除 openInjMoldSim**：依赖目录条目、下载顺序守卫（求解器需先下
   OpenFOAM）、白名单里的 krebeljk 源、编译脚本分支、web 侧自动编译钩子与
   面板文案（验收 grep 为零，仅测试保留负向断言）；
2. **求解环境升到 OpenFOAM 14 口径**：就绪探测 blockMesh（工具链）+ foamRun
   （模块化运行器，11+ 才有，旧版本给出升级提示）；
3. **投递方式改为预编译 bundle**：依赖目录 moldingfoam 条目指向
   `Yuki-Nagori/moldingFoam` 的 `releases/latest`，下载时按宿主架构解析资产
   （资产名含日期，无法用固定文件名直链），tar.xz 自动解压即用、无需编译；
   后续由 T35（虚拟机适配）/T54（部署版本比对）补齐执行与部署链路；
4. **求解入口单点**：`SOLVER_MODULE = "moldingFoam"`，case 生成器
   （`services/moldingfoam.rs`）与作业脚本都从该常量取模块名。

## fork 对接清单（全部落地）

| 项                                                  | 落地情况                                                             |
| --------------------------------------------------- | -------------------------------------------------------------------- |
| 依赖下载直链换成 fork 仓库                          | 已完成：`releases/latest` + 按宿主架构解析资产 + tar.xz 自动解压     |
| `SOLVER_MODULE` 换成 `moldingFoam`                  | 已完成（T36）                                                        |
| case 生成器按 moldingFoam `case-contract/` 布局重写 | 已完成（T36）：moldingDict + `physicalProperties.<相>` + 0/ 边界条件 |
| 端到端在 bundle 环境实跑并复核 case 模板字段        | T29 进行中：VM 内已跑通 0 → endTime 全段；剩余阻塞在求解器侧（见下） |

## 实测修正（T29 回填）

真实求解器跑通后回填的三处 Kairos 侧修正，均已在
[T29](T29-real-solve-e2e.md) 落地并加回归测试：

- **并行调用**：`foamRun` 不会自行调 mpirun，直接 `foamRun -parallel` 会以
  "attempt to run parallel on 1 processor" 退出；必须
  `mpirun -np <cores> foamRun -parallel`，且 `-np` 与 decomposeParDict 的
  numberOfSubdomains（同一 cores）一致；并行结果写在 `processor*/` 下，需
  `reconstructPar` 才有 case 级时间目录——统一收在
  `moldingfoam::solve_command`；
- **保压压力表**：契约要求 `packing.switchPressure` 与保压曲线起点一致以消除
  压力阶跃，不能再额外写一个大气压首点（曲线起点为 t=0 时会产生重复横坐标，
  被 `Function1s::Table::check` 判为 out-of-order 拒绝启动）；
- **polyMesh 写出**：`faces` / `owner` / `neighbour` 三个列表必须逐项对齐、
  内部面在前并按 (owner, neighbour) 升序排列；只重排其中两个会让 owner 与面
  错位，checkMesh 报上千个负体积单元、求解第一步即 NaN。

## 遗留（非本任务范围）

- moldingFoam bundle 在**退出期**发生堆破坏（`malloc_consolidate` 报错，
  argList 析构中触发），正常跑完也以非零码退出；零步运行同样复现，已定位到
  初始化路径，对照实验确认与 bundle 内 OpenFOAM 本体无关（原版算例 exit 0），
  属求解器侧问题，已按约定移交 moldingFoam 仓库修复。

## 非目标

- moldingFoam 项目本身的开发（外部仓库进行）；
- 注塑物理字典的字段重设计（沿用 case-contract 口径，等 T29 实测反馈回流）。

## 验收标准

- 仓库内无 openInjMoldSim 残留（代码 grep 为零；历史任务 / 评审文档除外）✅；
- `bun run verify` 全绿 ✅；
- 依赖面板只剩 moldingFoam 与 Gmsh ✅；
- 求解入口（foamRun + SOLVER_MODULE）保持单点可换 ✅
  （`services/moldingfoam.rs::SOLVER_MODULE`）。
