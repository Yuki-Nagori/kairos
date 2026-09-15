# Kairos 整体代码评审

日期：2026-09-15；基线：`07cab68e6031b40840da255d786cf54eeb336338`。

> 本文保留修复前的审计结论与复现证据。当前修复状态见[修复闭环](review-fixes-0915.md)，不要将下文的“待修复”视为当前状态。

## 结论

现有分层、二进制场通道、测试与覆盖率门禁已经形成基础，但本轮发现 **12 项需要处理的问题**。最优先的是工程数据隔离、求解执行与结果一致性：这些缺陷会导致使用错误几何、丢失编辑、取消无效，或把失败/旧结果当成新结果。

这是代码审计与定向复现报告，不是 T21 里程碑验收通过证明。未修改生产代码；临时回归测试已移出仓库。问题均保持待修复，没有把建议计作完成。

## 验证与边界

- 阅读 ARCHITECTURE 全文，以及 Rust/Web 规范、T21、测试地图、架构状态、性能预算和既往评审。
- 走查重点：工程打开/保存、网格持久化、case 生成、作业调度/取消、VM 日志协议、结果缓存/派生、IPC、CSP/capabilities 与 CI。
- `bun run verify` **通过，退出码 0**：前端 66 文件 / 694 测试；core 511 测试；契约 21 测试；前端行/分支/函数/语句全部 100%，core 行覆盖 100%。完整日志（本机临时证据：`/tmp/kairos-review-20260915/verify.log`）
- `bun run docs:check`：167 篇，断链 0、孤儿 0。日志（本机临时证据：`/tmp/kairos-review-20260915/docs-check.log`）
- 新写的三个前端行为断言 **3/3 失败，准确复现缺陷**，失败不是现有门禁回归。使用真实 store、mock IPC，未启动真实 GUI。测试（本机临时证据：`/tmp/kairos-review-20260915/review-audit.test.ts`） · 输出（本机临时证据：`/tmp/kairos-review-20260915/web-repro.log`）
- 编译独立 Rust 程序直接链接当前 core，复现调度、日志解析、退出码与路径问题。源码（本机临时证据：`/tmp/kairos-review-20260915/core.rs`） · 输出（本机临时证据：`/tmp/kairos-review-20260915/core-output.txt`） · 路径复现（本机临时证据：`/tmp/kairos-review-20260915/paths.rs`） · 路径输出（本机临时证据：`/tmp/kairos-review-20260915/paths-output.txt`）
- 没有运行真实 VM 求解、Windows/Linux 桌面、GPU/FPS/冷启动验收，也未跑完整物理精度/收敛矩阵；本轮不能证明这些维度通过。没有查询最新漏洞数据库，不能给出“依赖无漏洞”的结论。
- 未执行 T21 的生产代码消融：这里的新增失败测试是回归复现，与“破坏实现后旧测试变红”的消融不同。

复现前端测试时，把保留的 `review-audit.test.ts` 放到仓库 `tests/web/review-audit.temp.test.ts`，运行 `bunx vitest run tests/web/review-audit.temp.test.ts`；用完删除临时副本。它有意断言修复后的正确行为，当前版本预期三项失败。

## 按优先级排列的发现

### F1 · P1 · 切换工程没有恢复数据，也没有隔离上一工程状态

位置：[project.ts:105](../../src-web/stores/project.ts#L105)、[geometry.ts:223](../../src-web/stores/geometry.ts#L223)、[pipeline.ts:24](../../src-web/stores/pipeline.ts#L24)。

`openProjectAtPath` 只替换工程 JSON、路径和活跃方案。`restoreWorkspaceContent` 在生产源码只有定义，没有调用；菜单和快捷键最终都只调用 `openProject()`。几何/网格/结果 store 也没有在工程切换时清空。

后果：重启后打开工程不自动恢复几何网格；同一会话 A→B 时，A 的几何继续留下，而流水线取 `geometries[0]`，可能用 A 的网格配 B 的材料工艺进行求解。

复现：打开 B 后 `restoreCalls=0`、`oldPresent=true`。应在统一的工程切换事务里保存旧工程、清理旧会话、恢复新工程，并让求解使用明确的工程/方案/几何绑定。

### F2 · P1 · Linux/Windows 的取消没有可用进程句柄

位置：[jobs.rs:489](../../src-tauri/src/commands/jobs.rs#L489)、[jobs.rs:713](../../src-tauri/src/commands/jobs.rs#L713)。

`cancel_job` 从 `children` 取 Child 进行终止，但全文件没有任何 `children.insert`。实际 Child 直接移交给 `run_job_body` 的局部变量。因此原生路径只会把状态改成 Cancelled，运行进程继续；Windows 也没有 macOS 专用的远端终止分支。

应由运行控制对象持有 PID/进程组与取消令牌，终止成功后再释放预算；WSL 要终止 guest 进程。测试必须观察实际进程退出，而不只检查状态枚举。

相关风险：macOS 在准备/复制阶段取消时，SID 尚未建立，kill 可能空操作，准备线程又未检查取消状态而继续启动。远端日志循环也没有总超时或取消退出条件（[jobs.rs:565](../../src-tauri/src/commands/jobs.rs#L565)），永久断联可能留下常驻轮询。上述来自代码走查，本轮未在 VM 实测。

### F3 · P1 · 求解失败的退出码被 reconstructPar 覆盖

位置：[moldingfoam.rs:842](../../src-crates/kairos-core/src/services/moldingfoam.rs#L842)、[jobs.rs:143](../../src-crates/kairos-core/src/services/jobs.rs#L143)。

命令为 `decomposePar ... && mpirun ... ; reconstructPar`。后续重建成功会把整段退出码变为 0；只有捕获到特定 FOAM 日志标记才会补判失败，无法兜住 SIGSEGV、MPI 异常等没有该标记的退出。

实测：用 shell 函数令 mpirun 返回 139、reconstructPar 返回 0，得到 `overall=0`、`job_failure=None`。应保存分解/求解的真实退出码，重建后仍返回原失败；日志用于解释原因，不能取代退出状态。

### F4 · P1 · 增量日志把协议分隔换行计入文件偏移

位置：[vm.rs:505](../../src-crates/kairos-core/src/services/vm.rs#L505)、[vm.rs:519](../../src-crates/kairos-core/src/services/vm.rs#L519)。

读取命令在日志后额外输出 `\n##KAIROS-STATUS##`，解析只切哨兵文字，把额外换行也算入 `chunk.len()`。每轮偏移多加 1，空轮询同样前移，下一轮从真实日志更靠后的位置读，丢失新日志开头字符。

实测：11 字节日志得到 offset=12；没有新增日志的下一轮得到 13。可能损坏 `Time =` 或错误标记，进一步影响进度与失败识别。

修复应明确协议帧边界，只累计原始日志字节；补空轮询、分段行、多字节文本与错误标记跨块的测试。

### F5 · P1 · 防抖自动保存跨工程切换会丢失旧工程编辑

位置：[project.ts:36](../../src-web/stores/project.ts#L36)、[project.ts:256](../../src-web/stores/project.ts#L256)。

800ms 定时器到期才读取 `this.project` 和 `this.projectPath`。A 编辑后立刻打开 B，待执行的保存就写 B，A 的编辑没有落盘。

复现：派发 A 的自动保存、打开 B、推进计时器，保存调用仅包含 `/work/B/B.kairos`。应按工程绑定待保存快照/修订，并在切换前 flush 旧工程保存。新建方案 `addStudy` 也未调用 `scheduleAutoSave`，需要一并核对其持久化语义。

### F6 · P1 · 同一方案的多次求解共用可变 case 目录

位置：[pipeline.ts:51](../../src-web/stores/pipeline.ts#L51)、[project.rs:158](../../src-tauri/src/commands/project.rs#L158)、[moldingfoam.rs:698](../../src-crates/kairos-core/src/services/moldingfoam.rs#L698)。

case 路径只由方案决定，没有 run/job ID。生成器直接改写该目录；调度器不拒绝重复 case_dir。第一次提交很快返回后，用户能再次提交，第二次在入队之前已经改写第一次使用的输入。不同运行的日志/SID/结果路径也重合。

即使串行重跑，旧的时间目录也未由生成器清理，短时运行可能与旧长时结果混在一起。应采用每次运行独立目录与输入快照；若先做最小修复，必须在生成前建立目录占用约束，不能只在 submit 时检查。

### F7 · P1 · 结果缓存没有文件更新失效机制

位置：[results.rs:68](../../src-tauri/src/commands/results.rs#L68)、[results.rs:114](../../src-tauri/src/commands/results.rs#L114)。

缓存键仅含目录、时间步、场名。缓存命中后直接返回，不检查文件修改，也没有在目录重扫或求解回传后失效。重跑同一 case 或先读到未写完的场，再次加载仍可能显示旧值/不完整值，直到淘汰或重启。

应绑定不可变 run ID 与文件修订；对于在写结果，用 mtime/大小或显式完成标记识别更新。补“加载→覆盖文件→重扫→再次加载”的集成测试。

### F8 · P1 · 探针曲线读取使前后端主场不一致

位置：[results.ts:184](../../src-web/stores/results.ts#L184)、[results.ts:201](../../src-web/stores/results.ts#L201)、[results.rs:127](../../src-tauri/src/commands/results.rs#L127)。

遍历时间步调用 `loadResultField` 默认写入后端 primary；所谓恢复只给前端 `loadedField` 赋值。遍历结束后后端留在最后时间步，界面仍显示原时间步。后续派生从后端 primary 读取，会计算另一时刻的数据。

复现：原时间步 1、扫描 1/2 后得到 `frontend=1, backend=2`。建议在 core 提供不改变会话槽位的探针批量采样，IPC 只返回探针曲线。该实现还会把每个时间步整个场传到前端，只取少量节点，存在显著无效传输；越界值被 `?? 0` 替代也会伪造零值，应返回缺失状态。

### F9 · P1 · 子进程 stderr 建管道后无人读取

位置：[jobs.rs:358](../../src-tauri/src/commands/jobs.rs#L358)、[jobs.rs:376](../../src-tauri/src/commands/jobs.rs#L376)、[jobs.rs:497](../../src-tauri/src/commands/jobs.rs#L497)。

原生/WSL 启动设置 stdout/stderr 为 piped，运行主体只消费 stdout，之后直接 wait。stderr 的错误既不会进入日志判定，输出足够多时还会填满管道，子进程阻塞写 stderr，父线程等待 stdout 结束，形成互等。

应并发排空两条流，或在受控脚本中合并输出；不可在 stdout 结束后才补读 stderr。用持续写入大于管道容量的假子进程验证完整退出和错误可见性。本轮确认代码路径，未在真实求解器上制造阻塞。

### F10 · P1 · 工程中的方案 ID 可越过输出目录边界

位置：[project.rs:109](../../src-crates/kairos-core/src/services/project.rs#L109)、[workspace.rs:92](../../src-crates/kairos-core/src/services/workspace.rs#L92)、[workspace.rs:97](../../src-crates/kairos-core/src/services/workspace.rs#L97)。

工程读取只按 schema 分支，方案 ID 没有安全路径段校验；`mesh_dir`/`cases_dir` 直接 join(study_id)。std::path 解决平台语义，但不会自动阻止绝对路径覆盖前缀或 `..` 越界。

无写盘复现：工程接受 ID `/tmp/kairos-review-escaped`，期望根 `/tmp/intended-root` 下的 case 路径实际变成 `/tmp/kairos-review-escaped`。加载被修改的工程后再执行网格保存/case 生成，可能在工作区外创建或覆盖固定名字的文件。不是打开文件即执行代码。

应在 core 对 ID 强制单一路径段/合法字符/唯一性约束，并对最终输出路径建立工作区边界校验。读取也要执行结构校验，不能仅在序列化时做。

### F11 · P2 · 超过 8 核的请求会永久排队

位置：[jobs.rs:61](../../src-tauri/src/commands/jobs.rs#L61)、[jobs.rs:44](../../src-crates/kairos-core/src/services/jobs.rs#L44)、[jobs.rs:78](../../src-crates/kairos-core/src/services/jobs.rs#L78)。

调度器预算硬编码 8 核；提交只拒绝 0，面板没有上界，case 生成另允许到 64。任何 9–64 核请求都无法满足调度条件，又没有错误提示。

实测：9 核提交成功，提升列表空、状态 Queued。应统一 UI、case、调度器和实际 VM 的核数口径；拒绝永远无法执行的请求或允许独占大作业，不能静默排队。

### F12 · P2 · 网格提取与变形仍在同步命令中持锁重算

位置：[geometry.rs:516](../../src-tauri/src/commands/geometry.rs#L516)、[results.rs:174](../../src-tauri/src/commands/results.rs#L174)。

`get_render_mesh` 同步命令持 GeometryStore 锁提取体网格边界；`deform_render_mesh` 同步命令复制位移、重建渲染网格并逐顶点计算。数据量随网格增长，和 ARCHITECTURE 的“主线程不做重计算、锁不跨耗时计算”不符。

应使用网格快照/共享不可变数据，把计算放到 spawn_blocking，在网格修订变化时缓存边界提取；大网格返回采用二进制 Response。这里是代码层面的阻塞风险，未测量真实 GUI 卡顿时长。

## 代码质量与进一步优化

1. **优先补接缝测试。** 现有 100% 覆盖率能证明执行到了行，不能证明多个 store、IPC 槽位、shell 命令和真实子进程组成的行为正确。新增回归应围绕上述可观察后果，尤其是“取消后进程仍存在”和“界面显示 A，实际计算 B”。
2. **将运行生命周期下沉为可测试服务。** jobs 适配层仍负责准备/取消/启动/轮询/回传/租约，存在对 vm/downloads/geometry 等命令模块的横向依赖。沿 ARCHITECTURE 的 runner 接口抽取生命周期，Tauri 仅桥接 Channel、State 和调用装配。
3. **大场后处理继续减少跨 IPC 往返。** 探针批量采样、张量模量/值域、CSV 流式导出适合放 core。`useResultsPanel.vectorStats` 对全部分量重新 Math.hypot，`exportFieldCsv` 创建逐值嵌套数组；百万量级不宜留在 UI 热路径。
4. **缓存按来源与修订组织。** 工程/方案/几何修订/run/时间步/场关联应明确，不能仅依赖全局“最近加载”槽。先修正确性，再用 Arc 等减少加载时多次 Vec clone，避免把缓存和主场每次都拷贝一份。
5. **DTO 归属收口。** `EnvironmentCheck`、`CaseOutcome` 仍在 Tauri solver.rs 定义，偏离 models 为契约事实源的规则；迁移时同步 TS 和契约测试。
6. **文档健康检查增加语义一致性。** docs:check 通过只说明文件链接存在。architecture-status 顶部仍写截至 09-12，正文却含后续任务；T92/T93 的状态需要与已存在的 doe/optimize 实现区分“纯算法已实现”和“端到端可用”。

## 性能抽测

命令：`bun run bench:web`；`cargo bench -p kairos-core --bench ipc_dto -- --quick`，两者退出码 0。本机 release 微基准，quick 模式采样较少，不能当成三端发布验收或稳定回归统计。

| 核心测项                    | 当前中心估计 | 文档历史值 | 解读                          |
| --------------------------- | -----------: | ---------: | ----------------------------- |
| 浇口分析 / 625k 四面体      |    274.67 ms |     296 ms | 未见粗粒度退化，低于 3 秒预算 |
| 充填预览 / 625k 四面体      |    165.82 ms |     168 ms | 与历史接近                    |
| ASCII 解析 / 1e7 值、110 MB |    398.38 ms |     426 ms | 仅解析耗时，不含 IPC/首帧     |
| f32 编码 / 1e7 值、40 MB    |     30.10 ms |    31.4 ms | 与历史接近                    |
| f32 解码 / 1e7 值           |     10.48 ms |    12.4 ms | Rust 端基准                   |
| 矢量编码 / 1e6 单元         |      8.78 ms |     9.1 ms | 与历史接近                    |
| 矢量解码 / 1e6 单元         |      4.38 ms |     4.0 ms | quick 单次比较不足以判回归    |

前端：10 watcher 同步写 5.49µs；5 computed 相关写 0.694µs；属性读 33.9ns。它们衡量状态基础操作，不能替代视口 FPS 或大数组 UI 响应测试。core 日志（本机临时证据：`/tmp/kairos-review-20260915/bench-core.log`） · web 日志（本机临时证据：`/tmp/kairos-review-20260915/bench-web.log`）

## CAE 产品能力缺口

结合本地架构状态、任务索引与 moldingFoam README，当前必须继续区分“已有算法/字段/界面”与“物理求解闭环”：

- STEP/IGES 当前是镶嵌/多边形子集，不能按通用精确 B-rep CAD 导入承诺。
- 双域/中面已能生成，并不代表求解器已消费它们；T82 仍记录消费缺口。
- 浇口边界落地不等于流道实体压降与多型腔流量分配已完整求解；T83 仍需闭环。
- 冷却水路边界与位移场可视化不等于完整模具瞬态冷却、翘曲求解已验证。
- 内置材料是典型参考值；应按数据来源和标定状态呈现，避免与已验证生产牌号混同。
- DOE/寻优纯算法已有实现，本轮不把它们说成“没有代码”；缺的是可靠独立运行、指标来源、失败处理与优化结果的端到端证据。
- 求解器库身份/重复加载风险已有 T97 跟踪。本轮不改动共享 VM，也不把历史源码重建库的性能冒充当前发布制品性能。

## 建议实施顺序

1. **结果可信性与数据安全**：F1/F3/F4/F5/F6/F7/F8/F10。
2. **执行可靠性**：F2/F9/F11，补真实假进程与 Windows/WSL、macOS 准备阶段取消测试。
3. **性能与架构**：F12、探针批量采样、不可变运行快照、会话来源/修订、适配层收缩。
4. 完成后跑全量 verify，再做真实 VM L2、GUI 工程切换/重启恢复与多次求解回归；达到这些条件后再作里程碑完成判断。
