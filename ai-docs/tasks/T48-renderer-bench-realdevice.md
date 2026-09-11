# T48 · 渲染后端三端真机验收与 FPS 回填

- 阶段：待开工（回流自 T39 评审）
- 依赖：T39（后端 POC 与基准协议已就绪）
- 优先级：P1（B3 视口从 △ 转 [x] 的前置）

## 目标

在三个真实 Tauri WebView（macOS WKWebView / Windows WebView2 / Linux
WebKitGTK）上执行 T39 的基准协议并回填数字，关闭渲染后端决策门。

## 范围

- 三平台各执行 `bun src-web/utils/bench/webgpu-render.ts`：上传耗时、
  首帧、中位与 P05 FPS（确定性 999,698 三角形资产）、剖切切换延迟；
- 真机走查：旋转 / 缩放 / 云图着色 / 剖切（三轴 + 反向）/ 点击拾取 /
  探针时间曲线联动；
- 数字回填 ai-docs/reviews/T39-review.md 决策门表，B3 视口完成度转正。

## 验收标准

- 决策门四项（三端稳定性 / 表面性能 / 资源预算 / 架构隔离）均有真机数字
  或 ✅；数字与 T01 预算对照结论写入评审报告。
