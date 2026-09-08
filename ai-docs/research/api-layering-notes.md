# T25 · 第三方工作流 API 分层概念研究（结论文档）

- 日期：2026-09-08
- 研究对象：Autodesk `moldflow-api` Python 包（Apache-2.0，PyPI `pip install moldflow`，官方 GitHub 仓库 autodesk.github.io/moldflow-api）——Moldflow Synergy 桌面软件的官方 Python 封装
- 合规口径：**仅借鉴其公开文档描述的 API 分层概念**；不引入依赖（绑定 Windows + Synergy 2026 商业安装），不复制代码或文档内容
- 产出：五大模块概念对照表 + 改进建议清单（采纳 / 缓办）

## 一、moldflow-api 的分层与风格梳理（概念级）

1. **单门面 + 访问器**：唯一的入口对象 `Synergy()`，全部能力经由 `synergy.<manager>` 访问器按需获取——`project` / `study_doc` / `mesh_generator` / `mesh_editor` / `diagnosis_manager` / `material_finder` / `plot_manager` / `viewer` / `boundary_conditions` / `data_transform` / `unit_conversion` 等约 20 个 manager。没有全局函数，能力按职责归入具名 manager。
2. **枚举集中**：所有领域常量（成型工艺类型、网格类型、材料库域、绘图类型、视角、单位制……）集中在 `moldflow.common` 一个命名空间，脚本侧只需记住一个 import 点。
3. **对象生命周期**：门面连接到一个运行中的桌面实例；manager 是无状态的按取视图；数据容器（整型/浮点/字符串/向量数组）通过 `create_xxx_array()` 工厂创建，`from_list` / `to_list` 负责 Python 原生数据与内部表示的转换。
4. **错误处理**：混合风格——布尔返回（`generate()` / `save()`）与异常（无活动绘图时抛错）并存，没有统一的结构化错误契约。
5. **遍历协议**：集合类资源（材料、绘图、文件夹/图层）统一 `get_first` / `get_next` 游标模式。
6. **典型工作流**：新建项目 → 建研究（工艺/网格类型）→ 导入 CAD → 网格生成 → 自动修复 → 诊断摘要 → 触发分析 → 绘图读取/派生 → 视觉导出 → 报告组装（配合 python-pptx）。

## 二、五大模块概念对照（moldflow-api ↔ Kairos services 层）

| 模块              | moldflow-api 概念                                                                                                                               | Kairos 现状                                                                                                                         | 差距 / 借鉴点                                                                                                             |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| study             | `project` + `study_doc`：研究生命周期、工艺类型与网格类型挂在研究上                                                                             | `services::project`（工程持久化、原子写、schema 迁移、recents）+ `Study` 模型（材料/工艺归属研究、流道水路内嵌）                    | Kairos 的类型化模型更严格；**缺批处理入口**（多研究循环创建/求解的编排层）                                                |
| mesh              | `mesh_generator` / `mesh_editor` / `diagnosis_manager` 三权分立：生成、修复、诊断各自独立 manager；诊断输出含长径比、自由边、重叠单元等质量度量 | `services::meshing`（体素 + 5-四面体生成与报告一体）+ `services::geometry`（STL 流形/边检查）                                       | Kairos 诊断只有拓扑完整性，**缺长径比等形状质量度量**；auto_fix 自动修复是大工程（缓办，T22 Gmsh 集成是更现实的修复路线） |
| material          | `material_finder`（按库域检索：热塑/热固/underfill）+ `material_selector`（登记选择）                                                           | `services::material`（内置 + 自定义、校验、按 id 合并、宽容读取）                                                                   | moldflow 有**库域概念**；Kairos 单一自定义库当前够用，分类字段缓办                                                        |
| process / analyze | `study_doc.analyze_now`（触发求解）+ 分析类型枚举（WARP 等的边界条件随之变化）                                                                  | `services::openfoam`（case 生成）+ `services::jobs`（调度：并发预算 / 取消 / 进度流）+ `services::process`（参数校验）              | Kairos 调度器明显更现代（队列/预算/取消/日志流 vs 裸 `analyze_now` 阻塞调用）；**保持现状**                               |
| results / plot    | `plot_manager`（绘图创建/数据读取/XML 导出）+ `data_transform`（标量/矢量派生运算生成用户绘图）+ `viewer`（视角/截图/动画/书签）                | `services::results`（目录扫描 + 场读取）+ 前端 `render/`（WebGL2 视口）+ `render/snapshot`（报告截图）+ `lib/report`（自包含 HTML） | **缺派生结果算子**（对已有场做标量/矢量运算生成新场）——T20 GPU 算子目录已规划此类能力；视口书签/动画导出未做              |

## 三、改进建议清单（采纳 / 缓办）

| #   | 建议                                                                                                                                                | 决策       | 理由                                                                                              |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------- |
| 1   | 未来做 CLI / Python 绑定时采用**单门面 + 按域访问器**（`kairos.project` / `kairos.mesh` / `kairos.solve` / `kairos.results`），直接映射 services 层 | **采纳**   | moldflow-api 证明该形态对脚本化友好；T28 的 state 按域拆分后，Kairos 内部已是同构分片，暴露成本低 |
| 2   | 领域枚举集中一个 common 命名空间对外暴露（AnalysisStage、JobStatus、LicenseKind 等）                                                                | **采纳**   | 已基本满足（types.ts 单文件镜像）；未来 CLI 直接映射即可，零改造                                  |
| 3   | 网格诊断报告补充形状质量度量（长径比、体积比），而不只拓扑完整性                                                                                    | **缓办**   | 有价值但当前验收以拓扑健康为准；等 T22 Gmsh 集成时一并取 Gmsh 的质量指标更经济                    |
| 4   | 派生结果算子（对标 data_transform 的用户绘图）                                                                                                      | **缓办**   | T20 GPU 算子目录已覆盖同类需求（切片/归一化），按既有节奏推进                                     |
| 5   | 自定义材料库分类字段（热塑/热固/嵌件等域）                                                                                                          | **缓办**   | 内置示例 + 自定义两档已满足当前演示流；等真实材料库扩充需求出现                                   |
| 6   | 视口书签 / 动画导出 API                                                                                                                             | **缓办**   | 报告截图已有 snapshot 注册机制；书签无真实需求信号                                                |
| 7   | 单位换算助手（对标 unit_conversion）                                                                                                                | **缓办**   | 当前 mm 隐式约定由几何服务推断；等 STEP/英制导入需求出现再建                                      |
| 8   | `get_first/get_next` 游标遍历协议                                                                                                                   | **不采纳** | Kairos 返回 Vec / 数组更符合现代 API 习惯，无迁移价值                                             |

## 四、对脚本化 / 批处理接口的设计输入（汇总）

- **门面映射现成**：T28 之后前端状态已按域分片，Rust services 本就按域组织——未来 `kairos-cli` / `kairos-py` 只需把 commands 层的 IPC 参数翻译成函数签名，不需要新抽象。
- **批处理的关键前置**是「多研究编排」（对照表 study 行的缺口）：一个 `for study in studies: mesh → solve → collect` 的编排函数，现有 services 已能支撑，缺的只是入口（可作未来 CLI 任务的验收样例）。
- **错误契约是 Kairos 的优势**：moldflow-api 的 bool/异常混合在批处理中容易吞错；Kairos 的 `{code, message}` 结构化错误应原样穿透到脚本接口。
