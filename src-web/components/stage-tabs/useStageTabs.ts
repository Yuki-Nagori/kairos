/** 分析阶段选项卡逻辑：读写 store.stage，各列面板按阶段显隐由布局层（App 编排）
 *  负责；行内角标把方案任务中「需要立即注意」的状态投射到对应选项卡。 */
import { computed } from "vue";
import { useAppStore } from "../../stores/app";
import type { Stage } from "../../types";
import { useStudyTasksSnapshot } from "../../composables/useStudyTasksSnapshot";
import { stageBadges, type StageBadge } from "../../utils/study-tasks";

export const STAGES: [Stage, string][] = [
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
  const { tasks } = useStudyTasksSnapshot();
  const active = computed(() => app.stage);
  /** 阶段 → 角标（无角标的阶段返回 undefined）。 */
  const badges = computed(() => stageBadges(tasks.value));
  function badgeOf(stage: Stage): StageBadge | undefined {
    return badges.value[stage];
  }
  return { STAGES, app, active, badgeOf };
}
