# E2E 求解验证报告（T29 · moldingFoam 真机复跑）

最后更新：2026-09-15（bundle v1.0.0 轮：4 进程并行复测通过，见 §0 状态表）
执行环境：macOS（Apple M4）→ Multipass VM `kairos`（Ubuntu 24.04 arm64，5 核 / 12 G / 80 G）
→ `kairos-cli` 无头链路 + 桌面应用（Tauri）链路。

## 0. 结论摘要

| 验证项                                                                 | 状态                                                                                                                     |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| 依赖面板下载 bundle（按架构解析 release 资产）                         | ✅ v0.2.1 / v0.2.5 实测可达                                                                                              |
| bundle 部署进 VM（tar 传输 → 解压 → 版本标记 → OpenMPI）               | ✅ 两轮实测，`.kairos-release-tag` 与下载版本一致                                                                        |
| case 生成（polyMesh + 0/ 场 + system/ 字典，contract v1.1）            | ✅ 样例盒与真实件均通过 `checkMesh`                                                                                      |
| 求解（`decomposePar` → `mpirun foamRun -parallel` → `reconstructPar`） | ✅ 样例盒 4 进程跑到 endTime；真实件 1/2 进程跑到 endTime                                                                |
| 结果回传 + 读取（时间步扫描、四场统计）                                | ✅ 宿主侧 `results` 服务可读                                                                                             |
| 桌面应用作业链路（提交 → VM 复制 → 求解 → 日志回传）                   | ⚠️ 复制 / 分解 / 启动均正常，但求解进程中途消失、状态不收敛（§6 遗留 1）                                                 |
| 前端结果面板 / 视口（结果清单、色标读数）                              | ✅ 小 case GUI 复跑核验（§6）；截图与 XY 图表探针待可截图会话                                                            |
| 4 进程并行（v1.0.0）                                                   | ✅ 通过：78,125 四面体件走完整链路（transfer → setsid → `mpirun -np 4`）退出码 0、2220 步到 endTime、结果回传 + CLI 读场 |

**一句话**：case 生成 / 部署 / 回传 / 读取与前端展示链路可用；桌面端作业进程的生命周期管理
（求解进程中途消失、状态不收敛）与求解器的真实件多进程并行是当前两个未闭环项。

> **执行条件说明（2026-09-15 核对后补）**：上表 v1.0.0 那一行的 np4 复跑，是在 VM 里
> **手工禁用了 bundle 自带库、改由 `FOAM_USER_LIBBIN` 下的一份源码重建库生效**的条件下完成的
> （重建库 sha256 `7d4c7965…`，归档里的库 `76e5e5e3…`，两者不是同一份构建）。也就是说它验证了
> 「求解链路 + 重建库」，**没有验证上游归档里的那份库**。归档本身是修好的形态（单实体 + 相对符号链接），
> 干净部署复跑作为剩余项记在 T29；双载的判据、复现与证据见
> [solver-lib-duplication-report.md](solver-lib-duplication-report.md)。

## 1. 环境与产物

| 项             | 值                                                                                                                                                                                                                      |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| bundle（当前） | `moldingFoam-v1.0.0-arm64`，113,871,868 B，sha256 `ecbd8b09…`（官方 release digest，本机下载后核对一致），发布 2026-09-14                                                                                               |
| bundle（上轮） | `moldingFoam-v0.2.5-arm64`，113,869,420 B，sha256 `b36b9f33…`，发布 2026-09-13T15:37Z                                                                                                                                   |
| bundle（首轮） | moldingFoam v0.2.1（113,818,536 B）                                                                                                                                                                                     |
| 部署路径       | `~/moldingfoam-env`（VM 内），`downloads/moldingfoam`（宿主解压副本）                                                                                                                                                   |
| 求解命令       | `decomposePar -force && mpirun -np <cores> foamRun -parallel; reconstructPar`                                                                                                                                           |
| case 契约      | v1.1：`constant/{moldingDict,momentumTransport,physicalProperties.*,phaseProperties,g,fvModels}`、`system/{controlDict,decomposeParDict,fvSchemes,fvSolution}`、`0/{alpha.melt,p,p_rgh,T,U}`；坐标以米写入（mm × 1e-3） |

## 2. 链路①：样例方盒（10 mm 立方体）

|                       | v0.2.1 轮                                                     | v0.2.5 轮                                                                                    |
| --------------------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| 网格                  | 1331 节点 / 5000 四面体（1 mm 体素）                          | 同左（另跑过 9261 / 40000，0.5 mm 体素）                                                     |
| `checkMesh`           | Mesh OK：体积精确 1000 mm³、最大偏斜 0.25、单连通域           | 同左                                                                                         |
| 求解                  | exit 0，无 NaN、无堆破坏，21 个时间目录（0…2 s），4 进程 26 s | 同左                                                                                         |
| V/P 切换              | —（尚未形成稳定切换）                                         | t = 1.0419 s，填充分数 0.9624（switchFraction 0.96）                                         |
| 保压                  | p_rgh 仅 0.15 MPa（未进入保压量级）                           | p_gate 30.4 → 58.5 MPa 跟随 60 MPa 斜坡                                                      |
| t = 2 s 场            | α 均值 0.751                                                  | α 均值 **0.990**（min 0.600）；T 364–518 K；p_rgh 57.6 MPa（std 0.97 kPa）；\|U\| ≤ 2.4 mm/s |
| 分层填充（按 z 层 α） | 1.000 … 0.576（顶层角部残留空腔，前沿正常推进）               | 整体 0.99 量级，无残留空腔读数                                                               |

首轮遗留的「填不满 / 门控压力上不去」在 v0.2.5 不复现，根因与修复见 §5.4。

## 3. 链路②：真实 STL（Bulbasaur 控制器支架，466,988 面）

素材：`samples/stl/Bulbasaur Controller Stand - 7406183`（Thingiverse 用户 TriDimen，
CC BY-NC-SA；`samples/` 在 `.gitignore` 中，不随仓库分发）。包围盒
111.1 × 167.4 × 132.9 mm，三角面有向体积和 880.8 cm³。

|                     | 首轮（v0.2.1）                                                         | v0.2.5 轮                                                                                                                                                                                        |
| ------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 导入 + 体素网格     | 5 / 4 / 3 mm 三档：体积保真 99.8 / 99.8 / 99.3%，零负体积单元，Mesh OK | 5 mm：9584 节点 / 37540 四面体，Mesh OK，非正体积 0                                                                                                                                              |
| 入口面口径          | inlet 382 面中 **39% 为侧向面**（熔体从外壁注入）                      | inlet 234 面**全部水平**（\|nz\| ≥ 0.9），侧向 0%                                                                                                                                                |
| 求解                | **t ≈ 0.0134 s（填充 1.6%）发散**，发散时刻与 1/Q 成正比（空间事件）   | np=1 / np=2 **跑到 endTime 2 s**（5083 / 5080 步，exit 0）：V/P 切换 t = 1.0802 s（分数 0.9607）、保压 6.9 → 59.3 MPa、无 NaN                                                                    |
| 并行                | —                                                                      | np=4 **死锁**：第 1 个时间步后不再前进，4 rank 常驻 90% CPU、无输出无报错。上游定位为「边界 patch 循环内归约」（各 rank patch 数 5/6/5/6 → 归约次数不等）、已修复并加回归用例，**v1.0.0 已发版** |
| 死锁定位            | —                                                                      | gdb attach：rank0/2 停在 `moldingFoam::preSolve → VoFSolver::preSolve → correctCoNum → gMax → PMPI_Recv`，rank1/3 停在 allreduce —— 不同 rank 走了不同通信路径                                   |
| 结果（np=1 回传后） | —                                                                      | 21 时间步；t = 2 场 13 个（含新出现的 `moldingStage` 等非场对象）；α 均值 0.9949、T 313–510 K、p_rgh 0.10–57.7 MPa                                                                               |

np=4 死锁已提上游：[moldingFoam#7](https://github.com/Yuki-Nagori/moldingFoam/issues/7)
（附 84 KB 最小复现 case；同一 case np=1/2 正常、np=4 必现）。
上游已修复（046：patch 循环内归约改为循环内局部求和 + 循环外一次归约，并跳过 processor patch；
新用例 `parallelMassBudget` + `run-solver-tests.sh` 并行超时门禁）；**v1.0.0 已发版**——
Kairos 侧复测通过（78,125 四面体件 np4：退出码 0、2220 步到 endTime、结果回传 + 读场）。
**在求解器修复前，真实件请用 1–2 进程提交。**

## 4. 集成缺陷与修复（Kairos 侧）

### 4.1 首轮（v0.2.1）

| #   | 缺陷                                                                      | 修复                                                                              |
| --- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| 1   | 保压压力表重复横坐标（固定写 `(0 1e5)` 后又追加曲线起点 t=0）             | 按契约写 `packing.switchPressure`（曲线起点压力），去掉大气压首点                 |
| 2   | `write_poly_mesh` 三数组错位（`faces`/`owner` 重排而 `neighbour` 未同步） | 三列表统一重排（内部面 → inlet → vent → walls），内部面按 (owner, neighbour) 升序 |
| 3   | `foamRun -parallel` 未经 mpirun 启动（"1 processor" 退出）                | `solve_command`：`mpirun -np <cores> foamRun -parallel`                           |
| 4   | 并行结果未重建（只在 `processor*/` 下）                                   | 追加 `reconstructPar`                                                             |
| 5   | 作业脚本 `cd` 宿主路径（VM 内不存在）                                     | `copy_case_into_vm` 返回 VM 内路径并据此执行；重跑前清理同名目录                  |
| 6   | 结果无回传（留在 VM 原生文件系统）                                        | `copy_results_from_vm`（tar 管道 + `results::time_dir_names` 筛查）               |

### 4.2 第二轮（v0.2.5，2026-09-14）

| #   | 缺陷                                                                                                                                | 修复与验证                                                                                                          |
| --- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| 1   | VM 内 case 解压多套一层同名目录（解压目标选成 case 目录本身）→ 求解器找不到 `system/controlDict`                                    | core `vm_case_extract_command`：按「归档条目自带叶子名前缀」解到 case 根；`services::vm` 单测                       |
| 2   | 求解器报错标记只在退出码非零时才算失败（求解命令用 `;` 串接 `reconstructPar`，求解中途报错仍可能以 0 退出）                         | core `job_failure`：错误标记优先于退出码；`services::jobs` 单测                                                     |
| 3   | 时间目录里的非场对象（如 `moldingStage`）被列进场清单，结果面板一点即报「暂不支持该场类型」                                         | `scan_times` 按 FoamFile 头 `class` 过滤（只读 512 B 前缀）；单测覆盖目录 / 缺失文件两分支                          |
| 4   | **GUI 提交的作业永不启动**：`submit_job` 先把作业提升为 Running，随后 `promote_and_spawn` 看不到待提升作业 → 作业永远停在「运行中」 | 提升与起线程收敛到 `promote_and_spawn` 单点；新增回归测试（消融：恢复旧写法即红，作业 10 s 内不退出运行态）         |
| 5   | 方案配置（材料 / 工艺 / 杆系 / 几何引用）编辑后不落盘，关掉再打开 `.kairos` 全部丢失                                                | `project` store 800 ms 防抖自动保存（写最新工程；无工程 / 无路径不排队），显式保存与另存为不变；store 单测 + 消融   |
| 6   | 工艺面板空表单要逐格填 9 个值才能应用；且「保压压力」回填取曲线末点、应用按峰值铺曲线，来回一次压力衰减 20%                         | 面板带出厂默认值 + 常驻「出厂默认」预设（可随时回填）；回填改取曲线峰值（幂等）；面板单测覆盖预设 / 重名拒绝 / 幂等 |

缺陷 4 是前两轮未暴露的关键项：前两轮走 CLI / 脚本链路，绕开了桌面端调度器；本次改由 GUI
提交后暴露。修复后复跑确认：case 复制进 VM → `decomposePar` 生成 `processor*/` →
`foamRun` 输出 `Selecting solver moldingFoam` → 日志回传到「分析日志」面板。

## 5. 上游（moldingFoam）事项

1. **v0.1.1 SIGILL**（v0.2.0 已修）：`libmoldingFoam.so` 含 M4 不支持的指令。
2. **退出期堆破坏**（v0.2.1 已修）：bundle 把同一求解模块打包成两份独立 `.so`，
   运行期两份都加载 → 18 条 `Duplicate entry … in runtime selection table` + 退出期
   `malloc_consolidate` 堆报告；改符号链接后 exit 0。v0.2.5 仍保持该修复。
3. **CI 掩盖**（已修）：`scripts/run-solver-tests.sh` 曾用 `foamRun … || true` 吞退出码。
4. **样例 case 失稳**（v0.2.2 + T66 已闭环）：原因是排气口按通用开放边界处理（密封逻辑
   不生效、入口流量 92% 逃逸）+ mm-as-m 单位制；改为 SI 坐标 / 流量 + `moldingVent*`
   - `ventSealAlpha` 后，样例盒复跑为 V/P 切换 t ≈ 1.16 s、填充 0.973、exit 0。
5. **4 进程并行**（已闭环，issue #7）：上游 046 定位根因（边界 patch 循环内归约 → 各 rank 归约次数不等）
   并修复，v1.0.0 发版；Kairos 侧在 78,125 四面体件上复测通过（退出码 0、2220 步到 endTime）。

## 6. 最小连通性复跑（2026-09-14，GUI 链路）

样例盒 10 mm 立方体 / 1 mm 体素（1331 节点 / 5000 四面体）/ 2 进程，全部经桌面应用操作：

| 环节                                              | 结果                                                                                                                                                                                      |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 新建项目 → 导入样例 → 目标尺寸 1.0 → 生成体积网格 | ✅ 5000 四面体，体积 1000.000                                                                                                                                                             |
| 材料「用于当前方案」                              | ✅，且**自动保存**已落盘（`project.kairos` 的 `materialId` 随即出现）                                                                                                                     |
| 工艺面板（表单带出厂默认值）→「校验并应用到方案」 | ✅ 一次点击完成，无需逐格填写                                                                                                                                                             |
| 求解「提交求解作业」                              | ⚠️ case 复制进 VM、`decomposePar` 产物（`processor0/1`）、`foamRun` 启动（日志回传 `Selecting solver moldingFoam`）均正常；**数十秒后 VM 内求解进程消失、作业仍停「运行中」**（见遗留 1） |
| 同一 case 在 VM 内手动重跑（与桌面端同一套命令）  | ✅ 跑到 endTime：31 个时间目录、V/P 切换 t = 1.6709 s（填充 0.9605）、保压 p_gate 59.9 MPa                                                                                                |
| 结果回传宿主 + 结果面板「扫描结果」               | ✅ 时间步 0…3.0 与每步可用场列出；**非场对象 `moldingStage` 已被过滤**（修复 4.2-3 生效）                                                                                                 |
| 视口「载入网格到视口」+「加载 T 场」              | ✅ 渲染控件由禁用转为可用（网格已渲染），色标读数 **506.40 / 498.11 / 353.28**，与 t = 3 s 的 T 场（353–506 K）一致                                                                       |
| XY 图表探针                                       | ⏳ 未做（加探针需键入节点号或点击画布，本会话输入通道被系统拒绝，见遗留 2）                                                                                                               |

## 7. 未决与遗留

1. **GUI 提交的作业中途消失**（新发现，已定位到机制，待改实现）：`foamRun` 在 VM 内启动后
   数十秒内进程消失（无输出、无时间目录），同一 case 手动重跑跑满 endTime。
   证据与机制：`multipass exec` 每次调用在 VM 内对应一个 ssh 会话，**会话关闭时
   systemd-logind 会连带回收该会话内的进程**（VM 日志可见 `session-*: Deactivated` →
   `Removed session`）；手动重跑之所以成功，是因为命令以 `nohup … &` 启动、脱离了会话进程组。
   即桌面端作业的存活期被绑在 `multipass exec` 客户端上，客户端一旦提前退出，求解随之被回收；
   界面侧另外两个问题（列表不刷新、失败原因不可见）已分别修复（自动轮询 + 失败行显示
   `job.message`）。
   修法（下一步）：远端命令改为**脱离会话**执行（`setsid` + 日志重定向到 case 内文件），
   客户端改为 `tail` 该日志流式回传并在结束时取退出状态；这样求解不再依赖 ssh 会话存活，
   进度回传与失败判定也都有据可依。
2. **前端视口 / 图表视觉核验**：本会话 Computer Use 的图片栅格不可交付（屏幕录制与辅助功能
   权限均已授予，但模型侧收不到图像），`screencapture` 亦受 TCC 限制；另输入通道在窗口不在
   前屏时拒绝键入。功能面已按 AX 树核验（结果清单、色标读数、控件可用性），
   **截图核验与 XY 图表探针待下一次可截图 / 可输入的前台会话补做**。
3. **并行**：上游 046 已修并入 v1.0.0；Kairos 侧 np4 复测通过（同规模 + 全链路口径），
   真实件原档已失、改按同规模验收。
4. **真实件工艺参数**：默认工艺对 880 cm³ 件偏激进（体积流量 879 cm³/s 超出常规注塑机
   包络 ≈ 500 cm³/s）；CLI 已给「建议注射时间 ≥ 1.8 s」提示，界面按件设置。
5. **（历史）样例 case 填充窗口**：v0.2.1 轮在 endTime 6 s / `maxAlphaCo` 0.03 下的失稳
   现象在 v0.2.5 不复现，记录保留供回溯。

## 8. 最小连通性复现（CLI，5 分钟量级）

```bash
# 0. VM 与求解环境（或走应用内「依赖面板 → 部署到虚拟机」）
multipass start kairos

# 1. 生成样例 case
kairos-cli pipeline run --sample-box --out-dir /tmp/e2e/box --cores 4 --target-size 1.0

# 2. 送进 VM 求解（与桌面端同一套命令）
tar -C /tmp/e2e/box/cases -czf - cli | \
  multipass exec kairos -- bash -lc "rm -rf /home/ubuntu/cli && tar -xzf - -C /home/ubuntu"
multipass exec kairos -- bash -lc \
  "source ~/moldingfoam-env/openfoam14/etc/bashrc; cd /home/ubuntu/cli && \
   decomposePar -force && mpirun -np 4 foamRun -parallel; reconstructPar"

# 3. 回传与读取
multipass exec kairos -- bash -lc "cd /home/ubuntu/cli && tar -czf - \$(ls -d [0-9]*)" | \
  tar -xzf - -C /tmp/e2e/box/cases/cli
kairos-cli results list --case-dir /tmp/e2e/box/cases/cli --json
kairos-cli results dump --case-dir /tmp/e2e/box/cases/cli --time 2 --field alpha.melt | head -c 200
```

桌面端等价路径：新建项目 → 几何「导入样例」→ 生成体积网格 → 材料「用于当前方案」→
工艺（表单已带出厂默认值）「校验并应用到方案」→ 求解「提交求解作业」→ 结果「扫描」。
