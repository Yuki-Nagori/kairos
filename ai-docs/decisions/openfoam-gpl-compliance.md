# openInjMoldSim / OpenFOAM 集成许可证合规结论（T09）

- 结论日期：2026-09-08
- 状态：**生效**。本结论是 T09 及后续所有求解器集成工作的合规红线，违反即回滚。

## 事实

1. openInjMoldSim 以 **GPL-3.0** 许可发布（继承 OpenFOAM 的 GPL 族许可）。
2. Kairos 本体以 **Apache-2.0** 许可发布。
3. GPL 与 Apache-2.0 **不能以链接方式组合**成一个程序（无论静态或动态链接、同一进程内插件调用，均可能被认定为衍生作品）。

## 采用的隔离方案：独立子进程 + 文件交换

Kairos 与求解器的全部交互仅通过：

1. **文件系统**：Kairos 写出 OpenFOAM case 目录（网格、场、字典），求解器读取；求解结果文件由 Kairos 读取；
2. **独立子进程**：Kairos 以 `std::process::Command` 启动独立的求解器进程（`decomposePar` / `openInjMoldSim`），通过 stdout 观测进度，通过进程信号终止。

这是 FSF 与主流法律共识认可的「聚合（mere aggregation）」方式：两个独立程序通过数据管道协作，Kairos 不成为 openInjMoldSim 的衍生作品。

## 红线（代码评审必查）

- 本仓库、安装包内的 Kairos 部分**不得包含**任何 GPL 源码、头文件片段、或 GPL 代码的非平凡改编；
- **不得**以 `#[link]`、FFI、同进程插件等方式加载 OpenFOAM / openInjMoldSim 的任何二进制或库；
- **不得**分发 openInjMoldSim / OpenFOAM 的修改版而不提供对应源码；
- Kairos 生成的 case 文件、结果文件是**纯数据**，不受 GPL 约束（Kairos 拥有全部所有权）；
- 若未来必须分发求解器二进制（T18 捆绑方案）：独立目录存放、附带其许可证文本与源码获取途径，Kairos 安装器与其解耦。

## 后果

- Kairos 保持 Apache-2.0，可自由闭源扩展（如未来商业模块）；
- 求解器缺失时 Kairos 必须优雅降级（功能不可用提示，而非崩溃）。
