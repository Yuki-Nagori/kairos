# R1 消融测试报告

时间：2026-09-12。对象：[R1 整体评审](R1-full-project-review.md)修复批次
（38f846d … 2e87184）的每一个修复点。方法：

1. **测试保护消融**——逐修复点临时移除（还原为修复前行为），跑定向测试，
   确认回归被捕获，随后 `git checkout` 还原并核对工作树干净；
2. **GPU/CPU 消融计时**——对 GPU 接生产做规模扫描计时，对照
   perf-budget「GPU 辅助算子 ≥5×」预算，并量化 device 缓存的贡献；
3. 消融中暴露的缺口就地处置（加固 / 修真 bug / 接受并记录）。

## 一、测试保护消融矩阵

| #   | 消融点（还原为修复前行为）                    | 定向验证                                              | 结果                 | 保护测试                                                                      |
| --- | --------------------------------------------- | ----------------------------------------------------- | -------------------- | ----------------------------------------------------------------------------- |
| A1  | gmsh `-clmax` 目标尺寸透传移除                | `cargo test -p kairos-core --lib tetrahedralize_args` | ✅ 捕获（1 failed）  | `tetrahedralize_args_carries_target_size_as_clmax`                            |
| A2  | store 丢弃修复报告                            | `bun run test -- geometry.test`                       | ✅ 捕获（1 failed）  | store 测试断言 repairReports 记录                                             |
| A3  | GPU 归一化退化为恒等映射                      | `cargo test -p kairos --lib derive_normalize_gpu`     | ✅ 捕获（1 failed）  | GPU↔CPU 一致性 `derive_normalize_gpu_matches_cpu_reference`                   |
| A4  | withBusy 吞掉错误不进全局状态                 | `bun run test -- app.test`                            | ✅ 捕获（1 failed）  | withBusy 错误路由用例                                                         |
| A5  | 状态栏 CODE_HINTS 清空                        | `bun run test -- status-bar`                          | ✅ 捕获（1 failed）  | code 徽章与处置提示用例                                                       |
| A6  | CommandError 归一丢弃 code                    | `bun run test -- ipc.test`                            | ✅ 捕获（2 failed）  | 五类 code 锁定 + 既有 code 透传用例                                           |
| A7  | 法线跳过归一化                                | `bun run test -- normals`                             | ✅ 捕获（2 failed）  | normals 单测                                                                  |
| A8  | materials 自定义 id 去掉序列号                | `bun run test -- materials.test`                      | ✅ 捕获（1 failed）  | id 形状正则                                                                   |
| A9  | exportFieldCsv 不触发下载                     | `bun run test -- results.test`                        | ✅ 捕获（2 failed）  | downloadTextFile 调用断言                                                     |
| A10 | fitCameraToBounds 还原旧三轴共用 min/max 算法 | `bun run test -- math.test`                           | ✅ 捕获（2 failed）  | 「不居中网格按逐轴中点取注视点」回归用例——**消融的正是修复前的原始 bug 算法** |
| A11 | quickselect 退化为直接取位不分区              | `bun run test -- stats.test`                          | ✅ 捕获（2 failed）  | 与全量排序对拍用例                                                            |
| A12 | 视口动画背压移除                              | 全量 `bun run test`                                   | ⚠ 未捕获（507 全过） | 无保护（见缺口）                                                              |

**结论：12 个可代码化消融点中 11 个被测试套件当场捕获；A12 为已知口径外
缺口（见下）。**

## 二、GPU/CPU 消融计时（规模扫描）

新增常驻测量入口（`#[ignore]`，不占常规门禁）：

```console
cargo test -p kairos --lib --release gpu_derive_ablation -- --ignored --nocapture
```

本机（M 系列 / Apple GPU / 统一内存，2026-09-12，release profile，确定性数据）：

| 规模（值） | GPU cold（含 device 创建） | GPU warm（OnceLock 缓存） | CPU 参考实现 | CPU/GPU |
| ---------: | -------------------------- | ------------------------- | ------------ | ------- |
|    100 000 | 32.4 ms                    | 2.23 ms                   | 0.39 ms      | 0.2×    |
|  1 000 000 | 同上（进程内仅一次）       | 9.79 ms                   | 3.88 ms      | 0.4×    |
| 10 000 000 | 同上                       | 87.2 ms                   | 39.2 ms      | 0.4×    |
| 30 000 000 | 同上                       | 266.9 ms                  | 113.3 ms     | 0.4×    |

**结论（如实记录，与 ≥5× 预算的关系）：**

- **device 缓存的贡献被消融证实**：冷（每次调用重建 device）与热（OnceLock
  缓存）相差一个量级（32 ms vs 2~9 ms）；若不做缓存，每次派生都要多付一次
  device 创建成本。
- **简单逐元素派生算子（线性映射等）在已测全尺寸段 GPU 慢于 CPU**（0.2~0.4×）：
  这类算子是内存带宽受限，CPU 无需上传/回读往返；f64→f32 转换与 MAP_READ
  回读构成固定开销。≥5× 预算针对的是切片投影与 LOD 生成这类重算子（T19/T20），
  不适用于本类算子——该算子的 GPU 主路径价值在架构一致性（GPU 驻留管线，
  后续重算子与 WebGPU 视口消费免 CPU 往返），不在当前单算子加速。
- **per-op 回读是瓶颈**：多算子串联（归一化→阈值）若合并为一次 dispatch
  可消去中间回读，列为后续优化项。

## 三、消融暴露并已修复的真 bug

**GPU dispatch 超限 panic（高严重度）**：规模扫描在 10M 值处触发 wgpu 校验
panic——`dispatch_workgroups` 每维 workgroup 数 156 250 超过 Metal/Vulkan 的
65 535 硬限。即**结果场 > 4 194 240 值（约 33MB f64，100MB 级结果必超）时
derive_field / derive_difference 直接崩溃**，与「大结果数据链」目标直接冲突。

修复：标量管线与矢量模量均按 `MAX_ELEMENTS_PER_DISPATCH = 65_535 × 64`
切块 dispatch（管线只建一次，逐块回读拼接），并新增超限回归测试
（`MAX_ELEMENTS_PER_DISPATCH + 7` 值与 CPU 参考逐值对拍）。

## 四、缺口处置

| 缺口                                                                              | 处置                                                                                                                              |
| --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| A12 动画背压无测试保护（useViewportPanel 在覆盖率口径外，需渲染器集成才能测时序） | **接受并记录**；归 B3 持久化/懒加载批次随抽纯函数一并收编                                                                         |
| fitToMesh、quickselect 修复点此前无法消融验证                                     | **已加固**——抽为纯函数 `fitCameraToBounds`（render/math.ts）与 `utils/stats::quickselect` 并入测试口径（本报告的 A10/A11 即验证） |
| derive 在 10M+ 值 panic                                                           | **已修复**（超限切块 + 回归测试）                                                                                                 |

## 验证

- 消融矩阵 12 轮全部还原，每轮后 `git diff` 为空；
- 全量 `bun run verify` 通过（前端 507 测试四维 100%、Rust core 255 测试
  行覆盖 100%、clippy -D warnings、knip）。

---

# 第二阶段：全项目消融（2026-09-12）

范围从「R1 修复批次」扩展到**整个项目**：Rust core 领域服务、Tauri 命令层、
前端 stores/utils/render 逐模块选代表性消融点，临时还原/破坏行为 → 跑定向
测试 → 还原（工作树逐轮核对干净）。前置修复（动画节拍器与 storage 抽
util）使上一阶段的 A12 口径外缺口变为可消融验证。

## 消融矩阵

| #   | 模块               | 消融点（破坏方式）                    | 结果                                                                                                                                    | 保护测试                                                       |
| --- | ------------------ | ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| B1  | core/gmsh          | msh 节点索引去掉 1-based→0-based 偏移 | ✅ 捕获                                                                                                                                 | parses_two_tet_mesh                                            |
| B2  | core/repair        | 顶点焊接容差归零（只焊完全重合）      | ⚠ 未捕获 → 加固后 ✅                                                                                                                    | near_duplicate_vertices_weld_within_tolerance（1e-9 噪声顶点） |
| B3  | core/jobs          | promote 忽略核数预算                  | ✅ 捕获                                                                                                                                 | 并发预算用例                                                   |
| B4  | core/dualdomain    | 厚度配对「双向取近」改取远            | ✅ 捕获                                                                                                                                 | pairing 用例                                                   |
| B5  | core/openfoam      | inlet/vent 分带判定翻转               | ⚠ 未捕获 → 加固后 ✅                                                                                                                    | boundary_bands_classify_bottom_inlet_top_vent                  |
| B6  | core/results       | FieldCache FIFO 淘汰改淘汰最新        | ✅ 捕获                                                                                                                                 | cache_evicts_oldest_beyond_capacity                            |
| B7  | core/results       | 阈值掩码 ≥ 改 >                       | ✅ 捕获                                                                                                                                 | threshold 用例                                                 |
| B8  | core/material      | CSV 表头校验跳过                      | ⚠ 未捕获 → 加固后 ✅（旧断言 `contains("表头")` 被行级错误消息误满足；增 21 列错名用例）                                                | csv_import_rejects_bad_header_row_count_and_number             |
| B9  | core/project       | schema 版本门移除                     | ✅ 捕获                                                                                                                                 | 版本拒绝用例                                                   |
| B10 | core/process       | 熔体温度上界移除                      | ✅ 捕获                                                                                                                                 | validate 用例                                                  |
| B11 | core/render_mesh   | face_cells owner 归属错位             | ✅ 捕获                                                                                                                                 | owner 归属用例                                                 |
| B12 | core/geometry      | check_mesh 开放边不计数               | ✅ 捕获                                                                                                                                 | summarize 用例 ×2                                              |
| B13 | cmd/gpu_ops        | dispatch 切块移除（3 倍限值模拟）     | ✅ 捕获（精确复现原始 panic：196605 > 65535）                                                                                           | derive_gpu_handles_over_dispatch_limit_lengths                 |
| B14 | cmd/gpu_ops        | 两场差值长度校验移除                  | ✅ 捕获                                                                                                                                 | rejects_length_mismatch                                        |
| B16 | web/field-binary   | 解码端序小端→大端                     | ✅ 捕获                                                                                                                                 | decode 用例                                                    |
| B17 | web/report         | escapeHtml 的 `<` 转义移除            | ✅ 捕获                                                                                                                                 | XSS 注入用例 ×4                                                |
| B18 | web/shortcuts      | 平台判定翻转（⌘↔Ctrl）                | ✅ 捕获                                                                                                                                 | 匹配用例 ×5                                                    |
| B19 | web/pipeline store | 几何前置校验移除                      | ✅ 捕获                                                                                                                                 | 防御校验用例                                                   |
| B20 | web/project store  | touchActiveStudy 不盖 updatedMs       | ⚠ 未捕获 → 加固后 ✅（旧断言 `toBeGreaterThanOrEqual` 连不盖章都通过；改假计时器严格相等）                                              | stamps updatedMs                                               |
| B21 | web/animation      | 背压移除（上阶段 A12 缺口）           | ✅ 捕获（前置修复后受保护）                                                                                                             | 背压跳拍用例                                                   |
| B22 | web/storage        | 损坏 JSON 不兜底                      | ✅ 捕获                                                                                                                                 | 损坏回退用例                                                   |
| B23 | web/picking        | 射线方向 right 分量丢弃               | ✅ 捕获                                                                                                                                 | 拾取用例 ×2                                                    |
| B24 | web/mesh-asset     | 网格步长漂移                          | ⚠ 未捕获 → 加固后 ✅（旧测试只验证两次运行互相一致；增单位网格精确布局断言。注：2e-16 级漂移经 Float32 窄化不可见，复验用 1e-2 级漂移） | 顶点坐标精确落格用例                                           |

**统计：23 点，加固前 18 捕获（78%），5 缺口全部加固后 23/23 可捕获（100%）。**

## 加固清单（本阶段落盘的测试）

1. core/repair：`near_duplicate_vertices_weld_within_tolerance`——容差焊接语义
   （此前只覆盖完全重合顶点）；
2. core/openfoam：`boundary_bands_classify_bottom_inlet_top_vent`——分带方向；
3. core/material：21 列错名表头用例——表头校验本身（旧断言可被行级消息误满足）；
4. web/project：盖章断言改假计时器严格相等（旧断言过弱）；
5. web/mesh-asset：单位网格精确布局（旧测试只验证运行间一致，不验证布局正确）。

## 经验

- 「测试存在」≠「语义被锁定」：5 个缺口里 3 个（B8/B20/B24）是断言过弱
  （宽松比较 / 只验一致性不验正确性），2 个（B2/B5）是语义完全无覆盖；
- 消融驱动的测试加固能精确暴露弱断言，随 T21 评审常态化（对关键纯函数
  做定期抽样消融）——**已采纳**：写入
  [T21 清单第 5 项](../tasks/T21-milestone-review.md)（2026-09-12）；
- Float32 顶点缓冲使 <1e-7 级的布局漂移不可检测——确定性锁定断言要按
  目标精度设计。

## 验证

- 每轮消融后工作树核对干净（一处例外：复验阶段 git checkout 误还原了
  未提交的加固测试，已重新应用并全量验证——流程教训：消融还原应使用
  文件级暂存而非 checkout）；
- 全量 `bun run verify` 通过：前端 516 测试四维 100%、core 257 测试行覆盖
  100%、clippy -D warnings、knip。
