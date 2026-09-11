# Kairos 架构状态与验收清单

对标商业注塑成型软件的标准架构（前处理 / 求解器 / 后处理 / 扩展模块），
逐分支记录实现、集成与验收的真实状态；它是功能状态的权威来源，任务文件名 `-done`
只表示有实现提交。

**特殊标注**：`🏷️ moldingFoam` = 该能力由
[Yuki-Nagori/moldingFoam](https://github.com/Yuki-Nagori/moldingFoam)
（基于 OpenFOAM-14 的注塑求解项目）承担，Kairos 侧只负责调用、数据准备
与结果消费（GPL 隔离：子进程 + 文件交换）。

> 状态截至 2026-09-11。任务编号（Txx）对应 ai-docs/tasks/ 索引。

## 状态口径

- `- [x]`：功能已实现并接入当前产品路径；
- `- △`：仅有基础实现或原型，关键集成、性能或跨平台验收未闭环；
- `- [ ]`：尚未实现；
- `- ⚠`：外部依赖或已知问题阻塞。

“完成度”仅衡量实现覆盖，**不替代性能、三端兼容与真实求解验收**。后处理以
硬件加速 GPU 为运行前提：无可用 GPU 时显示能力不受支持，CPU 代码只作数值对拍，
不再作为运行时回退。后处理渲染路线见 [T39](tasks/T39-postprocess-renderer.md)。

## A · 商业注塑成型软件（Kairos）

三大主分支 + 扩展模块之外，Kairos 还有一个图形里没有的**平台底座**层
（桌面壳 / VM 求解执行 / 依赖管理 / 发布），见文末。

---

## B1 · 前处理（几何与网格）

### C1 CAD 接口（模型导入与修复）

- [x] STL 导入（ASCII / 二进制解析）
- [x] 健康检查：非流形边、退化三角形、法向不一致检测与统计
- [x] 内置样例几何（样例方盒，首次使用引导）
- [x] STEP 镶嵌网格导入（AP242 TRIANGULATED_FACE_SET / POLY_LOOP 面子集）
- [ ] IGES 导入（106/63 镶嵌子集，待实现）
- [x] 模型修复工具：重复顶点焊接、退化面移除、边界环孔洞填充、
      法向 BFS 一致化、自交对检测计数（面板「修复」动作）

### C2 网格划分（3D / 双域 / 中面）

- [x] 3D 体素网格：体素占用 + 立方体 5-四面体保形分解（规则几何）
- [x] 3D Gmsh 引擎：Delaunay 四面体（复杂/曲面件，T30）🏷️ moldingFoam
      消费其输出（引擎本体为 gmsh 官方可执行文件，经依赖面板分发）
- [x] 网格质量度量：最长边/最短边比统计、最小体积、引擎标识
- [ ] 双域网格（表面 + 杆系/中面耦合）
- [ ] 中面网格（1D/2.5D 快速分析路线）
- [ ] 局部加密 / 边界层

### C3 材料数据库（PVT / 流变 / 热性能）

- [x] 内置公开典型值参考库（PP / HDPE / PA66 / POM / PMMA 等多族参考牌号，非生产级
      商业牌号数据）
- [x] Cross-WLF 黏度七参数（流变）
- [x] Tait 双域 PVT 八参数 + 力学参数预留（弹性模量/泊松比）
- [x] 温度表热物性：比热、导热（线性插值 + 端点截断）
- [x] 自定义材料：文件导入、按 id 合并覆盖、校验
- [x] 牌号数据集扩充：内置参考牌号 + 21 列 CSV 批量导入格式
- [x] 纤维/填料参数组：类型、质量分数、长径比和数据说明；尚未接入纤维取向求解

---

## B2 · 求解器（核心计算引擎）

> Kairos 不内置求解器。求解物理全部由 🏷️ **moldingFoam**（OpenFOAM-14
> foamRun 模块化框架 + 注塑求解模块）承担；Kairos 负责 case 生成
> （case-contract v1.1 字典）、虚拟机执行链路、日志/进度流回传。

### C4 流动分析（充填 / 保压）

- [x] 🏷️ moldingFoam 求解模块（compressibleVoF 基座，熔体/空气 VoF 填充）
- [x] 🏷️ moldingFoam V/P 切换保压：填充体积分数触发 + 闸口压力跟随
      `packing.pressure` 曲线（双模式浇口边界条件）
- [x] Kairos case 生成：contract v1.1 字典布局（moldingDict /
      momentumTransport CrossWlf / physicalProperties Tait+hMelt /
      phaseProperties / 0/ 五场）
- [x] Kairos 侧 VM 执行链路：bundle 部署 + tar 复制 + multipass exec
      （T36）
- [ ] 真机求解复跑：阻塞于 bundle SIGILL（CPU 指令兼容性，已移交
      moldingFoam 侧修复，见 ai-docs/reviews/e2e-solve-report.md）
- [ ] 多型腔 / 流道系统参与填充（依赖浇口几何落地）

### C5 冷却分析

- [x] 🏷️ moldingFoam M3：模壁恒温冷却 + 保压释放 + 顶出判据
      （`cooling.ejectionTemperature` / `releasePressure`）
- [x] Kairos 工艺映射：模温/顶出温度/冷却时间 → 字典与阶段控制
- [ ] 冷却水路瞬态求解（水路网络数据模型已就绪，求解侧无）

### C6 翘曲分析

- [ ] 🏷️ moldingFoam M4（规划中；Kairos `material.mechanics` 弹性模量/
      泊松比已预留）
- [ ] Kairos 侧翘曲结果场消费与变形可视化

---

## B3 · 后处理（结果可视化）

### C7 结果云图 / 曲线

- △ 3D 视口：当前为 WebGL2 表面网格、标量云图、基础平面剖切和线段叠加；
  WebGPU 后端、百万三角形性能和完整剖切尚未验收
- △ XY 曲线 + 探针：支持按节点序号取值；尚无空间拾取、场关联或曲线/时间轴联动
- △ 派生结果算子（T31）：归一化 / 阈值掩码已接入 core CPU 路径；线性映射、两场差值
  与 WGSL 原型尚未接入产品管线，未完成百万值 GPU 性能验收
- [ ] 三维体渲染（体绘制）
- [ ] 多视口联动 / 剖面切片

> 结果数据目前整文件读入并以 JSON 数组传输；无二进制分块、缓存淘汰、完整矢量场或
> 研究结果持久关联。因此不能将现有视口视为大结果后处理已完成。

### C8 报告生成

- [x] 结果统计摘要（min/max/计数/完整性）
- [x] 场数据 CSV 导出
- [ ] 富文本/HTML 分析报告（配置快照 + 云图截图 + 曲线）
- [ ] 模板化自定义报告

---

## D · 扩展 / 高级模块

### E1 特殊工艺模块

- [ ] 气体辅助注塑（GAIM）
- [ ] 双色/多组分注塑
- [ ] 微发泡（MuCell）

### E2 多物理场耦合

- [ ] 纤维取向对收缩的影响
- [ ] 流固耦合（制品与模具变形交互）

### E3 优化与自动化

- [x] 无头 CLI 与批处理入口（T32：project / mesh / solve / results /
      pipeline run，--json 结构化输出）
- [ ] DOE / 正交试验编排（依赖 T29 真机求解闭环）
- [ ] 工艺参数自动寻优（基于结果的闭环迭代）

---

## D0 · 平台底座（架构图之外的已完成层）

- [x] 桌面壳：Tauri 三平台（macOS/Windows/Linux CI 全绿）、原生菜单、
      全局快捷键、状态栏、主题（明/暗）
- [x] VM 求解执行层（T35/T36）：Multipass（macOS）/ WSL2（Windows）
      一键安装 + 按宿主机规格动态建 VM + 应用内交互 Shell + 退出联动关机；
      bundle 部署自动化（tar 传输 + 解压 + OpenMPI 安装 + 国内 apt 镜像）
- [x] 依赖管理器（T27/T34）：许可分级 + 官方直链白名单下载 + 自动解压 +
      GitHub release 在线检查更新（按宿主架构选资产、国内镜像源）
- [x] 求解环境分发：moldingFoam release bundle（OpenFOAM-14 环境树 +
      libmoldingFoam），替代源码全量编译
- [x] 无头 CLI（T32）
- △ 覆盖率门槛已并入 verify 门禁（前端逻辑层 100% · Rust core 行 100%）+
  cargo audit；当前 GPU 测试在无适配器环境会直接失败，尚未符合“GPU 必需但
  CI 可验证”的新策略
- [ ] updater 签名证书（T33 尾项：证书就绪后启用 macOS 公证 / Windows
      signtool）
- [ ] Linux 原生执行通道（当前 Linux 走本机 bash，OpenFOAM 需自装）

---

## 里程碑对照（快速结论）

| 分支                   | 完成度 | 最大缺口                                              |
| ---------------------- | ------ | ----------------------------------------------------- |
| B1 前处理              | ~75%   | CAD 主流格式、双域/中面                               |
| B2 求解器（Kairos 侧） | ~65%   | 真机复跑（等 bundle SIGILL 修复）；物理在 moldingFoam |
| B3 后处理              | ~20%   | 大结果数据链、WebGPU、切片/体渲染、报告               |
| D 扩展                 | ~10%   | 全部待排                                              |
| D0 平台底座            | ~80%   | GPU 必需策略下的 CI、updater 证书、Linux 通道         |
