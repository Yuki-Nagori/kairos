import { card, hint } from "./ui";

/** 工作区占位：注塑成型领域模块落地前，仅按 前处理 / 求解 / 后处理 预留三块版面。 */
export function createWorkspace(): HTMLElement {
  const root = document.createElement("div");
  root.className = "grid flex-1 grid-cols-1 gap-4 lg:grid-cols-3";

  const sections: Array<[string, string]> = [
    ["前处理", "制品 / 模具建模，网格划分（占位）"],
    ["求解", "填充 / 保压 / 冷却 / 翘曲分析（占位）"],
    ["后处理", "结果可视化与仿真报告（占位）"],
  ];
  for (const [title, text] of sections) {
    const { root: panel, body } = card(title);
    body.append(hint(text));
    panel.classList.add("min-h-64");
    root.append(panel);
  }
  return root;
}
