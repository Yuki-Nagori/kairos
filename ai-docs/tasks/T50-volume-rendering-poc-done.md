# T50 · 三维体渲染 POC（体绘制）

- 阶段：数据前置与 WebGPU 光线步进 POC 已实现；未接入产品视口交互，
  交互式迁移函数与真机性能数字待后续
- 依赖：T39（WebGPU 后端决策与类型基础）、T06（体积网格）
- 优先级：P2

## 目标

落地 C7「三维体渲染（体绘制）」的前半段：把非结构四面体场重采样为
结构化体素场（数据前置），并在 WebGPU 上以固定步长光线步进完成体绘制
POC——为非结构网格体渲染的独立设计提供可验证的实现基础。

## 方案与范围（已实现）

- `kairos-core` `services/volume_field.rs`（CPU，行覆盖计入门槛）：
  - `resample(volume, field, params)`：四面体网格 + 单元关联场 →
    结构化体素场 `VolumeFieldGrid`（dims / origin / spacing / values /
    covered_ratio）；
  - 采样按四面体包围盒栅格化 + 重心坐标同号判定（含浮点容差），
    复杂度与全场体素数解耦；先到先得，覆盖率作诊断输出；
  - 分辨率 8..=256、空网格、场值/四面体数量不一致均明确报错；
- `src-web/render/webgpu/`：
  - `volume.ts`：`VolumeRaymarcher`——体素场 storage buffer 上传 +
    全屏三角形光线步进（96 步、三线性采样、前向合成、冷热迁移函数、
    轨道相机旋转 / 缩放、FPS 回报）；
  - `shaders_volume.ts`：uniform 布局按 vec4 成员对齐（dims / origin /
    spacing / eye / 相机基向量 / 视口 / 标量），逐像素由相机基向量构造
    射线（无逆矩阵）；
- `utils/bench/volume-render.ts`：128³ 确定性球体密度场上传 / 首帧 /
  中位 FPS 基准（需 GPU 环境运行，数字待真机）。

## 当前实现边界

- 未接入产品视口：体渲染以独立模块 + 基准入口交付，产品接线（视口
  模式开关、迁移函数 UI）待 T48 真机数字后决策；
- 迁移函数固定（冷热映射 + 值相关不透明度），步数固定 96；
- 覆盖空洞（网格点未被任何四面体命中）按 0 处理，诊断靠 covered_ratio。

## 验收标准（已满足）

- core 单测：分辨率边界、空网格 / 数量不一致报错、四面体内部采样与
  外部置零、两四面体区分值域、确定性（kairos-core 行覆盖 100%）；
- WebGPU 模块类型检查通过（渲染行为属 GPU 集成豁免口径，真机验证）；
- `bun run verify` 全绿。
