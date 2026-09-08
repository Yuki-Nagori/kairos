# T28 · 仓库结构优化

- 阶段：M5 后整理（结构债清理）
- 依赖：—（纯重构，不改行为）
- 优先级：P2

## 目标

在不动行为的前提下，收敛三处随功能增长积累的结构债，让下一个功能阶段的改动热区变小、审查面变小。

## 现状评审结论（2026-09-08 全仓走查）

已健康、不需要动的部分：

- Cargo workspace 三层（`kairos-core` 纯领域 / `src-tauri` 适配 / `src-web` UI），GPL 隔离已物理落地；
- `src-tauri/gen/schemas/`、`dist/`、`target/` 均已 gitignore；`.vscode/` 仅提交共享配置两件；
- 前端 `services/`（IPC 调用）/ `lib/`（纯逻辑 + 同目录测试）/ `render/`（渲染）/ `components/` 分层清楚；
- `types.ts` 作为 Rust DTO 单文件镜像 + `kairos-core/tests/contract.rs` 契约测试锁定，是刻意设计，不拆。

## 范围（按优先级）

1. **P1 · state.ts 拆分**（760 行，聚合全部领域的动作函数，是全项目改动热区）
   拆为 `src-web/state/` 目录：`store.ts`（AppState + store + setError 等共享件）加按域分片
   `project.ts` / `materials.ts` / `geometry.ts` / `jobs.ts` / `results.ts` / `dependencies.ts`；
   原 `state.ts` 保留为纯 re-export 入口，既有导入路径零改动，`state.test.ts` 不动或按片跟随。
2. **P2 · WGSL 着色器外置**：`commands/gpu_ops.rs` 内联 `r#""#` 着色器移到
   `src-tauri/shaders/*.wgsl`，经 `include_str!` 引入；为 T20 之后算子目录化铺路。
3. **P3 · components/ 分组**：15 个平铺文件按角色分组——业务面板进 `components/panels/`，
   框架件（app-header、project-bar）与共享控件 `ui.ts` 留顶层；纯移动 + import 更新。

## 非目标

- 不拆 `types.ts`（单文件镜像 + contract 测试是契约设计）；
- 不引入组件框架/构建工具变更（保持原生 DOM + Tailwind）；
- `kairos-core` 不拆 crate（`services/` 若再膨胀，届时另立 `kairos-solver` 任务）；
- 不补新测试，只保证既有 39 + 82 全绿。

## 交付物

- `src-web/state/` 目录 + 薄 `state.ts` 入口；
- `src-tauri/shaders/` 目录，gpu_ops.rs 引用外置着色器；
- `components/panels/` 子目录，import 全部更新。

## 验收标准

- `bun run verify` 全绿（前端 39 + Rust 82 测试、clippy/eslint/knip 零告警）；
- diff 只含移动与 import 调整，无行为变更（面板交互回归走查一遍）；
- knip 对新目录结构零告警（re-export 入口被正确识别为使用中）。
