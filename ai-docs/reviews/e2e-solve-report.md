# E2E 求解验证报告（T29 真机复跑 · 2026-09-12）

环境：macOS（Apple M4）→ Multipass VM `kairos`（Ubuntu 24.04 arm64，5 核/12 G/80 G）
→ moldingFoam bundle **v0.2.1**（linuxArm64GccDPInt32Opt，113 MB，
发布日期 2026-09-12）→ 经应用内依赖面板下载、`vm_deploy_bundle` 部署进 VM。
Case：`kairos-cli pipeline run --sample-box` 生成的 contract v1.1 case
（10 mm 立方体，1331 节点 / 5000 四面体，4 进程并行，默认工艺：注射 1 s、
保压 60→40 MPa、模温 40 °C、熔温 230 °C、顶出 90 °C）。

## 1. 链路验证结果

| 环节                                                        | 结果                                                                |
| ----------------------------------------------------------- | ------------------------------------------------------------------- |
| bundle 下载（依赖面板 → `releases/latest`，按架构解析资产） | ✅ v0.2.1 arm64 资产，113,818,536 B                                 |
| 部署进 VM（tar 传输 + 解压 + 标记文件 + apt OpenMPI）       | ✅ `~/moldingfoam-env`，`.kairos-release-tag` = `v0.2.1`            |
| case 生成（polyMesh + 0/ 场 + system/ 字典）                | ✅ 1331 节点 / 5000 四面体                                          |
| `checkMesh`                                                 | ✅ **Mesh OK**（体积精确 1000、非正交 ~0、最大偏斜 0.25、单连通域） |
| 求解（`decomposePar` → `mpirun -np 4 foamRun -parallel`）   | ✅ 0 → 2 s 全程无 NaN，`exit 0`，无堆破坏、无 Duplicate 警告        |
| `reconstructPar` → case 级时间目录                          | ✅ 21 个时间目录（0, 0.1 … 2）                                      |
| 结果回传宿主（tar 管道）+ `kairos-cli results list/dump`    | ✅ 时间步列举与四个场（alpha.melt / T / p_rgh / U）均可读           |
| 前端视口/图表                                               | ⏳ 未做（桌面控制权限未授权，见 §5）                                |

耗时：整链 41 s（4 进程）。两个时间窗口的复跑（零步与完整链路）均为 `exit 0`。

## 2. 求解结果摘要（t = 2 s）

| 场           | 统计                                                                       |
| ------------ | -------------------------------------------------------------------------- |
| `alpha.melt` | 均值 0.751，min −1.3e−19（数值零），max 1.000                              |
| `T`          | min 288.2 K（模壁侧）/ max 503.2 K（熔体 230 °C-入口）/ 均值 486.6 K       |
| `p_rgh`      | 15.6 kPa … 0.43 MPa，均值 0.15 MPa（仍处填充阶段，远低于 60 MPa 切换压力） |
| `U`          | 0.003 … 36.9 m/s，均值 10.6 m/s（入口体积流量 Q = 型腔体积/注射时间折算）  |

填充推进（求解器自报填充分数，`moldingFoam: filling` 行）：

| t / s | 0.1   | 0.2   | 0.5   | 0.6   | 1.0   | 1.5   | 1.97  | 2.0   |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| 分数  | 0.095 | 0.196 | 0.496 | 0.569 | 0.677 | 0.716 | 0.747 | 0.750 |

分层填充（按 z 层平均 α，k=0 底 → k=9 顶）：
`1.000 / 0.986 / 0.855 / 0.753 / 0.695 / 0.691 / 0.674 / 0.646 / 0.636 / 0.576`
——前沿由底部（入口）推进到顶层，顶层角部残留空腔（顶层 166/500 单元 α ≤ 0.05），
是正常填充剖面；整体 0.75 的读数含界面过渡带（10 单元高的粗网格上约 18% 体积
处于 0.05 < α < 0.95）与顶层未充满部分。

## 3. 本轮修复的集成缺陷（Kairos 侧，已提交）

| #   | 缺陷                                                                    | 修复                                                                       |
| --- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| 1   | 保压压力表重复横坐标：固定写 `(0 1e5)` 后又追加曲线起点 t=0             | 按契约改为写 `packing.switchPressure` = 曲线起点压力，去掉大气压首点       |
| 2   | `write_poly_mesh` 三数组错位：`faces`/`owner` 重排而 `neighbour` 未同步 | 三列表统一重排（内部面→inlet→vent→walls），内部面按 (owner,neighbour) 升序 |
| 3   | `foamRun -parallel` 未经 mpirun 启动（"1 processor" 退出）              | `solve_command`：`mpirun -np <cores> foamRun -parallel`                    |
| 4   | 并行结果未重建：结果只在 `processor*/` 下                               | 追加 `reconstructPar`                                                      |
| 5   | 作业脚本 `cd` 宿主路径（VM 内不存在）                                   | `copy_case_into_vm` 返回 VM 内路径并据此执行；重跑前清理同名目录           |
| 6   | 结果无回传：结果留在 VM 原生文件系统                                    | `copy_results_from_vm`（tar 管道 + `results::time_dir_names` 筛查）        |

真机复核：修复 1/2 后 `checkMesh` 由「1968/5000 负体积单元、116 连通域」变为
**Mesh OK**；修复 3–6 后并行链路跑通并产出可读结果。

## 4. 上游（moldingFoam）缺陷与本轮结论

1. **v0.1.1 SIGILL**（v0.2.0 已修）：`libmoldingFoam.so` 含 M4 不支持的指令；
   v0.2.0 起正常迭代。
2. **退出期堆破坏**（v0.2.1 已修）：bundle 把同一求解模块打包成两份独立 .so
   （`libmoldingFoam.so` 新构建 + `libmoldingFoamSolver.so` 陈旧构建，均非符号链接），
   controlDict 的 `libs` 行与 foamRun 的模块探测各加载一份，运行日志出现
   18 条 `Duplicate entry … in runtime selection table`，退出析构时
   `malloc_consolidate` 堆破坏。A/B 判别：仅加载陈旧那份 → SIGILL(132)；
   把 `libmoldingFoamSolver.so` 改为指向 `libmoldingFoam.so` 的符号链接（只加载一份）
   → `exit 0`、无重复警告与堆报告。v0.2.1 已按此修复，本报告 §1 为复跑确认。
3. **CI 掩盖**（对方本轮自查发现并修复）：`scripts/run-solver-tests.sh` 曾用
   `foamRun … || true` 吞掉退出码，使「End 后崩溃」被判为全绿。

## 5. 未决与遗留

- **前端视口/图表验证未做**：macOS 对 ZCode Computer Use 的「辅助功能 + 屏幕录制」
  未授权，桌面控制 fail-closed；需授权后再补这一轮（含结果截图）。
  结果目录已备好：宿主 `/tmp/kairos-e2e/box6-results`。
- **真实 STL 一轮未跑**：本机（含 `~/eit`、`~/Downloads`）无任何 STL/STEP 素材，
  需要一份真实零件几何后再补；届时同时验证 STL 导入 → 体素网格 → case → 求解。
- **样例 case 的填充窗口与边值问题**（交 moldingFoam 侧判断）：
  - 注射 1 s 的名义流量下，t=2 s 时前沿刚推进到顶层、体积填充约 80%，
    α 均值读数 0.75；顶层角部有残留空腔；
  - 把 endTime 拉到 6 s：**t ≈ 2.05 s 起发散**（`alpha`/`p_rgh` 全 NaN），
    发散前一步门控压力由 0.26 MPa 跳到 0.51 MPa；
  - 把 `maxAlphaCo` 由 0.1 收紧到契约 case 的 0.03：发散提前到 t ≈ 0.46 s
    （同刻物理量与 0.1 配置一致，说明是稳定性边界附近的敏感行为而非轨迹差异）；
  - 契约 case 在相同设置下跑到 t = 0.54 s 未见发散，可作为对照。
- Kairos 侧的「退出期崩溃兜底」已在 v0.2.1 复跑确认后撤除（其带有
  `decomposePar` 也打印 `End` 的误判风险），恢复严格判定。

## 6. 验收结论

- 样例立方体填充分析在真实求解器上**跑通到 endTime 且结果可读**：满足；
  定量「填满」受粗网格（10 单元高）与填充窗口（2× 注射时间）限制，见 §5；
- 发现的集成缺陷（6 项 Kairos 侧）全部修复并加回归测试；上游 2 项已由
  moldingFoam 修复并复跑确认；
- 依赖面板下载 / bundle 部署流程实测可用（v0.2.1 全链复跑）；
- 待补：前端展示截图、真实 STL 一轮（见 §5）。
