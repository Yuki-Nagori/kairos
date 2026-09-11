# Kairos 测试地图

所有测试集结于根目录 `tests/`，按端与职能分类。跑法：`bun run verify`（全量）或下述单项命令。

## 前端（web 前端，vitest + happy-dom；与 src-web 镜像）

| 目录              | 内容                                                                    | 跑法                                   |
| ----------------- | ----------------------------------------------------------------------- | -------------------------------------- |
| `web/utils/`      | 纯逻辑单元测试（chart / ipc / pipeline / report / stats / environment） | `bunx vitest run tests/web/utils`      |
| `web/stores/`     | Pinia store 测试（app / project / vm / dependencies）                   | `bunx vitest run tests/web/stores`     |
| `web/components/` | UI 组件测试（AppHeader/StatusBar/VmPanel 及 ui 基础组件）               | `bunx vitest run tests/web/components` |
| `web/render/`     | 渲染数学工具（mat4 等）                                                 | `bunx vitest run tests/web/render`     |

- 覆盖率门槛（100%，四维）：`bun run test:coverage`（口径 = `src-web/utils/**` + `stores/**` + `composables/**` + 各 `use*.ts`，见 vitest.config.ts）。

## Rust（cargo test --workspace）

| 位置                                                                      | 内容                                              | 跑法                                         |
| ------------------------------------------------------------------------- | ------------------------------------------------- | -------------------------------------------- |
| `src-crates/kairos-core/src/**`（`#[cfg(test)] mod tests` / `gap_tests`） | 领域单元测试（Rust 惯例：与源码同文件，无法外移） | `cargo test -p kairos-core --lib`            |
| `src-tauri/src/commands/**`                                               | 适配层单元测试（下载清单 / GPU）                  | `cargo test -p kairos --lib`                 |
| `tests/rust/contract/main.rs`                                             | DTO 契约测试（serde 形态锁定）                    | `cargo test -p kairos-tests --test contract` |

- 覆盖率门槛（kairos-core 行 100%）：`bun run coverage:rust`（cargo-llvm-cov，统计口径 = lib 单元测试）；非 rustup 管理的 rustc（如 Homebrew）由 `scripts/coverage-rust.mts` 自动定位 LLVM 工具。
- 新增 Rust 集成测试：在 `tests/rust/<分类>/main.rs` 落文件（根包的 `[[test]]` 目标自动发现），并在上方表格登记。

## 原则

- 前端测试与源码分离（本目录）；Rust 单元测试与源码同文件（语言惯例），集成测试归集于此；
- 测试不写业务逻辑——被测逻辑一律住 `kairos-core`；
- 覆盖率门槛已并入 verify 门禁，本地与 CI 同卡点（前端四维 100% / Rust core 行 100%）。
