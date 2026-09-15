# T97 · 求解环境库重复守卫与部署前清理

- 阶段：D0（平台底座 / 求解环境）
- 依赖：无外部依赖（随时可做）；证据来源见
  [solver-lib-duplication-report.md](../reviews/solver-lib-duplication-report.md)
- 优先级：**P1**——它防的是「求解跑完、析构阶段才崩」的静默失败，而我们的验收恰好踩在这个坑上
- 状态：**进行中**（部署前清理、core/原生库布局探测已完成；真实 VM 验收待下一批）

## 目标

把「求解器库在加载路径上被映射两次」从「偶发崩溃、事后抓日志」变成**提交作业前的显式报错**，
并让部署不再把旧 bundle 的文件叠加到环境树上。

## 背景

037 退出崩溃的根因是同一求解模块被两个 dlopen 名字加载到**两个不同实体**（机理与实测见
[solver-lib-duplication-report.md](../reviews/solver-lib-duplication-report.md)）。当前代码有两处缺口：

1. **就绪探测不看库布局**：`src-tauri/src/commands/solver.rs:57` 只检查 `blockMesh` / `foamRun`
   是否可用，库被拆成两份时仍判「已就绪」，问题要到求解跑完、析构阶段才以 `malloc_consolidate`
   的形式暴露（`End` 之后再崩，最容易被误读成「跑完了」）；
2. **部署不清环境树**：`env_deploy_command`（`src-crates/kairos-core/src/services/vm.rs:370`）是
   `mkdir -p` + `tar -xJf` + 复核 bashrc，解压前不删旧树——新旧 bundle 的文件会叠加，
   而 `FOAM_USER_LIBBIN` 更不在我们管辖范围内（那边一旦残留一个名字，就构成「名字被拆开」的坏状态）。

## 范围

### 1. 就绪探测加「库重复」检查

- **不变式**（与上游对齐的口径）：对每个 dlopen 名字（`libmoldingFoam.so`、`libmoldingFoamSolver.so`），
  在 `FOAM_LIBBIN` / `FOAM_SITE_LIBBIN` / `FOAM_USER_LIBBIN` 上必须**只有一个实体可达**；
  且 `lib<名字>Solver.so` 必须是**同目录**下指向 `lib<名字>.so` 的相对符号链接。
- **实现**：脚本构造与判定都放 core（Rust 优先）；实体计数用
  `find <dir> -maxdepth 1 -name 'libmoldingFoam*'`（不跟随符号链接）+ 按 inode 去重
  （硬链接不算两份实体），环境变量由 `source <env_root>/openfoam14/etc/bashrc` 取。
- **判定方式**：出口码 / 结构化输出，命令层按**值**判定——不匹配 message 文本
  （沿用 `native_env_status` 的既有口径）。
- **提示文案要能照着修**：指出坏在哪个目录、涉及哪两个实体（路径 + sha256 前缀）
  与「该移除哪一份」，例如「`FOAM_USER_LIBBIN` 里 `libmoldingFoamSolver.so` 是实体而非符号链接，
  请删除它，改由归档解压出的符号链接提供」。

### 2. 部署前清理环境树

- `env_deploy_command` 在 `tar` 之前删掉 `<env_root>/openfoam14`（`.kairos-release-tag` 由部署末尾重写，
  不受影响），解压后仍走 `env_probe_command` 复核——避免「解压半截也算成功」与新旧文件叠加。

### 3. 测试与消融

- core 侧纯函数锁脚本原文与判定表（与现有命令构造用例同风格）；
- **L2 求解用例补一条断言：日志里 `Duplicate entry` 必须为 0**——双载能一路跑到 `End`，
  只在退出期的析构里崩，现有的「退出码 + 错误标记」判据抓不到它（v0.2.1 之前我们正是靠
  过渡兜底把崩溃当成功）；
- 消融：把符号链接改成实体 → 判定用例必须变红；把「先清后解压」的顺序反过来 → 顺序用例必须变红。

## 非目标

- **不改求解器侧**：打包形态与符号链接由 moldingFoam 仓库负责（v1.0.0 归档已核对无误）；
- **不引入上游的 `scripts/diag/check-lib-duplication.sh`**：跨仓库脚本不进我们的运行路径，
  逻辑按「计算/判定归 core」的约定自己实现；
- **不做自动修复**：自动删库风险高（删错一份就变成缺库），只报错 + 给可执行的修复指引。

## 交付物

- core：库重复检查的脚本构造 + 判定纯函数（含用例与消融记录）；
- 命令层：接进求解环境探测（若沿用现有 `SolverStatus.message` / `ready`，则**不动 DTO**，
  避免双端镜像与契约测试的连带改动）；
- 部署：`env_deploy_command` 的清理前置；
- 真机记录：在 VM 里人为造出「名字被拆开」的坏状态，确认探测报错、恢复后复跑通过。

## 验收标准

- 坏状态（两个实体 / 符号链接丢失）判**不就绪**并指出路径与修法；干净状态（单实体 + 相对符号链接）判就绪；
- `env_deploy_command` 用例锁住「先清后解压」的顺序，消融可红；
- 真机：人为造坏 → 探测报错；恢复 → 复跑 np4 时 `grep -c 'Duplicate entry'` = 0、
  `End` 之后退出码 0；
- `kairos-core` 行覆盖 100%、`bun run verify` 全绿。

## 执行顺序

与 [T29](../tasks/T29-real-solve-e2e.md) 的「干净部署复跑归档库」并行：T29 那次复跑正好用本任务的守卫
当判定工具，谁先落地都不阻塞对方。

## 批次记录

- **部署清理批次（已完成）**：`env_deploy_command` 在解压前删除受管的
  `<环境根>/openfoam14`，再创建根目录、解压并复核 bashrc；core 用例锁定命令顺序。
- **库布局守卫（core + 原生状态已接入）**：core 生成加载 bashrc 后的结构化守卫命令，
  检查三个库目录中的重复名字及 solver 相对软链接；原生状态通过 `libraryReady` 与独立提示暴露。
  仍待真实 VM 坏状态 / 恢复回归，以及按 inode 去重和路径摘要提示的增强。
