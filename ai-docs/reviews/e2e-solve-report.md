# E2E 求解验证报告（T29 真机首跑 · 2026-09-09）

环境：macOS（Apple Silicon）→ Multipass VM `kairos`（Ubuntu 24.04 arm64，5核/12G/80G）
→ moldingFoam bundle v0.1.1（linuxArm64GccDPInt32Opt，113MB）
Case：kairos-cli pipeline run --sample-box 生成的 contract v1.1 case（5000 四面体）

## 链路验证结果

| 环节                                              | 结果                                                                   |
| ------------------------------------------------- | ---------------------------------------------------------------------- |
| VM 创建/启动（multipass，8核/16G/80G/24.04）      | ✅                                                                     |
| 云镜像国内镜像源下载（TUNA）                      | ✅                                                                     |
| bundle 下载 + GitHub API 按架构解析资产           | ✅                                                                     |
| bundle transfer 进 VM + 解压（~/moldingfoam-env） | ✅                                                                     |
| Kairos 生成 case 的字典被求解器接受               | ✅ Tait/hMelt 熔体相、perfectGas 空气相、CrossWlf 黏度全部选择回显成功 |
| polyMesh（体素 5000 四面体）读取                  | ✅                                                                     |
| 求解迭代                                          | ❌ **SIGILL 崩溃（阻塞项，见下）**                                     |

## 阻塞问题（求解器侧，按约定转用户优化）

**现象**：foamRun 在 `Selecting generalised Newtonian model CrossWlf` 之后立即
`Illegal instruction`（SIGILL, exit 132）崩溃。

**判别实验**：moldingFoam 自带的 `case-contract/`（blockMesh 后原样运行）在
同一 VM、同一 bundle 上**复现同样崩溃** → 与 Kairos 生成的 case 无关。

**定位**：bundle 内 `libmoldingFoam.so` 的构建目标 CPU 特性高于用户
VM 的 CPU。疑似 xmake/CI 在 GitHub ARM runner 上编译时捕捉了 runner 的
CPU 特性（native/march 过高），用户 Apple Silicon 代际较旧执行到非法指令。

**建议修复（moldingFoam 仓库）**：编译 flags 显式回退基线
`-march=armv8-a`（或 xmake `set_values("toolchains", ...)` 对应项），
并在 release 页注明最低 CPU 要求；可用 `readelf -A libmoldingFoam.so`
对比本机 `/proc/cpuinfo` 的 Features 验证。

## 连带发现（Kairos/VM 侧）

- VM 内需安装 openmpi（`libopenmpi-dev openmpi-bin`），否则 foamRun 缺
  `libmpi.so.40`；`vm_deploy_bundle` 后续应自动 apt 安装（待办）；
- multipass mount 的 sshfs 权限映射不可用（子目录全 Permission denied），
  作业执行已改为 tar 管道复制进 VM 原生文件系统（本轮已实现）；
- 求解器 libs 双路径加载产生 `Duplicate entry` 非致命警告（Kairos 不改，
  bundle 侧可去除 libmoldingFoamSolver.so 链接消除）。

## 验收结论

环境与数据链路全部打通；求解执行阻塞于 bundle 二进制兼容性（SIGILL），
按「求解器问题跳过并汇总」约定移交用户；T29 保持开放至修复后复跑。
