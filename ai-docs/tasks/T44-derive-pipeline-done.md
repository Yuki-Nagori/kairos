# T44 · 派生结果算子产品管线补全（线性映射 + 两场差值）

- 阶段：已实现并接入产品路径（CPU 管线四算子齐备）；GPU 接入与百万值性能验收归 T39
- 依赖：T13（结果模型）、T31（算子基础实现与 WGSL 原型）
- 优先级：P2

## 目标

补全 T31 的产品管线缺口：归一化 / 阈值掩码之外，把线性映射与两场差值
接入 core 派生管线与结果面板 UI，使「用户派生绘图」的四类基础算子
（对标 moldflow-api data_transform）全部可从产品操作到达。

## 范围（已实现）

- `kairos-core` `models/results.rs`：`DeriveRequest` 可辨识请求 DTO
  （`Normalize | Threshold | Linear{scale,offset} | Difference`，serde 内部
  标记 `kind`，camelCase）；
- `kairos-core` `services/results.rs`：
  - `derive_scalar_field` 改收 `DeriveRequest`，新增线性映射
    （命名 `源场 · 线性映射 ×scale +offset`）；Difference 经单场入口
    明确报错（需要两份数据）；
  - 新增 `derive_difference(primary, compare)`：逐值相减，长度不一致报
    验证错误，完整性取两场与，命名 `主场 - 对比场`；
- `src-tauri` `commands/results.rs`：
  - `ResultSession` 扩为双槽（primary 主场 / compare 对比场）；
  - `load_result_field` 增加 `slot` 参数（"primary" 默认 / "compare"）；
  - `derive_field` 收 `DeriveRequest`；新增 `derive_difference` 命令
    （缺任一场时报验证错误并指明缺失槽位）；
  - 派生不改写会话缓存：源场保持不变，派生场只回显前端（后续派生
    始终基于源场，避免链式漂移）。
- 前端：
  - 结果面板场加载增加槽位选择（主场 / 对比场），两场差值按钮在双场
    就绪前禁用并提示对比场状态；
  - 派生类型下拉扩展线性映射，附 scale / offset 数值输入；
  - store 增加 `compareField` 状态与 `deriveDifference` action。

## 当前实现边界

- 派生管线仍在 core CPU（f64）执行；WGSL 算子（scalar_linear /
  scalar_threshold / scalar_difference 已在 gpu_ops.rs 且有 GPU-CPU
  一致性测试）接入产品管线与百万值 GPU 性能验收，按架构关口归 T39；
- 派生场不持久化（不在工程文件 / 结果目录登记），刷新后需重新生成；
- 差值只支持两场，N 场组合不在范围。

## 验收标准（已满足）

- core 单测：线性映射数值与命名、差值逐值相减与命名、长度不一致报错、
  完整性标记与、Difference 经单场入口报错（kairos-core 行覆盖 100%）；
- `DeriveRequest` 契约测试锁定 `kind` 标签与字段形状；
- 前端 store / 面板测试覆盖线性参数透传、对比场槽位、差值派生与错误路径；
- `bun run verify` 全绿。
