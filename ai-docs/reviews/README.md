# 评审与专项报告索引

本目录放三类材料，命名前缀区分：

| 前缀        | 是什么                                 | 触发时机                   |
| ----------- | -------------------------------------- | -------------------------- |
| `M*`、`B2`  | 里程碑 / 分支整体评审                  | 里程碑收尾（T21 循环任务） |
| `T*-review` | 单任务评审（范围、证据、遗留）         | 任务收口时随提交入库       |
| 专项报告    | 复现报告、走查记录、上游往来、专项审计 | 按需                       |

### 关于「重复内容」

任务评审按 T21 的八项清单**逐项回填**，因此多份评审会共享同一张清单表头与相同的
「不适用 / 通过」结论措辞——这是评审证据的结构，**不是可删的重复**。机械查重工具会
把它们标为重复块，此处明确保留。

## 里程碑 / 分支评审

- [M0 评审](M0-review.md) · [M2 评审](M2-review.md) · [M3 评审](M3-review.md) ·
  [M4 评审](M4-review.md) · [M5 评审](M5-review.md) ·
  [M6 评审](M6-review.md) · [M6 第二轮](M6-review-round2.md)
- [B2 求解器评审](B2-solver-review.md)

## 整体审计

- [R1 全项目对照评审](R1-full-project-review.md)
- [R1 消融报告](R1-ablation-report.md)
- [R2 注释审计](R2-code-comment-audit.md)

## 真实求解链路（端到端）

- [e2e 求解报告](e2e-solve-report.md)：三条链路 + 4 进程并行（v1.0.0）
- [v0.2.3 复测](v023-retest-report.md)
- [v0.2.4 保压斜坡实验矩阵](v024-ramp-matrix-report.md)
- [GUI 走查记录](gui-walkthrough.md)

## 上游往来

- [上游问题清单](upstream-questions.md)：等上游答复的问题（含我方证据）
- [给上游的回复草稿](upstream-reply-draft.md)：核对上游建议后标出的问题

## 专项

- [路径重构评审](paths-refactor-review.md)

## 任务评审

T33 · T39 · T44 · T45 · T46 · T47 · T49 · T50 · T51 · T52 · T53 · T55 · T56 · T57 ·
T58 · T59 · T60 · T65 · T67 · T69 · T72 · T75 · T84 · T85 · T86 · T87 · T88 · T90 · T91
（各为 `<任务 id>-review.md`，如 [T60 评审](T60-review.md)）
