# T40 · IGES 导入（106 / 63 镶嵌子集）

- 阶段：已实现并接入产品路径（核心解析器 + `import_iges` 命令 + 导入按扩展名分发）
- 依赖：T05（几何导入与摘要管线）
- 优先级：P2

## 目标

补齐 C1 CAD 接口在 IGES 一侧的镶嵌网格导入：与 STEP 导入同级的「镶嵌子集」
口径——只接受网格化导出，不做 B-rep 求值。对标商业软件的 IGES 网格导入，
覆盖 CAD 常用的 Copious Data（实体 106）与 Compact Plane Subfigure（实体 63）。

## 范围（已实现）

- `kairos-core` `services/iges.rs`：
  - 固定 80 列行格式分节（D / P 节），目录条目按奇数序号索引；
  - 实体 63：封闭多边形（form 0），开放多边形（form 1）跳过并计数；
  - 实体 106：form 0（重复「点数 + (x,y)」块）、form 1 / 2（3D 路径 / 点列，
    首尾重合视为封闭）、form 11 / 12（分组线性路径）；封闭环扇形三角化；
  - 实体 124 变换矩阵：12 参数仿射定位，二维定义空间点先提升 (z=0) 再变换；
  - Hollerith 字符串（nH…）整体吞掉，内容中的分隔符不破坏切分；
  - 不属于子集的实体（未知类型 / 不可解析 / 未成封闭环）静默跳过，
    全部不可导入时才明确报错（与 STEP 解析器同一口径），请用户以镶嵌形式重新导出。
- `src-tauri`：`import_iges` 命令（异步阻塞线程池 + 会话缓存，与 STL/STEP 共用
  `store_import` 登记路径）。
- 前端：`importIges` API；导入对话框过滤器增加 `igs / iges`；几何 store 按
  扩展名分发（step/stp → STEP，igs/iges → IGES，其余 → STL）。

## 非目标（当前实现边界）

- 不解析 G 节自定义分隔符（按标准 `,` `;` 处理）；
- 不支持 NURBS / 解析曲面等 B-rep 实体（106 与 63 之外的几何实体全部跳过）；
- 不解析实体 106 的 form 13 及 40–45 系列变体；
- 单位不读 G 节，沿用几何摘要的包围盒推断。

## 验收标准（已满足）

- `cargo test -p kairos-core iges`：21 项单测覆盖各 form、变换、Hollerith、
  畸形与截断行、跳过统计（kairos-core 行覆盖保持 100%）；
- 前端 geometry store 分发测试覆盖 igs/iges 扩展名；
- `bun run verify` 全绿。
