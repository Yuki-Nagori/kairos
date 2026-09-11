# AGENTS.md

Kairos：注塑成型 CAE 仿真软件，对标行业领先的同类产品（自主实现全部功能，不使用任何第三方专有代码或数据）。本文件是面向 AI 编码代理的项目入口（路由），改代码前先按需阅读所引文档。

## 文档路由

| 需要了解                                                          | 去处                                                     |
| ----------------------------------------------------------------- | -------------------------------------------------------- |
| 分层规则、IPC/错误契约、长任务与进度回传模式、新增领域模块 recipe | [ai-docs/ARCHITECTURE.md](ai-docs/ARCHITECTURE.md)       |
| Web 前端职责边界、composable/store 约定、反模式清单               | [ai-docs/web-conventions.md](ai-docs/web-conventions.md) |
| 图标设计语言、双轨分类与强制参数                                  | [ai-docs/icon-design.md](ai-docs/icon-design.md)         |
| 项目推进到哪了、每个阶段的时间与决策                              | [ai-docs/timeline.md](ai-docs/timeline.md)               |
| 未来路线图、任务拆解（做新功能前必读）                            | [ai-docs/tasks/README.md](ai-docs/tasks/README.md)       |
| 测试地图：全部测试的位置、分类与跑法                              | [tests/README.md](tests/README.md)                       |
| 面向人的项目简介、环境要求、快速开始、常用命令                    | [README.md](README.md)                                   |

## 不可违反的约定（速查）

- **目录职责**：`src-web/` 前端（TypeScript）；`src-crates/` Rust 领域层 crate；`src-tauri/` Tauri 桌面适配层。各目录内部的 `src/` 是 Rust crate 固定结构，勿混淆。
- **依赖方向**：前端 `views / components → stores → api → utils / render`；Rust `src-tauri → kairos-core`。`kairos-core` 禁止依赖 tauri；命令层不写业务逻辑，业务只住 core。
- **错误契约**：命令一律返回 `Result<T, KairosError>`，跨 IPC 序列化为 `{ code, message }`；`code ∈ validation / not_found / io / solver / internal`。前端按 `code` 分支，禁止文本匹配 message。
- **DTO 双端镜像**：Rust `models/` ↔ `src-web/types/index.ts`，任何改动必须同步两处并让契约测试（`src-crates/kairos-core/tests/contract.rs`）通过。
- **线程模型**：同步 Tauri 命令跑在主线程，重计算必须异步 / 另起线程；进度回传用 `tauri::ipc::Channel`；大体积数据用 `tauri::ipc::Response`。
- **覆盖率门槛**：`kairos-core` 行覆盖 100%（`bun run coverage:rust`，cargo-llvm-cov）、前端逻辑层全量 100%（`utils/` + `stores/` + `composables/` + 各 `use*.ts`，`bun run test:coverage`，行/函数/语句/分支全 100）；api / render 薄适配层不计入门槛。两端均已并入 verify 门禁。
- **GPU 计算**：统一经 wgpu 抽象层覆盖 NVIDIA / AMD / Intel / Apple（Vulkan/DX12/Metal），禁止引入 CUDA 等单厂商 SDK；后处理以硬件加速 GPU 为运行前提，不提供 CPU 运行时回退。CPU 实现仅可作为正确性基准与测试参考；缺少可用 GPU 时必须明确提示该能力不受支持。
- **锁文件**：根 `Cargo.lock` 与 `bun.lock` 必须提交、保持同步（整个工作区只有根目录这一份 Cargo.lock）。
- **提交前门禁**：仓库根 `bun run verify`（typecheck + clippy -D warnings + format + test + 覆盖率门槛 + knip，前端与 Rust 全量），通过才算完成。
- **时间线**：每完成一个任务，在 [ai-docs/timeline.md](ai-docs/timeline.md) 末尾追加一行（时间 + 阶段 + 内容），**一行对应一个 commit**，随该任务的提交一并入库。
- **文档图表**：md 里的架构图 / 流程图 / 时序图优先用 mermaid 代码块（GitHub 与主流编辑器原生渲染），不用 ASCII 字符画；目录树保持纯文本代码块。新写图表后须经渲染校验（如 mermaid.ink）再入库。
- **里程碑评审**：每个里程碑结束强制执行 [T21 整体评审与优化](ai-docs/tasks/T21-milestone-review.md)（架构 / 性能 / 质量 / 文档 / 安全八项清单），发现按「立即修 / 回流任务 / 接受并记录」闭环，未通过不得开启下一里程碑。

详细理由与代码模板见 [ai-docs/ARCHITECTURE.md](ai-docs/ARCHITECTURE.md)。

- 测试地图见 [tests/README.md](tests/README.md)。
