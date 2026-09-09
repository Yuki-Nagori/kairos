# M6 评审记录（T31–T36 批次，逐任务触发）

## T31 触发评审（派生算子）

逐项核对（覆盖派生算子与结果面板改动）：

1. 架构一致性：✓ 算子在 src-tauri（GPU）+ core（CPU 参考），前端派生在组件层纯计算，无依赖违例；UpdateCheck/新 DTO 契约测试已锁；
2. 性能预算：GPU 一致性测试通过（1000 点）；≥5× 基准归入 T01 套件扩展（回流：T01 增补 derive 基准项）；
3. 代码质量：✓ 无 TODO 残留；【立即修】清理了 unused_mut 与 unnecessary_unwrap（本轮提交内）；
4. 文档同步：✓ T31 状态与 timeline 更新；
5. 测试覆盖：✓ 4 个 CPU 算子测试 + 3 个 GPU 一致性测试；派生 UI 断言 img/svg 资源；
6. 依赖健康：✓ knip 通过；未新增依赖；
7. 安全：✓ 未动 CSP 与 capabilities；
8. 上轮遗留：F 系列已闭环（前轮）。

分类结论：立即修 2 项（已当场修）；回流 1 项（T01 derive 基准）；接受 0 项。

## T32 触发评审（无头 CLI）

1. 架构一致性：✓ kairos-cli 仅依赖 core（零 tauri），结构化错误契约原样穿透；
2. 性能预算：✓ 生成路径为一次性行为，无预算项；
3. 代码质量：✓ 子命令分发单一 match；【立即修】solve 的 tail -5 会吞整段日志——回流：后续改流式输出（记录为 T32 已知项）；
4. 文档同步：✓ README CLI 段已加；
5. 测试覆盖：✓ 手工冒烟 sample-box 生成 1331 节点/5000 四面体 + JSON 模式；CLI crate 单测待补（接受：clap 派生的解析由集成冒烟覆盖）；
6. 依赖健康：✓ 新增 clap 4.5/serde（BSD/Apache）；
7. 安全：✓ 无网络/无敏感操作；
8. 上轮遗留：无。

分类结论：立即修 0；回流 1（solve 流式日志）；接受 1（CLI 单测以冒烟替代）。

## T32/T30/T33/T29 触发评审（CLI/Gmsh/加固/E2E 批次）

- T32（CLI）：✓ 独立 crate 零 tauri；solve tail -5 吞日志 → 回流（改流式，已记录 T32 已知项）；
- T30（Gmsh）：✓ core write_stl_binary + tetrahedralize_args 纯函数化；MeshingReport engine 字段双端镜像 + 契约测试；【立即修】无；
- T33（加固）：✓ updater 插件接入（端点占位）；CSP 收窄 img-src；style-src unsafe-inline 保留（主题注入依赖）→ 接受并记录（nonce 化改造待主题文件化）；
- T29（E2E）：环境链路全通；阻塞于 bundle SIGILL → 报告已出（e2e-solve-report.md），按约定移交用户。

分类结论：立即修 0；回流 2（T32 流式日志、T01 derive 基准）；接受 2（unsafe-inline、Duplicate entry 警告）。
