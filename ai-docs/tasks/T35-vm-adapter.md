# T35 · 虚拟机适配层：Multipass / WSL2 一键安装 + 内嵌 Shell + 退出联动关机

- 阶段：进行中
- 依赖：T34（求解入口收口；moldingFoam 的 Linux 产物由用户侧 CI 打包，不在本任务范围）
- 优先级：**P1**

## 背景

moldingFoam 以 Linux 压缩包交付（用户 CI 打包）。macOS / Windows 桌面用户没有原生
Linux 环境：macOS 走 **Multipass**（Ubuntu 虚拟机），Windows 走 **WSL2**（openfoam.org
官方 macOS 路径即 Multipass；WSL2 是 Windows 官方的 Linux 子系统）。Kairos 需要：

1. 一键安装虚拟机运行时（无则装，有则跳过）；
2. 启动 / 初始化受管虚拟机实例（multipass 实例名 `kairos`；WSL 发行版 `Ubuntu-24.04`）；
3. 应用内嵌 Shell（Rust 持有子进程双向管道，TS 只渲染输出与输入框）；
4. 停止虚拟机；**应用退出时联动关闭虚拟机**（detach 一个 stop 进程，不等它退出）。

## 架构分层（按仓库铁律）

- `kairos-core`：纯逻辑——provider 判定、命令行参数构造、输出解析（含 WSL 的
  UTF-16LE 输出解码）、状态提示文案，全部带单测（覆盖率门槛适用）；
- `src-tauri`：进程副作用——探测 / 安装 / 启动 / Shell 子进程管理与流式回传、
  退出清理；macOS GUI 进程 PATH 需显式补充 `/opt/homebrew/bin`、`/usr/local/bin`；
- `src-web`：仅渲染——状态行、按钮组、日志 `<pre>`、输入框；动作全部在 state 分片。

## 平台适配要点

| 事项     | macOS（Multipass）                                                                 | Windows（WSL2）                                                                                      |
| -------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| 探测     | `multipass --version`                                                              | `wsl --status`                                                                                       |
| 安装     | `brew install --cask multipass`（无 brew 报错指引 multipass.run）                  | 提权 `Start-Process wsl -ArgumentList '--install' -Verb RunAs`（UAC 窗口，无法回传日志，可能需重启） |
| 实例     | `multipass info kairos` → 缺则 `launch --cpus 4 --mem 8G --disk 40G`，停则 `start` | `wsl -l -v` → 缺发行版则 `wsl --install -d Ubuntu-24.04`，WSL 实例随首次执行自启                     |
| Shell    | `multipass exec kairos -- bash --login`（管道友好，规避非 TTY 限制）               | `wsl -d Ubuntu-24.04`                                                                                |
| 停止     | `multipass stop kairos`                                                            | `wsl --terminate Ubuntu-24.04`                                                                       |
| 输出编码 | UTF-8                                                                              | `wsl` 系列为 UTF-16LE，core 解码函数统一处理                                                         |

## 已知限制（v1 明示）

- Shell 为行式管道流（非完整 VT 终端）：无颜色 / 行编辑 / Ctrl+C 转义；
  卡住可「停止 Shell」重开，完整交互可开系统终端用 `multipass shell` / `wsl`；
- Windows 安装走 UAC 提权，安装日志不可回传，完成后需「重新探测」；
- WSL 发行版固定 `Ubuntu-24.04`；用户自装其他发行版不会被受管流程识别。

## 与 moldingFoam README 的对齐（2026-09-09 确认）

- 规格与镜像对齐 README 2.2 节验证配方：`--cpus 8 --memory 16G --disk 80G
24.04`；实例名保留 `kairos`（Kairos 独立受管实例，不与 README 的 of14
  开发虚拟机混用，两台可共存）；
- WSL 命令（`wsl --install -d Ubuntu-24.04` / `wsl -d Ubuntu-24.04`）与
  README 2.3 节一致；
- 应用内 Shell 用 `multipass exec kairos -- bash --login`（README 的脚本式
  用法，管道友好）；交互完整版仍是系统终端里的 `multipass shell`。

README 带来的下一步关键信息（求解包进虚拟机任务用）：

1. VM 内 OpenFOAM 走 **apt 官方二进制 `openfoam14`**（`/opt/openfoam14`），
   不是源码全量编译；xmake 只编译 moldingFoam 本体；
2. macOS 仓库经 `multipass mount` 挂载 + `scripts/vm-sync.sh` 同步到 VM
   原生目录构建（挂载目录大小写不敏感，不能直接构建）；
3. 分发形态是 `xmake run bundle` 产出的 tar.xz（OpenFOAM 官方环境树 +
   libmoldingFoam 并入），解压后 `. etc/bashrc` 即用；
4. 已知坑：multipassd 下载器不继承用户网络配置（镜像可 `file://` 手动
   导入）；macOS 26+ 需在 系统设置→隐私与安全性→本地网络 给 App 开权限，
   否则 `multipass exec` 报 `No route to host`。

## 验收标准

- `bun run verify` 全绿；core 新增纯函数 100% 分支有测试；契约测试覆盖 VmStatus DTO；
- 依赖面板之外新增「虚拟机」面板：状态、安装、启动、进入 Shell、发送命令、
  停止 Shell、停止虚拟机均可操作；
- 应用退出（窗口关闭 / Cmd+Q）后，Shell 子进程被杀死且 stop 命令被派发。
