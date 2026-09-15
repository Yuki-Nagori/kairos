# T102 · Dual Domain Mug 完全一致基线

- 阶段：E3（求解器一致性与验证）
- 依赖：T29、T82、T100、T101
- 状态：**进行中：CLI FillPackCool 与双域拓扑前置校验已接入，solver 消费待实现**
- 目标：在 Kairos 中导入或重建与参考结果相同的 Dual Domain 中面网格，使用同一拓扑、厚度、边界、材料和工艺参数，完成 Mug 的可审计基线对照。

## 冻结基线参数

唯一参数来源为 [`ai-docs/report/mug-baseline.md`](../report/mug-baseline.md) 与本地受控参考设置。实现和测试不得在命令、GUI、case 生成器之间复制另一份默认值。

| 类别     | 冻结值                                                                                                                                                                         |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 几何     | `report/mug-moldflow/mug.stl`；总体积 `1163.6855 cm³`                                                                                                                          |
| 参考网格 | Dual Domain；`33418` nodes；`66830` triangles                                                                                                                                  |
| 材料     | 内置 `PP-REF-01`；参数快照见 Mug baseline 第 3.1 节                                                                                                                            |
| 温度     | 熔体 `220 °C`；型腔/型芯模温 `50 / 50 °C`                                                                                                                                      |
| 注射     | 注射时间 `5.5 s`；名义流量 `211.5792 cm³/s`                                                                                                                                    |
| 保压     | `(0 s, 0.9229 MPa) → (0.2 s, 27.6282 MPa) → (315.0797 s, 27.6282 MPa)`                                                                                                         |
| 冷却     | `20 s`                                                                                                                                                                         |
| 参考输出 | 充填结束 `5.7916 s`、最大注射压力 `2.5106 MPa`、质量 `899.7875 g`、最大锁模力 `6.5605 t`、平均填充温度 `218.9642 °C`、最大剪切应力 `0.1552 MPa`、最大剪切速率 `1.0061×10⁴ s⁻¹` |

## 实施范围

1. **Dual Domain 网格**
   - 定义受支持的中面网格输入格式，保存节点、三角形、厚度、法向和双面匹配关系。
   - 优先复用 Kairos 现有中面/双域导入能力；无法读取参考网格时，使用确定性算法从 STL 重建并输出匹配率、厚度误差和质量报告。
   - 记录网格摘要哈希；禁止把不同网格静默标记为同一基线。

2. **双面厚度与积分**
   - 在 core 中实现双面温度、压力、速度和体积分数的厚度方向积分。
   - 固定厚度采样点、积分权重、穿透方向和退化单元处理。
   - 对薄壁、厚度突变、法向翻转和未匹配面提供结构化错误。

3. **solver 拓扑消费**
   - moldingFoam 接收同一节点/面拓扑、边界 patch、厚度和匹配关系。
   - 统一入口、排气、壁面、模具热边界和 V/P 切换面定义。
   - 运行前输出拓扑、patch 面积、厚度统计和网格质量；运行后原样归档。

4. **材料与工艺一致**
   - 复用内置 `PP-REF-01`，Cross-WLF、Tait、比热、导热率和机械字段只从材料 DTO 生成。
   - 复用 CLI 的完整保压曲线与冷却时间参数，不在 solver 字典中硬编码第二套值。
   - 统一 SI 单位、温标、压力基准、密度/比容定义和结果采样时间。

5. **输出指标一致**
   - 明确填充结束、最大注射压力、V/P 切换压力、质量、锁模力、平均填充温度、剪切应力和剪切速率的定义。
   - 输出原始 solver 日志、时间目录、指标 JSON/CSV、网格摘要和运行时间戳。
   - 报告逐项给出参考值、Kairos 值、绝对差、相对差和误差来源分类。

## 当前实现进度

- CLI 已新增 `dual-domain export`，只接受真实零件 STL，生成 `dual-domain/v1` JSON；Dual Domain 不再提供闭合方盒样例，避免把 `sum(area × thickness)` 误当作实体体积。
- 上游联调顺序和字段定义已记录在 [T102 Dual Domain 上游联调契约](../reviews/t102-dualdomain-contract.md)。
- 仓库保留 `tests/fixtures/dual-domain-v1.sample.json` 作为纯 DTO 拓扑契约 fixture（不是物理几何或基线结果），完整 Mug 导出必须使用本地 `report/mug-moldflow/mug.stl`；实验 manifest 已指向生成的 `report/mug-moldflow/mug-dual-domain-v1.json`。

示例：

```bash
cargo run -p kairos-cli -- dual-domain export \
  --stl report/mug-moldflow/mug.stl \
  --out /private/tmp/mug-dual-domain-v1.json --json
```

该命令会读取 Mug STL，输出节点、三角形、厚度和梁耦合数据；不再接受 `--sample-box`。

同一 Mug 也生成了三维体网格 fixture `tests/fixtures/volume-mesh-v1.mug.json`（15,868 节点 / 48,605 四面体），manifest 的 `volumeMesh` 字段指向它，供上游同时验证两种输入。

## 路线确认（2026-09-16）

moldingFoam v1.1.0 当前求解器通过 `foamRun` 消费三维 OpenFOAM 体网格（`polyMesh`、体心场和体单元离散），源码没有壳单元、Dual Domain 双面节点或中面厚度场的输入契约。现有 `DualDomainMesh` 是 Kairos 前处理产物，不能直接写成现有 `physicalProperties`/`polyMesh` 并声称等价。

T102 后续采用两步路线：

1. 先定义稳定的 `DualDomainSolverInput`（中面节点、三角形、厚度、双面匹配、边界和积分规则）及契约/golden 测试。
2. 在 moldingFoam 增加显式降维求解模块后，再由 Kairos 写出该模块的 case；在此之前只允许拓扑前置校验和报告导出，禁止静默回退到 3D 体网格。

因此，当前 Mug 命令已经做到工艺参数一致，但 Dual Domain 网格与 solver 消费仍未完成；报告中的数值结果不得标记为完全一致。

- CLI 已支持 `fill-pack-cool` 完整工艺阶段。
- `dualdomain::validate_solver_topology` 已阻止未配对厚度、越界三角形和非法拓扑进入未来 solver 适配层。
- 已新增版本化 `DualDomainSolverInput`（`dual-domain/v1`，显式 mm 单位）及 Rust/IPC 契约测试。
- moldingFoam 当前仍只消费 `VolumeMesh`；双域表面/厚度数据尚未转换为可求解的降维 case，因此尚未宣称 Dual Domain 求解完成。

## 验收标准

- 参考网格或重建网格的节点、面、厚度和匹配关系均有可审计摘要；不能只比较节点数。
- 同一 case 在 CLI、Tauri 和 moldingFoam 生成的字典内容逐字一致（时间戳等非物理字段除外），并使用 `FillPackCool` 阶段覆盖完整工艺时长。
- 固定基线重复运行结果在容许数值误差内重现，所有随机性和并行归约策略有说明。
- Mug 结果逐项完成与参考值对照；网格、材料、边界或 solver 差异必须单独列出。
- 失败运行保留原始日志、退出码、参数和网格摘要，不能用成功摘要覆盖。
- `bun run verify`、Dual Domain 契约测试、solver golden 测试和一次五核 VM 真实运行全部通过。

## 非目标

- 不把不同网格的结果宣称为逐点等价。
- 不将未经授权的参考材料原始文件或专有网格文件提交到仓库。
- 不通过调整材料参数或后处理比例强行贴合参考输出；任何校准都必须形成独立材料修订和实验记录。

## 交付物

- Dual Domain 输入/重建格式说明与契约测试。
- 中面网格质量、厚度和匹配关系报告。
- moldingFoam 双面厚度积分实现及 golden 点。
- Mug 完全一致基线命令、原始运行归档和逐项差异表。
- 更新后的 [Mug 基线报告](../report/mug-baseline.md) 与时间线记录。
