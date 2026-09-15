/**
 * 工作台骨架状态：左右列折叠偏好（持久到 localStorage，启动时恢复）、
 * 三列网格列宽，以及面板按分析阶段的显隐判断。
 * 中列始终保留——视口是工作台主角，不随折叠消失。
 */
import { computed, ref } from "vue";
import { useAppStore } from "../stores/app";
import type { Stage } from "../types";
import { storageGet, storageKey, storageSet } from "../utils/storage";

function readCollapsed(side: "left" | "right"): boolean {
  return storageGet(storageKey("layout", side), false);
}

const leftCollapsed = ref(false);
const rightCollapsed = ref(false);
let initialized = false;

export function useLayout() {
  const app = useAppStore();

  // 惰性初始化：首次调用时恢复持久化偏好（早于挂载，避免首帧跳动）。
  if (!initialized) {
    initialized = true;
    leftCollapsed.value = readCollapsed("left");
    rightCollapsed.value = readCollapsed("right");
  }

  function toggleLeft(): void {
    leftCollapsed.value = !leftCollapsed.value;
    storageSet(storageKey("layout", "left"), leftCollapsed.value);
  }

  function toggleRight(): void {
    rightCollapsed.value = !rightCollapsed.value;
    storageSet(storageKey("layout", "right"), rightCollapsed.value);
  }

  /** 三列网格列宽随折叠状态切换（Tailwind 任意值类需整串出现在源码中）。 */
  const gridClass = computed(() => {
    if (leftCollapsed.value && rightCollapsed.value) {
      return "grid-cols-[0px_minmax(0,1fr)_0px]";
    }
    if (leftCollapsed.value) {
      return "grid-cols-[0px_minmax(0,1fr)_320px]";
    }
    if (rightCollapsed.value) {
      return "grid-cols-[280px_minmax(0,1fr)_0px]";
    }
    return "grid-cols-[280px_minmax(0,1fr)_320px]";
  });

  /** 面板是否属于当前分析阶段（home 是总览阶段，各面板都配了它）。 */
  function stageVisible(stages: Stage[]): boolean {
    return stages.includes(app.stage);
  }

  return {
    leftCollapsed,
    rightCollapsed,
    toggleLeft,
    toggleRight,
    gridClass,
    stageVisible,
  };
}
