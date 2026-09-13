# T64 · 翘曲结果场消费（位移场口径）

- 阶段：B2/C6（求解器 · 翘曲分析的 Kairos 侧）
- 依赖：T13（结果模型与流式读取）、T61（矢量场口径惯例）
- 优先级：**P2**

## 目标

把翘曲分析的位移场纳入 Kairos 的结果消费口径，使 moldingFoam M4 的翘曲输出
一落地就能被结果面板/视口展示（变形量云图），不必再改读取层。

## 范围

- `services/results.rs::is_vector_field` 增加 `D`（位移，OpenFOAM 结构求解
  惯例）：位移场按矢量口径解析为**模量**（`is_magnitude = true`），与 `U`
  走同一条 `parse_internal_vector_magnitudes` 路径，可直接在视口按云图显示；
- 同步模块注释与文档，写明字段名待 M4 定稿后同步；
- 变形（视口按位移场扭曲网格）另立 T65，本任务只解决"消费"。

## 非目标

- 视口变形显示（T65）；
- 矢量场**三分量**数据通道（T65 需要；现在只取模量）；
- 翘曲求解本身（moldingFoam M4）。

## 验收标准

- 合成 `D` 场（非均匀矢量）读出模量、`is_magnitude = true`、`complete` 正确
  ✅（单测 `displacement_field_reads_as_magnitude`）；
- 不影响既有标量/矢量场读取与扫描（全量测试绿）；
- `bun run verify` 全绿。

## 当前实现边界

- 字段名 `D` 是按 OpenFOAM 惯例的**预置口径**：M4 若改用别的名字（如
  `displacement`），改 `is_vector_field` 一处并同步本文档；
- 只支持模量展示，不支持三分量（变形显示在 T65）。

## 回填（2026-09-13，上游 038 §7b 契约）

上游给出翘曲场契约：`D`（volVectorField，**SI 米**，显示按 mm ×1000）、
`sigma`（volSymmTensorField）、`sigmaEq`（volScalarField）、`T`（K）等。
据此把「按场名猜类型」换成**按 FoamFile 头的 `class` 行判定**：

- `FieldKind::{Scalar, VectorMagnitude, Unsupported}`：`volScalarField` 读标量、
  `volVectorField` 读模量（`D`、`U` 一视同仁，不再维护场名白名单），
  `volSymmTensorField` 等明确报「暂不支持该场类型」——避免把张量按标量误解析；
- 位移场的显示单位（m → mm）属展示层换算，随变形显示一并做（见 T65）。
