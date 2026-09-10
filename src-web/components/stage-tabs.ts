/**
 * 分析阶段选项卡：主页/几何/网格/工艺/求解/结果/报告。
 * 点击切换 store.stage，各列面板按 data-stages 显隐（main 编排）。
 */
import { appStore } from "../state";
import type { Stage } from "../types";

const STAGES: [Stage, string][] = [
  ["home", "主页"],
  ["geometry", "几何"],
  ["mesh", "网格"],
  ["process", "工艺"],
  ["solve", "求解"],
  ["results", "结果"],
  ["report", "报告"],
];

export function createStageTabs(): HTMLElement {
  const bar = document.createElement("div");
  bar.className = "flex items-center gap-1";

  const buttons = STAGES.map(([stage, label]) => {
    const tab = document.createElement("button");
    tab.type = "button";
    tab.textContent = label;
    tab.dataset.stage = stage;
    tab.className =
      "px-4 border-b-2 border-transparent text-xs text-zinc-500 hover:text-zinc-200 transition-colors";
    tab.addEventListener("click", () => appStore.set({ stage }));
    bar.append(tab);
    return tab;
  });

  const render = (): void => {
    const active = appStore.get().stage;
    for (const tab of buttons) {
      const on = tab.dataset.stage === active;
      tab.classList.toggle("border-emerald-400", on);
      tab.classList.toggle("text-emerald-400", on);
      tab.classList.toggle("font-semibold", on);
      tab.classList.toggle("border-transparent", !on);
    }
  };
  render();
  appStore.subscribe(render);
  return bar;
}
