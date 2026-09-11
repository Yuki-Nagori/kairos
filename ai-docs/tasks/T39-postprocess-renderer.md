# T39 · 后处理渲染后端 POC 与技术决策

- 阶段：待开工
- 依赖：T13、T14
- 优先级：P0（后处理继续扩展前的架构关口）

## 已采纳的方向

Kairos 的主路径是**自研前端渲染后端**：以 WebGPU 为目标主后端，现有 WebGL2
保留为仍使用硬件 GPU 的兼容实现。后处理要求硬件加速 GPU；没有可用 GPU 时显示
能力不受支持，不提供 CPU 渲染或 CPU 算子回退。

不在产品主包中引入 VTK.js，也不让 VTK 类型进入 core、IPC DTO、Pinia store 或工程
文件。VTK.js 只允许作为独立、可删除的 POC，用来量化它是否能显著降低表面后处理
（云图、裁剪、拾取、多视图）的实现成本。

## 为什么不是直接采用 VTK.js

- Kairos 的结果是非结构四面体网格及 cell/point 关联场；VTK.js 的 `VolumeMapper`
  面向规则 `ImageData`，不能直接完成四面体体渲染。
- VTK.js 的 WebGPU 路径仍属预览能力，不能支撑 Kairos 的三端主后端承诺；其成熟
  路径是 WebGL。
- Rust `wgpu` 计算资源不能和 WebView 中的 VTK.js/WebGPU 资源共享；引入 VTK.js
  不会消除结果传输、分块缓存或 GPU 资源管理问题。
- 当前自研数据结构可直接表达单元归属、时间步和将来的非结构切片；把这些语义提前
  映射为 `vtkPolyData` / `ImageData` 会固化额外转换成本。

## POC 范围

1. 在真实 Tauri WebView（macOS WKWebView、Windows WebView2、Linux WebKitGTK）
   中验证，不以独立 Chrome 页面代替；
2. 仅导入 VTK.js 的表面渲染、色标、裁剪和 cell picking 所需模块，使用动态导入；
3. 用 Kairos 中立的 `SurfaceMesh + ScalarField` 适配到 VTK.js，适配代码必须只在
   `src-web/render/vtk/` 内；
4. 使用确定性 100 万三角形、32 时间步的资产，测量首帧、稳定 FPS、拖动裁剪、拾取、
   峰值内存和增量包体；
5. 与同资产的自研 WebGL2/WebGPU 后端对比，不把 VTK.js 内置的 ImageData 体渲染当成
   非结构网格体渲染的验证。

## 验收与决策门

| 条件       | 通过标准                                                       | 结论                       |
| ---------- | -------------------------------------------------------------- | -------------------------- |
| 三端稳定性 | 三个真实 Tauri WebView 均可初始化、无 API/驱动规避补丁         | 才能考虑保留 VTK.js 适配器 |
| 表面性能   | 100 万三角形稳定 60 FPS；32 时步切换和裁剪不产生可感知卡顿     | 否则继续自研               |
| 资源预算   | 增量包体、峰值内存和首帧符合 T01 预算                          | 超预算则不引入             |
| 架构隔离   | VTK 类型不穿透 `render/vtk/`，卸载 POC 不影响领域/IPC/项目文件 | 不满足则不合并             |
| WebGPU     | 仅记录可用性；不以 VTK.js WebGPU 的成功作为产品依赖条件        | 仍由自研后端负责主路径     |

POC 通过也只说明 VTK.js 可作为表面后处理的**可替换实现**；并不改变自研 WebGPU、
中立结果数据模型和非结构体渲染独立设计这三项决策。
