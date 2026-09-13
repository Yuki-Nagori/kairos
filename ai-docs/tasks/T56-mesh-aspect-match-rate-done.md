# T56 · 网格纵横比 + 双域匹配率进质量报告

- 阶段：B1（网格质量度量）
- 依赖：T06（体积网格）、T41（双域网格）
- 优先级：P2（立即做；回流自 R1 全项目对照评审步骤 4）

## 目标

补齐 Moldflow 用户最关注的两个网格统计量：

1. **纵横比（Aspect Ratio）**：core 新增统计——三角形纵横比 = 最长边 ÷
   最短高；四面体纵横比 = 最长棱 ÷ 最短高（具体定义开工时按数值稳定性
   定稿），报告输出 max / avg；
2. **双域匹配率（Match Ratio）**：DualDomainReport 新增
   `match_ratio = paired / (paired + unpaired)`（配对数据 T41 已有，只差
   计算与展示）。

## 验收标准

- MeshingReport 增加 `aspect_max` / `aspect_avg`（camelCase 契约锁定），
  几何面板与报告展示；
- DualDomainReport 增加 `matchRatio`（0~1），面板展示；
- 纯函数统计逻辑单测（含退化单元边界）+ 消融抽查；
- 契约测试同步更新。
