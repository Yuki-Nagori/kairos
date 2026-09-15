# T101 · Generic PP 自定义材料拟合与 Mug baseline

- 阶段：E3（优化与自动化）
- 依赖：T04、T76、T100
- 状态：**进行中（先补齐通用材料曲线输入与拟合能力）**
- 目标：建立可审计的 Kairos Generic PP 自定义材料，拟合并验证 PVT/Tait 与 Cross-WLF 参数，使用 `report/mug-moldflow/mug.stl` 重新生成 baseline。

## 数据输入契约

1. PVT 数据集应能表达多个压力面、温度点和比容/密度曲线；黏度数据集应能表达多个温度面、剪切速率和黏度曲线。文件名、列顺序和采样数量不能写死在业务层。
2. 导入契约必须携带列名、单位、温标、压力/剪切速率单位和数据来源标识；缺少单位或包含非有限值时拒绝拟合。
3. 参考工况与结果：`report/mug-moldflow/setting.txt`、`result.txt`、`log.txt`。原始材料资产只在本地或受控存储留档，不进入代码仓库。
4. 任何受控留档记录 SHA-256、来源、单位确认和授权状态；报告只提交脱敏后的参数、残差和生成时间。

## Kairos 当前缺口（实现清单）

当前材料服务仍未形成从曲线到可运行材料的闭环。曲线 CSV 的单位归一和基础质量校验已落到 `kairos-core`；T101 还需要补齐：

1. **曲线解析与单位归一**：已增加 `material_curve` 输入模型与 CSV/分段 TXT 解析，支持压力/温度/比容/剪切速率/黏度的单位归一、排序和重复点拒绝；后续补充更丰富的元数据格式。
2. **拟合核心**：提供确定性的 Tait 与 Cross-WLF 拟合服务，输出参数、权重、拟合区间、收敛状态和版本化算法标识；数值计算留在 Rust core。
3. **残差报告**：已在 `kairos-core` 增加 Cross-WLF 与 Tait 预测、摘要计算及 JSON/CSV 摘要与逐点明细导出（绝对误差、相对误差、RMSE、`log10(η)` 最大误差与 RMSE）。
4. **材料资产元数据**：扩展材料 DTO，记录来源类型、单位、拟合算法版本、参数修订号、数据摘要哈希和创建时间，支持版本化而不覆盖内置材料。
5. **CLI 闭环**：增加 `material fit` / `material validate`，并让 Mug/DOE case 按材料 ID 或路径选择自定义材料；core 已提供按 `.json`/`.csv` 扩展名分派的受控材料文件读取，`material fit`、`material validate`、`doe run --material <path>` 与 `pipeline run --material <path>` 已接线，fit 还会输出 moldingFoam 材料字典和逐点残差；真实 baseline 与完整多参数优化仍待完成。
6. **IPC 与界面入口**：补齐 Tauri 命令、前端材料面板的曲线预览/拟合结果/残差下载，以及错误码到 UI 的映射。
7. **测试与契约**：覆盖单位换算、乱序和重复点、缺失/非法数据、拟合失败、JSON/CSV round-trip、DTO 契约和 case 生成回归；使用合成夹具，不提交受限原始曲线。

## 当前已具备

- `kairos-core` 已支持内置材料读取、已拟合自定义材料 JSON/CSV 导入导出和基础字段校验。
- T100 已冻结 Mug 参考工况与对比口径，但 CLI 仍未把自定义材料选择接入真实求解。

## 运行时边界

- `kairos-core` Rust 负责曲线解析、单位归一、参数拟合、残差验证、材料版本和求解器配置生成。
- `moldingFoam` 在真实求解循环中按单元/时间步计算 Cross-WLF 与 Tait；不能通过 IPC 逐步回调 Rust，也不能由前端计算。
- 两端必须共享参数命名、单位和公式说明，并用固定参数的 golden 点测试校核黏度/比容结果，防止参考实现与运行时实现漂移。
- 当前 core 已固定 Cross-WLF/Tait golden 点；moldingFoam 运行时对照和 Mug 实际回归仍待完成。
- 本地受控曲线插入测试已验证解析和输出链路；原始内置 Tait 模板不兼容时会明确失败，避免静默生成伪拟合结果。

### Tait solver parity audit

2026-09-15 对 moldingFoam v1.1.0 源码复核后确认：运行时使用
`B(T)=b3·exp(-b4T)`、`Tt=b5+b6p` 和
`v=v0(T)·(1-C·ln(1+p/B(T)))`，并对固态/熔态分支做平滑过渡。当前 Kairos
临时 Tait 评估器已按该公式扩展，`Tait` DTO 已加入 `b3s/b6/C/smoothBand` 默认值，并实现固/熔态 smoothstep 过渡；仍需用 moldingFoam 运行时输出完成跨语言 golden 点和材料导出回归，再进行 Mug baseline。

## 工作范围

### PVT / Tait

- 确认第三列是比容还是密度，并固定单位换算；禁止凭数值猜单位。
- 对每个压力面拟合温度函数，检查熔体/固体转变区连续性。
- 拟合 `b1m/b1s/b2m/b2s/b3/b4m/b4s/b5`，保留残差、拟合区间和权重。
- 用原始数据回代，在所有压力/温度点输出最大绝对误差、最大相对误差和 RMSE。

### Cross-WLF

- 从黏度曲线拟合 `n/tauStar/D1/D2/D3/A1/A2`。
- 在四个温度面和完整剪切速率范围回代，输出 log10(黏度) 残差与最大误差。
- 特别核对 220 °C 工况附近的黏度曲线；不能只用单点拟合。

### Kairos 材料与 baseline

- 以独立自定义材料导入，不直接修改 `builtin-materials.json`。
- 材料 DTO、CSV/JSON 导入、case 生成统一使用同一份拟合参数。
- Mug baseline 固定使用 `report/mug-moldflow/mug.stl`、220 °C 熔体、50 °C 模具、5.5 s 注射，并记录体积网格尺寸与参考 Dual Domain 网格差异。
- 重新运行 CLI `doe run --solve --vm`，保留 case、`log.foamRun`、时间戳和原始汇总。

## 验收标准

- PVT 与黏度拟合报告可由原始文件重算，参数和误差均有脚本化输出。
- 自定义材料通过材料校验、DTO 契约测试和 case 生成回归。
- Mug baseline 至少对照填充结束时间、最大注射压力、V/P 切换压力、填充率、温度和质量预算残差。
- 报告明确区分：材料误差、网格类型差异、边界/工艺差异和求解器差异；不把不同网格结果宣称为逐点等价。
- `bun run verify` 全绿；所有执行记录按时间戳归档。

## 当前风险

- 原始文本缺少显式列单位和材料数据库元数据，需先完成单位确认。
- 参考报告的材料名称是 Generic PP，但数据库版本可能不同；名称相同不能推断参数完全相同。
- 公开典型值只能做量级检查，不能替代实测或已授权曲线。
