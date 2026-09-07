# AGENTS.md

Kairos：注塑成型 CAE 仿真软件，功能对标 Autodesk Moldflow（复刻其功能，不使用其任何代码与数据）。本文件是面向 AI 编码代理的项目入口（路由），改代码前先按需阅读所引文档。

## 文档路由

| 需要了解                                                          | 去处                                               |
| ----------------------------------------------------------------- | -------------------------------------------------- |
| 分层规则、IPC/错误契约、长任务与进度回传模式、新增领域模块 recipe | [ai-docs/ARCHITECTURE.md](ai-docs/ARCHITECTURE.md) |
| 项目推进到哪了、每个阶段的时间与决策                              | [ai-docs/timeline.md](ai-docs/timeline.md)         |
| 未来路线图、任务拆解（做新功能前必读）                            | [ai-docs/tasks/README.md](ai-docs/tasks/README.md) |
| 面向人的项目简介、环境要求、快速开始、常用命令                    | [README.md](README.md)                             |

## 不可违反的约定（速查）

- **目录职责**：`src-web/` 前端（TypeScript）；`src-crates/` Rust 领域层 crate；`src-tauri/` Tauri 桌面适配层。各目录内部的 `src/` 是 Rust crate 固定结构，勿混淆。
- **依赖方向**：前端 `components → state → services → lib`；Rust `src-tauri → kairos-core`。`kairos-core` 禁止依赖 tauri；命令层不写业务逻辑，业务只住 core。
- **错误契约**：命令一律返回 `Result<T, KairosError>`，跨 IPC 序列化为 `{ code, message }`；`code ∈ validation / not_found / io / solver / internal`。前端按 `code` 分支，禁止文本匹配 message。
- **DTO 双端镜像**：Rust `models/` ↔ `src-web/types.ts`，任何改动必须同步两处并让契约测试（`src-crates/kairos-core/tests/contract.rs`）通过。
- **线程模型**：同步 Tauri 命令跑在主线程，重计算必须异步 / 另起线程；进度回传用 `tauri::ipc::Channel`；大体积数据用 `tauri::ipc::Response`。
- **锁文件**：根 `Cargo.lock` 与 `bun.lock` 必须提交、保持同步（整个工作区只有根目录这一份 Cargo.lock）。
- **提交前门禁**：仓库根 `bun run verify`（typecheck + clippy -D warnings + format + test + knip，前端与 Rust 全量），通过才算完成。
- **时间线**：每完成一个阶段，先提交代码，再在 [ai-docs/timeline.md](ai-docs/timeline.md) 末尾追加一行（时间 + 阶段 + 内容），**一条对应一个 commit**；时间线改动随下一个提交入库。

详细理由与代码模板见 [ai-docs/ARCHITECTURE.md](ai-docs/ARCHITECTURE.md)。
