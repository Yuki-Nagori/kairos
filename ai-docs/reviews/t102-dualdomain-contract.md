# T102 Dual Domain 上游联调契约

## 生成命令

```bash
cargo run -p kairos-cli -- dual-domain export \
  --stl report/mug-moldflow/mug.stl \
  --out /private/tmp/mug-dual-domain-v1.json \
  --json
```

上游 moldingFoam 使用生成的 JSON 做输入适配测试。原始 Mug STL 和导出的 JSON 只保留在本地，不提交仓库。

## Schema

- `schemaVersion`: 固定为 `dual-domain/v1`。
- `lengthUnit`: 固定为 `mm`。
- `thicknessUnit`: 固定为 `mm`。
- `nodes`: `[x, y, z]` 节点坐标，单位由 `lengthUnit` 指定。
- `triangles`: 三角形节点索引，索引从 0 开始。
- `thickness`: 与 `triangles` 等长，按三角形给出厚度。
- `beams`: 流道/浇口梁，包含 `nodes`、`diameter`、`kind`。
- `couplings`: 梁端点到双域节点的耦合，包含 `beam`、`endpoint`、`node`、`distance`。

## 上游验收顺序

1. 先验证 JSON schema、单位、索引和厚度数组长度。
2. 用仓库内 `tests/fixtures/dual-domain-v1.sample.json` fixture 完成单元测试，不依赖 Mug 专有网格。
3. 用 Mug 导出的 JSON 做网格摘要和读取 smoke test。
4. solver 模块确认能够读取中面节点、三角形、厚度和耦合关系后，再接入材料与工艺字典。
5. 最后执行 `fill-pack-cool` Mug 实验，并保留原始日志、输入 JSON、网格摘要和时间戳。

## 禁止事项

- 不把 `DualDomainSolverInput` 静默转换成普通体网格后标记为 Dual Domain 求解。
- 不改变单位或把厚度从三角形数组改成隐式默认值。
- 不把缺失厚度、越界索引或未匹配面当作零值继续运行。
- 不将参考报告中的专有原始网格或材料文件提交到任一仓库。
