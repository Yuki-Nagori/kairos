# T02 · CI/CD 流水线

- 阶段：M0 地基
- 依赖：无
- 优先级：P0

## 目标

把本地 `bun run verify` 门禁升级为多平台自动化门禁，让每个 PR 与主干提交都被同一把尺子度量，构建产物可随时取用。

## 范围

- GitHub Actions 工作流：
  - 门禁 job：typecheck / lint / format / test / knip（即 verify 的分解），Rust 与前端并行；
  - 平台矩阵：至少 macOS + Windows，Ubuntu 可选；
  - cargo 与 bun 依赖缓存；
- 主干构建 job：产出桌面安装包工件并保留；
- 失败通知与必过分支保护建议文档。

## 非目标

- 代码签名与公证（属 T18）；
- 性能回归卡点（属 T01，CI 先只出报告）；
- 自动发布更新通道（属 T18）。

## 交付物

- `.github/workflows/` 门禁与构建工作流；
- CI 说明文档（触发规则、缓存策略、如何看产物）。

## 验收标准

- PR 提交自动触发全量门禁，Rust warning（clippy -D warnings）与格式问题会打回；
- 主干每次合并产出 macOS / Windows 安装包工件；
- 无缓存的冷构建时长控制在可接受范围（记录并公示）。
