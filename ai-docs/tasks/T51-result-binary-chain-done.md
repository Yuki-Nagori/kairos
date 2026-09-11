# T51 · 大结果数据链第一步（二进制通道 + 会话缓存淘汰）

- 阶段：已实现并接入产品路径（场加载走二进制 IPC 通道 + FIFO 场缓存）；
  研究结果持久关联、分块懒加载待后续任务
- 依赖：T13（结果模型）、T44（会话双槽）
- 优先级：P1

## 目标

回应 C7 备注中的大结果数据链缺口第一步：场数据不再以 JSON 数组跨 IPC
传输（体积与解析双开销），改走二进制原始字节通道；会话增加有界缓存，
重复加载同一（目录, 时间步, 场）跳过磁盘读取并淘汰最旧条目。

## 方案与范围（已实现）

- `kairos-core` `services/results.rs`：
  - `field_binary` 编解码：`[magic "KF1\\0" 4B][meta_len u32 LE][meta JSON]
[f64 值 × n LE]`——元数据（场名 / 时间步 / 完整性等）走 JSON 便于
    演进，值区走定宽小端二进制；解码对魔数 / 截断 / 数量逐一校验；
  - `FieldCache`：有界 FIFO 缓存（容量 1..=64），同键覆盖不重复占位、
    超容量淘汰最旧条目；
- `src-tauri`：
  - `ResultSession` 纳入 `FieldCache`（默认容量 8）；`load_result_field`
    命中缓存跳过磁盘读取；
  - 新增 `load_result_field_binary` 命令：返回 `tauri::ipc::Response`
    原始字节（AGENTS「大体积数据用 Response」通道），同样写入槽位与缓存；
- 前端：
  - `utils/field-binary.ts`：`decodeFieldBinary`（DataView 小端解析，
    魔数 / 截断 / 值区校验）；
  - `api/results.ts` 的 `loadResultField` 切换为二进制通道并解析回
    `ScalarField`——store 与视口 / 图表零改动受益。

## 当前实现边界

- 二进制值为 f64 定宽 LE（未压缩）；值域差分 / 压缩（如 f32 截断或
  zstd）待基准数字支撑后决策；
- 缓存为进程内 FIFO（非 LRU），容量 8 固定；
- 时间序列加载（T46）已自然受益于磁盘读缓存外的二进制通道，但逐时间步
  仍是整文件读取——分块懒加载（按需读子区域）不在本任务。

## 验收标准（已满足）

- core 单测：二进制往返（含特殊值）、魔数 / 截断报错、FIFO 淘汰顺序、
  同键覆盖不重复占位、容量边界（kairos-core 行覆盖 100%）；
- 前端解码器单测：手写字节流逐字段解析、魔数 / 元数据截断 / 值区截断 /
  短缓冲报错（utils 四维覆盖 100%）；
- `bun run verify` 全绿。
