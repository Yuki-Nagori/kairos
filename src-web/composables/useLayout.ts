/**
 * 工作台布局状态：左右列折叠偏好（持久到 localStorage，启动时恢复）。
 * 中列始终保留——视口是工作台主角，不随折叠消失。
 */
import { ref } from "vue";
import { storageGet, storageKey, storageSet } from "../utils/storage";

function readCollapsed(side: "left" | "right"): boolean {
  return storageGet(storageKey("layout", side), false);
}

const leftCollapsed = ref(false);
const rightCollapsed = ref(false);
let initialized = false;

export function useLayout() {
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

  return { leftCollapsed, rightCollapsed, toggleLeft, toggleRight };
}
