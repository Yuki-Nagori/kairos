# R1 整体评审记录（全项目 · 架构/代码/文档三维并行深查）

评审时间：2026-09-12。评审对象：全仓库（Rust 领域层 + Tauri 适配层 + 前端 +
文档体系），三维并行深查后逐项闭环。闭环纪律遵循
[T21 里程碑评审](../tasks/T21-milestone-review.md)：「立即修 / 回流任务 /
接受并记录」三分类。

## 发现与闭环总表

| #   | 维度     | 发现                                                                                                                        | 严重度         | 处理                                                                                                                              | 落点 commit |
| --- | -------- | --------------------------------------------------------------------------------------------------------------------------- | -------------- | --------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| 1   | 命令层   | `generate_gmsh_mesh` 的 target_size 被静默丢弃（`let _ = target_size`），用户设置不生效                                     | 高（功能缺陷） | 立即修：经 `-clmax` 透传 Gmsh，含正数校验                                                                                         | 38f846d     |
| 2   | 命令层   | `repair_geometry` 算完修复报告即丢弃，与 doc 注释不符                                                                       | 高             | 立即修：RepairReport 迁入 models/ + RepairOutcome DTO 跨 IPC 返回，面板渲染                                                       | 38f846d     |
| 3   | 重复     | gmsh 子进程编排 CLI 与桌面命令各写一份                                                                                      | 中             | 立即修：下沉 core `gmsh::tetrahedralize` 共用                                                                                     | 38f846d     |
| 4   | 线程模型 | multipass 的 case 目录 tar 复制跑在主线程，大 case 冻结 UI；tar 启动失败被静默跳过（无 case 也启动求解）                    | 高             | 立即修：复制移入作业线程，失败明确报错 + fail_and_promote 保持队列推进                                                            | f0f1061     |
| 5   | 线程模型 | `/releases/latest` 资产解析（阻塞网络 IO）在 spawn_blocking 之外                                                            | 中             | 立即修：移入阻塞线程                                                                                                              | f0f1061     |
| 6   | GPU      | gpu_ops 四个算子生产路径零调用，derive 走 CPU，与「GPU 为后处理运行前提」承诺矛盾；device 每次调用重建；管线模板重复 ~80 行 | 高（战略）     | 立即修：derive_field/difference 接 wgpu compute（async + spawn_blocking），进程级 device 缓存，公共尾段合并，死代码与误导话术清除 | 6ac5dc1     |
| 7   | 契约     | 8 个命令不返回 `Result`，违反「命令一律 Result」                                                                            | 中             | 立即修：全部包装（无失败路径包 Ok 保持口径）                                                                                      | f433637     |
| 8   | 分层     | 渲染网格提取（边界面+owner）~65 行领域逻辑住在命令层                                                                        | 中             | 立即修：下沉 core services/render_mesh + models/render DTO + 契约测试                                                             | f433637     |
| 9   | 前端分层 | 4 处 composable 绕过 store 直连 api 且 `.then()` 无 `.catch`（启动即 unhandled rejection）                                  | 高             | 立即修：全部下沉 store 走 app.setError 管道；process 校验新建 store 并改走 touchActiveStudy                                       | 49358ee     |
| 10  | 前端重复 | beginBusy/try/catch/finally 样板 ~20 处                                                                                     | 中             | 立即修：app store 新增 `withBusy` 统一编排并推广                                                                                  | 49358ee     |
| 11  | 错误契约 | `CommandError.code` 全前端零消费，契约有名无实                                                                              | 中             | 立即修：app.error 保留 code，状态栏渲染 code 徽章 + 五类处置提示；ipc 测试锁定全量 code                                           | 49358ee     |
| 12  | 渲染     | fitToMesh 三轴共用 min/max，网格不居中时视口瞄错                                                                            | 高（功能缺陷） | 立即修：逐轴包围盒                                                                                                                | 6c25f43     |
| 13  | 渲染     | 每帧 getUniformLocation ~10 次 + 矩阵每帧分配 + dispose 不释放 GL 资源 + 上下文恢复后 lineProgram 句柄失效                  | 中             | 立即修：uniform 缓存、math out 参数复用暂存、全量释放与恢复重建                                                                   | 6c25f43     |
| 14  | 渲染     | 两后端法线算法语义不一致（平滑 vs 平面覆写），代码 90% 重复                                                                 | 中             | 立即修：共享 render/normals.ts 平滑法线                                                                                           | 6c25f43     |
| 15  | 前端健壮 | 动画 400ms 固定节拍无背压（大场加载叠加排队）；图例逐帧全量排序                                                             | 低             | 立即修：加载中跳过节拍；quickselect O(n)                                                                                          | 6c25f43     |
| 16  | 前端边界 | store action 内直接 DOM 下载；materials 自定义 id 同毫秒可撞                                                                | 低             | 立即修：DOM 外移 utils/download.ts；id 加序列号                                                                                   | 281d951     |
| 17  | 文档     | architecture-status 里程碑表 B3 ~20% 与正文/timeline 的 ~75% 实锤矛盾；头部状态滞后（T44–T48 后）                           | 中             | 立即修：B3 ~75% 并立「百分比只在里程碑表维护」单一事实源条款                                                                      | (本 commit) |
| 18  | 文档     | tasks/README 缺 T51/T52/T53 行、T45–T47 重复三行                                                                            | 低             | 立即修：补齐 + 去重                                                                                                               | (本 commit) |
| 19  | 文档     | timeline 中间四天日期头丢失、时间戳漂移、重复行、273 commit 仅 ~219 行                                                      | 中             | 立即修：整体重建——直接从 git log 生成一行一 commit 的完整台账（280 条，真实时间）                                                 | (本 commit) |
| 20  | 文档     | AGENTS.md 契约测试路径过时（实际在 tests/rust/contract/main.rs）；README「框架搭建阶段」旧口径                              | 低             | 立即修                                                                                                                            | (本 commit) |

## 回流任务

- WebGL2 与 WebGPU 的渲染真机验收统一归 **T48**（含派生算子百万值 GPU
  性能、体渲染、FPS）。
- localStorage 持久化分散（5 处 key 命名不一致 + 工艺预设遍历扫描）：回流至
  研究结果持久化任务一并设计统一方案。
- useViewportPanel（428 行）仍是唯一被覆盖率口径排除的逻辑文件：其中
  quickselect/图例逻辑已可抽纯函数纳入口径，归 B3 持久化/懒加载批次。
- GPU 测试在无适配器环境直接失败的 CI 策略问题维持 D0 △ 项。

## 接受并记录

- 8 个无失败路径命令包装 `Result<Ok>` 的口径折中：换来契约全量统一，
  未来失败路径自动获得结构化错误语义。
- derive 派生 f64 场转 f32 参与 GPU 计算：可视化后处理精度，GPU↔CPU
  一致性测试以 1e-5 容差锁定。

## 验证

- `bun run verify`（typecheck + clippy -D warnings + format + 前端/Rust 全量
  测试 + 双端 100% 覆盖率门禁 + knip）逐 commit 全绿。
- 修复批次共 7 个 commit（38f846d … 边界修复），前端测试 474 → 498，
  Rust core 测试 246 → 255。
