# Kairos

注塑成型 CAE 仿真软件，对标行业领先的同类产品：覆盖**前处理（制品/模具建模与网格）→ 求解（填充、保压、冷却、翘曲）→ 后处理（结果可视化与报告）**的完整仿真工作流。

当前处于框架搭建阶段：技术栈与工程化设施全部就位，项目管理、材料数据库、STL 几何导入（健康检查）、3D 体积网格生成（体素 + 5-四面体保形分解）、浇口流道与冷却水路、成型工艺设置、OpenFOAM/openInjMoldSim 求解集成（作业调度 + 进度流）、结果读取与统计、3D 视口（WebGL2）、XY 曲线与探针、报告导出、GPU 探测与算子（wgpu 跨厂商）已落地。

技术栈：Tauri 2 + Bun + TypeScript + Vite + Tailwind CSS v4；Rust 侧为 Cargo 工作区（领域层 + Tauri 适配层）。架构与开发约定见 [AGENTS.md](AGENTS.md) 及 [ai-docs/](ai-docs/)。

## 环境要求

- [Bun](https://bun.sh) ≥ 1.2
- [Rust](https://rustup.rs) stable（macOS 需要 Xcode Command Line Tools）

## 快速开始

```bash
bun install           # 安装前端依赖（Rust 依赖在首次构建时自动拉取）
bun run tauri dev     # 启动桌面应用（热更新）
bun run tauri build   # 打包安装包（macOS 下生成 .app / .dmg）
```

只改前端时也可以脱离 Rust 在浏览器里调试（IPC 调用会显示降级提示）：

```bash
bun run dev           # Vite 开发服务器（http://localhost:1420）
```

## 常用命令

| 命令                                            | 说明                                                                 |
| ----------------------------------------------- | -------------------------------------------------------------------- |
| `bun run dev`                                   | 仅启动前端（Vite，端口 1420，固定端口防止 Tauri 错连）               |
| `bun run build`                                 | 类型检查 + 前端生产构建（输出 `dist/`）                              |
| `bun run tauri dev`                             | 启动 Tauri 桌面应用                                                  |
| `bun run tauri build`                           | 打包桌面应用                                                         |
| `bun run typecheck`                             | TypeScript 类型检查（`tsc --noEmit`）                                |
| `bun run typecheck:rust`                        | Rust 编译检查（`cargo check --workspace`）                           |
| `bun run lint` / `lint:fix` / `lint:rust`       | ESLint 检查 / 自动修复 / cargo clippy（Rust warning 视为错误）       |
| `bun run format` / `format:check`               | Prettier 格式化 / 校验                                               |
| `bun run format:rust` / `format:rust:check`     | rustfmt 格式化 / 校验                                                |
| `bun run test` / `test:watch` / `test:coverage` | Vitest 单测                                                          |
| `bun run test:rust`                             | Rust 单测（`cargo test`）                                            |
| `bun run bench` / `cargo bench`                 | 性能基准（预算见 `ai-docs/perf-budget.md`）                          |
| `bun run knip`                                  | 检测未使用的文件、导出、依赖                                         |
| `bun run verify`                                | 一键全量门禁：前端 + Rust 的 typecheck → lint → format → test → knip |

## 说明

- 应用标识为 `com.yuki.kairos`（见 `src-tauri/tauri.conf.json`）；用自己的域名分发时改成反域名即可（改 ID 会切换本地数据目录）。
- 应用图标源是 `src-tauri/icons/icon.svg`，替换后运行 `bun run tauri icon src-tauri/icons/icon.svg` 重新生成（该命令会同时输出 iOS/Android 图标目录，本应用只面向桌面端，生成后删掉即可）。
- 已配置 husky pre-commit：每次 `git commit` 自动执行 `bun run verify`。紧急情况可用 `git commit --no-verify` 跳过；克隆后 `bun install` 会通过 `prepare` 脚本自动初始化钩子。
