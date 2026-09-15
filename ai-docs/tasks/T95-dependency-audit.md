# T95 · 依赖取向审查：用现成库 vs 自造轮子

- 阶段：E4（工程质量）
- 依赖：无；涉及换库的条目须先完成许可、性能与包体对照。
- 优先级：P2
- 状态：**进行中**。P1-a 已实现有摘要时的校验；P1-b、P2-a、P2-b、P2-c 已完成；P3-b 已完成主方向与原清单收编，但仍有漏项；P3-c 已完成网格编码替换；P3-d 保持系统工具。剩余包括 P3-a 图表评估、gl-matrix 取舍、色标统一、`repair` 向量消重与证据补齐，不能表述为「仅待图表视觉对照」。
- 来源：T60 讨论 PPTX 导出时提出的依赖取向问题。
- 最近核对：2026-09-15，基于仓库 `b55e617` 的源码、依赖清单、锁文件、测试、基准与时间线。

## 目标与边界

逐块判断「采用成熟库 / 继续自写」，给出可追溯的现状、理由和后续动作。
本次更新核对实现与任务状态，不执行未完成的换库、视觉验收或重新测量性能。

判断标准按优先级排列：

1. **规格归属**：CSV、压缩、线性代数等通用问题可用库；case 契约、网格口径、浇口语义、IPC 布局与错误契约由项目掌握。采用底层库不等于外包产品规格。
2. **许可**：只接受 permissive 的进程内依赖；GPL/AGPL 仅可经子进程与文件交换隔离。GPU 计算走 wgpu，不引单厂商 SDK。
3. **可验证性**：core 行覆盖 100%，前端逻辑层四维覆盖 100%；包装层仍需测试，覆盖门槛不等于跨平台或真实界面验收。
4. **性能、包体与离线**：换实现前先测基线，旧实现保留在基准中同台对比；包体使用同一构建口径。npm 解包大小不能代替最终 gzip 增量，Rust 二进制不能与前端资源大小混比。
5. **维护成本**：比较库适配、升级与自写维护的总成本；维护活跃度与候选版本在真正引入前重新核实。

准入与代码归属遵循 [ARCHITECTURE.md](../ARCHITECTURE.md)。本文件是依赖取向审查，
**不是全量传递依赖的许可证或漏洞审计报告**。

## 已采用依赖：锁定版本与许可核对

版本来自根 `Cargo.lock` / `bun.lock`；许可来自本机对应版本的 Cargo registry 包元数据与
`node_modules/@vueuse/core/package.json`，不把首轮候选库信息当成当前已安装事实。

| 依赖           | 锁定版本                | 包元数据声明的许可        | 当前用途                                      |
| -------------- | ----------------------- | ------------------------- | --------------------------------------------- |
| `csv`          | 1.4.0                   | Unlicense / MIT           | core 材料 CSV 导入                            |
| `sha2`         | 0.10.9                  | MIT OR Apache-2.0         | core 文件 SHA-256                             |
| `nalgebra`     | 0.35.0                  | Apache-2.0                | core 主方向特征分解；关闭默认特性，仅开 `std` |
| `bytemuck`     | 1.25.2                  | Zlib OR Apache-2.0 OR MIT | core 网格编码及 Tauri GPU 数据转换            |
| `@vueuse/core` | 14.4.0                  | MIT                       | 自动保存防抖、方案任务菜单关闭                |
| `ppt-rs`       | 0.2.26                  | Apache-2.0                | T60 已落地的 Rust PPTX 生成                   |
| `base64`       | 0.22.1（core 直接依赖） | MIT OR Apache-2.0         | 报告图片 dataURL 解码                         |
| `regex`        | 1.13.1                  | MIT OR Apache-2.0         | core `utils/regex.rs` 通用正则入口            |

`package.json` 与 `bun.lock` 均未引入 `uplot` 或 `gl-matrix`。
PPTX 已由 [T60](T60-import-log-and-pptx.md) 实现，不再重复列为选库待办。
未采用候选库的首轮版本、下载量和解包体积不作为准入证据；实施时应记录实际选定版本及许可。

## Rust 决策表（按当前代码）

以下 `services/`、`utils/` 路径默认相对 `src-crates/kairos-core/src/`，桌面适配层另行标明。

| 审查项 / 源码证据                                                     | 当前实现                                                                                                                                  | 结论与边界                                                                                                                                                   |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 材料 CSV：`services/material.rs::parse_custom_csv`                    | 已使用 `csv::ReaderBuilder`，保留表头、列数、数值与属性表校验                                                                             | **P1-b 完成**。BOM、CRLF、引号内逗号、双引号转义、未闭合引号与损坏表头有用例；不能再写「手工 split、BOM 会失败」                                             |
| 主方向：`services/results.rs::principal_axis`                         | 已用 nalgebra，基准保留旧 Jacobi 实现                                                                                                     | **P3-b 主方向完成**。几何容差与网格算法仍自写；`parry3d` 暂不引                                                                                              |
| 向量原语：`services/vec3.rs`                                          | meshing、thickness、dualdomain、iges、gate_location、fill_preview 等消费共享函数；`repair.rs::triangles_intersect` 内仍定义 sub/cross/dot | **原收编清单已处理，仍有漏项**。见下表，不能声称全仓原语只有一份                                                                                             |
| STL：`services/geometry.rs`                                           | 自写解析、写出与错误检查                                                                                                                  | **保持**。`stl_io` 仅为备选；扩大格式支持或语义维护成本明显增加时再评估                                                                                      |
| STEP / IGES：`services/step.rs`、`services/iges.rs`                   | STEP 支持 AP242 镶嵌与面化 B-rep 子集；IGES 支持 106/63 多边形与 124 变换，未实现完整精确 B-rep / NURBS                                   | **当前子集保持自写**。完整 CAD 导入应单独评估格式库与几何内核，不能把子集解析记成完整格式支持；IGES 还不解析 G 节自定义分隔符                                |
| 下载摘要：`services/digest.rs`、`src-tauri/src/commands/downloads.rs` | 下载后总会计算 SHA-256；解析 `/releases/latest` 时可得到 GitHub asset digest，有有效摘要则比对，不一致时尝试删归档并报错                  | **P1-a 条件校验已实现**。静态直链、缺失摘要或无效摘要只记录本地值；不是所有下载都通过官方摘要验证                                                            |
| 下载解压：`src-tauri/src/commands/downloads.rs::extract_archive`      | Windows ZIP 用 tar；macOS / Linux ZIP 用 unzip；tar.xz 用 tar -xJf，其他支持的 gzip 归档用 tar -xzf                                       | **P3-d 暂不换库**。候选 tar/flate2 不能单独覆盖 xz；xz2/lzma-rs 需另行对照。仓库没有解压候选的同台基准，不再断言纯 Rust 必然更慢或系统工具在所有机器必然可用 |
| case 传输 / VM 部署：`services/vm.rs`、桌面 jobs/vm 命令              | 宿主与客体的归档、解压仍依赖系统 tar 和 VM 命令                                                                                           | **单独保留外部工具边界**。替换宿主下载解压并不能消除客体 tar 依赖；工具缺失、归档类型变化或部署故障时重审                                                    |
| 网格二进制：`services/mesh_store.rs`                                  | 节点和索引用 bytemuck 批量编码；解码保留原实现                                                                                            | **P3-c 网格编码完成**。布局保持小端，批量转换依赖目标为小端的前提                                                                                            |
| KF1：`services/results.rs::field_binary`                              | 字段载荷仍逐值 to_le_bytes 编码，未迁移为 bytemuck                                                                                        | **保留当前实现**，有性能需求再测；不能把网格编码迁移扩写为全部二进制编解码已收编。bincode/postcard 不直接替换现有 IPC 协议                                   |
| 加权 LRU：`services/results.rs`                                       | 自写值数预算与命中统计                                                                                                                    | **保持**。通用 lru 容量语义不同，当前无替换收益证据                                                                                                          |
| 时间 / ID：`utils/time.rs`、`services/project.rs`                     | now_ms 已共享，jobs/downloads/CLI 均复用；Rust new_id 仍为时间戳加原子序号                                                                | **已收敛，不引 uuid/chrono**。本地标识不等于随机令牌或跨进程全局唯一保证                                                                                     |
| 路径：`services/paths.rs`                                             | std::path 操作配合存储归一、WSL 映射与文件名清洗                                                                                          | **保持**。库处理原生路径结构，服务承载跨机器存储与产品约定                                                                                                   |
| 终端清洗 / 命令探测：`services/vm.rs`、`services/dependencies.rs`     | 自写平台适配与文本规则                                                                                                                    | **保持**；strip-ansi-escapes / which 仅作备选，需按实际平台语义对照，不能笼统断言候选必定丢失语义                                                            |
| 原子写：`utils/fs.rs::write_atomic`                                   | project 包装、下载清单、部署记录已复用同一实现                                                                                            | **已统一，不引 tempfile**。旧文「两处重复，第三处再统一」已过时                                                                                              |

### P1-a 的剩余边界

- `ManifestEntry.sha256` 仅在下载清单落盘，`SavedDownload` 与前端 DTO 未暴露摘要；依赖面板展示仍待做。
- [上游问题清单 §4](../reviews/upstream-questions.md) 已记录 GitHub digest 可用并关闭原「要求 SHA256SUMS」事项，不再列为上游阻塞。
- `parse_sha256_digest` 与 `sha256_file` 在 core 有单测；真正的下载比对、删除及解压顺序位于 Tauri 层，不能用 core 100% 覆盖率代替该路径的集成验证。
- 源码中下载摘要计算前仍有「只记录、不比对」的旧注释，后续修改该路径时应同步纠正；本轮仅更新任务文档。

## 前端决策表（按当前代码）

路径相对 `src-web/`。

| 审查项 / 源码证据                                                                                              | 当前实现                                                                                                                                | 结论与剩余动作                                                                                                                                                              |
| -------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| formatter：`utils/format.ts` 及各面板                                                                          | 展示调用已迁移到 fixed/significant 等；chart.ts 仍有两处 toFixed，bench 脚本保留自身格式化                                              | **P2-b 展示迁移完成**。画布刻度待图表批次处理；表单初值的 toPrecision 不属于展示迁移。旧「45 处、20 文件、无测试」是迁移前统计                                              |
| XY 曲线：`utils/chart.ts`、`views/xy-chart/useXyChartPanel.ts`                                                 | 仍为自绘 Canvas；保留 min-max 保形抽样、探针叠加、主题重绘、报告快照与 CSV 导出；新增 `bench/chart.ts` 记录 100 万点值域扫描 / 抽样基线 | **P3-a 结论：继续自绘**。uPlot 未引入：候选库不能直接覆盖上述产品语义，当前没有实测交互瓶颈；引入会增加适配与包体，暂无收益证据。基准可用 `bun src-web/bench/chart.ts` 复核 |
| CSV 导出：`utils/chart.ts::toCsv`                                                                              | 字段转义、CRLF、UTF-8 BOM 已实现并有测试                                                                                                | **P2-c 完成，不引 papaparse**                                                                                                                                               |
| VueUse：`stores/project.ts`、`views/study-tasks/useStudyTasks.ts`                                              | useDebounceFn 用于自动保存，onClickOutside/onKeyStroke 用于任务菜单                                                                     | **P2-a 完成逐处判定**。网格估算防抖、错误消失定时器、标题栏菜单和 storage 索引协议保留自定义实现，不是全部通用设施都已换库                                                  |
| 低层数学：`render/math.ts`、`render/webgpu/volume.ts`                                                          | 自写约 10 个 mat4/vec3 原语，统一列主序 `Float32Array`；WebGL/WebGPU renderer、拾取、剖切共用，已有矩阵与向量对拍测试                   | **结论：继续自写**。调用面小且语义固定，当前无测得数学热点；gl-matrix 会引入数组类型和约定适配成本，暂无性能或维护收益证据                                                  |
| 色标：`render/palette.ts`、`render/webgpu/shaders.ts`、`shaders_volume.ts`、`views/viewport/ViewportPanel.vue` | 表面、体积、WebGL 与图例共用蓝橙端点；WGSL 通过模板生成同一组常量                                                                       | **基础统一完成**。继续采用自建常量方案、不引 d3-scale-chromatic；仍需桌面截图回归确认视觉观感                                                                               |
| 快捷键：`utils/shortcuts.ts`                                                                                   | 自写展示与匹配                                                                                                                          | **保持**。需与原生菜单展示口径一致，tinykeys 仅为候选                                                                                                                       |
| 菜单定位：`views/study-tasks/useStudyTasks.ts`                                                                 | 自写视口边缘夹取；关闭事件已交 VueUse                                                                                                   | **定位保持自写**，Floating UI 仅为备选；关闭与定位是两项独立决策                                                                                                            |
| ID：`utils/id.ts`                                                                                              | newId 优先 crypto.randomUUID，无可用 API 时用 Date.now 加模块内序号；方案、流道、水路、自定义材料调用共享入口                           | **P2-c 完成**。回退分支不是 CSPRNG；前端时间戳更新用 Date.now 属正常用途，不是遗漏的 ID 生成                                                                                |
| JSON / storage：`utils/storage.ts` 等                                                                          | 使用平台 JSON API，storage 另有产品索引协议                                                                                             | **保持**，不因调用 JSON.parse/stringify 就引通用对象库                                                                                                                      |

编辑器、虚拟滚动、剪贴板与拖拽属于原审查面的候选方向，首轮未提供逐项实施结论；
当前不据此生成换库任务。出现明确需求与性能问题时，先列实际消费点再评估。

视口后端、剖切、体积光线步进、`utils/field-binary.ts`、任务阻断规则、报告版式、
拾取的单元/节点语义继续由项目维护。通用底层库是否适用应与这些产品语义分开判断。

## 原统一收编清单：核对结果

| 原条目                                                 | 当前证据                                                      | 状态                                                 |
| ------------------------------------------------------ | ------------------------------------------------------------- | ---------------------------------------------------- |
| meshing 的 cross3/dot3/sub                             | 已调用 services::vec3，无原本地函数                           | 已完成                                               |
| thickness::unit_normal                                 | 薄包装调用 vec3::unit_or_none，阈值由调用方传入               | 已完成                                               |
| dualdomain / midplane 各有 ray_triangle                | ray_triangle 在 dualdomain；midplane 复用 TriangleGrid 的求交 | **作废：重复项不存在**                               |
| dualdomain / midplane 各有 nearest_node                | midplane 直接导入 dualdomain::nearest_node                    | **作废：重复项不存在**                               |
| volume 的 normalize/cross/sub                          | 已导入 render/math                                            | 已完成                                               |
| 本次发现：repair::triangles_intersect 内 sub/cross/dot | 三个局部函数仍重复共享向量运算                                | 待消重；三角形相交算法本身与射线相交不同，不合并算法 |

共享原语消融的**历史记录**为 cross、dot、distance_sq、normalized、unit_or_none 共 5/5 红，
见时间线 2026-09-14 23:25；本轮未重新执行消融。volume 前端消重当时只核对零向量等价语义，
无直接消融证据，不能因为 Rust 消融通过就声称前端也已覆盖。

## 性能与包体：历史结果及证据缺口

以下数字保留为时间线记录，**不是当前 HEAD 的复测结果**。不同提交的数据不合并为本轮全量结论。

| 项目               | 历史口径与结果                                                                                                                                  | 仓库可复核证据 / 缺口                                                                          |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| KF1 / 网格编码基线 | 10 万字段值、5 万节点网格：字段编码 608 µs、解码 227 µs，网格编码 2.22 ms、解码 536 µs                                                          | `cargo bench -p kairos-core --bench kf1_codec`；当前基准调用现实现，**没有保留旧网格编码函数** |
| bytemuck 网格编码  | 2223 → 163 µs，约 13.6 倍；网格解码未改                                                                                                         | 时间线 2026-09-14 23:55；应补旧实现同台基准，历史数字不能代替该要求                            |
| 主方向             | 512 个张量：nalgebra 204 µs vs Jacobi 259 µs，约快 20%                                                                                          | `cargo bench -p kairos-core --bench principal_axis` 已保留 jacobi_reference                    |
| Rust 三库包体      | csv + sha2 + nalgebra：app 内二进制 7,445,536 → 7,478,512 B，+32,976 B（+0.44%）                                                                | 原审查记录；不包含后来所有改动，不能用它证明 bytemuck / PPTX 等的独立增量                      |
| VueUse 包体        | 首批 build 532.04 → 533.03 kB，gzip 169.52 → 170.03 kB                                                                                          | 原审查记录；不能外推到后来菜单 hooks 的新增调用                                                |
| XY 图表候选库      | 当前未引入 uPlot，无法提供同口径 gzip / 包体增量；Canvas 基准（Apple Silicon，本地 Bun）：1M 值域扫描 951,406 ns/op，1M→2K 抽样 1,316,075 ns/op | 不以假设数字替代对照；只有出现实测瓶颈并选定候选版本后才做同构构建对照                         |

原文「本轮所有引库在性能和包体均无冲突」证据不足：CSV、SHA-256、VueUse 未在此附同台性能基准，
部分包体记录也缺精确提交与构建环境。后续补齐时记录基线/新实现提交、命令、平台、构建配置与结果；
不把现有历史数字包装成完整准入验收。

## 剩余执行清单

已实现条目不重复开工；以下按独立批次推进，每批先核实消费点，完成后同步时间线和任务索引。

1. **P3-b 补漏**：已完成 repair 局部向量原语改用 vec3，保留相交容差与算法；repair / 几何相关用例通过，消融记录待补。
2. **色标统一（已完成基础收编）**：表面、体积、图例及 WebGL 共用 `render/palette.ts` 的蓝橙端点；仍需桌面截图回归确认视觉观感。
3. **P3-a 图表评估（已完成取舍）**：保留当前 Canvas 自绘；`src-web/bench/chart.ts` 提供大序列数据处理基线。uPlot 需要重新适配主题、探针、截图导出与报告快照，当前没有实测瓶颈或包体收益证据，暂不引入；若后续基准或真实交互暴露瓶颈，再按同口径引入候选并补 gzip 对照。
4. **前端数学取舍（已完成）**：继续自写 `render/math.ts`。现有实现规模小、列主序约定直接对应 WebGL/WebGPU，调用集中且已有对拍测试；暂不引入 gl-matrix。若未来出现矩阵计算热点，再保留当前实现做基线并对拍精度、约定与包体。
5. **P1-a 后续**：补摘要与验证状态的展示方案；区分「本地记录」与「官方比对通过」，同步 DTO / fixtures / 契约测试，并补下载比对流程的验证证据。缺失摘要是否拒绝下载需单独定策略，当前行为是允许并记录。
6. **证据补齐**：优先补网格旧编码同台基准；为已引入依赖补可复现的性能与包体记录，或明确记录未满足项与处置结论。

解压与完整 CAD 格式支持按上面的重审条件触发，不因为评估项存在就立即换库。
涉及实现的批次必须通过仓库根 `bun run verify`；文档状态核对另运行 `bun run docs:check` 与格式检查。

## 本次更新范围

修正过时现状、失效行号、已完成项误列待办、遗漏的前端数学/色标/repair 项、P1-a 校验范围与旧上游阻塞，
并将许可和性能结论限定在实际证据范围。未新增依赖，未修改运行时行为，未把待验证项标记为完成。
