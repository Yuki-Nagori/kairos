# T54 · 求解环境"更新未部署"提醒（VM 部署版本比对）

- 阶段：B3/D0（依赖管理与 VM 执行链路的衔接）
- 依赖：T27（组件下载）、T35/T36（VM 适配与执行链路）
- 优先级：P2

## 目标

消除「依赖面板更新完成」与「VM 内已部署版本」之间的静默错位：用户更新
moldingFoam 后若忘记点「部署到虚拟机」，提交的作业仍运行旧环境且 UI 无任何
提示。本任务让求解 / 依赖面板在检测到 **manifest 版本 ≠ VM 内已部署版本**
时显式提示"有新环境待部署"。

## 方案

- 部署（`vm_deploy_bundle`）成功后把 manifest 的 `releaseTag` 写入 VM 标记
  文件 `~/moldingfoam-env/.kairos-release-tag`；
- 新命令 `vm_deployed_release_tag` 读取标记文件（非 multipass 平台返回
  null——部署概念仅存在于 macOS+multipass 通道）；
- 纯函数 `isPendingDeploy(downloadedTag, deployedTag)`：downloadedTag 非空
  且 ≠ deployedTag（含 deployedTag 为 null 的"从未部署"）→ true；
- 依赖面板 moldingfoam 行与求解作业面板顶部展示待部署提示；部署成功后
  刷新已部署版本，提示消失。

## 验收标准

- 更新 bundle 后（未部署），依赖面板与求解面板出现"待部署"提示；
- 部署成功后提示消失；
- 非 release 流组件（无版本标签）不触发提示；
- 非 multipass 平台（Windows/Linux）不出现该提示；
- `parse_deployed_tag` / `isPendingDeploy` 纯函数单测锁定（含消融抽查）。

## 当前实现边界

- 比对粒度为版本标签字符串相等性，不做语义化版本比较；
- VM 内标记文件是唯一事实源，不做双向同步；
- WSL / Linux 本机执行通道无部署概念，提示仅 macOS 可见。
