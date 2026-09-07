# 开发时间线

按时间顺序记录 Kairos 的提交序列，**一条对应一个 commit**，便于对照 `git log` 回溯。起点为壳子完成（旧项目清理与命名收尾不记）。约定：此后每完成一个阶段追加一行并伴随一次提交。

| 时间（2026-09-07） | 阶段                 | 内容                                                                                                                                                                                                |
| ------------------ | -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 21:16              | 工程基础配置         | Node 工具链（Bun / Vite / Tailwind / Vitest / ESLint / Prettier / knip / husky）、编辑器规范、Apache-2.0 许可证、favicon                                                                            |
| 21:30              | Cargo 工作区与领域层 | `kairos-core`（models / services / error）分层；统一错误契约 `{ code, message }`；IPC 契约测试锁定序列化形状                                                                                        |
| 21:35              | Tauri 适配层         | `system_info` 探活命令（薄适配）；Kairos 命名与标识、1440×900 主窗口、全套平台图标                                                                                                                  |
| 21:45              | 前端工作台壳子       | 标题栏状态优先级（error > busy > info）、工作区三占位、IPC 网关（`CommandError` 归一化）与 `select` 切片订阅                                                                                        |
| 21:50              | AI 文档体系          | 根 `AGENTS.md`（文档路由 + 铁律速查）+ `ai-docs/`（ARCHITECTURE 权威文档、时间线）                                                                                                                  |
| 22:00              | README 与收尾        | README 定稿为「介绍 + 使用说明」；定位明确为注塑成型仿真（对标 Moldflow，复刻功能不碰代码/数据）；运行时冒烟通过，初始历史完成                                                                      |
| 22:25              | 路线图规划           | 求解器选型确认：openInjMoldSim（OpenFOAM 7，GPL-3.0，填充/保压/冷却，3D 网格路线）；任务拆解 T01–T18 入库 `ai-docs/tasks/`（总览 + 里程碑 M0–M5），性能预算与 GPL 隔离为全局红线                    | verify            |
| 22:45              | 需求扩展             | 新增 GPU 辅助计算要求：统一 wgpu 路线覆盖 NVIDIA / AMD / Intel / Apple 四厂商；新增任务 T19（GPU 基础设施）、T20（GPU 加速后处理算子），性能预算与架构红线同步更新                                  | verify            |
| 22:55              | 流程补强             | 整体优化与 review 固化为计划的一部分：新增循环任务 T21（里程碑评审与整体优化，八项清单），任务完成自查与里程碑评审规则写入全局原则与 AGENTS                                                         | verify            |
| 23:05              | T01 性能预算         | `ai-docs/perf-budget.md` 六项预算；基准套件上线：前端 tinybench（store 热路径，40–95 ns）、Rust criterion（IPC DTO 序列化 ~86 ns），基线已回填；vitest 5 已移除内置 bench，故前端基准直用 tinybench | verify + 基准运行 |
