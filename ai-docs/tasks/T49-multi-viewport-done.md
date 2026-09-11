# T49 · 多视口联动（多实例布局与相机 / 时间轴同步）

- 阶段：已实现并接入产品路径（单视口 / 四分格布局，相机与时间轴联动）；
  WebGPU POC 后端的相机发射为单向（无交互事件源），见当前实现边界
- 依赖：T14（视口引擎）、T39（后端工厂与公共方法面）、T45（拾取）、
  T46（时间曲线）
- 优先级：P2

## 目标

补齐 C7「多视口联动」：视口支持多实例布局（单视口 / 四分格），任一实例
的相机操作联动到其余实例；时间轴（结果场加载 / 播放 / 跳转）为全局状态，
所有实例同步呈现同一时刻。

## 方案与范围（已实现）

- `stores/viewport.ts`：新增 `layout: "single" | "quad"` 状态与
  `setLayout` 动作（图层意图同域管理，跨面板共享）；
- `render/renderer.ts`（WebGL2）与 `render/webgpu/renderer.ts`（POC）：
  - `ViewState` 扩展轨道参数（yaw / pitch / distance），
    新增 `getOrbit() / setOrbit()`；
  - 交互（旋转拖拽 / 滚轮 / 缩放 / 适应 / 复位）发出轨道快照事件，
    WebGPU POC 补齐同一事件面（此前缺失）；
- `render/backend.ts`：`ViewportBackend` 接口纳入轨道方法，工厂
  `onView` 回调携带完整轨道快照；
- `useViewportPanel` 重构为多实例：
  - `slots`（4 个实例槽位，reactive）+ `attachCanvas` ref 回调；
    共享网格数据一次获取、所有实例复用上传；
  - 相机联动：任一实例交互 → `syncOrbitFrom` 把其轨道复制到其余实例
    （源实例自引用跳过）；
  - 时间轴同步按构造成立：实例共享 `results.loadedField`，加载 / 播放 /
    跳转的 watch 链路对全部实例生效；
  - 面板控制条（载入 / 播放 / 剖切）广播到全部实例；拾取在任一实例可用；
  - 布局切换后新增实例自动补建（共享网格已缓存）。

## 当前实现边界

- WebGPU POC 后端作为相机联动的事件**源**不可用（其交互不发轨道事件），
  可作为联动目标；WebGL2 实例间双向联动完整；
- 剖切 / 图层 / 云图在全部实例间是全局联动（无独立剖切状态）；
- 四分格布局共享同一几何与结果数据，不支持各视口加载不同几何。

## 验收标准（已满足）

- 视口 store 布局状态单测（默认 single、setLayout 往返）；
- `ViewportBackend` 接口双后端编译期校验（vue-tsc）；
- 前端四维覆盖率保持 100%；`bun run verify` 全绿；
- 相机联动的运行时行为属命令式 WebGL 集成（vitest 豁免口径），真机验证。
