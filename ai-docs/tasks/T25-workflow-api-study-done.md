# T25 · 第三方工作流 API 分层概念研究

- 阶段：下一阶段（研究）
- 依赖：—
- 优先级：P2

## 目标

研究成熟的注塑仿真 Python 封装（Apache-2.0 官方开源）的 API 分层概念——它把仿真工作流拆为 study / mesh / material / plot 等模块——并将可借鉴的分层与命名思想对照 Kairos 的 services 层，产出概念对照笔记。仅概念研究：不引入该依赖到 Kairos（它绑定 Windows 桌面环境与商业软件安装）。

## 范围

- 该封装的模块划分、对象生命周期、错误处理风格梳理；
- 与 Kairos `services` 层的逐模块对照表（能力覆盖 / 缺口 / 命名建议）；
- 对 Kairos 未来脚本化 / 批处理接口（CLI、Python 绑定）的设计建议输入。

## 非目标

- 引入该依赖或互操作；
- 复制其代码或文档内容（仅借鉴公开 API 的分层概念）。

## 交付物

- `ai-docs/research/api-layering-notes.md` 概念对照笔记；
- 对 Kairos services 层的改进建议清单（如有）。

## 验收标准

- 对照表覆盖五大模块（study / mesh / material / process / results）；
- 每条改进建议标注「采纳 / 缓办」并给出理由。
