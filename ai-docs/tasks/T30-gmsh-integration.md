# T30 · Gmsh 网格引擎正式集成

- 阶段：下一阶段（网格能力升级）
- 依赖：T22（评估与解析原型，已完成）
- 优先级：P1

## 目标

把 T22 评估过的 Gmsh 从「解析原型」升级为正式网格引擎：复杂几何（体素法失效的薄壁/曲面件）由 Gmsh Delaunay 生成真实四面体网格，Kairos 侧解析 .msh 并接入既有渲染/求解链路。

## 范围

- commands 层新增 `generate_gmsh_mesh`：调用应用内下载的 Gmsh 可执行文件（子进程 + .geo 临时脚本，遵循 GPL 隔离红线）；
- `services::gmsh` 解析器从原型扩展到生产：物理组、阶次过滤、节点/单元规模上限；
- `MeshingReport` 增加 Gmsh 引擎标识与质量度量（长径比 aspect ratio——顺带兑现 T25 建议 #3）；
- 几何面板增加引擎选择（体素 / Gmsh），Gmsh 缺失时引导下载（依赖面板已有直链）。

## 非目标

- Gmsh 自动修复 / 曲面重建；
- OpenFOAM case 模板对非体素网格的 snappyHexMesh 适配（后续任务）。

## 交付物

- `generate_gmsh_mesh` 命令 + services::gmsh 生产化解析；
- 几何面板引擎选择 UI；
- 测试：T22 的 .msh fixture 扩充为集成测试。

## 验收标准

- 给定曲面 STL，Gmsh 引擎产出四面体网格并渲染/读取成功；
- 体素引擎回归不受影响（既有测试全绿）；
- 报告含长径比统计。
