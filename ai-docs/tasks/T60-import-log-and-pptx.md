# T60 · 导入日志流 + PPT 报告导出（低优先 / 可选拆分）

- 阶段：B1/B3（打磨项）
- 优先级：P4（回流自 R1 全项目对照评审；两项可独立做或拆分）
- 状态：**部分实现**（2026-09-13）——导入日志流已落地（`ImportOutcome` +
  `import_log` + 日志区展示）；**PPTX 导出未做**。
  依赖取向已定：**Rust 侧生成 pptx**（报告走 core → 命令层落盘，前端只触发与打开）。
  候选库（crates.io，均需先核许可与 API 覆盖）：`ppt-rs`（8.8 万下载，「create, read, update
  PowerPoint 2007+」）、`pptx`（约 8 千）、`pptxboss-write`（纯 Rust 确定性写 ECMA-376）。
  准入条件：许可为 permissive（GPL/AGPL 不得进进程内，见 AGENTS.md 的隔离约定）、
  能覆盖模板 + 表格 + 图片 + 中文排版；若不满足则回退「`zip` + 手写最小 OOXML」。
  见 [T60 评审](../reviews/T60-review.md)
- 来源：Moldflow 流程"建议打开显示导入日志"与"报告向导输出 PPT"

## 目标

1. **导入日志流**：几何导入（STL/STEP/IGES）过程向日志区输出详细日志
   （文件头 / 单位判定 / 三角形数 / 耗时 / 修复建议），复用 LogTabs；
2. **PPT 报告导出**：报告面板新增 PPTX 导出（基于既有 HTML 报告的
   六分区数据），作为 HTML 之外的交付格式。

## 验收标准

- 导入日志在日志区可见且可复制；不阻塞导入主流程；
- PPTX 在无网络环境可生成，分区与 HTML 报告一致；
- 两项各自独立提交、独立评审。
