# T85 评审记录（T21 清单 · 随任务执行）

评审对象：`services::vm` 的原生环境路径 / 提示（core 纯函数）、`commands::jobs` 的原生求解
脚本与调度器注入、`commands::downloads::native_env_root`、`native_env_status` /
`native_deploy_bundle` / `vm_deployed_release_tag`（原生分支）、依赖面板与作业面板的原生措辞。

| #   | 清单项         | 结论                                                                                                                                                 |
| --- | -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | 架构一致性     | 通过。路径规则 / 命令构造 / 提示文案在 core（纯函数，零平台分支）；适配层只做 IO（目录扫描、写标记、起进程）                                         |
| 2   | 性能预算对照   | 通过。原生环境定位是受管目录下的一次浅扫描（深度 ≤4），只在启动 / 提交作业时执行，不进热路径                                                         |
| 3   | 代码与注释质量 | 通过。「env source 必须在 PATH 导出之前」这类顺序约束写进函数文档；`bash_single_quote` 单点收口转义                                                  |
| 4   | 文档同步       | 通过。architecture-status D0 更新；任务索引 T85 转已实现                                                                                             |
| 5   | 测试覆盖缺口   | 通过。core 3 个（路径布局、含空格/引号路径转义、依赖提示状态组合）；适配层 2 个（脚本顺序、可选段缺省）；前端 store 3 个 + 面板 4 个 + 作业面板 2 个 |
| 6   | 依赖健康       | 无新增依赖                                                                                                                                           |
| 7   | 安全           | 通过。路径进 bash 前一律单引号字面量转义（含 `'` → `'\''`），防路径注入；版本标记只写受管环境目录                                                    |
| 8   | 上轮遗留       | D0「Linux 需自装 OpenFOAM」在本轮 Kairos 侧闭环；真机验收见下                                                                                        |

## 口径与设计

- **不引入平台条件编译**：原生环境的判定按**值**（`VmProviderKind::Native` / `vm_shell` 是否
  为 None）而不是 `cfg`，因此 macOS 上就能跑完整单测（这是 T21 上一轮「平台 cfg 分支本地测不到」
  教训的直接落实）。
- **bundle 解压即就位**：Linux 上「部署」不做任何拷贝——`downloads/moldingfoam/**` 里含
  `openfoam14/etc/bashrc` 的目录就是环境根（多版本并存取最近修改），`native_deploy_bundle` 只
  做结构校验 + 写 `.kairos-release-tag`（与 VM 内同名标记，T54 的「更新未部署」提醒因此
  在 Linux 上同样生效）。
- **求解脚本顺序**：`source <env>/openfoam14/etc/bashrc; export PATH='<受管 bin>:$PATH'; cd '<case>' && decomposePar …`
  ——环境 source 在最前，受管 bin 目录随后追加（避免被环境树 PATH 覆盖）。
- **快照刷新**：调度器的原生环境在启动时注入，并在**每次提交作业前**重新扫描（`submit_job`
  开头调用 `refresh_native_env`），刚下载 / 解压的 bundle 立刻可用，不依赖前端协调。
- **面板措辞**：依赖面板按钮按 provider 显示「部署到本机」/「部署到虚拟机」，待部署提醒与作业
  面板提醒同步换词；原生平台额外显示 OpenMPI 缺失 / 环境未就绪的处置提示。

## 处理记录

- **立即修**：`run_job_body` 参数增至 8 个触发 clippy `too_many_arguments` → 抽出
  `RunContext`（受管 PATH + VM shell + 原生环境）随作业线程传递，顺带消掉 `fail_and_promote`
  的长参数表。
- **消融抽查**：把 `native_solve_script` 的 source 段挪到 `cd` 之后 →
  `native_solve_script_orders_source_path_and_solve` 变红（顺序断言）；文件级还原复绿。
- **回流任务**：无新增。
- **接受并记录**：**Linux 真机验收未做**——本机是 macOS，`apt` 安装 OpenMPI、bundle 解压后
  实际 `mpirun` 拉起 `foamRun` 的端到端只能在 CI（Ubuntu）或 Linux 机器上跑；本轮交付的是
  **本地可跑的完整单测**（core 路径与转义、脚本拼装、提示组合、面板措辞）+ 与 VM 通道同源的
  逻辑，真机复跑并入 T29 的验收清单（Linux job 提交一次）。
