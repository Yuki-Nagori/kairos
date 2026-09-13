# T85 · Linux 原生执行通道

- 阶段：D0（平台底座 · 求解执行）
- 依赖：T35（VM 适配层）、T36（case 与执行链路）、T27（依赖管理）
- 优先级：**P2**
- 状态：**待开工**（状态文档 D0 的未勾选项）

## 目标

Linux 上开箱可算：应用内下载 bundle、本机解压、提交作业即可跑，无需手工装 OpenFOAM。

## 现状

- macOS 走 Multipass、Windows 走 WSL2，两条 VM 通道已闭环；
- Linux 走**本机 bash**，OpenFOAM 需用户自装（`solve` 直接调 `foamRun`）；
- 依赖面板已有 bundle 下载与解压（宿主侧副本），但 Linux 分支没有「部署到本机」路径。

## 范围

1. **环境落地**：bundle 解压到应用数据目录（复用 `downloads/moldingfoam`），
   求解前注入 `source <env>/openfoam14/etc/bashrc`；版本标记与「更新未部署」提醒对齐
   （T54 的语义扩展到本机通道）；
2. **依赖检查**：OpenMPI、libmpi.so.40 等运行时的探测与提示（与 VM 内一致的判据）；
3. **执行路径统一**：作业执行仍走 core 的 `solve_command`（decomposePar + mpirun +
   reconstructPar），平台差异只在「环境前缀 + 工作目录」；
4. **面板文案**：VM 面板 / 依赖面板在 Linux 上不出现 VM 概念（避免「启动虚拟机」这类误导）。

## 非目标

- Linux 发行版矩阵（先覆盖 Ubuntu LTS 系）；
- 容器化执行。

## 验收标准

- Ubuntu 上应用内提交作业可完成（无手工装 OpenFOAM），与 macOS/Windows 行为一致；
- 依赖缺失时给出明确处置提示；
- `bun run verify` 全绿。
