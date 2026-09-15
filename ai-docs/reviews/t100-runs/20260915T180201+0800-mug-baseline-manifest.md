# Mug baseline 试跑记录

- 模型：`report/mug-moldflow/mug.stl`
- 工况：220 °C 熔体、50 °C 模具、5.5 s 注射、保压起点 0.9229 MPa、保压终止 20 s
- 网格：Kairos 体积网格，目标尺寸 4.0 mm，1 核，Multipass `kairos` VM，运行时 v1.1.0
- 原始输出：[20260915T180201+0800-mug-baseline.raw.txt](20260915T180201+0800-mug-baseline.raw.txt)
- 结果：未完成；日志推进到物理时间 `0.838202 s` 后为控制资源停止，退出码 `1`。

本记录不作为数值对比结果。参考材料字段、公开 Generic PP 合理性检查和网格/终止时间差异已写入 T100 task。
