# T45 · 视口空间拾取与探针场关联

- 阶段：已实现并接入产品路径（视口点击拾取 → 探针 → XY 图表数值联动）
- 依赖：T14（视口引擎）、T15（XY 曲线与探针）
- 优先级：P2

## 目标

补全 XY 曲线 + 探针链路的前半段：在 3D 视口点击模型表面拾取单元，
自动加入探针列表；探针的数值与已加载场即时关联（XY 图表点标注与
数值随场加载 / 时间步切换刷新）。曲线/时间轴联动（探针时间序列）
拆分为后续任务。

## 方案与范围（已实现）

- `src-web/render/picking.ts`（纯函数，无 WebGL 依赖，可独立单测）：
  - `rayFromPointer(camera, x, y)`：轨道相机（eye / target / fov / aspect /
    画布尺寸）+ 指针坐标 → 世界射线（NDC 反投影，不用矩阵求逆）；
  - `pickCell(mesh, ray)`：Möller–Trumbore 逐面求交取最近命中，返回
    `faceCells` 映射的单元索引——与场值索引同域（场为单元关联），
    探针直接可用；
- `renderer.ts`：新增 `getCamera()` 相机快照（eye / fov / aspect / 画布尺寸），
  FOV 常量提取为 `FOV_Y` 消除渲染循环中的重复字面量；
- `useViewportPanel`：画布 pointerdown/up 位移 < 4px 判定为点击（不与
  旋转拖拽冲突），命中即 `results.addProbe(cell)`；未命中静默；
- 场关联复用既有链路：XY 图表探针点与值标注读取 `loadedField.values`，
  时间步切换 / 派生场刷新时自动更新。

## 当前实现边界

- 视口内不做探针标记点渲染（探针以 XY 图表圆点 + 值标注呈现），
  3D 标记依赖叠加层扩展，留待视口下一轮；
- 拾取目标为渲染网格（体积边界面或 STL 表面），非原始体素内部单元；
- 时间序列曲线与时间轴双向联动拆分为 T46。

## 验收标准（已满足）

- picking 单测：中心射线命中期望单元、偏离命中另一单元、射线背向 /
  脱空返回 null、非对称 aspect 下 NDC 映射正确（tests/web/render）；
- 视口面板测试：点击（位移 < 4px）触发探针加入，拖拽（大位移）不触发；
- `bun run verify` 全绿。
