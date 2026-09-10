# 注释规范

全仓库（Rust + TypeScript + Vue SFC）统一执行。

## 1. 风格：Doxygen 风格 + 中文

- 文件头：`/** @file xxx.ts @brief 一句话职责 */`
- 公共函数 / 接口 / 结构体：`@param` / `@return` / `@note`
- 模块内私有函数：行注释 `//` 即可，不强制 Doxygen

## 2. 精简，只写 Why

- 函数内注释解释**为什么**这样做，不逐行复述**做什么**。
- 如果一段代码不加注释也能看懂，就不需要注释。
- 注释过期比没有注释更糟糕——改实现时必须同步改注释。

## 3. 禁带内部规划引用

注释中禁止出现任务编号（Txx）、PLAN 章节、commit hash、review 引用等
内部规划信息。规划与进度追踪查 `ai-docs/tasks/` 和 `ai-docs/timeline.md`，
不进代码注释。

### 反例

```ts
// T36 Part 2 修复真机反馈，对标 Moldflow Insight 优化
export function createVmPanel(): HTMLElement {
```

### 正例

```ts
/** 虚拟机面板：安装 / 启动 / Shell / 关闭。 */
export function createVmPanel(): HTMLElement {
```

## 4. 中文

- 所有注释、工具 `description`、`z.string().describe(...)` 等
  发给模型或用户的内容用中文。
- 技术术语可保留英文原文（如 foamRun、VoF），不强行翻译。

## 5. 不标注来源或对比

- 不写「对标 xxx」「参考 xxx」「基于 xxx」「原 card() 工厂」等出处说明。
- 不写「真机踩坑」「真机反馈」等开发过程叙事。
- 如果某个决策的 Why 需要背景信息，用一句中文说清楚结论即可
  （例：「sshfs 权限映射不可用，改用 tar 管道」）。

## 执行

- 每次提交前自查：删除所有含 Txx / Moldflow / 真机 / 对标 的注释。
- 代码 review 时按本规范逐条检查。
