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

### Web GUI 回归（默认快速路径）

日常 GUI 回归直接跑 Web 端，不需要先打包 Tauri：

```bash
bun run test:web                         # 前端组件、store、composable 与渲染工具
bun run dev -- --host 127.0.0.1         # 需要手工浏览器走查时启动 Web 预览
```

Web 测试通过 mock IPC 覆盖页面状态和交互流程，适合每次提交和重构后的快速回归。Tauri 打包只在发布候选、原生窗口/菜单/文件对话框/真实 IPC 变更后做桌面冒烟；VM 求解链路仍按下面的 Rust L2 真机层执行。

### 真实 GPU 视口回归

启动上述 Vite 服务后打开 `/tests/viewport-browser.html`（默认端口 1420）。
该夹具仅替代 Rust 网格读取，保留真实 Pinia、Vue 视口组件与 WebGPU/WebGL2 后端；
happy-dom 的渲染器 mock 不能验证 WGSL 编译或实际像素，不能替代这一步。

1. 点击导入 10 mm 立方体：无需刷新，出现完整模型。
2. 改为 1000 mm 立方体：模型自动取景，缩放、复位后仍可见。
3. 切换四分格：四个画布均出现模型；切回单视口后正常工作。
4. 删除几何后恢复空态，再导入仍可显示；浏览器控制台无 GPU 错误。

2026-09-15 在 macOS Edge 实测单视口大小模型与四分格出图、删除空态。
夹具不覆盖原生文件选择、STL/STEP 解析和真实 IPC；这些仍由对应 Rust 测试及桌面冒烟验证。

## Rust（cargo test --workspace）

| 位置                                                                      | 内容                                                                                                                                           | 跑法                                                               |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `src-crates/kairos-core/src/**`（`#[cfg(test)] mod tests` / `gap_tests`） | 领域单元测试（Rust 惯例：与源码同文件，无法外移）                                                                                              | `cargo test -p kairos-core --lib`                                  |
| `src-tauri/src/commands/**`                                               | 适配层单元测试（下载清单 / GPU）                                                                                                               | `cargo test -p kairos --lib`                                       |
| `tests/rust/contract/main.rs`                                             | DTO 契约测试（serde 形态锁定）                                                                                                                 | `cargo test -p kairos-tests --test contract`                       |
| `tests/rust/e2e/main.rs`                                                  | 全流程集成测试：L1 无条件（几何 → 网格 → case → 结果扫描）；L2 真机 VM（case → 求解 → 回传 → 读场 + 改坏 case 的失败可见性 + 启动后 2 秒取消） | `cargo test -p kairos-tests --test e2e`（L2 加 `KAIROS_E2E_VM=1`） |

- 覆盖率门槛（kairos-core 行 100%）：`bun run coverage:rust`（cargo-llvm-cov，统计口径 = lib 单元测试）；非 rustup 管理的 rustc（如 Homebrew）由 `scripts/coverage-rust.mts` 自动定位 LLVM 工具。
- 新增 Rust 集成测试：在 `tests/rust/<分类>/main.rs` 落文件（根包的 `[[test]]` 目标自动发现），并在上方表格登记。

### L2 真机层（全流程集成测试，需真机条件）

前置：macOS + multipass、名为 `kairos` 的实例（`multipass list` 可见）、虚拟机内已部署求解环境
（依赖面板「部署到虚拟机」，或 `multipass exec kairos -- test -f ~/moldingfoam-env/openfoam14/etc/bashrc`）。
缺任何一项时**显式开关下即失败**，不会静默放过：

```bash
KAIROS_E2E_VM=1 cargo test -p kairos-tests --test e2e -- --nocapture   # 全流程（约 15 秒）
KAIROS_E2E_VM=1 KAIROS_E2E_KEEP=1 cargo test -p kairos-tests --test e2e # 保留现场（打印工作区路径）
```

覆盖：case 进 VM（打包 → 传输 → 解压）→ 脱离会话求解到退出码 0 → 结果回传 → 宿主侧扫描并读出速度场；
再把 case 改坏（删 `constant/polyMesh/boundary`）跑一遍，断言**求解器错误标记**与可归因的失败原因
（日志标记与进程退出码共同判定失败；重建不会覆盖分解或求解的非零退出码）。
最后跑一遍取消：启动后 2 秒按会话 id 整组终止（`solver_stop_command`），断言退出码非 0、日志不再增长、
且不出现求解器错误标记（取消不能被误报成失败）。取消与启动必须在**同一次 multipass 往返**里发出——
一次往返 1~2 秒，小算例墙钟只有几秒，分两次调用会撞上「求解已结束」。

**发版前手动门禁**：发布候选版本时跑一次上面的 L2（CI 的三端托管 runner 没有 multipass 与 120MB 求解环境，
跑不了这一层），把输出与 case 路径记入当次发布记录。层级与剩余范围见
[T96](../ai-docs/tasks/T96-full-flow-e2e-done.md)。

## 原则

- 接缝靠集成层守：单元测试盯不住「命令怎么拼、分步怎么排、成败怎么判」，全流程链路按 [T96](../ai-docs/tasks/T96-full-flow-e2e-done.md) 的分层跑（L1 进 CI，L2 真机）；shell 命令的**语法**用 `bash -n` 做契约用例（引号失衡只会在真机上报错）。

- 前端测试与源码分离（本目录）；Rust 单元测试与源码同文件（语言惯例），集成测试归集于此；
- 测试不写业务逻辑——被测逻辑一律住 `kairos-core`；
- 覆盖率门槛已并入 verify 门禁，本地与 CI 同卡点（前端四维 100% / Rust core 行 100%）。

## 工程与作业一致性回归

- 前端 `tests/web/stores/{app,project,geometry,results}.test.ts` 与视口测试：并发/嵌套忙碌状态、切换保存、会话重置、网格归属、探针不污染主场、旧视口请求失效。
- core `services/{jobs,paths,project,workspace,moldingfoam,vm,results,render_mesh}.rs`：核数和 ID 约束、独立运行、退出码、日志帧、只读采样、二进制编码。
- 桌面 `commands/{jobs,geometry,results}.rs`：真实子进程取消、双流和非 UTF-8 日志、网格快照修订、场缓存与双槽一致性。Unix 子进程测试通过不等于 WSL/VM 已验收。
- [修复及消融记录](../ai-docs/reviews/review-fixes-0915.md)。
