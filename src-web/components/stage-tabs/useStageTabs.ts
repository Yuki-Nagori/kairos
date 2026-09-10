/** 分析阶段选项卡逻辑：读写 store.stage，各列面板按阶段显隐由布局层（App 编排）负责。 */
import { computed } from "vue";
import { useAppStore } from "../../stores/app";
import type { Stage } from "../../types";

const STAGES: [Stage, string][] = [
  ["home", "主页"],
  ["geometry", "几何"],
  ["mesh", "网格"],
  ["process", "工艺"],
  ["solve", "求解"],
  ["results", "结果"],
  ["report", "报告"],
];

export function useStageTabs() {
  const app = useAppStore();
  const active = computed(() => app.stage);
  return { STAGES, app, active };
}
